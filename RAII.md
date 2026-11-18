# RAII Status in Carnax/FOSSIL

## Overview

FOSSIL's ownership system is built around `own[T]` (unique ownership) and `ptr[T]` (borrowed reference). The semantic analyzer tracks ownership at compile time, but **true RAII (automatic destructor calls at scope exit) is not yet implemented in codegen**.

---

## ✅ Currently Implemented

### Type System
| Feature | Location |
|---------|----------|
| `own[T]` type in AST | `src/ast.rs:123` |
| `own` keyword parsing | `src/parser.rs:273,302-303`, `src/token.rs:28,144` |
| `Own(Box<Ty>)` in semantic types | `src/semantic.rs:35` |
| Ownership predicates (`is_owner`, `is_pointer`, `inner`) | `src/semantic.rs:46-56` |

### Ownership Tracking (Semantic Analyzer)
| Feature | Location |
|---------|----------|
| Variable ownership state (`is_owner`, `is_moved`, `is_initialized`) | `src/semantic.rs:173` |
| Function parameter ownership tracking | `src/semantic.rs:207,277` |
| Move detection on assignment | `src/semantic.rs:999-1005` |
| Return value ownership transfer | `src/semantic.rs:1228-1277` |
| Scope exit leak detection | `src/semantic.rs:2011-2016` |
| Branch merge ownership state | `src/semantic.rs:917-970` |

### Codegen
| Feature | Location |
|---------|----------|
| `own[T]` → `T*` type lowering | `src/codegen.rs:2215` |

---

## ❌ Missing (True RAII Not Yet Implemented)

| Feature | Status | Effort |
|---------|--------|--------|
| **Destructor codegen at scope exit** | ❌ Critical | Medium |
| **Destructor declaration syntax** | ❌ Critical | Low |
| **Struct field auto-destruction** | ❌ High | Medium |
| **Move codegen (nullify source)** | ❌ High | Medium |
| **`own` parameter passing convention** | ❌ High | Low |
| **Return value move codegen** | ❌ High | Medium |
| **Array/union field destruction** | ❌ Medium | Medium |

---

## Current Semantic Model

```rust
// semantic.rs:2011-2016 - Scope exit validation (no codegen)
fn check_scope_exit_owners(&mut self) {
    for (_, var) in &self.current_scope().vars {
        if var.is_owner && var.is_initialized && !var.is_moved {
            // owner is being dropped at scope exit: that's the cleanup.
            // We just track it; actual drop codegen comes later.
        }
    }
}
```

**Current behavior:** Semantic analyzer validates that owned values aren't leaked, but emits no destructor calls. The comment "actual drop codegen comes later" is the gap.

---

## Implementation Plan

### Phase 1: Destructor Syntax & Declaration (Low Effort)

#### 1.1 Add Destructor Declaration
```fsl
; Option A: Standalone drop function
FN drop[T](x: own[T]): void

; Option B: Method syntax
IMPL T {
    DTOR(x: own[Self]): void
}

; Option C: Trait-like
TRAIT Drop {
    FN drop(x: own[Self]): void
}
```

**Recommended:** Option A (simplest, matches existing `FN` syntax)

#### 1.2 AST Changes
```rust
// ast.rs
pub struct Function {
    // ... existing
    pub is_destructor: bool,  // or pub kind: FunctionKind
}
```

#### 1.3 Parser
```rust
// parser.rs - add in parse_item()
TokenKind::Drop => Ok(Item::Function(self.parse_destructor()?)),
```

### Phase 2: Destructor Codegen at Scope Exit (Critical)

#### 2.1 Semantic Analyzer: Emit Drop Calls
```rust
// semantic.rs - extend check_scope_exit_owners
fn emit_scope_exit_drops(&mut self, drops: &mut Vec<Expr>) {
    for (name, var) in &self.current_scope().vars {
        if var.is_owner && var.is_initialized && !var.is_moved {
            // Generate: drop(name)
            drops.push(Expr::Call {
                callee: Box::new(Expr::Ident("drop".into())),
                args: vec![Expr::Ident(name.clone())],
            });
            var.is_moved = true; // mark as dropped
        }
    }
}
```

#### 2.2 Call Sites
- **Block exit**: After all statements, before popping scope
- **Function exit**: Before `ret` (after all blocks)
- **Match arms**: After arm body, before jump to end
- **Loop exit**: After body, before back-edge
- **Early return**: Before return statement

#### 2.3 Codegen: Emit `drop` Calls
```rust
// codegen.rs - in emit_block, emit_function, etc.
fn emit_scope_drops(&mut self, scope_vars: &HashMap<String, VarState>) {
    for (name, var) in scope_vars {
        if var.is_owner && var.is_initialized && !var.is_moved {
            self.emit_call("drop", vec![Expr::Ident(name.clone())])?;
        }
    }
}
```

### Phase 3: Move Semantics Codegen (High Effort)

