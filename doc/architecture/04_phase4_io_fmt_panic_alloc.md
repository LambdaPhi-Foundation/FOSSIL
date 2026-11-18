# Phase 4: CORE.io, CORE.fmt, CORE.panic, CORE.alloc

## Goal
Build I/O abstraction, formatting engine, panic path, and allocator trait on top of the trait system.

---

## 4.1 `src/core/io.fsl` — Write Trait & Sink

```fsl
MODULE io
    USE CORE.slice
    USE CORE.err
    USE CORE.fmt

    ; --- Write Trait ---

    TRAIT Write
        ; Required: single write call
        FN write(self: ptr[void], buf: slice[u8]): Result[usize, Error]
        
        ; Optional: default implementations
        FN write_all(self: ptr[void], buf: slice[u8]): Result[usize, Error]
            VAR written: usize = 0
            WHILE written < buf.len
                VAR res = self.write(buf.slice(written, buf.len))
                MATCH res
                    CASE Ok(n) => written = written + n
                    CASE Err(e) => RETURN Err(e)
                ENDMATCH
            ENDWHILE
            RETURN Ok(written)
        ENDFN

        FN write_str(self: ptr[void], s: Str): Result[usize, Error]
            RETURN self.write(s.as_slice())
        ENDFN

        FN flush(self: ptr[void]): Result[void, Error]
            RETURN Ok(())
        ENDFN
    ENDTRAIT

    ; --- Null Writer (discards output) ---

    STRUCT NullWriter
    ENDSTRUCT

    IMPL Write FOR NullWriter
        FN write(self: ptr[void], buf: slice[u8]): Result[usize, Error]
            RETURN Ok(buf.len)
        ENDFN
    ENDIMPL

    ; --- Standard Writers (set by platform) ---

    VAR _stdout: Option[ptr[Write]] = None
    VAR _stderr: Option[ptr[Write]] = None

    FN set_stdout(w: ptr[Write]): void
        _stdout = Some(w)
    ENDFN

    FN set_stderr(w: ptr[Write]): void
        _stderr = Some(w)
    ENDFN

    FN stdout(): [ptr[Write], Error]
        MATCH _stdout
            CASE Some(w) => Ok(w)
            CASE None => Err(error_new(ErrorKind.NotConnected, "stdout not set"))
        ENDMATCH
    ENDFN

    FN stderr(): [ptr[Write], Error]
        MATCH _stderr
            CASE Some(w) => Ok(w)
            CASE None => Err(error_new(ErrorKind.NotConnected, "stderr not set"))
        ENDMATCH
    ENDFN

    ; --- Convenience Macros ---

    MACRO print(fmt: str, args: ...)
        VAR w = stdout().unwrap()
        format(w, fmt, args...)
    ENDMACRO

    MACRO println(fmt: str, args: ...)
        VAR w = stdout().unwrap()
        writeln(w, fmt, args...)
    ENDMACRO

    MACRO eprint(fmt: str, args: ...)
        VAR w = stderr().unwrap()
        format(w, fmt, args...)
    ENDMACRO

    MACRO eprintln(fmt: str, args: ...)
        VAR w = stderr().unwrap()
        writeln(w, fmt, args...)
    ENDMACRO

ENDMODULE
```

---

## 4.2 `src/core/fmt.fsl` — Allocation-Free Formatting

