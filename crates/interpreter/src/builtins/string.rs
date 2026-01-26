//! String manipulation built-in functions.

use crate::{converters, Interpreter};
use piptable_core::{PipError, PipResult, Value};

/// Handle string manipulation built-in functions.
pub async fn call_string_builtin(
    _interpreter: &Interpreter,
    name: &str,
    args: Vec<Value>,
    line: usize,
) -> Option<PipResult<Value>> {
    match name {
        "str" => {
            if args.len() != 1 {
                return Some(Err(PipError::runtime(
                    line,
                    "str() takes exactly 1 argument",
                )));
            }
            Some(Ok(Value::String(converters::value_to_string(&args[0]))))
        }

        "int" => {
            if args.len() != 1 {
                return Some(Err(PipError::runtime(
                    line,
                    "int() takes exactly 1 argument",
                )));
            }
            let result = match &args[0] {
                Value::Int(n) => Value::Int(*n),
                Value::Float(f) => Value::Int(*f as i64),
                Value::String(s) => match s.parse::<i64>() {
                    Ok(n) => Value::Int(n),
                    Err(_) => {
                        return Some(Err(PipError::runtime(
                            line,
                            format!("Cannot convert '{}' to integer", s),
                        )));
                    }
                },
                Value::Bool(b) => Value::Int(if *b { 1 } else { 0 }),
                _ => {
                    return Some(Err(PipError::runtime(
                        line,
                        format!("Cannot convert {} to integer", args[0].type_name()),
                    )));
                }
            };
            Some(Ok(result))
        }

        "float" => {
            if args.len() != 1 {
                return Some(Err(PipError::runtime(
                    line,
                    "float() takes exactly 1 argument",
                )));
            }
            let result = match &args[0] {
                Value::Int(n) => Value::Float(*n as f64),
                Value::Float(f) => Value::Float(*f),
                Value::String(s) => match s.parse::<f64>() {
                    Ok(f) => Value::Float(f),
                    Err(_) => {
                        return Some(Err(PipError::runtime(
                            line,
                            format!("Cannot convert '{}' to float", s),
                        )));
                    }
                },
                _ => {
                    return Some(Err(PipError::runtime(
                        line,
                        format!("Cannot convert {} to float", args[0].type_name()),
                    )));
                }
            };
            Some(Ok(result))
        }

        _ => None,
    }
}
