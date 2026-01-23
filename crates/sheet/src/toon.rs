//! TOON (Token-Oriented Object Notation) support for Sheet
//!
//! TOON is a compact, LLM-friendly format that minimizes tokens while remaining human-readable.
//! See: <https://github.com/toon-format/toon>
//!
//! # Format
//!
//! TOON uses a tabular array format for sheet data:
//!
//! ```text
//! rows[2]{name,age,city}:
//!   Alice,30,NYC
//!   Bob,25,LA
//! ```
//!
//! This is ~40% more token-efficient than equivalent JSON.

use crate::cell::CellValue;
use crate::error::{Result, SheetError};
use crate::sheet::Sheet;
use indexmap::IndexMap;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::Path;

/// Default name for the array in TOON format
const DEFAULT_ARRAY_NAME: &str = "rows";

impl Sheet {
    /// Load a sheet from a TOON file
    ///
    /// # Example
    /// ```no_run
    /// use piptable_sheet::Sheet;
    ///
    /// // File contains:
    /// // rows[2]{name,age}:
    /// //   Alice,30
    /// //   Bob,25
    /// let sheet = Sheet::from_toon("data.toon").unwrap();
    /// ```
    pub fn from_toon<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path.as_ref())?;
        let reader = BufReader::new(file);
        Self::from_toon_reader(reader)
    }

    /// Load a sheet from a TOON string
    pub fn from_toon_str(content: &str) -> Result<Self> {
        Self::from_toon_reader(content.as_bytes())
    }

    /// Load a sheet from a reader containing TOON data
    pub fn from_toon_reader<R: Read>(reader: R) -> Result<Self> {
        let buf_reader = BufReader::new(reader);
        let mut lines = buf_reader.lines();

        // Parse header line: name[count]{field1,field2,...}:
        let header_line = lines
            .next()
            .ok_or_else(|| SheetError::Parse("Empty TOON file".to_string()))??;

        let (column_names, expected_count) = parse_toon_header(&header_line)?;

        if column_names.is_empty() {
            return Ok(Sheet::new());
        }

        // Parse data rows
        let mut records: Vec<IndexMap<String, CellValue>> = Vec::new();

        for (line_num, line_result) in lines.enumerate() {
            let line = line_result?;
            let trimmed = line.trim();

            // Skip empty lines
            if trimmed.is_empty() {
                continue;
            }

            let values = parse_toon_row(trimmed)?;

            if values.len() != column_names.len() {
                return Err(SheetError::Parse(format!(
                    "Row {} has {} values, expected {} (columns: {:?})",
                    line_num + 1,
                    values.len(),
                    column_names.len(),
                    column_names
                )));
            }

            let mut record = IndexMap::new();
            for (name, value) in column_names.iter().zip(values.into_iter()) {
                record.insert(name.clone(), value);
            }
            records.push(record);
        }

        // Validate count if specified
        if let Some(count) = expected_count {
            if records.len() != count {
                return Err(SheetError::Parse(format!(
                    "Expected {} rows, got {}",
                    count,
                    records.len()
                )));
            }
        }

        if records.is_empty() {
            // Return sheet with just column names
            let mut sheet = Sheet::new();
            let header_row: Vec<CellValue> =
                column_names.into_iter().map(CellValue::String).collect();
            *sheet.data_mut() = vec![header_row];
            sheet.name_columns_by_row(0)?;
            return Ok(sheet);
        }

        Sheet::from_records(records)
    }

    /// Save the sheet to a TOON file
    ///
    /// Requires columns to be named.
    pub fn save_as_toon<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        self.write_toon(writer)
    }

    /// Write the sheet to a writer as TOON
    pub fn write_toon<W: Write>(&self, mut writer: W) -> Result<()> {
        let names = self.column_names().ok_or_else(|| {
            SheetError::ColumnsNotNamed("Columns must be named to export as TOON".to_string())
        })?;

        let records = self.to_records().ok_or_else(|| {
            SheetError::ColumnsNotNamed("Columns must be named to export as TOON".to_string())
        })?;

        // Exclude header row from count (from_records includes it)
        let data_rows = if records.is_empty() {
            0
        } else {
            records.len() - 1
        };

        // Write header: rows[count]{field1,field2,...}:
        write!(writer, "{}[{}]{{", DEFAULT_ARRAY_NAME, data_rows)?;
        for (i, name) in names.iter().enumerate() {
            if i > 0 {
                write!(writer, ",")?;
            }
            write!(writer, "{}", validate_toon_field(name)?)?;
        }
        writeln!(writer, "}}:")?;

        // Write data rows (skip header row which is at index 0)
        for record in records.iter().skip(1) {
            write!(writer, "  ")?;
            for (i, name) in names.iter().enumerate() {
                if i > 0 {
                    write!(writer, ",")?;
                }
                let value = record.get(name).unwrap_or(&CellValue::Null);
                write!(writer, "{}", format_toon_value(value))?;
            }
            writeln!(writer)?;
        }

        Ok(())
    }

    /// Convert the sheet to a TOON string
    ///
    /// # Errors
    ///
    /// Returns error if columns are not named.
    pub fn to_toon_string(&self) -> Result<String> {
        let mut buffer = Vec::new();
        self.write_toon(&mut buffer)?;
        // Safe: we only write ASCII-compatible UTF-8 content
        Ok(String::from_utf8(buffer).expect("TOON output is always valid UTF-8"))
    }
}

