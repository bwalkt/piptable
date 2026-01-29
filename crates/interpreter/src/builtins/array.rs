//! Array manipulation functions (FILTER, SORT, UNIQUE, etc.)

use crate::Interpreter;
use piptable_core::{PipError, PipResult, Value};

/// Handle array function calls
pub async fn call_array_builtin(
    interpreter: &Interpreter,
    name: &str,
    args: Vec<Value>,
    line: usize,
) -> Option<PipResult<Value>> {
    match name {
        "filter" => Some(filter(interpreter, args, line).await),
        _ => None,
    }
}

/// FILTER(array, include, [if_empty])
/// Filters an array based on boolean criteria.
///
/// # Arguments
/// - array: The array or range to filter
/// - include: An array of boolean values or a condition that evaluates to boolean array
/// - if_empty: Optional value to return if all values are filtered out
///
/// # Example
/// ```
/// data = [[1, "A"], [2, "B"], [3, "C"], [4, "D"]]
/// criteria = [true, false, true, false]
/// result = filter(data, criteria)  // Returns [[1, "A"], [3, "C"]]
/// ```
async fn filter(
    _interpreter: &Interpreter,
    args: Vec<Value>,
    line: usize,
) -> PipResult<Value> {
    if args.len() < 2 || args.len() > 3 {
        return Err(PipError::runtime(
            line,
            "FILTER requires 2 or 3 arguments: FILTER(array, include, [if_empty])",
        ));
    }

    let array = match &args[0] {
        Value::Array(arr) => arr,
        _ => {
            return Err(PipError::runtime(
                line,
                "FILTER: first argument must be an array",
            ))
        }
    };

    // Handle the include criteria
    let include = match &args[1] {
        Value::Array(arr) => arr.clone(),
        Value::Bool(b) => {
            // Single boolean applies to all elements
            vec![Value::Bool(*b); array.len()]
        }
        _ => {
            // Try to evaluate as a condition for each element
            // For now, we'll require an array of booleans
            return Err(PipError::runtime(
                line,
                "FILTER: include must be an array of boolean values",
            ));
        }
    };

    // Check that include array has same length as data array
    if include.len() != array.len() {
        return Err(PipError::runtime(
            line,
            format!(
                "FILTER: include array length ({}) must match data array length ({})",
                include.len(),
                array.len()
            ),
        ));
    }

    // Filter the array
    let mut result = Vec::new();
    for (i, item) in array.iter().enumerate() {
        let include_item = match &include[i] {
            Value::Bool(b) => *b,
            Value::Int(n) => *n != 0,  // Non-zero integers are truthy
            Value::Float(f) => *f != 0.0 && !f.is_nan(),  // Non-zero, non-NaN floats are truthy
            Value::Null => false,
            Value::String(s) => !s.is_empty(),  // Non-empty strings are truthy
            _ => {
                return Err(PipError::runtime(
                    line,
                    format!("FILTER: include array element {} must be convertible to boolean", i + 1),
                ))
            }
        };

        if include_item {
            result.push(item.clone());
        }
    }

    // Handle empty result
    if result.is_empty() {
        if args.len() == 3 {
            // Return the if_empty value
            Ok(args[2].clone())
        } else {
            // Return #CALC! error (Excel's way of indicating empty filter result)
            Ok(Value::String("#CALC!".to_string()))
        }
    } else {
        Ok(Value::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_interpreter() -> Interpreter {
        Interpreter::new()
    }

    #[tokio::test]
    async fn test_filter_basic() {
        let interp = create_interpreter().await;
        let array = Value::Array(vec![
            Value::Int(1),
            Value::Int(2),
            Value::Int(3),
            Value::Int(4),
        ]);
        let include = Value::Array(vec![
            Value::Bool(true),
            Value::Bool(false),
            Value::Bool(true),
            Value::Bool(false),
        ]);

        let result = filter(&interp, vec![array, include], 0).await.unwrap();
        match result {
            Value::Array(arr) => {
                assert_eq!(arr.len(), 2);
                assert!(matches!(arr[0], Value::Int(1)));
                assert!(matches!(arr[1], Value::Int(3)));
            }
            _ => panic!("Expected array result"),
        }
    }

    #[tokio::test]
    async fn test_filter_2d_array() {
        let interp = create_interpreter().await;
        let array = Value::Array(vec![
            Value::Array(vec![Value::Int(1), Value::String("A".to_string())]),
            Value::Array(vec![Value::Int(2), Value::String("B".to_string())]),
            Value::Array(vec![Value::Int(3), Value::String("C".to_string())]),
        ]);
        let include = Value::Array(vec![
            Value::Bool(true),
            Value::Bool(false),
            Value::Bool(true),
        ]);

        let result = filter(&interp, vec![array, include], 0).await.unwrap();
        match result {
            Value::Array(arr) => {
                assert_eq!(arr.len(), 2);
                match &arr[0] {
                    Value::Array(row) => {
                        assert!(matches!(row[0], Value::Int(1)));
                        match &row[1] {
                            Value::String(s) => assert_eq!(s, "A"),
                            _ => panic!("Expected string"),
                        }
                    }
                    _ => panic!("Expected array row"),
                }
            }
            _ => panic!("Expected array result"),
        }
    }

    #[tokio::test]
    async fn test_filter_empty_result() {
        let interp = create_interpreter().await;
        let array = Value::Array(vec![Value::Int(1), Value::Int(2)]);
        let include = Value::Array(vec![Value::Bool(false), Value::Bool(false)]);

        let result = filter(&interp, vec![array, include], 0).await.unwrap();
        match result {
            Value::String(s) => assert_eq!(s, "#CALC!"),
            _ => panic!("Expected #CALC! string"),
        }
    }

    #[tokio::test]
    async fn test_filter_with_if_empty() {
        let interp = create_interpreter().await;
        let array = Value::Array(vec![Value::Int(1), Value::Int(2)]);
        let include = Value::Array(vec![Value::Bool(false), Value::Bool(false)]);
        let if_empty = Value::String("No results".to_string());

        let result = filter(&interp, vec![array, include, if_empty], 0)
            .await
            .unwrap();
        match result {
            Value::String(s) => assert_eq!(s, "No results"),
            _ => panic!("Expected if_empty string"),
        }
    }

    #[tokio::test]
    async fn test_filter_truthy_values() {
        let interp = create_interpreter().await;
        let array = Value::Array(vec![
            Value::String("A".to_string()),
            Value::String("B".to_string()),
            Value::String("C".to_string()),
            Value::String("D".to_string()),
        ]);
        let include = Value::Array(vec![
            Value::Int(1),      // truthy
            Value::Int(0),      // falsy
            Value::Float(2.5),  // truthy
            Value::Null,        // falsy
        ]);

        let result = filter(&interp, vec![array, include], 0).await.unwrap();
        match result {
            Value::Array(arr) => {
                assert_eq!(arr.len(), 2);
                match &arr[0] {
                    Value::String(s) => assert_eq!(s, "A"),
                    _ => panic!("Expected string A"),
                }
                match &arr[1] {
                    Value::String(s) => assert_eq!(s, "C"),
                    _ => panic!("Expected string C"),
                }
            }
            _ => panic!("Expected array result"),
        }
    }

    #[tokio::test]
    async fn test_filter_length_mismatch_error() {
        let interp = create_interpreter().await;
        let array = Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
        let include = Value::Array(vec![Value::Bool(true), Value::Bool(false)]);

        let result = filter(&interp, vec![array, include], 0).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must match data array length"));
    }
}
