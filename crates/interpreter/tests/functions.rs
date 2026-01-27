//! Functions tests for the PipTable interpreter.

#![allow(clippy::needless_raw_string_hashes)]

mod common {
    include!("common_impl.txt");
}
use common::*;

use piptable_core::Value;

/// Verifies that a user-defined function can return a computed value which is assigned to a variable.
///
/// # Examples
///
/// ```
/// let (interp, _) = run_script(r#"
///     function double(n)
///         return n * 2
///     end function
///     dim x = double(21)
/// "#).await;
/// assert!(matches!(interp.get_var("x").await, Some(Value::Int(42))));
/// ```
#[tokio::test]
async fn test_simple_function() {
    let (interp, _) = run_script(
        r#"
        function double(n)
            return n * 2
        end function
        dim x = double(21)
    "#,
    )
    .await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Int(42))));
}

#[tokio::test]
async fn test_function_multiple_params() {
    let (interp, _) = run_script(
        r#"
        function add(a, b)
            return a + b
        end function
        dim x = add(10, 32)
    "#,
    )
    .await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Int(42))));
}

/// Verifies that a parameterless function returns its declared value when invoked.
///
/// # Examples
///
/// ```
/// let (interp, _) = run_script(
///     r#"
///     function get_answer()
///         return 42
///     end function
///     dim x = get_answer()
/// "#,
/// )
/// .await;
/// assert!(matches!(interp.get_var("x").await, Some(Value::Int(42))));
/// ```
#[tokio::test]
async fn test_function_no_params() {
    let (interp, _) = run_script(
        r#"
        function get_answer()
            return 42
        end function
        dim x = get_answer()
    "#,
    )
    .await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Int(42))));
}

/// Verifies that a function can call another function and that nested calls return the expected value.
///
/// # Examples
///
/// ```no_run
/// #[tokio::test]
/// async fn example() {
///     let (interp, _) = run_script(
///         r#"
///         function double(n)
///             return n * 2
///         end function
///         function quadruple(n)
///             return double(double(n))
///         end function
///         dim x = quadruple(10)
///     "#,
///     )
///     .await;
///     assert!(matches!(interp.get_var("x").await, Some(Value::Int(40))));
/// }
/// ```
#[tokio::test]
async fn test_function_calls_function() {
    let (interp, _) = run_script(
        r#"
        function double(n)
            return n * 2
        end function
        function quadruple(n)
            return double(double(n))
        end function
        dim x = quadruple(10)
    "#,
    )
    .await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Int(40))));
}

#[tokio::test]
async fn test_recursive_function() {
    let (interp, _) = run_script(
        r#"
        function factorial(n)
            if n <= 1 then
                return 1
            else
                return n * factorial(n - 1)
            end if
        end function
        dim x = factorial(5)
    "#,
    )
    .await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Int(120))));
}

/// Verifies that a function's local variable remains local and that the function returns its computed value.
///
/// The test defines a global `global_x`, a function `compute` that declares a local `local_x` and returns `local_x * 2`, then assigns the call result to `result`. It asserts `result` equals `10` and `global_x` remains `100`.
///
/// # Examples
///
/// ```
/// // Script:
/// // dim global_x = 100
/// // function compute()
/// //     dim local_x = 5
/// //     return local_x * 2
/// // end function
/// // dim result = compute()
/// // After evaluation: result == 10, global_x == 100
/// ```
#[tokio::test]
async fn test_function_with_local_var() {
    let (interp, _) = run_script(
        r#"
        dim global_x = 100
        function compute()
            dim local_x = 5
            return local_x * 2
        end function
        dim result = compute()
    "#,
    )
    .await;
    assert!(matches!(
        interp.get_var("result").await,
        Some(Value::Int(10))
    ));
    // Global should be unchanged
    assert!(matches!(
        interp.get_var("global_x").await,
        Some(Value::Int(100))
    ));
}

/// Verifies that a sub procedure can modify a module-level variable.
///
/// # Examples
///
/// ```
/// #[tokio::test]
/// async fn example_sub_procedure_increments_module_var() {
///     let (interp, _) = run_script(
///         r#"
///         dim counter = 0
///         sub increment()
///             counter = counter + 1
///         end sub
///         increment()
///         increment()
///         increment()
///     "#,
///     )
///     .await;
///     assert!(matches!(
///         interp.get_var("counter").await,
///         Some(Value::Int(3))
///     ));
/// }
/// ```
#[tokio::test]
async fn test_sub_procedure() {
    let (interp, _) = run_script(
        r#"
        dim counter = 0
        sub increment()
            counter = counter + 1
        end sub
        increment()
        increment()
        increment()
    "#,
    )
    .await;
    assert!(matches!(
        interp.get_var("counter").await,
        Some(Value::Int(3))
    ));
}
