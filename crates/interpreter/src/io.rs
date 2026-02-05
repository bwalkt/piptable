//! Import and export operations for files.

use piptable_core::{ImportOptions, Value};
#[cfg(not(target_arch = "wasm32"))]
use piptable_sheet::XlsxReadOptions;
use piptable_sheet::{Book, CellValue, CsvOptions, Sheet};
use std::path::Path;

/// Convert a CellValue to a serde_json Value
fn cell_to_json_value(cell: CellValue) -> serde_json::Value {
    use serde_json::Value as JsonValue;
    match cell {
        CellValue::Null => JsonValue::Null,
        CellValue::Bool(b) => JsonValue::Bool(b),
        CellValue::Int(i) => JsonValue::Number(i.into()),
        CellValue::Float(f) => {
            if f.is_finite() {
                // For finite floats, from_f64 should always succeed
                JsonValue::Number(
                    serde_json::Number::from_f64(f).expect("finite f64 should be representable"),
                )
            } else {
                JsonValue::Null // NaN and Infinity become null
            }
        }
        CellValue::String(s) => JsonValue::String(s),
        CellValue::Formula(formula) => {
            let mut obj = serde_json::Map::new();
            obj.insert("formula".to_string(), JsonValue::String(formula.source));
            if let Some(cached) = formula.cached {
                obj.insert("cached".to_string(), cell_to_json_value(*cached));
            }
            JsonValue::Object(obj)
        }
    }
}

/// Check if the first row of sheet data matches the column names (i.e., is a header row)
fn has_header_row(sheet: &Sheet) -> bool {
    match sheet.column_names() {
        Some(names) => sheet.data().first().is_some_and(|row| {
            names.iter().enumerate().all(|(idx, name)| {
                row.get(idx)
                    .map(|cell| cell.as_str() == name.as_str())
                    .unwrap_or(false)
            })
        }),
        None => false,
    }
}

/// Detect if a CSV file has headers by reading only the first few rows
fn detect_csv_headers(path: &str, delimiter: u8) -> Result<bool, String> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    let file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    // Read first row
    let first_line = match lines.next() {
        Some(Ok(line)) => line,
        Some(Err(e)) => return Err(format!("Failed to read first line: {}", e)),
        None => return Ok(false), // Empty file, no headers
    };

    // Read second row if it exists
    let second_line = lines.next().and_then(|r| r.ok());

    // Parse rows
    let first_row = parse_csv_line(&first_line, delimiter);
    let second_row = second_line.as_ref().map(|l| parse_csv_line(l, delimiter));

    // Check if first row is likely headers:
    // 1. All values in first row are non-numeric strings
    // 2. Second row (if exists) has numeric values OR significantly different patterns
    let all_strings = first_row.iter().all(|v| {
        // Check if it's a non-empty string that doesn't parse as a number
        !v.is_empty() && v.parse::<f64>().is_err() && v != "true" && v != "false"
    });

    let has_numbers_or_different_pattern = second_row
        .as_ref()
        .map(|row| {
            // Check for numeric/bool values
            let has_numbers = row
                .iter()
                .any(|v| v.parse::<f64>().is_ok() || v == "true" || v == "false");

            // For string-only data, only consider it headers if there are STRONG indicators:
            // - Headers contain underscores/descriptive terms vs normal names
            // - Headers are significantly longer/shorter than data
            // - Headers contain common header words
            let different_patterns = !has_numbers && {
                let header_indicators = first_row.iter().any(|h| {
                    h.contains('_')
                        || h.to_lowercase().contains("id")
                        || h.to_lowercase().contains("name")
                        || h.to_lowercase().contains("type")
                        || h.to_lowercase().contains("value")
                });

                let size_difference = row.iter().zip(first_row.iter()).any(|(data, header)| {
                    data.len() > 30 && header.len() < 15 // Significantly longer data vs short headers
                });

                header_indicators || size_difference
            };

            has_numbers || different_patterns
        })
        .unwrap_or(false);

    Ok(all_strings && (second_row.is_none() || has_numbers_or_different_pattern))
}

