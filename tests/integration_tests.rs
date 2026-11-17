use carnax::driver::build_source;
use carnax::parser::parse_source;
use carnax::semantic::analyze;
use std::fs;

const EXAMPLES_DIR: &str = "examples";
const ERRORS_DIR: &str = "examples/errors";

fn ex(path: &str) -> std::path::PathBuf {
    std::path::Path::new(EXAMPLES_DIR).join(path)
}

fn err(path: &str) -> std::path::PathBuf {
    std::path::Path::new(ERRORS_DIR).join(path)
}

#[test]
fn test_negative_const_reassign() {
    let source = fs::read_to_string(err("04_const_reassign.fsl")).unwrap();
    let program = parse_source(&source).unwrap();
    let res = analyze(&program);
    assert!(res.has_errors);
    assert!(res.errors.iter().any(|e| e.contains("cannot assign to constant")));
}

#[test]
fn test_negative_machine_no_main() {
    let source = fs::read_to_string(err("11_machine_no_main.fsl")).unwrap();
    let program = parse_source(&source).unwrap();
    let res = analyze(&program);
    assert!(res.has_errors);
    assert!(res.errors.iter().any(|e| e.contains("requires a 'main' function")));
}

#[test]
fn test_negative_machine_wrong_argv() {
    let source = fs::read_to_string(err("12_machine_wrong_argv.fsl")).unwrap();
    let program = parse_source(&source).unwrap();
    let res = analyze(&program);
    assert!(res.has_errors);
    assert!(res.errors.iter().any(|e| e.contains("main argv parameter must be ptr[ptr[u8]]")));
}

#[test]
fn test_negative_machine_wrong_return() {
    let source = fs::read_to_string(err("13_machine_wrong_return.fsl")).unwrap();
    let program = parse_source(&source).unwrap();
    let res = analyze(&program);
    assert!(res.has_errors);
    assert!(res.errors.iter().any(|e| e.contains("main in MACHINE mode must return i32")));
}

#[test]
fn test_negative_return_type() {
    let source = fs::read_to_string(err("06_return_type.fsl")).unwrap();
    let program = parse_source(&source).unwrap();
    let res = analyze(&program);
    assert!(res.has_errors);
    assert!(res.errors.iter().any(|e| e.contains("return type mismatch")));
}

#[test]
fn test_negative_type_mismatch() {
    let source = fs::read_to_string(err("03_type_mismatch.fsl")).unwrap();
    let program = parse_source(&source).unwrap();
    let res = analyze(&program);
    assert!(res.has_errors);
    assert!(res.errors.iter().any(|e| e.contains("type mismatch")));
}

#[test]
fn test_negative_undeclared() {
    let source = fs::read_to_string(err("02_undeclared.fsl")).unwrap();
    let program = parse_source(&source).unwrap();
    let res = analyze(&program);
    assert!(res.has_errors);
    assert!(res.errors.iter().any(|e| e.contains("undeclared variable")));
}

#[test]
fn test_negative_parse_error() {
    let source = fs::read_to_string(err("14_incomplete_var.fsl")).unwrap();
    assert!(parse_source(&source).is_err());
}

#[test]
fn test_end_to_end_native_execution_simple() {
    let exit_code = build_source(&ex("01_basic_types.fsl"), None, true, &[]).expect("build and run failed");
    assert_eq!(exit_code, 0);
}

#[test]
fn test_end_to_end_native_execution_valid() {
    let exit_code = build_source(&err("01_valid.fsl"), None, true, &[]).expect("build and run failed");
    assert_eq!(exit_code, 0);
}

#[test]
fn test_end_to_end_native_execution_test4() {
    let exit_code = build_source(&ex("01b_basic_comments.fsl"), None, true, &[]).expect("build and run failed");
    assert_eq!(exit_code, 42);
}

#[test]
fn test_end_to_end_native_execution_fibonacci() {
    let exit_code = build_source(&ex("11_recursion.fsl"), None, true, &[]).expect("build and run failed");
    assert_eq!(exit_code, 0);
}

