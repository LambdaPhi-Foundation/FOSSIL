# FOSSIL Language — Introduction & Documentation

**Version:** 0.1.0 (Stage One: LLVM-backed)  
**Compiler:** `carnax` (Rust implementation)  
**Target:** Systems programming with explicit control

---

## 1. Philosophy

FOSSIL is a **systems programming language** designed around a small set of orthogonal principles:

| Principle | Meaning |
|-----------|---------|
| **Imperative first** | Programs are procedures that operate on and modify state |
| **Explicit behavior** | No hidden allocation, implicit conversions, or surprising control flow |
| **Data-oriented** | Data layouts defined separately from procedures |
| **Minimal surface** | Few general mechanisms over many specialized constructs |
| **Systems control** | Direct access to pointers, memory layout, calling conventions |
| **Predictable compilation** | Semantics defined independently of LLVM implementation details |
| **Optional infrastructure** | Platform services and libraries not mandatory |

**FOSSIL is not:**
- A functional language (no closures, no lazy evaluation)
- An OO language (no classes, inheritance, implicit `this`)
- A language with mandatory GC or runtime
- A language that hides allocation or platform dependencies

---

## 2. Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    carnax Compiler                          │
├─────────────────────────────────────────────────────────────┤
│  Source (.fsl)                                             │
│       │                                                     │
│       ▼                                                     │
│  ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐  │
│  │ Lexer   │───▶│ Parser  │───▶│  AST    │───▶│ Semantic│  │
│  │         │    │         │    │         │    │Analysis │  │
│  └─────────┘    └─────────┘    └─────────┘    └────┬────┘  │
│                                                     │       │
│                                                     ▼       │
│                                            ┌─────────────┐  │
│                                            │  Codegen    │  │
│                                            │ (LLVM IR)   │  │
│                                            └──────┬──────┘  │
│                                                   │         │
│                                                   ▼         │
│                                            ┌─────────────┐  │
│                                            │    clang    │  │
│                                            │  (linker)   │  │
│                                            └──────┬──────┘  │
│                                                   │         │
│                                                   ▼         │
│                                            Native Binary    │
└─────────────────────────────────────────────────────────────┘
```

### Stages

| Stage | Description |
|-------|-------------|
| **Stage One (current)** | LLVM-backed compiler written in Rust; `MACHINE`/`NO_MACHINE` platform split |
| **Stage Two (future)** | Self-hosted compiler in FOSSIL; pluggable platform/package system |

---

## 3. Language Reference

### 3.1 Program Structure

```relic
MACHINE                    ; or NO_MACHINE

STRUCT Point               ; data layout
    x: i32
    y: i32
ENDSTRUCT

ENUM Result                ; sum type
    Success(i32)
    Failure(i32)
ENDENUM

EXTERN FN puts(s: ptr[u8]): i32 ENDFN  ; external C function

FN add(a: i32, b: i32): i32            ; function
    RETURN a + b
ENDFN

FN main(): i32                         ; entry point (MACHINE)
    VAR x: i32 = 10
    CONST LIMIT: i32 = 100
    RETURN add(x, LIMIT)
ENDFN
```

### 3.2 Types

| Category | Types |
|----------|-------|
| **Primitive** | `bool`, `i8`/`i16`/`i32`/`i64`, `u8`/`u16`/`u32`/`u64`, `f32`/`f64`, `void` |
| **Pointer** | `ptr[T]` — raw pointer, no ownership |
| **Owner** | `own[ptr[T]]` — unique ownership, move semantics |
| **Aggregate** | `STRUCT`, `ENUM` (with optional payload) |
| **Array** | `[T; N]` — fixed-size |
| **Tuple** | `(T1, T2, ...)` — product type, multiple return values |
| **Union** | `[T1, T2, ...]` — sum type, any one of these (planned) |

### 3.3 Variables & Assignment

```relic
VAR mutable: i32 = 0       ; mutable binding
CONST immutable: i32 = 42  ; immutable binding

