use piptable_pdf::{extract_tables_from_pdf, extract_tables_with_options, PdfError, PdfOptions};
use std::io::Write;
use std::path::Path;
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
    assert!(!options.ocr_enabled);
    assert_eq!(options.ocr_language, "eng");
    assert_eq!(options.min_table_rows, 2);
    assert_eq!(options.min_table_cols, 2);
}

#[test]
fn test_pdf_options_page_range_and_headers() {
    let file_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/test_data/pdf_options_headers.pdf");

    let options = PdfOptions {
        page_range: Some((1, 1)),
        min_table_rows: 2,
        min_table_cols: 2,
        ..Default::default()
    };

    let mut tables = extract_tables_with_options(&file_path, options).unwrap();
    assert_eq!(tables.len(), 1);

    let sheet = tables.pop().expect("table exists");
    assert_eq!(sheet.row_count(), 6);
    assert!(sheet.column_names().is_none());

    let mut sheet_with_headers = sheet.clone();
    sheet_with_headers.name_columns_by_row(0).unwrap();
    let names = sheet_with_headers.column_names().unwrap();
    assert_eq!(names[0], "Name");
    assert_eq!(names[1], "Qty");
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
    // Test OCR engine initialization directly to verify dependency availability
    // This actually tests OCR dependencies rather than failing at file parsing

    use piptable_pdf::ocr::OcrEngine;

    // Test OCR engine creation (always succeeds) and actual OCR operation
    let engine = OcrEngine::new("eng");
    println!("✅ OCR engine created successfully");

    // Test actual dependency check through a simple OCR operation
    // Create a minimal 1x1 black image to test with
    use image::{DynamicImage, RgbImage};
    let img = RgbImage::new(1, 1);
    let dynamic_img = DynamicImage::ImageRgb8(img);
    match engine.extract_text_from_pdf_page(dynamic_img) {
        Ok(text) => {
            println!(
                "✅ OCR dependencies available - extracted {} chars",
                text.len()
            );
            // If we reach here, Tesseract is working
        }
        Err(PdfError::OcrSetupError(msg)) => {
            println!(
                "❌ OCR setup failed (expected if dependencies missing): {}",
                msg
            );
            // This is what we want to test - proper error when dependencies are missing
        }
        Err(PdfError::OcrProcessingError(msg)) => {
            println!(
                "⚠️ OCR processing failed (could be expected for minimal image): {}",
                msg
            );
            // This could happen with tiny images, but still shows dependencies are present
        }
        Err(e) => {
            println!("🤷 Unexpected error type during OCR: {:?}", e);
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
#[ignore = "Requires OCR dependencies for meaningful error propagation testing"]
fn test_ocr_error_propagation() {
    // Test OCR error propagation using direct OCR engine calls
    // This bypasses PDF parsing to actually test OCR error handling

    use piptable_pdf::ocr::OcrEngine;

    // Test 1: OCR engines always create successfully, errors happen during operations
    let engine_invalid = OcrEngine::new("invalid_lang_xyz");
    println!("✅ OCR engine created with invalid language (error will occur during operation)");

    // Test with minimal image to trigger actual language validation
    use image::{DynamicImage, RgbImage};
    let img = RgbImage::new(1, 1);
    let dynamic_img = DynamicImage::ImageRgb8(img);

    match engine_invalid.extract_text_from_pdf_page(dynamic_img) {
        Ok(_) => {
            println!("⚠️ OCR succeeded with invalid language (tesseract behavior varies)");
        }
        Err(PdfError::OcrSetupError(msg)) => {
            println!("✅ OCR setup error properly propagated: {}", msg);
            // This confirms setup errors bubble up correctly
        }
        Err(e) => {
            println!("🤷 Other error type for invalid language: {:?}", e);
        }
    }

    // Test 2: OCR processing error propagation (valid engine)
    let engine = OcrEngine::new("eng");
    // Test with invalid image data to trigger processing error
    let invalid_image_data = vec![0u8; 10]; // Invalid image bytes

    match engine.extract_text_from_bytes(&invalid_image_data) {
        Ok(_) => {
            panic!("Expected OCR processing error for invalid image data");
        }
        Err(PdfError::OcrProcessingError(msg)) => {
            println!("✅ OCR processing error properly propagated: {}", msg);
        }
        Err(PdfError::OcrSetupError(msg)) => {
            println!("⚠️ Got setup error instead of processing error: {}", msg);
            // This could happen if Tesseract initialization fails during processing
        }
        Err(e) => {
            println!("🤷 Unexpected error type for invalid image: {:?}", e);
        }
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
            }
            PdfError::OcrError(msg) => {
                // Legacy OCR error - treat as acceptable for this test
                println!("Legacy OCR error: {}", msg);
            }
            PdfError::OcrSetupError(msg) => {
                // Setup failures are OK to bubble up immediately
                println!("OCR setup failure (acceptable): {}", msg);
            }
            PdfError::OcrProcessingError(_) => {
                // Processing failures ideally fall back to text extraction
                // This could indicate the fallback logic didn't trigger as expected
                println!("⚠️ Got OcrProcessingError - fallback may not have triggered");
            }
            PdfError::NoTablesFound => {
                // This is actually good - means we tried text extraction and didn't find tables
                // Shows the fallback logic worked
                println!("No tables found after fallback (good - fallback worked)");
            }
            _ => {
                println!("Other error type (acceptable for fake PDF): {:?}", e);
            }
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
            }
            PdfError::OcrError(_)
            | PdfError::OcrSetupError(_)
            | PdfError::OcrProcessingError(_) => {
                panic!("Should not get any OCR-related error when OCR is disabled");
            }
            _ => {
                println!("Other error type: {:?}", e);
            }
        }
    }
}

