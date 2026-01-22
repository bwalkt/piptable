//! Integration tests for piptable interpreter.
//!
//! These tests verify the interpreter executes full scripts correctly.

use piptable_core::Value;
use piptable_interpreter::Interpreter;
use piptable_parser::PipParser;
use std::io::Write;
use tempfile::NamedTempFile;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ============================================================================
// Helper Functions
// ============================================================================

async fn run_script(script: &str) -> (Interpreter, Value) {
    let mut interp = Interpreter::new();
    let program = PipParser::parse_str(script).expect("Failed to parse script");
    let result = interp.eval(program).await.expect("Failed to eval script");
    (interp, result)
}

async fn run_script_err(script: &str) -> String {
    let mut interp = Interpreter::new();
    let program = PipParser::parse_str(script).expect("Failed to parse script");
    interp
        .eval(program)
        .await
        .expect_err("Expected error")
        .to_string()
}

fn create_temp_csv(content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::with_suffix(".csv").expect("Failed to create temp file");
    file.write_all(content.as_bytes())
        .expect("Failed to write temp file");
    file.flush().expect("Failed to flush temp file");
    file
}

// ============================================================================
// Variables and Assignment Tests
// ============================================================================

mod variables {
    use super::*;

    #[tokio::test]
    async fn test_dim_integer() {
        let (interp, _) = run_script("dim x = 42").await;
        assert!(matches!(interp.get_var("x").await, Some(Value::Int(42))));
    }

    #[tokio::test]
    async fn test_dim_float() {
        let (interp, _) = run_script("dim x = 3.14").await;
        match interp.get_var("x").await {
            Some(Value::Float(f)) => assert!((f - 3.14).abs() < 0.001),
            _ => panic!("Expected float"),
        }
    }

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

    #[tokio::test]
    async fn test_assignment_updates_variable() {
        let (interp, _) = run_script("dim x = 1\nx = 2").await;
        assert!(matches!(interp.get_var("x").await, Some(Value::Int(2))));
    }

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
}

// ============================================================================
// Arithmetic and Operators Tests
// ============================================================================

mod arithmetic {
    use super::*;

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

    #[tokio::test]
    async fn test_multiplication() {
        let (interp, _) = run_script("dim x = 6 * 7").await;
        assert!(matches!(interp.get_var("x").await, Some(Value::Int(42))));
    }

    #[tokio::test]
    async fn test_division() {
        let (interp, _) = run_script("dim x = 20 / 4").await;
        assert!(matches!(interp.get_var("x").await, Some(Value::Int(5))));
    }

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

    #[tokio::test]
    async fn test_operator_precedence() {
        let (interp, _) = run_script("dim x = 2 + 3 * 4").await;
        assert!(matches!(interp.get_var("x").await, Some(Value::Int(14))));
    }

    #[tokio::test]
    async fn test_parentheses() {
        let (interp, _) = run_script("dim x = (2 + 3) * 4").await;
        assert!(matches!(interp.get_var("x").await, Some(Value::Int(20))));
    }

    #[tokio::test]
    async fn test_float_arithmetic() {
        let (interp, _) = run_script("dim x = 10.5 + 2.5").await;
        match interp.get_var("x").await {
            Some(Value::Float(f)) => assert!((f - 13.0).abs() < 0.001),
            _ => panic!("Expected float"),
        }
    }

    #[tokio::test]
    async fn test_mixed_int_float() {
        let (interp, _) = run_script("dim x = 10 + 0.5").await;
        match interp.get_var("x").await {
            Some(Value::Float(f)) => assert!((f - 10.5).abs() < 0.001),
            _ => panic!("Expected float"),
        }
    }
}

// ============================================================================
// Comparison and Logical Operators Tests
// ============================================================================

mod comparisons {
    use super::*;

    #[tokio::test]
    async fn test_equal() {
        let (interp, _) = run_script("dim x = 5 == 5").await;
        assert!(matches!(interp.get_var("x").await, Some(Value::Bool(true))));
    }

    #[tokio::test]
    async fn test_not_equal() {
        let (interp, _) = run_script("dim x = 5 != 3").await;
        assert!(matches!(interp.get_var("x").await, Some(Value::Bool(true))));
    }

