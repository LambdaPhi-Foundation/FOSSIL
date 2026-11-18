# Carnax (FOSSIL Language Compiler) - Agent Instructions

## Project Overview
- **Compiler** for FOSSIL language (`.fsl` files), written in Rust
- Targets LLVM IR → native binaries via Clang
- Single binary crate, no workspace
- Edition 2024, no external dependencies

## Key Commands
```bash
# Build
cargo build --release

# Run tests
cargo test

# Lint & format
cargo clippy
cargo fmt --check

# Run compiler (dev)
cargo run -- <file.fsl>           # emit LLVM IR to stdout
cargo run -- <file.fsl> out.ll    # emit LLVM IR to file
cargo run -- <file.fsl> out.exe   # build native binary
cargo run -- build <file.fsl> [-o out.exe]
cargo run -- run <file.fsl> [-- args...]
cargo run -- emit-ir <file.fsl> [-o out.ll]
```

## Architecture
```
src/
├── main.rs           # CLI entry point
├── lib.rs            # Public exports
├── driver.rs         # Build pipeline (parse→analyze→codegen→clang)
├── lexer.rs          # Tokenizer
├── parser.rs         # Recursive descent parser
├── ast.rs            # Abstract Syntax Tree
├── semantic.rs       # Type checking, ownership, name resolution
├── codegen.rs        # LLVM IR text emission
├── module_loader.rs  # Built-in MACHINE module loading
├── token.rs          # Token definitions
└── machine/          # Built-in MACHINE modules (.fsl)
```

## Testing
- Integration tests in `tests/integration_tests.rs`
- Positive tests: `examples/*.fsl` (expected exit code 0 or specific)
- Negative tests: `examples/errors/*.fsl` (semantic/parse errors)
- Run single test: `cargo test test_negative_const_reassign`

## CLI Behavior
- Default (`carnax file.fsl`): emit LLVM IR to stdout
- `carnax file.fsl out.ll`: write IR to file
- `carnax file.fsl out.exe`: build native binary
- `carnax build|run|emit-ir`: explicit subcommands with `-o` flag

## Build Pipeline (driver.rs)
1. Parse source → AST
2. Semantic analysis (type check, ownership, name resolution)
3. Emit LLVM IR (text-based, not LLVM APIs)
4. Compile IR → native binary via Clang (two-step for MACHINE mode: IR→.o→exe)

## Key Conventions
- **Module loading**: Both semantic analyzer and codegen load `src/machine/*.fsl` via `module_loader.rs`
- **MACHINE mode**: Requires `main(): i32`, links with CRT (two-step compilation)
- **NO_MACHINE mode**: Freestanding, no stdlib, single-step compilation
- **Ownership**: Move detection on assignment, scope exit checks for leaked owners
- **Name mangling**: `MACHINE.Conout.write_str` preserved in LLVM IR
- **Clang discovery**: PATH → `CLANG` env → `LLVM_PATH`/bin → standard install dirs

## Known Limitations (from README)
1. Module system: parser bug with nested IF in machine modules
2. `usize`/`isize` vs `i32`/`i64` conversions need refinement
3. `str` vs `string` distinction needs standardization
4. Platform support: Windows-focused (kernel32.dll); Linux/macOS need `MACHINE::linux`/`MACHINE::macos`

## Example Files Structure
```
examples/
├── 01_basic_types.fsl       # Hello world, basic types
├── 04_structs.fsl           # Struct operations
├── 05_enums_unit.fsl        # Unit enums
├── 06_enums_payload.fsl     # Payload enums
├── 07_match_expressions.fsl # Match statements
├── 08_arrays.fsl            # Array operations
├── 09_multiple_returns.fsl  # Multi-return functions
├── 11_recursion.fsl         # Recursion
├── 12_nested_loops.fsl      # Nested loops
├── 13_early_returns.fsl     # Early returns
├── 15_bitwise.fsl           # Bitwise ops
├── 16_machine_basic.fsl     # MACHINE mode basics
├── 17_machine_args.fsl      # Command line args
├── 18_nomachine.fsl         # NO_MACHINE mode
├── 19_inlineasm_basic.fsl   # Inline assembly
├── 20_misc_comprehensive.fsl# Comprehensive test
└── errors/                  # Negative test cases
```