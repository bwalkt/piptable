pub mod error;
mod table;

use error::{MarkdownError, Result};
use piptable_sheet::{CellValue, Sheet};

pub use table::{MarkdownOptions, MarkdownTable, MarkdownTables};

/// Extract all tables from a markdown string as Sheets.
pub fn extract_tables(markdown: &str) -> Result<Vec<Sheet>> {
    let tables = MarkdownTables::from_markdown(markdown)?;
    let mut sheets = Vec::new();

    for table in tables.tables {
        sheets.push(table.to_sheet()?);
    }

    if sheets.is_empty() {
        Err(MarkdownError::NoTablesFound)
    } else {
        Ok(sheets)
    }
}

/// Extract all tables from a markdown string with options.
pub fn extract_tables_with_options(markdown: &str, options: MarkdownOptions) -> Result<Vec<Sheet>> {
    let tables = MarkdownTables::from_markdown_with_options(markdown, options)?;
    let mut sheets = Vec::new();

    for table in tables.tables {
        sheets.push(table.to_sheet()?);
    }

    if sheets.is_empty() {
        Err(MarkdownError::NoTablesFound)
    } else {
        Ok(sheets)
    }
}

/// Convert a markdown table to a Sheet.
fn table_to_sheet(table: &MarkdownTable) -> Result<Sheet> {
    let mut sheet = Sheet::new();

    if let Some(headers) = &table.headers {
        let header_row: Vec<CellValue> = headers.iter().map(|s| to_cell_value(s)).collect();
        sheet.row_append(header_row)?;
    }

    for row in &table.rows {
        let row_values: Vec<CellValue> = row.iter().map(|s| to_cell_value(s)).collect();
        sheet.row_append(row_values)?;
    }

    Ok(sheet)
}

fn to_cell_value(text: &str) -> CellValue {
    let trimmed = text.trim();

    if trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("null")
        || trimmed.eq_ignore_ascii_case("n/a")
    {
        return CellValue::Null;
    }

    if trimmed.eq_ignore_ascii_case("true") {
        return CellValue::Bool(true);
    }
    if trimmed.eq_ignore_ascii_case("false") {
        return CellValue::Bool(false);
    }

    if let Ok(int_val) = trimmed.parse::<i64>() {
        return CellValue::Int(int_val);
    }

    if let Ok(float_val) = trimmed.parse::<f64>() {
        return CellValue::Float(float_val);
    }

    CellValue::String(trimmed.to_string())
}

impl MarkdownTable {
    pub fn to_sheet(&self) -> Result<Sheet> {
        table_to_sheet(self)
    }
}
