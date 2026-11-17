# MACHINE::IO Implementation Plan

Status: **Design Phase**
Target: Stage One (LLVM-backed compiler)

---

## 1. Overview

`MACHINE::IO` provides zero-libc I/O for `MACHINE` programs via Windows API (kernel32.dll) or direct syscalls. The API is organized into three submodules.

> **Language**: FOSSIL (Finitely Organized Simple Systems Instruction Language)

```
USE MACHINE::IO

Conout.write("hello")      ; unbuffered write to stdout
Conout.writeln("hello")    ; write + newline
Conout.flush()             ; flush stdout

Conin.read(buf, len)       ; read up to len bytes
Conin.scanline(buf, len)   ; read line (stops at \n)
Conin.read_char()          ; read single char

Conerr.write("error")      ; unbuffered write to stderr
Conerr.writeln("error")    ; write + newline
```

---

## 2. Module Structure

```
MACHINE::IO
├── Conout          ; Standard Output (STDOUT)
│   ├── write(buf: ptr[u8], len: usize): isize
│   ├── writeln(buf: ptr[u8], len: usize): isize
│   ├── write_str(s: str): isize
│   ├── writeln_str(s: str): isize
│   ├── flush(): void
│   └── handle(): ptr[u8]        ; raw HANDLE for advanced use
│
├── Conin           ; Standard Input (STDIN)
│   ├── read(buf: ptr[u8], len: usize): isize
│   ├── scanline(buf: ptr[u8], len: usize): isize
│   ├── read_char(): i32         ; returns -1 on EOF
│   ├── read_str(buf: ptr[u8], len: usize): isize
│   └── handle(): ptr[u8]
│
├── Conerr          ; Standard Error (STDERR)
│   ├── write(buf: ptr[u8], len: usize): isize
│   ├── writeln(buf: ptr[u8], len: usize): isize
│   ├── write_str(s: str): isize
│   ├── writeln_str(s: str): isize
│   ├── flush(): void
│   └── handle(): ptr[u8]
│
└── internals       ; Private implementation
    ├── win32.rs    ; Windows API bindings
    ├── syscall.rs  ; Direct syscall wrappers (NO_MACHINE)
    └── buffer.rs   ; Optional: line buffering
```

---

## 3. Windows API Bindings (kernel32.dll)

### 3.1 EXTERN Declarations

```relic
; src/machine/win32.fsl (built-in, auto-available in MACHINE mode)
MACHINE

; Handle management
EXTERN FN GetStdHandle(nStdHandle: i32): ptr[u8] ENDFN
EXTERN FN GetCurrentProcess(): ptr[u8] ENDFN

; Console I/O (preferred for interactive)
EXTERN FN WriteConsoleA(
    hConsoleOutput: ptr[u8],
    lpBuffer: ptr[u8],
    nNumberOfCharsToWrite: u32,
    lpNumberOfCharsWritten: ptr[u32],
    lpReserved: ptr[u8]
): i32 ENDFN

EXTERN FN ReadConsoleA(
    hConsoleInput: ptr[u8],
    lpBuffer: ptr[u8],
    nNumberOfCharsToRead: u32,
    lpNumberOfCharsRead: ptr[u32],
    pInputControl: ptr[u8]
): i32 ENDFN

; File/Handle I/O (works for redirected pipes)
EXTERN FN WriteFile(
    hFile: ptr[u8],
    lpBuffer: ptr[u8],
    nNumberOfBytesToWrite: u32,
    lpNumberOfBytesWritten: ptr[u32],
    lpOverlapped: ptr[u8]
): i32 ENDFN

EXTERN FN ReadFile(
    hFile: ptr[u8],
    lpBuffer: ptr[u8],
    nNumberOfBytesToRead: u32,
    lpNumberOfBytesRead: ptr[u32],
    lpOverlapped: ptr[u8]
): i32 ENDFN

EXTERN FN FlushFileBuffers(hFile: ptr[u8]): i32 ENDFN

; Process control
EXTERN FN ExitProcess(uExitCode: u32): void ENDFN

; Console mode
EXTERN FN GetConsoleMode(hConsoleHandle: ptr[u8], lpMode: ptr[u32]): i32 ENDFN
EXTERN FN SetConsoleMode(hConsoleHandle: ptr[u8], dwMode: u32): i32 ENDFN

; Constants
CONST STD_INPUT_HANDLE: i32 = -10
CONST STD_OUTPUT_HANDLE: i32 = -11
CONST STD_ERROR_HANDLE: i32 = -12

CONST ENABLE_PROCESSED_INPUT: u32 = 0x0001
CONST ENABLE_LINE_INPUT: u32 = 0x0002
CONST ENABLE_ECHO_INPUT: u32 = 0x0004
CONST ENABLE_WINDOW_INPUT: u32 = 0x0008
CONST ENABLE_MOUSE_INPUT: u32 = 0x0010
CONST ENABLE_INSERT_MODE: u32 = 0x0020
CONST ENABLE_QUICK_EDIT_MODE: u32 = 0x0040
CONST ENABLE_EXTENDED_FLAGS: u32 = 0x0080
CONST ENABLE_AUTO_POSITION: u32 = 0x0100
CONST ENABLE_VIRTUAL_TERMINAL_INPUT: u32 = 0x0200
```

