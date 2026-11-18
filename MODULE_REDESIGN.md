# Module System Redesign Plan

## Overview

Redesign the module system to support:
1. **MACHINE as a library of Modules** - MACHINE is a top-level namespace containing modules (IO, FILE, VECTORS, STRINGS, etc.)
2. **Global values as module exports** - Modules export structs, functions, constants, and global values directly
3. **Unified Function Call Syntax (UFCS)** - `func(x, y)` and `x.func(y)` are both valid for any function
4. **Direct visibility on import** - `USE MACHINE::IO` makes all exports directly visible (no `IO.` prefix needed)

---

## Current State Analysis

### Current Module Structure
```
MACHINE
├── IO (module)
│   ├── Conout (submodule)
│   ├── Conin (submodule)
│   └── Conerr (submodule)
├── win32
├── io_handles
└── ...
```

### Current Import Behavior
```fsl
USE MACHINE::IO
; Must use: IO.Conout.writeln(...)
```

---

## New Design

### 1. Module System Redesign

#### Module Structure
```
MACHINE (namespace)
├── IO (module)
│   ├── Conout: OutputStream (global value)
│   ├── Conin: InputStream (global value)
│   ├── Conerr: OutputStream (global value)
│   ├── writeln(stream: OutputStream, s: str): isize (function)
│   ├── write(stream: OutputStream, buf: ptr[u8], len: usize): isize (function)
│   ├── readln(stream: InputStream, buf: ptr[u8], len: usize): isize (function)
│   └── ...
├── FILE
├── VECTORS
├── STRINGS
└── ...
```

#### Import Behavior
```fsl
USE MACHINE::IO
; All exports from IO become directly visible:
writeln(Conout, "hello")  ; Direct function call
Conout.writeln("hello")   ; UFCS - equivalent to above
readln(Conin, buf, 256)   ; Direct function call
```

#### Module Declaration Syntax
```fsl
MODULE MACHINE::IO
    ; Type definitions
    STRUCT OutputStream
        fd: i32
    ENDSTRUCT

    STRUCT InputStream
        fd: i32
    ENDSTRUCT

    ; Global values (constants)
    CONST Conout: OutputStream = OutputStream { fd: 1 }
    CONST Conerr: OutputStream = OutputStream { fd: 2 }
    CONST Conin: InputStream = InputStream { fd: 0 }

    ; Functions
    FN writeln(stream: OutputStream, s: str): isize
        ; implementation
    ENDFN

    FN write(stream: OutputStream, buf: ptr[u8], len: usize): isize
        ; implementation
    ENDFN

    FN readln(stream: InputStream, buf: ptr[u8], len: usize): isize
        ; implementation
    ENDFN
ENDMODULE
```

### 2. Unified Function Call Syntax (UFCS)

#### Specification
For any function `fn func(first: T, rest...): Ret`:
- `func(x, y, z)` is valid
- `x.func(y, z)` is valid (where `x: T`)
- Both are semantically identical

#### Implementation Points
1. **Parser**: When parsing `x.func(y)`, if `x` has type `T` and there's a function `func(T, ...)`, treat as `func(x, ...)`
2. **Semantic Analysis**: Resolve method calls by looking up functions where first parameter type matches receiver type
3. **Codegen**: Both forms generate identical LLVM IR

#### Example
```fsl
; All equivalent:
writeln(Conout, "hello")
Conout.writeln("hello")

readln(Conin, buf, 256)
Conin.readln(buf, 256)
```

#### Method Resolution Order
1. Check if `x` has a field/method named `func` (struct method)
2. Check if there's a free function `func(T, ...)` where `T` matches `x`'s type
3. Error if neither found

### 3. Import System

#### Direct Visibility on Import
```fsl
USE MACHINE::IO

; All public exports from IO become directly visible:
writeln(Conout, "hello")  ; No prefix needed
Conout.writeln("hello")   ; UFCS works too

USE MACHINE::FILE
; FILE exports now also visible
```

#### Export Control
```fsl
MODULE MACHINE::IO
    ; Public by default
    STRUCT OutputStream ...
    CONST Conout: OutputStream = ...
    FN writeln(...)

    ; Private - not exported
    PRIVATE FN internal_helper(...)
ENDMODULE
```

### 4. Type System Integration

#### Global Values as Constants
```fsl
CONST Conout: OutputStream = OutputStream { fd: 1 }
CONST Conerr: OutputStream = OutputStream { fd: 2 }
CONST Conin: InputStream = InputStream { fd: 0 }
```

These are compile-time constants, not mutable globals.

### 4. UFCS Resolution in Semantic Analysis