mutable := 10              ; reassignment (requires VAR)
ptr := &mutable            ; address-of
val := *ptr                ; dereference
VOLATILE *mmio := 0xFF     ; volatile store
x := VOLATILE *mmio        ; volatile load
```

### 3.4 Control Flow

```relic
IF condition
    ...
ELSE
    ...
ENDIF

WHILE condition
    ...
ENDWHILE

FOR i = 0 TO 10            ; inclusive range
    ...
ENDFOR

MATCH expr
    CASE Variant
        ...
    CASE Variant(payload)
        ...
    CASE _                 ; wildcard
        ...
    ELSE                   ; optional catch-all
        ...
ENDMATCH

RETURN value               ; single return
RETURN a, b, c             ; multiple returns (tuple)
```

### 3.5 Functions

```relic
FN name(param: type, ...): return_type
    ...
ENDFN

; Multiple returns
FN div(a: i32, b: i32): (i32, bool)
    IF b == 0
        RETURN 0, FALSE
    ENDIF
    RETURN a / b, TRUE
ENDFN

; Call with tuple assignment
VAR q: i32 = 0
VAR ok: bool = FALSE
q, ok := div(10, 2)

; Tuple literal in variable declaration
VAR t: (i32, i32, i32) = (1, 2, 3)
```

### 3.6 Ownership & Moves

```relic
VAR a: own[ptr[i32]]       ; owner allocated
VAR b: own[ptr[i32]]
b := a                     ; MOVE: a is now invalid, b owns
; *a  ; ERROR: use of moved value
```

- Ownership transfer is a **move**, not a copy
- Moved-from variables cannot be read
- Scope exit drops un-moved owners (cleanup)

### 3.7 Platform Modes

```relic
MACHINE
FN main(): i32              ; or main(argc: i32, argv: ptr[ptr[u8]]): i32
    RETURN 0
ENDFN

NO_MACHINE
FN entry(): void            ; user-defined entry symbol
ENDFN
```

| Mode | Entry | Startup | Stdlib | Use Case |
|------|-------|---------|--------|----------|
| `MACHINE` | `main` (i32) | Toolchain-provided | Optional | OS applications |
| `NO_MACHINE` | User-defined | None | None | Firmware, kernels, bootloaders |

---

## 4. Machine-Facing Capabilities

### 4.1 EXTERN Functions
Call C/OS functions directly:
```relic
EXTERN FN malloc(size: usize): ptr[u8] ENDFN
EXTERN FN free(p: ptr[u8]): void ENDFN
EXTERN FN syscall(nr: i64, ...): i64 ENDFN
```

### 4.2 VOLATILE Memory Access
For memory-mapped I/O:
```relic
VOLATILE *uart_reg := 0x41     ; write
VAR c: i32 = VOLATILE *uart_reg ; read
```

### 4.3 Inline Assembly (Planned)
```relic
ASM
    mov rax, `a`
    add rax, `b`
    mov `result`, rax
ENDASM CLobbers("rax") Flags("volatile")
```

---

## 5. Building & Running

### 5.1 Compiler Commands
```bash
carnax build hello.fsl -o hello.exe    # compile to native binary
carnax run hello.fsl                   # compile, run, print exit code
carnax emit-ir hello.fsl -o hello.ll   # emit LLVM IR
```

### 5.2 Requirements
- **LLVM/Clang** in PATH (or `CLANG` / `LLVM_PATH` env vars)
- Windows: MSVC toolchain recommended
- Linux/macOS: clang or gcc-compatible toolchain

### 5.3 Example Session
```bash
> carnax run examples/01_basic_types.fsl
Exit code: 0

> carnax build examples/10_extern_volatile.fsl -o io_test.exe
Built native binary 'io_test.exe'