/// Parse a TOON header line: name[count]{field1,field2,...}:
fn parse_toon_header(line: &str) -> Result<(Vec<String>, Option<usize>)> {
    let line = line.trim();

    // Find the bracket positions
    let open_bracket = line
        .find('[')
        .ok_or_else(|| SheetError::Parse("Invalid TOON header: missing '['".to_string()))?;

    let close_bracket = line
        .find(']')
        .ok_or_else(|| SheetError::Parse("Invalid TOON header: missing ']'".to_string()))?;

    let open_brace = line
        .find('{')
        .ok_or_else(|| SheetError::Parse("Invalid TOON header: missing '{'".to_string()))?;

    let close_brace = line
        .find('}')
        .ok_or_else(|| SheetError::Parse("Invalid TOON header: missing '}'".to_string()))?;

    // Validate structure
    if open_bracket > close_bracket || close_bracket > open_brace || open_brace > close_brace {
        return Err(SheetError::Parse(
            "Invalid TOON header structure".to_string(),
        ));
    }

    // Must end with ':'
    if !line.ends_with(':') {
        return Err(SheetError::Parse(
            "TOON header must end with ':'".to_string(),
        ));
    }

    // Parse count (optional)
    let count_str = &line[open_bracket + 1..close_bracket];
    let count = if count_str.is_empty() {
        None
    } else {
        Some(
            count_str
                .parse::<usize>()
                .map_err(|_| SheetError::Parse(format!("Invalid row count: '{count_str}'")))?,
        )
    };

    // Parse field names
    let fields_str = &line[open_brace + 1..close_brace];
    let fields: Vec<String> = if fields_str.is_empty() {
        Vec::new()
    } else {
        fields_str
            .split(',')
            .map(|s| s.trim().to_string())
            .collect()
    };

    Ok((fields, count))
}

/// Parse a TOON data row (comma-separated values)
fn parse_toon_row(line: &str) -> Result<Vec<CellValue>> {
    let mut values = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' if !in_quotes => {
                in_quotes = true;
            }
            '"' if in_quotes => {
                // Check for escaped quote
                if chars.peek() == Some(&'"') {
                    current.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            }
            ',' if !in_quotes => {
                values.push(parse_toon_value(&current));
                current.clear();
            }
            _ => {
                current.push(c);
            }
        }
    }

    // Don't forget the last value
    values.push(parse_toon_value(&current));

    Ok(values)
}

/// Parse a single TOON value into a CellValue
fn parse_toon_value(s: &str) -> CellValue {
    let trimmed = s.trim();

    // Empty means null
    if trimmed.is_empty() {
        return CellValue::Null;
    }

    // Check for explicit null
    if trimmed.eq_ignore_ascii_case("null") {
        return CellValue::Null;
    }

    // Check for boolean
    if trimmed.eq_ignore_ascii_case("true") {
        return CellValue::Bool(true);
    }
    if trimmed.eq_ignore_ascii_case("false") {
        return CellValue::Bool(false);
    }

    // Check for integer
    if let Ok(i) = trimmed.parse::<i64>() {
        return CellValue::Int(i);
    }

    // Check for float
    if let Ok(f) = trimmed.parse::<f64>() {
        return CellValue::Float(f);
    }

    // Default to string
    CellValue::String(trimmed.to_string())
}

/// Format a CellValue for TOON output
fn format_toon_value(value: &CellValue) -> String {
    match value {
        CellValue::Null => String::new(),
        CellValue::Bool(b) => b.to_string(),
        CellValue::Int(i) => i.to_string(),
        CellValue::Float(f) => f.to_string(),
        CellValue::String(s) => {
            // Quote strings that contain commas, newlines, or quotes
            if s.contains(',') || s.contains('\n') || s.contains('"') {
                format!("\"{}\"", s.replace('"', "\"\""))
            } else {
                s.clone()
            }
        }
    }
}