    #[tokio::test]
    async fn test_greater_than() {
        let (interp, _) = run_script("dim x = 10 > 5").await;
        assert!(matches!(interp.get_var("x").await, Some(Value::Bool(true))));
    }

    #[tokio::test]
    async fn test_less_than() {
        let (interp, _) = run_script("dim x = 3 < 7").await;
        assert!(matches!(interp.get_var("x").await, Some(Value::Bool(true))));
    }

    #[tokio::test]
    async fn test_greater_or_equal() {
        let (interp, _) = run_script("dim x = 5 >= 5").await;
        assert!(matches!(interp.get_var("x").await, Some(Value::Bool(true))));
    }

    #[tokio::test]
    async fn test_less_or_equal() {
        let (interp, _) = run_script("dim x = 5 <= 5").await;
        assert!(matches!(interp.get_var("x").await, Some(Value::Bool(true))));
    }

    #[tokio::test]
    async fn test_and_operator() {
        let (interp, _) = run_script("dim x = true and true").await;
        assert!(matches!(interp.get_var("x").await, Some(Value::Bool(true))));
    }

    #[tokio::test]
    async fn test_or_operator() {
        let (interp, _) = run_script("dim x = false or true").await;
        assert!(matches!(interp.get_var("x").await, Some(Value::Bool(true))));
    }

    #[tokio::test]
    async fn test_not_operator() {
        let (interp, _) = run_script("dim x = not false").await;
        assert!(matches!(interp.get_var("x").await, Some(Value::Bool(true))));
    }

    #[tokio::test]
    async fn test_short_circuit_and() {
        // If short-circuit works, undefined_var won't be evaluated
        let (interp, _) = run_script("dim x = false and undefined_var").await;
        assert!(matches!(
            interp.get_var("x").await,
            Some(Value::Bool(false))
        ));
    }

    #[tokio::test]
    async fn test_short_circuit_or() {
        // If short-circuit works, undefined_var won't be evaluated
        let (interp, _) = run_script("dim x = true or undefined_var").await;
        assert!(matches!(interp.get_var("x").await, Some(Value::Bool(true))));
    }
}

// ============================================================================
// Control Flow Tests
// ============================================================================

mod control_flow {
    use super::*;

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
        assert!(matches!(
            interp.get_var("sum").await,
            Some(Value::Int(15))
        ));
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
        assert!(matches!(
            interp.get_var("sum").await,
            Some(Value::Int(30))
        ));
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
        assert!(matches!(
            interp.get_var("sum").await,
            Some(Value::Int(15))
        ));
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
        assert!(matches!(
            interp.get_var("sum").await,
            Some(Value::Int(15))
        ));
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
}

// ============================================================================
// Function Tests
// ============================================================================

mod functions {
    use super::*;

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
}

// ============================================================================
// Array and Object Tests
// ============================================================================

mod collections {
    use super::*;

    #[tokio::test]
    async fn test_array_literal() {
        let (interp, _) = run_script("dim arr = [1, 2, 3]").await;
        match interp.get_var("arr").await {
            Some(Value::Array(items)) => assert_eq!(items.len(), 3),
            _ => panic!("Expected array"),
        }
    }

    #[tokio::test]
    async fn test_array_index() {
        let (interp, _) = run_script("dim arr = [10, 20, 30]\ndim x = arr[1]").await;
        assert!(matches!(interp.get_var("x").await, Some(Value::Int(20))));
    }

    #[tokio::test]
    async fn test_array_negative_index() {
        let (interp, _) = run_script("dim arr = [10, 20, 30]\ndim x = arr[-1]").await;
        assert!(matches!(interp.get_var("x").await, Some(Value::Int(30))));
    }

    #[tokio::test]
    async fn test_array_assignment() {
        let (interp, _) = run_script("dim arr = [1, 2, 3]\narr[1] = 99\ndim x = arr[1]").await;
        assert!(matches!(interp.get_var("x").await, Some(Value::Int(99))));
    }