/// Simple CSV line parser for header detection
fn parse_csv_line(line: &str, delimiter: u8) -> Vec<String> {
    let delimiter_char = delimiter as char;
    let mut result = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '"' {
            if in_quotes && chars.peek() == Some(&'"') {
                // Escaped quote
                current.push('"');
                chars.next();
            } else {
                in_quotes = !in_quotes;
            }
        } else if ch == delimiter_char && !in_quotes {
            result.push(current.trim().to_string());
            current.clear();
        } else {
            current.push(ch);
        }
    }
    result.push(current.trim().to_string());
    result
}

/// Export mode options
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExportMode {
    /// Overwrite the file
    Overwrite,
    /// Append all rows
    Append,
    /// Append only distinct rows based on a key
    AppendDistinct { key_column: Option<usize> },
    /// Update existing rows or append new ones based on a key
    AppendOrUpdate { key_column: Option<usize> },
}

/// Export a sheet to a file with specified mode.
pub fn export_sheet_with_mode(sheet: &Sheet, path: &str, mode: ExportMode) -> Result<(), String> {
    export_sheet_with_mode_impl(sheet, path, mode)
}

/// Export a sheet to a file with optional append mode (for backward compatibility).
pub fn export_sheet_with_append(sheet: &Sheet, path: &str, append: bool) -> Result<(), String> {
    let mode = if append {
        ExportMode::Append
    } else {
        ExportMode::Overwrite
    };
    export_sheet_with_mode_impl(sheet, path, mode)
}

