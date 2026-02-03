use crate::error::{PdfError, Result};
use image::{DynamicImage, ImageFormat};
use std::path::Path;
use tesseract::Tesseract;
use tracing::{debug, warn};

pub struct OcrEngine {
    language: String,
    dpi: u32,
}

impl Default for OcrEngine {
    /// Returns a default OCR engine configuration.
    fn default() -> Self {
        Self {
            language: "eng".to_string(),
            dpi: 300,
        }
    }
}

impl OcrEngine {
    pub fn new(language: impl Into<String>) -> Self {
        Self {
            language: language.into(),
            dpi: 300,
        }
    }

    pub fn with_dpi(mut self, dpi: u32) -> Self {
        self.dpi = dpi;
        self
    }

    /// Extract text from an image file using OCR
    pub fn extract_text_from_image(&self, image_path: &Path) -> Result<String> {
        debug!("Starting OCR extraction from image: {:?}", image_path);

        // Load and preprocess the image
        let image = image::open(image_path)
            .map_err(|e| PdfError::OcrProcessingError(format!("Failed to load image: {}", e)))?;

        self.extract_text_from_processed_image(image)
    }

    /// Extract text from image bytes using OCR
    pub fn extract_text_from_bytes(&self, image_data: &[u8]) -> Result<String> {
        debug!("Starting OCR extraction from {} bytes", image_data.len());

        // Load image from bytes
        let image = image::load_from_memory(image_data)
            .map_err(|e| PdfError::OcrProcessingError(format!("Failed to decode image: {}", e)))?;

        self.extract_text_from_processed_image(image)
    }

    /// Extract text from a PDF page that's been rendered to an image
    pub fn extract_text_from_pdf_page(&self, page_image: DynamicImage) -> Result<String> {
        debug!("Starting OCR extraction from PDF page image");
        self.extract_text_from_processed_image(page_image)
    }

    /// Shared OCR pipeline: preprocess image → encode → tesseract → extract text
    fn extract_text_from_processed_image(&self, image: DynamicImage) -> Result<String> {
        // Preprocess the image for better OCR results
        let processed = self.preprocess_image(image)?;

        // Convert to bytes for Tesseract
        let mut buffer = Vec::new();
        processed
            .write_to(&mut std::io::Cursor::new(&mut buffer), ImageFormat::Png)
            .map_err(|e| PdfError::OcrProcessingError(format!("Failed to encode image: {}", e)))?;

        // Initialize Tesseract
        let tesseract = Tesseract::new(None, Some(&self.language)).map_err(|e| {
            PdfError::OcrSetupError(format!("Failed to initialize Tesseract: {}", e))
        })?;

        // Perform OCR
        let text = tesseract
            .set_image_from_mem(&buffer)
            .map_err(|e| PdfError::OcrProcessingError(format!("Failed to set image: {}", e)))?
            .set_source_resolution(self.dpi as i32)
            .get_text()
            .map_err(|e| PdfError::OcrProcessingError(format!("OCR extraction failed: {}", e)))?;

        debug!("OCR extracted {} characters", text.len());
        Ok(text)
    }

    /// Preprocess image for better OCR results
    fn preprocess_image(&self, image: DynamicImage) -> Result<DynamicImage> {
        debug!("Preprocessing image for OCR");

        // Convert to grayscale for better OCR accuracy
        let mut processed = image.grayscale();

        // Enhance contrast
        processed = processed.adjust_contrast(20.0);

        // Ensure minimum DPI (scale up if needed)
        let (width, height) = (processed.width(), processed.height());
        let min_dimension = 2000; // Minimum pixel dimension for good OCR

        if width < min_dimension || height < min_dimension {
            // Cap scaling to prevent memory issues with extreme aspect ratios
            let scale = (min_dimension as f32 / width.min(height) as f32).min(4.0);
            let new_width = (width as f32 * scale) as u32;
            let new_height = (height as f32 * scale) as u32;

            debug!(
                "Scaling image from {}x{} to {}x{}",
                width, height, new_width, new_height
            );
            processed =
                processed.resize(new_width, new_height, image::imageops::FilterType::Lanczos3);
        }

        Ok(processed)
    }

    /// Check if a PDF page likely needs OCR (is it scanned/image-based?)
    ///
    /// **Design Intent**: Primary trigger is minimal text extraction (< 50 chars),
    /// indicating likely scanned/image-based PDF that needs OCR processing.
    ///
    /// **Logic**:
    /// 1. If text extraction yielded sufficient content (≥50 chars) → no OCR needed
    /// 2. If text is minimal AND we have a page image → check image variance to reduce false positives
    /// 3. If text is minimal AND no image provided → assume OCR needed
    ///
    /// This ensures OCR triggers reliably for scanned PDFs called from the extractor
    /// with `needs_ocr(&text, None)` while still supporting image-based refinement
    /// when page images are available.
    pub fn needs_ocr(text: &str, page_image: Option<&DynamicImage>) -> bool {
        // If we got very little or no text from regular extraction
        let text_is_minimal = text.trim().len() < 50;

        // Primary trigger: minimal text extraction indicates likely scanned PDF
        if text_is_minimal {
            // If we have an image, do additional variance check to reduce false positives
            if let Some(img) = page_image {
                let gray = img.to_luma8();
                let pixels: Vec<u8> = gray.as_raw().clone();

                if !pixels.is_empty() {
                    let mean = pixels.iter().map(|&p| p as f32).sum::<f32>() / pixels.len() as f32;
                    let variance = pixels
                        .iter()
                        .map(|&p| {
                            let diff = p as f32 - mean;
                            diff * diff
                        })
                        .sum::<f32>()
                        / pixels.len() as f32;

                    // High variance suggests text content, low variance suggests blank page
                    let has_content = variance > 1000.0;
                    if has_content {
                        warn!(
                            "Page needs OCR: minimal text ({} chars) with high-variance image (variance: {:.1})",
                            text.trim().len(), variance
                        );
                    }
                    return has_content;
                }
            }

            // No image available or empty image - rely solely on minimal text detection
            warn!(
                "Page needs OCR: minimal text extraction ({} chars)",
                text.trim().len()
            );
            true
        } else {
            false
        }
    }
}

/// Tests for OCR engine configuration.
#[cfg(test)]
mod tests {
    use super::*;

    /// Ensures the default OCR engine initializes.
    #[test]
    fn test_ocr_engine_creation() {
        let engine = OcrEngine::default();
        assert_eq!(engine.language, "eng");
        assert_eq!(engine.dpi, 300);
    }

    /// Ensures custom language selection is stored.
    #[test]
    fn test_ocr_engine_with_language() {
        let engine = OcrEngine::new("fra");
        assert_eq!(engine.language, "fra");
    }

    /// Ensures DPI configuration is stored.
    #[test]
    fn test_ocr_engine_with_dpi() {
        let engine = OcrEngine::default().with_dpi(600);
        assert_eq!(engine.dpi, 600);
    }

    /// Ensures OCR necessity detection works for scanned PDFs.
    #[test]
    fn test_needs_ocr_detection() {
        // Minimal text should trigger OCR need
        assert!(OcrEngine::needs_ocr("Short text", None));

        // Sufficient text should not need OCR
        let long_text = "This is a much longer piece of text that clearly came from text extraction and doesn't need OCR processing.";
        assert!(!OcrEngine::needs_ocr(long_text, None));
    }
}
