# LLVM Lowering Design — Carnax

Status: Design draft
Target: Stage One (LLVM-backed compiler)

This document defines how Carnax AST + semantic analysis output is lowered to LLVM IR. It is the bridge between `semantic.rs` (typed, ownership-checked AST) and object-file emission.

## 1. Guiding principles

- LLVM is a code-generation backend, **not** the definition of Carnax semantics.
- Carnax types map to LLVM types; Carnax control flow maps to LLVM basic blocks and branches.
- `no_heap` and `MACHINE`/`NO_MACHINE` are handled by linker/startup configuration, not by emitting implicit runtime calls.
- Ownership cleanup (drop on scope exit) is emitted explicitly by the backend; LLVM does not know about Carnax ownership.

## 2. Target setup

### 2.1 Target triple and data layout

The compiler accepts a target triple (e.g. `x86_64-pc-windows-msvc`, `aarch64-linux-gnu`) and derives a data layout from LLVM's default for that triple. No Carnax-specific layout assumptions are baked in.

```
Target triple:  <user-specified>
Data layout:    <LLVM default for triple>
```

### 2.2 Calling convention

Default: platform C calling convention (`ccc`). Machine-specific calling conventions (e.g. `fastcc`, `arm_aapcs_vfp`) may be selected per-function via a future attribute; not part of stage one.

## 3. Type mapping

Carnax primitive and aggregate types lower to LLVM IR types as follows:

| Carnax | LLVM IR | Notes |
|--------|---------|-------|
| `bool` | `i1` | |
| `i8` | `i8` | |
| `i16` | `i16` | |
| `i32` | `i32` | |
| `i64` | `i64` | |
| `u8` | `i8` | unsigned is a signedness property of ops, not the type |
| `u16` | `i16` | |
| `u32` | `i32` | |
| `u64` | `i64` | |
| `f32` | `float` | |
| `f64` | `double` | |
| `void` | `void` | |
| `ptr[T]` | `T*` | opaque/raw pointer |
| `own[ptr[T]]` | `T*` | same representation as `ptr[T]`; drop inserted at scope exit |
| `Named(S)` | `%struct.S` | opaque struct type, body defined by struct declaration |
| `[T; N]` | `[N x T]` | fixed array |
| `[T1, T2]` (tuple) | `{ T1, T2 }` | LLVM struct |
| Function type | `Ret (P1, P2, ...)` | function pointer type |

### 3.1 Struct layout

Struct fields are laid out in declaration order. LLVM structs are **packed by default** unless the spec later requires target-native alignment. Padding is not inserted automatically; if the ABI requires it, the struct type body must include explicit padding fields.

```
%struct.Point = type { i32, i32 }
```

### 3.2 Enum representation

Tagged enums lower to `{ i32, <payload> }` where the first member is the tag discriminant. Payload-less variants use a zero-sized placeholder (`{ i32 }` or just `i32` for the whole enum if all variants are payload-less). Tag values are assigned in declaration order starting at 0.

```
%enum.ReadResult = type { i32, i32 }   ; tag + payload (payload i32, poison for tagless variants)
```

## 4. Value representation

### 4.1 Variables

Every Carnax `VAR`/`CONST` binding allocates an **alloca** in the entry block of its enclosing function. The alloca is typed by the Carnax type. Subsequent reads/writes use `load`/`store`.

- `CONST` bindings: alloca is written once (at init) and never `store`d again. LLVM does not enforce immutability; the semantic analyzer does.
- `own[ptr[T]]` bindings: alloca holds a `T*`; a `drop` call is emitted at scope exit if the owner is initialized and not moved.

### 4.2 Parameters

Function parameters lower to LLVM function arguments. Each parameter is immediately copied into a dedicated alloca (so the function can mutate its locals uniformly). For `own` parameters, ownership transfers to the callee; the callee is responsible for drop.

### 4.3 Multiple return values

Carnax `[T1, T2]` return types lower to a single LLVM struct return `{ T1, T2 }`. The caller uses `extractvalue` to get individual components. Single-value returns lower to the scalar type directly.

```
define i32 @main() {
  ...
  ret i32 0
}

define { i32, i1 } @divide(i32, i32) {
  ...
  ret { i32, i1 } { i32 5, i1 true }
}
```

## 5. Function lowering

### 5.1 Function prologue

For each Carnax function:

1. Declare/define the LLVM function with the mapped signature.
2. Create entry basic block.
3. Insert `alloca` instructions for every local variable (including parameters copied into locals).
4. Store parameter values into their parameter allocas.
5. Emit the body.

