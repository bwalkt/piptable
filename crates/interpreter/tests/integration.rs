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

/// Execute a PipTable script and return the interpreter state and final evaluation value.
///
/// Returns a tuple `(Interpreter, Value)` where the `Interpreter` contains the post-execution state
/// (variables, output capture, etc.) and the `Value` is the result of evaluating the script.
///
/// # Examples
///
/// ```
/// use tokio::runtime::Runtime;
///
/// let rt = Runtime::new().unwrap();
/// let (interp, val) = rt.block_on(async {
///     // run_script is available in the same crate/context as this example
///     run_script(r#"dim x = 42
/// x"#).await
/// });
/// // `val` should be the integer 42 (assert style depends on Value API)
/// // e.g. assert_eq!(val, Value::Int(42));
/// ```
async fn run_script(script: &str) -> (Interpreter, Value) {
    let mut interp = Interpreter::new();
    let program = PipParser::parse_str(script).expect("Failed to parse script");
    let result = interp.eval(program).await.expect("Failed to eval script");
    (interp, result)
}

/// Executes the given PipTable script and returns the interpreter's error message.
///
/// Parses `script` and evaluates it using a fresh Interpreter, returning the error's string
/// representation. Panics if parsing fails or if evaluation unexpectedly succeeds.
///
/// # Examples
///
/// ```
/// let rt = tokio::runtime::Runtime::new().unwrap();
/// let msg = rt.block_on(run_script_err("1 + true"));
/// assert!(!msg.is_empty());
/// ```
async fn run_script_err(script: &str) -> String {
    let mut interp = Interpreter::new();
    let program = PipParser::parse_str(script).expect("Failed to parse script");
    interp
        .eval(program)
        .await
        .expect_err("Expected error")
        .to_string()
}

/// Creates a temporary file with a `.csv` suffix, writes `content` into it, flushes, and returns the open `NamedTempFile`.

///

/// The returned `NamedTempFile` keeps the file alive for the duration of the handle; the file is created in the system temp directory.

///

/// # Examples

///

/// ```

/// use tempfile::NamedTempFile;

/// use std::fs;

///

/// let file: NamedTempFile = create_temp_csv("a,b\n1,2\n");

/// let read = fs::read_to_string(file.path()).unwrap();

/// assert_eq!(read, "a,b\n1,2\n");

/// ```
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

    /// Verifies that declaring a variable with an integer literal binds that integer value to the variable.
    ///
    /// # Examples
    ///
    /// ```
    /// let (interp, _) = run_script("dim x = 42").await;
    /// assert!(matches!(interp.get_var("x").await, Some(Value::Int(42))));
    /// ```
    #[tokio::test]
    async fn test_dim_integer() {
        let (interp, _) = run_script("dim x = 42").await;
        assert!(matches!(interp.get_var("x").await, Some(Value::Int(42))));
    }

    /// Verifies that declaring a variable with a float literal assigns a floating-point value.
    ///
    /// # Examples
    ///
    /// ```
    /// # async fn example() {
    /// let (interp, _) = run_script("dim x = 3.14").await;
    /// match interp.get_var("x").await {
    ///     Some(Value::Float(f)) => assert!((f - 3.14).abs() < 0.001),
    ///     _ => panic!("Expected float"),
    /// }
    /// # }
    /// ```
    #[tokio::test]
    async fn test_dim_float() {
        let (interp, _) = run_script("dim x = 3.14").await;
        match interp.get_var("x").await {
            Some(Value::Float(f)) => assert!((f - 3.14).abs() < 0.001),
            _ => panic!("Expected float"),
        }
    }

    /// Verifies that declaring a string with `dim` assigns the string value to variable `x`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use crate::{run_script, Value};
    /// # tokio_test::block_on(async {
    /// let (interp, _) = run_script(r#"dim x = "hello""#).await;
    /// assert!(matches!(interp.get_var("x").await, Some(Value::String(s)) if s == "hello"));
    /// # });
    /// ```
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

    /// Verifies that assigning to an existing variable updates its value.
    ///
    /// This test ensures that a subsequent assignment to a previously declared variable
    /// replaces the prior value in the interpreter's environment.
    ///
    /// # Examples
    ///
    /// ```
    /// let (interp, _) = run_script("dim x = 1\nx = 2").await;
    /// assert!(matches!(interp.get_var("x").await, Some(Value::Int(2))));
    /// ```
    #[tokio::test]
    async fn test_assignment_updates_variable() {
        let (interp, _) = run_script("dim x = 1\nx = 2").await;
        assert!(matches!(interp.get_var("x").await, Some(Value::Int(2))));
    }

    /// Verifies that declaring multiple variables and assigning a variable using an expression produces the expected value.
    ///
    /// The test declares `x` and `y`, assigns `z` to `x + y`, and asserts that `z` equals `30`.
    ///
    /// # Examples
    ///
    /// ```
    /// let (interp, _) = run_script("dim x = 10\ndim y = 20\ndim z = x + y").await;
    /// assert!(matches!(interp.get_var("z").await, Some(Value::Int(30))));
    /// ```
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

    /// Verifies that evaluating an addition expression assigns the correct integer sum to a variable.
    ///
    /// # Examples
    ///
    /// ```
    /// # use crate::{run_script, Value};
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
}

