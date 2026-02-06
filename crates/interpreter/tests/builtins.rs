//! Builtins tests for the PipTable interpreter.

#![allow(clippy::needless_raw_string_hashes)]

/// Shared test helpers.
mod common {
    include!("common_impl.txt");
}
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
    let expected: f64 = "3.14".parse().unwrap();
    match interp.get_var("x").await {
        Some(Value::Float(f)) => assert!((f - expected).abs() < 0.001),
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
    assert!(matches!(
        interp.get_var("x").await,
        Some(Value::Float(f)) if (f - 1.0).abs() < 1e-9
    ));
}

#[tokio::test]
async fn test_max() {
    let (interp, _) = run_script("dim x = max(5, 3, 8, 1, 9)").await;
    assert!(matches!(
        interp.get_var("x").await,
        Some(Value::Float(f)) if (f - 9.0).abs() < 1e-9
    ));
}

#[tokio::test]
async fn test_sum() {
    let (interp, _) = run_script("dim x = sum([1, 2, 3, 4, 5])").await;
    assert!(matches!(
        interp.get_var("x").await,
        Some(Value::Float(f)) if (f - 15.0).abs() < 1e-9
    ));
}

#[tokio::test]
async fn test_avg() {
    let (interp, _) = run_script("dim x = avg([10, 20, 30])").await;
    match interp.get_var("x").await {
        Some(Value::Float(f)) => assert!((f - 20.0).abs() < 0.001),
        _ => panic!("Expected float"),
    }
}

#[tokio::test]
async fn test_len_book_and_values() {
    let script = r#"
        dim book = book_from_dict({
            "One": [["col"], [1]],
            "Two": [["col"], [2]]
        })
        dim count = book_sheet_count(book)
        dim names = keys(book)
        dim sheets = values(book)
    "#;
    let (interp, _) = run_script(script).await;
    assert!(matches!(interp.get_var("count").await, Some(Value::Int(2))));
    match interp.get_var("names").await {
        Some(Value::Array(items)) => assert_eq!(items.len(), 2),
        _ => panic!("Expected names array"),
    }
    match interp.get_var("sheets").await {
        Some(Value::Array(items)) => assert_eq!(items.len(), 2),
        _ => panic!("Expected sheets array"),
    }
}

