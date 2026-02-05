//! Standard spreadsheet functions implementation

use chrono::{Local, TimeZone, Utc};
use piptable_primitives::{ErrorValue, Value};
use piptable_utils::datetime::datetime_to_excel_date;

fn local_to_excel(local_dt: Option<chrono::DateTime<Local>>) -> Value {
    match local_dt {
        Some(dt) => Value::Float(datetime_to_excel_date(dt.with_timezone(&Utc))),
        None => Value::Error(ErrorValue::Value),
    }
}

fn date_to_local(date: chrono::NaiveDate, hour: u32, minute: u32, second: u32) -> Value {
    let Some(naive) = date.and_hms_opt(hour, minute, second) else {
        return Value::Error(ErrorValue::Value);
    };
    let local_dt = Local.from_local_datetime(&naive).earliest();
    local_to_excel(local_dt)
}

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

fn read_required_number(value: Option<&Value>) -> Result<f64, ErrorValue> {
    let value = value.unwrap_or(&Value::Empty);
    match value {
        Value::Error(err) => Err(err.clone()),
        other => to_number(other)
            .filter(|n| n.is_finite())
            .ok_or(ErrorValue::Value),
    }
}

fn read_optional_count(value: Option<&Value>) -> Result<f64, ErrorValue> {
    let Some(value) = value else {
        return Ok(1.0);
    };
    match value {
        Value::Error(err) => Err(err.clone()),
        other => to_number(other)
            .filter(|n| n.is_finite())
            .ok_or(ErrorValue::Value),
    }
}

fn to_index(value: &Value) -> Result<usize, ErrorValue> {
    match value {
        Value::Int(n) => {
            if *n < 1 {
                Err(ErrorValue::Value)
            } else {
                Ok(*n as usize)
            }
        }
        Value::Float(f) => {
            if f.is_nan() || f.is_infinite() || *f < 1.0 || *f > (usize::MAX as f64) {
                Err(ErrorValue::Value)
            } else {
                Ok(*f as usize)
            }
        }
        _ => Err(ErrorValue::Value),
    }
}

fn to_offset(value: &Value) -> Result<i64, ErrorValue> {
    match value {
        Value::Int(n) => Ok(*n),
        Value::Float(f) => {
            if f.is_nan() || f.is_infinite() || *f > (i64::MAX as f64) || *f < (i64::MIN as f64) {
                Err(ErrorValue::Value)
            } else {
                Ok(*f as i64)
            }
        }
        _ => Err(ErrorValue::Value),
    }
}

fn values_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Empty, Value::Empty) => true,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Int(a), Value::Int(b)) => a == b,
        (Value::Float(a), Value::Float(b)) => (a - b).abs() < f64::EPSILON,
        (Value::Int(a), Value::Float(b)) | (Value::Float(b), Value::Int(a)) => {
            (*a as f64 - b).abs() < f64::EPSILON
        }
        (Value::String(a), Value::String(b)) => a == b,
        _ => false,
    }
}

fn compare_values(left: &Value, right: &Value) -> Result<i32, ErrorValue> {
    match (left, right) {
        (Value::Empty, Value::Empty) => Ok(0),
        (Value::Empty, _) => Ok(-1),
        (_, Value::Empty) => Ok(1),
        (Value::Int(a), Value::Int(b)) => Ok(a.cmp(b) as i32),
        (Value::Float(a), Value::Float(b)) => Ok(if (a - b).abs() < f64::EPSILON {
            0
        } else if a < b {
            -1
        } else {
            1
        }),
        (Value::Int(a), Value::Float(b)) => {
            let a_f = *a as f64;
            Ok(if (a_f - b).abs() < f64::EPSILON {
                0
            } else if a_f < *b {
                -1
            } else {
                1
            })
        }
        (Value::Float(a), Value::Int(b)) => {
            let b_f = *b as f64;
            Ok(if (a - b_f).abs() < f64::EPSILON {
                0
            } else if *a < b_f {
                -1
            } else {
                1
            })
        }
        (Value::String(a), Value::String(b)) => Ok(a.cmp(b) as i32),
        _ => Err(ErrorValue::Value),
    }
}

#[derive(Clone, Copy)]
enum WildcardToken {
    AnySeq,
    AnyChar,
    Literal(char),
}

fn tokenize_wildcard(pattern: &str) -> Vec<WildcardToken> {
    let mut tokens = Vec::new();
    let mut chars = pattern.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\\' => {
                if let Some(next) = chars.next() {
                    tokens.push(WildcardToken::Literal(next));
                } else {
                    tokens.push(WildcardToken::Literal('\\'));
                }
            }
            '*' => tokens.push(WildcardToken::AnySeq),
            '?' => tokens.push(WildcardToken::AnyChar),
            _ => tokens.push(WildcardToken::Literal(ch)),
        }
    }
    tokens
}

fn wildcard_match(pattern: &str, text: &str, case_insensitive: bool) -> bool {
    let (pattern, text) = if case_insensitive {
        (pattern.to_lowercase(), text.to_lowercase())
    } else {
        (pattern.to_string(), text.to_string())
    };

    let tokens = tokenize_wildcard(&pattern);
    let text_chars: Vec<char> = text.chars().collect();
    let mut dp = vec![false; text_chars.len() + 1];
    dp[0] = true;

    for token in tokens {
        match token {
            WildcardToken::AnySeq => {
                let mut next = vec![false; text_chars.len() + 1];
                let mut seen = false;
                for i in 0..=text_chars.len() {
                    if dp[i] {
                        seen = true;
                    }
                    if seen {
                        next[i] = true;
                    }
                }
                dp = next;
            }
            WildcardToken::AnyChar => {
                let mut next = vec![false; text_chars.len() + 1];
                for i in 0..text_chars.len() {
                    if dp[i] {
                        next[i + 1] = true;
                    }
                }
                dp = next;
            }
            WildcardToken::Literal(ch) => {
                let mut next = vec![false; text_chars.len() + 1];
                for i in 0..text_chars.len() {
                    if dp[i] && text_chars[i] == ch {
                        next[i + 1] = true;
                    }
                }
                dp = next;
            }
        }
    }

    dp[text_chars.len()]
}

fn validate_sorted(values: &[Value], ascending: bool) -> Result<(), ErrorValue> {
    for i in 1..values.len() {
        let cmp = compare_values(&values[i - 1], &values[i])?;
        if ascending && cmp > 0 {
            return Err(ErrorValue::Value);
        }
        if !ascending && cmp < 0 {
            return Err(ErrorValue::Value);
        }
    }
    Ok(())
}

