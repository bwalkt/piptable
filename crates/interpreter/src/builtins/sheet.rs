//! Sheet manipulation built-in functions.

use crate::Interpreter;
use piptable_core::{PipError, PipResult, Value};
use piptable_sheet::CellValue;

/// Convert a CellValue to a Value
fn cell_to_value(cell: &CellValue) -> Value {
    match cell {
        CellValue::Null => Value::Null,
        CellValue::String(s) => Value::String(s.clone()),
        CellValue::Int(i) => Value::Int(*i),
        CellValue::Float(f) => Value::Float(*f),
        CellValue::Bool(b) => Value::Bool(*b),
    }
}

/// Convert a Value to a CellValue
fn value_to_cell(value: &Value) -> Option<CellValue> {
    match value {
        Value::String(s) => Some(CellValue::String(s.clone())),
        Value::Int(i) => Some(CellValue::Int(*i)),
        Value::Float(f) => Some(CellValue::Float(*f)),
        Value::Bool(b) => Some(CellValue::Bool(*b)),
        Value::Null => Some(CellValue::Null),
        _ => None,
    }
}

/// Handle sheet manipulation built-in functions.
pub async fn call_sheet_builtin(
    _interpreter: &Interpreter,
    name: &str,
    args: Vec<Value>,
    line: usize,
) -> Option<PipResult<Value>> {
    match name {
        "sheet_name_columns_by_row" => {
            if args.len() != 2 {
                return Some(Err(PipError::runtime(
                    line,
                    "sheet_name_columns_by_row() takes exactly 2 arguments",
                )));
            }
            match (&args[0], &args[1]) {
                (Value::Sheet(sheet), Value::Int(row_idx)) => {
                    if *row_idx < 0 {
                        return Some(Err(PipError::runtime(line, "Row index cannot be negative")));
                    }
                    let mut new_sheet = sheet.clone();
                    match new_sheet.name_columns_by_row(*row_idx as usize) {
                        Ok(()) => Some(Ok(Value::Sheet(new_sheet))),
                        Err(e) => Some(Err(PipError::runtime(
                            line,
                            format!("Failed to name columns: {}", e),
                        ))),
                    }
                }
                (Value::Sheet(_), _) => {
                    Some(Err(PipError::runtime(line, "Row index must be an integer")))
                }
                _ => Some(Err(PipError::runtime(
                    line,
                    "First argument must be a sheet",
                ))),
            }
        }

        "sheet_transpose" => {
            if args.len() != 1 {
                return Some(Err(PipError::runtime(
                    line,
                    "sheet_transpose() takes exactly 1 argument",
                )));
            }
            match &args[0] {
                Value::Sheet(sheet) => {
                    let mut new_sheet = sheet.clone();
                    new_sheet.transpose();
                    Some(Ok(Value::Sheet(new_sheet)))
                }
                _ => Some(Err(PipError::runtime(line, "Argument must be a sheet"))),
            }
        }

        "sheet_select_columns" => {
            if args.len() != 2 {
                return Some(Err(PipError::runtime(
                    line,
                    "sheet_select_columns() takes exactly 2 arguments",
                )));
            }
            match (&args[0], &args[1]) {
                (Value::Sheet(sheet), Value::Array(columns)) => {
                    let mut new_sheet = sheet.clone();
                    let column_names: Result<Vec<&str>, _> = columns
                        .iter()
                        .map(|v| match v {
                            Value::String(s) => Ok(s.as_str()),
                            _ => Err(PipError::runtime(line, "Column names must be strings")),
                        })
                        .collect();
                    match column_names {
                        Ok(names) => match new_sheet.select_columns(&names) {
                            Ok(()) => Some(Ok(Value::Sheet(new_sheet))),
                            Err(e) => Some(Err(PipError::runtime(
                                line,
                                format!("Failed to select columns: {}", e),
                            ))),
                        },
                        Err(e) => Some(Err(e)),
                    }
                }
                (Value::Sheet(_), _) => Some(Err(PipError::runtime(
                    line,
                    "Second argument must be an array of column names",
                ))),
                _ => Some(Err(PipError::runtime(
                    line,
                    "First argument must be a sheet",
                ))),
            }
        }

        "sheet_remove_columns" => {
            if args.len() != 2 {
                return Some(Err(PipError::runtime(
                    line,
                    "sheet_remove_columns() takes exactly 2 arguments",
                )));
            }
            match (&args[0], &args[1]) {
                (Value::Sheet(sheet), Value::Array(columns)) => {
                    let mut new_sheet = sheet.clone();
                    let column_names: Result<Vec<&str>, _> = columns
                        .iter()
                        .map(|v| match v {
                            Value::String(s) => Ok(s.as_str()),
                            _ => Err(PipError::runtime(line, "Column names must be strings")),
                        })
                        .collect();
                    match column_names {
                        Ok(names) => match new_sheet.remove_columns(&names) {
                            Ok(()) => Some(Ok(Value::Sheet(new_sheet))),
                            Err(e) => Some(Err(PipError::runtime(
                                line,
                                format!("Failed to remove columns: {}", e),
                            ))),
                        },
                        Err(e) => Some(Err(e)),
                    }
                }
                (Value::Sheet(_), _) => Some(Err(PipError::runtime(
                    line,
                    "Second argument must be an array of column names",
                ))),
                _ => Some(Err(PipError::runtime(
                    line,
                    "First argument must be a sheet",
                ))),
            }
        }

        "sheet_remove_empty_rows" => {
            if args.len() != 1 {
                return Some(Err(PipError::runtime(
                    line,
                    "sheet_remove_empty_rows() takes exactly 1 argument",
                )));
            }
            match &args[0] {
                Value::Sheet(sheet) => {
                    let mut new_sheet = sheet.clone();
                    new_sheet.remove_empty_rows();
                    Some(Ok(Value::Sheet(new_sheet)))
                }
                _ => Some(Err(PipError::runtime(line, "Argument must be a sheet"))),
            }
        }

        "sheet_row_count" => {
            if args.len() != 1 {
                return Some(Err(PipError::runtime(
                    line,
                    "sheet_row_count() takes exactly 1 argument",
                )));
            }
            match &args[0] {
                Value::Sheet(sheet) => Some(Ok(Value::Int(sheet.row_count() as i64))),
                _ => Some(Err(PipError::runtime(line, "Argument must be a sheet"))),
            }
        }

        "sheet_col_count" => {
            if args.len() != 1 {
                return Some(Err(PipError::runtime(
                    line,
                    "sheet_col_count() takes exactly 1 argument",
                )));
            }
            match &args[0] {
                Value::Sheet(sheet) => Some(Ok(Value::Int(sheet.col_count() as i64))),
                _ => Some(Err(PipError::runtime(line, "Argument must be a sheet"))),
            }
        }

        "sheet_get_a1" => {
            if args.len() != 2 {
                return Some(Err(PipError::runtime(
                    line,
                    "sheet_get_a1() takes exactly 2 arguments (sheet, notation)",
                )));
            }
            match (&args[0], &args[1]) {
                (Value::Sheet(sheet), Value::String(notation)) => match sheet.get_a1(notation) {
                    Ok(cell) => Some(Ok(cell_to_value(cell))),
                    Err(e) => Some(Err(PipError::runtime(
                        line,
                        format!("Invalid cell notation '{}': {}", notation, e),
                    ))),
                },
                _ => Some(Err(PipError::runtime(
                    line,
                    "Arguments must be (sheet, string)",
                ))),
            }
        }

        "sheet_set_a1" => {
            if args.len() != 3 {
                return Some(Err(PipError::runtime(
                    line,
                    "sheet_set_a1() takes exactly 3 arguments (sheet, notation, value)",
                )));
            }
            match (&args[0], &args[1], &args[2]) {
                (Value::Sheet(sheet), Value::String(notation), value) => {
                    let mut sheet_clone = sheet.clone();

                    if let Some(cell_value) = value_to_cell(value) {
                        match sheet_clone.set_a1(notation, cell_value) {
                            Ok(()) => Some(Ok(Value::Sheet(sheet_clone))),
                            Err(e) => Some(Err(PipError::runtime(
                                line,
                                format!("Failed to set cell '{}': {}", notation, e),
                            ))),
                        }
                    } else {
                        Some(Err(PipError::runtime(
                            line,
                            "Unsupported value type for cell",
                        )))
                    }
                }
                _ => Some(Err(PipError::runtime(
                    line,
                    "Arguments must be (sheet, string, value)",
                ))),
            }
        }

        "sheet_get_range" => {
            if args.len() != 2 {
                return Some(Err(PipError::runtime(
                    line,
                    "sheet_get_range() takes exactly 2 arguments (sheet, range_notation)",
                )));
            }
            match (&args[0], &args[1]) {
                (Value::Sheet(sheet), Value::String(notation)) => match sheet.get_range(notation) {
                    Ok(sub_sheet) => Some(Ok(Value::Sheet(sub_sheet))),
                    Err(e) => Some(Err(PipError::runtime(
                        line,
                        format!("Invalid range '{}': {}", notation, e),
                    ))),
                },
                _ => Some(Err(PipError::runtime(
                    line,
                    "Arguments must be (sheet, string)",
                ))),
            }
        }

        "sheet_column_by_name" => {
            if args.len() != 2 {
                return Some(Err(PipError::runtime(
                    line,
                    "sheet_column_by_name() takes exactly 2 arguments (sheet, column_name)",
                )));
            }
            match (&args[0], &args[1]) {
                (Value::Sheet(sheet), Value::String(col_name)) => {
                    match sheet.column_by_name(col_name) {
                        Ok(column) => {
                            let array: Vec<Value> = column.iter().map(cell_to_value).collect();
                            Some(Ok(Value::Array(array)))
                        }
                        Err(e) => Some(Err(PipError::runtime(
                            line,
                            format!("Failed to get column '{}': {}", col_name, e),
                        ))),
                    }
                }
                _ => Some(Err(PipError::runtime(
                    line,
                    "Arguments must be (sheet, string)",
                ))),
            }
        }

        "sheet_get_by_name" => {
            if args.len() != 3 {
                return Some(Err(PipError::runtime(
                    line,
                    "sheet_get_by_name() takes exactly 3 arguments (sheet, row_index, column_name)",
                )));
            }
            match (&args[0], &args[1], &args[2]) {
                (Value::Sheet(sheet), Value::Int(row), Value::String(col_name)) => {
                    if *row < 0 {
                        return Some(Err(PipError::runtime(line, "Row index cannot be negative")));
                    }
                    match sheet.get_by_name(*row as usize, col_name) {
                        Ok(cell) => Some(Ok(cell_to_value(cell))),
                        Err(e) => Some(Err(PipError::runtime(
                            line,
                            format!(
                                "Failed to get cell at row {} column '{}': {}",
                                row, col_name, e
                            ),
                        ))),
                    }
                }
                _ => Some(Err(PipError::runtime(
                    line,
                    "Arguments must be (sheet, int, string)",
                ))),
            }
        }

        "sheet_set_by_name" => {
            if args.len() != 4 {
                return Some(Err(PipError::runtime(line, "sheet_set_by_name() takes exactly 4 arguments (sheet, row_index, column_name, value)")));
            }
            match (&args[0], &args[1], &args[2], &args[3]) {
                (Value::Sheet(sheet), Value::Int(row), Value::String(col_name), value) => {
                    if *row < 0 {
                        return Some(Err(PipError::runtime(line, "Row index cannot be negative")));
                    }
                    let mut sheet_clone = sheet.clone();

                    if let Some(cell_value) = value_to_cell(value) {
                        match sheet_clone.set_by_name(*row as usize, col_name, cell_value) {
                            Ok(()) => Some(Ok(Value::Sheet(sheet_clone))),
                            Err(e) => Some(Err(PipError::runtime(
                                line,
                                format!(
                                    "Failed to set cell at row {} column '{}': {}",
                                    row, col_name, e
                                ),
                            ))),
                        }
                    } else {
                        Some(Err(PipError::runtime(
                            line,
                            "Unsupported value type for cell",
                        )))
                    }
                }
                _ => Some(Err(PipError::runtime(
                    line,
                    "Arguments must be (sheet, int, string, value)",
                ))),
            }
        }

        _ => None,
    }
}
