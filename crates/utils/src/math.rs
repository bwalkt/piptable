//! Shared math functions used by formula and DSL helpers.

use piptable_primitives::{ErrorValue, Value};

fn walk_values(values: &[Value], f: &mut dyn FnMut(&Value)) {
    for value in values {
        match value {
            Value::Array(items) => walk_values(items, f),
            _ => f(value),
        }
    }
}

fn to_number(value: &Value) -> Option<f64> {
    match value {
        Value::Int(n) => Some(*n as f64),
        Value::Float(f) => Some(*f),
        _ => None,
    }
}

/// Sum function - adds all numeric values.
pub fn sum(values: &[Value]) -> Value {
    let mut total = 0.0;
    walk_values(values, &mut |value| {
        if let Some(num) = to_number(value) {
            total += num;
        }
    });
    Value::Float(total)
}

/// Average function - calculates mean of numeric values.
pub fn average(values: &[Value]) -> Value {
    let mut total = 0.0;
    let mut count = 0;

    walk_values(values, &mut |value| {
        if let Some(num) = to_number(value) {
            total += num;
            count += 1;
        }
    });

    if count == 0 {
        Value::Error(ErrorValue::Div0)
    } else {
        Value::Float(total / count as f64)
    }
}

/// Count function - counts numeric values.
pub fn count(values: &[Value]) -> Value {
    let mut count = 0usize;
    walk_values(values, &mut |value| {
        if to_number(value).is_some() {
            count += 1;
        }
    });
    Value::Int(count as i64)
}

/// Max function - finds maximum value.
pub fn max(values: &[Value]) -> Value {
    let mut max_val: Option<f64> = None;

    walk_values(values, &mut |value| {
        let Some(num) = to_number(value) else {
            return;
        };
        max_val = Some(match max_val {
            None => num,
            Some(m) => m.max(num),
        });
    });

    match max_val {
        Some(v) => Value::Float(v),
        None => Value::Error(ErrorValue::Value),
    }
}

/// Min function - finds minimum value.
pub fn min(values: &[Value]) -> Value {
    let mut min_val: Option<f64> = None;

    walk_values(values, &mut |value| {
        let Some(num) = to_number(value) else {
            return;
        };
        min_val = Some(match min_val {
            None => num,
            Some(m) => m.min(num),
        });
    });

    match min_val {
        Some(v) => Value::Float(v),
        None => Value::Error(ErrorValue::Value),
    }
}

