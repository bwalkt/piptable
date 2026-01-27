//! Strings tests for the PipTable interpreter.

#![allow(clippy::approx_constant)]
#![allow(clippy::needless_raw_string_hashes)]

mod common;
use common::*;

use piptable_core::Value;

#[tokio::test]
async fn test_string_escape_newline() {
    let (interp, _) = run_script(r#"dim x = "line1\nline2""#).await;
    match interp.get_var("x").await {
        Some(Value::String(s)) => assert!(s.contains('\n')),
        _ => panic!("Expected string"),
    }
}

/// Verifies that a string literal containing the `\t` escape is parsed and stored with an actual tab character.
///
/// Runs a script that assigns `"col1\tcol2"` to variable `x` and asserts the interpreter returns a `String` containing `'\t'`.
///
/// # Examples
///
/// ```
/// let (interp, _) = run_script(r#"dim x = "col1\tcol2""#).await;
/// match interp.get_var("x").await {
///     Some(Value::String(s)) => assert!(s.contains('\t')),
///     _ => panic!("Expected string"),
/// }
/// ```
#[tokio::test]
async fn test_string_escape_tab() {
    let (interp, _) = run_script(r#"dim x = "col1\tcol2""#).await;
    match interp.get_var("x").await {
        Some(Value::String(s)) => assert!(s.contains('\t')),
        _ => panic!("Expected string"),
    }
}

/// Verifies that an empty string literal assigns an empty `String` value to a variable.
///
/// # Examples
///
/// ```
/// // Asynchronously run a script that defines an empty string and check the variable.
/// let (interp, _) = run_script(r#"dim x = """#).await;
/// assert!(matches!(interp.get_var("x").await, Some(Value::String(s)) if s.is_empty()));
/// ```
#[tokio::test]
async fn test_empty_string() {
    let (interp, _) = run_script(r#"dim x = """#).await;
    assert!(matches!(interp.get_var("x").await, Some(Value::String(s)) if s.is_empty()));
}

#[tokio::test]
async fn test_len_empty_string() {
    let (interp, _) = run_script(r#"dim x = len("")"#).await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Int(0))));
}
