//! Built-in functions for the piptable interpreter.

/// Array-related built-in functions.
mod array;
/// Core interpreter built-in functions.
mod core;
/// Math-related built-in functions.
mod math;
/// Sheet-related built-in functions.
mod sheet;
/// String-related built-in functions.
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

    if let Some(result) =
        sheet::call_sheet_builtin(interpreter, &builtin_name, args.clone(), line).await
    {
        return Some(result);
    }

    if let Some(result) = array::call_array_builtin(interpreter, &builtin_name, args, line).await {
        return Some(result);
    }

    None
}

pub fn is_builtin(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        // core
        "print" | "len" | "length" | "type" | "keys" | "values"
            // math
            | "abs" | "sum" | "min" | "max" | "avg" | "average"
            // string
            | "str" | "int" | "float"
            // sheet
            | "sheet_name_columns_by_row"
            | "sheet_name_rows_by_column"
            | "sheet_transpose"
            | "sheet_select_columns"
            | "sheet_remove_columns"
            | "sheet_remove_empty_rows"
            | "sheet_remove_duplicates"
            | "sheet_validate_column"
            | "sheet_clean_data"
            | "sheet_row_count"
            | "sheet_col_count"
            | "sheet_get_a1"
            | "sheet_get_a1_eval"
            | "sheet_get_cell"
            | "sheet_get_cell_value"
            | "is_sheet_cell_formula"
            | "sheet_eval_formula"
            | "sheet_set_formula"
            | "sheet_evaluate_formulas"
            | "sheet_set_a1"
            | "sheet_get_range"
            | "sheet_column_by_name"
            | "sheet_get_by_name"
            | "sheet_set_by_name"
            | "sheet_set_column_by_name"
            | "sheet_set_row_by_name"
            | "sheet_map"
            | "sheet_filter_rows"
            // array
            | "filter"
    )
}
