//! Mathematical built-in functions.

use crate::Interpreter;
use piptable_core::{PipError, PipResult, Value};
use piptable_primitives::Value as FormulaValue;
use piptable_utils::math as shared_math;

fn core_to_formula_value(value: &Value) -> Option<FormulaValue> {
    match value {
        Value::Null => Some(FormulaValue::Empty),
        Value::Bool(b) => Some(FormulaValue::Bool(*b)),
        Value::Int(i) => Some(FormulaValue::Int(*i)),
        Value::Float(f) => Some(FormulaValue::Float(*f)),
        Value::String(s) => Some(FormulaValue::String(s.clone())),
        Value::Array(items) => {
            let mut converted = Vec::with_capacity(items.len());
            for item in items {
                converted.push(core_to_formula_value(item)?);
            }
            Some(FormulaValue::Array(converted))
        }
        _ => None,
    }
}

fn formula_to_core_value(value: FormulaValue) -> Value {
    match value {
        FormulaValue::Empty => Value::Null,
        FormulaValue::Bool(b) => Value::Bool(b),
        FormulaValue::Int(i) => Value::Int(i),
        FormulaValue::Float(f) => Value::Float(f),
        FormulaValue::String(s) => Value::String(s),
        FormulaValue::Error(err) => Value::String(format!("#{:?}!", err)),
        FormulaValue::Array(items) => {
            Value::Array(items.into_iter().map(formula_to_core_value).collect())
        }
    }
}

fn formula_result_to_core(
    result: FormulaValue,
    line: usize,
    func: &str,
    empty_message: Option<&'static str>,
) -> PipResult<Value> {
    match result {
        FormulaValue::Error(err) => {
            if let Some(message) = empty_message {
                return Err(PipError::runtime(line, message));
            }
            Err(PipError::runtime(
                line,
                format!("{func}() returned error: {err:?}"),
            ))
        }
        other => Ok(formula_to_core_value(other)),
    }
}

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
            let Some(formula_val) = core_to_formula_value(&args[0]) else {
                return Some(Err(PipError::runtime(
                    line,
                    format!("sum() expects array, got {}", args[0].type_name()),
                )));
            };
            let result = shared_math::sum(&[formula_val]);
            Some(formula_result_to_core(result, line, "sum", None))
        }

        "avg" | "average" => {
            if args.len() != 1 {
                return Some(Err(PipError::runtime(
                    line,
                    "avg() takes exactly 1 argument",
                )));
            }
            let Some(formula_val) = core_to_formula_value(&args[0]) else {
                return Some(Err(PipError::runtime(
                    line,
                    format!("avg() expects array, got {}", args[0].type_name()),
                )));
            };
            let result = shared_math::average(&[formula_val]);
            Some(formula_result_to_core(
                result,
                line,
                "avg",
                Some("avg() cannot average empty array"),
            ))
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

            let mut converted = Vec::with_capacity(values.len());
            for value in values {
                let Some(formula_val) = core_to_formula_value(&value) else {
                    return Some(Err(PipError::runtime(
                        line,
                        "min() requires numeric values",
                    )));
                };
                converted.push(formula_val);
            }
            let result = shared_math::min(&converted);
            Some(formula_result_to_core(
                result,
                line,
                "min",
                Some("min() cannot find min of empty array"),
            ))
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

            let mut converted = Vec::with_capacity(values.len());
            for value in values {
                let Some(formula_val) = core_to_formula_value(&value) else {
                    return Some(Err(PipError::runtime(
                        line,
                        "max() requires numeric values",
                    )));
                };
                converted.push(formula_val);
            }
            let result = shared_math::max(&converted);
            Some(formula_result_to_core(
                result,
                line,
                "max",
                Some("max() cannot find max of empty array"),
            ))
        }

        _ => None,
    }
}
