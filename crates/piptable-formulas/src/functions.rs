//! Standard spreadsheet functions implementation

use chrono::{Local, TimeZone, Utc};
use piptable_primitives::{ErrorValue, Value};
use piptable_utils::datetime::datetime_to_excel_date;

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

fn coerce_to_text(value: &Value) -> Result<String, ErrorValue> {
    match value {
        Value::Empty => Ok(String::new()),
        Value::String(s) => Ok(s.clone()),
        Value::Int(n) => Ok(n.to_string()),
        Value::Float(f) => Ok(f.to_string()),
        Value::Bool(b) => Ok(b.to_string()),
        Value::Error(err) => Err(err.clone()),
        Value::Array(values) => {
            let first = values.first().unwrap_or(&Value::Empty);
            coerce_to_text(first)
        }
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

pub fn concat(values: &[Value]) -> Value {
    let mut first_error: Option<ErrorValue> = None;
    let mut result = String::new();

    walk_values(values, &mut |value| match value {
        Value::Error(err) => {
            if first_error.is_none() {
                first_error = Some(err.clone());
            }
        }
        _ => match coerce_to_text(value) {
            Ok(text) => result.push_str(&text),
            Err(err) => {
                if first_error.is_none() {
                    first_error = Some(err);
                }
            }
        },
    });

    if let Some(err) = first_error {
        Value::Error(err)
    } else {
        Value::String(result)
    }
}

pub fn len(values: &[Value]) -> Value {
    let value = values.first().unwrap_or(&Value::Empty);
    match coerce_to_text(value) {
        Ok(text) => Value::Int(text.chars().count() as i64),
        Err(err) => Value::Error(err),
    }
}

pub fn left(values: &[Value]) -> Value {
    let text = values.first().unwrap_or(&Value::Empty);
    let count = values.get(1).and_then(to_number).unwrap_or(1.0);
    if count < 0.0 {
        return Value::Error(ErrorValue::Value);
    }
    let count = count.floor() as usize;
    match coerce_to_text(text) {
        Ok(text) => {
            let result: String = text.chars().take(count).collect();
            Value::String(result)
        }
        Err(err) => Value::Error(err),
    }
}

pub fn right(values: &[Value]) -> Value {
    let text = values.first().unwrap_or(&Value::Empty);
    let count = values.get(1).and_then(to_number).unwrap_or(1.0);
    if count < 0.0 {
        return Value::Error(ErrorValue::Value);
    }
    let count = count.floor() as usize;
    match coerce_to_text(text) {
        Ok(text) => {
            let len = text.chars().count();
            let skip = len.saturating_sub(count);
            let result: String = text.chars().skip(skip).collect();
            Value::String(result)
        }
        Err(err) => Value::Error(err),
    }
}

pub fn today(_: &[Value]) -> Value {
    let local = Local::now();
    let date = local.date_naive();
    let Some(naive) = date.and_hms_opt(0, 0, 0) else {
        return Value::Error(ErrorValue::Value);
    };
    let local_dt = Local.from_local_datetime(&naive).single();
    match local_dt {
        Some(dt) => Value::Float(datetime_to_excel_date(dt.with_timezone(&Utc))),
        None => Value::Error(ErrorValue::Value),
    }
}

pub fn now(_: &[Value]) -> Value {
    let local = Local::now();
    Value::Float(datetime_to_excel_date(local.with_timezone(&Utc)))
}

pub fn date(values: &[Value]) -> Value {
    let year = values.first().and_then(to_number);
    let month = values.get(1).and_then(to_number);
    let day = values.get(2).and_then(to_number);

    let (Some(year), Some(month), Some(day)) = (year, month, day) else {
        return Value::Error(ErrorValue::Value);
    };

    let year = year.floor() as i32;
    let month = month.floor() as u32;
    let day = day.floor() as u32;

    let Some(date) = chrono::NaiveDate::from_ymd_opt(year, month, day) else {
        return Value::Error(ErrorValue::Value);
    };
    let Some(naive) = date.and_hms_opt(0, 0, 0) else {
        return Value::Error(ErrorValue::Value);
    };
    let local_dt = Local.from_local_datetime(&naive).single();
    match local_dt {
        Some(dt) => Value::Float(datetime_to_excel_date(dt.with_timezone(&Utc))),
        None => Value::Error(ErrorValue::Value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_concat_and_len() {
        let values = vec![
            Value::String("ab".to_string()),
            Value::Array(vec![Value::Int(3), Value::Empty]),
        ];
        let result = concat(&values);
        assert_eq!(result, Value::String("ab3".to_string()));

        let result = len(&[Value::String("hello".to_string())]);
        assert_eq!(result, Value::Int(5));
    }

    #[test]
    fn test_left_right_defaults() {
        let result = left(&[Value::String("hello".to_string())]);
        assert_eq!(result, Value::String("h".to_string()));

        let result = right(&[Value::String("hello".to_string())]);
        assert_eq!(result, Value::String("o".to_string()));
    }

    #[test]
    fn test_date_returns_number() {
        let result = date(&[Value::Int(2024), Value::Int(1), Value::Int(1)]);
        assert!(matches!(result, Value::Float(_)));
    }

    #[test]
    fn test_count_and_counta() {
        let values = vec![
            Value::Int(1),
            Value::Float(2.5),
            Value::String("x".to_string()),
            Value::Empty,
            Value::Array(vec![Value::Int(7), Value::String("y".to_string())]),
        ];
        assert_eq!(count(&values), Value::Int(3));

        let values = vec![Value::Empty, Value::String("a".to_string())];
        assert_eq!(counta(&values), Value::Int(1));

        let values = vec![Value::Int(1), Value::Error(ErrorValue::Div0)];
        assert_eq!(counta(&values), Value::Error(ErrorValue::Div0));
    }

    #[test]
    fn test_if_truthiness() {
        let result = if_fn(&[
            Value::Bool(true),
            Value::Int(1),
            Value::Int(2),
        ]);
        assert_eq!(result, Value::Int(1));

        let result = if_fn(&[Value::Int(0), Value::Int(1), Value::Int(2)]);
        assert_eq!(result, Value::Int(2));

        let result = if_fn(&[Value::Empty, Value::Int(1), Value::Int(2)]);
        assert_eq!(result, Value::Int(2));

        let result = if_fn(&[
            Value::String("x".to_string()),
            Value::Int(1),
            Value::Int(2),
        ]);
        assert_eq!(result, Value::Error(ErrorValue::Value));
    }

    #[test]
    fn test_and_or_behaviors() {
        let result = and_fn(&[Value::Bool(true), Value::Int(1), Value::Float(1.0)]);
        assert_eq!(result, Value::Bool(true));

        let result = and_fn(&[Value::Bool(true), Value::Int(0)]);
        assert_eq!(result, Value::Bool(false));

        let result = and_fn(&[Value::Empty]);
        assert_eq!(result, Value::Error(ErrorValue::Value));

        let result = and_fn(&[Value::Float(f64::NAN), Value::Empty]);
        assert_eq!(result, Value::Error(ErrorValue::Value));

        let result = and_fn(&[Value::Error(ErrorValue::Div0), Value::Bool(false)]);
        assert_eq!(result, Value::Error(ErrorValue::Div0));

        let result = or_fn(&[Value::Bool(false), Value::Int(1)]);
        assert_eq!(result, Value::Bool(true));

        let result = or_fn(&[Value::Empty]);
        assert_eq!(result, Value::Error(ErrorValue::Value));

        let result = or_fn(&[Value::Error(ErrorValue::Div0), Value::Bool(true)]);
        assert_eq!(result, Value::Error(ErrorValue::Div0));
    }

    #[test]
    fn test_not_semantics() {
        let result = not_fn(&[Value::Empty]);
        assert_eq!(result, Value::Bool(true));

        let result = not_fn(&[Value::Int(1)]);
        assert_eq!(result, Value::Bool(false));

        let result = not_fn(&[Value::String("x".to_string())]);
        assert_eq!(result, Value::Error(ErrorValue::Value));
    }

    #[test]
    fn test_today_now_returns_number() {
        let result = today(&[]);
        assert!(matches!(result, Value::Float(_)));

        let result = now(&[]);
        assert!(matches!(result, Value::Float(_)));
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
    let mut any_true = false;
    let mut first_error: Option<ErrorValue> = None;

    walk_values(values, &mut |value| match value {
        Value::Empty => {}
        Value::Bool(b) => {
            has_coercible = true;
            if *b {
                any_true = true;
            }
        }
        Value::Int(n) => {
            has_coercible = true;
            if *n != 0 {
                any_true = true;
            }
        }
        Value::Float(f) => {
            if f.is_nan() {
                return;
            }
            has_coercible = true;
            if *f != 0.0 {
                any_true = true;
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
    Value::Bool(any_true)
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