### 3.2 Handle Caching

```relic
; src/machine/io_handles.fsl
MACHINE

; Cached handles (initialized on first use)
VAR _stdout_handle: ptr[u8] = null
VAR _stderr_handle: ptr[u8] = null
VAR _stdin_handle: ptr[u8] = null

FN _get_stdout(): ptr[u8]
    IF _stdout_handle == null
        _stdout_handle = GetStdHandle(STD_OUTPUT_HANDLE)
    ENDIF
    RETURN _stdout_handle
ENDFN

FN _get_stderr(): ptr[u8]
    IF _stderr_handle == null
        _stderr_handle = GetStdHandle(STD_ERROR_HANDLE)
    ENDIF
    RETURN _stderr_handle
ENDFN

FN _get_stdin(): ptr[u8]
    IF _stdin_handle == null
        _stdin_handle = GetStdHandle(STD_INPUT_HANDLE)
    ENDIF
    RETURN _stdin_handle
ENDFN
```

---

## 4. Conout Implementation

```relic
; Conout submodule
MODULE Conout
    USE MACHINE::IO::internals

    FN write(buf: ptr[u8], len: usize): isize
        VAR h = _get_stdout()
        VAR written: u32 = 0
        ; Try WriteConsoleA first (handles Unicode, colors)
        VAR result = WriteConsoleA(h, buf, len as u32, &written, null)
        IF result == 0
            ; Fallback to WriteFile for redirected output
            result = WriteFile(h, buf, len as u32, &written, null)
            IF result == 0
                RETURN -1
            ENDIF
        ENDIF
        RETURN written as isize
    ENDFN

    FN write_str(s: str): isize
        RETURN write(s.ptr, s.len)
    ENDFN

    FN writeln(buf: ptr[u8], len: usize): isize
        VAR n = write(buf, len)
        IF n < 0
            RETURN n
        ENDIF
        VAR nl: u8 = '\n' as u8
        VAR m = write(&nl, 1)
        IF m < 0
            RETURN m
        ENDIF
        RETURN n + m
    ENDFN

    FN writeln_str(s: str): isize
        VAR n = write_str(s)
        IF n < 0
            RETURN n
        ENDIF
        VAR nl: u8 = '\n' as u8
        VAR m = write(&nl, 1)
        IF m < 0
            RETURN m
        ENDIF
        RETURN n + m
    ENDFN

    FN flush(): void
        VAR h = _get_stdout()
        FlushFileBuffers(h)
    ENDFN

    FN handle(): ptr[u8]
        RETURN _get_stdout()
    ENDFN
ENDMODULE
```

---

## 5. Conin Implementation

```relic
; Conin submodule
MODULE Conin
    USE MACHINE::IO::internals

    FN read(buf: ptr[u8], len: usize): isize
        VAR h = _get_stdin()
        VAR read: u32 = 0
        VAR result = ReadConsoleA(h, buf, len as u32, &read, null)
        IF result == 0
            ; Fallback for redirected input
            result = ReadFile(h, buf, len as u32, &read, null)
            IF result == 0
                RETURN -1
            ENDIF
        ENDIF
        RETURN read as isize
    ENDFN

    FN scanline(buf: ptr[u8], max_len: usize): isize
        VAR total: usize = 0
        WHILE total < max_len - 1
            VAR c = read_char()
            IF c < 0
                IF total == 0
                    RETURN -1  ; EOF
                ENDIF
                BREAK
            ENDIF
            IF c == '\n' as i32
                BREAK
            ENDIF
            buf[total] = c as u8
            total += 1
        ENDWHILE
        IF total < max_len
            buf[total] = 0  ; null-terminate
        ENDIF
        RETURN total as isize
    ENDFN

    FN read_char(): i32
        VAR c: u8 = 0
        VAR n = read(&c, 1)
        IF n <= 0
            RETURN -1
        ENDIF
        RETURN c as i32
    ENDFN

    FN read_str(buf: ptr[u8], len: usize): isize
        RETURN read(buf, len)
    ENDFN

    FN handle(): ptr[u8]
        RETURN _get_stdin()
    ENDFN
ENDMODULE
```

