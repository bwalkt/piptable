//! Sheet manipulation built-in functions.

use crate::{formula, Interpreter};
use piptable_core::{PipError, PipResult, Value};
use piptable_sheet::{CellValue, CleanOptions, NullStrategy, ValidationRule};

/// Convert a CellValue to a Value
fn cell_to_value(cell: &CellValue) -> Value {
    match cell {
        CellValue::Null => Value::Null,
        CellValue::String(s) => Value::String(s.clone()),
        CellValue::Int(i) => Value::Int(*i),
        CellValue::Float(f) => Value::Float(*f),
        CellValue::Bool(b) => Value::Bool(*b),
        CellValue::Formula(formula) => match formula.cached.as_deref() {
            Some(cached) => cell_to_value(cached),
            None => Value::String(formula.source.clone()),
        },
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
    interpreter: &Interpreter,
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

        "sheet_remove_duplicates" => {
            if args.len() > 2 {
                return Some(Err(PipError::runtime(
                    line,
                    "sheet_remove_duplicates() takes 1 or 2 arguments (sheet, columns?)",
                )));
            }
            let columns: Vec<&str> = if args.len() == 2 {
                match &args[1] {
                    Value::String(name) => vec![name.as_str()],
                    Value::Array(values) => {
                        let mut names = Vec::with_capacity(values.len());
                        for value in values {
                            match value {
                                Value::String(s) => names.push(s.as_str()),
                                _ => {
                                    return Some(Err(PipError::runtime(
                                        line,
                                        "Column names must be strings",
                                    )))
                                }
                            }
                        }
                        names
                    }
                    _ => {
                        return Some(Err(PipError::runtime(
                            line,
                            "Second argument must be a column name or array of names",
                        )))
                    }
                }
            } else {
                Vec::new()
            };

            match &args[0] {
                Value::Sheet(sheet) => {
                    let mut new_sheet = sheet.clone();
                    match new_sheet.remove_duplicates_by_columns(&columns) {
                        Ok(_) => Some(Ok(Value::Sheet(new_sheet))),
                        Err(e) => Some(Err(PipError::runtime(
                            line,
                            format!("Failed to remove duplicates: {}", e),
                        ))),
                    }
                }
                _ => Some(Err(PipError::runtime(
                    line,
                    "First argument must be a sheet",
                ))),
            }
        }

        "sheet_validate_column" => {
            if args.len() < 3 || args.len() > 5 {
                return Some(Err(PipError::runtime(
                    line,
                    "sheet_validate_column() takes 3-5 arguments (sheet, column, rule, ...)",
                )));
            }
            let (Value::Sheet(sheet), Value::String(column), Value::String(rule)) =
                (&args[0], &args[1], &args[2])
            else {
                return Some(Err(PipError::runtime(
                    line,
                    "Arguments must be (sheet, column_name, rule)",
                )));
            };

            let validation_rule = match rule.as_str() {
                "email" => ValidationRule::Email,
                "phone" => ValidationRule::Phone,
                "range" => {
                    if args.len() < 5 {
                        return Some(Err(PipError::runtime(
                            line,
                            "range validation requires min and max arguments",
                        )));
                    }
                    let min = match &args[3] {
                        Value::Int(i) => *i as f64,
                        Value::Float(f) => *f,
                        _ => {
                            return Some(Err(PipError::runtime(line, "range min must be a number")))
                        }
                    };
                    let max = match &args[4] {
                        Value::Int(i) => *i as f64,
                        Value::Float(f) => *f,
                        _ => {
                            return Some(Err(PipError::runtime(line, "range max must be a number")))
                        }
                    };
                    ValidationRule::Range { min, max }
                }
                "regex" => {
                    if args.len() < 4 {
                        return Some(Err(PipError::runtime(
                            line,
                            "regex validation requires a pattern argument",
                        )));
                    }
                    match &args[3] {
                        Value::String(pattern) => ValidationRule::Regex(pattern.clone()),
                        _ => {
                            return Some(Err(PipError::runtime(
                                line,
                                "regex pattern must be a string",
                            )))
                        }
                    }
                }
                _ => {
                    return Some(Err(PipError::runtime(
                        line,
                        "Unknown rule. Supported: email, phone, range, regex",
                    )))
                }
            };

            match sheet.validate_column(column, validation_rule) {
                Ok(rows) => Some(Ok(Value::Array(
                    rows.into_iter().map(|r| Value::Int(r as i64)).collect(),
                ))),
                Err(e) => Some(Err(PipError::runtime(
                    line,
                    format!("Failed to validate column: {}", e),
                ))),
            }
        }

        "sheet_clean_data" => {
            if args.len() < 2 || args.len() > 3 {
                return Some(Err(PipError::runtime(
                    line,
                    "sheet_clean_data() takes 2-3 arguments (sheet, operations, fill_value?)",
                )));
            }

            let operations = match &args[1] {
                Value::String(op) => vec![op.clone()],
                Value::Array(values) => {
                    let mut ops = Vec::with_capacity(values.len());
                    for value in values {
                        match value {
                            Value::String(s) => ops.push(s.clone()),
                            _ => {
                                return Some(Err(PipError::runtime(
                                    line,
                                    "Operations must be strings",
                                )))
                            }
                        }
                    }
                    ops
                }
                _ => {
                    return Some(Err(PipError::runtime(
                        line,
                        "Operations must be a string or array of strings",
                    )))
                }
            };

            let mut options = CleanOptions::default();
            for op in operations {
                match op.as_str() {
                    "trim" => options.trim = true,
                    "lower" => options.lower = true,
                    "upper" => options.upper = true,
                    "normalize_whitespace" => options.normalize_whitespace = true,
                    "empty_to_null" => options.null_strategy = NullStrategy::EmptyToNull,
                    "null_to_empty" => options.null_strategy = NullStrategy::NullToEmpty,
                    "fill_nulls" => {
                        let fill_value = args.get(2).and_then(value_to_cell).ok_or_else(|| {
                            PipError::runtime(line, "fill_nulls requires a fill value")
                        });
                        match fill_value {
                            Ok(value) => options.null_strategy = NullStrategy::FillWith(value),
                            Err(err) => return Some(Err(err)),
                        }
                    }
                    _ => {
                        return Some(Err(PipError::runtime(
                            line,
                            format!(
                                "Unknown operation '{}'. Supported: trim, lower, upper, normalize_whitespace, empty_to_null, null_to_empty, fill_nulls",
                                op
                            ),
                        )))
                    }
                }
            }

            match &args[0] {
                Value::Sheet(sheet) => {
                    let mut new_sheet = sheet.clone();
                    match new_sheet.clean_data(&options) {
                        Ok(()) => Some(Ok(Value::Sheet(new_sheet))),
                        Err(e) => Some(Err(PipError::runtime(
                            line,
                            format!("Failed to clean data: {}", e),
                        ))),
                    }
                }
                _ => Some(Err(PipError::runtime(
                    line,
                    "First argument must be a sheet",
                ))),
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

        "sheet_get_cell" => {
            if args.len() != 2 {
                return Some(Err(PipError::runtime(
                    line,
                    "sheet_get_cell() takes exactly 2 arguments (sheet, notation)",
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

        "sheet_get_cell_value" => {
            if args.len() != 2 {
                return Some(Err(PipError::runtime(
                    line,
                    "sheet_get_cell_value() takes exactly 2 arguments (sheet, notation)",
                )));
            }
            match (&args[0], &args[1]) {
                (Value::Sheet(sheet), Value::String(notation)) => {
                    let mut engine = interpreter.formula_engine.lock().await;
                    Some(formula::eval_sheet_cell_cached(
                        &mut engine,
                        sheet,
                        notation,
                        line,
                    ))
                }
                _ => Some(Err(PipError::runtime(
                    line,
                    "Arguments must be (sheet, string)",
                ))),
            }
        }

        "is_sheet_cell_formula" => {
            if args.len() != 2 {
                return Some(Err(PipError::runtime(
                    line,
                    "is_sheet_cell_formula() takes exactly 2 arguments (sheet, notation)",
                )));
            }
            match (&args[0], &args[1]) {
                (Value::Sheet(sheet), Value::String(notation)) => match sheet.get_a1(notation) {
                    Ok(cell) => {
                        let is_formula = matches!(
                            cell,
                            CellValue::String(s) if s.trim_start().starts_with('=')
                        ) || matches!(cell, CellValue::Formula(_));
                        Some(Ok(Value::Bool(is_formula)))
                    }
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

        "sheet_get_a1_eval" => {
            if args.len() != 2 {
                return Some(Err(PipError::runtime(
                    line,
                    "sheet_get_a1_eval() takes exactly 2 arguments (sheet, notation)",
                )));
            }
            match (&args[0], &args[1]) {
                (Value::Sheet(sheet), Value::String(notation)) => {
                    let mut engine = interpreter.formula_engine.lock().await;
                    Some(formula::eval_sheet_cell_cached(
                        &mut engine,
                        sheet,
                        notation,
                        line,
                    ))
                }
                _ => Some(Err(PipError::runtime(
                    line,
                    "Arguments must be (sheet, string)",
                ))),
            }
        }

        "sheet_eval_formula" => {
            if args.len() != 2 {
                return Some(Err(PipError::runtime(
                    line,
                    "sheet_eval_formula() takes exactly 2 arguments (sheet, formula)",
                )));
            }
            match (&args[0], &args[1]) {
                (Value::Sheet(sheet), Value::String(formula_text)) => {
                    let mut engine = interpreter.formula_engine.lock().await;
                    Some(formula::eval_sheet_formula_cached(
                        &mut engine,
                        sheet,
                        formula_text,
                        line,
                        "sheet_eval_formula",
                    ))
                }
                _ => Some(Err(PipError::runtime(
                    line,
                    "Arguments must be (sheet, string)",
                ))),
            }
        }

        "sheet_set_formula" => {
            if args.len() != 3 {
                return Some(Err(PipError::runtime(
                    line,
                    "sheet_set_formula() takes exactly 3 arguments (sheet, notation, formula)",
                )));
            }
            match (&args[0], &args[1], &args[2]) {
                (Value::Sheet(sheet), Value::String(notation), Value::String(formula_text)) => {
                    let mut sheet_clone = sheet.clone();
                    match sheet_clone.set_formula(notation, formula_text) {
                        Ok(()) => Some(Ok(Value::Sheet(sheet_clone))),
                        Err(e) => Some(Err(PipError::runtime(
                            line,
                            format!("Failed to set formula '{}': {}", notation, e),
                        ))),
                    }
                }
                _ => Some(Err(PipError::runtime(
                    line,
                    "Arguments must be (sheet, string, string)",
                ))),
            }
        }

        "sheet_evaluate_formulas" => {
            if args.len() != 1 {
                return Some(Err(PipError::runtime(
                    line,
                    "sheet_evaluate_formulas() takes exactly 1 argument (sheet)",
                )));
            }
            match &args[0] {
                Value::Sheet(sheet) => {
                    let mut sheet_clone = sheet.clone();
                    match sheet_clone.evaluate_formulas() {
                        Ok(()) => Some(Ok(Value::Sheet(sheet_clone))),
                        Err(e) => Some(Err(PipError::runtime(
                            line,
                            format!("Failed to evaluate formulas: {}", e),
                        ))),
                    }
                }
                _ => Some(Err(PipError::runtime(line, "Arguments must be (sheet)"))),
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
                    Ok(sub_sheet) => Some(Ok(Value::Sheet(Box::new(sub_sheet)))),
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

        "sheet_map" => {
            // Simple version: map all cells to uppercase if they're strings
            if args.len() != 2 {
                return Some(Err(PipError::runtime(
                    line,
                    "sheet_map() takes exactly 2 arguments (sheet, operation)",
                )));
            }
            match (&args[0], &args[1]) {
                (Value::Sheet(sheet), Value::String(operation)) => {
                    let mut new_sheet = sheet.clone();
                    match operation.as_str() {
                        "upper" => {
                            new_sheet.map(|cell| {
                                if let CellValue::String(s) = cell {
                                    CellValue::String(s.to_uppercase())
                                } else {
                                    cell.clone()
                                }
                            });
                            Some(Ok(Value::Sheet(new_sheet)))
                        }
                        "lower" => {
                            new_sheet.map(|cell| {
                                if let CellValue::String(s) = cell {
                                    CellValue::String(s.to_lowercase())
                                } else {
                                    cell.clone()
                                }
                            });
                            Some(Ok(Value::Sheet(new_sheet)))
                        }
                        "trim" => {
                            new_sheet.map(|cell| {
                                if let CellValue::String(s) = cell {
                                    CellValue::String(s.trim().to_string())
                                } else {
                                    cell.clone()
                                }
                            });
                            Some(Ok(Value::Sheet(new_sheet)))
                        }
                        _ => Some(Err(PipError::runtime(
                            line,
                            format!(
                                "Unknown operation '{}'. Supported: upper, lower, trim",
                                operation
                            ),
                        ))),
                    }
                }
                _ => Some(Err(PipError::runtime(
                    line,
                    "Arguments must be (sheet, string_operation)",
                ))),
            }
        }

        "sheet_filter_rows" => {
            // Simple version: filter rows where a specific column matches a value
            if args.len() != 3 {
                return Some(Err(PipError::runtime(
                    line,
                    "sheet_filter_rows() takes exactly 3 arguments (sheet, column_name, value)",
                )));
            }
            match (&args[0], &args[1], &args[2]) {
                (Value::Sheet(sheet), Value::String(col_name), filter_value) => {
                    // Find the column index
                    let Some(col_names) = sheet.column_names() else {
                        return Some(Err(PipError::runtime(
                            line,
                            "Sheet must have named columns for filtering",
                        )));
                    };
                    let Some(col_idx) = col_names.iter().position(|n| n == col_name) else {
                        return Some(Err(PipError::runtime(
                            line,
                            format!("Column '{}' not found", col_name),
                        )));
                    };

                    let mut new_sheet = sheet.clone();
                    let Some(filter_cell) = value_to_cell(filter_value) else {
                        return Some(Err(PipError::runtime(line, "Invalid filter value type")));
                    };

                    new_sheet.filter_rows(|_row_idx, row| {
                        if col_idx < row.len() {
                            row[col_idx] == filter_cell
                        } else {
                            false
                        }
                    });

                    Some(Ok(Value::Sheet(new_sheet)))
                }
                _ => Some(Err(PipError::runtime(
                    line,
                    "Arguments must be (sheet, column_name, filter_value)",
                ))),
            }
        }

        _ => None,
    }
}
