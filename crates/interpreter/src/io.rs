//! Import and export operations for files.

use piptable_core::{ImportOptions, Value};
use piptable_sheet::{CsvOptions, Sheet, XlsxReadOptions};
use std::collections::HashMap;
use std::path::Path;

/// Export a sheet to a file based on extension.
pub fn export_sheet(sheet: &Sheet, path: &str) -> Result<(), String> {
    let path_lower = path.to_lowercase();
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

/// Import a sheet from a file based on extension.
pub fn import_sheet(
    path: &str,
    sheet_name: Option<&str>,
    has_headers: bool,
) -> Result<Sheet, String> {
    let path_lower = path.to_lowercase();
    if path_lower.ends_with(".csv") || path_lower.ends_with(".tsv") {
        let mut sheet =
            Sheet::from_csv(path).map_err(|e| format!("Failed to import CSV: {}", e))?;
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