#### 3.1 Assignment Move
```fsl
VAR a: own[i32] = 10
VAR b: own[i32] = a  ; move: a becomes uninitialized
```

**Codegen:**
```llvm
; b = a
%a_val = load i32, ptr %a
store i32 %a_val, ptr %b
; a = poison (uninitialized)
store i32 poison, ptr %a
```

#### 3.2 Function Call Move
```fsl
FN take(x: own[i32]): void
VAR a: own[i32] = 42
take(a)  ; a moved, cannot use after
```

**Codegen:** Pass pointer directly, mark source as moved in semantic state.

#### 3.3 Return Move
```fsl
FN make(): own[i32]
    VAR x: own[i32] = 42
    RETURN x  ; move out
ENDFN
```

**Codegen:** Return via pointer parameter (LLVM `sret` or out-param).

### Phase 4: Struct Field Destruction (Medium Effort)

#### 4.1 Destructor Order
```fsl
STRUCT S {
    a: own[i32],
    b: own[Vec],
}
```
Fields destroyed in **reverse declaration order** (C++/Rust rule).

#### 4.2 Codegen
```rust
fn emit_struct_drop(&mut self, struct_ptr: &str, fields: &[StructField]) {
    for field in fields.iter().rev() {
        if field.ty.is_owner() {
            let field_ptr = self.emit_gep(struct_ptr, field.index);
            self.emit_call("drop", vec![Expr::Deref(field_ptr)])?;
        }
    }
}
```

### Phase 5: Parameter/Return Convention (Low Effort)

| Scenario | Current | Required |
|----------|---------|----------|
| `FN f(x: own[T])` | Pass as `T*` | Pass `T*`, callee owns |
| `FN f() -> own[T]` | Return `T*` | Use `sret` / out-param |
| `FN f(x: ptr[T])` | Pass as `T*` | Callee borrows |

**Codegen fix:** Mark `own` params/functions with `sret` attribute or explicit out-param.

---

## Codegen Drop Insertion Points

```
emit_function:
  entry:
    alloca params
    emit_block(body)
    emit_scope_drops(current_scope)  ; function exit
    ret

emit_block:
  for stmt in statements:
    emit_statement(stmt)
    if stmt is control flow (return/break/continue):
      emit_scope_drops(current_scope)
  
emit_if:
  emit_cond
  then:
    emit_block(then_branch)
    emit_scope_drops(then_scope)
    br endif
  else:
    emit_block(else_branch)  
    emit_scope_drops(else_scope)
    br endif
  endif:

emit_while:
  head:
    emit_cond
    br body/exit
  body:
    emit_block(body)
    emit_scope_drops(body_scope)
    br head
  exit:

emit_match:
  switch
  case_0:
    emit_block(arm)
    emit_scope_drops(arm_scope)
    br end
  ...
  end:
```

---

## Testing Checklist

### Basic RAII
- [ ] Variable dropped at end of scope
- [ ] Variable dropped at end of function
- [ ] Variable dropped in if/else branches
- [ ] Variable dropped in loop body
- [ ] Variable dropped in match arms

### Move Semantics
- [ ] Move on assignment (`b = a`)
- [ ] Move on function call (`f(a)`)
- [ ] Move on return (`return a`)
- [ ] Moved-from value unusable
- [ ] Double-move detected

### Structs
- [ ] Struct with owned fields dropped
- [ ] Fields destroyed in reverse order
- [ ] Nested structs destroyed correctly

### Edge Cases
- [ ] Early return drops locals
- [ ] Break/continue drops loop locals
- [ ] Panic/unwind (future) drops locals

---

## Dependencies

| Task | Blocks |
|------|--------|
| Destructor syntax + semantic registration | Phase 2, 3 |
| Scope exit drop emission | Phase 3, 4 |
| Move codegen | Phase 3, 4 |
| Struct field destruction | Phase 4 |
| Param/return convention | None (independent) |

---

## Example: Complete RAII Flow

```fsl
STRUCT File {
    handle: own[ptr[u8]],
    name: str,
}

FN open(name: str): own[File]
    VAR f: own[File] = File { handle: ..., name }
    RETURN f
ENDFN

FN close(f: own[File]): void
    ; implicit drop calls File::drop
    ; which closes handle
ENDFN

FN main(): i32
    VAR f: own[File] = open("test.txt")
    ; ... use f
    ; f dropped here automatically
    RETURN 0
ENDFN
```

**Expected LLVM IR:**
```llvm
define i32 @main() {
entry:
  %f = alloca %struct.File
  call %struct.File* @open(i8* @.str.test.txt)
  ; ... body ...
  call void @drop_File(%struct.File* %f)  ; auto-inserted
  ret i32 0
}
```

---

## References

- Rust drop semantics: <https://doc.rust-lang.org/nomicon/dropck.html>
- C++ destructor order: <https://en.cppreference.com/w/cpp/language/destructor>
- LLVM `sret` attribute: <https://llvm.org/docs/LangRef.html#sret-attribute>