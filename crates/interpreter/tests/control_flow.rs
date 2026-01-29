//! Control_Flow tests for the PipTable interpreter.

#![allow(clippy::needless_raw_string_hashes)]

mod common {
    include!("common_impl.txt");
}
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

#[tokio::test]
async fn test_exit_function() {
    let (interp, _) = run_script(
        r#"
        function testFunc()
            dim x = 1
            if x = 1 then
                exit function
            end if
            x = 99
            return x
        end function
        
        dim result = testFunc()
    "#,
    )
    .await;
    // Should return null since exit function was called before return
    assert!(matches!(interp.get_var("result").await, Some(Value::Null)));
}

#[tokio::test]
async fn test_exit_sub() {
    let (interp, _) = run_script(
        r#"
        dim executed = false
        
        sub testSub()
            executed = true
            if true then
                exit sub
            end if
            executed = false
        end sub
        
        call testSub()
    "#,
    )
    .await;
    // Should be true since exit sub was called after setting it to true
    assert!(matches!(
        interp.get_var("executed").await,
        Some(Value::Bool(true))
    ));
}

#[tokio::test]
async fn test_exit_for() {
    let (interp, _) = run_script(
        r#"
        dim sum = 0
        for i = 1 to 10
            sum = sum + i
            if i = 3 then
                exit for
            end if
        next
    "#,
    )
    .await;
    // Should be 1 + 2 + 3 = 6 since loop exited at i = 3
    assert!(matches!(interp.get_var("sum").await, Some(Value::Int(6))));
}

#[tokio::test]
async fn test_exit_for_with_step() {
    let (interp, _) = run_script(
        r#"
        dim sum = 0
        for i = 0 to 20 step 3
            sum = sum + i
            if i >= 6 then
                exit for
            end if
        next
    "#,
    )
    .await;
    // Should be 0 + 3 + 6 = 9 since loop exited when i >= 6
    assert!(matches!(interp.get_var("sum").await, Some(Value::Int(9))));
}

#[tokio::test]
async fn test_exit_foreach() {
    let (interp, _) = run_script(
        r#"
        dim items = [1, 2, 3, 4, 5]
        dim sum = 0
        for each item in items
            sum = sum + item
            if item = 3 then
                exit for
            end if
        next
    "#,
    )
    .await;
    // Should be 1 + 2 + 3 = 6 since loop exited at item = 3
    assert!(matches!(interp.get_var("sum").await, Some(Value::Int(6))));
}

#[tokio::test]
async fn test_exit_while() {
    let (interp, _) = run_script(
        r#"
        dim x = 0
        while x < 10
            x = x + 1
            if x = 4 then
                exit while
            end if
        wend
    "#,
    )
    .await;
    // Should be 4 since loop exited when x = 4
    assert!(matches!(interp.get_var("x").await, Some(Value::Int(4))));
}

#[tokio::test]
#[ignore = "nested for loops parser issue"]
async fn test_exit_for_nested() {
    let (interp, _) = run_script(
        r#"
        dim outerSum = 0
        dim innerSum = 0
        
        for i = 1 to 3
            outerSum = outerSum + i
            for j = 1 to 5
                innerSum = innerSum + j
                if j = 2 then
                    exit for
                end if
            next
        next
    "#,
    )
    .await;
    // outerSum should be 1 + 2 + 3 = 6
    assert!(matches!(
        interp.get_var("outerSum").await,
        Some(Value::Int(6))
    ));
    // innerSum should be (1 + 2) * 3 = 9 (inner loop runs 3 times, exits at j=2 each time)
    assert!(matches!(
        interp.get_var("innerSum").await,
        Some(Value::Int(9))
    ));
}

#[tokio::test]
async fn test_exit_function_with_return_value() {
    let (interp, _) = run_script(
        r#"
        function getValue(x)
            if x < 0 then
                return -1
            end if
            if x = 0 then
                exit function  ' Should return null
            end if
            return x * 2
        end function
        
        dim result1 = getValue(-5)
        dim result2 = getValue(0) 
        dim result3 = getValue(3)
    "#,
    )
    .await;
    assert!(matches!(
        interp.get_var("result1").await,
        Some(Value::Int(-1))
    ));
    assert!(matches!(interp.get_var("result2").await, Some(Value::Null)));
    assert!(matches!(
        interp.get_var("result3").await,
        Some(Value::Int(6))
    ));
}

