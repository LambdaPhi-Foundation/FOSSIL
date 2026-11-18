# TeethAndClaw IR (TAC-IR) - Implementation Plan

## Overview
TeethAndClaw IR is an intermediate representation between Fossil's semantic analysis and LLVM IR. It provides:
- SSA-based representation with explicit control flow
- Fossil-aware types (ownership, tuples, unions, arrays)
- Target-independent optimization surface
- Serializable format (text + binary)
- Foundation for multiple backends (LLVM, Cranelift, WASM)

---

## Current Pipeline
```
Fossil (.fsl) → Lexer → Parser → AST → Semantic Analysis → Typed AST
                                                    ↓
                                           Codegen (text LLVM IR)
                                                    ↓
                                                Clang → Binary
```

## Target Pipeline
```
Fossil (.fsl) → Lexer → Parser → AST → Semantic Analysis → Typed AST
                                                    ↓
                                           TAC Lowering → TeethAndClaw IR
                                                    ↓
                                           TAC Optimizer (optional)
                                                    ↓
                                           TAC → LLVM IR Translator
                                                    ↓
                                                Clang → Binary
```

---

## TAC-IR Type System

```rust
enum TacType {
    // Primitive
    Bool, I8, I16, I32, I64, U8, U16, U32, U64, F32, F64, Void,
    // Composite
    Ptr(Box<TacType>),           // ptr[T]
    Own(Box<TacType>),           // own[T] - ownership tracked
    Array(Box<TacType>, u64),    // [T; N]
    Tuple(Vec<TacType>),         // (T1, T2, ...)
    Union(Vec<TacType>),         // [T1, T2, ...] tagged
    Struct(String),              // named struct (opaque handle)
    Enum(String),                // named enum (opaque handle)
    // Function
    Fn(Vec<TacType>, Box<TacType>), // (params) -> ret
}
```

Maps 1:1 with `semantic::Ty` but with explicit layout and ownership tracking.

---

## TAC-IR Instruction Set (SSA)

### Control Flow
| Instruction | Description |
|-------------|-------------|
| `Br(label)` | Unconditional branch |
| `BrCond(value, label_true, label_false)` | Conditional branch |
| `Switch(value, [(const, label)], default_label)` | Match/switch |

### Function
| Instruction | Description |
|-------------|-------------|
| `Call(fn_name, args, ret_type)` | Direct call |
| `CallIndirect(fn_ptr, args, ret_type)` | Function pointer call |
| `Ret(value?)` | Return |

### Memory (Ownership-Aware)
| Instruction | Description |
|-------------|-------------|
| `Alloc(ty)` | Stack allocation |
| `AllocHeap(ty)` | Heap allocation (for `own`) |
| `Free(ptr)` | Explicit free (own drop) |
| `Load(ptr, ty)` | Load from memory |
| `Store(ptr, value)` | Store to memory |
| `GEP(ptr, indices)` | GetElementPointer |

### Ownership
| Instruction | Description |
|-------------|-------------|
| `Move(src, dst)` | Ownership transfer |
| `Copy(src, dst)` | Shallow copy (ptr) |
| `Clone(src, dst)` | Deep copy |
| `Drop(value)` | Explicit destructor call |

### Composite Construction
| Instruction | Description |
|-------------|-------------|
| `TupleConstruct(values)` | Build tuple |
| `TupleExtract(tuple, index)` | Extract tuple element |
| `StructConstruct(name, values)` | Build struct |
| `StructExtract(struct, field)` | Extract struct field |
| `UnionConstruct(tag, payload)` | Build tagged union |
| `UnionTag(union)` | Extract tag |
| `UnionPayload(union)` | Extract payload (checked) |

### Arrays
| Instruction | Description |
|-------------|-------------|
| `ArrayConstruct(values)` | Build array |
| `ArrayLen(array)` | Length of `[T; N]` |
| `ArrayGet(array, index)` | Element access |
| `ArraySet(array, index, value)` | Element update |

### Arithmetic/Logic
| Instruction | Description |
|-------------|-------------|
| `Add/Sub/Mul/Div/Rem` | Integer arithmetic |
| `And/Or/Xor/Not` | Bitwise |
| `Shl/Shr` | Shifts |
| `ICmp(pred, lhs, rhs)` | Integer compare |
| `FCmp(pred, lhs, rhs)` | Float compare |

