# MACHINE::IO Implementation Progress - Phase 1 & 2

## Project Status

> **⚠ Early Development** - The FOSSIL language and Carnax compiler are in early alpha. The language design is **far from its final stage**. Expect breaking changes, missing features, and rough edges.

> **🤖 AI-Assisted Development** - While this project leverages AI assistance for implementation speed, **meticulous architectural planning and deliberate design decisions** have gone into every component: the ownership system, module resolution, semantic analysis pipeline, and codegen strategy. The goal is to evolve this into a reliable, production-quality systems language over time.

---

## I/O Module Design

The I/O system is organized as a **single top-level module** `MACHINE::IO` that provides three sub-modules and convenience functions. There are **not two separate sets** of I/O functions — just `MACHINE::IO` with:

```
MACHINE::IO
├── Conout (stdout)
├── Conin  (stdin)
├── Conerr (stderr)
└── Convenience functions: print, println, eprint, eprintln, read_line, exit
```

Usage:
```fsl
USE MACHINE::IO

FN main(): i32
    IO.println("Hello!")           ; stdout with newline
    IO.Conout.writeln("World")     ; explicit stdout
    IO.Conerr.writeln("Error!")    ; stderr
    IO.Conin.scanline(buf, 256)    ; stdin line read
    IO.exit(0)
ENDFN
```

---

## Phase 1 Summary: Module Syntax Support

Implemented basic module syntax support (`MODULE`/`ENDMODULE`) for the FOSSIL language. This is the foundation for the `MACHINE::IO` module system.

### Changes Made (Phase 1)

1. **Token Definitions** (`src/token.rs`)
   - Added `Module` and `EndModule` variants to `TokenKind` enum
   - Added keyword mappings for `"MODULE"` → `TokenKind::Module` and `"ENDMODULE"` → `TokenKind::EndModule`

2. **AST Definitions** (`src/ast.rs`)
   - Added `ModuleDecl` struct with `name` and `items`
   - Added `ModuleDecl(ModuleDecl)` variant to `Item` enum

3. **Parser** (`src/parser.rs`)
   - Added `TokenKind::Module` case to `parse_item()`
   - Implemented `parse_module_decl()` supporting nested modules

4. **Semantic Analyzer** (`src/semantic.rs`)
   - Added module analysis with scope management
   - Extended declaration collection to recursively process module items

5. **Codegen** (`src/codegen.rs`)
   - Added module emission with `emit_module_item()`
   - Fixed pre-existing bug in `emit_expr()` for `Expr::Call`

### Test Results (Phase 1)
- ✅ Basic module parsing, type-checking, LLVM IR emission, native compilation
- ✅ Nested modules support
- ✅ Module scope management

---

## Phase 2 Summary: Module Resolution & Import System

### Changes Made (Phase 2)

1. **Module Loader** (`src/module_loader.rs` - NEW)
   - Loads built-in MACHINE modules from `src/machine/*.fsl`
   - Supports: IO, win32, io_handles, Conout, Conin, Conerr
   - Caches loaded modules

2. **Extended Token Support** (`src/token.rs`)
   - Added `As` (for `as` cast), `Unreachable`, `Null` keywords
   - Case-insensitive keyword matching for `as`/`AS`

3. **AST Extensions** (`src/ast.rs`)
   - Added `ConstDecl` for module-level constants
   - Added `VarDecl` item variant for module-level variables
   - Added `Cast` expression for `as` casts
   - Added `Null` literal and `NullPtr` type

4. **Parser Extensions** (`src/parser.rs`)
   - Added `ConstDecl` and `VarDecl` parsing at module/item level
   - Added `as` cast expression parsing (postfix)
   - Added `Unreachable` statement
   - Fixed `!=` operator tokenization

5. **Semantic Analyzer** (`src/semantic.rs`)
   - Module loader integration with import resolution
   - Namespace alias tracking (`imported_modules: HashMap<short_name, full_path>`)
   - Qualified name resolution for function calls (`Conout.write_str` → `MACHINE.Conout.write_str`)
   - Type compatibility for constants (integer promotion)
   - Null pointer type (`NullPtr`) compatible with all pointer types

6. **Codegen Extensions** (`src/codegen.rs`)
   - Module loader with error reporting (no silent failures)
   - Module stack for qualified name generation (`MACHINE.Conout.write_str`)
   - Import path tracking for qualified name resolution
   - Constant/variable emission at module level
   - Function registration with qualified names
   - Cast expression codegen
   - Null pointer emission

### New Language Features
- **Module constants**: `CONST NAME: type = value`
- **Module variables**: `VAR name: type = value` (mutable)
- **Cast expressions**: `expr as Type`
- **Null literal**: `null` compatible with all pointer types
- **Unreachable statement**: `UNREACHABLE` for compiler hints

### Remaining Issues

1. **Parser Bug in Codegen Module Loader**
   - The codegen's `load_machine_module` fails to parse `src/machine/conout.fsl` with "expected Colon at line 21 col 15, got EqSign"
   - Same file parses correctly in semantic analyzer's module loader
   - Root cause: Parser state issue when loading modules recursively during codegen
   - Workaround: Embed Conout/Conin/Conerr functions directly in codegen (like old `add_machine_io_functions`)

2. **User-defined Module Resolution**
   - `USE` for user-defined modules (non-MACHINE) not fully implemented
   - Qualified name resolution for user modules needs work

3. **Type System Gaps**
   - `usize` vs `isize` vs `i32`/`i64` conversions need refinement
   - String literal type (`str` vs `string`) needs standardization

### Test Status
- ✅ Phase 1: Basic modules, nested modules
- ✅ Phase 2: MACHINE module loading, qualified names, imports (semantic analyzer)
- ❌ Phase 2: Codegen module loading for Conout/Conin/Conerr (parser bug)
- ⚠️ Phase 2: User-defined module imports (partial)

### Next Steps (Phase 3)
1. Fix codegen parser bug or embed Conout/Conin/Conerr directly in codegen
2. Complete user-defined module import resolution
3. Implement `Conin.read`, `Conin.scanline`, `Conerr.write` functions
4. Add inline assembly for syscalls (Linux/macOS support)
5. Standardize type system (`str` vs `string`, `usize`/`isize`)