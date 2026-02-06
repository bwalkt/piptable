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
