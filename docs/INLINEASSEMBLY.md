# Inline Assembly Implementation Plan — FOSSIL

Status: **Phase 4 Complete (Codegen) — Gaps Documented**
Target: Stage One (LLVM-backed compiler)

---

## 1. Syntax Decision: Backtick Operands

The proposed `inlineasm.fossil` uses a cleaner syntax:

```relic
FN main(): i32 
    ASM
        mov rax, `a`
        add rax, `b`
        mov `total`, rax
    ENDASM
ENDFN
```

**Key difference from `minlineAsm.md`**: Backtick expressions `` `expr` `` replace explicit constraint strings `"=r"(var)`.

### Rationale
- **Ergonomic**: Variables referenced directly, no duplicate declaration
- **Type-driven constraints**: Compiler infers LLVM constraints from Carnax types
- **Readable**: Assembly template looks like real assembly
- **Safe**: Compiler validates operand types, register allocation handled by LLVM

---

## 2. Finalized Syntax

### 2.1 Statement-Level Block

```relic
ASM
    <instruction> `operand1`, `operand2`, ...
    <instruction> `operand`
ENDASM
```

### 2.2 Expression-Level (Returns Value)

```relic
VAR result: i64 = ASM
    rdtsc
    ; outputs in rax, rdx combined
ENDASM
```

### 2.3 Operand Rules

| Operand Position | Backtick Syntax | Meaning |
|-----------------|-----------------|---------|
| Source (read) | `` `var` `` | Input operand, type → constraint |
| Destination (write) | `` `var` `` | Output operand, type → constraint |
| Read-write | `` `var` `` (appears on both sides) | `+` constraint |
| Immediate | `` `42` `` or `` `CONSTANT` `` | Immediate constraint `i`/`n` |

### 2.4 Clobbers & Flags (Optional, after ENDASM)

```relic
ASM
    cpuid
ENDASM CLobbers("rax", "rbx", "rcx", "rdx") Flags("volatile")
```

Or as attributes on the block:

```relic
ASM volatile clobbers("rax", "rbx", "rcx", "rdx")
    cpuid
ENDASM
```

---

## 3. Type → Constraint Mapping

| Carnax Type | LLVM Constraint | Register Class |
|-------------|-----------------|----------------|
| `i8`/`u8` | `r` | 8-bit GPR (al, bl, ...) |
| `i16`/`u16` | `r` | 16-bit GPR (ax, bx, ...) |
| `i32`/`u32` | `r` | 32-bit GPR (eax, ebx, ...) |
| `i64`/`u64` | `r` | 64-bit GPR (rax, rbx, ...) |
| `f32` | `f` | XMM (SSE) |
| `f64` | `f` | XMM (SSE) |
| `bool` | `r` | 8-bit GPR |
| `ptr[T]` | `r` | 64-bit GPR |
| `own[ptr[T]]` | `r` | 64-bit GPR (pointer value) |

**Inference rules**:
- Output-only (LHS of mov) → `=r`
- Input-only (RHS of mov) → `r`
- Both read and write → `+r`
- Immediate constant → `i` (or `n` if in range)

---

## 4. Implementation Architecture

### 4.1 AST Additions (`ast.rs`)

```rust
#[derive(Debug, Clone)]
pub enum Statement {
    ...
    AsmBlock(AsmBlock),
}

#[derive(Debug, Clone)]
pub struct AsmBlock {
    pub instructions: Vec<AsmInstruction>,
    pub clobbers: Vec<String>,      // register names
    pub flags: AsmFlags,
    pub is_expr: bool,              // true if used as expression
    pub result_type: Option<Type>,  // for expression form
}

#[derive(Debug, Clone)]
pub struct AsmInstruction {
    pub template: String,           // "mov rax, `a`"
    pub operands: Vec<AsmOperand>,  // parsed from backticks
}

#[derive(Debug, Clone)]
pub struct AsmOperand {
    pub expr: Expr,                 // the Carnax expression inside backticks
    pub is_output: bool,            // appears on LHS
    pub is_input: bool,             // appears on RHS
}

#[derive(Debug, Clone, Default)]
pub struct AsmFlags {
    pub volatile: bool,
    pub intel_syntax: bool,
    pub noreturn: bool,
}
```

### 4.2 Parser (`parser.rs`)