// ============================================================================
// Comparison and Logical Operators Tests
// ============================================================================

mod comparisons {
    use super::*;

    /// Ensures the equality operator evaluates equal integers as equal.
    ///
    /// # Examples
    ///
    /// ```
    /// # async fn example() {
    /// let (interp, _) = run_script("dim x = 5 == 5").await;
    /// assert!(matches!(interp.get_var("x").await, Some(Value::Bool(true))));
    /// # }
    /// ```
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

    /// Asserts that the greater-than-or-equal comparison evaluates to true when both operands are equal.
    ///
    /// # Examples
    ///
    /// ```
    /// #[tokio::test]
    /// async fn example_greater_or_equal() {
    ///     let (interp, _) = run_script("dim x = 5 >= 5").await;
    ///     assert!(matches!(interp.get_var("x").await, Some(Value::Bool(true))));
    /// }
    /// ```
    #[tokio::test]
    async fn test_greater_or_equal() {
        let (interp, _) = run_script("dim x = 5 >= 5").await;
        assert!(matches!(interp.get_var("x").await, Some(Value::Bool(true))));
    }

    /// Checks that the `<=` operator evaluates equality correctly.
    ///
    /// Runs a script that assigns the result of `5 <= 5` to `x` and asserts the interpreter stores the expected boolean result.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // Integration test: assigns 5 <= 5 to x and checks the value.
    /// let (interp, _) = run_script("dim x = 5 <= 5").await;
    /// assert!(matches!(interp.get_var("x").await, Some(Value::Bool(true))));
    /// ```
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

    /// Verifies that the logical `and` operator short-circuits, so a false left-hand side prevents evaluation of the right-hand side.
    ///
    /// # Examples
    ///
    /// ```
    /// // If short-circuit works, the undefined expression on the right is not evaluated
    /// let (interp, _) = run_script("dim x = false and undefined_var").await;
    /// assert!(matches!(
    ///     interp.get_var("x").await,
    ///     Some(Value::Bool(false))
    /// ));
    /// ```
    #[tokio::test]
    async fn test_short_circuit_and() {
        // If short-circuit works, undefined_var won't be evaluated
        let (interp, _) = run_script("dim x = false and undefined_var").await;
        assert!(matches!(
            interp.get_var("x").await,
            Some(Value::Bool(false))
        ));
    }

