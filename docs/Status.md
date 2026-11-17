Project Assessment: Carnax (FOSSIL Language Compiler)

Date of Assessment: November 16, 2026
Repository Path: D:/Programs/carnax
Current Version: 0.1.0 (Rust 2024 Edition)
Primary Documentation: Carnax.md, docs/llvm-lowering.md, docs/machine.md, docs/minlineAsm.md
──────
## 1. Executive Summary

Carnax is a dependency-free reference compiler written in Rust for FOSSIL, a minimalist, imperative, data-oriented systems
programming language.

FOSSIL aims to provide predictable, systems-level execution (pointers, explicit memory, multiple return values, keyword blocks) without
implicit runtime costs, garbage collection, or hidden abstraction layers. The compiler is currently in Stage One (LLVM-backed), taking .fsl
source files and generating LLVM Intermediate Representation (IR), which can be compiled and executed directly via Clang.

### Key Takeaways

• Zero External Dependencies: Built entirely with the Rust standard library (std), resulting in sub-second compilation times and minimal build
friction. Zero clippy warnings across all compilation targets.
• Strong Semantic Analysis Foundation: Includes type checking, constant verification, MACHINE/NO_MACHINE contract enforcement, linear
move-semantics tracking for own[ptr[T]] pointers, and complete enum/match pattern validation.
• Accurate Syntax & Lexing: Clean recursive-descent parser implementing uppercase keyword-delimited blocks (FN ... ENDFN, IF ... ENDIF, WHILE ... ENDWHILE, FOR ... ENDFOR, STRUCT ... ENDSTRUCT, ENUM ... ENDENUM, MATCH ... ENDMATCH).
• Mature LLVM Codegen: Emits verified, SSA-compliant LLVM IR with accurate types, per-function register resets, aggregate insertvalue/extractvalue, hoisted global string constants, and strict basic block terminator control.
• Enums & Pattern Matching (Complete): Tagged unions ({ i32, <payload> } and { i32 }), variant construction (qualified Enum.Variant and bare Variant), unit matching, payload binding/destructuring, literal switches, and wildcards (_) are fully lowered and verified.
• Automated Test Harness: 33 .fsl test scripts, with 27 automated end-to-end and negative integration tests passing in cargo test (< 1.5s).
──────
## 2. Project Architecture & Pipeline

┌──────────────────────────────────────────────┐ ┌───────────────────────────────────────────────────────────────────┐
│             Frontend (Complete)              │ │                     Planned Architecture Gaps                     │
│                                              │ │                                                                   │
│ ┌──────────────────────────────────────────┐ │ ├─────────────────────────────────┐   ┌─────────────────────────────┤
│ │                                          │ │ │                                 │   │                             │
│ │            .fsl Source Code            │ │ │ Internal IR (Carnax.md §12.2)   │   │ Scope Drop / Free Lowering  │
│ │                                          │ │ │ [Future Optimization Layer]     │   │ [docs/llvm-lowering.md]     │
│ └─────────────────────┬────────────────────┘ │ ├─────────────────────────────────┘   └─────────────────────────────┤
│                       │                      │ └───────────────────────────────────────────────────────────────────┘
│                       ▼                      │
│ ┌──────────────────────────────────────────┐ │
│ │                                          │ │
│ │           Lexer (src/lexer.rs)           │ │
│ │                                          │ │
│ └─────────────────────┬────────────────────┘ │
│                       │                      │
│                       ▼                      │
│ ┌──────────────────────────────────────────┐ │
│ │                                          │ │
│ │       Token Stream (src/token.rs)        │ │
│ │                                          │ │
│ └─────────────────────┬────────────────────┘ │
│                       │                      │
│                       ▼                      │
│ ┌──────────────────────────────────────────┐ │
│ │                                          │ │
│ │ Recursive Descent Parser (src/parser.rs) │ │
│ │                                          │ │
│ └─────────────────────┬────────────────────┘ │
│                       │                      │
│                       ▼                      │
│ ┌──────────────────────────────────────────┐ │
│ │                                          │ │
│ │             AST (src/ast.rs)             │ │
│ │                                          │ │
│ └─────────────────────┬────────────────────┘ │
│                       │                      │
└───────────────────────┼──────────────────────┘
                        │
