# Allocator Interface — Carnax (FOSSIL)

Status: Design draft
Target: Stage One (LLVM-backed compiler)

---

## 1. Goals

- Allocator as a **library interface**, not builtin
- Implemented via **inline assembly** (syscalls on Linux, HeapAlloc on Windows)
- Compatible with `no_heap` mode (compile-time rejection if used)
- Multiple allocators supported (bump, freelist, malloc wrapper)

---

## 2. Interface

```relic
; core/alloc.fsl

; Allocator trait (vtable-based for now, generics later)
STRUCT Allocator
    allocate_fn: ptr[FN(usize, usize): ptr[u8]]  ; size, align -> ptr
    deallocate_fn: ptr[FN(ptr[u8], usize, usize): void]  ; ptr, size, align
    user_data: ptr[u8]  ; allocator-specific state
ENDSTRUCT

; Global default allocator (set at startup)
VAR global_allocator: Allocator

; Convenience functions
FN allocate(alloc: ptr[Allocator], size: usize, align: usize): ptr[u8]
    RETURN alloc.allocate_fn(size, align)
ENDFN

FN deallocate(alloc: ptr[Allocator], ptr: ptr[u8], size: usize, align: usize): void
    alloc.deallocate_fn(ptr, size, align)
ENDFN

; Default allocator implementations
FN bump_allocator_init(buffer: ptr[u8], size: usize): Allocator
    ...
ENDFN

FN malloc_allocator(): Allocator
    ...
ENDFN
```

---

## 3. Inline Assembly Implementations

### 3.1 Linux (x86_64) - mmap/munmap

```relic
FN sys_mmap(addr: ptr[u8], len: usize, prot: i32, flags: i32, fd: i32, off: i64): ptr[u8]
    ASM
        mov rax, 9        ; mmap syscall
        mov rdi, `addr`
        mov rsi, `len`
        mov rdx, `prot`
        mov r10, `flags`
        mov r8, `fd`
        mov r9, `off`
        syscall
        mov `result`, rax
    ENDASM CLobbers("rax", "r10", "rcx", "r11", "memory") Flags("volatile")
    RETURN result
ENDFN

FN sys_munmap(addr: ptr[u8], len: usize): i32
    ASM
        mov rax, 11       ; munmap syscall
        mov rdi, `addr`
        mov rsi, `len`
        syscall
        mov `result`, rax
    ENDASM CLobbers("rax", "rcx", "r11", "memory") Flags("volatile")
    RETURN result
ENDFN
```

### 3.2 Windows - HeapAlloc/HeapFree

```relic
EXTERN FN GetProcessHeap(): ptr[u8] ENDFN
EXTERN FN HeapAlloc(heap: ptr[u8], flags: u32, size: usize): ptr[u8] ENDFN
EXTERN FN HeapFree(heap: ptr[u8], flags: u32, ptr: ptr[u8]): i32 ENDFN
```

---

## 4. no_heap Mode

```relic
; Semantic analyzer rejects:
; - Calls to allocate/deallocate
; - Instantiation of Allocator
; - Use of Vec/String/Map (library types requiring allocator)
```

---

## 5. String and Vec Using Allocator

```relic
; core/string.fsl
STRUCT String
    ptr: ptr[u8]
    len: usize
    cap: usize
    alloc: ptr[Allocator]
ENDSTRUCT

FN String.new(alloc: ptr[Allocator]): String
    RETURN String { ptr: 0, len: 0, cap: 0, alloc: alloc }
ENDFN

FN String.push_str(self: ptr[String], s: ptr[u8], len: usize): void
    IF self.len + len > self.cap
        self.grow(self.len + len)
    ENDIF
    ; memcpy
    ...
ENDFN

; core/vec.fsl
STRUCT Vec[T]
    ptr: ptr[T]
    len: usize
    cap: usize
    alloc: ptr[Allocator]
ENDSTRUCT

FN Vec[T].new(alloc: ptr[Allocator]): Vec[T]
    RETURN Vec[T] { ptr: 0, len: 0, cap: 0, alloc: alloc }
ENDFN
```

---

## 5. Dependencies

| Requirement | Status |
|-------------|--------|
| Generics | Needed for `Vec[T]`, `String` (if generic over allocator) |
| Inline ASM | Needed for syscall-based allocators |
| EXTERN | Needed for Windows HeapAlloc/Free |
| no_heap mode | Needed to reject allocator usage |

---

## 6. Implementation Order

1. **Inline ASM** — ASM/ENDASM blocks
2. **Allocator interface** — core/alloc.fsl (using EXTERN + ASM)
3. **Generics** — For `Vec[T]`, `Map[K,V]`
4. **String/Vec/Map** — Library types using allocator
5. **no_heap enforcement** — Semantic check