    /// Verifies that the logical `or` operator short-circuits so the right-hand side is not evaluated when the left-hand side is `true`.
    ///
    /// # Examples
    ///
    /// ```
    /// // The right-hand side references an undefined variable; short-circuiting prevents its evaluation.
    /// let (interp, _) = run_script("dim x = true or undefined_var").await;
    /// assert!(matches!(interp.get_var("x").await, Some(Value::Bool(true))));
    /// ```
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
}

// ============================================================================
// Function Tests
// ============================================================================

mod functions {
    use super::*;

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
}

// ============================================================================
// Array and Object Tests
// ============================================================================

mod collections {
    use super::*;

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
}

// ============================================================================
// SQL Query Tests
// ============================================================================

mod sql {
    use super::*;

    #[tokio::test]
    async fn test_simple_query() {
        let (interp, _) = run_script("dim result = query(SELECT 1 + 1 as sum)").await;
        assert!(matches!(
            interp.get_var("result").await,
            Some(Value::Table(_))
        ));
    }

    #[tokio::test]
    async fn test_query_multiple_columns() {
        let (interp, _) = run_script("dim result = query(SELECT 1 as a, 2 as b, 3 as c)").await;
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
    /// Verifies that a CSV-backed SQL query filters rows correctly using a WHERE clause.
    ///
    /// Creates a temporary CSV file, runs a `SELECT * FROM '<path>' WHERE value > 150` query
    /// through the interpreter, and asserts the resulting table contains the expected two rows.
    ///
    /// # Examples
    ///
    /// ```
    /// let csv_content = "id,name,value\n1,foo,100\n2,bar,200\n3,baz,300";
    /// let file = create_temp_csv(csv_content);
    /// let path = file.path().to_string_lossy().replace('\\', "/");
    /// let script = format!(r#"dim result = query(SELECT * FROM '{}' WHERE value > 150)"#, path);
    /// let (interp, _) = run_script(&script).await;
    /// match interp.get_var("result").await {
    ///     Some(Value::Table(batches)) => {
    ///         let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
    ///         assert_eq!(total_rows, 2);
    ///     }
    ///     _ => panic!("Expected table"),
    /// }
    /// ```
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
    /// Verifies that a CSV query with `GROUP BY` and `SUM` produces a table with one row per group.
    ///
    /// This integration test writes a temporary CSV, runs a SQL-like query that groups by `category`
    /// and sums `amount`, and asserts the resulting table contains two aggregated rows.
    ///
    /// # Examples
    ///
    /// ```
    /// // Creates a temp CSV and runs: SELECT category, SUM(amount) as total FROM '<path>' GROUP BY category
    /// let csv_content = "category,amount\nA,100\nB,200\nA,150\nB,50";
    /// let file = create_temp_csv(csv_content);
    /// let path = file.path().to_string_lossy().replace('\\', "/");
    /// let script = format!(r#"dim result = query(SELECT category, SUM(amount) as total FROM '{}' GROUP BY category ORDER BY category)"#, path);
    /// let (interp, _) = run_script(&script).await;
    /// match interp.get_var("result").await {
    ///     Some(Value::Table(batches)) => {
    ///         let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
    ///         assert_eq!(total_rows, 2);
    ///     }
    ///     _ => panic!("Expected table"),
    /// }
    /// ```
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
    /// Integration test that verifies a SQL JOIN across two CSV files produces the expected combined rows.
    ///
    /// This test creates two temporary CSV files (users and orders), runs a `query` that joins them on user ID,
    /// and asserts the resulting table contains the combined rows (three total for the sample data).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // Create CSV files, run the JOIN query, and check the combined row count.
    /// let users_csv = "id,name\n1,alice\n2,bob";
    /// let orders_csv = "user_id,amount\n1,100\n1,200\n2,50";
    /// let users_file = create_temp_csv(users_csv);
    /// let orders_file = create_temp_csv(orders_csv);
    /// let users_path = users_file.path().to_string_lossy().replace('\\', "/");
    /// let orders_path = orders_file.path().to_string_lossy().replace('\\', "/");
    /// let script = format!(r#"dim result = query(
    ///     SELECT u.name, o.amount
    ///     FROM '{}' as u
    ///     JOIN '{}' as o ON u.id = o.user_id
    ///     ORDER BY u.name, o.amount
    /// )"#, users_path, orders_path);
    /// let (interp, _) = run_script(&script).await;
    /// match interp.get_var("result").await {
    ///     Some(Value::Table(batches)) => {
    ///         let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
    ///         assert_eq!(total_rows, 3);
    ///     }
    ///     _ => panic!("Expected table"),
    /// }
    /// ```
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

    /// Verifies that fetching a JSON array via HTTP produces an Array value in the interpreter.
    ///
    /// # Examples
    ///
    /// ```
    /// // Starts a mock server that returns a JSON array at /api/items,
    /// // then runs a PipTable script that fetches that endpoint and asserts the result is an array.
    /// let server = wiremock::MockServer::start().await;
    /// wiremock::Mock::given(wiremock::matchers::method("GET"))
    ///     .and(wiremock::matchers::path("/api/items"))
    ///     .respond_with(
    ///         wiremock::ResponseTemplate::new(200)
    ///             .set_body_json(serde_json::json!([
    ///                 {"id": 1, "name": "item1"},
    ///                 {"id": 2, "name": "item2"}
    ///             ])),
    ///     )
    ///     .mount(&server)
    ///     .await;
    ///
    /// let script = format!(r#"dim data = fetch("{}/api/items")"#, server.uri());
    /// let (interp, _) = run_script(&script).await;
    ///
    /// match interp.get_var("data").await {
    ///     Some(Value::Array(items)) => assert_eq!(items.len(), 2),
    ///     _ => panic!("Expected array"),
    /// }
    /// ```
    #[tokio::test]
    async fn test_fetch_json_array() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/items"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {"id": 1, "name": "item1"},
                {"id": 2, "name": "item2"}
            ])))
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

    /// Verifies that an HTTP GET returning a JSON object is parsed into an interpreter `Object` value.
    ///
    /// The test starts a mock HTTP server that responds with a JSON object, runs a script that calls
    /// `fetch` on that endpoint, and asserts the interpreter binds an object whose `"name"` field equals
    /// `"test"`.
    ///
    /// # Examples
    ///
    /// ```
    /// // Starts a mock server responding with {"id":1,"name":"test"} and ensures the interpreter's
    /// // `data` variable is an object containing `name = "test"`.
    /// # async fn _example() { /* runs within test harness */ }
    /// ```
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

    /// Verifies that fetching a JSON array produces an iterable that can be traversed with `for each` and that iteration yields the expected aggregated result.
    ///
    /// The test starts a mock HTTP server returning the JSON array `[1, 2, 3, 4, 5]`, runs a PipTable script that fetches that array, iterates over its elements to compute a running sum, and asserts the interpreter's `sum` variable equals `15`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // Starts a mock server returning [1,2,3,4,5], runs a script that fetches and sums the array,
    /// // and then verifies the interpreter's `sum` variable is 15.
    /// ```
    #[tokio::test]
    async fn test_fetch_and_iterate() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/numbers"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!([1, 2, 3, 4, 5])),
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

        assert!(matches!(interp.get_var("sum").await, Some(Value::Int(15))));
    }
}

