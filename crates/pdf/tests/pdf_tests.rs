use piptable_pdf::{extract_tables_from_pdf, PdfOptions};
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_extract_tables_basic() {
    // This test would require actual PDF files to be meaningful
    // For now, we test that the API works correctly with non-existent files

    let result = extract_tables_from_pdf("/non/existent/file.pdf");
    assert!(result.is_err());
}

#[test]
fn test_extract_tables_with_options() {
    let options = PdfOptions {
        page_range: Some((1, 3)),
        ocr_enabled: false,
        ocr_language: "eng".to_string(),
        min_table_rows: 3,
        min_table_cols: 2,
    };

    let result = piptable_pdf::extract_tables_with_options("/non/existent/file.pdf", options);
    assert!(result.is_err());
}

#[test]
fn test_pdf_options_default() {
    let options = PdfOptions::default();
    assert_eq!(options.page_range, None);
    assert_eq!(options.ocr_enabled, false);
    assert_eq!(options.ocr_language, "eng");
    assert_eq!(options.min_table_rows, 2);
    assert_eq!(options.min_table_cols, 2);
}

// Helper function to create a temporary text file for testing
fn create_temp_text_file(content: &str) -> NamedTempFile {
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    write!(temp_file, "{}", content).expect("Failed to write to temp file");
    temp_file
}

#[test]
fn test_table_detection_basic() {
    // This would require creating actual PDF files to test properly
    // For Phase 1, we'll focus on the API structure
    let table_text = "Name    Age    City\nJohn    25     NYC\nJane    30     LA";
    let _test_file = create_temp_text_file(table_text);

    // In a full implementation, we would test:
    // 1. PDF text extraction
    // 2. Table detection from extracted text
    // 3. Cell value parsing
    // 4. Sheet conversion
}

#[test]
fn test_cell_value_parsing() {
    use piptable_sheet::CellValue;

    // Test the private parse_cell_value function via public API
    // We'll create a minimal table to test parsing
    let _table_text =
        "Integer    Float    Boolean    Text    Empty\n123    45.67    true    hello    \n";

    // This would test that our cell parsing correctly identifies:
    // - Integers: 123 -> CellValue::Int(123)
    // - Floats: 45.67 -> CellValue::Float(45.67)
    // - Booleans: true -> CellValue::Bool(true)
    // - Strings: hello -> CellValue::String("hello")
    // - Empty/Null: "" -> CellValue::Null
}
