//! Shared math functions used by formula and DSL helpers.

use piptable_primitives::{ErrorValue, Value};

/// Recursively traverses a slice of `Value`, invoking the provided callback for every non-array element.
///
/// The callback is called for each `Value` that is not `Value::Array`; nested arrays are visited depth-first.
///
/// # Examples
///
/// ```
/// let mut seen = Vec::new();
/// walk_values(
///     &[
///         Value::Int(1),
///         Value::Array(vec![Value::Int(2), Value::Array(vec![Value::Float(3.0)])]),
///     ],
///     &mut |v| seen.push(v.clone()),
/// );
/// assert_eq!(seen.len(), 3);
/// ```
fn walk_values(values: &[Value], f: &mut dyn FnMut(&Value)) {
    for value in values {
        match value {
            Value::Array(items) => walk_values(items, f),
            _ => f(value),
        }
    }
}

/// Convert a numeric `Value` into a floating-point number.
///
/// Returns `Some(f)` with the value as an `f64` when the input is `Value::Int` or `Value::Float`,
/// and `None` for all other variants.
///
/// # Examples
///
/// ```
/// let v = Value::Int(3);
/// assert_eq!(to_number(&v), Some(3.0));
///
/// let v = Value::Float(2.5);
/// assert_eq!(to_number(&v), Some(2.5));
///
/// let v = Value::Empty;
/// assert_eq!(to_number(&v), None);
/// ```
fn to_number(value: &Value) -> Option<f64> {
    match value {
        Value::Int(n) => Some(*n as f64),
        Value::Float(f) => Some(*f),
        _ => None,
    }
}

/// Compute the sum of all numeric entries (recursively) in the input slice.
///
/// Non-numeric values are ignored during aggregation.
///
/// # Returns
///
/// `Value::Float` containing the sum of all numeric values found.
///
/// # Examples
///
/// ```
/// let vals = [Value::Int(1), Value::Float(2.5)];
/// let result = sum(&vals);
/// assert_eq!(result, Value::Float(3.5));
/// ```
pub fn sum(values: &[Value]) -> Value {
    let mut total = 0.0;
    walk_values(values, &mut |value| {
        if let Some(num) = to_number(value) {
            total += num;
        }
    });
    Value::Float(total)
}

/// Computes the arithmetic mean of all numeric values found in `values`.
///
/// Non-numeric entries and nested arrays are ignored. If at least one numeric value is present,
/// returns `Value::Float(mean)`; if no numeric values are found, returns `Value::Error(ErrorValue::Div0)`.
///
/// # Examples
///
/// ```
/// let vals = &[
///     Value::Int(1),
///     Value::Array(vec![Value::Float(2.0), Value::Int(3)]),
///     Value::Empty,
/// ];
/// let avg = average(vals);
/// assert_eq!(avg, Value::Float(2.0));
/// ```
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

/// Count numeric entries in a (possibly nested) slice of `Value`.
///
/// Traverses `values` (recursing into any `Value::Array`) and counts elements that are integers or floats.
///
/// # Returns
///
/// `Value::Int` containing the number of numeric entries found.
///
/// # Examples
///
/// ```
/// let vals = vec![Value::Int(1), Value::Array(vec![Value::Float(2.5), Value::Empty]), Value::Int(3)];
/// let c = count(&vals);
/// assert_eq!(c, Value::Int(3));
/// ```
pub fn count(values: &[Value]) -> Value {
    let mut count = 0usize;
    walk_values(values, &mut |value| {
        if to_number(value).is_some() {
            count += 1;
        }
    });
    Value::Int(count as i64)
}

/// Finds the maximum numeric value among the provided values, recursively traversing any nested arrays.
///
/// Non-numeric values are ignored. If no numeric values are found, returns `Value::Error(ErrorValue::Value)`.
///
/// # Examples
///
/// ```
/// use crates::utils::Value;
/// use crates::utils::ErrorValue;
/// let vals = &[Value::Int(1), Value::Array(vec![Value::Float(3.5), Value::Int(2)])];
/// let res = max(vals);
/// assert_eq!(res, Value::Float(3.5));
/// ```
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

/// Finds the smallest numeric value among the provided `Value` items, recursing into nested arrays.
///
/// Non-numeric values are ignored. If at least one numeric value is found the result is
/// `Value::Float(minimum)`. If no numeric values are present the function returns
/// `Value::Error(ErrorValue::Value)`.
///
/// # Examples
///
/// ```
/// let vals = &[
///     Value::Int(3),
///     Value::Array(vec![Value::Float(2.5), Value::Int(7)]),
///     Value::Empty,
/// ];
/// assert_eq!(min(vals), Value::Float(2.5));
///
/// let none = &[Value::Empty, Value::Error(ErrorValue::Value)];
/// assert_eq!(min(none), Value::Error(ErrorValue::Value));
/// ```
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

/// Compute the absolute value of the first numeric entry in `values`.
///
/// If the first element is an `Int` or `Float`, returns a `Value::Float` with its absolute value.
/// If the first element is missing or not numeric, returns `Value::Error(ErrorValue::Value)`.
///
/// # Examples
///
/// ```
/// let v = abs(&[Value::Int(-3)]);
/// assert_eq!(v, Value::Float(3.0));
///
/// let v = abs(&[Value::Float(-2.5)]);
/// assert_eq!(v, Value::Float(2.5));
///
/// let v = abs(&[]);
/// assert!(matches!(v, Value::Error(ErrorValue::Value)));
/// ```
pub fn abs(values: &[Value]) -> Value {
    let value = values.first().unwrap_or(&Value::Empty);
    match to_number(value) {
        Some(n) => Value::Float(n.abs()),
        None => Value::Error(ErrorValue::Value),
    }
}