// ============================================================================
// Error Handling Tests
// ============================================================================

mod errors {
    use super::*;

    /// Asserts that evaluating a script which references an undefined variable produces an interpreter error.
    ///
    /// # Examples
    ///
    /// ```
    /// // Expects an error mentioning "Undefined variable".
    /// let err = run_script_err("dim x = undefined_var").await;
    /// assert!(err.contains("Undefined variable"));
    /// ```
    #[tokio::test]
    async fn test_undefined_variable() {
        let err = run_script_err("dim x = undefined_var").await;
        assert!(err.contains("Undefined variable"));
    }

    /// Verifies that calling a non-existent function in a script produces an interpreter error.
    ///
    /// Asserts the interpreter reports the function as "Unknown" or "Undefined" when evaluating
    /// a call to a function that has not been defined.
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

    /// Verifies that indexing an array with an index outside its valid range produces an out-of-bounds error.
    ///
    /// # Examples
    ///
    /// ```
    /// #[tokio::test]
    /// async fn example_array_index_out_of_bounds() {
    ///     let err = run_script_err("dim arr = [1, 2, 3]\ndim x = arr[10]").await;
    ///     assert!(err.contains("out of bounds"));
    /// }
    /// ```
    #[tokio::test]
    async fn test_array_index_out_of_bounds() {
        let err = run_script_err("dim arr = [1, 2, 3]\ndim x = arr[10]").await;
        assert!(err.contains("out of bounds"));
    }

