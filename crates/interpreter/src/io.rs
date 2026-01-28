//! Import and export operations for files.

use piptable_core::{ImportOptions, Value};
use piptable_sheet::{CellValue, CsvOptions, Sheet, XlsxReadOptions};
use std::collections::HashMap;
use std::path::Path;

/// Export a sheet to a file with optional append mode.
pub fn export_sheet_with_mode(sheet: &Sheet, path: &str, append: bool) -> Result<(), String> {
    let path_lower = path.to_lowercase();

    // For append mode, we need to handle CSV specially
    if append && (path_lower.ends_with(".csv") || path_lower.ends_with(".tsv")) {
        // If file exists, load it first and append new data
        if std::path::Path::new(path).exists() {
            // First, load the file without assuming headers to detect if headers exist
            // TODO: Consider optimizing to avoid double I/O for large files by transforming in-place
            let raw_sheet = if path_lower.ends_with(".tsv") {
                Sheet::from_csv_with_options(path, CsvOptions::tsv())
                    .map_err(|e| format!("Failed to load existing TSV: {}", e))?
            } else {
                Sheet::from_csv(path).map_err(|e| format!("Failed to load existing CSV: {}", e))?
            };

            // Detect if the existing file has headers by checking:
            // 1. The first row contains all strings (typical of headers)
            // 2. Second row (if exists) has different types (typical of data)
            let has_headers = raw_sheet
                .data()
                .first()
                .map(|first_row| {
                    // Check if all values in first row are strings
                    let all_strings = first_row
                        .iter()
                        .all(|cell| matches!(cell, CellValue::String(_)));

                    // If we have a second row, check if it has different types (indicates headers)
                    let has_different_types = raw_sheet
                        .data()
                        .get(1)
                        .map(|second_row| {
                            // If second row has any non-string values, first row is likely headers
                            second_row
                                .iter()
                                .any(|cell| !matches!(cell, CellValue::String(_)))
                        })
                        .unwrap_or(false);

                    // If new sheet has column names, also check if column count matches
                    let column_count_matches = if let Some(new_cols) = sheet.column_names() {
                        first_row.len() == new_cols.len()
                    } else {
                        false
                    };

                    // Consider it has headers if:
                    // - All first row values are strings AND
                    // - Either second row has different types OR (if new sheet has columns) column count matches
                    all_strings && (has_different_types || column_count_matches)
                })
                .unwrap_or(false);

            // Now reload with proper header handling
            let mut existing_sheet = if has_headers {
                import_sheet(path, None, true)
                    .map_err(|e| format!("Failed to load existing file with headers: {}", e))?
            } else {
                import_sheet(path, None, false)
                    .map_err(|e| format!("Failed to load existing file without headers: {}", e))?
            };

            // Append new data to existing sheet
            append_sheet_data(&mut existing_sheet, sheet)
                .map_err(|e| format!("Failed to append data: {}", e))?;

            // Save the combined sheet
            if path_lower.ends_with(".tsv") {
                existing_sheet
                    .save_as_csv_with_options(path, CsvOptions::tsv())
                    .map_err(|e| format!("Failed to export TSV: {}", e))
            } else {
                existing_sheet
                    .save_as_csv(path)
                    .map_err(|e| format!("Failed to export CSV: {}", e))
            }
        } else {
            // File doesn't exist, just save normally
            if path_lower.ends_with(".tsv") {
                sheet
                    .save_as_csv_with_options(path, CsvOptions::tsv())
                    .map_err(|e| format!("Failed to export TSV: {}", e))
            } else {
                sheet
                    .save_as_csv(path)
                    .map_err(|e| format!("Failed to export CSV: {}", e))
            }
        }
    } else if append {
        // Append mode not supported for other formats yet
        Err("Append mode is only supported for CSV and TSV files".to_string())
    } else {
        // Normal export without append
        if path_lower.ends_with(".csv") {
            sheet
                .save_as_csv(path)
                .map_err(|e| format!("Failed to export CSV: {}", e))
        } else if path_lower.ends_with(".tsv") {
            sheet
                .save_as_csv_with_options(path, CsvOptions::tsv())
                .map_err(|e| format!("Failed to export TSV: {}", e))
        } else if path_lower.ends_with(".json") {
            sheet
                .save_as_json(path)
                .map_err(|e| format!("Failed to export JSON: {}", e))
        } else if path_lower.ends_with(".jsonl") {
            sheet
                .save_as_jsonl(path)
                .map_err(|e| format!("Failed to export JSONL: {}", e))
        } else if path_lower.ends_with(".xlsx") {
            sheet
                .save_as_xlsx(path)
                .map_err(|e| format!("Failed to export Excel: {}", e))
        } else if path_lower.ends_with(".parquet") {
            sheet
                .save_as_parquet(path)
                .map_err(|e| format!("Failed to export Parquet: {}", e))
        } else if path_lower.ends_with(".toon") {
            sheet
                .save_as_toon(path)
                .map_err(|e| format!("Failed to export TOON: {}", e))
        } else {
            Err(format!("Unsupported export format for '{}'", path))
        }
    }
}