---

## 6. Conerr Implementation

```relic
; Conerr submodule (mirrors Conout but for stderr)
MODULE Conerr
    USE MACHINE::IO::internals

    FN write(buf: ptr[u8], len: usize): isize
        VAR h = _get_stderr()
        VAR written: u32 = 0
        VAR result = WriteConsoleA(h, buf, len as u32, &written, null)
        IF result == 0
            result = WriteFile(h, buf, len as u32, &written, null)
            IF result == 0
                RETURN -1
            ENDIF
        ENDIF
        RETURN written as isize
    ENDFN

    FN write_str(s: str): isize
        RETURN write(s.ptr, s.len)
    ENDFN

    FN writeln(buf: ptr[u8], len: usize): isize
        VAR n = write(buf, len)
        IF n < 0
            RETURN n
        ENDIF
        VAR nl: u8 = '\n' as u8
        VAR m = write(&nl, 1)
        IF m < 0
            RETURN m
        ENDIF
        RETURN n + m
    ENDFN

    FN writeln_str(s: str): isize
        VAR n = write_str(s)
        IF n < 0
            RETURN n
        ENDIF
        VAR nl: u8 = '\n' as u8
        VAR m = write(&nl, 1)
        IF m < 0
            RETURN m
        ENDIF
        RETURN n + m
    ENDFN

    FN flush(): void
        VAR h = _get_stderr()
        FlushFileBuffers(h)
    ENDFN

    FN handle(): ptr[u8]
        RETURN _get_stderr()
    ENDFN
ENDMODULE
```

---

## 7. Public API Re-exports

```relic
; MACHINE::IO top-level (src/machine/io.fsl)
MODULE IO
    USE MACHINE::IO::Conout
    USE MACHINE::IO::Conin
    USE MACHINE::IO::Conerr

    ; Re-export for convenience
    FN print(s: str): isize = Conout.write_str(s)
    FN println(s: str): isize = Conout.writeln_str(s)
    FN eprint(s: str): isize = Conerr.write_str(s)
    FN eprintln(s: str): isize = Conerr.writeln_str(s)
    FN read_line(buf: ptr[u8], len: usize): isize = Conin.scanline(buf, len)
    FN exit(code: i32): void = ExitProcess(code as u32)
ENDMODULE
```

---

## 8. Usage Examples

### 8.1 Basic Hello World

```relic
USE MACHINE::IO

FN main(): i32
    Conout.writeln_str("Hello, World!")
    exit(0)
ENDFN
```

### 8.2 Echo Program

```relic
USE MACHINE::IO

FN main(): i32
    VAR buf: [u8, 256]
    LOOP
        Conout.write_str("> ")
        Conout.flush()
        VAR n = Conin.scanline(&buf, 256)
        IF n <= 0
            BREAK
        ENDIF
        Conout.writeln_str("Echo: ")
        Conout.write(&buf, n as usize)
        Conout.write_str("\n")
    ENDLOOP
    exit(0)
ENDFN
```

### 8.3 Error Handling

```relic
USE MACHINE::IO

FN copy_file(src: str, dst: str): i32
    VAR src_h = open(src, READ)
    IF src_h == null
        Conerr.writeln_str("Error: cannot open source")
        RETURN 1
    ENDIF
    VAR dst_h = open(dst, WRITE)
    IF dst_h == null
        Conerr.writeln_str("Error: cannot create destination")
        close(src_h)
        RETURN 1
    ENDIF
    
    VAR buf: [u8, 4096]
    LOOP
        VAR n = read(src_h, &buf, 4096)
        IF n <= 0
            BREAK
        ENDIF
        VAR w = write(dst_h, &buf, n as usize)
        IF w != n
            Conerr.writeln_str("Error: write failed")
            close(src_h)
            close(dst_h)
            RETURN 1
        ENDIF
    ENDLOOP
    
    close(src_h)
    close(dst_h)
    RETURN 0
ENDFN
```