### 5.2 Function epilogue

- Insert drop calls for any live `own` allocas at scope exit (see section 8).
- `ret <value>` or `ret void`.

### 5.3 Example

Carnax:
```
FN add(a: i32, b: i32): i32
    RETURN a + b
ENDFN
```

LLVM:
```
define i32 @add(i32 %a, i32 %b) {
entry:
  %a.alloca = alloca i32
  %b.alloca = alloca i32
  store i32 %a, ptr %a.alloca
  store i32 %b, ptr %b.alloca
  %a.val = load i32, ptr %a.alloca
  %b.val = load i32, ptr %b.alloca
  %sum = add i32 %a.val, %b.val
  ret i32 %sum
}
```

## 6. Expression lowering

All expressions produce an LLVM value (or, for lvalues, a pointer). The lowering is syntax-directed:

### 6.1 Literals

| Carnax | LLVM |
|--------|------|
| `42` | `i32 42` |
| `3.14` | `double 3.140000e+00` |
| `true` | `i1 true` |
| `'x'` | `i8 120` |
| `"hi"` | pointer to global constant string (see 10.1) |

### 6.2 Identifiers

`load` from the variable's alloca. If the variable is an `own` that has been moved, semantic analysis has already rejected the program; the backend assumes the value is valid.

### 6.3 Binary operators

Integer arithmetic uses `add`, `sub`, `mul`, `sdiv`, `srem` (signed) or `udiv`, `urem` (unsigned). The Carnax spec must define whether `/` and `%` are signed or unsigned per operand type; the default assumption is signed for `i*` and unsigned for `u*`.

Comparisons: `icmp eq/ne/slt/sgt/sle/sge` for integers; `fcmp oeq/one/olt/ogt/ole/oge` for floats.

Logical `&&`/`||` short-circuit using conditional branches (see 7.2), not `and`/`or` on `i1`, to preserve short-circuit semantics.

Bitwise: `and`, `or`, `xor`, `shl`, `lshr`/`ashr`.

### 6.4 Unary operators

| Carnax | LLVM |
|--------|------|
| `-x` | `sub 0, x` (int) or `fsub 0.0, x` (float) |
| `!x` | `xor x, true` (on `i1`) |
| `&x` | pointer to x's alloca (no instruction; the alloca pointer itself) |
| `*x` | `load` from pointer `x` |
| `~x` | `not x` |

### 6.5 Member access

`object.field` → `getelementptr` on the object pointer/value to index field `N`, then `load` if reading.

### 6.6 Indexing

`array[i]` → `getelementptr` with indices `[0, i]`, then `load`.

### 6.7 Function calls

Direct call to `@funcname` with mapped argument types. Multiple return values: the callee returns a struct; the caller `extractvalue`s components.

`own` arguments: the argument value (a pointer) is passed by value; ownership transfers, callee drops it.

## 7. Control flow lowering

### 7.1 Blocks

A Carnax block maps to a sequence of instructions in the current basic block. Variable declarations allocate in the entry block (or at point of declaration; LLVM allows alloca anywhere but entry-block allocation is simpler for the initial implementation).

### 7.2 If/Else

```
if (cond) then else endif
```

Lowers to:

```
%cond = load i1, ptr %cond.alloca   ; or i1 value directly
br i1 %cond, label %then, label %else

then:
  ...
  br label %merge

else:
  ...
  br label %merge

merge:
  ; phi nodes if needed
```

### 7.3 While

```
while (cond) body endwhile
```

```
br label %loop.head

loop.head:
  %cond = ...
  br i1 %cond, label %loop.body, label %loop.exit

loop.body:
  ...
  br label %loop.head

loop.exit:
```

### 7.4 For

`FOR i = start TO end body ENDFOR` — `start` and `end` are evaluated once. The loop variable `i` is an alloca. `start` is stored into `i`, then:

```
br label %loop.head

loop.head:
  %i.val = load i32, ptr %i
  %cmp = icmp sle i32 %i.val, %end
  br i1 %cmp, label %loop.body, label %loop.exit

loop.body:
  ...
  %i.next = add i32 %i.val, 1
  store i32 %i.next, ptr %i
  br label %loop.head
```

Inclusive bound (`<= end`) per the illustrative syntax in `Carnax.md`.

### 7.5 Return

`RETURN v1, v2` → construct the return struct (or scalar), then `ret`. Early return from nested blocks: LLVM `ret` from the current block; no unwinding (Carnax has no exceptions).

## 8. Ownership and drop

