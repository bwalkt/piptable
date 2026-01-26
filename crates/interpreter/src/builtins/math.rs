//! Mathematical built-in functions.

use crate::Interpreter;
use piptable_core::{PipError, PipResult, Value};

/// Handle mathematical built-in functions.
pub async fn call_math_builtin(
    _interpreter: &Interpreter,
    name: &str,
    args: Vec<Value>,
    line: usize,
) -> Option<PipResult<Value>> {
    match name {
        "abs" => {
            if args.len() != 1 {
                return Some(Err(PipError::runtime(
                    line,
                    "abs() takes exactly 1 argument",
                )));
            }
            let result = match &args[0] {
                Value::Int(n) => Value::Int(n.abs()),
                Value::Float(f) => Value::Float(f.abs()),
                _ => {
                    return Some(Err(PipError::runtime(
                        line,
                        format!("abs() expects number, got {}", args[0].type_name()),
                    )));
                }
            };
            Some(Ok(result))
        }

        "sum" => {
            if args.len() != 1 {
                return Some(Err(PipError::runtime(
                    line,
                    "sum() takes exactly 1 argument",
                )));
            }
            match &args[0] {
                Value::Array(arr) => {
                    let mut sum_int: i64 = 0;
                    let mut sum_float: f64 = 0.0;
                    let mut has_float = false;
                    for val in arr {
                        match val {
                            Value::Int(n) => {
                                if has_float {
                                    sum_float += *n as f64;
                                } else {
                                    sum_int = sum_int.saturating_add(*n);
                                }
                            }
                            Value::Float(f) => {
                                if !has_float {
                                    sum_float = sum_int as f64;
                                    has_float = true;
                                }
                                sum_float += f;
                            }
                            _ => {
                                return Some(Err(PipError::runtime(
                                    line,
                                    format!("sum() found non-numeric value: {}", val.type_name()),
                                )));
                            }
                        }
                    }
                    Some(Ok(if has_float {
                        Value::Float(sum_float)
                    } else {
                        Value::Int(sum_int)
                    }))
                }
                _ => Some(Err(PipError::runtime(
                    line,
                    format!("sum() expects array, got {}", args[0].type_name()),
                ))),
            }
        }

        "avg" | "average" => {
            if args.len() != 1 {
                return Some(Err(PipError::runtime(
                    line,
                    "avg() takes exactly 1 argument",
                )));
            }
            match &args[0] {
                Value::Array(arr) if !arr.is_empty() => {
                    let mut sum: f64 = 0.0;
                    for val in arr {
                        match val {
                            Value::Int(n) => sum += *n as f64,
                            Value::Float(f) => sum += f,
                            _ => {
                                return Some(Err(PipError::runtime(
                                    line,
                                    format!("avg() found non-numeric value: {}", val.type_name()),
                                )));
                            }
                        }
                    }
                    Some(Ok(Value::Float(sum / arr.len() as f64)))
                }
                Value::Array(_) => Some(Err(PipError::runtime(
                    line,
                    "avg() cannot average empty array",
                ))),
                _ => Some(Err(PipError::runtime(
                    line,
                    format!("avg() expects array, got {}", args[0].type_name()),
                ))),
            }
        }

        "min" => {
            if args.is_empty() {
                return Some(Err(PipError::runtime(
                    line,
                    "min() requires at least 1 argument",
                )));
            }

            // Handle both forms: min(array) or min(a, b, c, ...)
            let values = if args.len() == 1 {
                match &args[0] {
                    Value::Array(arr) => arr.clone(),
                    _ => args.to_vec(),
                }
            } else {
                args.to_vec()
            };

            if values.is_empty() {
                return Some(Err(PipError::runtime(
                    line,
                    "min() cannot find min of empty array",
                )));
            }

            let mut min_val = values[0].clone();
            for val in values.iter().skip(1) {
                match (&min_val, val) {
                    (Value::Int(a), Value::Int(b)) if b < a => min_val = val.clone(),
                    (Value::Float(a), Value::Float(b)) if b < a => min_val = val.clone(),
                    (Value::Int(a), Value::Float(b)) if *b < *a as f64 => {
                        min_val = val.clone();
                    }
                    (Value::Float(a), Value::Int(b)) if (*b as f64) < *a => {
                        min_val = val.clone();
                    }
                    (Value::Int(_) | Value::Float(_), Value::Int(_) | Value::Float(_)) => {}
                    _ => {
                        return Some(Err(PipError::runtime(
                            line,
                            "min() requires all numeric values",
                        )));
                    }
                }
            }
            Some(Ok(min_val))
        }

        "max" => {
            if args.is_empty() {
                return Some(Err(PipError::runtime(
                    line,
                    "max() requires at least 1 argument",
                )));
            }

            // Handle both forms: max(array) or max(a, b, c, ...)
            let values = if args.len() == 1 {
                match &args[0] {
                    Value::Array(arr) => arr.clone(),
                    _ => args.to_vec(),
                }
            } else {
                args.to_vec()
            };

            if values.is_empty() {
                return Some(Err(PipError::runtime(
                    line,
                    "max() cannot find max of empty array",
                )));
            }

            let mut max_val = values[0].clone();
            for val in values.iter().skip(1) {
                match (&max_val, val) {
                    (Value::Int(a), Value::Int(b)) if b > a => max_val = val.clone(),
                    (Value::Float(a), Value::Float(b)) if b > a => max_val = val.clone(),
                    (Value::Int(a), Value::Float(b)) if *b > *a as f64 => {
                        max_val = val.clone();
                    }
                    (Value::Float(a), Value::Int(b)) if (*b as f64) > *a => {
                        max_val = val.clone();
                    }
                    (Value::Int(_) | Value::Float(_), Value::Int(_) | Value::Float(_)) => {}
                    _ => {
                        return Some(Err(PipError::runtime(
                            line,
                            "max() requires all numeric values",
                        )));
                    }
                }
            }
            Some(Ok(max_val))
        }

        _ => None,
    }
}