    #[tokio::test]
    async fn test_nested_array() {
        let (interp, _) =
            run_script("dim arr = [[1, 2], [3, 4]]\ndim x = arr[0][1]").await;
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

    #[tokio::test]
    async fn test_object_bracket_assignment() {
        let (interp, _) = run_script(
            r#"dim obj = { name: "old" }
obj["name"] = "new"
dim x = obj["name"]"#,
        )
        .await;
        assert!(matches!(interp.get_var("x").await, Some(Value::String(s)) if s == "new"));
    }

    #[tokio::test]
    async fn test_object_bracket_assignment_new_key() {
        let (interp, _) = run_script(
            r#"dim obj = { a: 1 }
obj["b"] = 2
dim x = obj["b"]"#,
        )
        .await;
        assert!(matches!(interp.get_var("x").await, Some(Value::Int(2))));
    }

    #[tokio::test]
    async fn test_array_float_index_coercion() {
        // Float indices are coerced to int for consistency
        let (interp, _) = run_script(
            r#"dim arr = [10, 20, 30]
dim x = arr[1.0]"#,
        )
        .await;
        assert!(matches!(interp.get_var("x").await, Some(Value::Int(20))));
    }

    #[tokio::test]
    async fn test_array_float_index_assignment() {
        let (interp, _) = run_script(
            r#"dim arr = [10, 20, 30]
arr[1.0] = 99
dim x = arr[1]"#,
        )
        .await;
        assert!(matches!(interp.get_var("x").await, Some(Value::Int(99))));
    }
}

// ============================================================================
// Built-in Functions Tests
// ============================================================================

mod builtins {
    use super::*;

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

    #[tokio::test]
    async fn test_abs_positive() {
        let (interp, _) = run_script("dim x = abs(42)").await;
        assert!(matches!(interp.get_var("x").await, Some(Value::Int(42))));
    }

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
}

// ============================================================================
// SQL Query Tests
// ============================================================================

mod sql {
    use super::*;

    #[tokio::test]
    async fn test_simple_query() {
        let (interp, _) = run_script("dim result = query(SELECT 1 + 1 as sum)").await;
        assert!(matches!(interp.get_var("result").await, Some(Value::Table(_))));
    }

    #[tokio::test]
    async fn test_query_multiple_columns() {
        let (interp, _) =
            run_script("dim result = query(SELECT 1 as a, 2 as b, 3 as c)").await;
        match interp.get_var("result").await {
            Some(Value::Table(batches)) => {
                assert!(!batches.is_empty());
                let batch = &batches[0];
                assert_eq!(batch.num_columns(), 3);
                assert_eq!(batch.num_rows(), 1);
            }
            _ => panic!("Expected table"),
        }
    }

    // TODO: WHERE clause parsing has issues with the current grammar
    #[tokio::test]
    #[ignore = "SQL WHERE clause parsing issue"]
    async fn test_csv_query() {
        let csv_content = "id,name,value\n1,foo,100\n2,bar,200\n3,baz,300";
        let file = create_temp_csv(csv_content);
        let path = file.path().to_string_lossy().replace('\\', "/");

        let script = format!(
            r#"dim result = query(SELECT * FROM '{}' WHERE value > 150)"#,
            path
        );
        let (interp, _) = run_script(&script).await;

        match interp.get_var("result").await {
            Some(Value::Table(batches)) => {
                let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
                assert_eq!(total_rows, 2); // bar and baz
            }
            _ => panic!("Expected table"),
        }
    }

    // TODO: GROUP BY parsing has issues with the current grammar
    #[tokio::test]
    #[ignore = "SQL GROUP BY parsing issue"]
    async fn test_csv_aggregation() {
        let csv_content = "category,amount\nA,100\nB,200\nA,150\nB,50";
        let file = create_temp_csv(csv_content);
        let path = file.path().to_string_lossy().replace('\\', "/");

        let script = format!(
            r#"dim result = query(SELECT category, SUM(amount) as total FROM '{}' GROUP BY category ORDER BY category)"#,
            path
        );
        let (interp, _) = run_script(&script).await;

        match interp.get_var("result").await {
            Some(Value::Table(batches)) => {
                let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
                assert_eq!(total_rows, 2);
            }
            _ => panic!("Expected table"),
        }
    }

