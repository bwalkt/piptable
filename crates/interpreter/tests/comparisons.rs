//! Comparisons tests for the PipTable interpreter.

#![allow(clippy::needless_raw_string_hashes)]

mod common {
    include!("common_impl.txt");
}
use common::*;

use piptable_core::Value;

/// Ensures the equality operator evaluates equal integers as equal.
///
/// # Examples
///
/// ```
/// # async fn example() {
/// let (interp, _) = run_script("dim x = 5 == 5").await;
/// assert!(matches!(interp.get_var("x").await, Some(Value::Bool(true))));
/// # }
/// ```
#[tokio::test]
async fn test_equal() {
    let (interp, _) = run_script("dim x = 5 == 5").await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Bool(true))));
}

#[tokio::test]
async fn test_not_equal() {
    let (interp, _) = run_script("dim x = 5 != 3").await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Bool(true))));
}

#[tokio::test]
async fn test_greater_than() {
    let (interp, _) = run_script("dim x = 10 > 5").await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Bool(true))));
}

#[tokio::test]
async fn test_less_than() {
    let (interp, _) = run_script("dim x = 3 < 7").await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Bool(true))));
}

/// Asserts that the greater-than-or-equal comparison evaluates to true when both operands are equal.
///
/// # Examples
///
/// ```
/// #[tokio::test]
/// async fn example_greater_or_equal() {
///     let (interp, _) = run_script("dim x = 5 >= 5").await;
///     assert!(matches!(interp.get_var("x").await, Some(Value::Bool(true))));
/// }
/// ```
#[tokio::test]
async fn test_greater_or_equal() {
    let (interp, _) = run_script("dim x = 5 >= 5").await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Bool(true))));
}

/// Checks that the `<=` operator evaluates equality correctly.
///
/// Runs a script that assigns the result of `5 <= 5` to `x` and asserts the interpreter stores the expected boolean result.
///
/// # Examples
///
/// ```no_run
/// // Integration test: assigns 5 <= 5 to x and checks the value.
/// let (interp, _) = run_script("dim x = 5 <= 5").await;
/// assert!(matches!(interp.get_var("x").await, Some(Value::Bool(true))));
/// ```
#[tokio::test]
async fn test_less_or_equal() {
    let (interp, _) = run_script("dim x = 5 <= 5").await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Bool(true))));
}

#[tokio::test]
async fn test_and_operator() {
    let (interp, _) = run_script("dim x = true and true").await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Bool(true))));
}

#[tokio::test]
async fn test_or_operator() {
    let (interp, _) = run_script("dim x = false or true").await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Bool(true))));
}

#[tokio::test]
async fn test_not_operator() {
    let (interp, _) = run_script("dim x = not false").await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Bool(true))));
}

/// Verifies that the logical `and` operator short-circuits, so a false left-hand side prevents evaluation of the right-hand side.
///
/// # Examples
///
/// ```
/// // If short-circuit works, the undefined expression on the right is not evaluated
/// let (interp, _) = run_script("dim x = false and undefined_var").await;
/// assert!(matches!(
///     interp.get_var("x").await,
///     Some(Value::Bool(false))
/// ));
/// ```
#[tokio::test]
async fn test_short_circuit_and() {
    // If short-circuit works, undefined_var won't be evaluated
    let (interp, _) = run_script("dim x = false and undefined_var").await;
    assert!(matches!(
        interp.get_var("x").await,
        Some(Value::Bool(false))
    ));
}

/// Verifies that the logical `or` operator short-circuits so the right-hand side is not evaluated when the left-hand side is `true`.
///
/// # Examples
///
/// ```
/// // The right-hand side references an undefined variable; short-circuiting prevents its evaluation.
/// let (interp, _) = run_script("dim x = true or undefined_var").await;
/// assert!(matches!(interp.get_var("x").await, Some(Value::Bool(true))));
/// ```
#[tokio::test]
async fn test_short_circuit_or() {
    // If short-circuit works, undefined_var won't be evaluated
    let (interp, _) = run_script("dim x = true or undefined_var").await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Bool(true))));
}
