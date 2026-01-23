//! JSON and JSONL (JSON Lines) support for Sheet
//!
//! Provides reading and writing sheets as:
//! - JSON: Array of objects `[{"name": "Alice", "age": 30}, ...]`
//! - JSONL: One JSON object per line (newline-delimited JSON)

use crate::cell::CellValue;
use crate::error::{Result, SheetError};
use crate::sheet::Sheet;
use indexmap::IndexMap;
use serde_json::{Map, Value};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

impl Sheet {
    // =========================================================================
    // JSON (Array of Objects)
    // =========================================================================

    /// Load a sheet from a JSON file containing an array of objects
    ///
    /// # Example
    /// ```no_run
    /// use piptable_sheet::Sheet;
    ///
    /// // File contains: [{"name": "Alice", "age": 30}, {"name": "Bob", "age": 25}]
    /// let sheet = Sheet::from_json("data.json").unwrap();
    /// ```
    pub fn from_json<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path.as_ref())?;
        let reader = BufReader::new(file);
        Self::from_json_reader(reader)
    }

    /// Load a sheet from a JSON string containing an array of objects
    pub fn from_json_str(content: &str) -> Result<Self> {
        Self::from_json_reader(content.as_bytes())
    }

    /// Load a sheet from a reader containing JSON array of objects
    pub fn from_json_reader<R: std::io::Read>(reader: R) -> Result<Self> {
        let value: Value = serde_json::from_reader(reader)
            .map_err(|e| SheetError::Parse(format!("Invalid JSON: {e}")))?;

        let array = value
            .as_array()
            .ok_or_else(|| SheetError::Parse("JSON must be an array of objects".to_string()))?;

        if array.is_empty() {
            return Ok(Sheet::new());
        }

        // Extract column names from first object
        let first_obj = array[0]
            .as_object()
            .ok_or_else(|| SheetError::Parse("Array elements must be objects".to_string()))?;

        let column_names: Vec<String> = first_obj.keys().cloned().collect();

        // Build records
        let mut records: Vec<IndexMap<String, CellValue>> = Vec::with_capacity(array.len());

        for (idx, item) in array.iter().enumerate() {
            let obj = item.as_object().ok_or_else(|| {
                SheetError::Parse(format!("Element at index {idx} must be an object"))
            })?;

            let mut record = IndexMap::new();
            for name in &column_names {
                let value = obj.get(name).unwrap_or(&Value::Null);
                record.insert(name.clone(), json_value_to_cell(value));
            }
            records.push(record);
        }

        Sheet::from_records(records)
    }

    /// Save the sheet to a JSON file as an array of objects
    ///
    /// Requires columns to be named.
    pub fn save_as_json<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        self.write_json(writer, false)
    }

    /// Save the sheet to a JSON file as a pretty-printed array of objects
    pub fn save_as_json_pretty<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        self.write_json(writer, true)
    }

    /// Write the sheet to a writer as JSON
    pub fn write_json<W: Write>(&self, writer: W, pretty: bool) -> Result<()> {
        let records = self.to_records().ok_or_else(|| {
            SheetError::ColumnsNotNamed("Columns must be named to export as JSON".to_string())
        })?;

        // Skip the header row (index 0) since column names become keys
        let json_array: Vec<Map<String, Value>> = records
            .into_iter()
            .skip(1)
            .map(|record| {
                record
                    .into_iter()
                    .map(|(k, v)| (k, cell_to_json_value(&v)))
                    .collect()
            })
            .collect();

        if pretty {
            serde_json::to_writer_pretty(writer, &json_array)
                .map_err(|e| SheetError::Serialize(format!("JSON write error: {e}")))?;
        } else {
            serde_json::to_writer(writer, &json_array)
                .map_err(|e| SheetError::Serialize(format!("JSON write error: {e}")))?;
        }

        Ok(())
    }

    /// Convert the sheet to a JSON string
    ///
    /// # Errors
    ///
    /// Returns error if columns are not named.
    pub fn to_json_string(&self) -> Result<String> {
        let mut buffer = Vec::new();
        self.write_json(&mut buffer, false)?;
        // Safe: serde_json always outputs valid UTF-8
        Ok(String::from_utf8(buffer).expect("JSON output is always valid UTF-8"))
    }

    /// Convert the sheet to a pretty-printed JSON string
    pub fn to_json_string_pretty(&self) -> Result<String> {
        let mut buffer = Vec::new();
        self.write_json(&mut buffer, true)?;
        // Safe: serde_json always outputs valid UTF-8
        Ok(String::from_utf8(buffer).expect("JSON output is always valid UTF-8"))
    }

    // =========================================================================
    // JSONL (JSON Lines / NDJSON)
    // =========================================================================

    /// Load a sheet from a JSONL file (one JSON object per line)
    ///
    /// # Example
    /// ```no_run
    /// use piptable_sheet::Sheet;
    ///
    /// // File contains:
    /// // {"name": "Alice", "age": 30}
    /// // {"name": "Bob", "age": 25}
    /// let sheet = Sheet::from_jsonl("data.jsonl").unwrap();
    /// ```
    pub fn from_jsonl<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path.as_ref())?;
        let reader = BufReader::new(file);
        Self::from_jsonl_reader(reader)
    }

    /// Load a sheet from a JSONL string
    pub fn from_jsonl_str(content: &str) -> Result<Self> {
        Self::from_jsonl_reader(content.as_bytes())
    }

    /// Load a sheet from a reader containing JSONL
    pub fn from_jsonl_reader<R: std::io::Read>(reader: R) -> Result<Self> {
        let buf_reader = BufReader::new(reader);
        let mut records: Vec<IndexMap<String, CellValue>> = Vec::new();
        let mut column_names: Option<Vec<String>> = None;

        for (line_num, line_result) in buf_reader.lines().enumerate() {
            let line = line_result?;
            let trimmed = line.trim();

            // Skip empty lines
            if trimmed.is_empty() {
                continue;
            }

            let value: Value = serde_json::from_str(trimmed)
                .map_err(|e| SheetError::Parse(format!("Invalid JSON on line {}: {e}", line_num + 1)))?;

            let obj = value.as_object().ok_or_else(|| {
                SheetError::Parse(format!("Line {} must be a JSON object", line_num + 1))
            })?;

            // Get column names from first object
            if column_names.is_none() {
                column_names = Some(obj.keys().cloned().collect());
            }

            let names = column_names.as_ref().unwrap();
            let mut record = IndexMap::new();
            for name in names {
                let value = obj.get(name).unwrap_or(&Value::Null);
                record.insert(name.clone(), json_value_to_cell(value));
            }
            records.push(record);
        }

        if records.is_empty() {
            return Ok(Sheet::new());
        }

        Sheet::from_records(records)
    }

    /// Save the sheet to a JSONL file (one JSON object per line)
    ///
    /// Requires columns to be named.
    pub fn save_as_jsonl<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        self.write_jsonl(writer)
    }

    /// Write the sheet to a writer as JSONL
    pub fn write_jsonl<W: Write>(&self, mut writer: W) -> Result<()> {
        let records = self.to_records().ok_or_else(|| {
            SheetError::ColumnsNotNamed("Columns must be named to export as JSONL".to_string())
        })?;

        // Skip the header row (index 0) since column names become keys
        for record in records.into_iter().skip(1) {
            let json_obj: Map<String, Value> = record
                .into_iter()
                .map(|(k, v)| (k, cell_to_json_value(&v)))
                .collect();

            serde_json::to_writer(&mut writer, &json_obj)
                .map_err(|e| SheetError::Serialize(format!("JSON write error: {e}")))?;
            writeln!(writer)?;
        }

        Ok(())
    }

    /// Convert the sheet to a JSONL string
    ///
    /// # Errors
    ///
    /// Returns error if columns are not named.
    pub fn to_jsonl_string(&self) -> Result<String> {
        let mut buffer = Vec::new();
        self.write_jsonl(&mut buffer)?;
        // Safe: serde_json always outputs valid UTF-8
        Ok(String::from_utf8(buffer).expect("JSONL output is always valid UTF-8"))
    }
}

