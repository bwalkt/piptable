//! Edge_Cases tests for the PipTable interpreter.

#![allow(clippy::needless_raw_string_hashes)]

mod _common;
use _common::*;

use piptable_core::Value;

/// Verifies that declaring an empty array binds a variable to an empty array.
///
/// # Examples
///
/// ```
/// # async fn example() {
/// let (interp, _) = run_script("dim arr = []").await;
/// match interp.get_var("arr").await {
///     Some(Value::Array(items)) => assert_eq!(items.len(), 0),
///     _ => panic!("Expected empty array"),
/// }
/// # }
/// ```
#[tokio::test]
async fn test_empty_array() {
    let (interp, _) = run_script("dim arr = []").await;
    match interp.get_var("arr").await {
        Some(Value::Array(items)) => assert_eq!(items.len(), 0),
        _ => panic!("Expected empty array"),
    }
}

/// Verifies that declaring an empty object creates an object variable with no fields.
///
/// This integration test runs a script that defines `obj` as an empty object and
/// asserts the interpreter stores `obj` as an `Object` with length 0.
///
/// # Examples
///
/// ```
/// let (interp, _) = run_script("dim obj = {}").await;
/// match interp.get_var("obj").await {
///     Some(Value::Object(map)) => assert_eq!(map.len(), 0),
///     _ => panic!("Expected empty object"),
/// }
/// ```
#[tokio::test]
async fn test_empty_object() {
    let (interp, _) = run_script("dim obj = {}").await;
    match interp.get_var("obj").await {
        Some(Value::Object(map)) => assert_eq!(map.len(), 0),
        _ => panic!("Expected empty object"),
    }
}

/// Verifies that a `for` loop with a start greater than the end and the default step does not execute.
///
/// The test runs a script that initializes `count` to 0 and iterates `for i = 10 to 1` with the implicit step of 1;
/// the loop body should never run and `count` must remain 0.
///
/// # Examples
///
/// ```
/// let (interp, _) = run_script(r#"
///     dim count = 0
///     for i = 10 to 1
///         count = count + 1
///     next
/// "#).await;
/// assert!(matches!(interp.get_var("count").await, Some(Value::Int(0))));
/// ```
#[tokio::test]
async fn test_for_loop_no_iterations() {
    let (interp, _) = run_script(
        r#"
        dim count = 0
        for i = 10 to 1
            count = count + 1
        next
    "#,
    )
    .await;
    // With default step 1, 10 to 1 should not execute
    assert!(matches!(interp.get_var("count").await, Some(Value::Int(0))));
}

/// Verifies that a `for each` loop over an empty array never executes its body (counter remains 0).
///
/// # Examples
///
/// ```
/// # async fn run_example() {
/// let (interp, _) = run_script(
///     r#"
///     dim items = []
///     dim count = 0
///     for each item in items
///         count = count + 1
///     next
/// "#,
/// )
/// .await;
/// assert!(matches!(interp.get_var("count").await, Some(Value::Int(0))));
/// # }
/// ```
#[tokio::test]
async fn test_foreach_empty_array() {
    let (interp, _) = run_script(
        r#"
        dim items = []
        dim count = 0
        for each item in items
            count = count + 1
        next
    "#,
    )
    .await;
    assert!(matches!(interp.get_var("count").await, Some(Value::Int(0))));
}

/// Verifies that adding 1 to the maximum 64-bit integer produces an overflow error.
///
/// # Examples
///
/// ```
/// # async fn _doc() {
/// let err = run_script_err(&format!("dim x = {} + 1", i64::MAX)).await;
/// assert!(err.contains("overflow"));
/// # }
/// ```
#[tokio::test]
async fn test_integer_overflow_add() {
    let err = run_script_err(&format!("dim x = {} + 1", i64::MAX)).await;
    assert!(err.contains("overflow"));
}

// i64::MIN literal cannot be parsed directly (too large)
// Test overflow via negation of i64::MAX and subtraction instead
/// Verifies the interpreter reports an integer overflow when subtracting past the 64-bit integer lower bound.
///
/// The test executes a script that performs a subtraction which exceeds the minimum i64 value and asserts the resulting error message contains "overflow".
///
/// # Examples
///
/// ```
/// // This mirrors the test: attempting to subtract beyond i64::MIN should produce an overflow error.
/// let err = run_script_err("dim x = -9223372036854775807 - 2").await;
/// assert!(err.contains("overflow"));
/// ```
#[tokio::test]
async fn test_integer_overflow_sub() {
    // Use a large negative number that can be parsed
    let err = run_script_err("dim x = -9223372036854775807 - 2").await;
    assert!(
        err.contains("overflow"),
        "Expected overflow error, got: {}",
        err
    );
}

#[tokio::test]
async fn test_integer_overflow_mul() {
    let err = run_script_err(&format!("dim x = {} * 2", i64::MAX)).await;
    assert!(err.contains("overflow"));
}

#[tokio::test]
async fn test_string_concatenation() {
    let (interp, _) = run_script(r#"dim x = "hello" + " " + "world""#).await;
    assert!(matches!(interp.get_var("x").await, Some(Value::String(s)) if s == "hello world"));
}

#[tokio::test]
async fn test_deeply_nested_expression() {
    let (interp, _) = run_script("dim x = ((((1 + 2) * 3) - 4) / 5)").await;
    // ((1+2)*3-4)/5 = (3*3-4)/5 = (9-4)/5 = 5/5 = 1
    assert!(matches!(interp.get_var("x").await, Some(Value::Int(1))));
}

/// Verifies that reusing a variable name as a for-loop index does not cause a runtime error and that the original name remains bound after the loop.
///
/// Asserts the interpreter has a binding for `i` after executing a script where `i` is declared before the loop and the loop uses `i` as its loop variable.
///
/// # Examples
///
/// ```
/// // Ensures variable shadowing inside a for-loop does not remove the outer binding.
/// let (interp, _) = run_script(
///     r#"
///     dim i = 100
///     for i = 1 to 3
///         dim x = i
///     next
/// "#,
/// )
/// .await;
/// assert!(interp.get_var("i").await.is_some());
/// ```
#[tokio::test]
async fn test_variable_shadowing_in_loop() {
    let (interp, _) = run_script(
        r#"
        dim i = 100
        for i = 1 to 3
            dim x = i
        next
    "#,
    )
    .await;
    // After loop, i should be the loop's final value or shadowed
    // depends on implementation - just verify it doesn't error
    assert!(interp.get_var("i").await.is_some());
}