/// Internal implementation of export with mode.
fn export_sheet_with_mode_impl(sheet: &Sheet, path: &str, mode: ExportMode) -> Result<(), String> {
    // For backward compatibility
    let append = matches!(
        mode,
        ExportMode::Append | ExportMode::AppendDistinct { .. } | ExportMode::AppendOrUpdate { .. }
    );
    let path_lower = path.to_lowercase();

    // For append mode, we need to handle CSV specially
    if append && (path_lower.ends_with(".csv") || path_lower.ends_with(".tsv")) {
        // If file exists, load it first and append new data
        if std::path::Path::new(path).exists() {
            // Efficiently detect if the existing file has headers by reading only the first few rows
            let delimiter = if path_lower.ends_with(".tsv") {
                b'\t'
            } else {
                b','
            };
            let has_headers = detect_csv_headers(path, delimiter)?;

            // Load the file with proper header handling
            let import_options = ImportOptions {
                has_headers: Some(has_headers),
                ..ImportOptions::default()
            };
            let mut existing_sheet = import_sheet(path, None, &import_options)
                .map_err(|e| format!("Failed to load existing file: {}", e))?;

            // Append new data to existing sheet based on mode
            match mode {
                ExportMode::Append => {
                    append_sheet_data(&mut existing_sheet, sheet)
                        .map_err(|e| format!("Failed to append data: {}", e))?;
                }
                ExportMode::AppendDistinct { key_column } => {
                    append_sheet_distinct(&mut existing_sheet, sheet, key_column)
                        .map_err(|e| format!("Failed to append distinct data: {}", e))?;
                }
                ExportMode::AppendOrUpdate { key_column } => {
                    append_sheet_or_update(&mut existing_sheet, sheet, key_column)
                        .map_err(|e| format!("Failed to append or update data: {}", e))?;
                }
                ExportMode::Overwrite => unreachable!(),
            }

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
    } else if append && path_lower.ends_with(".json") {
        // JSON append mode - need to load entire array, append, and save
        if std::path::Path::new(path).exists() {
            // Load existing JSON file
            let import_options = ImportOptions {
                has_headers: Some(false),
                ..ImportOptions::default()
            };
            let mut existing_sheet = import_sheet(path, None, &import_options)
                .map_err(|e| format!("Failed to load existing JSON: {}", e))?;

            // Append new data to existing sheet based on mode
            match mode {
                ExportMode::Append => {
                    append_sheet_data(&mut existing_sheet, sheet)
                        .map_err(|e| format!("Failed to append data: {}", e))?;
                }
                ExportMode::AppendDistinct { key_column } => {
                    append_sheet_distinct(&mut existing_sheet, sheet, key_column)
                        .map_err(|e| format!("Failed to append distinct data: {}", e))?;
                }
                ExportMode::AppendOrUpdate { key_column } => {
                    append_sheet_or_update(&mut existing_sheet, sheet, key_column)
                        .map_err(|e| format!("Failed to append or update data: {}", e))?;
                }
                ExportMode::Overwrite => unreachable!(),
            }

            // Save the combined sheet
            existing_sheet
                .save_as_json(path)
                .map_err(|e| format!("Failed to export JSON: {}", e))
        } else {
            // File doesn't exist, just save normally
            sheet
                .save_as_json(path)
                .map_err(|e| format!("Failed to export JSON: {}", e))
        }
    } else if append && path_lower.ends_with(".jsonl") {
        // JSONL append mode - each line is a separate JSON object
        // Note: JSONL format intentionally allows schema evolution between lines,
        // so we don't validate column compatibility with existing data
        use std::fs::OpenOptions;
        use std::io::Write;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| format!("Failed to open JSONL file for append: {}", e))?;

        // Convert sheet to records and append each as a JSON line
        let records = sheet
            .to_records()
            .ok_or("Columns must be named to export as JSONL")?;

        // Check if first row is a header that matches column names
        let skip_header = has_header_row(sheet);

        let start_idx = if skip_header { 1 } else { 0 };
        for record in records.into_iter().skip(start_idx) {
            // Use IndexMap for deterministic key ordering
            let json_obj: indexmap::IndexMap<String, serde_json::Value> = record
                .into_iter()
                .map(|(k, v)| (k, cell_to_json_value(v)))
                .collect();

            serde_json::to_writer(&mut file, &json_obj)
                .map_err(|e| format!("Failed to serialize record to JSON: {}", e))?;
            writeln!(file).map_err(|e| format!("Failed to write to JSONL file: {}", e))?;
        }

        Ok(())
    } else if append {
        // Append mode not supported for other formats yet
        Err("Append mode is only supported for CSV, TSV, JSON, and JSONL files".to_string())
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
            #[cfg(not(target_arch = "wasm32"))]
            {
                sheet
                    .save_as_xlsx(path)
                    .map_err(|e| format!("Failed to export Excel: {}", e))
            }
            #[cfg(target_arch = "wasm32")]
            {
                Err("Excel export is not supported in the playground".to_string())
            }
        } else if path_lower.ends_with(".parquet") {
            #[cfg(not(target_arch = "wasm32"))]
            {
                sheet
                    .save_as_parquet(path)
                    .map_err(|e| format!("Failed to export Parquet: {}", e))
            }
            #[cfg(target_arch = "wasm32")]
            {
                Err("Parquet export is not supported in the playground".to_string())
            }
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
        (None, Some(_)) => {
            // Existing has no column names, new data has column names
            // This is allowed for regular append - we skip the new data's header row
            let existing_cols = existing.data().first().map(|r| r.len());
            let new_cols = new_data.data().get(1).map(|r| r.len()); // Skip header row
            match (existing_cols, new_cols) {
                (Some(e), Some(n)) if e != n => {
                    return Err(format!(
                        "Column count mismatch: existing has {} columns, new data has {} columns",
                        e, n
                    ));
                }
                _ => {}
            }
        }
        (Some(_), None) => {
            return Err(
                "Cannot append: existing file has column names but new data doesn't".to_string(),
            );
        }
    }

    // Determine if new_data has a physical header row to skip
    let skip_header = has_header_row(new_data);

    // Append rows, skipping header if present
    let start_index = if skip_header { 1 } else { 0 };
    for row in new_data.data().iter().skip(start_index) {
        existing
            .row_append(row.clone())
            .map_err(|e| format!("Failed to append row: {}", e))?;
    }

    Ok(())
}

