# Phase 8: Testing & Integration

## Goal
Verify all CORE modules work in isolation and through MACHINE. Ensure zero regressions.

---

## 8.1 Test Categories

### 8.1.1 CORE Unit Tests (`NO_MACHINE` mode)
```fsl
; tests/core/test_mem.fsl
NO_MACHINE
USE CORE.mem

FN main(): i32
    VAR a: [u8; 16] = [0; 16]
    VAR b: [u8; 16] = [42; 16]
    memcpy(a.ptr, b.ptr, 16)
    VAR eq = memcmp(a.ptr, b.ptr, 16)
    RETURN if eq == 0 { 0 } else { 1 }
ENDFN
```

```fsl
; tests/core/test_slice.fsl
NO_MACHINE
USE CORE.slice

FN main(): i32
    VAR arr: [i32; 4] = [1, 2, 3, 4]
    VAR s = slice_from_array(&arr)
    VAR v = s.get(2).unwrap()
    RETURN if v == 3 { 0 } else { 1 }
ENDFN
```

```fsl
; tests/core/test_err.fsl
NO_MACHINE
USE CORE.err

FN div(a: i32, b: i32): Result[i32, Error]
    IF b == 0
        RETURN Err(error_new(ErrorKind.InvalidInput, "div by zero"))
    ENDIF
    RETURN Ok(a / b)
ENDFN

FN main(): i32
    MATCH div(10, 2)
        CASE Ok(v) => RETURN if v == 5 { 0 } else { 1 }
        CASE Err(_) => RETURN 1
    ENDMATCH
ENDFN
```

```fsl
; tests/core/test_fmt.fsl
NO_MACHINE
USE CORE.fmt
USE CORE.io

FN main(): i32
    VAR w = NullWriter {}
    writeln(w, "Hello {} {}", 42, 3.14)
    RETURN 0
ENDFN
```

```fsl
; tests/core/test_panic.fsl
NO_MACHINE
USE CORE.panic

FN main(): i32
    assert(1 == 1, "should pass")
    assert(false, "should fail")  ; should panic
    RETURN 0
ENDFN
```

```fsl
; tests/core/test_alloc.fsl
NO_MACHINE
USE CORE.alloc
USE CORE.err

STRUCT MyAlloc
ENDSTRUCT

IMPL CORE.alloc.Allocator FOR MyAlloc
    FN alloc(self: ptr[void], size: usize, align: usize): Result[ptr[u8], Error]
        RETURN Err(error_new(ErrorKind.Other, "not implemented"))
    ENDFN
    FN dealloc(self: ptr[void], ptr: ptr[u8], size: usize, align: usize): void
    ENDFN
ENDIMPL

FN main(): i32
    VAR a = MyAlloc {}
    MATCH a.alloc(16, 8)
        CASE Err(_) => RETURN 0
        CASE Ok(_) => RETURN 1
    ENDMATCH
ENDFN
```

---

### 8.1.2 MACHINE Integration Tests
```fsl
; tests/machine/test_io.fsl
MACHINE
USE MACHINE.io

FN main(): i32
    println("Hello from MACHINE!")
    eprintln("Error output")
    RETURN 0
ENDFN
```

```fsl
; tests/machine/test_alloc.fsl
MACHINE
USE MACHINE.alloc
USE MACHINE.core

FN main(): i32
    init_global_allocator()
    VAR b = box_new(42).unwrap()
    RETURN *b
ENDFN
```

```fsl
; tests/machine/test_file.fsl
MACHINE
USE MACHINE.file

FN main(): i32
    VAR f = open("test.txt", OpenMode.Write).unwrap()
    f.write_str("hello").unwrap()
    f.flush().unwrap()
    f.close().unwrap()
    RETURN 0
ENDFN
```

---

### 8.1.3 Backward Compatibility Tests
```fsl
; tests/compat/test_old_io.fsl
MACHINE
USE MACHINE::IO

FN main(): i32
    IO.println("Old style still works")
    IO.Conout.writeln_str("Direct access")
    RETURN 0
ENDFN
```

