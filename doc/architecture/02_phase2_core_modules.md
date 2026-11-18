# Phase 2: CORE Modules (No Traits Required)

## Goal
Implement `core.mem`, `core.slice`, `core.err` — modules that work with current language features.

---

## 2.1 `src/core/mem.fsl` — Memory Primitives

```fsl
; CORE.mem — Memory operations as compiler intrinsics
; Lowered to LLVM intrinsics: memcpy, memset, memmove, memcmp

MODULE mem

    ; Copy n bytes from src to dest (non-overlapping)
    EXTERN FN memcpy(dest: ptr[u8], src: ptr[u8], n: usize): ptr[u8] ENDFN

    ; Copy n bytes (handles overlapping)
    EXTERN FN memmove(dest: ptr[u8], src: ptr[u8], n: usize): ptr[u8] ENDFN

    ; Set n bytes to value c
    EXTERN FN memset(dest: ptr[u8], c: i8, n: usize): ptr[u8] ENDFN

    ; Compare n bytes, return <0, 0, >0
    EXTERN FN memcmp(a: ptr[u8], b: ptr[u8], n: usize): i32 ENDFN

ENDMODULE
```

### Codegen Lowering
| Function | LLVM Intrinsic |
|----------|----------------|
| `memcpy` | `llvm.memcpy.p0.p0.i64` |
| `memmove` | `llvm.memmove.p0.p0.i64` |
| `memset` | `llvm.memset.p0.i64` |
| `memcmp` | `llvm.memcmp` (or inline loop) |

---

## 2.2 `src/core/slice.fsl` — Slice & Str Types

```fsl
; CORE.slice — Fat pointer types: slice[T] and Str
; No heap allocation, just (ptr, len) pairs

MODULE slice
    USE CORE.mem

    ; Generic slice: fat pointer (ptr + len)
    STRUCT slice[T]
        ptr: ptr[T]
        len: usize
    ENDSTRUCT

    ; String slice: immutable UTF-8 view
    STRUCT Str
        ptr: ptr[u8]
        len: usize
    ENDSTRUCT

    ; --- Construction ---

    FN slice_from_parts[T](ptr: ptr[T], len: usize): slice[T]
        RETURN slice { ptr, len }
    ENDFN

    FN str_from_parts(ptr: ptr[u8], len: usize): Str
        RETURN Str { ptr, len }
    ENDFN

    ; From array (compiler knows length)
    FN slice_from_array[T, N](arr: ptr[[T; N]]): slice[T]
        RETURN slice { ptr: arr, len: N }
    ENDFN

    ; From string literal (compiler emits .ptr and .len)
    FN str_from_literal(ptr: ptr[u8], len: usize): Str
        RETURN Str { ptr, len }
    ENDFN

    ; --- Accessors ---

    FN slice_len[T](s: slice[T]): usize
        RETURN s.len
    ENDFN

    FN slice_ptr[T](s: slice[T]): ptr[T]
        RETURN s.ptr
    ENDFN

    FN str_len(s: Str): usize
        RETURN s.len
    ENDFN

    FN str_ptr(s: Str): ptr[u8]
        RETURN s.ptr
    ENDFN

    FN str_as_slice(s: Str): slice[u8]
        RETURN slice { ptr: s.ptr, len: s.len }
    ENDFN

    ; --- Indexing (bounds-checked) ---

    FN slice_get[T](s: slice[T], i: usize): [T, Error]
        IF i >= s.len
            RETURN Err(Error { kind: ErrorKind.InvalidInput, message: "index out of bounds" })
        ENDIF
        RETURN Ok(*(s.ptr + i))
    ENDFN

    ; --- Subslice ---

    FN slice_sub[T](s: slice[T], start: usize, end: usize): [slice[T], Error]
        IF end > s.len OR start > end
            RETURN Err(Error { kind: ErrorKind.InvalidInput, message: "invalid subslice range" })
        ENDIF
        RETURN Ok(slice { ptr: s.ptr + start, len: end - start })
    ENDFN

    FN str_sub(s: Str, start: usize, end: usize): [Str, Error]
        IF end > s.len OR start > end
            RETURN Err(Error { kind: ErrorKind.InvalidInput, message: "invalid substring range" })
        ENDIF
        RETURN Ok(Str { ptr: s.ptr + start, len: end - start })
    ENDFN

    ; --- Comparison ---

    FN slice_eq[T](a: slice[T], b: slice[T]): bool
        IF a.len != b.len
            RETURN false
        ENDIF
        RETURN memcmp(a.ptr, b.ptr, a.len) == 0
    ENDFN

ENDMODULE
```

---

## 2.3 `src/core/err.fsl` — Error Types