/// Helper function to normalize column name state between two sheets
fn normalize_column_names(existing: &mut Sheet, new_data: &Sheet) -> Result<(), String> {
    let existing_cols = existing.column_names().cloned();
    let new_cols = new_data.column_names().cloned();

    if let (None, Some(_)) = (existing_cols, new_cols) {
        // Existing has no column names but new data does
        // Only name columns if the existing data looks like headers (detect_csv_headers would help)
        // For now, don't auto-assign column names to avoid data/header confusion
        // The append functions will handle this case appropriately
    }

    Ok(())
}

/// Helper function to append only distinct rows from one sheet to another.
fn append_sheet_distinct(
    existing: &mut Sheet,
    new_data: &Sheet,
    key_column: Option<usize>,
) -> Result<(), String> {
    // Normalize column names first
    normalize_column_names(existing, new_data)?;

    // Then do the same column validation as regular append
    let existing_cols = existing.column_names();
    let new_cols = new_data.column_names();

    match (existing_cols, new_cols) {
        (Some(e_cols), Some(n_cols)) => {
            if e_cols != n_cols {
                return Err(format!(
                    "Column mismatch: existing file has {:?}, new data has {:?}",
                    e_cols, n_cols
                ));
            }
        }
        (None, None) => {
            let existing_cols = existing.data().first().map(|r| r.len());
            let new_cols = new_data.data().first().map(|r| r.len());
            match (existing_cols, new_cols) {
                (Some(e), Some(n)) if e != n => {
                    return Err(format!(
                        "Column count mismatch: existing has {} columns, new data has {} columns",
                        e, n
                    ));
                }
                _ => {}
            }
        }
        _ => {
            return Err(
                "Cannot append: one sheet has column names while the other doesn't".to_string(),
            );
        }
    }

    // Determine key column index to use for uniqueness check
    let key_col = key_column.unwrap_or(0); // Default to first column as key

    // Validate key column is in bounds
    let max_cols = existing.col_count().max(new_data.col_count());
    if max_cols > 0 && key_col >= max_cols {
        return Err(format!(
            "Key column index {} is out of bounds ({} columns)",
            key_col, max_cols
        ));
    }

    // Build a set of existing keys for fast lookup (skip header row if present)
    let mut existing_keys = std::collections::HashSet::new();
    let skip_existing_header = has_header_row(existing);
    let start_existing = if skip_existing_header { 1 } else { 0 };

    for row in existing.data().iter().skip(start_existing) {
        if let Some(key) = row.get(key_col) {
            existing_keys.insert(key.as_str().to_string());
        }
    }

    // Determine if new_data has a header row to skip
    let skip_header = has_header_row(new_data);
    let start_index = if skip_header { 1 } else { 0 };

    // Append only rows with distinct keys
    for row in new_data.data().iter().skip(start_index) {
        if let Some(key) = row.get(key_col) {
            let key_str = key.as_str().to_string();
            if !existing_keys.contains(&key_str) {
                existing
                    .row_append(row.clone())
                    .map_err(|e| format!("Failed to append row: {}", e))?;
                existing_keys.insert(key_str);
            }
        }
    }

    Ok(())
}

