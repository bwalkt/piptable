//! Builtins tests for the PipTable interpreter.

#![allow(clippy::needless_raw_string_hashes)]

#[path = "_common.rs"]
mod common;
use common::*;

use piptable_core::Value;

#[tokio::test]
async fn test_print() {
    let (interp, _) = run_script(r#"print("Hello, World!")"#).await;
    let output = interp.output().await;
    assert_eq!(output, vec!["Hello, World!"]);
}

#[tokio::test]
async fn test_print_multiple() {
    let (interp, _) = run_script(
        r#"print("one")
print("two")
print("three")"#,
    )
    .await;
    let output = interp.output().await;
    assert_eq!(output, vec!["one", "two", "three"]);
}

#[tokio::test]
async fn test_len_array() {
    let (interp, _) = run_script("dim x = len([1, 2, 3, 4, 5])").await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Int(5))));
}

#[tokio::test]
async fn test_len_string() {
    let (interp, _) = run_script(r#"dim x = len("hello")"#).await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Int(5))));
}

/// Verifies that the `type` builtin returns "Int" for an integer literal.
///
/// # Examples
///
/// ```
/// #[tokio::test]
/// async fn example_type_int() {
///     let (interp, _) = run_script("dim x = type(42)").await;
///     assert!(matches!(interp.get_var("x").await, Some(Value::String(s)) if s == "Int"));
/// }
/// ```
#[tokio::test]
async fn test_type_int() {
    let (interp, _) = run_script("dim x = type(42)").await;
    assert!(matches!(interp.get_var("x").await, Some(Value::String(s)) if s == "Int"));
}

#[tokio::test]
async fn test_type_string() {
    let (interp, _) = run_script(r#"dim x = type("hello")"#).await;
    assert!(matches!(interp.get_var("x").await, Some(Value::String(s)) if s == "String"));
}

/// Verifies that the `type` built-in reports `"Array"` for an array literal.
///
/// # Examples
///
/// ```
/// // Creates an interpreter, runs a script that sets x to the type of an array,
/// // and asserts the variable `x` holds the string "Array".
/// let (interp, _) = run_script("dim x = type([1, 2])").await;
/// assert!(matches!(interp.get_var("x").await, Some(Value::String(s)) if s == "Array"));
/// ```
#[tokio::test]
async fn test_type_array() {
    let (interp, _) = run_script("dim x = type([1, 2])").await;
    assert!(matches!(interp.get_var("x").await, Some(Value::String(s)) if s == "Array"));
}

#[tokio::test]
async fn test_str_conversion() {
    let (interp, _) = run_script("dim x = str(42)").await;
    assert!(matches!(interp.get_var("x").await, Some(Value::String(s)) if s == "42"));
}

/// Verifies that converting a numeric string with `int(...)` yields an integer assigned to a variable.
///
/// # Examples
///
/// ```rust
/// let (interp, _) = run_script(r#"dim x = int("42")"#).await;
/// assert!(matches!(interp.get_var("x").await, Some(Value::Int(42))));
/// ```
#[tokio::test]
async fn test_int_conversion() {
    let (interp, _) = run_script(r#"dim x = int("42")"#).await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Int(42))));
}

#[tokio::test]
async fn test_float_conversion() {
    let (interp, _) = run_script(r#"dim x = float("3.14")"#).await;
    match interp.get_var("x").await {
        Some(Value::Float(f)) => assert!((f - 3.14).abs() < 0.001),
        _ => panic!("Expected float"),
    }
}

/// Checks that the `abs` built-in returns the same value for a positive integer.
///
/// # Examples
///
/// ```
/// # async fn run() {
/// let (interp, _) = run_script("dim x = abs(42)").await;
/// assert!(matches!(interp.get_var("x").await, Some(Value::Int(42))));
/// # }
/// ```
#[tokio::test]
async fn test_abs_positive() {
    let (interp, _) = run_script("dim x = abs(42)").await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Int(42))));
}

/// Verifies that `abs` applied to a negative integer produces its positive integer value and assigns it to `x`.
///
/// # Examples
///
/// ```
/// let (interp, _) = run_script("dim x = abs(-42)").await;
/// assert!(matches!(interp.get_var("x").await, Some(Value::Int(42))));
/// ```
#[tokio::test]
async fn test_abs_negative() {
    let (interp, _) = run_script("dim x = abs(-42)").await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Int(42))));
}

#[tokio::test]
async fn test_min() {
    let (interp, _) = run_script("dim x = min(5, 3, 8, 1, 9)").await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Int(1))));
}

#[tokio::test]
async fn test_max() {
    let (interp, _) = run_script("dim x = max(5, 3, 8, 1, 9)").await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Int(9))));
}

#[tokio::test]
async fn test_sum() {
    let (interp, _) = run_script("dim x = sum([1, 2, 3, 4, 5])").await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Int(15))));
}

#[tokio::test]
async fn test_avg() {
    let (interp, _) = run_script("dim x = avg([10, 20, 30])").await;
    match interp.get_var("x").await {
        Some(Value::Float(f)) => assert!((f - 20.0).abs() < 0.001),
        _ => panic!("Expected float"),
    }
}

/// Verifies that calling `keys(obj)` on an object returns an array of its field names.
///
/// Asserts that an object with two fields produces an array containing two elements.
///
/// # Examples
///
/// ```
/// // Create an object and retrieve its keys
/// let (interp, _) = run_script(r#"dim obj = { a: 1, b: 2 }
/// dim k = keys(obj)"#).await;
/// match interp.get_var("k").await {
///     Some(Value::Array(items)) => assert_eq!(items.len(), 2),
///     _ => panic!("Expected array"),
/// }
/// ```
#[tokio::test]
async fn test_keys() {
    let (interp, _) = run_script(
        r#"dim obj = { a: 1, b: 2 }
dim k = keys(obj)"#,
    )
    .await;
    match interp.get_var("k").await {
        Some(Value::Array(items)) => {
            assert_eq!(items.len(), 2);
        }
        _ => panic!("Expected array"),
    }
}

/// Verifies that extracting values from an object with `values(obj)` yields an array of the object's values.
///
/// # Examples
///
/// ```
/// // Creates an object and extracts its values into an array.
/// let (interp, _) = run_script(r#"dim obj = { a: 1, b: 2 }
/// dim v = values(obj)"#).await;
/// match interp.get_var("v").await {
///     Some(Value::Array(items)) => assert_eq!(items.len(), 2),
///     _ => panic!("Expected array"),
/// }
/// ```
#[tokio::test]
async fn test_values() {
    let (interp, _) = run_script(
        r#"dim obj = { a: 1, b: 2 }
dim v = values(obj)"#,
    )
    .await;
    match interp.get_var("v").await {
        Some(Value::Array(items)) => {
            assert_eq!(items.len(), 2);
        }
        _ => panic!("Expected array"),
    }
}
