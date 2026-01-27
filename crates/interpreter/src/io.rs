//! Import and export operations for files.

use piptable_core::{ImportOptions, Value};
use piptable_sheet::Sheet;
use std::collections::HashMap;
use std::path::Path;

/// Export a sheet to a file based on extension.
pub fn export_sheet(sheet: &Sheet, path: &str) -> Result<(), String> {
    let path_lower = path.to_lowercase();
    if path_lower.ends_with(".csv") || path_lower.ends_with(".tsv") {
        sheet
            .save_as_csv(path)
            .map_err(|e| format!("Failed to export CSV: {}", e))
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
        let mut sheet = if let Some(_name) = sheet_name {
            // For now, just load the first sheet and ignore sheet_name
            // TODO: implement proper sheet selection
            Sheet::from_excel(path).map_err(|e| format!("Failed to import Excel: {}", e))?
        } else {
            Sheet::from_excel(path).map_err(|e| format!("Failed to import Excel: {}", e))?
        };
        if has_headers {
            sheet
                .name_columns_by_row(0)
                .map_err(|e| format!("Failed to name columns: {}", e))?;
        }
        Ok(sheet)
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
        let name = path_obj
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("sheet")
            .to_string();

        let sheet = import_sheet(path, None, options.has_headers.unwrap_or(true))?;
        sheets.insert(name, Value::Sheet(sheet));
    }

    if sheets.len() == 1 {
        Ok(sheets.into_values().next().unwrap())
    } else {
        Ok(Value::Object(sheets))
    }
}
