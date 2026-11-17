# Import System Design — Carnax (FOSSIL)

Status: Design draft
Target: Stage One (LLVM-backed compiler)

---

## 1. Goals

- Modular code organization without a heavy package manager
- Explicit dependencies, no implicit search paths
- **`MACHINE` as the standard library root** — platform capabilities exposed via `MACHINE::*`
- Compatible with `MACHINE` / `NO_MACHINE` split
- Works with generics and ownership system
- Minimal syntax

---

## 2. Design: `MACHINE` as Standard Library Root

### 2.1 Import Syntax

```relic
; Import from MACHINE standard library
USE MACHINE::IO
USE MACHINE::MEM
USE MACHINE::FMT
USE MACHINE::OS

; Import with alias
USE MACHINE::IO AS io

; Import specific items (future)
USE MACHINE::IO :: { puts, putchar }
```

### 2.2 Path Resolution

| Prefix | Meaning |
|--------|---------|
| `MACHINE::` | Standard library (always available in `MACHINE` mode) |
| `./` or relative | User project files (relative to importing file) |
| `/` absolute | From project root (set by `-I` flag) |

### 2.3 Example

```relic
; main.fsl
MACHINE

USE MACHINE::IO
USE MACHINE::MEM
USE "./local_types.fsl" AS local

FN main(): i32
    MACHINE::IO.puts("Hello\n")
    MACHINE::MEM.memset(0, buffer, 100)
    RETURN 0
ENDFN
```

---

## 3. Standard Library Structure (`MACHINE::*`)

```
MACHINE::                    ; Root namespace (requires MACHINE mode)
  IO.fsl        ; puts, putchar, stdin/stdout/stderr
  MEM.fsl       ; memcpy, memset, memcmp, memmove
  FMT.fsl       ; printf-style formatting
  OS.fsl        ; exit, env, args, getpid
  
  SEQ.fsl       ; Sequence utilities (sort, search, iter)
  MATH.fsl      ; Math functions (sin, cos, sqrt, etc.)
  
  ALLOC/          ; Heap allocation (requires heap)
    ALLOCATOR.fsl ; Allocator interface
    BUMP.fsl    ; Bump allocator
    MALLOC.fsl  ; malloc/free wrapper
  
  COLLECTIONS/    ; Data structures (some require allocator)
    VEC.fsl     ; Vec[T] (requires ALLOC)
    SLICE.fsl   ; Slice[T] (no alloc, ptr + len)
    STRING.fsl  ; String (owned UTF-8, requires ALLOC)
    MAP.fsl     ; Map[K, V] (requires ALLOC)
    QUEUE.fsl   ; Queue[T]
    RING.fsl    ; Ring buffer
  
  PLATFORM/       ; Platform-specific (conditional)
    LINUX.fsl   ; Linux syscalls, /proc, etc.
    WINDOWS.fsl ; Win32 API
    UEFI.fsl    ; UEFI services
```

### 3.1 `NO_MACHINE` Mode

```relic
NO_MACHINE

; MACHINE::* NOT available
; Only relative imports allowed
USE "./hardware/uart.fsl"
USE "./memory/mmap.fsl"

FN entry(): void
    ; Bare metal setup
ENDFN
```

---

## 4. Module System

### 4.1 Every File is a Module

- No `MODULE` keyword
- Top-level items (fn, struct, enum, const) = public API
- Use naming conventions for privacy: `_private_fn()`

### 4.2 Relative Imports (User Code)

```relic
USE "./types.fsl"
USE "../common/utils.fsl" AS utils
USE "drivers/uart.fsl"
```

### 4.3 Re-export Pattern

```relic
; collections/relic
USE "./vec.fsl"
USE "./slice.fsl"
USE "./string.fsl"
```

---

## 5. no_heap Compatibility

