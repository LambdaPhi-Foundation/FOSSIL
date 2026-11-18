# Phase 6: MACHINE Refactoring

## Goal
Refactor existing MACHINE modules to build on CORE. Replace ad-hoc implementations with CORE traits.

---

## 6.1 Module Mapping

| Old Module | New Location | Purpose |
|------------|--------------|---------|
| `src/machine/win32.fsl` | `src/machine/win32.fsl` | Raw Windows API (unchanged) |
| `src/machine/io_handles.fsl` | `src/machine/io_handles.fsl` | Handle caching (unchanged) |
| `src/machine/conout.fsl` | `src/machine/io.fsl` | `Write` impl for stdout |
| `src/machine/conin.fsl` | `src/machine/io.fsl` | `Write` impl for stdin |
| `src/machine/conerr.fsl` | `src/machine/io.fsl` | `Write` impl for stderr |
| `src/machine/io.fsl` | `src/machine/core.fsl` | Re-export CORE + convenience fns |
| — | `src/machine/alloc.fsl` | libc allocator impl |
| — | `src/machine/thread.fsl` | Threading |
| — | `src/machine/time.fsl` | Time |
| — | `src/machine/file.fsl` | File I/O |
| — | `src/machine/process.fsl` | Process control |

---

## 6.2 `src/machine/core.fsl` — Re-export CORE

```fsl
MACHINE
MODULE core
    USE CORE.mem
    USE CORE.slice
    USE CORE.io
    USE CORE.fmt
    USE CORE.err
    USE CORE.panic
    USE CORE.alloc
    USE CORE.atomic
ENDMODULE
ENDMACHINE
```

---

## 6.3 `src/machine/io.fsl` — Write Implementations

```fsl
MACHINE
MODULE io
    USE MACHINE.core
    USE MACHINE.win32
    USE MACHINE.io_handles

    ; --- Conout Write Impl ---

    IMPL CORE.io.Write FOR MACHINE.Conout.Handle
        FN write(self: ptr[void], buf: slice[u8]): Result[usize, Error]
            VAR h = _get_stdout()
            VAR written: u32 = 0
            VAR res = WriteConsoleA(h, buf.ptr, buf.len as u32, &written, null)
            IF res != 0
                RETURN Ok(written as usize)
            ENDIF
            ; Fallback to WriteFile for redirected pipes
            res = WriteFile(h, buf.ptr, buf.len as u32, &written, null)
            IF res == 0
                RETURN Err(CORE.err.error_io("WriteFile failed"))
            ENDIF
            RETURN Ok(written as usize)
        ENDFN
    ENDIMPL

    ; --- Conerr Write Impl ---

    IMPL CORE.io.Write FOR MACHINE.Conerr.Handle
        FN write(self: ptr[void], buf: slice[u8]): Result[usize, Error]
            VAR h = _get_stderr()
            VAR written: u32 = 0
            VAR res = WriteConsoleA(h, buf.ptr, buf.len as u32, &written, null)
            IF res != 0
                RETURN Ok(written as usize)
            ENDIF
            res = WriteFile(h, buf.ptr, buf.len as u32, &written, null)
            IF res == 0
                RETURN Err(CORE.err.error_io("WriteFile failed"))
            ENDIF
            RETURN Ok(written as usize)
        ENDFN
    ENDIMPL

    ; --- Conin Read (separate trait) ---

    TRAIT Read
        FN read(self: ptr[void], buf: slice[u8]): Result[usize, Error]
    ENDTRAIT

    IMPL Read FOR MACHINE.Conin.Handle
        FN read(self: ptr[void], buf: slice[u8]): Result[usize, Error]
            VAR h = _get_stdin()
            VAR read: u32 = 0
            VAR res = ReadConsoleA(h, buf.ptr, buf.len as u32, &read, null)
            IF res != 0
                RETURN Ok(read as usize)
            ENDIF
            res = ReadFile(h, buf.ptr, buf.len as u32, &read, null)
            IF res == 0
                RETURN Err(CORE.err.error_io("ReadFile failed"))
            ENDIF
            RETURN Ok(read as usize)
        ENDFN
    ENDIMPL

    ; --- Global Writer Setup ---

    FN init_std_writers(): void
        CORE.io.set_stdout(&MACHINE.Conout.handle())
        CORE.io.set_stderr(&MACHINE.Conerr.handle())
    ENDFN

    ; --- Convenience Functions (backward compat) ---

    FN print(s: Str): isize
        VAR w = stdout().unwrap()
        w.write_str(s).unwrap_or(-1) as isize
    ENDFN

    FN println(s: Str): isize
        VAR w = stdout().unwrap()
        w.writeln_str(s).unwrap_or(-1) as isize
    ENDFN

    FN eprint(s: Str): isize
        VAR w = stderr().unwrap()
        w.write_str(s).unwrap_or(-1) as isize
    ENDFN

    FN eprintln(s: Str): isize
        VAR w = stderr().unwrap()
        w.writeln_str(s).unwrap_or(-1) as isize
    ENDFN

    FN read_line(buf: ptr[u8], len: usize): isize
        VAR w = stdin().unwrap()
        w.read(slice { ptr: buf, len }).unwrap_or(-1) as isize
    ENDFN

    FN exit(code: i32): void
        ExitProcess(code as u32)
        UNREACHABLE
    ENDFN

ENDMODULE
```