/// Helper function to update existing rows or append new ones based on a key.
fn append_sheet_or_update(
    existing: &mut Sheet,
    new_data: &Sheet,
    key_column: Option<usize>,
) -> Result<(), String> {
    // Normalize column names first
    normalize_column_names(existing, new_data)?;

    // Then do column validation
    let existing_cols = existing.column_names();
    let new_cols = new_data.column_names();

    match (existing_cols, new_cols) {
        (Some(e_cols), Some(n_cols)) => {
            if e_cols != n_cols {
                return Err(format!(
                    "Column mismatch: existing file has {:?}, new data has {:?}",
                    e_cols, n_cols
                ));
            }
        }
        (None, None) => {
            let existing_cols = existing.data().first().map(|r| r.len());
            let new_cols = new_data.data().first().map(|r| r.len());
            match (existing_cols, new_cols) {
                (Some(e), Some(n)) if e != n => {
                    return Err(format!(
                        "Column count mismatch: existing has {} columns, new data has {} columns",
                        e, n
                    ));
                }
                _ => {}
            }
        }
        (None, Some(_)) => {
            // Existing has no column names, new data has column names
            // This is allowed - we'll compare data against data (skipping new data's header row)
            let existing_cols = existing.data().first().map(|r| r.len());
            let new_cols = new_data.data().get(1).map(|r| r.len()); // Skip header row
            match (existing_cols, new_cols) {
                (Some(e), Some(n)) if e != n => {
                    return Err(format!(
                        "Column count mismatch: existing has {} columns, new data has {} columns",
                        e, n
                    ));
                }
                _ => {}
            }
        }
        (Some(_), None) => {
            return Err(
                "Cannot append: existing file has column names but new data doesn't".to_string(),
            );
        }
    }

    // Determine key column index
    let key_col = key_column.unwrap_or(0);

    // Validate key column is in bounds
    let max_cols = existing.col_count().max(new_data.col_count());
    if max_cols > 0 && key_col >= max_cols {
        return Err(format!(
            "Key column index {} is out of bounds ({} columns)",
            key_col, max_cols
        ));
    }

    // Build a map of existing rows by key for update
    let mut key_to_row_index = std::collections::HashMap::new();
    let skip_existing_header = has_header_row(existing);
    let start_existing = if skip_existing_header { 1 } else { 0 };

    for (idx, row) in existing.data().iter().enumerate().skip(start_existing) {
        if let Some(key) = row.get(key_col) {
            key_to_row_index.insert(key.as_str().to_string(), idx);
        }
    }

    // Process new data
    let skip_header = has_header_row(new_data);
    let start_index = if skip_header { 1 } else { 0 };

    for row in new_data.data().iter().skip(start_index) {
        if let Some(key) = row.get(key_col) {
            let key_str = key.as_str().to_string();
            if let Some(&row_idx) = key_to_row_index.get(&key_str) {
                // Update existing row
                for (col_idx, cell) in row.iter().enumerate() {
                    existing
                        .set(row_idx, col_idx, cell.clone())
                        .map_err(|e| format!("Failed to update cell: {}", e))?;
                }
            } else {
                // Append new row
                existing
                    .row_append(row.clone())
                    .map_err(|e| format!("Failed to append row: {}", e))?;
            }
        }
    }

    Ok(())
}

// URL import support - to be implemented in a future PR
// Requires implementing from_csv_string, from_json_string, from_html_string methods