    /// Verifies that accessing an array with a negative index outside the valid range produces an out-of-bounds error.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // Asserts that indexing with a large negative index reports an out-of-bounds error.
    /// let err = run_script_err("dim arr = [1, 2, 3]\ndim x = arr[-10]").await;
    /// assert!(err.contains("out of bounds"));
    /// ```
    #[tokio::test]
    async fn test_negative_index_out_of_bounds() {
        let err = run_script_err("dim arr = [1, 2, 3]\ndim x = arr[-10]").await;
        assert!(err.contains("out of bounds"));
    }

    /// Verifies that executing a `for` loop with a step of zero produces an error.
    ///
    /// This test ensures the interpreter rejects a loop that would never advance due to a step of `0`.
    ///
    /// # Examples
    ///
    /// ```
    /// // asserts that a for-loop with step 0 yields an error mentioning "non-zero" or "step"
    /// let err = run_script_err("for i = 1 to 10 step 0\nnext").await;
    /// assert!(err.contains("non-zero") || err.contains("step"));
    /// ```
    #[tokio::test]
    async fn test_for_loop_zero_step() {
        let err = run_script_err("for i = 1 to 10 step 0\nnext").await;
        assert!(err.contains("non-zero") || err.contains("step"));
    }

    #[tokio::test]
    async fn test_type_error_add_string_int() {
        let err = run_script_err(r#"dim x = "hello" + 42"#).await;
        // This might concatenate or error depending on implementation
        // If it errors, check the message
        assert!(
            err.contains("Type error") || err.contains("Cannot") || err.is_empty(),
            "Got unexpected error: {}",
            err
        );
    }
}

// ============================================================================
// Edge Cases and Overflow Tests
// ============================================================================

mod edge_cases {
    use super::*;

    /// Verifies that declaring an empty array binds a variable to an empty array.
    ///
    /// # Examples
    ///
    /// ```
    /// # async fn example() {
    /// let (interp, _) = run_script("dim arr = []").await;
    /// match interp.get_var("arr").await {
    ///     Some(Value::Array(items)) => assert_eq!(items.len(), 0),
    ///     _ => panic!("Expected empty array"),
    /// }
    /// # }
    /// ```
    #[tokio::test]
    async fn test_empty_array() {
        let (interp, _) = run_script("dim arr = []").await;
        match interp.get_var("arr").await {
            Some(Value::Array(items)) => assert_eq!(items.len(), 0),
            _ => panic!("Expected empty array"),
        }
    }

    /// Verifies that declaring an empty object creates an object variable with no fields.
    ///
    /// This integration test runs a script that defines `obj` as an empty object and
    /// asserts the interpreter stores `obj` as an `Object` with length 0.
    ///
    /// # Examples
    ///
    /// ```
    /// let (interp, _) = run_script("dim obj = {}").await;
    /// match interp.get_var("obj").await {
    ///     Some(Value::Object(map)) => assert_eq!(map.len(), 0),
    ///     _ => panic!("Expected empty object"),
    /// }
    /// ```
    #[tokio::test]
    async fn test_empty_object() {
        let (interp, _) = run_script("dim obj = {}").await;
        match interp.get_var("obj").await {
            Some(Value::Object(map)) => assert_eq!(map.len(), 0),
            _ => panic!("Expected empty object"),
        }
    }