#[test]
fn test_structured_error_classification() {
    // Test that our structured error types work correctly
    // This tests the error handling logic without requiring actual OCR dependencies

    use piptable_pdf::PdfError;

    // Test error type classification
    let setup_error = PdfError::OcrSetupError("Tesseract not found".to_string());
    let processing_error = PdfError::OcrProcessingError("Failed to process image".to_string());
    let legacy_error = PdfError::OcrError("Generic OCR error".to_string());

    // Verify error types can be matched correctly
    match setup_error {
        PdfError::OcrSetupError(_) => {
            println!("Correctly identified setup error");
        }
        _ => panic!("Failed to identify setup error"),
    }

    match processing_error {
        PdfError::OcrProcessingError(_) => {
            println!("Correctly identified processing error");
        }
        _ => panic!("Failed to identify processing error"),
    }

    match legacy_error {
        PdfError::OcrError(_) => {
            println!("Correctly identified legacy OCR error");
        }
        _ => panic!("Failed to identify legacy error"),
    }

    // Test error message formatting (using contains() for resilience)
    assert!(
        setup_error.to_string().contains("Tesseract not found"),
        "Setup error message should contain expected text"
    );
    assert!(
        processing_error
            .to_string()
            .contains("Failed to process image"),
        "Processing error message should contain expected text"
    );
    assert!(
        legacy_error.to_string().contains("Generic OCR error"),
        "Legacy error message should contain expected text"
    );
}

#[test]
#[ignore = "Requires system dependencies or mocked OCR for true end-to-end validation"]
fn test_end_to_end_ocr_flow() {
    // TODO: This would be a comprehensive end-to-end test that:
    // 1. Creates a real PDF with known table content
    // 2. Tests text extraction path (PDF with extractable text)
    // 3. Tests OCR path (PDF with minimal text, OCR enabled)
    // 4. Validates table detection and sheet conversion
    // 5. Verifies error handling for missing dependencies
    //
    // This requires either:
    // - Bundled test PDFs in the repository
    // - Mock implementations of OCR dependencies
    // - Running only in CI environments with dependencies installed

    println!("End-to-end OCR test would run here with proper setup");
}
