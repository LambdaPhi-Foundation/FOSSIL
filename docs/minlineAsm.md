# Inline Assembly Design — Carnax

Status: Design draft
Target: Stage One (LLVM-backed compiler)

This document defines inline assembly in Carnax: the `DO ... ENDASM` block form and the
inline assembly expression/statement forms. It is one of the machine-facing capabilities
listed in `Carnax.md` section 10.2 and is only meaningful in `MACHINE` mode (or in
`NO_MACHINE` when the target architecture is known).

## 1. Design goals

- Direct access to machine instructions, registers, and memory-mapped I/O.
- Explicit interface between Carnax values and assembly operands.
- No hidden behavior: clobbers, volatility, and side effects must be declared.
- Portable syntax (architecture-independent), architecture-specific semantics.
- Safe-by-default: the compiler may reject or warn on constructs it cannot verify
  (e.g. clobbering a register that holds a live Carnax value), but the programmer can
  override with explicit declarations.

## 2. Syntax

### 2.1 `DO ... ENDASM` block (statement-level)

```relic
DO
    ASM "mov %0, %1" : "=r"(out) : "r"(in) : "rax"
ENDASM
```

The block contains one or more `ASM` statements. Each `ASM` statement has:

```
ASM <template> : <outputs> : <inputs> : <clobbers> : <flags>
```

- `<template>`: assembly string literal, Carnax-specific placeholder syntax (see 3.1).
- `<outputs>`: zero or more output operands.
- `<inputs>`: zero or more input operands.
- `<clobbers>`: zero or more clobbered registers or memory.
- `<flags>`: optional flags modifier (`volatile`, `intel`, `att`, `noreturn`).

Colons separate operand groups; empty groups are written as nothing between colons.

### 2.2 Inline assembly expression (expression-level)

Assembly that produces a value can appear where an expression is expected:

```relic
VAR result: i32 = ASM "rdtsc" : "=a"(lo), "=d"(hi) : : "rcx";
```

### 2.3 `GOTO` label form (optional, deferred)

A computed-goto form (`ASM ... GOTO labels`) is deferred to a later stage because it
interacts with control-flow validation and ownership drop placement.

## 3. Operand syntax

### 3.1 Placeholders

Carnax uses numbered placeholders in the template:

```
"mov %0, %1"
```

`%0`, `%1`, … refer to operand indices in declaration order (outputs first, then inputs).

### 3.2 Constraint strings

Each operand is a constraint string plus a Carnax expression:

```
"=r"(variable)    ; output, general-purpose register
"r"(variable)     ; input, general-purpose register
"m"(variable)     ; memory
"i"(42)           ; immediate integer
"n"(42)           ; immediate integer (known range)
"f"(variable)     ; floating-point register
"X"(variable)     ; any operand
"p"(variable)     ; pointer/memory address
```

Modifier prefixes on output constraints:

- `=` : write-only output (default for outputs)
- `+` : read-write (input and output)
- `&` : early-clobber (output written before inputs are consumed)

### 3.3 Carnax type → constraint mapping

The compiler may infer constraints from Carnax types when the programmer omits them:

| Carnax type | Default constraint |
|-------------|-------------------|
| `i8`/`u8`   | `"r"` (byte register) |
| `i16`/`u16` | `"r"` |
| `i32`/`u32` | `"r"` |
| `i64`/`u64` | `"r"` |
| `f32`/`f64` | `"f"` |
| `bool`      | `"r"` |
| `ptr[T]`    | `"r"` |
| `own[ptr[T]]` | `"r"` (the pointer value, not the pointee) |

Omitting constraints entirely (`"(variable)"`) is allowed; the compiler infers from type
but may emit a warning if inference is ambiguous.

## 4. Volatility

By default, an `ASM` block is treated as **volatile** (side effects, not optimizable
away). The `volatile` flag is explicit:

```relic
ASM "hlt" ::: "volatile"
```

A non-volatile block (pure computation with declared outputs) may be reordered or removed
by LLVM optimizations if its outputs are unused:

```relic
ASM "bswap %0" : "+r"(value) :::   ; non-volatile, optimizable
```

If `volatile` is not specified, the block defaults to `volatile` for safety. The
programmer must opt into non-volatile with an empty flags list:

```relic
ASM "nop" ::::   ; non-volatile (fourth colon group present, empty)
```

Wait — four colons? Let me clarify: the syntax is

