use std::{fs, path::Path};

use lisprs::{
    common::{Lisp, Value},
    diagnostics::*,
};

fn run_file(name: &str) -> Result<Value, Error> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("programs")
        .join(name);

    let source = fs::read_to_string(path).expect("failed to read Lisp test file");

    let mut lisp = Lisp::new();
    lisp.stdlib();

    lisp.execute(&source)
}

fn assert_number(value: Value, expected: f64) {
    match value {
        Value::Number(actual) => {
            assert!(
                (actual - expected).abs() < f64::EPSILON,
                "expected {expected}, got {actual}"
            );
        }
        other => panic!("expected number {expected}, got {other:?}"),
    }
}

#[test]
fn basic_expressions() {
    assert_number(run_file("basic.el").unwrap(), 42.0);
}

#[test]
fn local_variables() {
    assert_number(run_file("locals.el").unwrap(), 30.0);
}

#[test]
fn lexical_scopes() {
    assert_number(run_file("let_scopes.el").unwrap(), 30.0);
}

#[test]
fn lexical_scopes_and_shadowing() {
    assert_number(run_file("let_scopes_shadowing.el").unwrap(), 20.0);
}

#[test]
fn functions_and_arguments() {
    assert_number(run_file("functions.el").unwrap(), 42.0);
}

#[test]
fn explicit_return() {
    assert_number(run_file("returns.el").unwrap(), 42.0);
}

#[test]
fn closures_capture_outer_locals() {
    assert_number(run_file("closures.el").unwrap(), 42.0);
}

#[test]
fn nested_closures_capture_through_multiple_levels() {
    assert_number(run_file("nested_closures.el").unwrap(), 42.0);
}

#[test]
fn closure_can_mutate_captured_variable() {
    assert_number(run_file("closure_mutation.el").unwrap(), 3.0);
}

#[test]
fn multiple_closures_share_captured_variable() {
    assert_number(run_file("closure_sharing.el").unwrap(), 3.0);
}

#[test]
fn capture_shadowing() {
    assert_number(run_file("capture_shadowing.el").unwrap(), 1.0);
}

#[test]
fn make_garbage() {
    assert_number(run_file("make_garbage.el").unwrap(), 42.0);
}

#[test]
fn gc_equal() {
    assert_number(run_file("gc_equal.el").unwrap(), 1.0);
}

#[test]
fn gc_capture() {
    assert_number(run_file("gc_capture.el").unwrap(), 43.0);
}

#[test]
fn undefined_variable_is_runtime_error() {
    let err = run_file("errors.el").unwrap_err();

    assert!(err.to_string().contains("undefined variable"));
}