#[tokio::test]
async fn test_exit_function_doesnt_leak_previous_value() {
    let (interp, _) = run_script(
        r#"
        function helperFunc() 
            return 999
        end function
        
        function testFunc()
            dim temp = helperFunc()  ' This returns 999
            temp = temp + 1          ' This becomes 1000
            exit function            ' Should return null, not 1000
        end function
        
        dim result = testFunc()
    "#,
    )
    .await;
    // Should be null, not 1000 from the previous statement
    assert!(matches!(interp.get_var("result").await, Some(Value::Null)));
}

#[tokio::test]
async fn test_exit_sub_doesnt_leak_previous_value() {
    let (interp, _) = run_script(
        r#"
        dim global_result = 0
        
        function helperFunc() 
            return 555
        end function
        
        sub testSub()
            dim temp = helperFunc()  ' This returns 555
            global_result = temp     ' Set global to 555
            exit sub                 ' Should not return 555
        end sub
        
        dim result = testSub()  ' Sub calls should return null
    "#,
    )
    .await;
    // result should be null, not 555
    assert!(matches!(interp.get_var("result").await, Some(Value::Null)));
    // But global_result should be 555 to confirm the sub executed
    assert!(matches!(
        interp.get_var("global_result").await,
        Some(Value::Int(555))
    ));
}

#[tokio::test]
async fn test_exit_function_outside_function_error() {
    let script = r#"
        dim x = 1
        exit function
    "#;
    let error_msg = run_script_err(script).await;
    assert!(error_msg.contains("Exit Function cannot be used outside of a function"));
}

#[tokio::test]
async fn test_exit_sub_outside_sub_error() {
    let script = r#"
        dim x = 1
        exit sub
    "#;
    let error_msg = run_script_err(script).await;
    assert!(error_msg.contains("Exit Sub cannot be used outside of a subroutine"));
}

#[tokio::test]
async fn test_exit_for_outside_loop_error() {
    let script = r#"
        dim x = 1
        exit for
    "#;
    let error_msg = run_script_err(script).await;
    assert!(error_msg.contains("Exit For cannot be used outside of a for loop"));
}

#[tokio::test]
async fn test_exit_while_outside_loop_error() {
    let script = r#"
        dim x = 1
        exit while
    "#;
    let error_msg = run_script_err(script).await;
    assert!(error_msg.contains("Exit While cannot be used outside of a while loop"));
}

#[tokio::test]
async fn test_exit_function_in_sub_error() {
    let script = r#"
        sub testSub()
            exit function  ' Should error - this is a sub, not a function
        end sub
        call testSub()
    "#;
    let error_msg = run_script_err(script).await;
    assert!(error_msg.contains("Exit Function cannot be used in a subroutine"));
}

#[tokio::test]
async fn test_exit_sub_in_function_error() {
    let script = r#"
        function testFunc()
            exit sub  ' Should error - this is a function, not a sub
        end function
        dim result = testFunc()
    "#;
    let error_msg = run_script_err(script).await;
    assert!(error_msg.contains("Exit Sub cannot be used in a function"));
}

#[tokio::test]
async fn test_correct_exit_usage() {
    let (interp, _) = run_script(
        r#"
        function testFunc()
            exit function  ' Correct usage
        end function
        
        sub testSub() 
            exit sub      ' Correct usage
        end sub
        
        dim result1 = testFunc()
        call testSub()
    "#,
    )
    .await;
    // Should execute without errors and return null for function
    assert!(matches!(interp.get_var("result1").await, Some(Value::Null)));
}

#[tokio::test]
async fn test_exit_with_correct_line_numbers() {
    let script = r#"
        dim x = 1
        exit function
    "#;
    let error_msg = run_script_err(script).await;
    assert!(error_msg.contains("Exit Function cannot be used outside of a function"));
    // The exit function statement should be on line 3 (accounting for the newline at the start)
    assert!(error_msg.contains("line 3"));
}