`own[ptr[T]]` is the only owning type in stage one. Lowering:

- Allocation (`ALLOC` / future builtin): a call to an allocator function (e.g. `@malloc`) returning `i8*`, bitcast to `T*`, stored in the owner alloca.
- Move (`b := a` where `a` is `own`): `store` the pointer value into `b`'s alloca; mark `a` as moved at the semantic level (backend does not track this — the semantic analyzer already validated it).
- Scope exit: if the owner is initialized and not moved, emit a call to the deallocator (e.g. `@free` or a type-specific drop function) before the block terminates.

Drop insertion points:
- End of function body.
- End of each block scope (`IF` branches, `WHILE` body, `FOR` body).
- Not on early `RETURN` within the same scope unless the scope is being exited (i.e. drop runs when leaving the block that contains the owner).

For `no_heap`: allocation calls are rejected by the semantic analyzer; the backend simply never emits `malloc` calls.

## 9. Memory allocation

### 9.1 Stack

All `VAR`/`CONST` bindings use `alloca`. No escape analysis in stage one — everything lives on the stack frame.

### 9.2 Heap

Heap allocation is a call to an external allocator. Stage one does not define a builtin `ALLOC`; allocation (if any) is a platform/library function call. The backend emits a `call @malloc(i64 <size>)` and bitcasts the result.

In `no_heap` mode, the semantic analyzer rejects any heap-allocation call, so the backend never sees one.

### 9.3 Static storage

Global variables (future) lower to `@name = global <type> <initializer>`. Not in stage one.

## 10. Strings and constants

String literals lower to global constant byte arrays with null termination:

```
@str.0 = private constant [4 x i8] c"hi\00"
```

Used as `ptr` to the first element via `getelementptr` or direct `bitcast`.

## 11. Internal IR → LLVM pipeline

The compiler has a small internal IR between semantic analysis and LLVM generation (per `Carnax.md` section 12.2). The lowering pass walks this IR:

```
Semantic AST
   |
   v
Lowering pass (this document)
   |
   v
LLVM IR (in-memory, via llvm-sys or inkwell)
   |
   v
LLVM optimization passes (optional, -O levels)
   |
   v
Object file / executable (via LLVM MCJIT or object writer + linker)
```

The lowering pass should be a single `Lower` struct holding:
- The LLVM builder/context/module.
- A mapping from Carnax variable names → LLVM `Value*` (alloca pointers).
- A mapping from Carnax function names → LLVM `Function*`.
- A mapping from Carnax struct names → LLVM `Type*`.

## 12. MACHINE / NO_MACHINE

These are **not** codegen modes in the backend. They are platform-linkage directives:

- `MACHINE`: link against platform startup/C runtime; entry point `main` (or target-specific).
- `NO_MACHINE`: freestanding; no default entry point, no standard library, user supplies startup. The backend emits the function as-is; the linker script/object format decides the entry symbol.

No LLVM IR difference between the two in stage one; the difference is in linker flags and startup object selection.

## 13. Open questions for the lowering implementation

1. **LLVM binding**: use `inkwell` (safe Rust wrappers) or `llvm-sys` (raw FFI)? Recommendation: `inkwell` for stage one productivity.
2. **Struct padding**: packed vs target-native alignment — decide before emitting structs.
3. **Drop function naming**: type-specific `@drop.S` vs generic `@free(ptr)` — decide.
4. **`own` of non-pointer types**: `own[i32]` is meaningless; the semantic analyzer should reject `own` applied to non-pointer types.
5. **No-return / unreachable**: `unreachable` for paths after `noreturn` calls (future).

## 14. Example: full lowering

Carnax (`test_valid.fsl`, simplified):

```
FN main(): i32
    VAR result: i32 = 0
    RETURN result
ENDFN
```

LLVM IR:

```
define i32 @main() {
entry:
  %result = alloca i32
  store i32 0, ptr %result
  %result.val = load i32, ptr %result
  ret i32 %result.val
}
```

## 15. Recommended implementation order

1. Set up `inkwell`/`llvm-sys` dependency; create `Lower` skeleton.
2. Emit primitive types and simple `RETUR
N` functions.
3. Add `alloca` + `load`/`store` for variables.
4. Add binary/unary expressions.
5. Add `IF`/`WHILE` control flow (basic blocks + branches).
6. Add function calls and multiple returns (struct return + extractvalue).
7. Add structs/enums/arrays (GEP + layout).
8. Add `own` drop calls at scope exit.
9. Add `no_heap` rejection (already in semantic layer).
10. Add object-file emission and linker integration.
