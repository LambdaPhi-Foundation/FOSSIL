# C++ Interop Plan for Carnax/FOSSIL

## Current State
- **Existing:** `EXTERN FN name(params): ret` → LLVM `declare` (C-only)
- **Gap:** No C++ name mangling, no class layout, no templates, no exceptions, no stdlib mapping

---

## Phase 1: C-Compatible C++ Interop (1-2 weeks)

### 1.1 Extended EXTERN Declaration
```fsl
EXTERN "C"   FN c_func(i32): i32      ; C linkage
EXTERN "C++" FN cpp_func(i32): i32    ; C++ mangled linkage
```

**Changes:**
- Add `linkage: Option<String>` to `ExternDecl` AST
- Parse `EXTERN "C" | "C++" FN ...`
- Codegen emits appropriate LLVM `declare`

### 1.2 Name Mangling
- Add `mangled_name: Option<String>` to `ExternDecl`
- Allow explicit: `EXTERN "C++" FN foo(i32): i32 = "_Z3fooi"`
- Provide `mangle("name", [types])` builtin

### 1.3 Calling Convention
- Add `calling_conv: Option<CallingConv>` (`cdecl`, `stdcall`, `thiscall`, etc.)

---

## Phase 2: C++ Class/Struct Interop (2-3 weeks)

### 2.1 Class Declaration
```fsl
EXTERN CLASS "MyClass" {
    FIELD _x: i32
    FIELD _y: i32
    
    CTOR(i32, i32) : ptr[MyClass]
    DTOR(ptr[MyClass]) : void
    
    METHOD get_x(ptr[MyClass]) : i32
    METHOD set_x(ptr[MyClass], i32) : void
    
    STATIC METHOD create(i32, i32) : ptr[MyClass]
}
```

**AST:** New `ExternClass` with fields, constructors, destructor, methods, static methods, base classes.

### 2.2 Layout Compatibility
- `#[repr(C++)]` attribute for FOSSIL structs
- Match C++ padding, alignment, vptr placement

### 2.3 Constructors/Destructors + Ownership
```fsl
VAR obj: ptr[MyClass] = MyClass::create(10, 20)
MyClass::dtor(obj)  ; manual

; RAII
VAR owned: own[MyClass] = MyClass::create(10, 20)  ; auto-destruct
```

### 2.4 Virtual Functions
```fsl
EXTERN CLASS "Base" {
    VIRTUAL METHOD foo(ptr[Base]) : i32
}
EXTERN CLASS "Derived" EXTENDS "Base" {
    OVERRIDE METHOD foo(ptr[Derived]) : i32
}
```
Codegen: emit vtable globals, vptr as first field, virtual call = vptr load + index + call.

---

## Phase 3: C++ Standard Library Type Mapping (2-3 weeks)

### 3.1 Type Mapping Registry
```fsl
TYPEMAP "std::string" AS Str {
    METHOD c_str(ptr[Str]) : ptr[u8]
    METHOD length(ptr[Str]) : usize
}

TYPEMAP "std::vector<T>" AS Vec<T> { ... }
```

### 3.2 Core Mappings

| C++ Type | FOSSIL Type |
|----------|-------------|
| `std::string` | `Str` (ref-counted, SSO) |
| `std::vector<T>` | `Vec<T>` |
| `std::unique_ptr<T>` | `Own<ptr[T]>` |
| `std::shared_ptr<T>` | `Shared<T>` |
| `std::optional<T>` | `Opt<T>` = `[T, void]` |
| `std::function<R(Args)>` | `Fn(R, Args...)` |

---

## Phase 4: Template Interop (2-3 weeks)

```fsl
EXTERN TEMPLATE "std::vector" INSTANTIATE [i32] AS Vec_i32
EXTERN TEMPLATE "std::map" INSTANTIATE [str, i32] AS Map_str_i32

VAR v: Vec_i32 = Vec_i32::new()
v.push_back(42)
```

---

## Phase 5: Exception Handling (2-3 weeks)

```fsl
EXTERN EXCEPTION "std::runtime_error" AS RuntimeError {
    CTOR(str) : ptr[RuntimeError]
    METHOD what(ptr[RuntimeError]) : str
}

FN call_cpp(): i32
    TRY
        RETURN risky_cpp_call()
    CATCH RuntimeError AS e
        IO.eprintln("Error: " + e.what())
        RETURN -1
    ENDTRY
ENDFN

FN validate(x: i32): void
    IF x < 0
        THROW RuntimeError("x must be non-negative")
    ENDIF
ENDFN
```

---

## Phase 6: Build Integration (1-2 weeks)

- CMake integration (`carnax.cmake`)
- `pkg-config` / CMake module generation
- `compile_commands.json` for LSP

---

## Phase 7: Advanced (Ongoing)

- Operator overloading binding
- Lambda/std::function interop
- Smart pointer interop (`unique_ptr` ↔ `own`, `shared_ptr` ↔ `Shared`)
- C++20 modules support

---

## Technical Decisions

| Decision | Choice |
|----------|--------|
| ABI | Itanium (Linux/macOS) + MSVC (Windows) via `llvm::CXXABI` |
| Header parsing | Manual bindings first; Clang LibTooling optional Phase 7+ |
| Multiple inheritance | MVP: single only; MI via composition |
| Smart pointers | `own[CppClass]` → calls dtor on drop |

---

## Next Steps

1. **Immediate:** Phase 1 - extend `ExternDecl` with linkage + mangling
2. **Design Review:** Confirm class declaration syntax
3. **Prototype:** Minimal `std::string` / `std::vector` binding
4. **Test:** Integrate with `fmt` library