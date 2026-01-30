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
        if to_number(value).is_some() {
            count += 1;
        }
    });
    Value::Int(count as i64)
}

pub fn counta(values: &[Value]) -> Value {
    let mut count = 0usize;
    let mut error: Option<ErrorValue> = None;

    walk_values(values, &mut |value| match value {
        Value::Empty => {}
        Value::Error(err) => {
            if error.is_none() {
                error = Some(err.clone());
            }
        }
        _ => {
            count += 1;
        }
    });

    if let Some(err) = error {
        Value::Error(err)
    } else {
        Value::Int(count as i64)
    }
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
    let condition = values.first().unwrap_or(&Value::Empty);
    let then_value = values.get(1).cloned().unwrap_or(Value::Bool(true));
    let else_value = values.get(2).cloned().unwrap_or(Value::Bool(false));

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
    let mut has_coercible = false;
    let mut any_false = false;
    let mut first_error: Option<ErrorValue> = None;

    walk_values(values, &mut |value| match value {
        Value::Empty => {}
        Value::Bool(b) => {
            has_coercible = true;
            if !b {
                any_false = true;
            }
        }
        Value::Int(n) => {
            has_coercible = true;
            if *n == 0 {
                any_false = true;
            }
        }
        Value::Float(f) => {
            if f.is_nan() {
                return;
            }
            has_coercible = true;
            if *f == 0.0 {
                any_false = true;
            }
        }
        Value::Error(err) => {
            if first_error.is_none() {
                first_error = Some(err.clone());
            }
        }
        _ => {
            if first_error.is_none() {
                first_error = Some(ErrorValue::Value);
            }
        }
    });

    if let Some(err) = first_error {
        return Value::Error(err);
    }
    if !has_coercible {
        return Value::Error(ErrorValue::Value);
    }
    Value::Bool(!any_false)
}

pub fn or_fn(values: &[Value]) -> Value {
    let mut has_coercible = false;
    let mut result = Value::Bool(false);

    walk_values(values, &mut |value| match value {
        Value::Empty => {}
        Value::Bool(b) => {
            has_coercible = true;
            if *b {
                result = Value::Bool(true);
            }
        }
        Value::Int(n) => {
            has_coercible = true;
            if *n != 0 {
                result = Value::Bool(true);
            }
        }
        Value::Float(f) => {
            has_coercible = true;
            if *f != 0.0 && !f.is_nan() {
                result = Value::Bool(true);
            }
        }
        Value::Error(err) => {
            result = Value::Error(err.clone());
        }
        _ => {
            result = Value::Error(ErrorValue::Value);
        }
    });

    if matches!(result, Value::Bool(true)) {
        return result;
    }
    if matches!(result, Value::Error(_)) {
        return result;
    }
    if has_coercible {
        Value::Bool(false)
    } else {
        Value::Error(ErrorValue::Value)
    }
}

pub fn not_fn(values: &[Value]) -> Value {
    let value = values.first().unwrap_or(&Value::Empty);
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