fn lower_bound_asc(values: &[Value], lookup: &Value) -> Result<usize, ErrorValue> {
    let mut lo = 0;
    let mut hi = values.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        let cmp = compare_values(&values[mid], lookup)?;
        if cmp < 0 {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    Ok(lo)
}

fn lower_bound_desc(values: &[Value], lookup: &Value) -> Result<usize, ErrorValue> {
    let mut lo = 0;
    let mut hi = values.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        let cmp = compare_values(&values[mid], lookup)?;
        if cmp > 0 {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    Ok(lo)
}

fn table_rows(value: &Value) -> Result<Vec<Vec<Value>>, ErrorValue> {
    let Value::Array(items) = value else {
        return Err(ErrorValue::Value);
    };
    if items.iter().all(|v| matches!(v, Value::Array(_))) {
        Ok(items
            .iter()
            .map(|row| match row {
                Value::Array(values) => values.clone(),
                _ => unreachable!(),
            })
            .collect())
    } else {
        Ok(items.iter().map(|v| vec![v.clone()]).collect())
    }
}

fn flatten_array(value: &Value) -> Result<Vec<Value>, ErrorValue> {
    let Value::Array(items) = value else {
        return Err(ErrorValue::Value);
    };
    let mut flat = Vec::new();
    for item in items {
        match item {
            Value::Array(values) => flat.extend(values.clone()),
            _ => flat.push(item.clone()),
        }
    }
    Ok(flat)
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

/// CONCATENATE implementation.
pub fn concat(values: &[Value]) -> Value {
    let mut first_error: Option<ErrorValue> = None;
    let mut result = String::new();

    walk_values(values, &mut |value| match coerce_to_text(value) {
        Ok(text) => result.push_str(&text),
        Err(err) => {
            if first_error.is_none() {
                first_error = Some(err);
            }
        }
    });

    if let Some(err) = first_error {
        Value::Error(err)
    } else {
        Value::String(result)
    }
}

/// Returns element count for arrays, or character count for text values.
/// Arrays are not coerced to their first element; instead, their length is returned.
pub fn len(values: &[Value]) -> Value {
    let value = values.first().unwrap_or(&Value::Empty);
    if let Value::Array(items) = value {
        return Value::Int(items.len() as i64);
    }
    match coerce_to_text(value) {
        Ok(text) => Value::Int(text.chars().count() as i64),
        Err(err) => Value::Error(err),
    }
}

/// LEFT implementation.
pub fn left(values: &[Value]) -> Value {
    let text = values.first().unwrap_or(&Value::Empty);
    let count = match read_optional_count(values.get(1)) {
        Ok(value) => value,
        Err(err) => return Value::Error(err),
    };
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

/// RIGHT implementation.
pub fn right(values: &[Value]) -> Value {
    let text = values.first().unwrap_or(&Value::Empty);
    let count = match read_optional_count(values.get(1)) {
        Ok(value) => value,
        Err(err) => return Value::Error(err),
    };
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

/// TODAY implementation.
pub fn today(_: &[Value]) -> Value {
    let local = Local::now();
    let date = local.date_naive();
    date_to_local(date, 0, 0, 0)
}

/// NOW implementation.
pub fn now(_: &[Value]) -> Value {
    let local = Local::now();
    local_to_excel(Some(local))
}

/// DATE implementation.
pub fn date(values: &[Value]) -> Value {
    let year = match read_required_number(values.first()) {
        Ok(value) => value,
        Err(err) => return Value::Error(err),
    };
    let month = match read_required_number(values.get(1)) {
        Ok(value) => value,
        Err(err) => return Value::Error(err),
    };
    let day = match read_required_number(values.get(2)) {
        Ok(value) => value,
        Err(err) => return Value::Error(err),
    };

    let year = year.floor() as i32;
    let month = month.floor() as u32;
    let day = day.floor() as u32;

    let Some(date) = chrono::NaiveDate::from_ymd_opt(year, month, day) else {
        return Value::Error(ErrorValue::Value);
    };
    date_to_local(date, 0, 0, 0)
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
    fn test_count() {
        let values = vec![
            Value::Int(1),
            Value::Float(2.5),
            Value::String("x".to_string()),
            Value::Empty,
            Value::Array(vec![Value::Int(7), Value::String("y".to_string())]),
        ];
        assert_eq!(count(&values), Value::Int(3));
    }

    #[test]
    fn test_if_truthiness() {
        let result = if_fn(&[Value::Bool(true), Value::Int(1), Value::Int(2)]);
        assert_eq!(result, Value::Int(1));

        let result = if_fn(&[Value::Int(0), Value::Int(1), Value::Int(2)]);
        assert_eq!(result, Value::Int(2));

        let result = if_fn(&[Value::Empty, Value::Int(1), Value::Int(2)]);
        assert_eq!(result, Value::Int(2));

        let result = if_fn(&[Value::String("x".to_string()), Value::Int(1), Value::Int(2)]);
        assert_eq!(result, Value::Error(ErrorValue::Value));
    }

    #[test]
    fn test_and_or_behaviors() {
        let result = and_fn(&[Value::Bool(true), Value::Int(1), Value::Float(1.0)]);
        assert_eq!(result, Value::Bool(true));

        let result = and_fn(&[Value::Bool(true), Value::Int(0)]);
        assert_eq!(result, Value::Bool(false));

        let result = and_fn(&[Value::Empty]);
        assert_eq!(result, Value::Bool(false));

        let result = and_fn(&[Value::Float(f64::NAN), Value::Empty]);
        assert_eq!(result, Value::Error(ErrorValue::Num));

        let result = and_fn(&[Value::Error(ErrorValue::Div0), Value::Bool(false)]);
        assert_eq!(result, Value::Error(ErrorValue::Div0));

        let result = or_fn(&[Value::Bool(false), Value::Int(1)]);
        assert_eq!(result, Value::Bool(true));

        let result = or_fn(&[Value::Empty]);
        assert_eq!(result, Value::Bool(false));

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

    #[test]
    fn test_average_no_numbers() {
        let result = average(&[Value::String("x".to_string()), Value::Empty]);
        assert_eq!(result, Value::Error(ErrorValue::Div0));
    }

    #[test]
    fn test_max_min_no_numbers() {
        let result = max(&[Value::String("x".to_string()), Value::Empty]);
        assert_eq!(result, Value::Error(ErrorValue::Value));

        let result = min(&[Value::String("x".to_string()), Value::Empty]);
        assert_eq!(result, Value::Error(ErrorValue::Value));
    }

    #[test]
    fn test_left_right_errors() {
        let result = left(&[Value::String("hello".to_string()), Value::Int(-1)]);
        assert_eq!(result, Value::Error(ErrorValue::Value));

        let result = right(&[Value::String("hello".to_string()), Value::Int(-2)]);
        assert_eq!(result, Value::Error(ErrorValue::Value));

        let result = left(&[Value::Error(ErrorValue::Ref)]);
        assert_eq!(result, Value::Error(ErrorValue::Ref));
    }

    #[test]
    fn test_len_error_and_array_length() {
        let result = len(&[Value::Error(ErrorValue::Num)]);
        assert_eq!(result, Value::Error(ErrorValue::Num));

        let result = len(&[Value::Array(vec![Value::String("abc".to_string())])]);
        assert_eq!(result, Value::Int(1));

        let result = len(&[Value::Array(vec![])]);
        assert_eq!(result, Value::Int(0));
    }

    #[test]
    fn test_date_invalid_inputs() {
        let result = date(&[Value::Int(2024), Value::Int(13), Value::Int(1)]);
        assert_eq!(result, Value::Error(ErrorValue::Value));

        let result = date(&[Value::Int(2024), Value::Int(1)]);
        assert_eq!(result, Value::Error(ErrorValue::Value));
    }

    #[test]
    fn test_not_implemented_returns_na() {
        let result = not_implemented(&[]);
        assert_eq!(result, Value::Error(ErrorValue::NA));
    }

    #[test]
    fn test_concat_array_and_error() {
        let result = concat(&[Value::Array(vec![]), Value::String("x".to_string())]);
        assert_eq!(result, Value::String("x".to_string()));

        let result = concat(&[Value::Array(vec![Value::Error(ErrorValue::Ref)])]);
        assert_eq!(result, Value::Error(ErrorValue::Ref));

        let result = concat(&[Value::Bool(true), Value::Float(1.5)]);
        assert_eq!(result, Value::String("true1.5".to_string()));
    }

    #[test]
    fn test_sum_with_nested_arrays() {
        let values = vec![
            Value::Array(vec![Value::Int(1), Value::String("x".to_string())]),
            Value::Array(vec![Value::Float(2.5), Value::Empty]),
        ];
        let result = sum(&values);
        assert!(matches!(result, Value::Float(f) if (f - 3.5).abs() < 1e-9));
    }

    #[test]
    fn test_average_with_numbers() {
        let values = vec![Value::Int(2), Value::Float(4.0)];
        let result = average(&values);
        assert!(matches!(result, Value::Float(f) if (f - 3.0).abs() < 1e-9));
    }

    #[test]
    fn test_max_min_with_numbers() {
        let values = vec![Value::Int(2), Value::Float(4.0)];
        let result = max(&values);
        assert!(matches!(result, Value::Float(f) if (f - 4.0).abs() < 1e-9));

        let result = min(&values);
        assert!(matches!(result, Value::Float(f) if (f - 2.0).abs() < 1e-9));
    }

    #[test]
    fn test_if_defaults() {
        let result = if_fn(&[Value::Bool(true)]);
        assert_eq!(result, Value::Error(ErrorValue::Value));

        let result = if_fn(&[Value::Bool(false)]);
        assert_eq!(result, Value::Error(ErrorValue::Value));

        let result = if_fn(&[Value::Bool(false), Value::Int(1)]);
        assert_eq!(result, Value::Bool(false));

        let result = if_fn(&[Value::Float(1.0), Value::Int(1), Value::Int(2)]);
        assert_eq!(result, Value::Int(1));
    }

    #[test]
    fn test_local_to_excel_failure() {
        let result = local_to_excel(None);
        assert_eq!(result, Value::Error(ErrorValue::Value));
    }

    #[test]
    fn test_date_to_local_invalid_time() {
        let date = chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let result = date_to_local(date, 24, 0, 0);
        assert_eq!(result, Value::Error(ErrorValue::Value));
    }

    #[test]
    fn test_right_error_and_or_float_cases() {
        let result = right(&[Value::Error(ErrorValue::Div0)]);
        assert_eq!(result, Value::Error(ErrorValue::Div0));

        let result = and_fn(&[Value::Float(0.0), Value::Bool(true)]);
        assert_eq!(result, Value::Bool(false));

        let result = and_fn(&[Value::String("x".to_string())]);
        assert_eq!(result, Value::Error(ErrorValue::Value));

        let result = or_fn(&[Value::Float(0.0), Value::Int(0)]);
        assert_eq!(result, Value::Bool(false));

        let result = or_fn(&[Value::Float(2.0), Value::Int(0)]);
        assert_eq!(result, Value::Bool(true));

        let result = or_fn(&[Value::Float(f64::NAN), Value::Empty]);
        assert_eq!(result, Value::Error(ErrorValue::Num));

        let result = or_fn(&[Value::String("x".to_string())]);
        assert_eq!(result, Value::Error(ErrorValue::Value));
    }

    #[test]
    fn test_not_bool_and_float() {
        let result = not_fn(&[Value::Bool(true)]);
        assert_eq!(result, Value::Bool(false));

        let result = not_fn(&[Value::Float(0.0)]);
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn test_not_error_cases() {
        let result = not_fn(&[Value::String("x".to_string())]);
        assert_eq!(result, Value::Error(ErrorValue::Value));

        let result = not_fn(&[Value::Float(f64::NAN)]);
        assert_eq!(result, Value::Error(ErrorValue::Num));
    }

    #[test]
    fn test_vlookup_and_hlookup() {
        let table = Value::Array(vec![
            Value::Array(vec![Value::String("A".to_string()), Value::Int(10)]),
            Value::Array(vec![Value::String("B".to_string()), Value::Int(20)]),
        ]);
        let result = vlookup(&[
            Value::String("B".to_string()),
            table.clone(),
            Value::Int(2),
            Value::Bool(false),
        ]);
        assert_eq!(result, Value::Int(20));

        let headers = Value::Array(vec![
            Value::Array(vec![
                Value::String("Q1".to_string()),
                Value::String("Q2".to_string()),
            ]),
            Value::Array(vec![Value::Int(100), Value::Int(150)]),
        ]);
        let result = hlookup(&[
            Value::String("Q2".to_string()),
            headers,
            Value::Int(2),
            Value::Bool(false),
        ]);
        assert_eq!(result, Value::Int(150));
    }

    #[test]
    fn test_index_and_match() {
        let data = Value::Array(vec![
            Value::Array(vec![Value::Int(10), Value::Int(20)]),
            Value::Array(vec![Value::Int(30), Value::Int(40)]),
        ]);
        let result = index(&[data.clone(), Value::Int(2), Value::Int(1)]);
        assert_eq!(result, Value::Int(30));

        let list = Value::Array(vec![
            Value::String("Apple".to_string()),
            Value::String("Banana".to_string()),
            Value::String("Cherry".to_string()),
        ]);
        let result = match_fn(&[Value::String("Banana".to_string()), list, Value::Int(0)]);
        assert_eq!(result, Value::Int(2));
    }

    #[test]
    fn test_xlookup_basic() {
        let names = Value::Array(vec![
            Value::String("Apple".to_string()),
            Value::String("Banana".to_string()),
        ]);
        let prices = Value::Array(vec![Value::Int(2), Value::Int(3)]);
        let result = xlookup(&[
            Value::String("Banana".to_string()),
            names,
            prices,
            Value::String("N/A".to_string()),
        ]);
        assert_eq!(result, Value::Int(3));
    }

    #[test]
    fn test_offset_matrix() {
        let matrix = Value::Array(vec![
            Value::Array(vec![Value::Int(1), Value::Int(2)]),
            Value::Array(vec![Value::Int(3), Value::Int(4)]),
        ]);
        let result = offset(&[
            matrix,
            Value::Int(1),
            Value::Int(0),
            Value::Int(1),
            Value::Int(2),
        ]);
        assert_eq!(
            result,
            Value::Array(vec![Value::Array(vec![Value::Int(3), Value::Int(4)])])
        );
    }

    #[test]
    fn test_abs() {
        assert_eq!(abs(&[Value::Float(-5.5)]), Value::Float(5.5));
        assert_eq!(abs(&[Value::Int(-10)]), Value::Float(10.0));
        assert_eq!(abs(&[Value::Float(3.14)]), Value::Float(3.14));
        assert_eq!(
            abs(&[Value::String("x".to_string())]),
            Value::Error(ErrorValue::Value)
        );
    }

    #[test]
    fn test_round_functions() {
        assert_eq!(
            round(&[Value::Float(3.456), Value::Int(2)]),
            Value::Float(3.46)
        );
        assert_eq!(round(&[Value::Float(3.456)]), Value::Float(3.0));
        assert_eq!(
            roundup(&[Value::Float(3.1), Value::Int(0)]),
            Value::Float(4.0)
        );
        assert_eq!(
            rounddown(&[Value::Float(3.9), Value::Int(0)]),
            Value::Float(3.0)
        );
        assert_eq!(
            roundup(&[Value::Float(-3.1), Value::Int(0)]),
            Value::Float(-4.0)
        );
        assert_eq!(
            rounddown(&[Value::Float(-3.9), Value::Int(0)]),
            Value::Float(-3.0)
        );
        assert_eq!(
            round(&[Value::Float(1234.0), Value::Int(-2)]),
            Value::Float(1200.0)
        );
        assert_eq!(
            roundup(&[Value::Float(1234.0), Value::Int(-2)]),
            Value::Float(1300.0)
        );
        assert_eq!(
            rounddown(&[Value::Float(1234.0), Value::Int(-2)]),
            Value::Float(1200.0)
        );
    }

    #[test]
    fn test_product() {
        let values = vec![Value::Int(2), Value::Float(3.0), Value::Int(4)];
        assert_eq!(product(&values), Value::Float(24.0));

        let values = vec![Value::String("x".to_string()), Value::Empty];
        assert_eq!(product(&values), Value::Int(0));
    }

    #[test]
    fn test_mod_fn() {
        assert_eq!(mod_fn(&[Value::Int(10), Value::Int(3)]), Value::Float(1.0));
        assert_eq!(
            mod_fn(&[Value::Float(10.5), Value::Float(2.0)]),
            Value::Float(0.5)
        );
        assert_eq!(
            mod_fn(&[Value::Int(10), Value::Int(0)]),
            Value::Error(ErrorValue::Div0)
        );
    }

    #[test]
    fn test_power_sqrt() {
        assert_eq!(power(&[Value::Int(2), Value::Int(3)]), Value::Float(8.0));
        assert_eq!(sqrt(&[Value::Float(16.0)]), Value::Float(4.0));
        assert_eq!(sqrt(&[Value::Float(-1.0)]), Value::Error(ErrorValue::Num));
    }

    #[test]
    fn test_text_functions() {
        assert_eq!(
            trim(&[Value::String("  hello  ".to_string())]),
            Value::String("hello".to_string())
        );
        assert_eq!(
            upper(&[Value::String("hello".to_string())]),
            Value::String("HELLO".to_string())
        );
        assert_eq!(
            lower(&[Value::String("HELLO".to_string())]),
            Value::String("hello".to_string())
        );
        assert_eq!(
            proper(&[Value::String("hello world".to_string())]),
            Value::String("Hello World".to_string())
        );
        assert_eq!(
            proper(&[Value::String("mIXed-case".to_string())]),
            Value::String("Mixed-Case".to_string())
        );
    }

    #[test]
    fn test_is_functions() {
        assert_eq!(isblank(&[Value::Empty]), Value::Bool(true));
        assert_eq!(isblank(&[Value::Int(0)]), Value::Bool(false));
        assert_eq!(isblank(&[Value::String(String::new())]), Value::Bool(false));

        assert_eq!(
            iserror(&[Value::Error(ErrorValue::Div0)]),
            Value::Bool(true)
        );
        assert_eq!(iserror(&[Value::Int(1)]), Value::Bool(false));

        assert_eq!(isna(&[Value::Error(ErrorValue::NA)]), Value::Bool(true));
        assert_eq!(isna(&[Value::Error(ErrorValue::Div0)]), Value::Bool(false));

        assert_eq!(isnumber(&[Value::Int(1)]), Value::Bool(true));
        assert_eq!(isnumber(&[Value::Float(1.5)]), Value::Bool(true));
        assert_eq!(
            isnumber(&[Value::String("1".to_string())]),
            Value::Bool(false)
        );

        assert_eq!(
            istext(&[Value::String("hello".to_string())]),
            Value::Bool(true)
        );
        assert_eq!(istext(&[Value::Int(1)]), Value::Bool(false));
    }

    #[test]
    fn test_even_odd() {
        assert_eq!(even(&[Value::Float(1.5)]), Value::Float(2.0));
        assert_eq!(even(&[Value::Float(2.1)]), Value::Float(4.0));
        assert_eq!(odd(&[Value::Float(2.5)]), Value::Float(3.0));
        assert_eq!(odd(&[Value::Float(3.1)]), Value::Float(5.0));
        assert_eq!(even(&[Value::Float(-1.5)]), Value::Float(-2.0));
        assert_eq!(even(&[Value::Float(-2.1)]), Value::Float(-4.0));
        assert_eq!(odd(&[Value::Float(-2.5)]), Value::Float(-3.0));
        assert_eq!(
            even(&[Value::String("x".to_string())]),
            Value::Error(ErrorValue::Value)
        );
        assert_eq!(
            odd(&[Value::String("x".to_string())]),
            Value::Error(ErrorValue::Value)
        );
    }

    #[test]
    fn test_int_trunc() {
        assert_eq!(int(&[Value::Float(3.9)]), Value::Float(3.0));
        assert_eq!(int(&[Value::Float(-3.9)]), Value::Float(-4.0));
        assert_eq!(trunc(&[Value::Float(3.9)]), Value::Float(3.0));
        assert_eq!(trunc(&[Value::Float(-3.9)]), Value::Float(-3.0));
        assert_eq!(
            trunc(&[Value::Float(3.456), Value::Int(2)]),
            Value::Float(3.45)
        );
    }

    #[test]
    fn test_sign() {
        assert_eq!(sign(&[Value::Float(5.0)]), Value::Int(1));
        assert_eq!(sign(&[Value::Float(-5.0)]), Value::Int(-1));
        assert_eq!(sign(&[Value::Float(0.0)]), Value::Int(0));
    }

    #[test]
    fn test_pi_exp_ln_log() {
        let pi_result = pi(&[]);
        assert!(matches!(pi_result, Value::Float(f) if (f - std::f64::consts::PI).abs() < 1e-10));

        let exp_result = exp(&[Value::Float(1.0)]);
        assert!(matches!(exp_result, Value::Float(f) if (f - std::f64::consts::E).abs() < 1e-10));

        assert_eq!(ln(&[Value::Float(std::f64::consts::E)]), Value::Float(1.0));
        assert_eq!(ln(&[Value::Float(-1.0)]), Value::Error(ErrorValue::Num));

        let log_result = log(&[Value::Float(100.0)]);
        assert!(matches!(log_result, Value::Float(f) if (f - 2.0).abs() < 1e-10));
        assert_eq!(
            log(&[Value::Float(100.0), Value::Float(1.0)]),
            Value::Error(ErrorValue::Num)
        );
        assert_eq!(
            log(&[Value::Float(-1.0), Value::Float(10.0)]),
            Value::Error(ErrorValue::Num)
        );
        assert_eq!(
            log(&[Value::Float(10.0), Value::Float(-2.0)]),
            Value::Error(ErrorValue::Num)
        );

        assert_eq!(log10(&[Value::Float(1000.0)]), Value::Float(3.0));
        assert_eq!(log10(&[Value::Float(0.0)]), Value::Error(ErrorValue::Num));
    }

    #[test]
    fn test_fact() {
        assert_eq!(fact(&[Value::Int(0)]), Value::Float(1.0));
        assert_eq!(fact(&[Value::Int(5)]), Value::Float(120.0));
        assert_eq!(fact(&[Value::Float(5.5)]), Value::Float(120.0)); // Floors to 5
        assert_eq!(fact(&[Value::Int(-1)]), Value::Error(ErrorValue::Num));
        assert_eq!(fact(&[Value::Int(171)]), Value::Error(ErrorValue::Num)); // Too large
    }

    #[test]
    fn test_rand() {
        let result = rand(&[]);
        assert!(matches!(result, Value::Float(f) if f >= 0.0 && f < 1.0));
    }

    #[test]
    fn test_randbetween() {
        let result = randbetween(&[Value::Int(1), Value::Int(10)]);
        assert!(matches!(result, Value::Int(n) if n >= 1 && n <= 10));

        assert_eq!(
            randbetween(&[Value::Int(10), Value::Int(1)]),
            Value::Error(ErrorValue::Value)
        );

        let result = randbetween(&[Value::Float(1.9), Value::Float(3.1)]);
        assert!(matches!(result, Value::Int(n) if n >= 1 && n <= 3));
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

/// IF implementation.
pub fn if_fn(values: &[Value]) -> Value {
    if values.len() < 2 {
        return Value::Error(ErrorValue::Value);
    }
    let condition = values.first().unwrap_or(&Value::Empty);
    let then_value = values.get(1).cloned().unwrap();
    let else_value = values.get(2).cloned().unwrap_or(Value::Bool(false));

    let truthy = match coerce_to_bool(condition) {
        Ok(value) => value,
        Err(err) => return Value::Error(err),
    };

    if truthy {
        then_value
    } else {
        else_value
    }
}

/// AND implementation.
pub fn and_fn(values: &[Value]) -> Value {
    let mut any_false = false;
    let mut first_error: Option<ErrorValue> = None;

    walk_values(values, &mut |value| match coerce_to_bool(value) {
        Ok(false) => {
            any_false = true;
        }
        Ok(true) => {}
        Err(err) => {
            if first_error.is_none() {
                first_error = Some(err);
            }
        }
    });

    if let Some(err) = first_error {
        return Value::Error(err);
    }
    Value::Bool(!any_false)
}

/// OR implementation.
pub fn or_fn(values: &[Value]) -> Value {
    let mut any_true = false;
    let mut first_error: Option<ErrorValue> = None;

    walk_values(values, &mut |value| match coerce_to_bool(value) {
        Ok(true) => {
            any_true = true;
        }
        Ok(false) => {}
        Err(err) => {
            if first_error.is_none() {
                first_error = Some(err);
            }
        }
    });

    if let Some(err) = first_error {
        return Value::Error(err);
    }
    Value::Bool(any_true)
}

/// NOT implementation.
pub fn not_fn(values: &[Value]) -> Value {
    let value = values.first().unwrap_or(&Value::Empty);
    match coerce_to_bool(value) {
        Ok(value) => Value::Bool(!value),
        Err(err) => Value::Error(err),
    }
}

/// Coerce a value to a boolean, returning a formula error when invalid.
pub fn coerce_to_bool(value: &Value) -> Result<bool, ErrorValue> {
    match value {
        Value::Bool(b) => Ok(*b),
        Value::Int(n) => Ok(*n != 0),
        Value::Float(f) => {
            if f.is_nan() {
                Err(ErrorValue::Num)
            } else {
                Ok(*f != 0.0)
            }
        }
        Value::Empty => Ok(false),
        Value::Error(err) => Err(err.clone()),
        _ => Err(ErrorValue::Value),
    }
}

/// VLOOKUP(lookup_value, table_array, col_index_num, [range_lookup])
pub fn vlookup(values: &[Value]) -> Value {
    let lookup_value = values.first().unwrap_or(&Value::Empty);
    let table_value = values.get(1).unwrap_or(&Value::Empty);
    let col_index = match values.get(2).and_then(|v| to_index(v).ok()) {
        Some(idx) => idx,
        None => return Value::Error(ErrorValue::Value),
    };

    let exact_match = if let Some(range_lookup) = values.get(3) {
        match range_lookup {
            Value::Bool(b) => !*b,
            Value::Int(0) => true,
            Value::Float(f) if *f == 0.0 => true,
            _ => false,
        }
    } else {
        false
    };

    let rows = match table_rows(table_value) {
        Ok(rows) => rows,
        Err(err) => return Value::Error(err),
    };

    if exact_match {
        for row in &rows {
            if row.is_empty() {
                continue;
            }
            if values_equal(&row[0], lookup_value) {
                if col_index > row.len() {
                    return Value::Error(ErrorValue::Ref);
                }
                return row[col_index - 1].clone();
            }
        }
    } else {
        let mut best_match: Option<&Vec<Value>> = None;
        for row in &rows {
            if row.is_empty() {
                continue;
            }
            match compare_values(&row[0], lookup_value) {
                Ok(cmp) if cmp <= 0 => best_match = Some(row),
                Ok(_) => break,
                Err(err) => return Value::Error(err),
            }
        }
        if let Some(row) = best_match {
            if col_index > row.len() {
                return Value::Error(ErrorValue::Ref);
            }
            return row[col_index - 1].clone();
        }
    }

    Value::Error(ErrorValue::NA)
}

/// HLOOKUP(lookup_value, table_array, row_index_num, [range_lookup])
pub fn hlookup(values: &[Value]) -> Value {
    let lookup_value = values.first().unwrap_or(&Value::Empty);
    let table_value = values.get(1).unwrap_or(&Value::Empty);
    let row_index = match values.get(2).and_then(|v| to_index(v).ok()) {
        Some(idx) => idx,
        None => return Value::Error(ErrorValue::Value),
    };

    let exact_match = if let Some(range_lookup) = values.get(3) {
        match range_lookup {
            Value::Bool(b) => !*b,
            Value::Int(0) => true,
            Value::Float(f) if *f == 0.0 => true,
            _ => false,
        }
    } else {
        false
    };

    let rows = match table_rows(table_value) {
        Ok(rows) => rows,
        Err(err) => return Value::Error(err),
    };

    if rows.is_empty() {
        return Value::Error(ErrorValue::NA);
    }
    if row_index > rows.len() {
        return Value::Error(ErrorValue::Ref);
    }

    let first_row = &rows[0];

    if exact_match {
        for (col_index, cell) in first_row.iter().enumerate() {
            if values_equal(cell, lookup_value) {
                let target_row = &rows[row_index - 1];
                if col_index < target_row.len() {
                    return target_row[col_index].clone();
                }
                return Value::Empty;
            }
        }
    } else {
        let mut best_match_index: Option<usize> = None;
        for (col_index, cell) in first_row.iter().enumerate() {
            match compare_values(cell, lookup_value) {
                Ok(cmp) if cmp <= 0 => best_match_index = Some(col_index),
                Ok(_) => break,
                Err(err) => return Value::Error(err),
            }
        }

        if let Some(col_index) = best_match_index {
            let target_row = &rows[row_index - 1];
            if col_index < target_row.len() {
                return target_row[col_index].clone();
            }
            return Value::Empty;
        }
    }

    Value::Error(ErrorValue::NA)
}

/// INDEX(array, row_num, [column_num])
pub fn index(values: &[Value]) -> Value {
    let array_value = values.first().unwrap_or(&Value::Empty);
    let row_num = match values.get(1).and_then(|v| to_index(v).ok()) {
        Some(idx) => idx,
        None => return Value::Error(ErrorValue::Value),
    };

    let rows = match table_rows(array_value) {
        Ok(rows) => rows,
        Err(err) => return Value::Error(err),
    };

    if row_num > rows.len() {
        return Value::Error(ErrorValue::Ref);
    }

    let row_data = &rows[row_num - 1];

    if values.len() == 2 {
        if rows.len() == 1 && row_data.len() == 1 {
            return row_data[0].clone();
        }
        return Value::Array(row_data.clone());
    }

    let col_num = match values.get(2).and_then(|v| to_index(v).ok()) {
        Some(idx) => idx,
        None => return Value::Error(ErrorValue::Value),
    };

    if col_num > row_data.len() {
        return Value::Error(ErrorValue::Ref);
    }
    row_data[col_num - 1].clone()
}

/// MATCH(lookup_value, lookup_array, [match_type])
pub fn match_fn(values: &[Value]) -> Value {
    let lookup_value = values.first().unwrap_or(&Value::Empty);
    let lookup_array = values.get(1).unwrap_or(&Value::Empty);

    let match_type = if let Some(value) = values.get(2) {
        match value {
            Value::Int(n) => *n,
            Value::Float(f) => *f as i64,
            _ => 1,
        }
    } else {
        1
    };

    let flat_array = match flatten_array(lookup_array) {
        Ok(values) => values,
        Err(err) => return Value::Error(err),
    };

    match match_type {
        0 => {
            for (i, val) in flat_array.iter().enumerate() {
                if values_equal(val, lookup_value) {
                    return Value::Int((i + 1) as i64);
                }
            }
            Value::Error(ErrorValue::NA)
        }
        1 => {
            let mut last_valid_index = None;
            for (i, val) in flat_array.iter().enumerate() {
                match compare_values(val, lookup_value) {
                    Ok(cmp) if cmp <= 0 => last_valid_index = Some(i + 1),
                    Ok(_) => break,
                    Err(err) => return Value::Error(err),
                }
            }
            match last_valid_index {
                Some(idx) => Value::Int(idx as i64),
                None => Value::Error(ErrorValue::NA),
            }
        }
        -1 => {
            let mut last_valid_index = None;
            for (i, val) in flat_array.iter().enumerate() {
                match compare_values(val, lookup_value) {
                    Ok(cmp) if cmp >= 0 => last_valid_index = Some(i + 1),
                    Ok(_) => break,
                    Err(err) => return Value::Error(err),
                }
            }
            match last_valid_index {
                Some(idx) => Value::Int(idx as i64),
                None => Value::Error(ErrorValue::NA),
            }
        }
        _ => Value::Error(ErrorValue::Value),
    }
}

/// XLOOKUP(lookup_value, lookup_array, return_array, [if_not_found], [match_mode], [search_mode])
pub fn xlookup(values: &[Value]) -> Value {
    let lookup_value = values.first().unwrap_or(&Value::Empty);
    let lookup_array = values.get(1).unwrap_or(&Value::Empty);
    let return_array = values.get(2).unwrap_or(&Value::Empty);

    let flat_lookup = match flatten_array(lookup_array) {
        Ok(values) => values,
        Err(err) => return Value::Error(err),
    };
    let flat_return = match flatten_array(return_array) {
        Ok(values) => values,
        Err(err) => return Value::Error(err),
    };

    if flat_lookup.len() != flat_return.len() {
        return Value::Error(ErrorValue::Value);
    }

    let if_not_found = values
        .get(3)
        .cloned()
        .unwrap_or(Value::Error(ErrorValue::NA));

    let match_mode = if let Some(value) = values.get(4) {
        match value {
            Value::Int(n) => *n,
            Value::Float(f) => *f as i64,
            _ => 0,
        }
    } else {
        0
    };

    let search_mode = if let Some(value) = values.get(5) {
        match value {
            Value::Int(n) => *n,
            Value::Float(f) => *f as i64,
            _ => 1,
        }
    } else {
        1
    };

    let case_insensitive = if let Some(value) = values.get(6) {
        match value {
            Value::Bool(b) => *b,
            Value::Int(n) => *n != 0,
            Value::Float(f) => *f != 0.0,
            _ => false,
        }
    } else {
        false
    };

    if search_mode == 2 || search_mode == -2 {
        if match_mode == 2 {
            return Value::Error(ErrorValue::Value);
        }

        let ascending = search_mode == 2;
        if let Err(err) = validate_sorted(&flat_lookup, ascending) {
            return Value::Error(err);
        }

        if flat_lookup.is_empty() {
            return if_not_found;
        }

        let index_result = if ascending {
            let lb = match lower_bound_asc(&flat_lookup, lookup_value) {
                Ok(idx) => idx,
                Err(err) => return Value::Error(err),
            };
            if lb < flat_lookup.len() {
                match compare_values(&flat_lookup[lb], lookup_value) {
                    Ok(0) => Some(lb),
                    Ok(_) => match match_mode {
                        -1 => lb.checked_sub(1),
                        1 => Some(lb),
                        _ => None,
                    },
                    Err(err) => return Value::Error(err),
                }
            } else {
                match match_mode {
                    -1 => lb.checked_sub(1),
                    _ => None,
                }
            }
        } else {
            let lb = match lower_bound_desc(&flat_lookup, lookup_value) {
                Ok(idx) => idx,
                Err(err) => return Value::Error(err),
            };
            if lb < flat_lookup.len() {
                match compare_values(&flat_lookup[lb], lookup_value) {
                    Ok(0) => Some(lb),
                    Ok(_) => match match_mode {
                        -1 => Some(lb),
                        1 => lb.checked_sub(1),
                        _ => None,
                    },
                    Err(err) => return Value::Error(err),
                }
            } else {
                match match_mode {
                    1 => flat_lookup.len().checked_sub(1),
                    _ => None,
                }
            }
        };

        if let Some(idx) = index_result {
            return flat_return[idx].clone();
        }
        return if_not_found;
    }

    let indices: Vec<usize> = match search_mode {
        -1 => (0..flat_lookup.len()).rev().collect(),
        _ => (0..flat_lookup.len()).collect(),
    };

    match match_mode {
        0 => {
            for i in indices {
                if values_equal(&flat_lookup[i], lookup_value) {
                    return flat_return[i].clone();
                }
            }
        }
        -1 => {
            for i in &indices {
                if values_equal(&flat_lookup[*i], lookup_value) {
                    return flat_return[*i].clone();
                }
            }

            let mut best_index: Option<usize> = None;
            for i in &indices {
                match compare_values(&flat_lookup[*i], lookup_value) {
                    Ok(cmp) if cmp < 0 => {
                        if let Some(best) = best_index {
                            match compare_values(&flat_lookup[*i], &flat_lookup[best]) {
                                Ok(cmp_best) if cmp_best > 0 => best_index = Some(*i),
                                Ok(_) => {}
                                Err(err) => return Value::Error(err),
                            }
                        } else {
                            best_index = Some(*i);
                        }
                    }
                    Ok(_) => {}
                    Err(err) => return Value::Error(err),
                }
            }

            if let Some(best) = best_index {
                return flat_return[best].clone();
            }
        }
        1 => {
            for i in &indices {
                if values_equal(&flat_lookup[*i], lookup_value) {
                    return flat_return[*i].clone();
                }
            }

            let mut best_index: Option<usize> = None;
            for i in &indices {
                match compare_values(&flat_lookup[*i], lookup_value) {
                    Ok(cmp) if cmp > 0 => {
                        if let Some(best) = best_index {
                            match compare_values(&flat_lookup[*i], &flat_lookup[best]) {
                                Ok(cmp_best) if cmp_best < 0 => best_index = Some(*i),
                                Ok(_) => {}
                                Err(err) => return Value::Error(err),
                            }
                        } else {
                            best_index = Some(*i);
                        }
                    }
                    Ok(_) => {}
                    Err(err) => return Value::Error(err),
                }
            }

            if let Some(best) = best_index {
                return flat_return[best].clone();
            }
        }
        2 => {
            let pattern = match lookup_value {
                Value::String(text) => text,
                _ => return Value::Error(ErrorValue::Value),
            };
            for i in indices {
                if let Value::String(candidate) = &flat_lookup[i] {
                    if wildcard_match(pattern, candidate, case_insensitive) {
                        return flat_return[i].clone();
                    }
                }
            }
        }
        _ => return Value::Error(ErrorValue::Value),
    }

    if_not_found
}

/// OFFSET(reference, rows, cols, [height], [width])
pub fn offset(values: &[Value]) -> Value {
    let reference = values.first().unwrap_or(&Value::Empty);
    let rows_offset = match values.get(1).and_then(|v| to_offset(v).ok()) {
        Some(v) => v,
        None => return Value::Error(ErrorValue::Value),
    };
    let cols_offset = match values.get(2).and_then(|v| to_offset(v).ok()) {
        Some(v) => v,
        None => return Value::Error(ErrorValue::Value),
    };

    let matrix = match reference {
        Value::Array(_) => match table_rows(reference) {
            Ok(rows) => rows,
            Err(err) => return Value::Error(err),
        },
        _ => vec![vec![reference.clone()]],
    };

    let ref_rows = matrix.len() as i64;
    let ref_cols = matrix.first().map(|row| row.len() as i64).unwrap_or(0);

    let height = if let Some(value) = values.get(3) {
        match to_index(value) {
            Ok(v) => v as i64,
            Err(err) => return Value::Error(err),
        }
    } else {
        ref_rows
    };

    let width = if let Some(value) = values.get(4) {
        match to_index(value) {
            Ok(v) => v as i64,
            Err(err) => return Value::Error(err),
        }
    } else {
        ref_cols
    };

    let start_row = rows_offset;
    let start_col = cols_offset;
    let end_row = start_row + height - 1;
    let end_col = start_col + width - 1;

    if start_row < 0
        || start_col < 0
        || end_row >= ref_rows
        || end_col >= ref_cols
        || ref_rows == 0
        || ref_cols == 0
    {
        return Value::Error(ErrorValue::Ref);
    }

    if height == 1 && width == 1 {
        return matrix[start_row as usize][start_col as usize].clone();
    }

    let mut out = Vec::new();
    for r in start_row..=end_row {
        let mut row = Vec::new();
        for c in start_col..=end_col {
            row.push(matrix[r as usize][c as usize].clone());
        }
        out.push(Value::Array(row));
    }

    Value::Array(out)
}

/// Placeholder for unsupported functions.
pub fn not_implemented(_: &[Value]) -> Value {
    Value::Error(ErrorValue::NA)
}

/// ABS function - returns absolute value
pub fn abs(values: &[Value]) -> Value {
    let value = values.first().unwrap_or(&Value::Empty);
    match to_number(value) {
        Some(n) => Value::Float(n.abs()),
        None => Value::Error(ErrorValue::Value),
    }
}

/// ROUND function - rounds to specified decimal places
pub fn round(values: &[Value]) -> Value {
    let number = values.first().and_then(to_number);
    let places = values.get(1).and_then(to_number).unwrap_or(0.0);

    match number {
        Some(n) => {
            let places = places.floor() as i32;
            let multiplier = 10_f64.powi(places);
            Value::Float((n * multiplier).round() / multiplier)
        }
        None => Value::Error(ErrorValue::Value),
    }
}

/// ROUNDUP function - rounds up to specified decimal places
pub fn roundup(values: &[Value]) -> Value {
    let number = values.first().and_then(to_number);
    let places = values.get(1).and_then(to_number).unwrap_or(0.0);

    match number {
        Some(n) => {
            let places = places.floor() as i32;
            let multiplier = 10_f64.powi(places);
            let scaled = n * multiplier;
            let rounded = if scaled.is_sign_negative() {
                scaled.floor()
            } else {
                scaled.ceil()
            };
            Value::Float(rounded / multiplier)
        }
        None => Value::Error(ErrorValue::Value),
    }
}

/// ROUNDDOWN function - rounds down to specified decimal places
pub fn rounddown(values: &[Value]) -> Value {
    let number = values.first().and_then(to_number);
    let places = values.get(1).and_then(to_number).unwrap_or(0.0);

    match number {
        Some(n) => {
            let places = places.floor() as i32;
            let multiplier = 10_f64.powi(places);
            let scaled = n * multiplier;
            let rounded = if scaled.is_sign_negative() {
                scaled.ceil()
            } else {
                scaled.floor()
            };
            Value::Float(rounded / multiplier)
        }
        None => Value::Error(ErrorValue::Value),
    }
}

/// PRODUCT function - multiplies all numeric values
pub fn product(values: &[Value]) -> Value {
    let mut result = 1.0;
    let mut has_number = false;

    walk_values(values, &mut |value| {
        if let Some(num) = to_number(value) {
            result *= num;
            has_number = true;
        }
    });

    if has_number {
        Value::Float(result)
    } else {
        Value::Int(0)
    }
}

/// MOD function - returns remainder after division
pub fn mod_fn(values: &[Value]) -> Value {
    let dividend = values.first().and_then(to_number);
    let divisor = values.get(1).and_then(to_number);

    match (dividend, divisor) {
        (Some(a), Some(b)) => {
            if b == 0.0 {
                Value::Error(ErrorValue::Div0)
            } else {
                Value::Float(a % b)
            }
        }
        _ => Value::Error(ErrorValue::Value),
    }
}

/// POWER function - raises number to a power
pub fn power(values: &[Value]) -> Value {
    let base = values.first().and_then(to_number);
    let exponent = values.get(1).and_then(to_number);

    match (base, exponent) {
        (Some(a), Some(b)) => Value::Float(a.powf(b)),
        _ => Value::Error(ErrorValue::Value),
    }
}

/// SQRT function - returns square root
pub fn sqrt(values: &[Value]) -> Value {
    let value = values.first().and_then(to_number);

    match value {
        Some(n) if n >= 0.0 => Value::Float(n.sqrt()),
        Some(_) => Value::Error(ErrorValue::Num),
        None => Value::Error(ErrorValue::Value),
    }
}

/// TRIM function - removes leading/trailing spaces
pub fn trim(values: &[Value]) -> Value {
    let text = values.first().unwrap_or(&Value::Empty);
    match coerce_to_text(text) {
        Ok(s) => Value::String(s.trim().to_string()),
        Err(err) => Value::Error(err),
    }
}

/// UPPER function - converts text to uppercase
pub fn upper(values: &[Value]) -> Value {
    let text = values.first().unwrap_or(&Value::Empty);
    match coerce_to_text(text) {
        Ok(s) => Value::String(s.to_uppercase()),
        Err(err) => Value::Error(err),
    }
}

/// LOWER function - converts text to lowercase
pub fn lower(values: &[Value]) -> Value {
    let text = values.first().unwrap_or(&Value::Empty);
    match coerce_to_text(text) {
        Ok(s) => Value::String(s.to_lowercase()),
        Err(err) => Value::Error(err),
    }
}

/// PROPER function - capitalizes first letter of each word
pub fn proper(values: &[Value]) -> Value {
    let text = values.first().unwrap_or(&Value::Empty);
    match coerce_to_text(text) {
        Ok(s) => {
            let mut result = String::with_capacity(s.len());
            let mut prev_is_letter = false;
            for c in s.chars() {
                if !prev_is_letter && c.is_alphabetic() {
                    result.extend(c.to_uppercase());
                } else {
                    result.extend(c.to_lowercase());
                }
                prev_is_letter = c.is_alphabetic();
            }
            Value::String(result)
        }
        Err(err) => Value::Error(err),
    }
}

/// ISBLANK function - checks if cell is blank
pub fn isblank(values: &[Value]) -> Value {
    let value = values.first().unwrap_or(&Value::Empty);
    Value::Bool(matches!(value, Value::Empty))
}

/// ISERROR function - checks if value is an error
pub fn iserror(values: &[Value]) -> Value {
    let value = values.first().unwrap_or(&Value::Empty);
    Value::Bool(matches!(value, Value::Error(_)))
}

/// ISNA function - checks if value is #N/A error
pub fn isna(values: &[Value]) -> Value {
    let value = values.first().unwrap_or(&Value::Empty);
    Value::Bool(matches!(value, Value::Error(ErrorValue::NA)))
}

/// ISNUMBER function - checks if value is a number
pub fn isnumber(values: &[Value]) -> Value {
    let value = values.first().unwrap_or(&Value::Empty);
    Value::Bool(matches!(value, Value::Int(_) | Value::Float(_)))
}

/// ISTEXT function - checks if value is text
pub fn istext(values: &[Value]) -> Value {
    let value = values.first().unwrap_or(&Value::Empty);
    Value::Bool(matches!(value, Value::String(_)))
}

/// EVEN function - rounds up to nearest even integer
pub fn even(values: &[Value]) -> Value {
    let value = values.first().and_then(to_number);
    match value {
        Some(n) => {
            let rounded = n.abs().ceil();
            let mut result = if rounded as i64 % 2 == 0 {
                rounded
            } else {
                rounded + 1.0
            };
            if n.is_sign_negative() {
                result = -result;
            }
            Value::Float(result)
        }
        None => Value::Error(ErrorValue::Value),
    }
}

/// ODD function - rounds up to nearest odd integer
pub fn odd(values: &[Value]) -> Value {
    let value = values.first().and_then(to_number);
    match value {
        Some(n) => {
            let rounded = n.abs().ceil();
            let mut result = if rounded as i64 % 2 != 0 {
                rounded
            } else {
                rounded + 1.0
            };
            if n.is_sign_negative() {
                result = -result;
            }
            Value::Float(result)
        }
        None => Value::Error(ErrorValue::Value),
    }
}

/// INT function - rounds down to integer
pub fn int(values: &[Value]) -> Value {
    let value = values.first().and_then(to_number);
    match value {
        Some(n) => Value::Float(n.floor()),
        None => Value::Error(ErrorValue::Value),
    }
}

/// TRUNC function - truncates number to integer
pub fn trunc(values: &[Value]) -> Value {
    let number = values.first().and_then(to_number);
    let places = values.get(1).and_then(to_number).unwrap_or(0.0);

    match number {
        Some(n) => {
            let places = places.floor() as i32;
            let multiplier = 10_f64.powi(places);
            Value::Float((n * multiplier).trunc() / multiplier)
        }
        None => Value::Error(ErrorValue::Value),
    }
}

/// SIGN function - returns sign of number (-1, 0, 1)
pub fn sign(values: &[Value]) -> Value {
    let value = values.first().and_then(to_number);
    match value {
        Some(n) if n > 0.0 => Value::Int(1),
        Some(n) if n < 0.0 => Value::Int(-1),
        Some(_) => Value::Int(0),
        None => Value::Error(ErrorValue::Value),
    }
}

/// RAND function - returns random number between 0 and 1
pub fn rand(_: &[Value]) -> Value {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    Value::Float(rng.gen_range(0.0..1.0))
}

/// RANDBETWEEN function - returns random integer between two numbers
pub fn randbetween(values: &[Value]) -> Value {
    use rand::Rng;
    let bottom = values.first().and_then(to_number);
    let top = values.get(1).and_then(to_number);

    match (bottom, top) {
        (Some(a), Some(b)) => {
            let min = a.floor() as i64;
            let max = b.floor() as i64;
            if min > max {
                Value::Error(ErrorValue::Value)
            } else {
                let mut rng = rand::thread_rng();
                Value::Int(rng.gen_range(min..=max))
            }
        }
        _ => Value::Error(ErrorValue::Value),
    }
}

/// PI function - returns value of pi
pub fn pi(_: &[Value]) -> Value {
    Value::Float(std::f64::consts::PI)
}

/// EXP function - returns e raised to a power
pub fn exp(values: &[Value]) -> Value {
    let value = values.first().and_then(to_number);
    match value {
        Some(n) => Value::Float(n.exp()),
        None => Value::Error(ErrorValue::Value),
    }
}

/// LN function - returns natural logarithm
pub fn ln(values: &[Value]) -> Value {
    let value = values.first().and_then(to_number);
    match value {
        Some(n) if n > 0.0 => Value::Float(n.ln()),
        Some(_) => Value::Error(ErrorValue::Num),
        None => Value::Error(ErrorValue::Value),
    }
}

/// LOG function - returns logarithm to specified base
pub fn log(values: &[Value]) -> Value {
    let number = values.first().and_then(to_number);
    let base = values.get(1).and_then(to_number).unwrap_or(10.0);

    match (number, base) {
        (Some(n), b) if n > 0.0 && b > 0.0 && b != 1.0 => Value::Float(n.log(b)),
        _ => Value::Error(ErrorValue::Num),
    }
}

/// LOG10 function - returns base-10 logarithm
pub fn log10(values: &[Value]) -> Value {
    let value = values.first().and_then(to_number);
    match value {
        Some(n) if n > 0.0 => Value::Float(n.log10()),
        Some(_) => Value::Error(ErrorValue::Num),
        None => Value::Error(ErrorValue::Value),
    }
}

/// FACT function - returns factorial
pub fn fact(values: &[Value]) -> Value {
    let value = values.first().and_then(to_number);
    match value {
        Some(n) if (0.0..=170.0).contains(&n) => {
            let n = n.floor() as u32;
            let result = (1..=n).fold(1.0, |acc, i| acc * i as f64);
            Value::Float(result)
        }
        Some(n) if n < 0.0 => Value::Error(ErrorValue::Num),
        Some(_) => Value::Error(ErrorValue::Num), // Too large
        None => Value::Error(ErrorValue::Value),
    }
}