---

## 6.4 `src/machine/alloc.fsl` — libc Allocator

```fsl
MACHINE
MODULE alloc
    USE MACHINE.core
    USE MACHINE.win32

    EXTERN FN malloc(size: usize): ptr[u8] ENDFN
    EXTERN FN free(ptr: ptr[u8]): void ENDFN
    EXTERN FN realloc(ptr: ptr[u8], size: usize): ptr[u8] ENDFN
    EXTERN FN aligned_alloc(align: usize, size: usize): ptr[u8] ENDFN

    STRUCT LibcAllocator
    ENDSTRUCT

    IMPL CORE.alloc.Allocator FOR LibcAllocator
        FN alloc(self: ptr[void], size: usize, align: usize): Result[ptr[u8], Error]
            VAR ptr = aligned_alloc(align, size)
            IF ptr == null
                RETURN Err(CORE.err.error_oom())
            ENDIF
            RETURN Ok(ptr)
        ENDFN

        FN dealloc(self: ptr[void], ptr: ptr[u8], size: usize, align: usize): void
            free(ptr)
        ENDFN

        FN realloc(self: ptr[void], ptr: ptr[u8], old_size: usize, new_size: usize, align: usize): Result[ptr[u8], Error]
            VAR new_ptr = realloc(ptr, new_size)
            IF new_ptr == null
                RETURN Err(CORE.err.error_oom())
            ENDIF
            RETURN Ok(new_ptr)
        ENDFN
    ENDIMPL

    VAR _libc_allocator: LibcAllocator = LibcAllocator {}

    FN init_global_allocator(): void
        CORE.alloc.set_global_allocator(&_libc_allocator)
    ENDFN

ENDMODULE
```

---

## 6.5 `src/machine/thread.fsl` — Threading

