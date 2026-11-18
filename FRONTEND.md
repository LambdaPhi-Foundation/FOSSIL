# Carnax Frontend in FOSSIL - Self-Hosting Plan

## Vision

Rewrite the Carnax compiler frontend (lexer, parser, AST, semantic analyzer) in FOSSIL itself, once the language is sufficiently mature. This is a major milestone demonstrating language maturity and enables dogfooding.

---

## Current State (Rust Frontend)

```
src/
├── main.rs           # CLI entry
├── driver.rs         # Pipeline orchestration
├── lexer.rs          # ~400 lines - Tokenizer
├── parser.rs         # ~1300 lines - Recursive descent
├── ast.rs            # ~330 lines - AST definitions
├── semantic.rs       # ~2100 lines - Type checking, ownership
├── codegen.rs        # ~2400 lines - LLVM IR emission
├── module_loader.rs  # ~140 lines - Built-in module loading
└── token.rs          # ~170 lines - Token definitions
```

**Total: ~4700 lines of Rust**

---

## Prerequisites for Self-Hosting

### Language Features Required

| Feature | Status | Notes |
|---------|--------|-------|
| Structs with generics | ✅ | `Vec<T>`, `Option<T>`, `Result<T,E>` |
| Tagged unions | ✅ | `Result<T,E>` pattern |
| Pattern matching | ✅ | `MATCH` with destructuring |
| Module system | ✅ | `MODULE`, `USE` |
| Ownership (`own`, `ptr`) | ✅ | Core ownership tracking |
| RAII / destructors | ❌ | **Blocker** - needed for memory safety |
| String manipulation | ⚠️ | Basic `str` type, needs more methods |
| File I/O | ✅ | Via `MACHINE::IO` |
| Dynamic allocation | ⚠️ | `own` exists, allocator TBD |
| Error handling | ⚠️ | `Result` type, no `try!`/`?` syntax |
| Assertions/testing | ❌ | Needed for self-test |

### Compiler Infrastructure Needed

- [ ] **Standard library**: `Vec`, `String`, `HashMap`, `Option`, `Result`
- [ ] **Allocator**: `malloc`/`free` bindings or custom allocator
- [ ] **Error handling**: `try`/`catch` or `?` operator
- [ ] **Testing framework**: Unit test harness
- [ ] **Build system**: FOSSIL can build FOSSIL (circular dependency resolved)

---

## Architecture: FOSSIL Frontend in FOSSIL

### Module Structure

```
carnax/
├── src/
│   ├── main.fsl              # CLI entry point
│   ├── driver.fsl            # Pipeline orchestration
│   ├── lexer/
│   │   ├── mod.fsl           # Lexer module exports
│   │   ├── token.fsl         # Token definitions
│   │   ├── tokenize.fsl      # Tokenization logic
│   │   └── keywords.fsl      # Keyword tables
│   ├── parser/
│   │   ├── mod.fsl           # Parser module exports
│   │   ├── ast.fsl           # AST node definitions
│   │   ├── parse.fsl         # Recursive descent parser
│   │   ├── expr.fsl          # Expression parsing
│   │   └── stmt.fsl          # Statement parsing
│   ├── semantic/
│   │   ├── mod.fsl           # Semantic module exports
│   │   ├── analyze.fsl       # Main analysis driver
│   │   ├── types.fsl         # Type system (Ty, TypeEq)
│   │   ├── scope.fsl         # Scope management
│   │   ├── ownership.fsl     # Ownership/move checking
│   │   ├── resolve.fsl       # Name resolution
│   │   └── error.fsl         # Error reporting
│   ├── ast/
│   │   ├── mod.fsl           # AST module exports
│   │   ├── nodes.fsl         # AST node types
│   │   ├── visit.fsl         # Visitor pattern
│   │   └── pretty.fsl        # AST pretty-printing
│   └── utils/
│       ├── mod.fsl
│       ├── span.fsl          # Source span/location
│       ├── symbol.fsl        # Symbol interning
│       └── diagnostic.fsl    # Error/warning diagnostics
```

### Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| **No visitor trait objects** | Use `MATCH` on tagged unions for AST traversal |
| **Error as union** | `Result<T, Error>` for fallible operations |
| **Arena allocation** | Use `Vec` + indices for AST nodes (no `own` cycles) |
| **Functional style** | Prefer immutable data, explicit state passing |
| **Modules = namespaces** | Each `.fsl` = one module, `USE` for imports |

---

## Bootstrapping Strategy

### Stage 0: Rust → FOSSIL (Current)
```
Rust frontend compiles FOSSIL programs
```

### Stage 1: Minimal Self-Hosted Lexer
```
1. Write lexer.fsl in FOSSIL
2. Compile with Rust carnax → lexer.exe
3. Verify: lexer.exe tokenizes lexer.fsl correctly
```

### Stage 2: Minimal Self-Hosted Parser
```
1. Write parser.fsl in FOSSIL (uses lexer.exe)
2. Compile with Rust carnax → parser.exe
3. Verify: parser.exe parses parser.fsl correctly
```

### Stage 3: Full Frontend Bootstrap
```
1. Write semantic.fsl, ast.fsl, driver.fsl
2. Compile with Rust carnax → carnax_fossil.exe
3. Verify: carnax_fossil.exe compiles carnax_fossil.fsl
```

### Stage 4: Self-Hosting Complete
```
Rust carnax builds FOSSIL carnax
FOSSIL carnax builds FOSSIL carnax (self-hosted)
Rust carnax retired (or kept for bootstrapping new platforms)
```

---

## Implementation Priorities

### P0: Blockers (Must Have Before Starting)
| Task | Estimate |
|------|----------|
| Implement RAII/destructors | 2-3 weeks |
| Standard library (Vec, String, HashMap, Option, Result) | 3-4 weeks |
| Error handling (`try`/`catch` or `?`) | 1-2 weeks |
| Memory allocator integration | 1 week |

### P1: Core Frontend (Can Start After P0)
| Module | Estimate | Dependencies |
|--------|----------|--------------|
| `lexer/token.fsl` | 1 week | P0 |
| `lexer/tokenize.fsl` | 1 week | token.fsl |
| `ast/nodes.fsl` | 1 week | P0 |
| `ast/visit.fsl` | 3 days | nodes.fsl |
| `parser/parse.fsl` | 2 weeks | lexer, ast |
| `semantic/types.fsl` | 1 week | P0 |
| `semantic/scope.fsl` | 1 week | types.fsl |
| `semantic/ownership.fsl` | 2 weeks | scope.fsl |
| `semantic/analyze.fsl` | 2 weeks | ownership.fsl |
| `driver.fsl` | 1 week | All above |

### P2: Polish (After Core Works)
| Feature | Estimate |
|---------|----------|
| Pretty printer | 3 days |
| Error recovery in parser | 1 week |
| Incremental compilation | 2 weeks |
| LSP support | 2 weeks |

---

## Memory Management in Self-Hosted Compiler

### Strategy: Arena + Indices
```fsl
STRUCT Arena<T> {
    data: Vec<T>,
}

FN new<T>(): Arena<T>
    RETURN Arena { data: Vec::new() }
ENDFN

FN alloc<T>(arena: ptr[Arena<T>], value: T): usize
    arena.data.push(value)
    RETURN arena.data.len() - 1
ENDFN

FN get<T>(arena: ptr[Arena<T>], idx: usize): ptr[T]
    RETURN &arena.data[idx]
ENDFN
```

- All AST nodes allocated in arenas
- Nodes reference each other via `usize` indices
- Arenas dropped at end of compilation (no per-node destructors needed)

---

## Testing Strategy