/// Import a sheet from a file or URL based on extension.
pub fn import_sheet(
    path: &str,
    sheet_name: Option<&str>,
    options: &ImportOptions,
) -> Result<Sheet, String> {
    let has_headers = resolve_has_headers(options);
    #[cfg(target_arch = "wasm32")]
    let _ = sheet_name;
    // URL support would go here in the future
    // if path.starts_with("http://") || path.starts_with("https://") {
    //     return import_sheet_from_url(path, sheet_name, has_headers);
    // }

    let path_lower = path.to_lowercase();
    if path_lower.ends_with(".csv") || path_lower.ends_with(".tsv") {
        let mut sheet = if path_lower.ends_with(".tsv") {
            Sheet::from_csv_with_options(path, CsvOptions::tsv())
                .map_err(|e| format!("Failed to import TSV: {}", e))?
        } else {
            Sheet::from_csv(path).map_err(|e| format!("Failed to import CSV: {}", e))?
        };
        if has_headers && !sheet.data().is_empty() {
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
        #[cfg(not(target_arch = "wasm32"))]
        {
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
        }
        #[cfg(target_arch = "wasm32")]
        {
            Err("Excel import is not supported in the playground".to_string())
        }
    } else if path_lower.ends_with(".parquet") {
        #[cfg(not(target_arch = "wasm32"))]
        {
            Sheet::from_parquet(path).map_err(|e| format!("Failed to import Parquet: {}", e))
        }
        #[cfg(target_arch = "wasm32")]
        {
            Err("Parquet import is not supported in the playground".to_string())
        }
    } else if path_lower.ends_with(".toon") {
        Sheet::from_toon(path).map_err(|e| format!("Failed to import TOON: {}", e))
    } else if path_lower.ends_with(".pdf") {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let tables = import_pdf_tables(path, options)?;
            if tables.len() > 1 {
                return Err(format!(
                    "PDF '{}' contains {} tables; import into a book to access all tables",
                    path,
                    tables.len()
                ));
            }
            let first = tables
                .into_iter()
                .next()
                .ok_or_else(|| format!("No tables found in PDF '{}'", path))?;
            Ok(first)
        }
        #[cfg(target_arch = "wasm32")]
        {
            Err("PDF import is not supported in the playground".to_string())
        }
    } else if path_lower.ends_with(".html") || path_lower.ends_with(".htm") {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let sheet = Sheet::from_html_with_headers(path, has_headers)
                .map_err(|e| format!("Failed to import HTML: {}", e))?;
            Ok(sheet)
        }
        #[cfg(target_arch = "wasm32")]
        {
            Err("HTML import is not supported in the playground".to_string())
        }
    } else if path_lower.ends_with(".md") {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let markdown = std::fs::read_to_string(path)
                .map_err(|e| format!("Failed to read Markdown file: {}", e))?;
            let mut sheets = piptable_markdown::extract_tables(&markdown)
                .map_err(|e| format!("Failed to import Markdown: {}", e))?;

            if sheets.is_empty() {
                return Err(format!("No tables found in Markdown '{}'", path));
            }

            if sheets.len() > 1 {
                return Err("Markdown import returned multiple tables; import into a book to access all tables".to_string());
            }

            let mut sheet = sheets.remove(0);
            if has_headers && !sheet.data().is_empty() {
                sheet
                    .name_columns_by_row(0)
                    .map_err(|e| format!("Failed to name columns: {}", e))?;
            }
            Ok(sheet)
        }
        #[cfg(target_arch = "wasm32")]
        {
            Err("Markdown import is not supported in the playground".to_string())
        }
    } else {
        Err(format!("Unsupported import format for '{}'", path))
    }
}

/// Import multiple files based on options.
pub fn import_multi_files(paths: &[String], options: &ImportOptions) -> Result<Value, String> {
    let mut book = Book::new();

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
        while book.has_sheet(&name) {
            name = format!("{}_{}", base_name, counter);
            counter += 1;
        }

        let sheet = import_sheet(path, None, options)?;
        book.add_sheet(&name, sheet)
            .map_err(|e| format!("Failed to add sheet: {}", e))?;
    }

    Ok(Value::Book(Box::new(book)))
}

/// Resolves whether to treat the first row as headers.
fn resolve_has_headers(options: &ImportOptions) -> bool {
    options
        .detect_headers
        .or(options.has_headers)
        .unwrap_or(true)
}

/// Resolves the minimum row count for table extraction.
fn resolve_min_table_rows(options: &ImportOptions) -> usize {
    options.min_table_rows.unwrap_or(2)
}

/// Resolves the minimum column count for table extraction.
fn resolve_min_table_cols(options: &ImportOptions) -> usize {
    options.min_table_cols.unwrap_or(2)
}

/// Resolves whether to extract PDF structure alongside tables.
fn resolve_extract_structure(options: &ImportOptions) -> bool {
    options.extract_structure.unwrap_or(false)
}

