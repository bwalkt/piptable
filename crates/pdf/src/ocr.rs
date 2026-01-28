use crate::error::{PdfError, Result};
use std::path::Path;
// use tesseract::Tesseract; // Disabled for Phase 1

pub struct OcrEngine {
    #[allow(dead_code)]
    language: String,
}

impl Default for OcrEngine {
    fn default() -> Self {
        Self {
            language: "eng".to_string(),
        }
    }
}

impl OcrEngine {
    pub fn new(language: impl Into<String>) -> Self {
        Self {
            language: language.into(),
        }
    }

    pub fn extract_text_from_image(&self, _image_path: &Path) -> Result<String> {
        // OCR disabled for Phase 1 - focusing on text-based PDFs only
        Err(PdfError::OcrError(
            "OCR not available in Phase 1 implementation".to_string(),
        ))
    }

    pub fn extract_text_from_bytes(&self, _image_data: &[u8]) -> Result<String> {
        // OCR disabled for Phase 1 - focusing on text-based PDFs only
        Err(PdfError::OcrError(
            "OCR not available in Phase 1 implementation".to_string(),
        ))
    }
}