- New keywords: `ASM`, `ENDASM`, `CLOBBERS`, `FLAGS`
- Parse `ASM` ... `ENDASM` as statement
- Parse backtick expressions: `` `expr` `` → `AsmOperand { expr, ... }`
- Detect output vs input by position in instruction template (heuristic: LHS of `,` or `=`)

### 4.3 Semantic Analysis (`semantic.rs`)

- Type-check each operand expression
- Verify `MACHINE` or `NO_MACHINE` with known target
- For expression form: infer result type from output operands
- Track register pressure (warn if too many live variables)

### 4.4 Codegen (`codegen.rs`)

Lower each `ASM` block to LLVM `call asm`:

```llvm
; ASM
;     mov rax, `a`
;     add rax, `b`
;     mov `total`, rax
; ENDASM
; where a:i32, b:i32, total:i32

%0 = call i32 asm sideeffect "mov $0, $1\0Aadd $0, $2\0Amov $3, $0", "=r,r,r,=r"(i32 %a, i32 %b, i32 %total)
```

**Template conversion**:
- `%0`, `%1` → `$0`, `$1` (LLVM uses `$`)
- Backtick operands → numbered placeholders in order
- Newlines → `\0A` in template string

---

## 5. MACHINE-Level Benefits

### 5.1 System Calls Without libc

```relic
FN sys_write(fd: i32, buf: ptr[u8], count: usize): isize
    ASM
        mov rax, 1          ; SYS_write
        mov rdi, `fd`
        mov rsi, `buf`
        mov rdx, `count`
        syscall
        mov `result`, rax
    ENDASM CLobbers("rax", "rdi", "rsi", "rdx", "rcx", "r11") Flags("volatile")
    RETURN result
ENDFN
```

### 5.2 CPU Control & Special Instructions

```relic
FN halt(): void
    ASM hlt ENDASM Flags("volatile", "noreturn")
ENDFN

FN cpuid(leaf: i32): [i32, i32, i32, i32]
    VAR eax: i32 = leaf
    VAR ebx: i32 = 0
    VAR ecx: i32 = 0
    VAR edx: i32 = 0
    ASM
        cpuid
    ENDASM CLobbers("rax", "rbx", "rcx", "rdx")
    RETURN [eax, ebx, ecx, edx]
ENDFN
```

### 5.3 Memory-Mapped I/O (Complements `VOLATILE`)

```relic
FN mmio_write(reg: ptr[u32], val: u32): void
    ASM
        mov eax, `val`
        mov [`reg], eax
    ENDASM CLobbers("rax", "memory") Flags("volatile")