### Casts
| Instruction | Description |
|-------------|-------------|
| `Cast(value, ty)` | Bitcast, inttoptr, etc. |
| `PtrToInt(ptr)` | Pointer to integer |
| `IntToPtr(int)` | Integer to pointer |

### Constants
| Instruction | Description |
|-------------|-------------|
| `ConstInt(i64, ty)` | Integer constant |
| `ConstFloat(f64, ty)` | Float constant |
| `ConstBool(bool)` | Boolean constant |
| `ConstNull(ty)` | Null pointer |
| `ConstString(string_id)` | Interned string |

### Intrinsics
| Instruction | Description |
|-------------|-------------|
| `Intrinsic(name, args)` | Built-in: memcpy, sqrt, etc. |

### Debug
| Instruction | Description |
|-------------|-------------|
| `DbgValue(value, var_name)` | Variable location |
| `DbgLocation(file, line, col)` | Source location |

---

## Module Structure

```rust
struct TacModule {
    name: String,
    functions: Vec<TacFunction>,
    globals: Vec<TacGlobal>,
    structs: HashMap<String, TacStructDef>,
    enums: HashMap<String, TacEnumDef>,
    strings: HashMap<String, String>,   // string_id -> content
}

struct TacFunction {
    name: String,
    params: Vec<TacParam>,      // (name, type, is_own)
    ret_type: TacType,
    blocks: Vec<TacBlock>,
    is_extern: bool,
    calling_conv: CallingConv,
}

struct TacBlock {
    label: Label,
    insts: Vec<TacInst>,
    // Terminator is last inst (Br, Ret, Switch)
}

struct TacParam {
    name: String,
    ty: TacType,
    is_own: bool,
}
```

---

## Text Format (`.tac`)

```
module "main" {
    struct Point { i32, i32 }
    enum Option<T> { None, Some(T) }
    
    fn main() -> i32 {
    entry:
        %x = alloc i32
        store %x, 42
        %y = load %x
        ret %y
    }
    
    fn make_point(i32, i32) -> %struct.Point {
    entry:
        %p = alloc %struct.Point
        store %p.field.0, %0
        store %p.field.1, %1
        ret %p
    }
}
```

---

## Implementation Phases

### Phase 1: IR Definition & Lowering (Core) - ~2000 lines

**New Files:**
- `src/tac/types.rs` - Type definitions
- `src/tac/inst.rs` - Instruction definitions
- `src/tac/module.rs` - Module/Function/Block structure
- `src/tac/lower.rs` - Typed AST → TAC lowering
- `src/tac/pretty.rs` - Text format printer

**Tasks:**
1. Define TAC type system mirroring `semantic::Ty`
2. Implement lowering from semantic-analyzed AST to TAC
3. Handle ownership: `Move`, `Drop`, `AllocHeap`/`Free`
4. Lower control flow: `IF`/`WHILE`/`MATCH` → CFG with SSA
5. Lower composites: tuples, unions, structs, arrays
6. Emit text format for debugging (`.tac` files)

### Phase 2: TAC → LLVM IR Translator - ~1500 lines

**New File:**
- `src/tac/to_llvm.rs` - Translator

**Tasks:**
1. Map TAC types → LLVM types (similar to current `ty_to_llvm`)
2. Translate TAC instructions → LLVM IR
3. Handle ownership: `Move` → bitcast, `Drop` → destructor call
4. Handle tuples/unions: LLVM struct types with tag field
5. Handle arrays: LLVM array types
6. Preserve debug info
7. Replace current `codegen.rs` `emit_ir` with TAC lowering + translation

### Phase 3: TAC Optimizations (Optional) - ~2000+ lines

**New Files:**
- `src/tac/opt/passes.rs` - Optimization passes
- `src/tac/opt/manager.rs` - Pass manager

**Candidate Passes:**
- Dead code elimination (DCE)
- Constant folding/propagation
- Copy propagation (eliminate `Copy`/`Move` pairs)
- Ownership optimization: elide `Move`/`Drop` pairs
- Function inlining (small functions)
- Loop invariant code motion
- Common subexpression elimination

### Phase 4: Serialization & Tooling - ~500 lines

