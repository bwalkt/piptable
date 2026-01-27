//! Control_Flow tests for the PipTable interpreter.

#![allow(clippy::needless_raw_string_hashes)]

mod common;
use common::*;

use piptable_core::Value;

#[tokio::test]
async fn test_if_true_branch() {
    let (interp, _) = run_script(
        r#"
        dim result = 0
        if true then
            result = 1
        end if
    "#,
    )
    .await;
    assert!(matches!(
        interp.get_var("result").await,
        Some(Value::Int(1))
    ));
}

/// Asserts that an `if ... else` statement executes the `else` branch when the condition is false.
///
/// # Examples
///
/// ```no_run
/// // The script sets `result` to 2 because the `if` condition is false.
/// let script = r#"
/// dim result = 0
/// if false then
///     result = 1
/// else
///     result = 2
/// end if
/// "#;
/// // run_script is an async test helper that parses and evaluates the script.
/// // After execution, `result` holds the value 2.
/// ```
#[tokio::test]
async fn test_if_else_branch() {
    let (interp, _) = run_script(
        r#"
        dim result = 0
        if false then
            result = 1
        else
            result = 2
        end if
    "#,
    )
    .await;
    assert!(matches!(
        interp.get_var("result").await,
        Some(Value::Int(2))
    ));
}

#[tokio::test]
async fn test_if_elseif() {
    let (interp, _) = run_script(
        r#"
        dim x = 5
        dim result = 0
        if x > 10 then
            result = 1
        elseif x > 3 then
            result = 2
        else
            result = 3
        end if
    "#,
    )
    .await;
    assert!(matches!(
        interp.get_var("result").await,
        Some(Value::Int(2))
    ));
}

/// Verifies that nested `if` statements execute the inner branch when both outer and inner conditions are true.
///
/// # Examples
///
/// ```
/// // Runs a script with nested ifs that should set `result` to 1.
/// let (interp, _) = run_script(r#"
/// dim x = 5
/// dim y = 10
/// dim result = 0
/// if x > 0 then
///     if y > 5 then
///         result = 1
///     end if
/// end if
/// "#).await;
/// assert!(matches!(interp.get_var("result").await, Some(Value::Int(1))));
/// ```
#[tokio::test]
async fn test_nested_if() {
    let (interp, _) = run_script(
        r#"
        dim x = 5
        dim y = 10
        dim result = 0
        if x > 0 then
            if y > 5 then
                result = 1
            end if
        end if
    "#,
    )
    .await;
    assert!(matches!(
        interp.get_var("result").await,
        Some(Value::Int(1))
    ));
}

/// Verifies a basic `for` loop accumulates the sum of integers from 1 through 5 into `sum`.
///
/// # Examples
///
/// ```
/// #[tokio::test]
/// async fn example_for_loop_basic() {
///     let (interp, _) = run_script(
///         r#"
///         dim sum = 0
///         for i = 1 to 5
///             sum = sum + i
///         next
///     "#,
///     )
///     .await;
///     assert_eq!(interp.get_var("sum").await, Some(Value::Int(15)));
/// }
/// ```
#[tokio::test]
async fn test_for_loop_basic() {
    let (interp, _) = run_script(
        r#"
        dim sum = 0
        for i = 1 to 5
            sum = sum + i
        next
    "#,
    )
    .await;
    assert!(matches!(interp.get_var("sum").await, Some(Value::Int(15))));
}

#[tokio::test]
async fn test_for_loop_with_step() {
    let (interp, _) = run_script(
        r#"
        dim sum = 0
        for i = 0 to 10 step 2
            sum = sum + i
        next
    "#,
    )
    .await;
    // 0 + 2 + 4 + 6 + 8 + 10 = 30
    assert!(matches!(interp.get_var("sum").await, Some(Value::Int(30))));
}

#[tokio::test]
async fn test_for_loop_negative_step() {
    let (interp, _) = run_script(
        r#"
        dim sum = 0
        for i = 5 to 1 step -1
            sum = sum + i
        next
    "#,
    )
    .await;
    // 5 + 4 + 3 + 2 + 1 = 15
    assert!(matches!(interp.get_var("sum").await, Some(Value::Int(15))));
}

#[tokio::test]
async fn test_foreach_array() {
    let (interp, _) = run_script(
        r#"
        dim items = [1, 2, 3, 4, 5]
        dim sum = 0
        for each item in items
            sum = sum + item
        next
    "#,
    )
    .await;
    assert!(matches!(interp.get_var("sum").await, Some(Value::Int(15))));
}

#[tokio::test]
async fn test_while_loop() {
    let (interp, _) = run_script(
        r#"
        dim x = 0
        while x < 5
            x = x + 1
        wend
    "#,
    )
    .await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Int(5))));
}

/// Ensures a while loop whose condition is false initially never executes its body.
///
/// Asserts that the loop body is skipped and the loop-initialized variable retains its original value.
///
/// # Examples
///
/// ```
/// let (interp, _) = run_script(r#"
///     dim x = 10
///     while x < 5
///         x = x + 1
///     wend
/// "#).await;
/// assert!(matches!(interp.get_var("x").await, Some(Value::Int(10))));
/// ```
#[tokio::test]
async fn test_while_never_executes() {
    let (interp, _) = run_script(
        r#"
        dim x = 10
        while x < 5
            x = x + 1
        wend
    "#,
    )
    .await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Int(10))));
}

// TODO: Nested for loops have a parser issue - see GitHub issue
#[tokio::test]
#[ignore = "nested for loops parser issue"]
async fn test_nested_loops() {
    let (interp, _) = run_script(
        r#"
        dim sum = 0
        for i = 1 to 3
            for j = 1 to 3
                sum = sum + 1
            next
        next
    "#,
    )
    .await;
    assert!(matches!(interp.get_var("sum").await, Some(Value::Int(9))));
}
