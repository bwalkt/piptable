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

#[tokio::test]
async fn test_byval_keyword_behaves_as_value_copy() {
    let (interp, _) = run_script(
        r#"
        function add_one(ByVal x)
            x = x + 1
            return x
        end function
        dim original = 5
        dim result = add_one(original)
    "#,
    )
    .await;
    assert!(matches!(
        interp.get_var("result").await,
        Some(Value::Int(6))
    ));
    assert!(matches!(
        interp.get_var("original").await,
        Some(Value::Int(5))
    ));
}

#[tokio::test]
async fn test_byref_updates_caller() {
    let (interp, _) = run_script(
        r#"
        function increment(ByRef x)
            x = x + 1
        end function
        dim n = 5
        call increment(n)
    "#,
    )
    .await;
    assert!(matches!(interp.get_var("n").await, Some(Value::Int(6))));
}

#[tokio::test]
async fn test_byref_requires_variable_argument() {
    let error_msg = run_script_err(
        r#"
        function increment(ByRef x)
            x = x + 1
        end function
        call increment(5)
    "#,
    )
    .await;
    assert!(error_msg.contains("ByRef parameter"));
}

#[tokio::test]
async fn test_byref_array_element() {
    let (interp, _) = run_script(
        r#"
        function bump(ByRef x)
            x = x + 1
        end function
        dim arr = [1, 2, 3]
        call bump(arr[1])
        dim result = arr[1]
    "#,
    )
    .await;
    assert!(matches!(
        interp.get_var("result").await,
        Some(Value::Int(3))
    ));
}

#[tokio::test]
async fn test_byref_object_field() {
    let (interp, _) = run_script(
        r#"
        function bump(ByRef x)
            x = x + 1
        end function
        dim obj = { a: 1, b: 2 }
        call bump(obj->a)
        dim result = obj->a
    "#,
    )
    .await;
    assert!(matches!(
        interp.get_var("result").await,
        Some(Value::Int(2))
    ));
}

#[tokio::test]
async fn test_optional_param_default() {
    let (interp, _) = run_script(
        r#"
        function add(a, optional b = 1)
            return a + b
        end function
        dim x = add(3)
        dim y = add(3, 4)
    "#,
    )
    .await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Int(4))));
    assert!(matches!(interp.get_var("y").await, Some(Value::Int(7))));
}

#[tokio::test]
async fn test_paramarray_collects_args() {
    let (interp, _) = run_script(
        r#"
        function sum_all(paramarray nums)
            return sum(nums)
        end function
        dim a = sum_all(1, 2, 3)
        dim b = sum_all()
    "#,
    )
    .await;
    assert!(matches!(
        interp.get_var("a").await,
        Some(Value::Float(f)) if (f - 6.0).abs() < 1e-9
    ));
    assert!(matches!(
        interp.get_var("b").await,
        Some(Value::Float(f)) if f.abs() < 1e-9
    ));
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

#[test]
fn test_recursive_function() {
    let handle = std::thread::Builder::new()
        .name("recursive-function-test".to_string())
        .stack_size(8 * 1024 * 1024)
        .spawn(|| {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime");
            runtime.block_on(async {
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
            });
        })
        .expect("spawn test thread");

    handle.join().expect("join test thread");
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

/// Verifies that a function procedure can modify a module-level variable.
///
/// # Examples
///
/// ```
/// #[tokio::test]
/// async fn example_function_procedure_increments_module_var() {
///     let (interp, _) = run_script(
///         r#"
///         dim counter = 0
///         function increment()
///             counter = counter + 1
///         end function
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
async fn test_function_procedure() {
    let (interp, _) = run_script(
        r#"
        dim counter = 0
        function increment()
            counter = counter + 1
        end function
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

#[tokio::test]
#[ignore = "planned parameter coverage"]
async fn test_complex_param_combinations() {
    let (interp, _) = run_script(
        r#"
        function complex(byval a, byref b, byval c)
            b = b + a
            return a + b + c
        end function
        dim x = 5
        dim result = complex(2, x, 10)
    "#,
    )
    .await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Int(7)))); // 5 + 2
    assert!(matches!(
        interp.get_var("result").await,
        Some(Value::Int(19))
    )); // 2 + 7 + 10
}

#[tokio::test]
#[ignore = "planned parameter coverage"]
async fn test_multiple_byref_params() {
    let (interp, _) = run_script(
        r#"
        function modify_both(byref a, byref b, byval multiplier)
            a = a * multiplier
            b = b + multiplier
        end function
        dim x = 5
        dim y = 10
        call modify_both(x, y, 3)
    "#,
    )
    .await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Int(15)))); // 5 * 3
    assert!(matches!(interp.get_var("y").await, Some(Value::Int(13)))); // 10 + 3
}

#[tokio::test]
#[ignore = "planned parameter coverage"]
async fn test_invalid_byref_argument_error() {
    let error_msg = run_script_err(
        r#"
        function modify(byref x)
            x = x + 1
        end function
        call modify(42)
    "#,
    )
    .await;
    assert!(error_msg.contains("ByRef") || error_msg.contains("reference"));
}

#[tokio::test]
#[ignore = "planned parameter coverage"]
async fn test_mixed_byval_byref_semantics() {
    let (interp, _) = run_script(
        r#"
        function mixed(byval a, byref b, byval c)
            a = a + 10  ' Should not affect original
            b = b + 20  ' Should affect original
            return a + b + c
        end function
        dim x = 1
        dim y = 2
        dim result = mixed(x, y, 5)
    "#,
    )
    .await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Int(1)))); // unchanged (ByVal)
    assert!(matches!(interp.get_var("y").await, Some(Value::Int(22)))); // changed (ByRef)
    assert!(matches!(
        interp.get_var("result").await,
        Some(Value::Int(38))
    )); // 11 + 22 + 5
}

#[tokio::test]
#[ignore = "planned parameter coverage"]
async fn test_nested_function_calls_with_byref() {
    let (interp, _) = run_script(
        r#"
        function increment(byref x)
            x = x + 1
        end function
        
        function double_increment(byref y)
            call increment(y)
            call increment(y)
        end function
        
        dim value = 10
        call double_increment(value)
    "#,
    )
    .await;
    assert!(matches!(
        interp.get_var("value").await,
        Some(Value::Int(12))
    )); // 10 + 1 + 1
}

#[tokio::test]
#[ignore = "planned parameter coverage"]
async fn test_function_with_all_byref_params() {
    let (interp, _) = run_script(
        r#"
        function transform_values(byref a, byref b, byref c)
            dim temp = a
            a = b
            b = c
            c = temp
        end function
        dim x = 1
        dim y = 2
        dim z = 3
        call transform_values(x, y, z)
    "#,
    )
    .await;
    assert!(matches!(interp.get_var("x").await, Some(Value::Int(2)))); // x = original y
    assert!(matches!(interp.get_var("y").await, Some(Value::Int(3)))); // y = original z
    assert!(matches!(interp.get_var("z").await, Some(Value::Int(1)))); // z = original x
}
