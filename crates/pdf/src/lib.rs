pub mod detector;
pub mod error;
pub mod extractor;
pub mod ocr;

use detector::TableRegion;
use error::Result;
use extractor::PdfExtractor;
use piptable_sheet::{CellValue, Sheet};
use std::path::Path;

/// Extract tables from a PDF file using default options
pub fn extract_tables_from_pdf<P: AsRef<Path>>(path: P) -> Result<Vec<Sheet>> {
    extract_tables_with_options(path, extractor::PdfOptions::default())
}

/// Extract tables from a PDF file with custom options
pub fn extract_tables_with_options<P: AsRef<Path>>(
    path: P,
    options: extractor::PdfOptions,
) -> Result<Vec<Sheet>> {
    let extractor = PdfExtractor::new(options);
    let table_regions = extractor.extract_tables_from_path(path.as_ref())?;
    
    let sheets: Vec<Sheet> = table_regions
        .into_iter()
        .map(convert_table_to_sheet)
        .collect();
    
    if sheets.is_empty() {
        Err(error::PdfError::NoTablesFound)
    } else {
        Ok(sheets)
    }
}

/// Convert a detected table region to a Sheet
fn convert_table_to_sheet(table: TableRegion) -> Sheet {
    let mut sheet = Sheet::new();
    
    // Convert each row and append to sheet
    for row_data in table.rows.iter() {
        let row_values: Vec<CellValue> = row_data
            .iter()
            .map(|cell_data| parse_cell_value(cell_data))
            .collect();
        
        // Use row_append which properly handles sheet expansion
        // Ignore error for empty rows (shouldn't happen with valid tables)
        let _ = sheet.row_append(row_values);
    }
    
    // Try to detect and set column headers
    if !table.rows.is_empty() {
        let first_row = &table.rows[0];
        let detector = detector::TableDetector::default();
        
        if detector.is_likely_header(first_row) {
            // TODO: Add column naming support in future phase
            // Would need to remove first row and set as column names
        }
    }
    
    sheet
}

/// Parse a cell value from string, attempting to detect the appropriate type
fn parse_cell_value(text: &str) -> CellValue {
    let trimmed = text.trim();
    
    // Check for empty/null
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("null") || trimmed.eq_ignore_ascii_case("n/a") {
        return CellValue::Null;
    }
    
    // Try to parse as boolean
    if trimmed.eq_ignore_ascii_case("true") {
        return CellValue::Bool(true);
    }
    if trimmed.eq_ignore_ascii_case("false") {
        return CellValue::Bool(false);
    }
    
    // Try to parse as integer
    if let Ok(int_val) = trimmed.parse::<i64>() {
        return CellValue::Int(int_val);
    }
    
    // Try to parse as float
    if let Ok(float_val) = trimmed.parse::<f64>() {
        return CellValue::Float(float_val);
    }
    
    // Default to string
    CellValue::String(trimmed.to_string())
}

// Re-export commonly used types
pub use error::PdfError;
pub use extractor::PdfOptions;