/// Convert a serde_json Value to a CellValue
fn json_value_to_cell(value: &Value) -> CellValue {
    match value {
        Value::Null => CellValue::Null,
        Value::Bool(b) => CellValue::Bool(*b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                CellValue::Int(i)
            } else if let Some(f) = n.as_f64() {
                CellValue::Float(f)
            } else {
                CellValue::String(n.to_string())
            }
        }
        Value::String(s) => CellValue::String(s.clone()),
        // For arrays and objects, convert to string representation
        Value::Array(_) | Value::Object(_) => CellValue::String(value.to_string()),
    }
}

/// Convert a CellValue to a serde_json Value
fn cell_to_json_value(cell: &CellValue) -> Value {
    match cell {
        CellValue::Null => Value::Null,
        CellValue::Bool(b) => Value::Bool(*b),
        CellValue::Int(i) => Value::Number((*i).into()),
        CellValue::Float(f) => {
            // from_f64 returns None for NaN and Infinity
            // Fall back to string representation to preserve data
            serde_json::Number::from_f64(*f)
                .map(Value::Number)
                .unwrap_or_else(|| Value::String(f.to_string()))
        }
        CellValue::String(s) => Value::String(s.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_from_json_str() {
        let json = r#"[
            {"name": "Alice", "age": 30},
            {"name": "Bob", "age": 25}
        ]"#;

        let sheet = Sheet::from_json_str(json).unwrap();

        assert_eq!(sheet.row_count(), 3); // header + 2 data rows
        assert_eq!(sheet.col_count(), 2);
        assert!(sheet.column_names().is_some());
    }

    #[test]
    fn test_from_json_empty() {
        let json = "[]";
        let sheet = Sheet::from_json_str(json).unwrap();
        assert_eq!(sheet.row_count(), 0);
    }

    #[test]
    fn test_to_json_string() {
        let mut sheet = Sheet::from_data(vec![
            vec!["name", "age"],
            vec!["Alice", "30"],
        ]);
        sheet.name_columns_by_row(0).unwrap();

        let json = sheet.to_json_string().unwrap();
        assert!(json.contains("Alice"));
        assert!(json.contains("30"));
    }

    #[test]
    fn test_json_roundtrip() {
        let json = r#"[{"name": "Alice", "age": 30}, {"name": "Bob", "age": 25}]"#;

        let sheet = Sheet::from_json_str(json).unwrap();
        let output = sheet.to_json_string().unwrap();

        let restored = Sheet::from_json_str(&output).unwrap();
        assert_eq!(sheet.row_count(), restored.row_count());
    }

    #[test]
    fn test_from_jsonl_str() {
        let jsonl = r#"{"name": "Alice", "age": 30}
{"name": "Bob", "age": 25}"#;

        let sheet = Sheet::from_jsonl_str(jsonl).unwrap();

        assert_eq!(sheet.row_count(), 3); // header + 2 data rows
        assert!(sheet.column_names().is_some());
    }

    #[test]
    fn test_from_jsonl_empty_lines() {
        let jsonl = r#"{"name": "Alice", "age": 30}

{"name": "Bob", "age": 25}
"#;

        let sheet = Sheet::from_jsonl_str(jsonl).unwrap();
        assert_eq!(sheet.row_count(), 3); // header + 2 data rows
    }

    #[test]
    fn test_to_jsonl_string() {
        let mut sheet = Sheet::from_data(vec![
            vec!["name", "age"],
            vec!["Alice", "30"],
            vec!["Bob", "25"],
        ]);
        sheet.name_columns_by_row(0).unwrap();

        let jsonl = sheet.to_jsonl_string().unwrap();
        let lines: Vec<&str> = jsonl.lines().collect();
        assert_eq!(lines.len(), 2); // 2 data rows (header row is skipped)
    }

    #[test]
    fn test_jsonl_roundtrip() {
        let jsonl = r#"{"name": "Alice", "age": 30}
{"name": "Bob", "age": 25}"#;

        let sheet = Sheet::from_jsonl_str(jsonl).unwrap();
        let output = sheet.to_jsonl_string().unwrap();

        let restored = Sheet::from_jsonl_str(&output).unwrap();
        assert_eq!(sheet.row_count(), restored.row_count());
    }

    #[test]
    fn test_json_file_io() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.json");

        let mut sheet = Sheet::from_data(vec![
            vec!["id", "value"],
            vec!["1", "foo"],
            vec!["2", "bar"],
        ]);
        sheet.name_columns_by_row(0).unwrap();

        sheet.save_as_json(&file_path).unwrap();

        let loaded = Sheet::from_json(&file_path).unwrap();
        assert_eq!(loaded.row_count(), sheet.row_count());
    }

    #[test]
    fn test_jsonl_file_io() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.jsonl");

        let mut sheet = Sheet::from_data(vec![
            vec!["id", "value"],
            vec!["1", "foo"],
            vec!["2", "bar"],
        ]);
        sheet.name_columns_by_row(0).unwrap();

        sheet.save_as_jsonl(&file_path).unwrap();

        let loaded = Sheet::from_jsonl(&file_path).unwrap();
        assert_eq!(loaded.row_count(), sheet.row_count());
    }

    #[test]
    fn test_json_types() {
        let json = r#"[
            {"bool": true, "int": 42, "float": 3.14, "string": "hello", "null": null}
        ]"#;

        let sheet = Sheet::from_json_str(json).unwrap();
        // Check that types are preserved
        let records = sheet.to_records().unwrap();
        let row = &records[1]; // first data row

        assert!(matches!(row.get("bool"), Some(CellValue::Bool(true))));
        assert!(matches!(row.get("int"), Some(CellValue::Int(42))));
    }

    #[test]
    fn test_json_nan_infinity() {
        // NaN and Infinity are converted to string representation in JSON
        let mut sheet = Sheet::new();
        sheet.data_mut().push(vec![
            CellValue::String("value".to_string()),
        ]);
        sheet.data_mut().push(vec![CellValue::Float(f64::NAN)]);
        sheet.data_mut().push(vec![CellValue::Float(f64::INFINITY)]);
        sheet.data_mut().push(vec![CellValue::Float(f64::NEG_INFINITY)]);
        sheet.name_columns_by_row(0).unwrap();

        let json = sheet.to_json_string().unwrap();

        // NaN and Infinity should be represented as strings, not null
        assert!(json.contains("\"NaN\"") || json.contains("\"nan\""));
        assert!(json.contains("\"inf\"") || json.contains("\"Infinity\""));
    }
}
