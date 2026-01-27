//! Core built-in functions like print, len, type, etc.

use crate::{converters, Interpreter};
use piptable_core::{PipError, PipResult, Value};

/// Handle core built-in functions.
pub async fn call_core_builtin(
    interpreter: &Interpreter,
    name: &str,
    args: Vec<Value>,
    line: usize,
) -> Option<PipResult<Value>> {
    match name {
        "print" => {
            let output: Vec<String> = args
                .iter()
                .map(|v| converters::value_to_string(v))
                .collect();
            let msg = output.join(" ");
            interpreter.print(&msg).await;
            Some(Ok(Value::Null))
        }

        "len" | "length" => {
            if args.len() != 1 {
                return Some(Err(PipError::runtime(
                    line,
                    "len() takes exactly 1 argument",
                )));
            }
            let result = match &args[0] {
                Value::String(s) => Value::Int(s.len() as i64),
                Value::Array(arr) => Value::Int(arr.len() as i64),
                Value::Object(obj) => Value::Int(obj.len() as i64),
                Value::Sheet(sheet) => Value::Int(sheet.row_count() as i64),
                Value::Table(batches) => {
                    let total: usize = batches.iter().map(|b| b.num_rows()).sum();
                    Value::Int(total as i64)
                }
                _ => {
                    return Some(Err(PipError::runtime(
                        line,
                        format!("len() not supported for type: {}", args[0].type_name()),
                    )));
                }
            };
            Some(Ok(result))
        }

        "type" => {
            if args.len() != 1 {
                return Some(Err(PipError::runtime(
                    line,
                    "type() takes exactly 1 argument",
                )));
            }
            Some(Ok(Value::String(args[0].type_name().to_string())))
        }

        "keys" => {
            if args.len() != 1 {
                return Some(Err(PipError::runtime(
                    line,
                    "keys() takes exactly 1 argument",
                )));
            }
            match &args[0] {
                Value::Object(obj) => {
                    let keys: Vec<Value> = obj.keys().map(|k| Value::String(k.clone())).collect();
                    Some(Ok(Value::Array(keys)))
                }
                _ => Some(Err(PipError::runtime(
                    line,
                    format!("keys() expects object, got {}", args[0].type_name()),
                ))),
            }
        }

        "values" => {
            if args.len() != 1 {
                return Some(Err(PipError::runtime(
                    line,
                    "values() takes exactly 1 argument",
                )));
            }
            match &args[0] {
                Value::Object(obj) => {
                    let values: Vec<Value> = obj.values().cloned().collect();
                    Some(Ok(Value::Array(values)))
                }
                _ => Some(Err(PipError::runtime(
                    line,
                    format!("values() expects object, got {}", args[0].type_name()),
                ))),
            }
        }

        _ => None,
    }
}