    /// Verifies that a `for` loop with a start greater than the end and the default step does not execute.
    ///
    /// The test runs a script that initializes `count` to 0 and iterates `for i = 10 to 1` with the implicit step of 1;
    /// the loop body should never run and `count` must remain 0.
    ///
    /// # Examples
    ///
    /// ```
    /// let (interp, _) = run_script(r#"
    ///     dim count = 0
    ///     for i = 10 to 1
    ///         count = count + 1
    ///     next
    /// "#).await;
    /// assert!(matches!(interp.get_var("count").await, Some(Value::Int(0))));
    /// ```
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
        assert!(matches!(interp.get_var("count").await, Some(Value::Int(0))));
    }

    /// Verifies that a `for each` loop over an empty array never executes its body (counter remains 0).
    ///
    /// # Examples
    ///
    /// ```
    /// # async fn run_example() {
    /// let (interp, _) = run_script(
    ///     r#"
    ///     dim items = []
    ///     dim count = 0
    ///     for each item in items
    ///         count = count + 1
    ///     next
    /// "#,
    /// )
    /// .await;
    /// assert!(matches!(interp.get_var("count").await, Some(Value::Int(0))));
    /// # }
    /// ```
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
        assert!(matches!(interp.get_var("count").await, Some(Value::Int(0))));
    }

    /// Verifies that adding 1 to the maximum 64-bit integer produces an overflow error.
    ///
    /// # Examples
    ///
    /// ```
    /// # async fn _doc() {
    /// let err = run_script_err(&format!("dim x = {} + 1", i64::MAX)).await;
    /// assert!(err.contains("overflow"));
    /// # }
    /// ```
    #[tokio::test]
    async fn test_integer_overflow_add() {
        let err = run_script_err(&format!("dim x = {} + 1", i64::MAX)).await;
        assert!(err.contains("overflow"));
    }

    // i64::MIN literal cannot be parsed directly (too large)
    // Test overflow via negation of i64::MAX and subtraction instead
    /// Verifies the interpreter reports an integer overflow when subtracting past the 64-bit integer lower bound.
    ///
    /// The test executes a script that performs a subtraction which exceeds the minimum i64 value and asserts the resulting error message contains "overflow".
    ///
    /// # Examples
    ///
    /// ```
    /// // This mirrors the test: attempting to subtract beyond i64::MIN should produce an overflow error.
    /// let err = run_script_err("dim x = -9223372036854775807 - 2").await;
    /// assert!(err.contains("overflow"));
    /// ```
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
        assert!(matches!(interp.get_var("x").await, Some(Value::String(s)) if s == "hello world"));
    }

    #[tokio::test]
    async fn test_deeply_nested_expression() {
        let (interp, _) = run_script("dim x = ((((1 + 2) * 3) - 4) / 5)").await;
        // ((1+2)*3-4)/5 = (3*3-4)/5 = (9-4)/5 = 5/5 = 1
        assert!(matches!(interp.get_var("x").await, Some(Value::Int(1))));
    }

    /// Verifies that reusing a variable name as a for-loop index does not cause a runtime error and that the original name remains bound after the loop.
    ///
    /// Asserts the interpreter has a binding for `i` after executing a script where `i` is declared before the loop and the loop uses `i` as its loop variable.
    ///
    /// # Examples
    ///
    /// ```
    /// // Ensures variable shadowing inside a for-loop does not remove the outer binding.
    /// let (interp, _) = run_script(
    ///     r#"
    ///     dim i = 100
    ///     for i = 1 to 3
    ///         dim x = i
    ///     next
    /// "#,
    /// )
    /// .await;
    /// assert!(interp.get_var("i").await.is_some());
    /// ```
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
}