ENDFN
```

### 5.4 Performance-Critical Primitives

```relic
FN atomic_cas(ptr: ptr[i64], expected: i64, desired: i64): bool
    VAR result: i64
    ASM
        lock cmpxchg [`ptr], `desired`
        mov `result`, rax
        setz al
        movzx `result`, al
    ENDASM CLobbers("rax", "memory") Flags("volatile")
    RETURN result != 0
ENDFN
```

### 5.5 Bootloader / Firmware Code

```relic
NO_MACHINE

FN enable_sse(): void
    ASM
        mov rax, cr0
        and rax, ~(1 << 2)   ; clear CR0.EM
        or  rax, 1 << 1      ; set CR0.MP
        mov cr0, rax
        mov rax, cr4
        or  rax, (1 << 9) | (1 << 10)  ; set CR4.OSFXSR | CR4.OSXMMEXCPT
        mov cr4, rax
    ENDASM CLobbers("rax") Flags("volatile")
ENDFN
```

---

## 6. Safety & Validation

### 6.1 Compiler Enforces
- All backtick operands are valid Carnax expressions
- Operand types map to supported constraints
- No duplicate output operands
- Expression form has exactly one output or packable multi-reg output

### 6.2 Programmer Responsibility
- Correct clobber lists (compiler trusts them)
- Valid instruction syntax for target
- No control flow into/out of asm block
- Ownership not violated (assembly doesn't free `own` pointers)

### 6.3 Diagnostics
- Warning: inferred constraint may not match intent
- Error: unsupported type for asm operand
- Error: asm in non-MACHINE/NO_MACHINE without target

---

## 7. Implementation Phases

| Phase | Tasks | Files | Status |
|-------|-------|-------|--------|
| **1. Lexer/Parser** | Keywords, `ASM`...`ENDASM`, backtick parsing | `token.rs`, `parser.rs` | ✅ Done |
| **2. AST** | `AsmBlock`, `AsmInstruction`, `AsmOperand` | `ast.rs` | ✅ Done |
| **3. Semantic** | Type-check operands, validate mode, infer result type | `semantic.rs` | ✅ Done |
| **4. Codegen** | Template → LLVM asm, constraint mapping, clobbers | `codegen.rs` | ✅ Done |
| **5. Testing** | Test files for syscalls, cpuid, atomics, MMIO | `test_asm_*.fsl` | ✅ Basic tests pass |
| **6. Operand Inference** | Template analysis for output/input detection | `parser.rs`, `semantic.rs` | ⏳ Planned |
| **7. Expression Form** | `VAR x = ASM ... ENDASM` return values | `codegen.rs`, `semantic.rs` | ⏳ Planned |
| **8. MACHINE I/O** | STDOUT/STDIN/STDERR via syscalls | `src/machine/`, `core/io.fsl` | ⏳ Planned |

---

## 8. Current Gaps & Limitations

### 8.1 Operand Direction Not Inferred (Critical)
**Problem**: Parser sets `is_output=false, is_input=false` for all backtick operands.
**Result**: All operands get `"r"` (read-only) constraint. Output operands need `"=r"`, read-write need `"+r"`.
**Impact**: Templates like `mov rax, `a`` work by accident (memory operand), but `mov `result`, rax` fails — result not written back.

**Fix**: Add template analysis in parser (`parse_asm_instruction`):
- Detect LHS of `mov` / `=` → `is_output=true`
- Detect RHS of `mov` / `,` → `is_input=true`
- Detect same variable on both sides → `is_output=true, is_input=true` (`+r`)

### 8.2 Explicit Register Names in Templates (Critical)
**Problem**: Templates using `rax`, `rdi`, etc. conflict with LLVM register allocation.
**Result**: Clang error "undefined symbol: rax" or AT&T register names (`%eax`) appear in Intel-syntax template.

**Workaround**: Use memory operands only:
```relic
; Works: mov `mem`, `val`  (memory-to-memory via register)
; Fails: mov rax, `val`   (explicit register)
```

**Fix Options**:
1. **Template rewrite**: Convert Intel register names to AT&T in template before LLVM (`rax` → `%rax`)
2. **Fixed register constraints**: Support `={rax}` syntax in backticks: `` `out:rax` ``
3. **Hybrid**: Use memory operands for inputs/outputs, clobber explicit registers

### 8.3 Expression Form Not Implemented
**Problem**: `VAR x = ASM ... ENDASM` not handled in codegen.
**Fix**: In `emit_asm_block`, when `asm.is_expr`, generate `call <ty> asm ...` with output constraint and assign to SSA register.

### 8.4 AT&T Syntax Not Tested
**Problem**: Only `#intel` flag tested. AT&T syntax (`%rax`, `$imm`) may have different constraint requirements.

### 8.5 No Label/Goto Support
Local labels (`.L1:`) and `goto` asm not implemented. Deferred to post-1.0.

---

## 9. Plan: MACHINE I/O (STDOUT, STDIN, STDERR)

### 9.1 Design Goal
Implement zero-libc I/O for `MACHINE` programs using inline assembly syscalls.
Target: Windows (x86_64) via `syscall` instruction (NT syscall interface) or `int 0x2e` (legacy).
**Primary**: Use Windows `WriteFile`/`ReadFile` via `kernel32.dll` imports for compatibility.
**Fallback**: Direct syscalls for `NO_MACHINE` / bare-metal.

### 9.2 Architecture

```
MACHINE program
    │
    ├── USE MACHINE::IO
    │       │
    │       ├── FN print(s: str) → void
    │       ├── FN println(s: str) → void
    │       ├── FN read_line(buf: ptr[u8], len: usize) → isize
    │       ├── FN write(fd: i32, buf: ptr[u8], len: usize) → isize
    │       └── FN exit(code: i32) → noreturn
    │
    ├── Inline ASM syscall wrappers (internal)
    │       ├── syscall0/1/3/6 (generic)
    │       └── Windows: GetStdHandle, WriteFile, ReadFile, ExitProcess
    │
    └── EXTERN declarations for kernel32.dll
            GetStdHandle, WriteFile, ReadFile, ExitProcess
```

### 9.3 Windows Syscall Strategy

**Option A: Import from kernel32.dll (Recommended for MACHINE)**
```relic
MACHINE

EXTERN FN GetStdHandle(nStdHandle: i32): ptr[u8] ENDFN
EXTERN FN WriteFile(hFile: ptr[u8], lpBuffer: ptr[u8], nNumberOfBytesToWrite: u32,
                    lpNumberOfBytesWritten: ptr[u32], lpOverlapped: ptr[u8]): i32 ENDFN
EXTERN FN ReadFile(hFile: ptr[u8], lpBuffer: ptr[u8], nNumberOfBytesToRead: u32,
                   lpNumberOfBytesRead: ptr[u32], lpOverlapped: ptr[u8]): i32 ENDFN
EXTERN FN ExitProcess(uExitCode: u32): void ENDFN

; STD_HANDLE constants
CONST STD_INPUT_HANDLE: i32 = -10
CONST STD_OUTPUT_HANDLE: i32 = -11
CONST STD_ERROR_HANDLE: i32 = -12
```

**Option B: Direct syscalls (NO_MACHINE / bare metal)**
```relic
NO_MACHINE

FN sys_write(fd: i32, buf: ptr[u8], len: usize): isize
    ASM
        mov rax, 1          ; Linux x86_64 SYS_write (not Windows)
        mov rdi, `fd`
        mov rsi, `buf`
        mov rdx, `len`
        syscall
        mov `result`, rax
    ENDASM CLobbers("rax", "rdi", "rsi", "rdx", "rcx", "r11") Flags("volatile")
    RETURN result
ENDFN
```

### 9.4 Implementation Steps

| Step | Task | Files | Est. Effort |
|------|------|-------|-------------|
| **9.4.1** | Add `EXTERN` support for Windows API (kernel32.dll) | `parser.rs`, `codegen.rs`, `semantic.rs` | 2 days |
| **9.4.2** | Create `src/machine/win32.rs` with constant definitions | `src/machine/win32.rs` | 1 day |
| **9.4.3** | Implement `MACHINE::IO` built-in module | `src/machine/io.fsl` (or `src/builtins/io.rs`) | 2 days |
| **9.4.4** | Add `USE MACHINE::IO` to import resolution | `semantic.rs`, `driver.rs` | 1 day |
| **9.4.5** | Write integration tests: hello world, echo, cat | `tests/integration_tests.rs`, `test_io_*.fsl` | 1 day |
| **9.4.6** | Fix operand inference (Gap 8.1) for reliable syscall wrappers | `parser.rs`, `semantic.rs` | 2 days |

**Total: ~9 days**

### 9.5 MACHINE::IO API Design

```relic
; core/io.fsl (built-in, auto-imported with USE MACHINE::IO)
MACHINE

; Low-level Windows API
EXTERN FN GetStdHandle(nStdHandle: i32): ptr[u8] ENDFN
EXTERN FN WriteFile(hFile: ptr[u8], lpBuffer: ptr[u8], nNumberOfBytesToWrite: u32,
                    lpNumberOfBytesWritten: ptr[u32], lpOverlapped: ptr[u8]): i32 ENDFN
EXTERN FN ReadFile(hFile: ptr[u8], lpBuffer: ptr[u8], nNumberOfBytesToRead: u32,
                   lpNumberOfBytesRead: ptr[u32], lpOverlapped: ptr[u8]): i32 ENDFN
EXTERN FN ExitProcess(uExitCode: u32): void ENDFN

CONST STDIN: i32 = -10
CONST STDOUT: i32 = -11
CONST STDERR: i32 = -12

; High-level API
FN stdout_handle(): ptr[u8]
    RETURN GetStdHandle(STD_OUTPUT_HANDLE)
ENDFN

FN stderr_handle(): ptr[u8]
    RETURN GetStdHandle(STD_ERROR_HANDLE)
ENDFN

FN stdin_handle(): ptr[u8]
    RETURN GetStdHandle(STD_INPUT_HANDLE)
ENDFN

FN write_bytes(handle: ptr[u8], buf: ptr[u8], len: usize): isize
    VAR written: u32 = 0
    VAR result: i32 = WriteFile(handle, buf, len as u32, &written, null)
    IF result == 0
        RETURN -1  ; error
    ENDIF
    RETURN written as isize
ENDFN

FN read_bytes(handle: ptr[u8], buf: ptr[u8], len: usize): isize
    VAR read: u32 = 0
    VAR result: i32 = ReadFile(handle, buf, len as u32, &read, null)
    IF result == 0
        RETURN -1
    ENDIF
    RETURN read as isize
ENDFN

; Convenience
FN print(s: str): void
    VAR h = stdout_handle()
    write_bytes(h, s.ptr, s.len)
ENDFN

FN println(s: str): void
    print(s)
    print("\n")
ENDFN

FN eprint(s: str): void
    VAR h = stderr_handle()
    write_bytes(h, s.ptr, s.len)
ENDFN

FN read_line(buf: ptr[u8], max_len: usize): isize
    VAR h = stdin_handle()
    RETURN read_bytes(h, buf, max_len)
ENDFN

FN exit(code: i32): void
    ExitProcess(code as u32)
    UNREACHABLE
ENDFN
```

### 9.6 Usage Example

```relic
USE MACHINE::IO

FN main(): i32
    println("Hello, World!")
    eprint("Error message\n")
    
    VAR buf: [u8, 256]
    VAR n = read_line(&buf, 256)
    IF n > 0
        print("You typed: ")
        write_bytes(stdout_handle(), &buf, n as usize)
        println("")
    ENDIF
    
    exit(0)
ENDFN
```

---

## 10. Open Decisions

1. **Clobber syntax**: `CLobbers(...)` after `ENDASM` vs attribute before `ASM` → **Current: after ENDASM**
2. **Intel vs AT&T syntax**: Default to Intel (matches example), flag for AT&T → **Current: `#intel` forced**
3. **Multiple instructions**: Single template with `\n` vs multiple `ASM` lines → **Current: multiple lines**
4. **Label support**: Local labels `.L1:` inside asm — allow? → **Deferred**
5. **Goto form**: Defer to post-1.0 → **Deferred**
6. **Windows vs Linux syscalls**: Use kernel32.dll imports for MACHINE, raw syscalls for NO_MACHINE → **Decided**
7. **Operand inference heuristic**: Template parsing vs explicit annotation → **TBD in Phase 6**

---

## 11. Example: Complete I/O Library in FOSSIL

With inline asm + extern + volatile, a full `core::io` package becomes possible:

```relic
; core/io.fsl
MACHINE

EXTERN FN malloc(size: usize): ptr[u8] ENDFN
EXTERN FN free(p: ptr[u8]): void ENDFN

FN syscall0(nr: i64): i64
    ASM
        mov rax, `nr`
        syscall
        mov `result`, rax
    ENDASM CLobbers("rax", "rcx", "r11", "memory") Flags("volatile")
    RETURN result
ENDFN

FN syscall3(nr: i64, a1: i64, a2: i64, a3: i64): i64
    ASM
        mov rax, `nr`
        mov rdi, `a1`
        mov rsi, `a2`
        mov rdx, `a3`
        syscall
        mov `result`, rax
    ENDASM CLobbers("rax", "rdi", "rsi", "rdx", "rcx", "r11", "memory") Flags("volatile")
    RETURN result
ENDFN

FN write(fd: i32, buf: ptr[u8], len: usize): isize
    RETURN syscall3(1, fd as i64, buf as i64, len as i64) as isize
ENDFN

FN read(fd: i32, buf: ptr[u8], len: usize): isize
    RETURN syscall3(0, fd as i64, buf as i64, len as i64) as isize
ENDFN
```

This achieves **zero libc dependency** for MACHINE programs — the core principle of FOSSIL.

---

## 12. Test Cases to Add

| Test | Description | Expected |
|------|-------------|----------|
| `test_asm_nop.fsl` | `ASM nop ENDASM` | ✅ Passes |
| `test_asm_memory.fsl` | `mov [mem], reg` / `mov reg, [mem]` | ✅ Passes |
| `test_asm_clobber.fsl` | `CLobbers("rax")` | ✅ Passes |
| `test_asm_output_operand.fsl` | `mov `out`, rax` | ❌ Fails (Gap 8.1) |
| `test_asm_fixed_register.fsl` | `mov rax, `a`` | ❌ Fails (Gap 8.2) |
| `test_asm_expr.fsl` | `VAR x = ASM rdtsc ENDASM` | ❌ Not implemented |
| `test_io_hello.fsl` | `USE MACHINE::IO; println("hi")` | ❌ Not implemented |
| `test_io_echo.fsl` | Read line, write back | ❌ Not implemented |
| `test_io_stderr.fsl` | `eprint("err")` | ❌ Not implemented |

---

*Last Updated: 2026-11-17*
*Status: Phase 4 Codegen complete. Gaps documented. MACHINE I/O planned.*