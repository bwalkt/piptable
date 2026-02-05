//! Utilities for converting between Book and Value representations.

use crate::sheet_conversions::{cell_to_value, value_to_cell, value_to_sheet};
use piptable_core::{PipError, PipResult, Value};
use piptable_sheet::{Book, CellValue, ConsolidateOptions, FileLoadOptions, Sheet};
use std::collections::HashMap;
use std::ptr;

/// Convert a Value into a Sheet for Book operations.
pub fn value_to_sheet_for_book(value: &Value) -> Result<Sheet, String> {
    match value {
        Value::Sheet(sheet) => Ok(*sheet.clone()),
        Value::Array(rows) => {
            let array_rows = rows
                .iter()
                .filter(|row| matches!(row, Value::Array(_)))
                .count();
            if array_rows == rows.len() {
                let data: Vec<Vec<CellValue>> = rows
                    .iter()
                    .map(|row| {
                        let Value::Array(cells) = row else {
                            return Vec::new();
                        };
                        cells.iter().map(value_to_cell).collect()
                    })
                    .collect();
                Ok(Sheet::from_data(data))
            } else if array_rows > 0 {
                Err("value_to_sheet_for_book: mixed row types (Value::Array and non-array rows) are not supported"
                    .to_string())
            } else {
                value_to_sheet(value)
            }
        }
        _ => Err(format!(
            "Expected Sheet or array data, got {}",
            value.type_name()
        )),
    }
}

/// Convert a Book into a Value::Object of sheet-name -> 2D array data.
pub fn book_to_value_dict(book: &Book) -> Value {
    let mut map: HashMap<String, Value> = HashMap::new();
    for (name, sheet) in book.sheets() {
        let rows: Vec<Value> = sheet
            .data()
            .iter()
            .map(|row| {
                let values: Vec<Value> = row.iter().cloned().map(cell_to_value).collect();
                Value::Array(values)
            })
            .collect();
        map.insert(name.to_string(), Value::Array(rows));
    }
    Value::Object(map)
}

/// Resolve the active sheet name for a book, if any.
pub fn active_sheet_name(book: &Book) -> Option<String> {
    let active = book.active_sheet()?;
    for (name, sheet) in book.sheets() {
        if ptr::eq(sheet, active) {
            return Some(name.to_string());
        }
    }
    None
}

/// Parse consolidate options from a DSL object value.
pub fn consolidate_options_from_value(
    value: Option<&Value>,
    line: usize,
) -> PipResult<ConsolidateOptions> {
    let Some(value) = value else {
        return Ok(ConsolidateOptions::default());
    };

    let Value::Object(map) = value else {
        return Err(PipError::runtime(
            line,
            "consolidate options must be an object",
        ));
    };

    let mut options = ConsolidateOptions::default();

    if let Some(add_source) = map.get("add_source_column") {
        let flag = add_source
            .as_bool()
            .ok_or_else(|| PipError::runtime(line, "add_source_column must be a boolean"))?;
        options.add_source_column = flag;
    }

    if let Some(name) = map.get("source_column_name") {
        let name = name
            .as_str()
            .ok_or_else(|| PipError::runtime(line, "source_column_name must be a string"))?;
        options.source_column_name = name.to_string();
    }

    Ok(options)
}

/// Parse file load options from a DSL object value.
pub fn file_load_options_from_value(
    value: Option<&Value>,
    line: usize,
) -> PipResult<FileLoadOptions> {
    let Some(value) = value else {
        return Ok(FileLoadOptions::default());
    };

    let Value::Object(map) = value else {
        return Err(PipError::runtime(
            line,
            "book load options must be an object",
        ));
    };

    let mut options = FileLoadOptions::default();

    if let Some(has_headers) = map.get("has_headers") {
        let flag = has_headers
            .as_bool()
            .ok_or_else(|| PipError::runtime(line, "has_headers must be a boolean"))?;
        options.has_headers = flag;
    }

    Ok(options)
}
