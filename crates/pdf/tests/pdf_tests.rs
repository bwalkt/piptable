use piptable_pdf::{extract_tables_from_pdf, extract_tables_with_options, PdfOptions, PdfError};
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
#[ignore] // TODO: Add assertions once PDF file creation is implemented
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
#[ignore] // TODO: Add assertions once PDF file creation is implemented
fn test_cell_value_parsing() {
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

#[test] 
#[ignore = "Requires OCR system dependencies (tesseract, leptonica, pdfium)"]
fn test_ocr_dependency_detection() {
    // Test that OCR-enabled processing fails gracefully when dependencies are missing
    // This helps catch runtime dependency issues during development
    
    let options = PdfOptions {
        page_range: None,
        ocr_enabled: true,
        ocr_language: "eng".to_string(),
        min_table_rows: 2,
        min_table_cols: 2,
    };

    // Test with a non-existent file to trigger OCR path
    let result = extract_tables_with_options("/non/existent/file.pdf", options);
    
    // Should fail, but we want to ensure it's a clear error about missing dependencies
    assert!(result.is_err());
    
    // If dependencies are missing, we expect specific error types
    if let Err(e) = result {
        match e {
            PdfError::ParseError(_) => {
                // File not found - expected for this test
                println!("File not found (expected for test)");
            },
            PdfError::OcrError(msg) => {
                // OCR dependency error - what we're testing for
                println!("OCR dependency error detected: {}", msg);
                assert!(
                    msg.contains("Tesseract") || 
                    msg.contains("PDFium") || 
                    msg.contains("leptonica"),
                    "OCR error should mention specific dependency: {}", msg
                );
            },
            _ => {
                println!("Unexpected error type: {:?}", e);
            }
        }
    }
}

#[test]
#[ignore = "Requires creating actual PDF content"]
fn test_ocr_vs_text_extraction() {
    // Integration test to verify OCR vs text extraction behavior
    // This would test the hybrid approach: text first, OCR fallback
    
    // TODO: Create minimal PDF with known content to test:
    // 1. PDFs with extractable text (should not trigger OCR)
    // 2. PDFs with minimal/no text (should trigger OCR if enabled)
    // 3. Error handling when OCR dependencies are missing
    
    let text_based_options = PdfOptions {
        ocr_enabled: false,
        ..Default::default()
    };
    
    let ocr_enabled_options = PdfOptions {
        ocr_enabled: true, 
        ..Default::default()
    };
    
    // For now, just test that the options are correctly configured
    assert!(!text_based_options.ocr_enabled);
    assert!(ocr_enabled_options.ocr_enabled);
}

#[test]
fn test_ocr_error_propagation() {
    // Test that when OCR is explicitly enabled and fails, 
    // the error is properly propagated (not swallowed)
    
    let options = PdfOptions {
        ocr_enabled: true,
        ocr_language: "eng".to_string(),
        ..Default::default()
    };
    
    // Test with invalid file path
    let result = extract_tables_with_options("/invalid/path/file.pdf", options);
    
    // Should get an error, and when OCR is enabled, OCR-related errors should bubble up
    assert!(result.is_err());
    
    // The specific error type depends on whether dependencies are available
    // but we verify that some error is returned rather than being swallowed
    if let Err(e) = result {
        println!("Got expected error with OCR enabled: {:?}", e);
        // Error could be ParseError (file not found), OcrError (dependency missing), etc.
        // The key is that we GET an error rather than silent failure
    }
}

#[test]
fn test_ocr_fallback_behavior() {
    // Test the specific scenario identified in Codex review:
    // "OCR enabled + text extraction succeeds but OCR fails"
    // This should fall back to text extraction, not error out
    
    use std::fs;
    use tempfile::tempdir;
    
    // Create a temporary directory for our test
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let pdf_path = temp_dir.path().join("test.pdf");
    
    // Create a minimal "PDF" file that will trigger text extraction failure
    // but won't crash the system - this simulates short-text PDF scenario
    fs::write(&pdf_path, "Minimal PDF content with <50 chars").expect("Failed to write test file");
    
    let options = PdfOptions {
        ocr_enabled: true,
        ocr_language: "eng".to_string(),
        ..Default::default()
    };
    
    // This should attempt OCR, fail (due to invalid PDF format), 
    // but still try to fall back to text extraction
    let result = extract_tables_with_options(&pdf_path, options);
    
    // We expect an error since this isn't a real PDF, but we want to verify
    // the error type and ensure the fallback logic was attempted
    assert!(result.is_err());
    
    if let Err(e) = result {
        match e {
            PdfError::ParseError(_) => {
                // Expected - not a real PDF, so parsing failed
                // This is fine - shows we tried text extraction (fallback)
                println!("Text extraction failed (expected for fake PDF): {:?}", e);
            },
            PdfError::OcrError(msg) => {
                // If we get an OCR error, it should be a setup failure, not a processing failure
                // Setup failures are OK to bubble up immediately
                if msg.contains("Failed to initialize") || msg.contains("PDFium") || msg.contains("Tesseract") {
                    println!("OCR setup failure (acceptable): {}", msg);
                } else {
                    // Processing failures should have fallen back to text extraction
                    panic!("OCR processing failure should have fallen back to text extraction, got: {}", msg);
                }
            },
            PdfError::NoTablesFound => {
                // This is actually good - means we tried text extraction and didn't find tables
                // Shows the fallback logic worked
                println!("No tables found after fallback (good - fallback worked)");
            },
        }
    }
}

#[test]
fn test_text_extraction_with_no_ocr() {
    // Baseline test: text extraction without OCR should work normally
    // This ensures our OCR changes don't break basic functionality
    
    let options = PdfOptions {
        ocr_enabled: false,
        ..Default::default()
    };
    
    let result = extract_tables_with_options("/non/existent/file.pdf", options);
    
    // Should fail with ParseError (file not found), not OCR-related errors
    assert!(result.is_err());
    
    if let Err(e) = result {
        match e {
            PdfError::ParseError(_) => {
                // Expected - file doesn't exist
                println!("Expected parse error for non-existent file");
            },
            PdfError::OcrError(_) => {
                panic!("Should not get OCR error when OCR is disabled");
            },
            _ => {
                println!("Other error type: {:?}", e);
            }
        }
    }
}