### 1. Round-Trip Tests
```fsl
FN test_roundtrip(source: str): bool
    VAR tokens = tokenize(source)
    VAR ast = parse(tokens)
    VAR output = pretty_print(ast)
    VAR tokens2 = tokenize(output)
    VAR ast2 = parse(tokens2)
    RETURN ast == ast2
ENDFN
```

### 2. Golden File Tests
```
tests/
├── lexer/
│   ├── test_basic.fsl
│   ├── test_keywords.fsl
│   └── ...
├── parser/
│   ├── test_expressions.fsl
│   ├── test_statements.fsl
│   └── ...
├── semantic/
│   ├── test_ownership.fsl
│   ├── test_types.fsl
│   └── ...
└── roundtrip/
    └── ...
```

### 3. Self-Test
```
# After bootstrap:
carnax_fossil.exe build src/main.fsl -o carnax_fossil2.exe
carnax_fossil2.exe build src/main.fsl -o carnax_fossil3.exe
diff carnax_fossil2.exe carnax_fossil3.exe  # Should be identical
```

---

## Migration Checklist

### Code Patterns to Convert (Rust → FOSSIL)

| Rust Pattern | FOSSIL Equivalent |
|--------------|-------------------|
| `enum` with data | Tagged union `[Variant1(T), Variant2]` |
| `Option<T>` | `[T, void]` or builtin `Option<T>` |
| `Result<T,E>` | Builtin `Result<T,E>` |
| `Vec<T>` | Builtin `Vec<T>` |
| `Box<T>` | `own[T]` |
| `&T` / `&mut T` | `ptr[T]` |
| `match` | `MATCH` with destructuring |
| `impl Trait` | Traits (future) / struct with fn ptrs |
| `Iterator` | `FN next(ptr[Iter]): Option<T>` |
| `HashMap<K,V>` | Builtin `Map<K,V>` |
| `Rc/Arc` | `Shared<T>` |
| `RefCell` | Interior mutability (future) |
| `trait` objects | Existential types (future) |

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Circular dependency (compiler needs compiler) | Keep Rust compiler for bootstrapping new platforms |
| FOSSIL not expressive enough | Extend language as needed (dogfood-driven design) |
| Performance regression | Profile, optimize hot paths; LLVM IR same |
| Debugging self-hosted compiler | Keep Rust version for debugging; add logging |
| Bootstrap failure | Keep Rust version as fallback; incremental migration |

---

## Success Criteria

1. ✅ `carnax_fossil.exe` builds all examples in `examples/`
2. ✅ `carnax_fossil.exe` passes all existing tests
3. ✅ `carnax_fossil.exe build src/main.fsl -o carnax2.exe` produces working compiler
4. ✅ `carnax2.exe` produces identical output to `carnax_fossil.exe`
5. ✅ Self-hosted compiler can build itself on clean machine (with Rust bootstrap)

---

## Timeline Estimate

| Phase | Duration | Cumulative |
|-------|----------|------------|
| P0: Blockers (RAII, stdlib, errors) | 6-8 weeks | 6-8 weeks |
| P1: Core frontend modules | 10-12 weeks | 16-20 weeks |
| P2: Integration & bootstrap | 4-6 weeks | 20-26 weeks |
| **Total** | | **~5-6 months** |

---

## Decision Points

1. **When to start?** After RAII + stdlib + error handling are solid
2. **Rewrite vs translate?** Translate mechanically first, then refactor to idiomatic FOSSIL
3. **Keep Rust backend?** Yes - LLVM IR emission stays in Rust initially; port later
5. **Testing during migration?** Each module must pass tests before integration

---

## References

- Rust compiler bootstrap: <https://github.com/rust-lang/rust/tree/master/compiler>
- Zig self-hosting: <https://github.com/ziglang/zig/blob/master/doc/langref.md#self-hosted-compiler>
- Go bootstrap: <https://go.dev/doc/faq#bootstrapping>