```relic
; In no_heap mode (MACHINE + no_heap flag):
USE MACHINE::IO          ; OK
USE MACHINE::SLICE       ; OK (no allocation)
USE MACHINE::MATH        ; OK
USE MACHINE::ALLOC       ; ERROR: requires heap
USE MACHINE::COLLECTIONS::VEC  ; ERROR: requires allocator
```

Semantic analyzer validates:
- Imported module's `no_heap` attribute
- Reject if incompatible with current mode

---

## 6. Conditional Imports (Future)

```relic
; Platform-specific
#[cfg(target_os = "linux")]
USE MACHINE::PLATFORM::LINUX

#[cfg(target_os = "windows")]  
USE MACHINE::PLATFORM::WINDOWS
```

Requires `cfg` attribute system (deferred to Stage 2).

---

## 6. Compiler Implementation

### 6.1 Import Resolution Pass

After parsing, before semantic analysis:
1. Build dependency graph from `USE` statements
2. Detect cycles (error)
3. Topological sort
4. Parse all imported files
5. Merge ASTs

### 6.2 `MACHINE::` Resolution

- Built-in namespace, no filesystem lookup
- Compiler has built-in index of `MACHINE::*` modules
- Source embedded in compiler or loaded from stdlib directory

### 6.3 Symbol Resolution

- `MACHINE::IO::puts` — qualified access
- `IO::puts` — if `USE MACHINE::IO` (module alias)
- `puts` — if `USE MACHINE::IO :: { puts }` (direct import)

---

## 7. Standard Library Implementation

### 7.1 `MACHINE::IO`

```relic
; MACHINE/IO.fsl
EXTERN FN putchar(c: i32): i32 ENDFN
EXTERN FN puts(s: ptr[u8]): i32 ENDFN

FN putchar(c: i32): i32
    RETURN putchar(c)
ENDFN

FN puts(s: ptr[u8]): i32
    RETURN puts(s)
ENDFN

VAR stdin: ptr[u8] = ...   ; from platform
VAR stdout: ptr[u8] = ...
VAR stderr: ptr[u8] = ...
```

### 7.2 `MACHINE::ALLOC`

```relic
; MACHINE/ALLOC/ALLOCATOR.fsl
STRUCT Allocator
    allocate_fn: ptr[FN(usize, usize): ptr[u8]]
    deallocate_fn: ptr[FN(ptr[u8], usize, usize): void]
    user_data: ptr[u8]
ENDSTRUCT

FN bump_allocator_init(buffer: ptr[u8], size: usize): Allocator
    ...
ENDFN
```

---

## 8. Build Integration

### 8.1 Compiler Flags

```
carnax build main.fsl -I ./src --stdlib /usr/local/lib/relic
```

### 8.2 Project Config (`relic.toml`)

```toml
[project]
name = "myapp"
version = "0.1.0"

[target]
machine = true          ; or false for NO_MACHINE
no_heap = false

[stdlib]
path = "/usr/local/lib/relic"  ; optional override
```

---

## 8. Open Decisions

1. **`MACHINE::` vs `STD::`** — `MACHINE::` ties stdlib to platform mode (explicit), `STD::` is more familiar
2. **Implicit prelude** — Auto-import `MACHINE::IO`, `MACHINE::MEM`?
3. **Cyclic imports** — Hard error (detected at resolution)
4. **Private items** — Naming convention only (`_private_fn()`)
5. **Inline imports** — `USE MACHINE::IO :: { puts }` syntax

---

## 9. Implementation Order

1. **Lexer/Parser** — `USE MACHINE::PATH`, `USE "./relative"`
2. **Import Resolution** — Graph, cycle detection, `MACHINE::` built-in index
3. **Symbol Table** — Module namespaces, qualified lookup
4. **Semantic** — Cross-module type checking, `no_heap` validation
5. **Codegen** — Multi-file LLVM emission + linking
6. **CLI** — `-I` flag, `--stdlib`, `relic.toml`
7. **Stdlib** — Implement `MACHINE::IO`, `MACHINE::MEM`, `MACHINE::ALLOC`, `MACHINE::SLICE`