/// Validate and return a field name for TOON header
/// Field names cannot contain special characters: [ ] { } , :
fn validate_toon_field(name: &str) -> Result<&str> {
    const INVALID_CHARS: &[char] = &['[', ']', '{', '}', ',', ':'];
    if name.chars().any(|c| INVALID_CHARS.contains(&c)) {
        return Err(SheetError::Parse(format!(
            "TOON field name '{}' contains unsupported characters (cannot contain [ ] {{ }} , :)",
            name
        )));
    }
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_parse_toon_header() {
        let (fields, count) = parse_toon_header("rows[2]{name,age,city}:").unwrap();
        assert_eq!(fields, vec!["name", "age", "city"]);
        assert_eq!(count, Some(2));
    }

    #[test]
    fn test_parse_toon_header_no_count() {
        let (fields, count) = parse_toon_header("data[]{id,value}:").unwrap();
        assert_eq!(fields, vec!["id", "value"]);
        assert_eq!(count, None);
    }

    #[test]
    fn test_from_toon_str() {
        let toon = r#"rows[2]{name,age,city}:
  Alice,30,NYC
  Bob,25,LA"#;

        let sheet = Sheet::from_toon_str(toon).unwrap();

        assert_eq!(sheet.row_count(), 3); // header + 2 data rows
        assert_eq!(sheet.col_count(), 3);
        assert!(sheet.column_names().is_some());
    }

    #[test]
    fn test_from_toon_types() {
        let toon = r#"data[1]{bool,int,float,string,null}:
  true,42,3.14,hello,"#;

        let sheet = Sheet::from_toon_str(toon).unwrap();
        let records = sheet.to_records().unwrap();
        let row = &records[1]; // first data row

        assert!(matches!(row.get("bool"), Some(CellValue::Bool(true))));
        assert!(matches!(row.get("int"), Some(CellValue::Int(42))));
        assert!(matches!(row.get("float"), Some(CellValue::Float(f)) if (*f - 3.14).abs() < 0.001));
        assert!(matches!(row.get("string"), Some(CellValue::String(s)) if s == "hello"));
        assert!(matches!(row.get("null"), Some(CellValue::Null)));
    }

    #[test]
    fn test_to_toon_string() {
        let mut sheet = Sheet::from_data(vec![
            vec!["name", "age"],
            vec!["Alice", "30"],
            vec!["Bob", "25"],
        ]);
        sheet.name_columns_by_row(0).unwrap();

        let toon = sheet.to_toon_string().unwrap();

        assert!(toon.starts_with("rows[2]{name,age}:"));
        assert!(toon.contains("Alice,30"));
        assert!(toon.contains("Bob,25"));
    }

    #[test]
    fn test_toon_roundtrip() {
        let toon = r#"rows[2]{name,age}:
  Alice,30
  Bob,25"#;

        let sheet = Sheet::from_toon_str(toon).unwrap();
        let output = sheet.to_toon_string().unwrap();

        let restored = Sheet::from_toon_str(&output).unwrap();
        assert_eq!(sheet.row_count(), restored.row_count());
        assert_eq!(sheet.col_count(), restored.col_count());
    }

    #[test]
    fn test_toon_with_quotes() {
        let toon = r#"rows[1]{name,description}:
  Product,"A great, wonderful item""#;

        let sheet = Sheet::from_toon_str(toon).unwrap();
        let records = sheet.to_records().unwrap();
        let desc = records[1].get("description").unwrap();
        assert_eq!(
            desc,
            &CellValue::String("A great, wonderful item".to_string())
        );
    }

    #[test]
    fn test_toon_empty() {
        let toon = "data[0]{id,name}:";
        let sheet = Sheet::from_toon_str(toon).unwrap();
        assert!(sheet.column_names().is_some());
    }

    #[test]
    fn test_toon_file_io() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.toon");

        let mut sheet = Sheet::from_data(vec![
            vec!["id", "value"],
            vec!["1", "foo"],
            vec!["2", "bar"],
        ]);
        sheet.name_columns_by_row(0).unwrap();

        sheet.save_as_toon(&file_path).unwrap();

        let loaded = Sheet::from_toon(&file_path).unwrap();
        assert_eq!(loaded.row_count(), sheet.row_count());
    }

    #[test]
    fn test_toon_wrong_count() {
        let toon = r#"rows[5]{name,age}:
  Alice,30
  Bob,25"#;

        let result = Sheet::from_toon_str(toon);
        assert!(result.is_err());
    }

    #[test]
    fn test_toon_value_formatting() {
        let mut sheet = Sheet::from_data(vec![vec!["name", "value"]]);
        sheet.name_columns_by_row(0).unwrap();

        // Add a row with a comma in string
        let mut row = Vec::new();
        row.push(CellValue::String("test, with comma".to_string()));
        row.push(CellValue::Int(42));
        sheet.data_mut().push(row);

        let toon = sheet.to_toon_string().unwrap();
        // String with comma should be quoted
        assert!(toon.contains("\"test, with comma\""));
    }

    #[test]
    fn test_toon_invalid_field_names() {
        // Field names containing special characters should fail
        let mut sheet = Sheet::new();
        sheet.data_mut().push(vec![
            CellValue::String("name[0]".to_string()),
            CellValue::String("value".to_string()),
        ]);
        sheet.data_mut().push(vec![
            CellValue::String("test".to_string()),
            CellValue::Int(42),
        ]);
        sheet.name_columns_by_row(0).unwrap();

        let result = sheet.to_toon_string();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("unsupported characters"));
    }
}