```
ASM template : outputs : inputs : clobbers : flags
```

A trailing empty flags group means non-volatile. Omitting the fifth group entirely means
volatile (default).

## 5. Clobbers

Clobbered registers must be listed so the compiler avoids allocating Carnax values there:

```relic
ASM "cpuid" : "=a"(eax), "=b"(ebx), "=c"(ecx), "=d"(edx) : "a"(leaf) : "rbx", "rcx", "rdx"
```

Special clobbers:

- `"memory"` : the assembly reads or writes memory unpredictably.
- `"cc"` : condition codes are modified.
- `"flags"` : same as `"cc"`.

If `"memory"` is clobbered, the compiler emits a compiler barrier (no load/store motion
past the asm block).

## 6. Labels and jumps inside asm

Labels inside assembly are local to the asm string and cannot be targeted by Carnax code.
Carnax labels cannot be jumped into from assembly. This restriction keeps ownership drop
placement well-defined.

## 7. Interaction with ownership

Inline assembly that reads or writes memory pointed to by a Carnax `own[ptr[T]]` does **not**
transfer ownership. The programmer is responsible for ensuring:

- An `own` value is not passed to assembly that frees it without updating Carnax state.
- Assembly does not read memory after the owner has been dropped at scope exit.
- If assembly writes to memory owned by Carnax, the program must ensure no aliasing
  violations occur.

The compiler performs **no** ownership analysis inside the assembly string. It is the
programmer's responsibility.

## 8. Architecture and portability

Assembly strings are architecture-specific. The compiler:

- Does not validate instruction syntax (passes through to LLVM/backend).
- Does not validate constraint strings against the target (passes through to LLVM).
- May reject constraint strings it knows are invalid for the current target triple.

To write portable code, use target-specific source files or conditional compilation
(deferred to the bootstrapped stage package system).

## 9. Examples

### 9.1 Read the timestamp counter (x86-64)

```relic
VAR lo: i32 = 0
VAR hi: i32 = 0

DO
    ASM "rdtsc" : "=a"(lo), "=d"(hi) : : "rcx"
ENDASM
```

### 9.2 Memory-mapped I/O write (ARM)

```relic
VAR reg: ptr[u32] = cast[ptr[u32]](0x40020000)

DO
    ASM "str %1, [%0]" : : "r"(reg), "r"(0x01) : "memory"
ENDASM
```

### 9.3 Halt the CPU

```relic
DO
    ASM "hlt" ::: "volatile"
ENDASM
```

### 9.4 Read-write operand

```relic
VAR x: i32 = 5

DO
    ASM "add %0, 1" : "+r"(x) ::: "volatile"
ENDASM
```

### 9.5 Inline assembly expression

```relic
VAR ticks: i64 = ASM "rdtsc" : "=a"(lo), "=d"(hi) : : "rcx"
// combine lo/hi into i64 (example only; actual combining needs shifts)
```

Note: multi-register outputs from an expression form require the compiler to pack them
into a single return value; this is supported only when the output constraints map cleanly
to one Carnax type.

## 10. Lowering to LLVM

Each `ASM` block lowers to an LLVM `asm` inline assembly instruction:

```
%result = call i32 asm sideeffect "mov $0, $1", "=r,r"(i32 %in)
```

LLVM handles constraint validation, register allocation, and clobber tracking. The
compiler maps Carnax constraints to LLVM constraint letters.

For `no_heap` and `NO_MACHINE`, inline assembly works identically — it does not require
any runtime support.

## 11. Open decisions

1. **Placeholder syntax**: `%0`/`%1` vs `$0`/`$1` vs `{0}`/`{1}`. Decide before freezing
   the syntax.
2. **Named operands**: `"=r"(out:name)` for named references in the template.
3. **Multiple ASM statements per block**: allowed, but each is independent; no control
   flow between them inside the block.
4. **`noreturn` asm blocks**: mark as `noreturn` so the compiler knows control never
   reaches the following code (relevant for drop placement).
5. **Mixed syntax**: allow `att` vs `intel` syntax per-block via flags.

## 12. Relationship to `MACHINE` / `NO_MACHINE`

- `MACHINE`: inline assembly is fully supported; the target architecture is known.
- `NO_MACHINE`: inline assembly is fully supported; often *more* important here because
  there is no library abstraction between the program and the hardware.
- Neither mode requires a runtime for inline assembly.
