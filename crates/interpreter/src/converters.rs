//! Conversion utilities for the piptable interpreter.

use arrow::array::{ArrayRef, BooleanArray, Float64Array, Int64Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use indexmap::{IndexMap, IndexSet};
use piptable_core::{PipError, PipResult, Value};
use piptable_sheet::{CellValue, Sheet};
use std::collections::HashMap;
use std::sync::Arc;

/// Convert a Value to a human-readable string representation.
pub fn value_to_string(val: &Value) -> String {
    match val {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Int(n) => n.to_string(),
        Value::Float(f) => f.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(_) => "[Array]".to_string(),
        Value::Object(_) => "[Object]".to_string(),
        Value::Table(_) => "[Table]".to_string(),
        Value::Sheet(_) => "[Sheet]".to_string(),
        Value::Book(_) => "[Book]".to_string(),
        Value::Function { name, .. } => format!("[Function: {name}]"),
        Value::Lambda { params, .. } => format!("[Lambda: |{}|]", params.join(", ")),
    }
}

/// Convert a Value to a numeric representation if possible.
pub fn value_to_number(val: &Value) -> Option<f64> {
    match val {
        Value::Int(n) => Some(*n as f64),
        Value::Float(f) => Some(*f),
        _ => None,
    }
}

/// Check if a string matches a LIKE pattern with % wildcards.
pub fn matches_like(s: &str, pattern: &str) -> bool {
    let parts: Vec<&str> = pattern.split('%').collect();
    if parts.len() == 1 {
        return s == pattern;
    }

    let mut pos = 0;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if i == 0 {
            // Must start with this part
            if !s.starts_with(part) {
                return false;
            }
            pos = part.len();
        } else if i == parts.len() - 1 {
            // Must end with this part
            if !s[pos..].ends_with(part) {
                return false;
            }
        } else {
            // Must contain this part
            if let Some(idx) = s[pos..].find(part) {
                pos += idx + part.len();
            } else {
                return false;
            }
        }
    }
    true
}

/// Convert a Sheet to Arrow RecordBatch for SQL operations.
#[allow(dead_code)]
pub fn sheet_to_arrow(sheet: &Sheet, skip_first_row: usize) -> PipResult<RecordBatch> {
    let column_names = sheet
        .column_names()
        .ok_or_else(|| PipError::runtime(0, "Sheet must have column names for SQL operations"))?;

    // Create schema with appropriate types
    let mut fields = Vec::new();
    let mut arrays: Vec<ArrayRef> = Vec::new();

    for (col_idx, col_name) in column_names.iter().enumerate() {
        // Determine column type by scanning values
        let mut has_int = false;
        let mut has_float = false;
        let mut has_bool = false;
        let mut has_string = false;

        for row_idx in skip_first_row..sheet.row_count() {
            if let Some(row) = sheet.data().get(row_idx) {
                if let Some(cell) = row.get(col_idx) {
                    match cell.cached_or_self() {
                        CellValue::Int(_) => has_int = true,
                        CellValue::Float(_) => has_float = true,
                        CellValue::Bool(_) => has_bool = true,
                        CellValue::String(_) => has_string = true,
                        CellValue::Formula(_) => has_string = true,
                        CellValue::Null => {}
                    }
                }
            }
        }

        // Choose the most appropriate type
        let (field_type, array): (DataType, ArrayRef) =
            if has_string || (has_int && has_float && has_bool) {
                // Mixed types -> use string
                let mut values = Vec::new();
                for row_idx in skip_first_row..sheet.row_count() {
                    if let Some(row) = sheet.data().get(row_idx) {
                        if let Some(cell) = row.get(col_idx) {
                            match cell.cached_or_self() {
                                CellValue::String(s) => values.push(Some(s.clone())),
                                CellValue::Int(i) => values.push(Some(i.to_string())),
                                CellValue::Float(f) => values.push(Some(f.to_string())),
                                CellValue::Bool(b) => values.push(Some(b.to_string())),
                                CellValue::Null => values.push(None),
                                CellValue::Formula(formula) => {
                                    values.push(Some(formula.source.clone()));
                                }
                            }
                        } else {
                            values.push(None);
                        }
                    } else {
                        values.push(None);
                    }
                }
                (DataType::Utf8, Arc::new(StringArray::from(values)))
            } else if has_float {
                // Numeric with float -> use float
                let mut values = Vec::new();
                for row_idx in skip_first_row..sheet.row_count() {
                    if let Some(row) = sheet.data().get(row_idx) {
                        if let Some(cell) = row.get(col_idx) {
                            match cell.cached_or_self() {
                                CellValue::Float(f) => values.push(Some(*f)),
                                CellValue::Int(i) => values.push(Some(*i as f64)),
                                CellValue::Bool(b) => values.push(Some(if *b { 1.0 } else { 0.0 })),
                                _ => values.push(None),
                            }
                        } else {
                            values.push(None);
                        }
                    } else {
                        values.push(None);
                    }
                }
                (DataType::Float64, Arc::new(Float64Array::from(values)))
            } else if has_int {
                // Only integers -> use int
                let mut values = Vec::new();
                for row_idx in skip_first_row..sheet.row_count() {
                    if let Some(row) = sheet.data().get(row_idx) {
                        if let Some(cell) = row.get(col_idx) {
                            match cell.cached_or_self() {
                                CellValue::Int(i) => values.push(Some(*i)),
                                CellValue::Bool(b) => values.push(Some(if *b { 1 } else { 0 })),
                                _ => values.push(None),
                            }
                        } else {
                            values.push(None);
                        }
                    } else {
                        values.push(None);
                    }
                }
                (DataType::Int64, Arc::new(Int64Array::from(values)))
            } else if has_bool {
                // Only booleans -> use bool
                let mut values = Vec::new();
                for row_idx in skip_first_row..sheet.row_count() {
                    if let Some(row) = sheet.data().get(row_idx) {
                        if let Some(cell) = row.get(col_idx) {
                            match cell.cached_or_self() {
                                CellValue::Bool(b) => values.push(Some(*b)),
                                _ => values.push(None),
                            }
                        } else {
                            values.push(None);
                        }
                    } else {
                        values.push(None);
                    }
                }
                (DataType::Boolean, Arc::new(BooleanArray::from(values)))
            } else {
                // All nulls or empty -> use string
                let values: Vec<Option<String>> =
                    (skip_first_row..sheet.row_count()).map(|_| None).collect();
                (DataType::Utf8, Arc::new(StringArray::from(values)))
            };

        fields.push(Field::new(col_name, field_type, true));
        arrays.push(array);
    }

    let schema = Arc::new(Schema::new(fields));
    RecordBatch::try_new(schema, arrays)
        .map_err(|e| PipError::runtime(0, format!("Failed to create RecordBatch: {}", e)))
}