- Binary format (bincode) for fast loading
- `tac-dis` tool: `.tac` binary → text
- `tac-opt` tool: run optimization passes
- CLI: `carnax emit-tac <file.fsl> [-o out.tac]`

---

## Integration Points

### Driver Changes (`driver.rs`)

```rust
// Current:
let ir = emit_ir(&program)?;

// New:
let tac_module = tac::lower::lower_program(&semantic_result)?;
if emit_tac {
    tac::pretty::write_tac(&tac_module, output)?;
} else {
    let llvm_ir = tac::to_llvm::translate(&tac_module)?;
    compile_ir_to_binary(&llvm_ir, ...)?;
}
```

### CLI Additions (`main.rs`)

```bash
carnax emit-tac <file.fsl> [-o out.tac]    # Emit TeethAndClaw IR
carnax emit-tac <file.fsl> --opt            # Emit optimized TAC
```

---

## Lowering Rules (Key Examples)

### Function with Ownership
```fsl
FN take_ownership(x: own[i32]): i32
    RETURN *x
ENDFN

FN main(): i32
    VAR owned: own[i32] = 42
    VAR result: i32 = take_ownership(owned)
    RETURN result
ENDFN
```

**TAC:**
```
fn take_ownership(%x: own i32) -> i32 {
entry:
    %val = load %x          ; deref own
    %moved = move %x        ; ownership transferred
    drop %moved             ; caller's own is now dead
    ret %val
}

fn main() -> i32 {
entry:
    %owned = allocheap i32
    store %owned, 42
    %result = call take_ownership(%owned)
    ret %result
}
```

### Match Expression
```fsl
MATCH opt
    CASE Some(x) => x
    CASE None => 0
ENDMATCH
```

**TAC:**
```
entry:
    %tag = uniontag %opt
    switch %tag [
        0 -> label_none   ; None
        1 -> label_some   ; Some
    ] label_none

label_none:
    br label_end

label_some:
    %payload = unionpayload %opt
    br label_end

label_end:
    %result = phi [0, label_none], [%payload, label_some]
    ...
```

### Tuple/Struct
```fsl
VAR p: (i32, i32) = (1, 2)
VAR s: Point = Point { x: 1, y: 2 }
```

**TAC:**
```
%p = tupleconstruct [1, 2]
%p0 = tupleextract %p, 0
%p1 = tupleextract %p, 1

%s = structconstruct "Point" [1, 2]
%s_x = structextract %s, "x"
%s_y = structextract %s, "y"
```

---

## Benefits for Fossil

| Benefit | Description |
|---------|-------------|
| **Ownership optimization** | Eliminate redundant `Move`/`Drop`, track ownership in IR |
| **Fossil-specific opts** | Tuple/union layout optimization, enum tag packing |
| **Multiple backends** | Cranelift (fast debug), WASM, GCC, MSVC |
| **Incremental builds** | Cache `.tac` files; only re-lower changed modules |
| **Better diagnostics** | IR-level error messages, visual CFG dumps |
| **Language evolution** | IR stable while frontend/backend evolve independently |

---

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Scope creep in IR design | Freeze IR v1 spec before implementation |
| Breaking existing tests | Run integration tests after each phase |
| Performance regression | Benchmark LLVM IR quality vs. current |
| LLVM API vs text emission | Keep text emission for now; API later |

---

## Estimated Effort

| Phase | Files | Est. Lines | Complexity |
|-------|-------|------------|------------|
| 1: IR + Lowering | 5 | ~2000 | High (core) |
| 2: LLVM Translator | 1 | ~1500 | Medium |
| 3: Optimizations | 3+ | ~2000+ | High (iterative) |
| 4: Serialization | 2 | ~500 | Low |
| **Integration** | 2 | ~300 | Low |

**Total: ~6300 lines, 4-6 weeks for MVP (Phases 1-2)**

---

## Next Steps

1. **Create `src/tac/` directory** and implement Phase 1 files
2. **Define IR v1 spec** (this document + type/inst definitions)
3. **Implement lowering** for core constructs (functions, control flow, ownership)
4. **Add `emit-tac` CLI command** for debugging
5. **Implement translator** to replace current LLVM IR emission
6. **Run full test suite** to verify correctness
7. **Add optimization passes** incrementally