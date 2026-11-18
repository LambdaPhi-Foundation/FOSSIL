# Phase 7: Compiler Changes

## Goal
Implement all compiler support needed for CORE: traits, macros, intrinsics, module loading.

---

## 7.1 Trait System (Semantic + Codegen)

### Files Modified
- `src/ast.rs` — Add `TraitDecl`, `ImplDecl`
- `src/parser.rs` — Parse `TRAIT`/`IMPL` syntax
- `src/semantic.rs` — Trait resolution, coherence, vtable layout
- `src/codegen.rs` — Emit vtables, dynamic dispatch

### Key Structures

```rust
// semantic.rs
struct TraitDef {
    name: String,
    generic_params: Vec<String>,
    methods: Vec<TraitMethodSig>,
    supertraits: Vec<String>,
}

struct TraitMethodSig {
    name: String,
    generic_params: Vec<String>,
    params: Vec<(String, Ty, bool)>,
    return_ty: Ty,
    has_default: bool,
}

struct ImplDef {
    trait_name: String,
    for_ty: Ty,
    generic_params: Vec<String>,
    methods: HashMap<String, FnSignature>,
}
```

### VTable Layout (codegen.rs)
```llvm
%vtable.Write = type {
    void (i8*)*,                    ; destructor
    i64 (i8*, %slice.u8)*,          ; write
    i64 (i8*, %Str)*,               ; write_str
    void (i8*)*,                    ; flush
}
```

### Trait Object = Fat Pointer
```llvm
%Write = type { i8*, %vtable.Write* }
```

---

## 7.2 Macro System (Format, Assert, Panic)

### Format String Parsing
- Compile-time parsing of `format!("{} {}", a, b)`
- Expands to direct `Format::fmt` calls
- `writeln!` adds newline

### Implementation
```rust
// parser.rs - extend macro parsing
fn parse_macro_invocation(&mut self, name: &str) -> Result<Expr, String> {
    match name {
        "format" => self.parse_format_macro(),
        "writeln" => self.parse_writeln_macro(),
        "assert" => self.parse_assert_macro(),
        "panic" => self.parse_panic_macro(),
        _ => Err(format!("unknown macro: {}", name)),
    }
}
```

---

## 7.3 Intrinsics for Formatting

### Codegen Intrinsics (`src/codegen.rs`)
```rust
fn emit_intrinsic(&mut self, name: &str, args: &[Expr]) -> CodegenResult<String> {
    match name {
        "int_to_str" => self.emit_int_to_str(args),
        "uint_to_str" => self.emit_uint_to_str(args),
        "float_to_str" => self.emit_float_to_str(args),
        "ptr_to_str" => self.emit_ptr_to_str(args),
        "memcpy" => self.emit_memcpy(args),
        "memset" => self.emit_memset(args),
        "memmove" => self.emit_memmove(args),
        "memcmp" => self.emit_memcmp(args),
        "atomic_load_*" => self.emit_atomic_load(args),
        "atomic_store_*" => self.emit_atomic_store(args),
        "atomic_cas_*" => self.emit_atomic_cas(args),
        "atomic_fetch_add_*" => self.emit_atomic_fetch_add(args),
        "trap" => self.emit_trap(),
        _ => Err(CodegenError::Msg(format!("unknown intrinsic: {}", name))),
    }
}
```

### Integer to String (fixed buffer, no allocation)
```llvm
; int_to_str(val: i64, buf: i8*, spec: FormatSpec) -> usize
; Writes to buf, returns length
```

---

## 7.4 Panic Runtime

### Codegen for Panic
```rust
fn emit_panic_call(&mut self, msg: &Expr) -> CodegenResult<String> {
    let msg_val = self.emit_expr(msg)?;
    self.line(&format!("call void @core.panic.panic({})", msg_val))?;
    self.line(&format!("call void @llvm.trap()"))?;
    self.line(&format!("unreachable"))?;
    Ok("void".into())
}
```

### Platform-Specific
- `NO_MACHINE` bare metal: `llvm.trap` → `ud2`/`bkpt`
- `MACHINE` Windows: `ExitProcess(1)`
- `MACHINE` Linux: `exit_group(1)` syscall

---

## 7.5 CORE Module Loading

### Module Loader (`src/module_loader.rs`)
```rust
impl ModuleLoader {
    pub fn load_module(&mut self, path: &[String]) -> Result<ModuleDecl, String> {
        if path.is_empty() {
            return Err("empty module path".into());
        }
        
        match path[0].as_str() {
            "MACHINE" => self.load_machine_module(path),
            "CORE" => self.load_core_module(path),
            _ => self.load_user_module(path),
        }
    }
    
    fn load_core_module(&mut self, path: &[String]) -> Result<ModuleDecl, String> {
        // Same pattern as MACHINE, but loads from src/core/
    }
}
```

### Semantic Analyzer Integration
```rust
fn collect_declarations(&mut self, program: &Program) {
    for item in &program.items {
        match item {
            Item::ImportDecl(imp) => {
                if imp.path.len() >= 2 && imp.path[0] == "CORE" {
                    // Register CORE namespace
                    self.imported_modules.insert(
                        imp.path[1].clone(), 
                        format!("CORE.{}", imp.path[1])
                    );
                    // Load and collect
                    if let Ok(module) = self.module_loader.load_module(&imp.path) {
                        self.collect_module_declarations(&module, &imp.path);
                    }
                }
                // ... MACHINE handling
            }
            // ...
        }
    }
}
```

---

## 7.6 Tagged Union Support

### Match Lowering for `[T, E]`
```fsl
MATCH result
    CASE Ok(v) => ...
    CASE Err(e) => ...
ENDMATCH
```
→ LLVM `switch` on tag (0 = Ok, 1 = Err)

### Layout
```llvm
%Result.i32.Error = type { i32, %union.payload }
%union.payload = type { [max(sizeof(i32), sizeof(Error)) x i8] }
```

---

## 7.7 Testing Infrastructure

### Integration Test Helpers
```rust
// tests/integration_tests.rs
fn build_core_test(source: &str) -> Result<i32, String> {
    let program = parse_source(source)?;
    let result = analyze(&program);
    if result.has_errors { return Err(...); }
    let ir = emit_ir(&program)?;
    // compile & run
}
```

### Test Categories
1. **Core memory tests** — `memcpy`, `memset`, `memcmp`
2. **Slice/Str tests** — construction, indexing, subslice
3. **Error handling tests** — `Result`, `Option`, combinators
4. **Formatting tests** — `format!`, `writeln!`, built-in impls
4. **Panic tests** — `panic!`, `assert!`, `assert_eq!`
5. **Allocator tests** — global allocator, `box_new`
6. **Atomic tests** — load/store/CAS/fetch-add

---

## Acceptance Criteria

- [ ] `TRAIT`/`IMPL` syntax parses and type-checks
- [ ] Dynamic dispatch via trait objects works
- [ ] `format!`/`writeln!` macros expand correctly
- [ ] Formatting intrinsics produce correct output
- [ ] `panic!`/`assert!` work in both modes
- [ ] `CORE` modules load like `MACHINE` modules
- [ ] Tagged union `[T, E]` match expressions work
- [ ] All new integration tests pass
- [ ] Zero regression on existing tests