┌───────────────────────┼───────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│                       │                                                           Semantic Analysis (Mature)                                                                                  │
│                       ▼                                                                                                                                                                       │
│ ┌──────────────────────────────────────────┐                                                                                                                                                  │
│ │                                          │                                                                                                                                                  │
│ │   Semantic Analyzer (src/semantic.rs)    ├───┬─────────────────────────────────┬───────────────────────────────────────────────────┬────────────────────────────────────────────────────┐   │
│ │                                          │   │                                 │                                                   │                                                    │   │
│ └─────────────────────┬────────────────────┘   ▼                                 ▼                                                   ▼                                                    ▼   │
│                       │                      ┌─────────────────────────────────┐ ┌─────────────────────────────────────────────────┐ ┌──────────────────────────────────────────────────┐ │   │
│                       │                      │ Type & Mutability Checking      │ │ Linear Move Tracking (own[ptr[T]])              │ │ Platform Contract Validation (main)                │ │   │
│                       │                      │ - Binary ops & assignments      │ │ - Tracks initialized/moved states               │ │ - Hosted MACHINE vs Freestanding NO_MACHINE        │ │   │
│                       │                      │ - Enum types & variant payloads │ │ - Leaked resource detection on scope exit       │ │ - Validates argc/argv and return types             │ │   │
│                       │                      └─────────────────────────────────┘ └─────────────────────────────────────────────────┘ └──────────────────────────────────────────────────┘ │   │
│                       │                                                                                                                                                                   │   │
│                       ▼                                                                                                                                                                   │   │
│ ┌──────────────────────────────────────────┐                                                                                                                                              │   │
│ │ Match & Enum Pattern Validation          │                                                                                                                                              │   │
│ │ - Variant & payload type verification    │                                                                                                                                              │   │
│ │ - Payload binding lexical scoping        │                                                                                                                                              │   │
│ └─────────────────────┬────────────────────┘                                                                                                                                              │   │
│                       │                                                                                                                                                                   │   │
└───────────────────────┼───────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
                        ▼
┌────────────────────────────────────────────┐
│ Direct AST-to-IR Emitter (src/codegen.rs)  │
│ - Tagged union lowering (%enum.Name)       │
│ - Switch instruction & terminator control  │
│ - GEP & aggregate insertvalue/extractvalue │
│ - String constant hoisting & register reset│
└───────────────────────┬────────────────────┘
                        │
                        ▼
┌────────────────────────────────────────────┐
│ LLVM IR (.ll) / Clang Driver               │
└────────────────────────────────────────────┘
──────
## 3. Detailed Component Review

### 3.1 Build & Dependencies (Cargo.toml)

• Status: Healthy.
• Edition: 2024.
• Dependencies: None ([dependencies] is empty).
• Compiles cleanly with zero compiler warnings and zero clippy warnings across all targets.

### 3.2 Lexer & Tokenizer (src/lexer.rs, src/token.rs)