#### Algorithm
When encountering `receiver.method(args...)`:
1. Get type of `receiver` -> `T`
2. Look for function `method(T, args...)` in current scope
3. If found, rewrite as `method(receiver, args...)`
4. Type check the rewritten call

---

## Implementation Plan

### Phase 1: Core Module System
1. **AST Changes**
   - Add `reexports` field to `ModuleDecl` (or replace with direct export model)
   - Update `ImportDecl` to support direct visibility model
   - Add `reexports` field to track module re-exports

2. **Parser**
   - Parse `EXPORT` / `REEXPORT` in module declarations
   - Parse `USE` with direct visibility model

3. **Semantic Analyzer**
   - Build module export tables
   - Handle `USE` with direct visibility (import all public exports into current scope)
   - Implement UFCS resolution in `type_of_expr`

### Phase 2: UFCS Implementation
1. **Parser**: Recognize `x.method(y)` as potential UFCS
2. **Semantic**: Resolve UFCS by finding matching free function
2. **Codegen**: Generate identical LLVM IR for both forms

### Phase 3: MACHINE Standard Library
1. Redefine IO module with OutputStream/InputStream structs
2. Define Conout, Conin, Conerr as global constants
2. Rewrite functions to take stream as first parameter

### Phase 4: Codegen
1. Generate identical LLVM IR for `func(x, y)` and `x.func(y)`
2. Handle global constants in LLVM IR

---

## Key Design Decisions

### 1. No Re-exports Needed
With direct visibility on import, `USE MACHINE::IO` brings everything into scope. No need for `IO` to re-export `Conout` from a submodule.

### 2. UFCS is Purely Syntactic
Both `func(x, y)` and `x.func(y)` generate identical LLVM IR. No runtime overhead.

### 3. Global Constants, Not Mutables
`Conout`, `Conin`, `Conerr` are compile-time constants (`CONST`), not mutable globals (`VAR`).

### 4. Method Resolution is Type-Based
UFCS resolution is based on the type of the receiver, not on struct method definitions.

---

## File Changes Required

### New Files
- `src/ufcs.rs` - UFCS resolution logic

### Modified Files
- `src/ast.rs` - ModuleDecl, ImportDecl changes
- `src/parser.rs` - Module/Import parsing, UFCS parsing
- `src/semantic.rs` - Module resolution, UFCS resolution, import handling
- `src/codegen.rs` - UFCS codegen (identical for both forms)
- `src/machine/io.fsl` - Redesigned IO module
- `src/machine/conout.fsl` - Remove (merged into io.fsl)
- `src/machine/conin.fsl` - Remove (merged into io.fsl)
- `src/machine/conerr.fsl` - Remove (merged into io.fsl)
- `src/token.rs` - Add REEXPORT/EXPORT keywords

---

## Example Usage After Redesign

```fsl
MACHINE

USE MACHINE::IO
USE MACHINE::FILE

FN main(): i32
    ; Direct function calls
    writeln(Conout, "Hello, World!")
    writeln(Conerr, "Error message")
    
    ; UFCS syntax
    Conout.writeln("Also works!")
    Conerr.writeln("Error also works!")
    
    VAR buf: [u8; 256]
    VAR n = readln(Conin, buf.ptr, 256)
    ; or
    VAR n = Conin.readln(buf.ptr, 256)
    
    ; FILE module functions also directly visible
    VAR f = fopen("test.txt", "r")
    fclose(f)
    
    IO.exit(0)
ENDFN
```

---

## Open Questions

1. **Method conflicts**: If a struct has a method `foo` and there's also a free function `foo(T, ...)`, which wins for UFCS?
   - Proposed: Struct methods win over free functions

2. **Private fields**: Should `OutputStream.fd` be accessible? Probably yes, it's a struct field.

3. **Multiple dispatch**: What if there are multiple `writeln` functions for different stream types?
   - Standard function overloading resolution applies

4. **Module privacy**: How to mark items as private within a module?
   - `PRIVATE` keyword before declaration

---

## Timeline

| Phase | Duration | Deliverable |
|-------|----------|-------------|
| 1. Module System Core | 1 week | AST, Parser, Semantic for new module system |
| 2. UFCS Implementation | 1 week | Parser, Semantic, Codegen for UFCS |
| 3. MACHINE Stdlib Rewrite | 1 week | New IO, FILE modules with global constants |
| 4. Integration & Testing | 1 week | End-to-end tests, examples |

**Total: ~4 weeks**

---

## Backward Compatibility

- Old `MACHINE` / `NO_MACHINE` syntax still works
- Old `IO.Conout.writeln` style still works (modules still work as namespaces)
- New style is additive, not breaking