```fsl
MACHINE
MODULE thread
    USE MACHINE.core
    USE MACHINE.win32
    USE MACHINE.alloc

    ; Thread handle
    STRUCT Thread
        handle: ptr[void]
        id: u32
    ENDSTRUCT

    ; Thread entry point
    EXTERN FN thread_entry(arg: ptr[void]): u32 ENDFN

    ; Spawn a new thread
    FN spawn(f: FN(ptr[void]): void, arg: ptr[void]): Result[Thread, Error]
        ; Allocate thread context
        VAR ctx = CORE.alloc.box_new(Closure { f, arg })
        MATCH ctx
            CASE Ok(boxed) =>
                VAR handle = CreateThread(null, 0, thread_entry, boxed.ptr, 0, null)
                IF handle == null
                    RETURN Err(CORE.err.error_io("CreateThread failed"))
                ENDIF
                RETURN Ok(Thread { handle, id: GetThreadId(handle) })
            CASE Err(e) => RETURN Err(e)
        ENDMATCH
    ENDFN

    ; Join thread
    FN join(t: Thread): Result[void, Error]
        WaitForSingleObject(t.handle, INFINITE)
        CloseHandle(t.handle)
        RETURN Ok(())
    ENDFN

    ; Current thread ID
    FN current_id(): u32
        RETURN GetCurrentThreadId()
    ENDFN

    ; Sleep
    FN sleep_ms(ms: u32): void
        Sleep(ms)
    ENDFN

    ; Thread-local storage (TLS)
    EXTERN FN tls_alloc(): u32 ENDFN
    EXTERN FN tls_free(idx: u32): void ENDFN
    EXTERN FN tls_get(idx: u32): ptr[void] ENDFN
    EXTERN FN tls_set(idx: u32, val: ptr[void]): void ENDFN

ENDMODULE
```

---

## 6.6 `src/machine/time.fsl` — Time

```fsl
MACHINE
MODULE time
    USE MACHINE.core
    USE MACHINE.win32

    STRUCT Instant
        ticks: u64
    ENDSTRUCT

    STRUCT Duration
        nanos: u64
    ENDSTRUCT

    ; High-resolution timer
    FN now(): Instant
        VAR freq: u64 = 0
        QueryPerformanceFrequency(&freq)
        VAR count: u64 = 0
        QueryPerformanceCounter(&count)
        RETURN Instant { ticks: count }
    ENDFN

    FN elapsed(start: Instant): Duration
        VAR freq: u64 = 0
        QueryPerformanceFrequency(&freq)
        VAR count: u64 = 0
        QueryPerformanceCounter(&count)
        VAR diff = count - start.ticks
        RETURN Duration { nanos: (diff * 1_000_000_000) / freq }
    ENDFN

    FN duration_as_secs(d: Duration): u64
        RETURN d.nanos / 1_000_000_000
    ENDFN

    FN duration_as_millis(d: Duration): u64
        RETURN d.nanos / 1_000_000
    ENDFN

    FN duration_as_micros(d: Duration): u64
        RETURN d.nanos / 1_000
    ENDFN

    ; Sleep
    FN sleep(d: Duration): void
        Sleep(d.nanos / 1_000_000)
    ENDFN

ENDMODULE
```

---

## 6.7 `src/machine/file.fsl` — File I/O

```fsl
MACHINE
MODULE file
    USE MACHINE.core
    USE MACHINE.win32
    USE MACHINE.alloc
    USE MACHINE.io

    STRUCT File
        handle: ptr[void]
    ENDSTRUCT

    ENUM OpenMode
        Read
        Write
        Append
        ReadWrite
    ENDENUM

    FN open(path: Str, mode: OpenMode): Result[File, Error]
        ; Convert Str to wide string for CreateFileW
        VAR wpath = str_to_wide(path)
        VAR access = match mode {
            Read => GENERIC_READ,
            Write => GENERIC_WRITE,
            Append => GENERIC_WRITE,
            ReadWrite => GENERIC_READ | GENERIC_WRITE,
        }
        VAR disp = match mode {
            Read => OPEN_EXISTING,
            Write => CREATE_ALWAYS,
            Append => OPEN_ALWAYS,
            ReadWrite => OPEN_ALWAYS,
        }
        VAR h = CreateFileW(wpath.ptr, access, FILE_SHARE_READ, null, disp, FILE_ATTRIBUTE_NORMAL, null)
        IF h == INVALID_HANDLE_VALUE
            RETURN Err(CORE.err.error_io("CreateFile failed"))
        ENDIF
        IF mode == Append
            SetFilePointer(h, 0, null, FILE_END)
        ENDIF
        RETURN Ok(File { handle: h })
    ENDFN

    IMPL CORE.io.Write FOR File
        FN write(self: ptr[void], buf: slice[u8]): Result[usize, Error]
            VAR f = cast self to File
            VAR written: u32 = 0
            VAR res = WriteFile(f.handle, buf.ptr, buf.len as u32, &written, null)
            IF res == 0
                RETURN Err(CORE.err.error_io("WriteFile failed"))
            ENDIF
            RETURN Ok(written as usize)
        ENDFN
    ENDIMPL

    IMPL CORE.io.Read FOR File
        FN read(self: ptr[void], buf: slice[u8]): Result[usize, Error]
            VAR f = cast self to File
            VAR read: u32 = 0
            VAR res = ReadFile(f.handle, buf.ptr, buf.len as u32, &read, null)
            IF res == 0
                RETURN Err(CORE.err.error_io("ReadFile failed"))
            ENDIF
            RETURN Ok(read as usize)
        ENDFN
    ENDIMPL

    FN close(f: File): Result[void, Error]
        CloseHandle(f.handle)
        RETURN Ok(())
    ENDFN

    FN flush(f: File): Result[void, Error]
        FlushFileBuffers(f.handle)
        RETURN Ok(())
    ENDFN

    FN seek(f: File, pos: i64, whence: SeekWhence): Result[u64, Error]
        ; SetFilePointerEx
    ENDFN

ENDMODULE
```