---

## 9. Implementation Phases

| Phase | Tasks | Files | Est. Effort |
|-------|-------|-------|-------------|
| **9.1** | EXTERN support for kernel32.dll functions | `parser.rs`, `codegen.rs`, `semantic.rs` | 2 days |
| **9.2** | Windows constants module (`win32.fsl`) | `src/machine/win32.fsl` | 1 day |
| **9.3** | Handle caching (`io_handles.fsl`) | `src/machine/io_handles.fsl` | 0.5 days |
| **9.4** | Conout implementation | `src/machine/conout.fsl` | 1 day |
| **9.5** | Conin implementation | `src/machine/conin.fsl` | 1 day |
| **9.6** | Conerr implementation | `src/machine/conerr.fsl` | 0.5 days |
| **9.7** | Top-level IO module + re-exports | `src/machine/io.fsl` | 0.5 days |
| **9.9** | Integration tests | `tests/integration_tests.rs`, `test_io_*.fsl` | 1 day |
| **9.10** | Fix operand inference (inline asm Gap 8.1) | `parser.rs`, `semantic.rs` | 2 days |

**Total: ~10.5 days**

---

## 10. Dependencies

### 10.1 EXTERN Support (Required First)
- Parse `EXTERN FN name(args): ret ENDFN`
- Generate `declare` in LLVM IR
- Link against `kernel32.lib` (MSVC) or `kernel32.dll` (MinGW)

### 10.2 Inline Assembly Operand Inference (Required for Reliable Syscalls)
- Parser must detect output operands from template
- Currently all operands get `"r"` constraint
- Needed for `WriteConsoleA` wrappers using `mov rax, `nr`` pattern

### 10.3 Module System
- `MODULE` / `USE` already implemented
- Need `MACHINE::` namespace resolution

---

## 11. NO_MACHINE / Bare Metal Variant

For `NO_MACHINE` programs (bootloaders, kernels), provide direct syscall version:

```relic
; src/machine/syscall.fsl
NO_MACHINE

FN sys_write(fd: i32, buf: ptr[u8], len: usize): isize
    ASM
        mov rax, 1
        mov rdi, `fd`
        mov rsi, `buf`
        mov rdx, `len`
        syscall
        mov `result`, rax
    ENDASM CLobbers("rax", "rdi", "rsi", "rdx", "rcx", "r11") Flags("volatile")
    RETURN result
ENDFN

FN sys_read(fd: i32, buf: ptr[u8], len: usize): isize
    ASM
        mov rax, 0
        mov rdi, `fd`
        mov rsi, `buf`
        mov rdx, `len`
        syscall
        mov `result`, rax
    ENDASM CLobbers("rax", "rdi", "rsi", "rdx", "rcx", "r11") Flags("volatile")
    RETURN result
ENDFN

FN sys_exit(code: i32): void
    ASM
        mov rax, 60
        mov rdi, `code`
        syscall
    ENDASM CLobbers("rax", "rdi") Flags("volatile", "noreturn")
    UNREACHABLE
ENDFN
```

Then `Conout`/`Conin`/`Conerr` can use `sys_write`/`sys_read`/`sys_exit` instead of Windows API.

---

## 12. Testing Strategy

| Test File | Description |
|-----------|-------------|
| `test_io_hello.relic` | `Conout.writeln_str("Hello")` |
| `test_io_stderr.relic` | `Conerr.writeln_str("Error")` |
| `test_io_echo.relic` | Read line, write back |
| `test_io_scanline.relic` | `Conin.scanline()` with various inputs |
| `test_io_redirect.relic` | Test with piped input/output |
| `test_io_mixed.relic` | Interleaved stdout/stderr |
| `test_io_no_machine.relic` | Same tests with `NO_MACHINE` + syscalls |

---

## 13. Open Questions

1. **Buffering**: Add line buffering for `Conout`/`Conerr`? (Current: unbuffered)
2. **Unicode**: Use `WriteConsoleW` / `ReadConsoleW` for UTF-16? (Current: ANSI `A` variants)
3. **Async I/O**: Overlapped I/O for non-blocking? (Deferred)
4. **Error codes**: Return Windows error via `GetLastError()`? (Add `errno()` equivalent)
5. **Path handling**: Full `open(path, mode)` abstraction? (Separate `MACHINE::FS` module)

---

*Last Updated: 2026-11-17*
*Status: Design complete. Ready for Phase 9.1 (EXTERN support).*