```fsl
; CORE.err — Tagged union error types
; [T, E] = Result, [T, void] = Option

MODULE err

    ; Error kind classification
    ENUM ErrorKind
        NotFound
        PermissionDenied
        ConnectionRefused
        ConnectionReset
        ConnectionAborted
        NotConnected
        AddrInUse
        AddrNotAvailable
        BrokenPipe
        AlreadyExists
        WouldBlock
        InvalidInput
        InvalidData
        TimedOut
        Interrupted
        WriteZero
        Other
    ENDENUM

    ; Error payload
    STRUCT Error
        kind: ErrorKind
        message: Str  ; empty Str if no message
    ENDSTRUCT

    ; Option type: [T, void] (None = void)
    UNION Option[T]
        Some(T)
        None
    ENDUNION

    ; Result type: [T, E]
    UNION Result[T, E]
        Ok(T)
        Err(E)
    ENDUNION

    ; --- Constructors ---

    FN error_new(kind: ErrorKind, msg: Str): Error
        RETURN Error { kind, message: msg }
    ENDFN

    FN error_io(msg: Str): Error
        RETURN error_new(ErrorKind.Other, msg)
    ENDFN

    FN error_oom(): Error
        RETURN error_new(ErrorKind.Other, "out of memory")
    ENDFN

    ; Option constructors
    FN some[T](v: T): Option[T]
        RETURN Some(v)
    ENDFN

    FN none[T](): Option[T]
        RETURN None
    ENDFN

    ; Result constructors
    FN ok[T, E](v: T): Result[T, E]
        RETURN Ok(v)
    ENDFN

    FN err[T, E](e: E): Result[T, E]
        RETURN Err(e)
    ENDFN

    ; --- Result Combinators ---

    FN result_map[T, U, E](r: Result[T, E], f: FN(T): U): Result[U, E]
        MATCH r
            CASE Ok(v) => Ok(f(v))
            CASE Err(e) => Err(e)
        ENDMATCH
    ENDFN

    FN result_and_then[T, U, E](r: Result[T, E], f: FN(T): Result[U, E]): Result[U, E]
        MATCH r
            CASE Ok(v) => f(v)
            CASE Err(e) => Err(e)
        ENDMATCH
    ENDFN

    FN result_or_else[T, E](r: Result[T, E], f: FN(E): Result[T, E]): Result[T, E]
        MATCH r
            CASE Ok(v) => Ok(v)
            CASE Err(e) => f(e)
        ENDMATCH
    ENDFN

    FN result_unwrap[T, E](r: Result[T, E]): T
        MATCH r
            CASE Ok(v) => v
            CASE Err(e) => panic("unwrap on Err: " + e.message)
        ENDMATCH
    ENDFN

    FN result_expect[T, E](r: Result[T, E], msg: Str): T
        MATCH r
            CASE Ok(v) => v
            CASE Err(e) => panic(msg + ": " + e.message)
        ENDMATCH
    ENDFN

    ; --- Option Combinators ---

    FN option_map[T, U](o: Option[T], f: FN(T): U): Option[U]
        MATCH o
            CASE Some(v) => Some(f(v))
            CASE None => None
        ENDMATCH
    ENDFN

    FN option_and_then[T, U](o: Option[T], f: FN(T): Option[U]): Option[U]
        MATCH o
            CASE Some(v) => f(v)
            CASE None => None
        ENDMATCH
    ENDFN

    FN option_unwrap[T](o: Option[T]): T
        MATCH o
            CASE Some(v) => v
            CASE None => panic("unwrap on None")
        ENDMATCH
    ENDFN

ENDMODULE
```

---

## 2.4 Codegen Support Needed

### Tagged Union Layout
```
; Result[T, E] = { i32 tag, union { T ok; E err; } }
%struct.Result = type { i32, %union.payload }
%union.payload = type { [max(sizeof(T), sizeof(E)) x i8] }
```

### Match Lowering
```fsl
MATCH result
    CASE Ok(v) => ...
    CASE Err(e) => ...
ENDMATCH
```
→ `switch` on tag (0 = Ok, 1 = Err)

---

## Testing

```fsl
; examples/test_core_err.fsl
NO_MACHINE
USE CORE.err

FN div(a: i32, b: i32): Result[i32, Error]
    IF b == 0
        RETURN Err(error_new(ErrorKind.InvalidInput, "div by zero"))
    ENDIF
    RETURN Ok(a / b)
ENDFN

FN main(): i32
    VAR r = div(10, 2)
    MATCH r
        CASE Ok(v) => RETURN 0
        CASE Err(e) => RETURN 1
    ENDMATCH
ENDFN
```

---

## Acceptance Criteria
- [ ] `core.mem` functions lower to LLVM intrinsics
- [ ] `slice[T]` and `Str` work as fat pointers
- [ ] `slice_get` returns `[T, Error]` with bounds check
- [ ] `Result[T, E]` and `Option[T]` match expressions work
- [ ] Combinators (`map`, `and_then`, `unwrap`) compile and run
- [ ] All tests pass in `NO_MACHINE` mode