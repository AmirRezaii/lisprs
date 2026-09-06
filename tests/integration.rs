use std::{fs, path::Path};

use lisprs::{diagnostics::*, lisp::Lisp, runtime::Value};

fn run_file(name: &str) -> Result<Value, Error> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("programs")
        .join(name);

    let source = fs::read_to_string(path).expect("failed to read Lisp test file");

    let mut lisp = Lisp::new();

    lisp.execute(name, &source)
}

fn run_file_error(name: &str) -> (Lisp, String, Error) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("programs")
        .join(name);

    let source = fs::read_to_string(path).expect("failed to read Lisp test file");
    let mut lisp = Lisp::new();
    let error = lisp
        .execute(name, &source)
        .expect_err("program was expected to fail");

    (lisp, source, error)
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

fn assert_bool(value: Value, expected: bool) {
    match value {
        Value::Bool(actual) => {
            assert!(actual == expected, "expected {expected}, got {actual}");
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
    assert_bool(run_file("gc_equal.el").unwrap(), true);
}

#[test]
fn gc_capture() {
    assert_number(run_file("gc_capture.el").unwrap(), 43.0);
}

#[test]
fn if_condition() {
    assert_number(run_file("if_condition.el").unwrap(), 23.0);
}

#[test]
fn if_nested() {
    assert_number(run_file("if_nested.el").unwrap(), 2.0);
}

#[test]
fn factorial() {
    assert_number(run_file("factorial.el").unwrap(), 120.0);
}

#[test]
fn while_loop() {
    assert_bool(run_file("while.el").unwrap(), true);
}

#[test]
fn mac() {
    assert_number(run_file("macro.el").unwrap(), 23.0);
}

#[test]
fn macro_optional_parameter_uses_default_and_supplied_value() {
    assert_number(run_file("macro_optional.el").unwrap(), 25.0);
}

#[test]
fn macro_rest_parameter_collects_extra_forms() {
    assert_number(run_file("macro_rest.el").unwrap(), 10.0);
}

#[test]
fn nested_macro_expansion_is_reexpanded() {
    assert_number(run_file("macro_nested.el").unwrap(), 5.0);
}

#[test]
fn macro_body_closures_capture_macro_parameters() {
    assert_number(run_file("macro_capture.el").unwrap(), 12.0);
}

#[test]
fn unused_macro_arguments_are_not_evaluated() {
    assert_number(run_file("macro_unused_argument.el").unwrap(), 42.0);
}

#[test]
fn macro_body_runtime_errors_point_into_the_macro_body() {
    let (mut lisp, source, error) = run_file_error("macro_body_error.el");
    let body_start = source.find("(car 1)").expect("macro body not found");

    match &error {
        Error::Macro(error) => {
            assert_eq!(error.span.start, body_start);
            assert_eq!(error.span.end, body_start + "(car 1)".len());

            match &error.kind {
                MacroErrorKind::EvaluationError(inner) => match inner.as_ref() {
                    Error::Runtime(runtime) => {
                        let location = runtime
                            .location
                            .expect("macro-body runtime error should be located");
                        assert_eq!(location.span.start, body_start);
                    }
                    other => panic!("expected nested runtime error, got {other:?}"),
                },
                other => panic!("expected macro evaluation error, got {other:?}"),
            }
        }
        other => panic!("expected macro error, got {other:?}"),
    }

    let rendered = lisp.render_error(error, "macro_body_error.el", &source);
    assert!(rendered.contains("(car 1)"));
}

#[test]
fn errors_from_generated_code_point_at_the_macro_call() {
    let (_lisp, source, error) = run_file_error("macro_generated_error.el");
    let call_start = source
        .find("(generated-error)")
        .expect("macro call not found");

    match error {
        Error::Runtime(error) => {
            let location = error.location.expect("runtime error should be located");
            assert_eq!(location.span.start, call_start);
            assert_eq!(location.span.end, call_start + "(generated-error)".len());
        }
        other => panic!("expected runtime error, got {other:?}"),
    }
}

#[test]
fn macro_argument_count_errors_point_at_the_macro_call() {
    let (_lisp, source, error) = run_file_error("macro_wrong_arity.el");
    let call_start = source.find("(needs-two 1)").expect("macro call not found");

    match error {
        Error::Macro(error) => {
            assert_eq!(error.span.start, call_start);
            match error.kind {
                MacroErrorKind::InvalidArgumentCount(
                    ArgCount::Exact(given),
                    ArgCount::Exact(expected),
                ) => {
                    assert_eq!(given, 1);
                    assert_eq!(expected, 2);
                }
                other => panic!("expected macro argument-count error, got {other:?}"),
            }
        }
        other => panic!("expected macro error, got {other:?}"),
    }
}

#[test]
fn optional_param() {
    assert_number(run_file("optional_param.el").unwrap(), 2.0);
}

#[test]
fn undefined_variable_is_runtime_error() {
    let err = run_file("errors.el").unwrap_err();

    assert!(err.to_string().contains("undefined variable"));
}