---

## 6.8 `src/machine/process.fsl` — Process Control

```fsl
MACHINE
MODULE process
    USE MACHINE.core
    USE MACHINE.win32

    ; Command line arguments
    FN args(): slice[Str]
        ; Parse GetCommandLineW
    ENDFN

    ; Environment variables
    FN env_var(name: Str): Option[Str]
        ; GetEnvironmentVariableW
    ENDFN

    FN set_env_var(name: Str, value: Str): Result[void, Error]
        ; SetEnvironmentVariableW
    ENDFN

    ; Process spawning
    STRUCT ChildProcess
        handle: ptr[void]
        id: u32
    ENDSTRUCT

    FN spawn(program: Str, args: slice[Str]): Result[ChildProcess, Error]
        ; CreateProcessW
    ENDFN

    FN wait(p: ChildProcess): Result[i32, Error]
        WaitForSingleObject(p.handle, INFINITE)
        VAR code: u32 = 0
        GetExitCodeProcess(p.handle, &code)
        CloseHandle(p.handle)
        RETURN Ok(code as i32)
    ENDFN

    FN exit(code: i32): void
        ExitProcess(code as u32)
        UNREACHABLE
    ENDFN

ENDMODULE
```

---

## 6.9 Backward Compatibility

The old `MACHINE::IO` convenience functions still work:

```fsl
; In MACHINE.io (re-exports CORE.io macros)
MACRO print(fmt: str, args: ...)
    CORE.io.print(fmt, args...)
ENDMACRO

MACRO println(fmt: str, args: ...)
    CORE.io.println(fmt, args...)
ENDMACRO
```

Existing code using `IO.println("hello")` continues to work.

---

## 6.10 Deleted/Deprecated Files

| Old File | Status |
|----------|--------|
| `src/machine/conout.fsl` | → merged into `io.fsl` |
| `src/machine/conin.fsl` | → merged into `io.fsl` |
| `src/machine/conerr.fsl` | → merged into `io.fsl` |
| `src/machine/io.fsl` | → renamed to `core.fsl` |
| `src/machine/io_handles.fsl` | Keep (handle caching) |
| `src/machine/win32.fsl` | Keep (raw API) |

---

## Acceptance Criteria

- [ ] `MACHINE.core` re-exports all CORE modules
- [ ] `Conout`/`Conerr` implement `CORE.io.Write`
- [ ] `Conin` implements `CORE.io.Read`
- [ ] `MACHINE.alloc` provides libc allocator
- [ ] Thread/time/file/process modules compile
- [ ] Old `IO.println()` still works
- [ ] All existing integration tests pass
- [ ] New tests using CORE through MACHINE pass