```fsl
MODULE fmt
    USE CORE.slice
    USE CORE.io
    USE CORE.err
    USE CORE.slice

    ; --- Format Specification (compile-time) ---

    STRUCT FormatSpec
        width: usize
        precision: Option[usize]
        fill: u8
        align: Align
        sign: Sign
        base: Base
        uppercase: bool
    ENDSTRUCT

    ENUM Align { Left, Right, Center }
    ENUM Sign { None, Plus, Minus }
    ENUM Base { Bin, Oct, Dec, Hex }

    ; --- Format Trait ---

    TRAIT Format
        FN fmt(self: ptr[void], writer: ptr[Write], spec: FormatSpec): Result[usize, Error]
    ENDTRAIT

    ; --- Format String Parsing (macro) ---

    MACRO format(writer: ptr[Write], fmt: str, args: ...): Result[usize, Error]
        ; Compile-time: parse fmt string like "{} {}"
        ; Generates: arg0.fmt(writer, spec0); arg1.fmt(writer, spec1); ...
    ENDMACRO

    MACRO writeln(writer: ptr[Write], fmt: str, args: ...): Result[usize, Error]
        VAR res = format(writer, fmt, args...)
        writer.write_str("\n")
        RETURN res
    ENDMACRO

    ; --- Built-in Implementations ---

    IMPL Format FOR i8
        FN fmt(self: ptr[void], w: ptr[Write], spec: FormatSpec): Result[usize, Error]
            VAR buf: [u8; 16]
            VAR len = int_to_str(*self, buf.ptr, spec)
            RETURN w.write(slice { ptr: buf.ptr, len })
        ENDFN
    ENDIMPL

    IMPL Format FOR u8
        FN fmt(self: ptr[void], w: ptr[Write], spec: FormatSpec): Result[usize, Error]
            VAR buf: [u8; 16]
            VAR len = uint_to_str(*self, buf.ptr, spec)
            RETURN w.write(slice { ptr: buf.ptr, len })
        ENDFN
    ENDIMPL

    IMPL Format FOR i16
        FN fmt(self: ptr[void], w: ptr[Write], spec: FormatSpec): Result[usize, Error]
            VAR buf: [u8; 16]
            VAR len = int_to_str(*self, buf.ptr, spec)
            RETURN w.write(slice { ptr: buf.ptr, len })
        ENDFN
    ENDIMPL

    IMPL Format FOR u16
        FN fmt(self: ptr[void], w: ptr[Write], spec: FormatSpec): Result[usize, Error]
            VAR buf: [u8; 16]
            VAR len = uint_to_str(*self, buf.ptr, spec)
            RETURN w.write(slice { ptr: buf.ptr, len })
        ENDFN
    ENDIMPL

    IMPL Format FOR i32
        FN fmt(self: ptr[void], w: ptr[Write], spec: FormatSpec): Result[usize, Error]
            VAR buf: [u8; 32]
            VAR len = int_to_str(*self, buf.ptr, spec)
            RETURN w.write(slice { ptr: buf.ptr, len })
        ENDFN
    ENDIMPL

    IMPL Format FOR u32
        FN fmt(self: ptr[void], w: ptr[Write], spec: FormatSpec): Result[usize, Error]
            VAR buf: [u8; 32]
            VAR len = uint_to_str(*self, buf.ptr, spec)
            RETURN w.write(slice { ptr: buf.ptr, len })
        ENDFN
    ENDIMPL

    IMPL Format FOR i64
        FN fmt(self: ptr[void], w: ptr[Write], spec: FormatSpec): Result[usize, Error]
            VAR buf: [u8; 64]
            VAR len = int_to_str(*self, buf.ptr, spec)
            RETURN w.write(slice { ptr: buf.ptr, len })
        ENDFN
    ENDIMPL

    IMPL Format FOR u64
        FN fmt(self: ptr[void], w: ptr[Write], spec: FormatSpec): Result[usize, Error]
            VAR buf: [u8; 64]
            VAR len = uint_to_str(*self, buf.ptr, spec)
            RETURN w.write(slice { ptr: buf.ptr, len })
        ENDFN
    ENDIMPL

    IMPL Format FOR isize
        FN fmt(self: ptr[void], w: ptr[Write], spec: FormatSpec): Result[usize, Error]
            VAR buf: [u8; 64]
            VAR len = int_to_str(*self, buf.ptr, spec)
            RETURN w.write(slice { ptr: buf.ptr, len })
        ENDFN
    ENDIMPL

    IMPL Format FOR usize
        FN fmt(self: ptr[void], w: ptr[Write], spec: FormatSpec): Result[usize, Error]
            VAR buf: [u8; 64]
            VAR len = uint_to_str(*self, buf.ptr, spec)
            RETURN w.write(slice { ptr: buf.ptr, len })
        ENDFN
    ENDIMPL

    IMPL Format FOR bool
        FN fmt(self: ptr[void], w: ptr[Write], spec: FormatSpec): Result[usize, Error]
            IF *self
                RETURN w.write_str("true")
            ELSE
                RETURN w.write_str("false")
            ENDIF
        ENDFN
    ENDIMPL

    IMPL Format FOR Str
        FN fmt(self: ptr[void], w: ptr[Write], spec: FormatSpec): Result[usize, Error]
            RETURN w.write_str(*self)
        ENDFN
    ENDIMPL

    IMPL Format FOR slice[u8]
        FN fmt(self: ptr[void], w: ptr[Write], spec: FormatSpec): Result[usize, Error]
            RETURN w.write(*self)
        ENDFN
    ENDIMPL

    IMPL Format FOR ptr[void]
        FN fmt(self: ptr[void], w: ptr[Write], spec: FormatSpec): Result[usize, Error]
            VAR buf: [u8; 32]
            VAR len = ptr_to_str(*self, buf.ptr, spec)
            RETURN w.write(slice { ptr: buf.ptr, len })
        ENDFN
    ENDIMPL

    ; --- Float formatting (requires more buffer) ---

    IMPL Format FOR f32
        FN fmt(self: ptr[void], w: ptr[Write], spec: FormatSpec): Result[usize, Error]
            VAR buf: [u8; 64]
            VAR len = float_to_str(*self, buf.ptr, spec)
            RETURN w.write(slice { ptr: buf.ptr, len })
        ENDFN
    ENDIMPL

    IMPL Format FOR f64
        FN fmt(self: ptr[void], w: ptr[Write], spec: FormatSpec): Result[usize, Error]
            VAR buf: [u8; 128]
            VAR len = float_to_str(*self, buf.ptr, spec)
            RETURN w.write(slice { ptr: buf.ptr, len })
        ENDFN
    ENDIMPL

    ; --- Primitive conversion functions (intrinsics) ---

    EXTERN FN int_to_str(val: i64, buf: ptr[u8], spec: FormatSpec): usize ENDFN
    EXTERN FN uint_to_str(val: u64, buf: ptr[u8], spec: FormatSpec): usize ENDFN
    EXTERN FN float_to_str(val: f64, buf: ptr[u8], spec: FormatSpec): usize ENDFN
    EXTERN FN ptr_to_str(val: ptr[void], buf: ptr[u8], spec: FormatSpec): usize ENDFN

ENDMODULE
```