/// ABS function - returns absolute value.
pub fn abs(values: &[Value]) -> Value {
    let value = values.first().unwrap_or(&Value::Empty);
    match to_number(value) {
        Some(n) => Value::Float(n.abs()),
        None => Value::Error(ErrorValue::Value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sum_basic() {
        let values = vec![Value::Int(1), Value::Int(2), Value::Int(3)];
        let result = sum(&values);
        assert!(matches!(result, Value::Float(f) if (f - 6.0).abs() < 1e-9));
    }

    #[test]
    fn test_sum_mixed_types() {
        let values = vec![Value::Int(1), Value::Float(2.5), Value::Int(3)];
        let result = sum(&values);
        assert!(matches!(result, Value::Float(f) if (f - 6.5).abs() < 1e-9));
    }

    #[test]
    fn test_sum_with_non_numeric() {
        let values = vec![
            Value::Int(1),
            Value::String("hello".to_string()),
            Value::Int(2),
        ];
        let result = sum(&values);
        assert!(matches!(result, Value::Float(f) if (f - 3.0).abs() < 1e-9));
    }

    #[test]
    fn test_sum_nested_arrays() {
        let values = vec![
            Value::Array(vec![Value::Int(1), Value::Int(2)]),
            Value::Int(3),
        ];
        let result = sum(&values);
        assert!(matches!(result, Value::Float(f) if (f - 6.0).abs() < 1e-9));
    }

    #[test]
    fn test_sum_empty() {
        let values = vec![];
        let result = sum(&values);
        assert!(matches!(result, Value::Float(f) if f.abs() < 1e-9));
    }

    #[test]
    fn test_sum_only_non_numeric() {
        let values = vec![Value::String("a".to_string()), Value::Bool(true)];
        let result = sum(&values);
        assert!(matches!(result, Value::Float(f) if f.abs() < 1e-9));
    }

    #[test]
    fn test_average_basic() {
        let values = vec![Value::Int(10), Value::Int(20), Value::Int(30)];
        let result = average(&values);
        assert!(matches!(result, Value::Float(f) if (f - 20.0).abs() < 1e-9));
    }

    #[test]
    fn test_average_single_value() {
        let values = vec![Value::Float(42.5)];
        let result = average(&values);
        assert!(matches!(result, Value::Float(f) if (f - 42.5).abs() < 1e-9));
    }

    #[test]
    fn test_average_empty() {
        let values = vec![];
        let result = average(&values);
        assert!(matches!(result, Value::Error(ErrorValue::Div0)));
    }

    #[test]
    fn test_average_no_numeric_values() {
        let values = vec![Value::String("a".to_string()), Value::Empty];
        let result = average(&values);
        assert!(matches!(result, Value::Error(ErrorValue::Div0)));
    }

    #[test]
    fn test_average_mixed_types() {
        let values = vec![
            Value::Int(10),
            Value::Float(20.0),
            Value::String("ignored".to_string()),
        ];
        let result = average(&values);
        assert!(matches!(result, Value::Float(f) if (f - 15.0).abs() < 1e-9));
    }

    #[test]
    fn test_count_basic() {
        let values = vec![Value::Int(1), Value::Float(2.0), Value::Int(3)];
        let result = count(&values);
        assert_eq!(result, Value::Int(3));
    }

    #[test]
    fn test_count_with_non_numeric() {
        let values = vec![
            Value::Int(1),
            Value::String("hello".to_string()),
            Value::Float(2.0),
            Value::Empty,
        ];
        let result = count(&values);
        assert_eq!(result, Value::Int(2));
    }

    #[test]
    fn test_count_empty() {
        let values = vec![];
        let result = count(&values);
        assert_eq!(result, Value::Int(0));
    }

    #[test]
    fn test_count_nested_arrays() {
        let values = vec![
            Value::Array(vec![Value::Int(1), Value::String("x".to_string())]),
            Value::Float(2.0),
        ];
        let result = count(&values);
        assert_eq!(result, Value::Int(2));
    }

    #[test]
    fn test_max_basic() {
        let values = vec![Value::Int(3), Value::Int(1), Value::Int(5), Value::Int(2)];
        let result = max(&values);
        assert!(matches!(result, Value::Float(f) if (f - 5.0).abs() < 1e-9));
    }

    #[test]
    fn test_max_negative_numbers() {
        let values = vec![Value::Int(-10), Value::Int(-5), Value::Int(-20)];
        let result = max(&values);
        assert!(matches!(result, Value::Float(f) if (f - (-5.0)).abs() < 1e-9));
    }

    #[test]
    fn test_max_mixed_int_float() {
        let values = vec![Value::Int(3), Value::Float(3.5), Value::Int(2)];
        let result = max(&values);
        assert!(matches!(result, Value::Float(f) if (f - 3.5).abs() < 1e-9));
    }

    #[test]
    fn test_max_empty() {
        let values = vec![];
        let result = max(&values);
        assert!(matches!(result, Value::Error(ErrorValue::Value)));
    }

    #[test]
    fn test_max_no_numeric() {
        let values = vec![Value::String("a".to_string()), Value::Empty];
        let result = max(&values);
        assert!(matches!(result, Value::Error(ErrorValue::Value)));
    }

    #[test]
    fn test_max_with_non_numeric_ignored() {
        let values = vec![Value::Int(5), Value::String("ignored".to_string()), Value::Int(3)];
        let result = max(&values);
        assert!(matches!(result, Value::Float(f) if (f - 5.0).abs() < 1e-9));
    }

    #[test]
    fn test_min_basic() {
        let values = vec![Value::Int(3), Value::Int(1), Value::Int(5), Value::Int(2)];
        let result = min(&values);
        assert!(matches!(result, Value::Float(f) if (f - 1.0).abs() < 1e-9));
    }

    #[test]
    fn test_min_negative_numbers() {
        let values = vec![Value::Int(-10), Value::Int(-5), Value::Int(-20)];
        let result = min(&values);
        assert!(matches!(result, Value::Float(f) if (f - (-20.0)).abs() < 1e-9));
    }

    #[test]
    fn test_min_mixed_int_float() {
        let values = vec![Value::Int(3), Value::Float(2.5), Value::Int(5)];
        let result = min(&values);
        assert!(matches!(result, Value::Float(f) if (f - 2.5).abs() < 1e-9));
    }

    #[test]
    fn test_min_empty() {
        let values = vec![];
        let result = min(&values);
        assert!(matches!(result, Value::Error(ErrorValue::Value)));
    }

    #[test]
    fn test_min_no_numeric() {
        let values = vec![Value::String("a".to_string()), Value::Empty];
        let result = min(&values);
        assert!(matches!(result, Value::Error(ErrorValue::Value)));
    }

    #[test]
    fn test_min_single_value() {
        let values = vec![Value::Float(42.0)];
        let result = min(&values);
        assert!(matches!(result, Value::Float(f) if (f - 42.0).abs() < 1e-9));
    }

    #[test]
    fn test_abs_positive_int() {
        let values = vec![Value::Int(42)];
        let result = abs(&values);
        assert!(matches!(result, Value::Float(f) if (f - 42.0).abs() < 1e-9));
    }

    #[test]
    fn test_abs_negative_int() {
        let values = vec![Value::Int(-42)];
        let result = abs(&values);
        assert!(matches!(result, Value::Float(f) if (f - 42.0).abs() < 1e-9));
    }

    #[test]
    fn test_abs_positive_float() {
        let values = vec![Value::Float(3.14)];
        let result = abs(&values);
        assert!(matches!(result, Value::Float(f) if (f - 3.14).abs() < 1e-9));
    }

    #[test]
    fn test_abs_negative_float() {
        let values = vec![Value::Float(-3.14)];
        let result = abs(&values);
        assert!(matches!(result, Value::Float(f) if (f - 3.14).abs() < 1e-9));
    }

    #[test]
    fn test_abs_zero() {
        let values = vec![Value::Int(0)];
        let result = abs(&values);
        assert!(matches!(result, Value::Float(f) if f.abs() < 1e-9));
    }

    #[test]
    fn test_abs_empty() {
        let values = vec![];
        let result = abs(&values);
        assert!(matches!(result, Value::Error(ErrorValue::Value)));
    }

    #[test]
    fn test_abs_non_numeric() {
        let values = vec![Value::String("hello".to_string())];
        let result = abs(&values);
        assert!(matches!(result, Value::Error(ErrorValue::Value)));
    }

    #[test]
    fn test_abs_error_value() {
        let values = vec![Value::Error(ErrorValue::Div0)];
        let result = abs(&values);
        assert!(matches!(result, Value::Error(ErrorValue::Value)));
    }

    #[test]
    fn test_walk_values_deeply_nested() {
        let values = vec![
            Value::Array(vec![
                Value::Int(1),
                Value::Array(vec![Value::Int(2), Value::Int(3)]),
            ]),
            Value::Int(4),
        ];
        let result = sum(&values);
        assert!(matches!(result, Value::Float(f) if (f - 10.0).abs() < 1e-9));
    }

    #[test]
    fn test_max_single_value() {
        let values = vec![Value::Int(42)];
        let result = max(&values);
        assert!(matches!(result, Value::Float(f) if (f - 42.0).abs() < 1e-9));
    }

    #[test]
    fn test_average_nested_arrays() {
        let values = vec![
            Value::Array(vec![Value::Int(10), Value::Int(20)]),
            Value::Int(30),
        ];
        let result = average(&values);
        assert!(matches!(result, Value::Float(f) if (f - 20.0).abs() < 1e-9));
    }

    #[test]
    fn test_sum_with_infinity() {
        let values = vec![Value::Float(f64::INFINITY), Value::Int(1)];
        let result = sum(&values);
        assert!(matches!(result, Value::Float(f) if f.is_infinite() && f.is_sign_positive()));
    }

    #[test]
    fn test_max_with_zero() {
        let values = vec![Value::Int(0), Value::Int(-5), Value::Int(3)];
        let result = max(&values);
        assert!(matches!(result, Value::Float(f) if (f - 3.0).abs() < 1e-9));
    }

    #[test]
    fn test_min_with_zero() {
        let values = vec![Value::Int(0), Value::Int(5), Value::Int(3)];
        let result = min(&values);
        assert!(matches!(result, Value::Float(f) if f.abs() < 1e-9));
    }
}