> io_test.exe
Hello from FOSSIL!
```

---

## 6. Example Catalog

| File | Demonstrates |
|------|--------------|
| `01_basic_types.fsl` | Variables, constants, basic arithmetic |
| `01b_basic_comments.fsl` | Line and inline comments |
| `02_functions.fsl` | Function definition, calls, parameters |
| `03_control_flow.fsl` | IF/ELSE, WHILE, FOR loops |
| `03b_control_flow_own.fsl` | FOR loop with ownership |
| `04_structs.fsl` | STRUCT definition, pointer access, methods |
| `05_enums_unit.fsl` | Unit enums, MATCH |
| `05b_enums_unit_bare.fsl` | Bare enum variants |
| `06_enums_payload.fsl` | Payload enums, MATCH with binding |
| `07_match_expressions.fsl` | MATCH with integer literals, ELSE |
| `07b_match_wildcard.fsl` | MATCH with wildcard `_` |
| `08_arrays.fsl` | Fixed-size arrays `[T; N]` |
| `09_multiple_returns.fsl` | Tuple returns, tuple assignment |
| `10_extern_volatile.fsl` | EXTERN functions, VOLATILE load/store |
| `11_recursion.fsl` | Fibonacci recursion |
| `11b_recursion_factorial.fsl` | Factorial recursion |
| `12_nested_loops.fsl` | WHILE + FOR + WHILE nesting |
| `13_early_returns.fsl` | Early RETURN from functions |
| `14_deep_nesting.fsl` | Complex expression nesting |
| `15_bitwise.fsl` | Bitwise ops, shifts, logical ops |
| `16_machine_basic.fsl` | MACHINE mode minimal main |
| `17_machine_args.fsl` | MACHINE with argc/argv |
| `18_nomachine.fsl` | NO_MACHINE freestanding entry |
| `19_inlineasm_basic.fsl` | ASM/ENDASM syntax (planned) |
| `20_misc_comprehensive.fsl` | Combined: structs, enums, tuples, control flow |

### Error Examples (`examples/errors/`)
| File | Error Type |
|------|------------|
| `01_valid.fsl` | Valid program (passes) |
| `02_undeclared.fsl` | Undeclared variable |
| `03_type_mismatch.fsl` | Type mismatch (string → i32) |
| `04_const_reassign.fsl` | CONST reassignment |
| `05_moved_owner.fsl` | Use of moved owner |
| `06_return_type.fsl` | Return type mismatch |
| `07_match_wrong_enum.fsl` | Match wrong enum type |
| `08_match_payload_mismatch.fsl` | Missing payload in match |
| `09_match_literal_on_enum.fsl` | Literal pattern on enum |
| `10_enum_unknown_variant.fsl` | Unknown enum variant |
| `11_machine_no_main.fsl` | MACHINE without main |
| `12_machine_wrong_argv.fsl` | Invalid argv type |
| `13_machine_wrong_return.fsl` | main returns non-i32 |
| `14_incomplete_var.fsl` | Incomplete variable declaration |

---

## 7. Documentation Files

| File | Description |
|------|-------------|
| `Carnax.md` | Full language design specification |
| `machine.md` | MACHINE/NO_MACHINE platform model |
| `minlineAsm.md` | Original inline assembly design |
| `INLINEASSEMBLY.md` | Implementation plan for inline asm |
| `llvm-lowering.md` | LLVM IR lowering details |
| `Status.md` | Project status tracking |
| `introdoc.md` | **This file** |

---

## 8. Current Limitations (Stage One)

- No generics / monomorphization
- No package/module system (`USE`)
- No `no_heap` mode enforcement
- No actor concurrency
- Inline assembly not yet implemented (parser only)
- Limited standard library (EXTERN for libc)
- No cross-compilation config (target triple hardcoded)

---

## 9. Contributing / Development

```bash
# Build compiler
cargo build --release

# Run integration tests
cargo test

# Compiler binary at target/release/carnax.exe
```

The compiler is written in Rust with no external dependencies beyond std + LLVM/Clang toolchain.

---

*FOSSIL: Explicit. Minimal. Systems.*