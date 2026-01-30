//! Standard spreadsheet functions implementation

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

/// Sum function - adds all numeric values
pub fn sum(values: &[Value]) -> Value {
    let mut total = 0.0;
    walk_values(values, &mut |value| {
        if let Some(num) = to_number(value) {
            total += num;
        }
    });
    Value::Float(total)
}

/// Average function - calculates mean of numeric values
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

/// Count function - counts non-empty cells
pub fn count(values: &[Value]) -> Value {
    let mut count = 0usize;
    walk_values(values, &mut |value| {
        if !matches!(value, Value::Empty) {
            count += 1;
        }
    });
    Value::Int(count as i64)
}

/// Max function - finds maximum value
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

/// Min function - finds minimum value  
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

pub fn if_fn(values: &[Value]) -> Value {
    let condition = values.get(0).unwrap_or(&Value::Empty);
    let then_value = values.get(1).cloned().unwrap_or(Value::Empty);
    let else_value = values.get(2).cloned().unwrap_or(Value::Empty);

    let truthy = match condition {
        Value::Bool(b) => *b,
        Value::Int(n) => *n != 0,
        Value::Float(f) => *f != 0.0,
        Value::Empty => false,
        _ => return Value::Error(ErrorValue::Value),
    };

    if truthy {
        then_value
    } else {
        else_value
    }
}

pub fn and_fn(values: &[Value]) -> Value {
    for value in values {
        match value {
            Value::Bool(b) => {
                if !b {
                    return Value::Bool(false);
                }
            }
            Value::Int(n) => {
                if *n == 0 {
                    return Value::Bool(false);
                }
            }
            Value::Float(f) => {
                if *f == 0.0 {
                    return Value::Bool(false);
                }
            }
            _ => return Value::Error(ErrorValue::Value),
        }
    }
    Value::Bool(true)
}

pub fn or_fn(values: &[Value]) -> Value {
    for value in values {
        match value {
            Value::Bool(b) => {
                if *b {
                    return Value::Bool(true);
                }
            }
            Value::Int(n) => {
                if *n != 0 {
                    return Value::Bool(true);
                }
            }
            Value::Float(f) => {
                if *f != 0.0 {
                    return Value::Bool(true);
                }
            }
            _ => return Value::Error(ErrorValue::Value),
        }
    }
    Value::Bool(false)
}

pub fn not_fn(values: &[Value]) -> Value {
    let value = values.get(0).unwrap_or(&Value::Empty);
    match value {
        Value::Bool(b) => Value::Bool(!b),
        Value::Int(n) => Value::Bool(*n == 0),
        Value::Float(f) => Value::Bool(*f == 0.0),
        Value::Empty => Value::Bool(true),
        _ => Value::Error(ErrorValue::Value),
    }
}

pub fn not_implemented(_: &[Value]) -> Value {
    Value::Error(ErrorValue::NA)
}
