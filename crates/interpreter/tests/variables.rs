//! Variables tests for the PipTable interpreter.

#![allow(clippy::needless_raw_string_hashes)]

mod _common;
use _common::*;

use piptable_core::Value;

/// Verifies that declaring a variable with an integer literal binds that integer value to the variable.
///
/// # Examples
///
/// ```
/// let (interp, _) = run_script("dim x = 42").await;
/// assert!(matches!(interp.get_var("x").await, Some(Value::Int(42))));
/// ```
#[tokio::test]
async fn test_dim_integer() {
    let (interp, _) = run_script("dim x = 42").await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Int(42))));
}

/// Verifies that declaring a variable with a float literal assigns a floating-point value.
///
/// # Examples
///
/// ```
/// # async fn example() {
/// let (interp, _) = run_script("dim x = 3.14").await;
/// match interp.get_var("x").await {
///     Some(Value::Float(f)) => assert!((f - 3.14).abs() < 0.001),
///     _ => panic!("Expected float"),
/// }
/// # }
/// ```
#[tokio::test]
async fn test_dim_float() {
    let (interp, _) = run_script("dim x = 3.14").await;
    match interp.get_var("x").await {
        Some(Value::Float(f)) => assert!((f - 3.14).abs() < 0.001),
        _ => panic!("Expected float"),
    }
}

/// Verifies that declaring a string with `dim` assigns the string value to variable `x`.
///
/// # Examples
///
/// ```no_run
/// # use crate::common::run_script;
/// # use piptable_core::Value;
/// # tokio_test::block_on(async {
/// let (interp, _) = run_script(r#"dim x = "hello""#).await;
/// assert!(matches!(interp.get_var("x").await, Some(Value::String(s)) if s == "hello"));
/// # });
/// ```
#[tokio::test]
async fn test_dim_string() {
    let (interp, _) = run_script(r#"dim x = "hello""#).await;
    assert!(matches!(interp.get_var("x").await, Some(Value::String(s)) if s == "hello"));
}

#[tokio::test]
async fn test_dim_boolean_true() {
    let (interp, _) = run_script("dim x = true").await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Bool(true))));
}

#[tokio::test]
async fn test_dim_boolean_false() {
    let (interp, _) = run_script("dim x = false").await;
    assert!(matches!(
        interp.get_var("x").await,
        Some(Value::Bool(false))
    ));
}

#[tokio::test]
async fn test_dim_null() {
    let (interp, _) = run_script("dim x = null").await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Null)));
}

/// Verifies that assigning to an existing variable updates its value.
///
/// This test ensures that a subsequent assignment to a previously declared variable
/// replaces the prior value in the interpreter's environment.
///
/// # Examples
///
/// ```
/// let (interp, _) = run_script("dim x = 1\nx = 2").await;
/// assert!(matches!(interp.get_var("x").await, Some(Value::Int(2))));
/// ```
#[tokio::test]
async fn test_assignment_updates_variable() {
    let (interp, _) = run_script("dim x = 1\nx = 2").await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Int(2))));
}

/// Verifies that declaring multiple variables and assigning a variable using an expression produces the expected value.
///
/// The test declares `x` and `y`, assigns `z` to `x + y`, and asserts that `z` equals `30`.
///
/// # Examples
///
/// ```
/// let (interp, _) = run_script("dim x = 10\ndim y = 20\ndim z = x + y").await;
/// assert!(matches!(interp.get_var("z").await, Some(Value::Int(30))));
/// ```
#[tokio::test]
async fn test_multiple_variables() {
    let (interp, _) = run_script("dim x = 10\ndim y = 20\ndim z = x + y").await;
    assert!(matches!(interp.get_var("z").await, Some(Value::Int(30))));
}

#[tokio::test]
async fn test_variable_expression() {
    let (interp, _) = run_script("dim x = 5\ndim y = x * 2 + 3").await;
    assert!(matches!(interp.get_var("y").await, Some(Value::Int(13))));
}
