//! Errors tests for the PipTable interpreter.

#![allow(clippy::needless_raw_string_hashes)]

#[path = "_common.rs"]
mod common;
use common::*;

/// Asserts that evaluating a script which references an undefined variable produces an interpreter error.
///
/// # Examples
///
/// ```
/// // Expects an error mentioning "Undefined variable".
/// let err = run_script_err("dim x = undefined_var").await;
/// assert!(err.contains("Undefined variable"));
/// ```
#[tokio::test]
async fn test_undefined_variable() {
    let err = run_script_err("dim x = undefined_var").await;
    assert!(err.contains("Undefined variable"));
}

/// Verifies that calling a non-existent function in a script produces an interpreter error.
///
/// Asserts the interpreter reports the function as "Unknown" or "Undefined" when evaluating
/// a call to a function that has not been defined.
#[tokio::test]
async fn test_undefined_function() {
    let err = run_script_err("dim x = unknown_func()").await;
    assert!(
        err.contains("Unknown function") || err.contains("Undefined function"),
        "Expected function error, got: {}",
        err
    );
}

#[tokio::test]
async fn test_division_by_zero() {
    let err = run_script_err("dim x = 10 / 0").await;
    assert!(err.contains("Division by zero"));
}

#[tokio::test]
async fn test_modulo_by_zero() {
    let err = run_script_err("dim x = 10 % 0").await;
    assert!(err.contains("Modulo by zero"));
}

/// Verifies that indexing an array with an index outside its valid range produces an out-of-bounds error.
///
/// # Examples
///
/// ```
/// #[tokio::test]
/// async fn example_array_index_out_of_bounds() {
///     let err = run_script_err("dim arr = [1, 2, 3]\ndim x = arr[10]").await;
///     assert!(err.contains("out of bounds"));
/// }
/// ```
#[tokio::test]
async fn test_array_index_out_of_bounds() {
    let err = run_script_err("dim arr = [1, 2, 3]\ndim x = arr[10]").await;
    assert!(err.contains("out of bounds"));
}

/// Verifies that accessing an array with a negative index outside the valid range produces an out-of-bounds error.
///
/// # Examples
///
/// ```no_run
/// // Asserts that indexing with a large negative index reports an out-of-bounds error.
/// let err = run_script_err("dim arr = [1, 2, 3]\ndim x = arr[-10]").await;
/// assert!(err.contains("out of bounds"));
/// ```
#[tokio::test]
async fn test_negative_index_out_of_bounds() {
    let err = run_script_err("dim arr = [1, 2, 3]\ndim x = arr[-10]").await;
    assert!(err.contains("out of bounds"));
}

/// Verifies that executing a `for` loop with a step of zero produces an error.
///
/// This test ensures the interpreter rejects a loop that would never advance due to a step of `0`.
///
/// # Examples
///
/// ```
/// // asserts that a for-loop with step 0 yields an error mentioning "non-zero" or "step"
/// let err = run_script_err("for i = 1 to 10 step 0\nnext").await;
/// assert!(err.contains("non-zero") || err.contains("step"));
/// ```
#[tokio::test]
async fn test_for_loop_zero_step() {
    let err = run_script_err("for i = 1 to 10 step 0\nnext").await;
    assert!(err.contains("non-zero") || err.contains("step"));
}

#[tokio::test]
async fn test_type_error_add_string_int() {
    let err = run_script_err(r#"dim x = "hello" + 42"#).await;
    // Verify the interpreter produces a type error for string + int
    assert!(
        err.contains("Type error") || err.contains("Cannot"),
        "Expected type error, got: {}",
        err
    );
}