#[test]
fn test_end_to_end_native_execution_bitwise() {
    let exit_code = build_source(&ex("15_bitwise.fsl"), None, true, &[]).expect("build and run failed");
    assert_eq!(exit_code, 0);
}

#[test]
fn test_end_to_end_native_execution_nested_loops() {
    let exit_code = build_source(&ex("12_nested_loops.fsl"), None, true, &[]).expect("build and run failed");
    assert_eq!(exit_code, 0);
}

#[test]
fn test_end_to_end_native_execution_struct_ops() {
    let exit_code = build_source(&ex("04_structs.fsl"), None, true, &[]).expect("build and run failed");
    assert_eq!(exit_code, 0);
}

#[test]
fn test_end_to_end_native_execution_multireturn() {
    let exit_code = build_source(&ex("09_multiple_returns.fsl"), None, true, &[]).expect("build and run failed");
    assert_eq!(exit_code, 0);
}

#[test]
fn test_end_to_end_native_execution_early_returns() {
    let exit_code = build_source(&ex("13_early_returns.fsl"), None, true, &[]).expect("build and run failed");
    assert_eq!(exit_code, 0);
}

#[test]
fn test_end_to_end_native_execution_arrays() {
    let exit_code = build_source(&ex("08_arrays.fsl"), None, true, &[]).expect("build and run failed");
    assert_eq!(exit_code, 0);
}

#[test]
fn test_negative_enum_unknown_variant() {
    let source = fs::read_to_string(err("10_enum_unknown_variant.fsl")).unwrap();
    let program = parse_source(&source).unwrap();
    let res = analyze(&program);
    assert!(res.has_errors);
    assert!(res.errors.iter().any(|e| e.contains("has no variant 'Stopped'")));
}

#[test]
fn test_negative_match_wrong_enum() {
    let source = fs::read_to_string(err("07_match_wrong_enum.fsl")).unwrap();
    let program = parse_source(&source).unwrap();
    let res = analyze(&program);
    assert!(res.has_errors);
    assert!(res.errors.iter().any(|e| e.contains("does not match matched expression enum")));
}

#[test]
fn test_negative_match_payload_mismatch() {
    let source = fs::read_to_string(err("08_match_payload_mismatch.fsl")).unwrap();
    let program = parse_source(&source).unwrap();
    let res = analyze(&program);
    assert!(res.has_errors);
    assert!(res.errors.iter().any(|e| e.contains("requires a payload")));
}

#[test]
fn test_negative_match_literal_on_enum() {
    let source = fs::read_to_string(err("09_match_literal_on_enum.fsl")).unwrap();
    let program = parse_source(&source).unwrap();
    let res = analyze(&program);
    assert!(res.has_errors);
    assert!(res.errors.iter().any(|e| e.contains("cannot match literal of type i32 against Kind")));
}

#[test]
fn test_end_to_end_native_execution_enum_unit() {
    let exit_code = build_source(&ex("05_enums_unit.fsl"), None, true, &[]).expect("build and run failed");
    assert_eq!(exit_code, 0);
}

#[test]
fn test_end_to_end_native_execution_enum_payload() {
    let exit_code = build_source(&ex("06_enums_payload.fsl"), None, true, &[]).expect("build and run failed");
    assert_eq!(exit_code, 0);
}

#[test]
fn test_end_to_end_native_execution_match_switch() {
    let exit_code = build_source(&ex("07_match_expressions.fsl"), None, true, &[]).expect("build and run failed");
    assert_eq!(exit_code, 0);
}

#[test]
fn test_end_to_end_native_execution_match_wildcard() {
    let exit_code = build_source(&ex("07b_match_wildcard.fsl"), None, true, &[]).expect("build and run failed");
    assert_eq!(exit_code, 0);
}

#[test]
fn test_end_to_end_native_execution_enum_bare() {
    let exit_code = build_source(&ex("05b_enums_unit_bare.fsl"), None, true, &[]).expect("build and run failed");
    assert_eq!(exit_code, 0);
}
