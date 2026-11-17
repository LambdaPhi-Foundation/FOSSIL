# Carnax - FOSSIL Language Compiler

A compiler for the **FOSSIL** programming language (`.fsl` files) written in Rust. Targets LLVM IR and compiles to native binaries via Clang.

## Project Status

> **⚠ Early Development** - This project is in early alpha. The FOSSIL language design is **far from its final stage**. Expect breaking changes, missing features, and rough edges. The compiler itself (Carnax) is functional but incomplete.

> **🤖 AI-Assisted Development** - While this project leverages AI assistance for implementation speed, **meticulous architectural planning and deliberate design decisions** have gone into every component: the ownership system, module resolution, semantic analysis pipeline, and codegen strategy. The goal is to evolve this into a reliable, production-quality systems language over time.

## Overview

**FOSSIL** is a systems programming language with:
- ML-like syntax with explicit block terminators (`ENDFN`, `ENDIF`, `ENDWHILE`, etc.)
- Ownership system (`OWN`, `PTR`) with move semantics
- Tagged unions (`[T1, T2]`) and tuples (`(T1, T2)`)
- Inline assembly with operand constraints
- `MACHINE`/`NO_MACHINE` modes for bare-metal vs hosted
- Module system with qualified imports
- Compile-time module loading from source files

## Quick Start