/// Helper function to append data from one sheet to another.
fn append_sheet_data(existing: &mut Sheet, new_data: &Sheet) -> Result<(), String> {
    // Check if columns match
    let existing_cols = existing.column_names();
    let new_cols = new_data.column_names();

    match (existing_cols, new_cols) {
        (Some(e_cols), Some(n_cols)) => {
            // Both have column names - verify they match
            if e_cols != n_cols {
                return Err(format!(
                    "Column mismatch: existing file has {:?}, new data has {:?}",
                    e_cols, n_cols
                ));
            }
        }
        (None, None) => {
            // Neither has column names - check column count
            let existing_cols = existing.data().first().map(|r| r.len());
            let new_cols = new_data.data().first().map(|r| r.len());

            match (existing_cols, new_cols) {
                (Some(e), Some(n)) if e != n => {
                    return Err(format!(
                        "Column count mismatch: existing has {} columns, new data has {} columns",
                        e, n
                    ));
                }
                (Some(_), Some(_)) => {
                    // Column counts match - this is fine, proceed with append
                }
                (None, Some(_)) => {
                    // Existing is empty, will take shape from new data - this is fine
                }
                (Some(_) | None, None) => {
                    // New data is empty or both are empty - nothing to append, but that's ok
                }
            }
        }
        _ => {
            return Err(
                "Cannot append: one sheet has column names while the other doesn't".to_string(),
            );
        }
    }

    // Determine if new_data has a physical header row to skip
    let skip_header = match new_data.column_names() {
        Some(names) => {
            // Check if first row matches column names
            new_data
                .data()
                .first()
                .map(|row| {
                    names.iter().enumerate().all(|(idx, name)| {
                        row.get(idx)
                            .map(|cell| cell.as_str() == name.as_str())
                            .unwrap_or(false)
                    })
                })
                .unwrap_or(false)
        }
        None => false,
    };

    // Append rows, skipping header if present
    let start_index = if skip_header { 1 } else { 0 };
    for row in new_data.data().iter().skip(start_index) {
        existing
            .row_append(row.clone())
            .map_err(|e| format!("Failed to append row: {}", e))?;
    }

    Ok(())
}

/// Import a sheet from a file based on extension.
pub fn import_sheet(
    path: &str,
    sheet_name: Option<&str>,
    has_headers: bool,
) -> Result<Sheet, String> {
    let path_lower = path.to_lowercase();
    if path_lower.ends_with(".csv") || path_lower.ends_with(".tsv") {
        let mut sheet = if path_lower.ends_with(".tsv") {
            Sheet::from_csv_with_options(path, CsvOptions::tsv())
                .map_err(|e| format!("Failed to import TSV: {}", e))?
        } else {
            Sheet::from_csv(path).map_err(|e| format!("Failed to import CSV: {}", e))?
        };
        if has_headers {
            sheet
                .name_columns_by_row(0)
                .map_err(|e| format!("Failed to name columns: {}", e))?;
        }
        Ok(sheet)
    } else if path_lower.ends_with(".json") {
        Sheet::from_json(path).map_err(|e| format!("Failed to import JSON: {}", e))
    } else if path_lower.ends_with(".jsonl") {
        Sheet::from_jsonl(path).map_err(|e| format!("Failed to import JSONL: {}", e))
    } else if path_lower.ends_with(".xlsx") || path_lower.ends_with(".xls") {
        let options = XlsxReadOptions::default().with_headers(has_headers);

        if let Some(name) = sheet_name {
            // Load specific sheet by name
            if path_lower.ends_with(".xlsx") {
                Sheet::from_xlsx_sheet_with_options(path, name, options)
                    .map_err(|e| format!("Failed to import Excel sheet '{}': {}", name, e))
            } else {
                Sheet::from_xls_sheet_with_options(path, name, options)
                    .map_err(|e| format!("Failed to import Excel sheet '{}': {}", name, e))
            }
        } else {
            // Load the first sheet (default behavior)
            Sheet::from_excel_with_options(path, options)
                .map_err(|e| format!("Failed to import Excel: {}", e))
        }
    } else if path_lower.ends_with(".parquet") {
        Sheet::from_parquet(path).map_err(|e| format!("Failed to import Parquet: {}", e))
    } else if path_lower.ends_with(".toon") {
        Sheet::from_toon(path).map_err(|e| format!("Failed to import TOON: {}", e))
    } else {
        Err(format!("Unsupported import format for '{}'", path))
    }
}

/// Import multiple files based on options.
pub fn import_multi_files(paths: &[String], options: &ImportOptions) -> Result<Value, String> {
    let mut sheets = HashMap::new();

    for path in paths {
        let path_obj = Path::new(path);
        let base_name = path_obj
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("sheet")
            .to_string();

        // Handle duplicate file stems by appending a suffix
        let mut name = base_name.clone();
        let mut counter = 1;
        while sheets.contains_key(&name) {
            name = format!("{}_{}", base_name, counter);
            counter += 1;
        }

        let sheet = import_sheet(path, None, options.has_headers.unwrap_or(true))?;
        sheets.insert(name, Value::Sheet(sheet));
    }

    if sheets.len() == 1 {
        Ok(sheets.into_values().next().unwrap())
    } else {
        Ok(Value::Object(sheets))
    }
}