### Formatting Intrinsics
Lower to compiler-builtin functions that write directly to fixed buffers — no heap allocation.

---

## 4.3 `src/core/panic.fsl` — Panic/Abort Path

```fsl
MODULE panic
    USE CORE.io
    USE CORE.fmt
    USE CORE.err

    ; Platform-specific abort
    EXTERN FN abort() ENDFN

    ; Trap instruction (bare metal)
    EXTERN FN trap() ENDFN

    ; --- Panic Entry Point ---

    FN panic(msg: Str): void
        ; Write directly to stderr (unbuffered, no locks)
        VAR w = match stderr() {
            Ok(w) => w,
            Err(_) => {
                ; Fallback: direct platform write
                platform_write_stderr(msg)
                abort()
            }
        }
        
        ; Synchronous write — no buffering
        w.writeln("PANIC: {}", msg)
        w.flush()
        
        ; Platform-specific termination
        ; Bare metal: trap() → ud2/bkpt
        ; Hosted: abort() → ExitProcess / exit
        platform_abort()
        UNREACHABLE
    ENDFN

    ; Platform-specific primitives
    EXTERN FN platform_write_stderr(msg: Str) ENDFN
    EXTERN FN platform_abort() ENDFN

    ; --- Assert Macros ---

    MACRO assert(cond: bool, msg: str)
        IF !cond
            panic(msg)
        ENDIF
    ENDMACRO

    MACRO assert_eq[T](a: T, b: T, msg: str)
        IF a != b
            panic("assertion failed: {} != {}: {}", a, b, msg)
        ENDIF
    ENDMACRO

    MACRO debug_assert(cond: bool, msg: str)
        ; Only in debug builds (compiler strips in release)
    ENDMACRO

    MACRO unreachable(msg: str)
        panic("unreachable: " + msg)
    ENDMACRO

ENDMODULE
```