### Prerequisites
- Rust 1.70+
- Clang/LLVM (for native binary compilation)
  - Windows: [LLVM installer](https://releases.llvm.org/) or `choco install llvm`
  - Linux: `apt install clang` / `pacman -S clang`
  - macOS: `brew install llvm` (or use Xcode's clang)

### Build
```bash
cargo build --release
```

### Run Examples
```bash
# Emit LLVM IR
cargo run -- emit-ir examples/01_basic_types.fsl

# Build native binary
cargo run -- build examples/01_basic_types.fsl

# Build and run
cargo run -- run examples/01_basic_types.fsl
```

## Language Reference

### Basic Syntax
```fsl
MACHINE

FN main(): i32
    VAR x: i32 = 42
    CONST PI: f64 = 3.14159
    RETURN x
ENDFN
```

### Types
| Category | Types |
|----------|-------|
| Integers | `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64`, `usize`, `isize` |
| Floats | `f32`, `f64` |
| Bool | `bool` |
| Pointers | `ptr[T]`, `own[T]` (ownership) |
| Arrays | `[T; N]` |
| Tuples | `(T1, T2, ...)` |
| Unions | `[T1, T2, ...]` (tagged) |
| Structs | User-defined |
| Enums | User-defined |

### Control Flow
```fsl
IF condition
    ...
ELSE
    ...
ENDIF

WHILE condition
    ...
ENDWHILE

FOR var = start TO end
    ...
ENDFOR

MATCH expr
    CASE Pattern
        ...
    CASE Pattern
        ...
    ELSE
        ...
ENDMATCH
```

### Functions
```fsl
FN name(param: type, ...): return_type
    VAR local: type = init
    RETURN value
ENDFN
```

### Ownership
```fsl
VAR owned: own[i32] = 42
VAR borrowed: ptr[i32] = &owned
```

### Modules
```fsl
MODULE Math
    FN add(a: i32, b: i32): i32
        RETURN a + b
    ENDFN
ENDMODULE

MACHINE
USE Math
FN main(): i32
    VAR x: i32 = Math.add(1, 2)
    RETURN x
ENDFN
```

### Imports
```fsl
USE MACHINE::IO
USE MACHINE::IO AS StdIO
USE MACHINE::Conout :: { write, writeln }
```

### Casts & Literals
```fsl
VAR x: i32 = 42
VAR y: i64 = x as i64
VAR ptr: ptr[u8] = null
```

### Inline Assembly
```fsl
ASM
    mov rax, `value`
    ret
ENDASM

; With constraints
ASM
    add `a`, `b`
CLobbers("rax")
ENDASM
```

### Strings
```fsl
CONST msg: str = "Hello, World!"
VAR ptr: ptr[u8] = msg.ptr
VAR len: usize = msg.len
```

## Built-in MACHINE Modules

The I/O system is organized as a **single top-level module** `MACHINE::IO` that provides three sub-modules and convenience functions. There are **not two separate sets** of I/O functions — just `MACHINE::IO` with:

### `MACHINE::IO`
Top-level I/O module providing three sub-modules and convenience functions:
- **Sub-modules**: `Conout` (stdout), `Conin` (stdin), `Conerr` (stderr)
- **Convenience functions**: `print`, `println`, `eprint`, `eprintln`, `read_line`, `exit`

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

### `MACHINE::Conout` (stdout)
- `write(ptr[u8], usize): isize`
- `write_str(str): isize`
- `writeln(ptr[u8], usize): isize`
- `writeln_str(str): isize`
- `flush(): void`
- `handle(): ptr[u8]`

### `MACHINE::Conin` (stdin)
- `read(ptr[u8], usize): isize`
- `scanline(ptr[u8], usize): isize`
- `read_char(): i32`
- `read_str(ptr[u8], usize): isize`
- `handle(): ptr[u8]`

### `MACHINE::Conerr` (stderr)
Mirrors `Conout` for stderr.

### `MACHINE::win32`
Raw Windows API bindings (kernel32.dll):
- `GetStdHandle`, `WriteConsoleA`, `ReadConsoleA`
- `WriteFile`, `ReadFile`, `FlushFileBuffers`
- `ExitProcess`, `GetConsoleMode`, `SetConsoleMode`

### `MACHINE::io_handles`
Cached standard handle accessors.

## Compiler Commands

| Command | Description |
|---------|-------------|
| `carnax <file.fsl>` | Emit LLVM IR to stdout |
| `carnax <file.fsl> output.ll` | Emit LLVM IR to file |
| `carnax <file.fsl> output.exe` | Build native binary |
| `carnax build <file.fsl> [-o output.exe]` | Build native binary |
| `carnax run <file.fsl> [-- args...]` | Build, run, return exit code |
| `carnax emit-ir <file.fsl> [-o output.ll]` | Emit LLVM IR to file |
| `carnax -h` / `carnax --help` | Show help |
| `carnax -V` / `carnax --version` | Show version |

## Project Structure

```
carnax/
├── src/
│   ├── main.rs           # CLI entry point
│   ├── lib.rs            # Public exports
│   ├── driver.rs         # Build pipeline (parse→analyze→codegen→clang)
│   ├── lexer.rs          # Tokenizer
│   ├── parser.rs         # Recursive descent parser
│   ├── ast.rs            # Abstract Syntax Tree
│   ├── semantic.rs       # Type checking, ownership, name resolution
│   ├── codegen.rs        # LLVM IR text emission
│   ├── module_loader.rs  # Built-in module loading
│   ├── token.rs          # Token definitions
│   └── machine/          # Built-in MACHINE modules (.fsl)
│       ├── io.fsl
│       ├── win32.fsl
│       ├── io_handles.fsl
│       ├── conout.fsl
│       ├── conin.fsl
│       └── conerr.fsl
├── examples/             # Example programs
├── tests/                # Test files
├── target/               # Build output
└── Cargo.toml
```

## Compiler Architecture

```
┌─────────────┐
│   Source    │
│  (.fsl)     │
└──────┬──────┘
       ▼
┌─────────────┐
│   Lexer     │  → Tokens
└──────┬──────┘
       ▼
┌─────────────┐
│   Parser    │  → AST
└──────┬──────┘
       ▼
┌─────────────┐
│  Semantic   │  → Type checking, ownership, name resolution
│  Analyzer   │
└──────┬──────┘
       ▼
┌─────────────┐
│  Codegen    │  → LLVM IR (text)
└──────┬──────┘
       ▼
┌─────────────┐
│   Clang     │  → Native binary (.exe / ELF)
└─────────────┘
```

## Examples

### Hello World
```fsl
MACHINE
USE MACHINE::IO

FN main(): i32
    IO.println("Hello, World!")
    IO.exit(0)
ENDFN
```

### File I/O
```fsl
MACHINE
USE MACHINE::IO

FN main(): i32
    VAR buf: [u8; 256]
    VAR n: isize = IO.read_line(buf.ptr, 256)
    IF n > 0
        IO.println("Read: " + buf.ptr)  ; pseudo-code
    ENDIF
    IO.exit(0)
ENDFN
```

### Ownership Transfer
```fsl
MACHINE

FN take_ownership(x: own[i32]): i32
    RETURN *x  ; deref and return
ENDFN

FN main(): i32
    VAR owned: own[i32] = 42
    VAR result: i32 = take_ownership(owned)  ; moved
    RETURN result
ENDFN
```

## Known Limitations

1. **Module System**: Codegen module loader has parser bug with nested IF statements in machine modules
2. **Type System**: `usize`/`isize` vs `i32`/`i64` conversions need refinement
3. **String Type**: `str` vs `string` distinction needs standardization
4. **Platform Support**: Currently Windows-focused (kernel32.dll); Linux/macOS need `MACHINE::linux`/`MACHINE::macos` modules

## Development

### Run Tests
```bash
cargo test
```

### Check Code Quality
```bash
cargo clippy
cargo fmt --check
```

### Debug Build
```bash
cargo build
./target/debug/carnax --help
```

## License

MIT License - see LICENSE file for details.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run `cargo clippy` and `cargo fmt`
5. Submit a pull request

## Architecture Notes

### Module Loading
- Semantic analyzer loads modules during declaration collection
- Codegen loads modules during IR emission
- Both use `module_loader.rs` to parse `src/machine/*.fsl` files
- Modules are cached after first load

### Name Mangling
- Module functions: `MACHINE.Conout.write_str` → `MACHINE.Conout.write_str` in LLVM IR
- Qualified calls resolved via `imported_modules` map

### Ownership Checking
- Move detection on assignment from owned identifiers
- Scope exit checks for leaked owners
- Branch merging for moved state (conservative)

### LLVM IR Generation
- Text-based IR emission (not using LLVM APIs)
- Structs: `%struct.Name = type { ... }`
- Enums: `%enum.Name = type { i32, payload }`
- Functions emitted with proper signatures