    // TODO: JOIN parsing has issues with the current grammar
    #[tokio::test]
    #[ignore = "SQL JOIN parsing issue"]
    async fn test_csv_join() {
        let users_csv = "id,name\n1,alice\n2,bob";
        let orders_csv = "user_id,amount\n1,100\n1,200\n2,50";

        let users_file = create_temp_csv(users_csv);
        let orders_file = create_temp_csv(orders_csv);

        let users_path = users_file.path().to_string_lossy().replace('\\', "/");
        let orders_path = orders_file.path().to_string_lossy().replace('\\', "/");

        let script = format!(
            r#"dim result = query(
                SELECT u.name, o.amount
                FROM '{}' as u
                JOIN '{}' as o ON u.id = o.user_id
                ORDER BY u.name, o.amount
            )"#,
            users_path, orders_path
        );
        let (interp, _) = run_script(&script).await;

        match interp.get_var("result").await {
            Some(Value::Table(batches)) => {
                let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
                assert_eq!(total_rows, 3);
            }
            _ => panic!("Expected table"),
        }
    }
}

// ============================================================================
// HTTP Fetch Tests
// ============================================================================

mod http {
    use super::*;

    #[tokio::test]
    async fn test_fetch_json_array() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/items"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!([
                        {"id": 1, "name": "item1"},
                        {"id": 2, "name": "item2"}
                    ])),
            )
            .mount(&server)
            .await;

        let script = format!(r#"dim data = fetch("{}/api/items")"#, server.uri());
        let (interp, _) = run_script(&script).await;

        match interp.get_var("data").await {
            Some(Value::Array(items)) => {
                assert_eq!(items.len(), 2);
            }
            _ => panic!("Expected array"),
        }
    }

    #[tokio::test]
    async fn test_fetch_json_object() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/user"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"id": 1, "name": "test"})),
            )
            .mount(&server)
            .await;

        let script = format!(r#"dim data = fetch("{}/api/user")"#, server.uri());
        let (interp, _) = run_script(&script).await;

        match interp.get_var("data").await {
            Some(Value::Object(map)) => {
                assert!(matches!(map.get("name"), Some(Value::String(s)) if s == "test"));
            }
            _ => panic!("Expected object"),
        }
    }

    #[tokio::test]
    async fn test_fetch_and_iterate() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/numbers"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!([1, 2, 3, 4, 5])),
            )
            .mount(&server)
            .await;

        let script = format!(
            r#"
            dim data = fetch("{}/api/numbers")
            dim sum = 0
            for each n in data
                sum = sum + n
            next
        "#,
            server.uri()
        );
        let (interp, _) = run_script(&script).await;

        assert!(matches!(
            interp.get_var("sum").await,
            Some(Value::Int(15))
        ));
    }
}

// ============================================================================
// Error Handling Tests
// ============================================================================

mod errors {
    use super::*;

    #[tokio::test]
    async fn test_undefined_variable() {
        let err = run_script_err("dim x = undefined_var").await;
        assert!(err.contains("Undefined variable"));
    }