/// Parses a 1-based page range like "1-3" or a single page like "2".
#[cfg(not(target_arch = "wasm32"))]
fn parse_page_range(range: &str) -> Result<Option<(usize, usize)>, String> {
    let trimmed = range.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    if let Some((start, end)) = trimmed.split_once('-') {
        let start_num = start
            .trim()
            .parse::<usize>()
            .map_err(|_| format!("Invalid page_range '{}': start must be a number", range))?;
        let end_num = end
            .trim()
            .parse::<usize>()
            .map_err(|_| format!("Invalid page_range '{}': end must be a number", range))?;
        if start_num == 0 || end_num == 0 || start_num > end_num {
            return Err(format!(
                "Invalid page_range '{}': must be 1-based start-end",
                range
            ));
        }
        Ok(Some((start_num, end_num)))
    } else {
        let page = trimmed.parse::<usize>().map_err(|_| {
            format!(
                "Invalid page_range '{}': must be a number or start-end",
                range
            )
        })?;
        if page == 0 {
            return Err(format!("Invalid page_range '{}': must be 1-based", range));
        }
        Ok(Some((page, page)))
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn import_pdf_tables(path: &str, options: &ImportOptions) -> Result<Vec<Sheet>, String> {
    let pdf_options = piptable_pdf::extractor::PdfOptions {
        page_range: match &options.page_range {
            Some(range) => parse_page_range(range)?,
            None => None,
        },
        ocr_enabled: false,
        ocr_language: "eng".to_string(),
        min_table_rows: resolve_min_table_rows(options),
        min_table_cols: resolve_min_table_cols(options),
        ..Default::default()
    };

    let mut tables = piptable_pdf::extract_tables_with_options(path, pdf_options)
        .map_err(|e| format!("Failed to import PDF: {}", e))?;

    if tables.is_empty() {
        return Err(format!("No tables found in PDF '{}'", path));
    }

    if resolve_has_headers(options) {
        for sheet in &mut tables {
            if !sheet.data().is_empty() {
                sheet
                    .name_columns_by_row(0)
                    .map_err(|e| format!("Failed to name columns: {}", e))?;
            }
        }
    }

    Ok(tables)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn import_pdf_structure(path: &str, options: &ImportOptions) -> Result<Value, String> {
    let pdf_options = piptable_pdf::extractor::PdfOptions {
        page_range: match &options.page_range {
            Some(range) => parse_page_range(range)?,
            None => None,
        },
        ocr_enabled: false,
        ocr_language: "eng".to_string(),
        min_table_rows: resolve_min_table_rows(options),
        min_table_cols: resolve_min_table_cols(options),
        extract_structure: resolve_extract_structure(options),
    };

    let doc = piptable_pdf::extract_structure_with_options(path, pdf_options)
        .map_err(|e| format!("Failed to import PDF structure: {}", e))?;

    Ok(Value::from_json(doc.to_llm_json()))
}

pub fn import_markdown_book(path: &str, options: &ImportOptions) -> Result<Value, String> {
    #[cfg(target_arch = "wasm32")]
    let _ = options;

    #[cfg(target_arch = "wasm32")]
    {
        return Err("Markdown import is not supported in the playground".to_string());
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let markdown = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read Markdown file: {}", e))?;
        let md_options = piptable_markdown::MarkdownOptions {
            min_table_rows: resolve_min_table_rows(options),
            min_table_cols: resolve_min_table_cols(options),
        };
        let mut sheets = piptable_markdown::extract_tables_with_options(&markdown, md_options)
            .map_err(|e| format!("Failed to import Markdown: {}", e))?;

        if sheets.is_empty() {
            return Err(format!("No tables found in Markdown '{}'", path));
        }

        let mut book = Book::new();
        for (idx, mut sheet) in sheets.drain(..).enumerate() {
            if resolve_has_headers(options) && !sheet.data().is_empty() {
                sheet
                    .name_columns_by_row(0)
                    .map_err(|e| format!("Failed to name columns: {}", e))?;
            }
            let name = format!("table_{}", idx + 1);
            book.add_sheet(&name, sheet)
                .map_err(|e| format!("Failed to add sheet: {}", e))?;
        }

        Ok(Value::Book(Box::new(book)))
    }
}
