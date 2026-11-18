# Phase 1: Foundation — Directory Structure & Module Loader

## Goal
Create `src/core/` directory and extend module loader to recognize `CORE` prefix.

## Deliverables

### 1.1 Directory Structure
```
src/
├── core/                    # NEW
│   ├── mod.fsl              # CORE root module
│   ├── mem.fsl              # Memory primitives
│   ├── slice.fsl            # Slice/Str types
│   ├── io.fsl               # Write trait
│   ├── fmt.fsl              # Formatting engine
│   ├── err.fsl              # Error types
│   ├── panic.fsl            # Panic/abort
│   ├── alloc.fsl            # Allocator trait
│   └── atomic.fsl           # Atomics (optional)
├── machine/                 # REFACTORED
│   ├── core.fsl             # Re-exports CORE
│   ├── win32.fsl            # Raw Windows API
│   ├── alloc.fsl            # libc malloc/free
│   ├── io.fsl               # Write impl for handles
│   ├── thread.fsl           # Threading
│   ├── time.fsl             # Time
│   ├── file.fsl             # File I/O
│   └── process.fsl          # Process control
```

### 1.2 Module Loader Changes (`src/module_loader.rs`)

```rust
pub fn load_core_module(&mut self, path: &[String]) -> Result<ModuleDecl, String> {
    if path.is_empty() || path[0] != "CORE" {
        return Err("Not a CORE module path".into());
    }
    
    let module_key = path.join("::");
    if let Some(cached) = self.module_cache.get(&module_key) {
        return Ok(cached.clone());
    }
    
    let module = if path.len() == 1 {
        ModuleDecl { name: "CORE".into(), items: vec![], reexports: vec![] }
    } else if path.len() == 2 {
        let module_name = path[1].to_lowercase();
        match module_name.as_str() {
            "mem" => self.load_mem_module()?,
            "slice" => self.load_slice_module()?,
            "io" => self.load_io_module()?,
            "fmt" => self.load_fmt_module()?,
            "err" => self.load_err_module()?,
            "panic" => self.load_panic_module()?,
            "alloc" => self.load_alloc_module()?,
            "atomic" => self.load_atomic_module()?,
            _ => return Err(format!("Unknown CORE module: {}", path[1])),
        }
    } else {
        return Err(format!("Invalid CORE module path: {}", module_key));
    };
    
    self.module_cache.insert(module_key, module.clone());
    Ok(module)
}
```

### 1.3 CORE Root Module (`src/core/mod.fsl`)
```fsl
; CORE - Freestanding standard library
; Available in both MACHINE and NO_MACHINE modes

MODULE core
    USE CORE.mem
    USE CORE.slice
    USE CORE.io
    USE CORE.fmt
    USE CORE.err
    USE CORE.panic
    USE CORE.alloc
    ; USE CORE.atomic  ; optional
ENDMODULE
```

### 1.4 MACHINE Core Re-export (`src/machine/core.fsl`)
```fsl
MACHINE
MODULE core
    USE CORE.mem
    USE CORE.slice
    USE CORE.io
    USE CORE.fmt
    USE CORE.err
    USE CORE.panic
    USE CORE.alloc
    ; USE CORE.atomic
ENDMODULE
ENDMACHINE
```

## Compiler Integration

- Modify `SemanticAnalyzer::collect_declarations` to handle `CORE` imports
- Add `CORE` to `imported_modules` map during import resolution
- CORE modules available without `MACHINE` declaration

## Testing

```fsl
; examples/test_core_load.fsl
NO_MACHINE
USE CORE.mem
USE CORE.slice

FN main(): i32
    VAR s: slice[u8] = slice_from_parts(null, 0)
    RETURN 0
ENDFN
```

## Acceptance Criteria
- [ ] `src/core/` directory exists with all module files (stubs OK)
- [ ] `USE CORE.mem` works in `NO_MACHINE` mode
- [ ] `USE MACHINE.core` re-exports CORE modules
- [ ] Module loader caches CORE modules correctly
- [ ] No breaking changes to existing MACHINE imports