### Platform Implementations

**Bare Metal (`NO_MACHINE`):**
```fsl
FN platform_write_stderr(msg: Str): void
    ; UART write / semihosting / framebuffer
    ; No syscalls
ENDFN

FN platform_abort(): void
    trap()  ; llvm.trap / ud2 / bkpt
ENDFN
```

**Hosted (`MACHINE` Windows):**
```fsl
FN platform_write_stderr(msg: Str): void
    WriteFile(stderr_handle, msg.ptr, msg.len, ...)
ENDFN

FN platform_abort(): void
    ExitProcess(1)
ENDFN
```

---

## 4.4 `src/core/alloc.fsl` — Allocator Trait

```fsl
MODULE alloc
    USE CORE.err
    USE CORE.slice

    ; --- Allocator Trait (Contract Only) ---

    TRAIT Allocator
        ; Allocate `size` bytes with `align` alignment
        FN alloc(self: ptr[void], size: usize, align: usize): Result[ptr[u8], Error]
        
        ; Deallocate previously allocated block
        FN dealloc(self: ptr[void], ptr: ptr[u8], size: usize, align: usize): void
        
        ; Reallocate (optional default)
        FN realloc(self: ptr[void], ptr: ptr[u8], old_size: usize, new_size: usize, align: usize): Result[ptr[u8], Error]
            VAR new_ptr = self.alloc(new_size, align)
            MATCH new_ptr
                CASE Ok(p) => 
                    memmove(p, ptr, min(old_size, new_size))
                    self.dealloc(ptr, old_size, align)
                    RETURN Ok(p)
                CASE Err(e) => RETURN Err(e)
            ENDMATCH
        ENDFN
    ENDTRAIT

    ; --- Global Allocator Registry ---

    VAR _global_allocator: Option[ptr[Allocator]] = None

    FN set_global_allocator(alloc: ptr[Allocator]): void
        _global_allocator = Some(alloc)
    ENDFN

    FN get_global_allocator(): ptr[Allocator]
        MATCH _global_allocator
            CASE Some(a) => a
            CASE None => panic("no global allocator set")
        ENDMATCH
    ENDFN

    ; --- Convenience Functions ---

    FN alloc(size: usize, align: usize): Result[ptr[u8], Error]
        get_global_allocator().alloc(size, align)
    ENDFN

    FN dealloc(ptr: ptr[u8], size: usize, align: usize): void
        get_global_allocator().dealloc(ptr, size, align)
    ENDFN

    FN realloc(ptr: ptr[u8], old_size: usize, new_size: usize, align: usize): Result[ptr[u8], Error]
        get_global_allocator().realloc(ptr, old_size, new_size, align)
    ENDFN

    ; --- Box / Own Helpers ---

    FN box_new[T](val: T): [own[T], Error]
        VAR size = sizeof(T)
        VAR align = alignof(T)
        VAR ptr = alloc(size, align)
        MATCH ptr
            CASE Ok(p) => 
                memcpy(p, &val, size)
                RETURN Ok(own::from_raw(p))
            CASE Err(e) => RETURN Err(e)
        ENDMATCH
    ENDFN

ENDMODULE
```

### MACHINE Implementation (`src/machine/alloc.fsl`)

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

    ; Auto-initialize on first use
    VAR _libc_allocator: LibcAllocator = LibcAllocator {}

    FN init_global_allocator(): void
        CORE.alloc.set_global_allocator(&_libc_allocator)
    ENDFN

ENDMODULE
```

---

## Acceptance Criteria

- [ ] `Write` trait with default methods works
- [ ] `format!`/`writeln!` macros expand to `Format::fmt` calls
- [ ] Built-in `Format` impls for primitives compile
- [ ] `panic()` writes to stderr and aborts
- [ ] `assert!`/`assert_eq!` macros expand correctly
- [ ] `Allocator` trait with global registry works
- [ ] `MACHINE.alloc` provides libc implementation
- [ ] All integration tests pass