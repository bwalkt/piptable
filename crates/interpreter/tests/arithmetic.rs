//! Arithmetic tests for the PipTable interpreter.

#![allow(clippy::approx_constant)]
#![allow(clippy::needless_raw_string_hashes)]

mod _common;
use _common::*;

use piptable_core::Value;

/// Verifies that evaluating an addition expression assigns the correct integer sum to a variable.
///
/// # Examples
///
/// ```
/// # use crate::common::run_script;
/// # use piptable_core::Value;
/// # #[tokio::test]
/// # async fn _example() {
/// let (interp, _) = run_script("dim x = 10 + 5").await;
/// assert!(matches!(interp.get_var("x").await, Some(Value::Int(15))));
/// # }
/// ```
#[tokio::test]
async fn test_addition() {
    let (interp, _) = run_script("dim x = 10 + 5").await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Int(15))));
}

#[tokio::test]
async fn test_subtraction() {
    let (interp, _) = run_script("dim x = 10 - 3").await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Int(7))));
}

/// Verifies that integer multiplication produces the expected value assigned to a variable.
///
/// # Examples
///
/// ```
/// # tokio_test::block_on(async {
/// let (interp, _) = run_script("dim x = 6 * 7").await;
/// assert!(matches!(interp.get_var("x").await, Some(Value::Int(42))));
/// # });
/// ```
#[tokio::test]
async fn test_multiplication() {
    let (interp, _) = run_script("dim x = 6 * 7").await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Int(42))));
}

/// Verifies that dividing two integer literals stores the integer quotient in a variable.
///
/// # Examples
///
/// ```
/// #[tokio::test]
/// async fn example_division() {
///     let (interp, _) = run_script("dim x = 20 / 4").await;
///     assert!(matches!(interp.get_var("x").await, Some(Value::Int(5))));
/// }
/// ```
#[tokio::test]
async fn test_division() {
    let (interp, _) = run_script("dim x = 20 / 4").await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Int(5))));
}

/// Verifies the integer modulo operator produces the correct remainder.
///
/// # Examples
///
/// ```
/// #[tokio::test]
/// async fn example_modulo() {
///     let (interp, _) = run_script("dim x = 17 % 5").await;
///     assert!(matches!(interp.get_var("x").await, Some(Value::Int(2))));
/// }
/// ```
#[tokio::test]
async fn test_modulo() {
    let (interp, _) = run_script("dim x = 17 % 5").await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Int(2))));
}

#[tokio::test]
async fn test_negation() {
    let (interp, _) = run_script("dim x = -42").await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Int(-42))));
}

/// Verifies that multiplication takes precedence over addition.
///
/// Evaluates the expression `2 + 3 * 4` in the interpreter and asserts that the variable
/// `x` is assigned the value 14.
///
/// # Examples
///
/// ```
/// # async fn example() {
/// let (interp, _) = run_script("dim x = 2 + 3 * 4").await;
/// assert!(matches!(interp.get_var("x").await, Some(Value::Int(14))));
/// # }
/// ```
#[tokio::test]
async fn test_operator_precedence() {
    let (interp, _) = run_script("dim x = 2 + 3 * 4").await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Int(14))));
}

/// Verifies that parentheses alter operator precedence so grouped additions are evaluated before multiplication.
///
/// # Examples
///
/// ```
/// // Runs the script and checks that (2 + 3) is evaluated first, producing 20 for x.
/// let (interp, _) = run_script("dim x = (2 + 3) * 4").await;
/// assert!(matches!(interp.get_var("x").await, Some(Value::Int(20))));
/// ```
#[tokio::test]
async fn test_parentheses() {
    let (interp, _) = run_script("dim x = (2 + 3) * 4").await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Int(20))));
}

/// Verifies that adding two floating-point literals yields the correct float result.
///
/// # Examples
///
/// ```
/// # async fn _example() {
/// let (interp, _) = run_script("dim x = 10.5 + 2.5").await;
/// match interp.get_var("x").await {
///     Some(Value::Float(f)) => assert!((f - 13.0).abs() < 0.001),
///     _ => panic!("Expected float"),
/// }
/// # }
/// ```
#[tokio::test]
async fn test_float_arithmetic() {
    let (interp, _) = run_script("dim x = 10.5 + 2.5").await;
    match interp.get_var("x").await {
        Some(Value::Float(f)) => assert!((f - 13.0).abs() < 0.001),
        _ => panic!("Expected float"),
    }
}

/// Verifies that adding an integer and a float produces a float value stored in `x`.
///
/// # Examples
///
/// ```
/// # async fn example() {
/// let (interp, _) = run_script("dim x = 10 + 0.5").await;
/// match interp.get_var("x").await {
///     Some(Value::Float(f)) => assert!((f - 10.5).abs() < 0.001),
///     _ => panic!("Expected float"),
/// }
/// # }
/// ```
#[tokio::test]
async fn test_mixed_int_float() {
    let (interp, _) = run_script("dim x = 10 + 0.5").await;
    match interp.get_var("x").await {
        Some(Value::Float(f)) => assert!((f - 10.5).abs() < 0.001),
        _ => panic!("Expected float"),
    }
}