/// Consolidate a book object (from multi-file import) into a single array.
pub fn consolidate_book(
    book: &HashMap<String, Value>,
    source_col: Option<&str>,
) -> Result<Value, String> {
    // Sort sheet names for deterministic processing
    let mut sheet_names: Vec<_> = book.keys().collect();
    sheet_names.sort();

    // Collect all column names across all sheets
    let mut all_columns: IndexSet<String> = IndexSet::new();

    // First pass: validate and collect all column names
    for sheet_name in &sheet_names {
        let value = &book[*sheet_name];
        match value {
            Value::Array(rows) => {
                for row in rows {
                    match row {
                        Value::Object(obj) => {
                            for key in obj.keys() {
                                all_columns.insert(key.clone());
                            }
                        }
                        _ => {
                            return Err(format!("Sheet '{}' contains non-object rows", sheet_name));
                        }
                    }
                }
            }
            Value::Sheet(sheet) => {
                if let Some(col_names) = sheet.column_names() {
                    for col_name in col_names {
                        all_columns.insert(col_name.clone());
                    }
                } else {
                    return Err(format!(
                        "Sheet '{}' has no column names for consolidation",
                        sheet_name
                    ));
                }
            }
            _ => {
                return Err(format!(
                    "Cannot consolidate non-array/sheet value in '{}'",
                    sheet_name
                ));
            }
        }
    }

    // Check for source column conflict
    if let Some(col) = source_col {
        if all_columns.contains(col) {
            return Err(format!(
                "Source column name '{}' conflicts with existing column",
                col
            ));
        }
    }

    // Second pass: build consolidated result with all columns
    let mut all_records = Vec::new();

    for sheet_name in &sheet_names {
        let value = &book[*sheet_name];
        match value {
            Value::Array(records) => {
                for (row_idx, record) in records.iter().enumerate() {
                    if let Value::Object(obj) = record {
                        // Only check for header row on the first row of each sheet
                        if row_idx == 0 {
                            // Skip header rows: check if this row contains mostly column names as string values
                            let string_values: Vec<&String> = obj
                                .values()
                                .filter_map(|v| {
                                    if let Value::String(s) = v {
                                        Some(s)
                                    } else {
                                        None
                                    }
                                })
                                .collect();

                            let is_header_row = if string_values.len() >= 2 {
                                // If ALL string values are column names, it's likely a header row
                                let matching_columns = string_values
                                    .iter()
                                    .filter(|s| all_columns.contains(s.as_str()))
                                    .count();
                                matching_columns == string_values.len() // All string values must match column names
                            } else {
                                false
                            };

                            if is_header_row {
                                continue; // Skip header rows
                            }
                        }

                        let mut new_row: IndexMap<String, Value> = IndexMap::new();

                        // Add source column if requested
                        if let Some(col) = source_col {
                            new_row.insert(col.to_string(), Value::String(sheet_name.to_string()));
                        }

                        // Add all columns (with nulls for missing)
                        for col_name in &all_columns {
                            let val = obj.get(col_name).cloned().unwrap_or(Value::Null);
                            new_row.insert(col_name.clone(), val);
                        }

                        all_records.push(Value::Object(new_row.into_iter().collect()));
                    }
                }
            }
            Value::Sheet(sheet) => {
                if let Some(records) = sheet.to_records() {
                    for record in records {
                        let mut new_row: IndexMap<String, Value> = IndexMap::new();

                        // Add source column if requested
                        if let Some(col) = source_col {
                            new_row.insert(col.to_string(), Value::String(sheet_name.to_string()));
                        }

                        // Add all columns (with nulls for missing from this sheet)
                        for col_name in &all_columns {
                            let val = record
                                .get(col_name.as_str())
                                .map(|cell| match cell.cached_or_self() {
                                    CellValue::Null => Value::Null,
                                    CellValue::String(s) => Value::String(s.clone()),
                                    CellValue::Int(i) => Value::Int(*i),
                                    CellValue::Float(f) => Value::Float(*f),
                                    CellValue::Bool(b) => Value::Bool(*b),
                                    CellValue::Formula(formula) => {
                                        Value::String(formula.source.clone())
                                    }
                                })
                                .unwrap_or(Value::Null);
                            new_row.insert(col_name.clone(), val);
                        }

                        all_records.push(Value::Object(new_row.into_iter().collect()));
                    }
                }
            }
            _ => {}
        }
    }

    Ok(Value::Array(all_records))
}