---

## 8.2 Test Runner Integration

### Add to `tests/integration_tests.rs`
```rust
#[test]
fn test_core_mem() {
    let exit_code = build_source(&ex("core/test_mem.fsl"), None, true, &[]).expect("build failed");
    assert_eq!(exit_code, 0);
}

#[test]
fn test_core_slice() { ... }

#[test]
fn test_core_err() { ... }

#[test]
fn test_core_fmt() { ... }

#[test]
fn test_core_panic() {
    // Expects non-zero exit (panic)
    let exit_code = build_source(&ex("core/test_panic.fsl"), None, true, &[]).expect("build failed");
    assert_ne!(exit_code, 0);
}

#[test]
fn test_machine_io() { ... }

#[test]
fn test_machine_alloc() { ... }

#[test]
fn test_compat_old_io() { ... }
```

---

## 8.3 CI Configuration

### GitHub Actions (`.github/workflows/ci.yml`)
```yaml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Install LLVM
        run: choco install llvm --version 17.0.6
      - name: Build
        run: cargo build --release
      - name: Test
        run: cargo test
      - name: Test CORE examples
        run: |
          cargo run -- build examples/core/test_mem.fsl
          cargo run -- build examples/core/test_fmt.fsl
      - name: Test MACHINE examples
        run: |
          cargo run -- build examples/machine/test_io.fsl
          cargo run -- build examples/machine/test_alloc.fsl
```

---

## 8.4 Regression Checklist

### Existing Tests Must Pass
- [ ] `test_end_to_end_native_execution_simple`
- [ ] `test_end_to_end_native_execution_fibonacci`
- [ ] `test_end_to_end_native_execution_struct_ops`
- [ ] `test_end_to_end_native_execution_enum_unit`
- [ ] `test_end_to_end_native_execution_enum_payload`
- [ ] `test_end_to_end_native_execution_match_switch`
- [ ] `test_end_to_end_native_execution_match_wildcard`
- [ ] `test_end_to_end_native_execution_enum_bare`
- [ ] `test_end_to_end_native_execution_multireturn`
- [ ] `test_end_to_end_native_execution_early_returns`
- [ ] `test_end_to_end_native_execution_nested_loops`
- [ ] `test_end_to_end_native_execution_arrays` (known bug)
- [ ] `test_end_to_end_native_execution_bitwise`
- [ ] `test_end_to_end_native_execution_test4`
- [ ] `test_end_to_end_native_execution_valid`
- [ ] All negative tests (`test_negative_*`)

### New Functionality Tests
- [ ] CORE memory intrinsics
- [ ] CORE slice/Str operations
- [ ] CORE error handling (Result/Option)
- [ ] CORE formatting (format!/writeln!)
- [ ] CORE panic/assert
- [ ] CORE allocator trait
- [ ] CORE atomics (if implemented)
- [ ] MACHINE Write impl for handles
- [ ] MACHINE libc allocator
- [ ] MACHINE thread/time/file/process
- [ ] Backward compat: IO.println, IO.Conout, etc.

---

## 8.5 Performance Benchmarks

### Microbenchmarks (optional)
```fsl
; bench/bench_fmt.fsl
NO_MACHINE
USE CORE.fmt
USE CORE.io

FN bench_format(n: usize): u64
    VAR w = NullWriter {}
    VAR i: usize = 0
    VAR start = now()
    WHILE i < n
        writeln(w, "iteration {} done", i)
        i = i + 1
    ENDWHILE
    RETURN elapsed(start).nanos
ENDFN
```

---

## Acceptance Criteria

- [ ] All existing integration tests pass (except known array bug)
- [ ] All new CORE tests pass in `NO_MACHINE` mode
- [ ] All new MACHINE integration tests pass
- [ ] Backward compatibility tests pass
- [ ] CI pipeline runs successfully
- [ ] No performance regression on existing benchmarks
- [ ] Documentation examples compile and run