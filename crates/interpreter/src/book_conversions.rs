//! Utilities for converting between Book and Value representations.

use crate::sheet_conversions::{cell_to_value, value_to_cell, value_to_sheet};
use piptable_core::{PipError, PipResult, Value};
use piptable_sheet::{Book, CellValue, ConsolidateOptions, FileLoadOptions, Sheet};
use std::collections::HashMap;

/// Convert a Value into a Sheet for Book operations.
pub fn value_to_sheet_for_book(value: &Value) -> Result<Sheet, String> {
    match value {
        Value::Sheet(sheet) => Ok(*sheet.clone()),
        Value::Array(rows) => {
            let array_rows = rows
                .iter()
                .filter(|row| matches!(row, Value::Array(_)))
                .count();
            let object_rows = rows
                .iter()
                .filter(|row| matches!(row, Value::Object(_)))
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
            } else if object_rows == rows.len() {
                value_to_sheet(value)
            } else {
                Err("value_to_sheet_for_book: rows must be arrays or objects".to_string())
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
    book.active_sheet_name().map(|name| name.to_string())
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

#[cfg(test)]
mod tests {
    use super::*;
    use piptable_sheet::CellValue;

    #[test]
    fn test_value_to_sheet_for_book_array_rows() {
        let value = Value::Array(vec![
            Value::Array(vec![Value::Int(1), Value::String("a".to_string())]),
            Value::Array(vec![Value::Int(2), Value::String("b".to_string())]),
        ]);
        let sheet = value_to_sheet_for_book(&value).expect("sheet");
        assert_eq!(sheet.row_count(), 2);
    }

    #[test]
    fn test_value_to_sheet_for_book_object_rows() {
        let mut row = HashMap::new();
        row.insert("a".to_string(), Value::Int(1));
        let value = Value::Array(vec![Value::Object(row)]);
        let sheet = value_to_sheet_for_book(&value).expect("sheet");
        assert_eq!(sheet.row_count(), 2);
    }

    #[test]
    fn test_value_to_sheet_for_book_mixed_rows_error() {
        let value = Value::Array(vec![
            Value::Array(vec![Value::Int(1)]),
            Value::Object(HashMap::new()),
        ]);
        let err = value_to_sheet_for_book(&value).expect_err("mixed row types");
        assert!(err.contains("mixed row types"));
    }

    #[test]
    fn test_value_to_sheet_for_book_scalar_rows_error() {
        let value = Value::Array(vec![Value::Int(1), Value::Int(2)]);
        let err = value_to_sheet_for_book(&value).expect_err("scalar rows");
        assert!(err.contains("rows must be arrays or objects"));
    }

    #[test]
    fn test_active_sheet_name() {
        let mut book = Book::new();
        book.add_sheet("Sheet1", Sheet::new())
            .expect("add sheet");
        book.set_active_sheet("Sheet1").expect("set active");
        assert_eq!(active_sheet_name(&book), Some("Sheet1".to_string()));
    }

    #[test]
    fn test_consolidate_options_parse() {
        let mut opts = HashMap::new();
        opts.insert("add_source_column".to_string(), Value::Bool(true));
        opts.insert(
            "source_column_name".to_string(),
            Value::String("source".to_string()),
        );
        let parsed = consolidate_options_from_value(Some(&Value::Object(opts)), 0)
            .expect("options");
        assert!(parsed.add_source_column);
        assert_eq!(parsed.source_column_name, "source");
    }

    #[test]
    fn test_consolidate_options_type_error() {
        let mut opts = HashMap::new();
        opts.insert("add_source_column".to_string(), Value::String("nope".to_string()));
        let err = consolidate_options_from_value(Some(&Value::Object(opts)), 0)
            .expect_err("type error");
        assert!(err.to_string().contains("add_source_column"));
    }

    #[test]
    fn test_file_load_options_parse() {
        let mut opts = HashMap::new();
        opts.insert("has_headers".to_string(), Value::Bool(false));
        let parsed = file_load_options_from_value(Some(&Value::Object(opts)), 0)
            .expect("options");
        assert!(!parsed.has_headers);
    }

    #[test]
    fn test_file_load_options_type_error() {
        let mut opts = HashMap::new();
        opts.insert("has_headers".to_string(), Value::String("no".to_string()));
        let err = file_load_options_from_value(Some(&Value::Object(opts)), 0)
            .expect_err("type error");
        assert!(err.to_string().contains("has_headers"));
    }

    #[test]
    fn test_book_to_value_dict_round_trip_shape() {
        let mut sheet = Sheet::new();
        sheet.row_append(vec![CellValue::Int(1)]).expect("row");
        let mut book = Book::new();
        book.add_sheet("Sheet1", sheet).expect("add sheet");
        let value = book_to_value_dict(&book);
        match value {
            Value::Object(map) => {
                assert!(map.contains_key("Sheet1"));
            }
            _ => panic!("expected object"),
        }
    }
}
