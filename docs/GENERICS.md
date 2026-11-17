# Generics Design — Carnax (FOSSIL)

Status: Design draft
Target: Stage One (LLVM-backed compiler)

---

## 1. Goals

- Compile-time monomorphization (like Rust/C++ templates)
- Zero-cost abstraction — no runtime overhead
- Minimal syntax — leverage existing type syntax
- Support for `Vec[T]`, `String`, `Map[K, V]`, custom types

---

## 2. Syntax

### 2.1 Generic Type Parameters

```relic
; Function with generic parameter
FN identity[T](x: T): T
    RETURN x
ENDFN

; Struct with generic parameter
STRUCT Vec[T]
    ptr: ptr[T]
    len: usize
    cap: usize
ENDSTRUCT

; Multiple parameters
STRUCT Map[K, V]
    ...
ENDSTRUCT

; Constraints (optional, future)
FN sort[T: Comparable](arr: ptr[T], n: usize): void
    ...
ENDFN
```

### 2.2 Instantiation

```relic
FN main(): i32
    VAR x: i32 = identity[i32](42)
    VAR v: Vec[i32] = Vec[i32].new()
    ...
ENDFN
```

Or implicit inference:
```relic
VAR x: i32 = identity(42)  ; T inferred as i32
```

---

## 3. Implementation Strategy

### 3.1 Monomorphization Pass

After semantic analysis, before codegen:
1. Collect all generic definitions (functions, structs)
2. Find all concrete instantiations (explicit or inferred)
3. Generate specialized versions for each concrete type combination
4. Replace generic references with specialized ones

### 3.2 Naming Scheme

```
identity -> identity__i32, identity__bool, identity__Vec_i32
Vec -> Vec__i32, Vec__bool, Vec__ptr_u8
Map -> Map__i32__bool, Map__u64__String
```

### 3.3 Codegen

Specialized versions are regular functions/structs — no special handling needed.

---

## 4. Minimal Viable Generics (Stage 1)

| Feature | Included? |
|---------|-----------|
| Generic functions | Yes |
| Generic structs | Yes |
| Multiple type params | Yes |
| Type constraints/traits | No (deferred) |
| Generic enums | Yes |
| Default type params | No |
| Variadic generics | No |
| Higher-kinded types | No |

---

## 5. Interaction with Other Features

| Feature | Interaction |
|---------|-------------|
| Ownership | `own[ptr[T]]` works with generic T |
| Extern | `EXTERN FN malloc[T](size: usize): ptr[T]` |
| Inline asm | Generic functions can contain asm |
| Unions | `Result[T, E]` = `[T, E]` works |
| Arrays | `[T; N]` where T is generic |

---

## 6. Open Decisions

1. **Explicit vs implicit instantiation** — Require `identity[i32](x)` or allow `identity(x)`?
2. **Monomorphization timing** — Separate pass after semantic, or during codegen?
3. **Duplicate elimination** — Share identical specializations across modules?
4. **Recursive generics** — `Vec[Vec[i32]]` support?
5. **Generic constants** — `FN foo[N: usize]()` for array sizes?

---

## 7. Implementation Order

1. **Parser** — Generic syntax on fn/struct/enum, instantiation syntax
2. **AST** — GenericParam, GenericType, GenericInstance
3. **Semantic** — Collect generic defs, type-check generic bodies, track instantiations
4. **Monomorphization pass** — Generate specialized versions
5. **Codegen** — Emit specialized versions as regular items