    #[tokio::test]
    async fn test_undefined_function() {
        let err = run_script_err("dim x = unknown_func()").await;
        assert!(
            err.contains("Unknown function") || err.contains("Undefined function"),
            "Expected function error, got: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_division_by_zero() {
        let err = run_script_err("dim x = 10 / 0").await;
        assert!(err.contains("Division by zero"));
    }

    #[tokio::test]
    async fn test_modulo_by_zero() {
        let err = run_script_err("dim x = 10 % 0").await;
        assert!(err.contains("Modulo by zero"));
    }

    #[tokio::test]
    async fn test_array_index_out_of_bounds() {
        let err = run_script_err("dim arr = [1, 2, 3]\ndim x = arr[10]").await;
        assert!(err.contains("out of bounds"));
    }

    #[tokio::test]
    async fn test_negative_index_out_of_bounds() {
        let err = run_script_err("dim arr = [1, 2, 3]\ndim x = arr[-10]").await;
        assert!(err.contains("out of bounds"));
    }

    #[tokio::test]
    async fn test_for_loop_zero_step() {
        let err = run_script_err("for i = 1 to 10 step 0\nnext").await;
        assert!(err.contains("non-zero") || err.contains("step"));
    }

    #[tokio::test]
    async fn test_type_error_add_string_int() {
        // Adding string + int is a type error (use explicit concat or conversion)
        let err = run_script_err(r#"dim x = "hello" + 42"#).await;
        assert!(
            err.contains("Cannot add"),
            "Expected 'Cannot add' error, got: {}",
            err
        );
    }
}

// ============================================================================
// Edge Cases and Overflow Tests
// ============================================================================

mod edge_cases {
    use super::*;

    #[tokio::test]
    async fn test_empty_array() {
        let (interp, _) = run_script("dim arr = []").await;
        match interp.get_var("arr").await {
            Some(Value::Array(items)) => assert_eq!(items.len(), 0),
            _ => panic!("Expected empty array"),
        }
    }

    #[tokio::test]
    async fn test_empty_object() {
        let (interp, _) = run_script("dim obj = {}").await;
        match interp.get_var("obj").await {
            Some(Value::Object(map)) => assert_eq!(map.len(), 0),
            _ => panic!("Expected empty object"),
        }
    }

    #[tokio::test]
    async fn test_for_loop_no_iterations() {
        let (interp, _) = run_script(
            r#"
            dim count = 0
            for i = 10 to 1
                count = count + 1
            next
        "#,
        )
        .await;
        // With default step 1, 10 to 1 should not execute
        assert!(matches!(
            interp.get_var("count").await,
            Some(Value::Int(0))
        ));
    }

    #[tokio::test]
    async fn test_foreach_empty_array() {
        let (interp, _) = run_script(
            r#"
            dim items = []
            dim count = 0
            for each item in items
                count = count + 1
            next
        "#,
        )
        .await;
        assert!(matches!(
            interp.get_var("count").await,
            Some(Value::Int(0))
        ));
    }

    #[tokio::test]
    async fn test_integer_overflow_add() {
        let err = run_script_err(&format!("dim x = {} + 1", i64::MAX)).await;
        assert!(err.contains("overflow"));
    }

    // i64::MIN literal cannot be parsed directly (too large)
    // Test overflow via negation of i64::MAX and subtraction instead
    #[tokio::test]
    async fn test_integer_overflow_sub() {
        // Use a large negative number that can be parsed
        let err = run_script_err("dim x = -9223372036854775807 - 2").await;
        assert!(
            err.contains("overflow"),
            "Expected overflow error, got: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_integer_overflow_mul() {
        let err = run_script_err(&format!("dim x = {} * 2", i64::MAX)).await;
        assert!(err.contains("overflow"));
    }

    #[tokio::test]
    async fn test_string_concatenation() {
        let (interp, _) = run_script(r#"dim x = "hello" + " " + "world""#).await;
        assert!(
            matches!(interp.get_var("x").await, Some(Value::String(s)) if s == "hello world")
        );
    }

    #[tokio::test]
    async fn test_deeply_nested_expression() {
        let (interp, _) = run_script("dim x = ((((1 + 2) * 3) - 4) / 5)").await;
        // ((1+2)*3-4)/5 = (3*3-4)/5 = (9-4)/5 = 5/5 = 1
        assert!(matches!(interp.get_var("x").await, Some(Value::Int(1))));
    }

    #[tokio::test]
    async fn test_variable_shadowing_in_loop() {
        let (interp, _) = run_script(
            r#"
            dim i = 100
            for i = 1 to 3
                dim x = i
            next
        "#,
        )
        .await;
        // After loop, i should be the loop's final value or shadowed
        // depends on implementation - just verify it doesn't error
        assert!(interp.get_var("i").await.is_some());
    }
}

// ============================================================================
// String Operations Tests
// ============================================================================

mod strings {
    use super::*;

    #[tokio::test]
    async fn test_string_escape_newline() {
        let (interp, _) = run_script(r#"dim x = "line1\nline2""#).await;
        match interp.get_var("x").await {
            Some(Value::String(s)) => assert!(s.contains('\n')),
            _ => panic!("Expected string"),
        }
    }

    #[tokio::test]
    async fn test_string_escape_tab() {
        let (interp, _) = run_script(r#"dim x = "col1\tcol2""#).await;
        match interp.get_var("x").await {
            Some(Value::String(s)) => assert!(s.contains('\t')),
            _ => panic!("Expected string"),
        }
    }

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
}
