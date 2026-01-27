//! Collections tests for the PipTable interpreter.

#![allow(clippy::needless_raw_string_hashes)]

mod _common;
use _common::*;

use piptable_core::Value;

/// Verifies that an array literal produces an Array value stored in a variable.
///
/// The test runs a script that defines `arr` as `[1, 2, 3]` and asserts the interpreter
/// stores an `Array` with three elements under the variable name `arr`.
///
/// # Examples
///
/// ```
/// let (interp, _) = run_script("dim arr = [1, 2, 3]").await;
/// match interp.get_var("arr").await {
///     Some(Value::Array(items)) => assert_eq!(items.len(), 3),
///     _ => panic!("Expected array"),
/// }
/// ```
#[tokio::test]
async fn test_array_literal() {
    let (interp, _) = run_script("dim arr = [1, 2, 3]").await;
    match interp.get_var("arr").await {
        Some(Value::Array(items)) => assert_eq!(items.len(), 3),
        _ => panic!("Expected array"),
    }
}

/// Verifies that indexing an array with a zero-based index yields the expected element.
///
/// # Examples
///
/// ```
/// # async fn run_example() {
/// let (interp, _) = run_script("dim arr = [10, 20, 30]\ndim x = arr[1]").await;
/// assert!(matches!(interp.get_var("x").await, Some(Value::Int(20))));
/// # }
/// ```
#[tokio::test]
async fn test_array_index() {
    let (interp, _) = run_script("dim arr = [10, 20, 30]\ndim x = arr[1]").await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Int(20))));
}

/// Verifies that indexing an array with a negative index accesses elements from the end.
///
/// This integration test runs a script that defines an array and assigns `x` to `arr[-1]`,
/// asserting the last element is returned.
///
/// # Examples
///
/// ```
/// // script: dim arr = [10, 20, 30]
/// //         dim x = arr[-1]
/// // resulting x should be 30
/// ```
#[tokio::test]
async fn test_array_negative_index() {
    let (interp, _) = run_script("dim arr = [10, 20, 30]\ndim x = arr[-1]").await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Int(30))));
}

/// Asserts that assigning a value to an array index updates the array and the updated element can be read.
///
/// # Examples
///
/// ```
/// let (interp, _) = run_script("dim arr = [1, 2, 3]\narr[1] = 99\ndim x = arr[1]").await;
/// assert!(matches!(interp.get_var("x").await, Some(Value::Int(99))));
/// ```
#[tokio::test]
async fn test_array_assignment() {
    let (interp, _) = run_script("dim arr = [1, 2, 3]\narr[1] = 99\ndim x = arr[1]").await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Int(99))));
}

/// Verifies that indexing into a nested array returns the expected inner element.
///
/// # Examples
///
/// ```
/// // Creates a 2x2 nested array and reads the element at arr[0][1], which is 2.
/// let (interp, _) = run_script("dim arr = [[1, 2], [3, 4]]\ndim x = arr[0][1]").await;
/// assert!(matches!(interp.get_var("x").await, Some(Value::Int(2))));
/// ```
#[tokio::test]
async fn test_nested_array() {
    let (interp, _) = run_script("dim arr = [[1, 2], [3, 4]]\ndim x = arr[0][1]").await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Int(2))));
}

#[tokio::test]
async fn test_object_literal() {
    let (interp, _) = run_script(r#"dim obj = { name: "test", value: 42 }"#).await;
    match interp.get_var("obj").await {
        Some(Value::Object(map)) => {
            assert_eq!(map.len(), 2);
            assert!(matches!(map.get("name"), Some(Value::String(s)) if s == "test"));
        }
        _ => panic!("Expected object"),
    }
}

#[tokio::test]
async fn test_object_field_access() {
    let (interp, _) = run_script(
        r#"dim obj = { name: "test", value: 42 }
dim x = obj.value"#,
    )
    .await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Int(42))));
}

#[tokio::test]
async fn test_object_bracket_access() {
    let (interp, _) = run_script(
        r#"dim obj = { name: "test" }
dim x = obj["name"]"#,
    )
    .await;
    assert!(matches!(interp.get_var("x").await, Some(Value::String(s)) if s == "test"));
}

#[tokio::test]
async fn test_object_field_assignment() {
    let (interp, _) = run_script(
        r#"dim obj = { value: 1 }
obj.value = 99
dim x = obj.value"#,
    )
    .await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Int(99))));
}
