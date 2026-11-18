# Phase 3: Trait System

## Goal
Implement traits (interfaces) with vtable-based dynamic dispatch. Required for `Write`, `Format`, `Allocator`.

---

## 3.1 AST Extensions (`src/ast.rs`)

```rust
#[derive(Debug, Clone)]
pub enum Item {
    // ... existing ...
    TraitDecl(TraitDecl),
    ImplDecl(ImplDecl),
}

#[derive(Debug, Clone)]
pub struct TraitDecl {
    pub name: String,
    pub generic_params: Vec<GenericParam>,  // TRAIT Foo[T]
    pub methods: Vec<TraitMethod>,
    pub supertraits: Vec<String>,           // TRAIT Foo : Bar
}

#[derive(Debug, Clone)]
pub struct TraitMethod {
    pub name: String,
    pub generic_params: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub return_type: Type,
    pub has_body: bool,                     // default impl
    pub body: Option<Block>,
}

#[derive(Debug, Clone)]
pub struct ImplDecl {
    pub trait_name: String,
    pub for_type: Type,                     // IMPL Foo FOR Bar
    pub generic_params: Vec<GenericParam>,  // IMPL [T] Foo FOR Bar[T]
    pub methods: Vec<Function>,             // method bodies
}
```

### Type System
- Add `TraitObject(trait_name)` to `Type` enum
- Trait objects are fat pointers: `{ data: ptr[void], vtable: ptr[void] }`

---

## 3.2 Parser Changes (`src/parser.rs`)

### Grammar
```
TRAIT Name [GenericParams]
    FN method_name(params): return_type ENDFN
    FN method_name(params): return_type { body } ENDFN  ; default impl
ENDTRAIT

IMPL [GenericParams] TraitName FOR Type
    FN method_name(params): return_type { body } ENDFN
ENDIMPL
```

### Parse Functions
- `parse_trait_decl()` → `Item::TraitDecl`
- `parse_impl_decl()` → `Item::ImplDecl`

---

## 3.3 Semantic Analysis (`src/semantic.rs`)

### Data Structures
```rust
struct TraitDef {
    name: String,
    generic_params: Vec<String>,
    methods: Vec<TraitMethodSig>,
    supertraits: Vec<String>,
}

struct TraitMethodSig {
    name: String,
    generic_params: Vec<String>,
    params: Vec<(String, Ty, bool)>,  // (name, type, is_own)
    return_ty: Ty,
    has_default: bool,
}

struct ImplDef {
    trait_name: String,
    for_ty: Ty,
    generic_params: Vec<String>,
    methods: HashMap<String, FnSignature>,  // concrete signatures
}
```

### Checks
1. **Trait coherence**: One impl per (trait, type) pair
2. **Method signatures**: Impl methods must match trait (after generic subst)
3. **Supertraits**: Impl must also satisfy supertraits
4. **Trait objects**: Only object-safe traits (no `Self` in return, no generics)

### Trait Resolution
```rust
fn resolve_trait_method(&self, trait_name: &str, for_ty: &Ty, method: &str) -> Option<FnSignature>
```

---

## 3.4 Codegen (`src/codegen.rs`)

### VTable Layout
```
%vtable.Write = type {
    i8*,                          ; destructor (optional)
    i64 (i8*, %slice.u8)*,        ; write
    i64 (i8*, %Str)*,             ; write_str
    void (i8*)*,                  ; flush
}
```

### Trait Object = Fat Pointer
```
%Write = type { i8*, %vtable.Write* }
```

### Impl Codegen
For each `IMPL Write FOR MyType`:
1. Generate vtable global constant
2. Generate wrapper functions that cast `i8*` to concrete type
3. Store vtable address in type metadata

### Dynamic Dispatch
```llvm
; call write on trait object %writer : %Write
%data = extractvalue %Write %writer, 0
%vtable = extractvalue %Write %writer, 1
%write_fn = getelementptr %vtable.Write, %vtable, 0, 1
%fn_ptr = load %write_fn
call i64 %fn_ptr(i8* %data, %slice.u8 %buf)
```

---

## 3.5 Trait Examples

### Write Trait (`core.io`)
```fsl
TRAIT Write
    FN write(self: ptr[void], buf: slice[u8]): Result[usize, Error]
    FN write_str(self: ptr[void], s: Str): Result[usize, Error]
    FN flush(self: ptr[void]): Result[void, Error]
ENDTRAIT
```

### Implementation (`machine.io`)
```fsl
IMPL CORE.io.Write FOR MACHINE.Conout.Handle
    FN write(self: ptr[void], buf: slice[u8]): Result[usize, Error]
        VAR h = _get_stdout()
        VAR written: u32 = 0
        VAR res = WriteConsoleA(h, buf.ptr, buf.len as u32, &written, null)
        IF res != 0
            RETURN Ok(written as usize)
        ENDIF
        RETURN Err(CORE.err.error_io("WriteConsoleA failed"))
    ENDFN
ENDIMPL
```

### Usage
```fsl
FN write_all(w: ptr[CORE.io.Write], buf: slice[u8]): Result[usize, Error]
    VAR written: usize = 0
    WHILE written < buf.len
        VAR res = w.write(buf.slice(written, buf.len))
        MATCH res
            CASE Ok(n) => written = written + n
            CASE Err(e) => RETURN Err(e)
        ENDMATCH
    ENDWHILE
    RETURN Ok(written)
ENDFN
```

---

## 3.6 Compiler Implementation Order

1. **AST + Parser**: Add `TraitDecl`, `ImplDecl`, parse syntax
2. **Semantic**: Collect trait/impl declarations, check coherence
3. **Semantic**: Resolve trait method calls, build vtable layout
4. **Codegen**: Emit vtables, trait objects, dynamic dispatch
5. **Built-in traits**: Define `Write`, `Format`, `Allocator` in CORE

---

## Acceptance Criteria
- [ ] `TRAIT ... ENDTRAIT` parses and type-checks
- [ ] `IMPL Trait FOR Type ... ENDIMPL` works
- [ ] Dynamic dispatch via trait objects works
- [ ] Multiple traits per type supported
- [ ] Generic impls: `IMPL [T] Foo FOR Bar[T]` work
- [ ] Supertraits enforced
- [ ] `core.io.Write` trait compiles and can be implemented