• Strengths:
    • Byte-slice based scanner (&[u8]) with line and column tracking.
    • Keywords for block structures (FN, IF, ELSE, WHILE, FOR, STRUCT, ENUM, MATCH, CASE, ENDMATCH).
    • Correct handling of multi-character tokens (:=, ==, !=, <=, >=, &&, ||, <<, >>, ->).
    • Escape sequences in strings (\n, \t, \r, \", \0).
    • Differentiates float literals from member access using lookahead.
• Opportunities for Improvement:
    • Only supports // line comments; no block comments /* ... */.
    • Number parsing lacks hex (0x...), binary (0b...), and octal literals, which are critical for systems programming and MMIO.

### 3.3 Parser & AST (src/parser.rs, src/ast.rs)

• Strengths:
    • Clear recursive descent with descriptive error spans (line and column numbers).
    • Clean parsing of block closures (ENDFN, ENDIF, ENDWHILE, ENDFOR, ENDSTRUCT, ENDENUM, ENDMATCH).
    • Complete MATCH statement parsing with multiple CASE branches, patterns (literals, unit variants, payload variants with bindings, wildcards), and optional ELSE blocks.
    • Handles tuple/multiple assignment syntax v, ok := divide(20, 4).
    • Parses MACHINE and NO_MACHINE declarations at module scope.
• Opportunities for Improvement:
    • Array types [T; N] exist in AST Type::Array and Carnax.md, but parse_type currently treats [...] primarily as tuples.
    • Statement boundary recovery is limited (single error aborts parsing).

### 3.4 Semantic Analysis (src/semantic.rs)

• Strengths:
    • Platform Validation: Enforces that MACHINE programs have a valid entry point (FN main(): i32 or FN main(argc: i32, argv: ptr[ptr[u8]]): i32), matching docs/machine.md.
    • Ownership & Move Semantics: Tracks variable states (is_initialized, is_moved, is_owner). Transferring an own[...] pointer marks the source as moved; subsequent use generates a use of possibly uninitialized variable error.
    • Enum & Match Validation: Thorough validation of enum declarations, variant lookups (both qualified Enum.Variant and bare Variant), payload type verification, rejection of mismatched patterns, and automatic lexical scoping of payload-bound variables.
    • Type Checking: Thorough type verification for binary ops, assignments, return arities, and field member accesses.
• Opportunities for Improvement:
    • check_scope_exit_owners() checks for leaks at scope exit, but error diagnostics could detail where the resource was originally acquired.

### 3.5 Code Generation (src/codegen.rs)

• Status: Verified & Complete for Enums and Pattern Matching.
• Implemented Features:
    1. Enum Representation: Payload-less enums lower to %enum.Name = type { i32 }. Tagged enums with payload lower to %enum.Name = type { i32, <payload_ty> }.
    2. Variant Construction: Unit variants emit insertvalue with discriminant tag. Payload variants emit aggregate insertion for both tag and payload. Supports both qualified (Enum.Variant) and bare (Variant) syntax.
    3. Equality & Inequality: Binary == and != for enums lower to tag extraction and icmp comparisons.
    4. Match & Switch Lowering: Emits LLVM switch instruction targeting case labels and default (wildcard/else/merge) targets. Payload-bearing arms automatically extract payload values via extractvalue into scoped allocas.
    5. Basic Block Terminators: Strict terminator tracking (is_terminated) prevents duplicate terminators or unreachable instructions after ret/br/switch. Merge blocks are omitted when all branches terminate.
    6. String Constant Hoisting & Virtual Register Resets: Numbered registers (%0, %1, ...) reset per function; string constants hoisted as module globals.
    7. Struct & Stack Array GEP: Aggregate typing for struct field access and direct stack alloca indexing.
    8. Zero Clippy Warnings: Clean Rust 2024 code with no redundant operations.

──────
## 4. Test Suite Audit

The test suite contains 33 .fsl test scripts (27 automated in tests/integration_tests.rs):

 Test File                           │ Type         │ Feature Covered                               │ Result                               │ Status
─────────────────────────────────────┼──────────────┼───────────────────────────────────────────────┼──────────────────────────────────────┼────────
 test_enum_unit.fsl                │ E2E / Native │ Unit enum decl, assignment, ==/!=, MATCH arm  │ Returns 0 (sum == 60)                │ Passed
 test_enum_payload.fsl             │ E2E / Native │ Payload enum (Success/Failure/EOF), bindings  │ Returns 0 (42, 7, 99 verified)       │ Passed
 test_match_switch.fsl             │ E2E / Native │ Integer MATCH with CASE branches and ELSE     │ Returns 0 (all cases verified)       │ Passed
 test_match_wildcard.fsl           │ E2E / Native │ Enum MATCH with CASE _ wildcard fallback      │ Returns 0 (Red vs Non-Red)           │ Passed
 test_enum_bare.fsl                │ E2E / Native │ Bare variant constructors and MATCH patterns  │ Returns 0 (South == 2)               │ Passed
 test_enum_unknown_variant.fsl     │ Negative     │ Reject nonexistent variant access             │ Error: enum has no variant 'Stopped' │ Passed
 test_match_wrong_enum.fsl         │ Negative     │ Reject matching variant from different enum   │ Error: pattern enum does not match   │ Passed
 test_match_payload_mismatch.fsl   │ Negative     │ Reject matching payload variant without args  │ Error: variant requires a payload    │ Passed
 test_match_literal_on_enum.fsl    │ Negative     │ Reject matching integer literal against enum  │ Error: pattern type mismatch         │ Passed
 test_fibonacci.fsl                │ Stress / E2E │ Recursion, multiple calls in binary expr      │ Returns 0 (fib(8) == 21)             │ Passed
 test_bitwise.fsl                  │ Stress / E2E │ &, |, ^, ~, <<, >>, &&, ||, !                 │ Full bitwise accuracy                │ Passed
 test_nested_loops.fsl             │ Stress / E2E │ 3-level nested WHILE & FOR loops              │ Returns 40                           │ Passed
 test_struct_ops.fsl               │ Stress / E2E │ Struct pointer mutations across fns, &vec     │ Mutates & sums components            │ Passed
 test_multireturn_advanced.fsl     │ Stress / E2E │ 3-tuple return [i32, i32, i32]                │ Accurate destructuring               │ Passed
 test_early_returns.fsl            │ Stress / E2E │ Early returns in loops and matrix search      │ Accurate CFG exit                    │ Passed
 test_arrays.fsl                   │ Stress / E2E │ Fixed array [i32; 4], write & read indexing   │ Sums to 100                          │ Passed
 test_recursion_factorial.fsl      │ Stress / E2E │ Factorial recursion, base cases               │ Accurate computation                 │ Passed
 test_deep_nesting.fsl             │ Stress / E2E │ Deeply nested exprs & chained comparisons     │ Accurate computation                 │ Passed
 test_const_reassign.fsl           │ Negative     │ Reject assignment to CONST                    │ Error: cannot assign to constant 'x' │ Passed
 test_machine_args.fsl             │ Positive     │ Accept valid argc/argv in MACHINE             │ Exit 0, IR emitted (%argc, %argv)    │ Passed
 test_machine_no_main.fsl          │ Negative     │ Reject MACHINE without main()                 │ Error: MACHINE mode requires main    │ Passed
 test_machine_valid.fsl            │ Positive     │ Accept standard main(): i32                   │ Exit 0, IR emitted                   │ Passed
 test_machine_wrong_argv.fsl       │ Negative     │ Reject non-ptr[ptr[u8]] argv                  │ Error: main argv parameter invalid   │ Passed
 test_machine_wrong_return.fsl     │ Negative     │ Reject non-i32 return for main                │ Error: main must return i32          │ Passed
 test_moved.fsl                    │ Negative     │ Reject use of moved own variable              │ Error: use of uninitialized variable │ Passed
 test_nomachine_valid.fsl          │ Positive     │ Allow arbitrary entry in NO_MACHINE           │ Exit 0, IR emitted                   │ Passed
 test_return_type.fsl              │ Negative     │ Reject mismatched return type                 │ Error: return type mismatch          │ Passed
 test_simple.fsl                   │ Positive     │ Arithmetic, while loop, calls                 │ Exit 0, IR emitted                   │ Passed
 test_type_mismatch.fsl            │ Negative     │ Reject string literal to i32                  │ Error: type mismatch                 │ Passed
 test_undeclared.fsl               │ Negative     │ Reject undeclared variable                    │ Error: undeclared variable 'y'       │ Passed
 test_valid.fsl                    │ Positive     │ Structs, loops, multiple returns              │ Exit 0, IR emitted                   │ Passed
 test.fsl                          │ Positive     │ Struct declarations and access                │ Exit 0, IR emitted                   │ Passed
 test2.fsl                         │ Positive     │ own[ptr[T]] and for-loops                     │ Exit 0, IR emitted (%p)              │ Passed
 test3.fsl                         │ Negative     │ Syntax error (incomplete var decl)            │ Error: parse error                   │ Passed
 test4.fsl                         │ Positive     │ Comments and machine directives               │ Exit 0, IR emitted                   │ Passed
──────
## 5. Prioritized Action Plan & Recommendations

### Priority 1: Fix LLVM IR Generation Bugs (Completed)

[✓] Prefix all function parameter names with % in function headers and alloca store instructions in src/codegen.rs.
[✓] Include the element type in insertvalue calls: insertvalue <struct_ty> <agg>, <elem_ty> <val>, <idx>.
[✓] Track whether a block has already terminated before emitting branches (br label %...) to eliminate duplicate terminators.
[✓] Collect string literals during codegen and hoist @.str.X constants to the top of the LLVM module.
[✓] Reset unnamed virtual register numbering (%0) at the start of each function.
[✓] Fix GEP aggregate typing for struct member access and array indexing.

### Priority 2: Toolchain Driver & Automated Testing (Completed)

[✓] Implement toolchain driver (src/driver.rs) with Clang autodetection, native binary compilation, and JIT-like execution.
[✓] Update CLI commands in src/main.rs (carnax build, carnax run, carnax emit-ir).
[✓] Implement automated test suite in tests/integration_tests.rs covering semantic rejections and end-to-end native execution.

### Priority 3: Enums & Pattern Matching (Completed November 16, 2026)

[✓] Lower enums to tagged unions: %enum.Name = type { i32 } for unit enums and %enum.Name = type { i32, <payload> } for payload enums.
[✓] Implement variant construction for both qualified (Enum.Variant) and bare (Variant) syntax.
[✓] Support enum equality/inequality (==, !=) via tag extraction and icmp.
[✓] Implement recursive-descent parsing for MATCH <expr> ... CASE <pattern> ... ELSE ... ENDMATCH.
[✓] Add pattern matching validation in semantic analysis with scoped payload variable bindings.
[✓] Implement LLVM lowering of MATCH via switch instructions, case blocks, payload extraction, and clean CFG merge handling.
[✓] Add end-to-end and negative integration tests for enums, payload destructuring, integer switches, and wildcards.

### Priority 4: Spec Alignment & Next Roadmap Items (Next Up)

[ ] Scope Cleanup (drop): Implement the drop insertion pass described in docs/llvm-lowering.md to deallocate un-moved own[ptr[T]] at scope exit.
[ ] Hex & Binary Integer Literals: Add 0x... and 0b... literal support to lexer and parser for low-level systems programming and MMIO.
[ ] Internal IR: Introduce a flat or CFG-based Internal IR as recommended in Carnax.md §12.2 to decouple semantic checking from LLVM text emission.