#[tokio::test]
async fn test_len_sheet_header_offset() {
    let script = r#"
        dim book = book_from_dict({
            "One": [["name", "age"], ["A", 1], ["B", 2]]
        })
        dim sheet = book_get_sheet(book, "One")
        sheet = sheet_name_columns_by_row(sheet, 0)
        dim count = len(sheet)
    "#;
    let (interp, _) = run_script(script).await;
    assert!(matches!(interp.get_var("count").await, Some(Value::Int(2))));
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

#[tokio::test]
async fn test_sum_array() {
    let (interp, _) = run_script("dim x = sum([1, 2, 3, 4])").await;
    assert!(matches!(
        interp.get_var("x").await,
        Some(Value::Float(f)) if (f - 10.0).abs() < 1e-9
    ));
}

#[tokio::test]
async fn test_sum_mixed_types_in_array() {
    let (interp, _) = run_script(r#"dim x = sum([1, 2.5, "text", 3])"#).await;
    assert!(matches!(
        interp.get_var("x").await,
        Some(Value::Float(f)) if (f - 6.5).abs() < 1e-9
    ));
}

#[tokio::test]
async fn test_sum_empty_array() {
    let (interp, _) = run_script("dim x = sum([])").await;
    assert!(matches!(
        interp.get_var("x").await,
        Some(Value::Float(f)) if f.abs() < 1e-9
    ));
}

#[tokio::test]
async fn test_sum_nested_arrays() {
    let (interp, _) = run_script("dim x = sum([[1, 2], [3, 4]])").await;
    assert!(matches!(
        interp.get_var("x").await,
        Some(Value::Float(f)) if (f - 10.0).abs() < 1e-9
    ));
}

#[tokio::test]
async fn test_avg_basic() {
    let (interp, _) = run_script("dim x = avg([10, 20, 30, 40])").await;
    assert!(matches!(
        interp.get_var("x").await,
        Some(Value::Float(f)) if (f - 25.0).abs() < 1e-9
    ));
}

#[tokio::test]
async fn test_avg_single_value() {
    let (interp, _) = run_script("dim x = avg([42])").await;
    assert!(matches!(
        interp.get_var("x").await,
        Some(Value::Float(f)) if (f - 42.0).abs() < 1e-9
    ));
}

#[tokio::test]
async fn test_avg_alias() {
    let (interp, _) = run_script("dim x = average([10, 20, 30])").await;
    assert!(matches!(
        interp.get_var("x").await,
        Some(Value::Float(f)) if (f - 20.0).abs() < 1e-9
    ));
}

#[tokio::test]
async fn test_min_array() {
    let (interp, _) = run_script("dim x = min([5, 2, 8, 1, 9])").await;
    assert!(matches!(
        interp.get_var("x").await,
        Some(Value::Float(f)) if (f - 1.0).abs() < 1e-9
    ));
}

#[tokio::test]
async fn test_min_negative_numbers() {
    let (interp, _) = run_script("dim x = min([-5, -10, -3])").await;
    assert!(matches!(
        interp.get_var("x").await,
        Some(Value::Float(f)) if (f - (-10.0)).abs() < 1e-9
    ));
}

#[tokio::test]
async fn test_min_single_value() {
    let (interp, _) = run_script("dim x = min([42])").await;
    assert!(matches!(
        interp.get_var("x").await,
        Some(Value::Float(f)) if (f - 42.0).abs() < 1e-9
    ));
}

#[tokio::test]
async fn test_max_array() {
    let (interp, _) = run_script("dim x = max([5, 2, 8, 1, 9])").await;
    assert!(matches!(
        interp.get_var("x").await,
        Some(Value::Float(f)) if (f - 9.0).abs() < 1e-9
    ));
}

#[tokio::test]
async fn test_max_negative_numbers() {
    let (interp, _) = run_script("dim x = max([-5, -10, -3])").await;
    assert!(matches!(
        interp.get_var("x").await,
        Some(Value::Float(f)) if (f - (-3.0)).abs() < 1e-9
    ));
}

#[tokio::test]
async fn test_max_single_value() {
    let (interp, _) = run_script("dim x = max([42])").await;
    assert!(matches!(
        interp.get_var("x").await,
        Some(Value::Float(f)) if (f - 42.0).abs() < 1e-9
    ));
}

#[tokio::test]
async fn test_abs_float() {
    let (interp, _) = run_script("dim x = abs(-3.14)").await;
    assert!(matches!(
        interp.get_var("x").await,
        Some(Value::Float(f)) if (f - 3.14).abs() < 1e-9
    ));
}

#[tokio::test]
async fn test_abs_zero() {
    let (interp, _) = run_script("dim x = abs(0)").await;
    assert!(matches!(
        interp.get_var("x").await,
        Some(Value::Int(0))
    ));
}

#[tokio::test]
async fn test_min_with_multiple_args() {
    let (interp, _) = run_script("dim x = min(10, 5, 20, 3)").await;
    assert!(matches!(
        interp.get_var("x").await,
        Some(Value::Float(f)) if (f - 3.0).abs() < 1e-9
    ));
}

#[tokio::test]
async fn test_max_with_multiple_args() {
    let (interp, _) = run_script("dim x = max(10, 5, 20, 3)").await;
    assert!(matches!(
        interp.get_var("x").await,
        Some(Value::Float(f)) if (f - 20.0).abs() < 1e-9
    ));
}

#[tokio::test]
async fn test_sum_with_floats() {
    let (interp, _) = run_script("dim x = sum([1.5, 2.5, 3.0])").await;
    assert!(matches!(
        interp.get_var("x").await,
        Some(Value::Float(f)) if (f - 7.0).abs() < 1e-9
    ));
}

#[tokio::test]
async fn test_avg_with_mixed_types() {
    let (interp, _) = run_script(r#"dim x = avg([10, 20, "text", 30])"#).await;
    assert!(matches!(
        interp.get_var("x").await,
        Some(Value::Float(f)) if (f - 20.0).abs() < 1e-9
    ));
}

#[tokio::test]
async fn test_min_with_zero() {
    let (interp, _) = run_script("dim x = min([0, 5, -5])").await;
    assert!(matches!(
        interp.get_var("x").await,
        Some(Value::Float(f)) if (f - (-5.0)).abs() < 1e-9
    ));
}

#[tokio::test]
async fn test_max_with_zero() {
    let (interp, _) = run_script("dim x = max([0, -5, -10])").await;
    assert!(matches!(
        interp.get_var("x").await,
        Some(Value::Float(f)) if f.abs() < 1e-9
    ));
}

#[tokio::test]
async fn test_sum_with_nested_and_mixed() {
    let (interp, _) = run_script(r#"dim x = sum([1, [2, 3], "text", 4])"#).await;
    assert!(matches!(
        interp.get_var("x").await,
        Some(Value::Float(f)) if (f - 10.0).abs() < 1e-9
    ));
}

#[tokio::test]
async fn test_abs_with_positive_int() {
    let (interp, _) = run_script("dim x = abs(5)").await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Int(5))));
}

#[tokio::test]
async fn test_avg_with_floats_and_ints() {
    let (interp, _) = run_script("dim x = avg([1, 2.0, 3, 4.0])").await;
    assert!(matches!(
        interp.get_var("x").await,
        Some(Value::Float(f)) if (f - 2.5).abs() < 1e-9
    ));
}

#[tokio::test]
async fn test_min_float_precision() {
    let (interp, _) = run_script("dim x = min([1.1, 1.2, 1.05])").await;
    assert!(matches!(
        interp.get_var("x").await,
        Some(Value::Float(f)) if (f - 1.05).abs() < 1e-9
    ));
}

#[tokio::test]
async fn test_max_float_precision() {
    let (interp, _) = run_script("dim x = max([1.1, 1.2, 1.25])").await;
    assert!(matches!(
        interp.get_var("x").await,
        Some(Value::Float(f)) if (f - 1.25).abs() < 1e-9
    ));
}

#[tokio::test]
async fn test_sum_large_array() {
    let (interp, _) = run_script("dim x = sum([1, 2, 3, 4, 5, 6, 7, 8, 9, 10])").await;
    assert!(matches!(
        interp.get_var("x").await,
        Some(Value::Float(f)) if (f - 55.0).abs() < 1e-9
    ));
}

#[tokio::test]
async fn test_avg_large_array() {
    let (interp, _) = run_script("dim x = avg([1, 2, 3, 4, 5, 6, 7, 8, 9, 10])").await;
    assert!(matches!(
        interp.get_var("x").await,
        Some(Value::Float(f)) if (f - 5.5).abs() < 1e-9
    ));
}