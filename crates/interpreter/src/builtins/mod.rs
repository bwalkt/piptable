//! Built-in functions for the piptable interpreter.

mod core;
mod math;
mod sheet;
mod string;

use crate::Interpreter;
use piptable_core::{PipResult, Value};

/// Execute a built-in function.
///
/// Returns `None` if the function is not a built-in, allowing the interpreter
/// to check for user-defined functions.
pub async fn call_builtin(
    interpreter: &Interpreter,
    name: &str,
    args: Vec<Value>,
    line: usize,
) -> Option<PipResult<Value>> {
    let builtin_name = name.to_lowercase();

    // Try each category of built-ins
    if let Some(result) =
        core::call_core_builtin(interpreter, &builtin_name, args.clone(), line).await
    {
        return Some(result);
    }

    if let Some(result) =
        math::call_math_builtin(interpreter, &builtin_name, args.clone(), line).await
    {
        return Some(result);
    }

    if let Some(result) =
        string::call_string_builtin(interpreter, &builtin_name, args.clone(), line).await
    {
        return Some(result);
    }

    if let Some(result) = sheet::call_sheet_builtin(interpreter, &builtin_name, args, line).await {
        return Some(result);
    }

    None
}
