use crate::detector::{TableDetector, TableRegion};
use crate::error::{PdfError, Result};
use crate::ocr::OcrEngine;
use lopdf::Document;
use pdf_extract::extract_text;
use pdfium_render::prelude::*;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct PdfOptions {
    pub page_range: Option<(usize, usize)>,
    pub ocr_enabled: bool,
    pub ocr_language: String,
    pub min_table_rows: usize,
    pub min_table_cols: usize,
}

impl Default for PdfOptions {
    fn default() -> Self {
        Self {
            page_range: None,
            ocr_enabled: false,
            ocr_language: "eng".to_string(),
            min_table_rows: 2,
            min_table_cols: 2,
        }
    }
}

pub struct PdfExtractor {
    options: PdfOptions,
    detector: TableDetector,
    ocr_engine: Option<OcrEngine>,
}

impl PdfExtractor {
    pub fn new(options: PdfOptions) -> Self {
        let detector = TableDetector::new(options.min_table_rows, options.min_table_cols);
        let ocr_engine = if options.ocr_enabled {
            Some(OcrEngine::new(&options.ocr_language))
        } else {
            None
        };

        Self {
            options,
            detector,
            ocr_engine,
        }
    }

    pub fn extract_tables_from_path(&self, path: &Path) -> Result<Vec<TableRegion>> {
        // First try to extract text directly
        let text = self.extract_text_from_pdf(path)?;

        // Check if we need OCR based on the amount of text extracted
        let content_needs_ocr = OcrEngine::needs_ocr(&text, None);
        let ocr_available = self.ocr_engine.is_some();

        if content_needs_ocr && ocr_available {
            // If minimal text found and OCR is enabled, try OCR extraction
            tracing::info!(
                "Minimal text found ({} chars), attempting OCR extraction",
                text.trim().len()
            );

            // Try OCR extraction first
            match self.extract_tables_with_ocr(path) {
                Ok(ocr_tables) if !ocr_tables.is_empty() => {
                    tracing::info!("Found {} tables via OCR", ocr_tables.len());
                    return Ok(ocr_tables);
                }
                Ok(_) => {
                    tracing::warn!("OCR completed but found no tables");
                }
                Err(e) => {
                    tracing::warn!("OCR extraction failed: {}", e);
                    // Fall back to using whatever text we got
                }
            }
        }

        // Detect tables from extracted text (either original or after failed OCR)
        let tables = self.detector.detect_tables(&text);

        if tables.is_empty() {
            if content_needs_ocr && !ocr_available {
                Err(PdfError::OcrError(
                    "No tables found. This appears to be a scanned PDF - enable OCR for better results".to_string()
                ))
            } else {
                Err(PdfError::NoTablesFound)
            }
        } else {
            Ok(tables)
        }
    }

    fn extract_text_from_pdf(&self, path: &Path) -> Result<String> {
        // Use lopdf when a page range is requested, so the range is honored
        if self.options.page_range.is_some() {
            return self.extract_text_with_lopdf(path);
        }

        // Use pdf-extract for full-document extraction
        match extract_text(path) {
            Ok(text) => Ok(text),
            Err(e) => {
                // Try alternative method with lopdf
                tracing::warn!("pdf-extract failed, trying lopdf: {}", e);
                self.extract_text_with_lopdf(path)
            }
        }
    }

    fn extract_text_with_lopdf(&self, path: &Path) -> Result<String> {
        let doc = Document::load(path)
            .map_err(|e| PdfError::ParseError(format!("Failed to load PDF: {}", e)))?;

        let mut all_text = String::new();
        let mut any_page_extracted = false;
        let mut last_error: Option<String> = None;
        let pages = doc.get_pages();

        let (start, end) = if let Some((s, e)) = self.options.page_range {
            // Validate page range
            if s > e {
                return Err(PdfError::InvalidPageRange(format!(
                    "Start page {} is greater than end page {}",
                    s, e
                )));
            }
            if s < 1 {
                return Err(PdfError::InvalidPageRange(
                    "Page numbers must be >= 1".to_string(),
                ));
            }
            let clamped_end = e.min(pages.len());
            if s > clamped_end {
                return Err(PdfError::InvalidPageRange(format!(
                    "Start page {} exceeds document length of {} pages",
                    s,
                    pages.len()
                )));
            }
            (s, clamped_end)
        } else {
            (1, pages.len())
        };

        for page_num in start..=end {
            // Extract text from page using lopdf
            // lopdf expects page numbers directly
            match doc.extract_text(&[page_num as u32]) {
                Ok(content) => {
                    any_page_extracted = true;
                    all_text.push_str(&content);
                    all_text.push('\n');
                }
                Err(e) => {
                    last_error = Some(e.to_string());
                    tracing::warn!("lopdf extract_text failed on page {}: {}", page_num, e);
                }
            }
        }

        // If no pages were successfully extracted, return an error
        if !any_page_extracted {
            let error_suffix = last_error
                .as_ref()
                .map(|e| format!(": {}", e))
                .unwrap_or_default();
            return Err(PdfError::ParseError(format!(
                "Failed to extract text from any page{}",
                error_suffix
            )));
        }

        Ok(all_text)
    }

    fn extract_tables_with_ocr(&self, path: &Path) -> Result<Vec<TableRegion>> {
        let ocr_engine = self
            .ocr_engine
            .as_ref()
            .ok_or_else(|| PdfError::OcrError("OCR engine not initialized".to_string()))?;

        tracing::info!("Attempting OCR extraction from PDF: {:?}", path);

        // Initialize PDFium library
        let pdfium = Pdfium::new(
            Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
                .or_else(|_| Pdfium::bind_to_system_library())
                .map_err(|e| PdfError::OcrError(format!("Failed to initialize PDFium: {}", e)))?,
        );

        // Load the PDF document
        let document = pdfium
            .load_pdf_from_file(path, None)
            .map_err(|e| PdfError::OcrError(format!("Failed to load PDF for OCR: {}", e)))?;

        let mut all_ocr_text = String::new();
        let total_pages = document.pages().len();

        let (start, end) = if let Some((s, e)) = self.options.page_range {
            // Convert to 0-based indexing for pdfium
            let start_idx = s.saturating_sub(1);
            let end_idx = e.min(total_pages).saturating_sub(1);
            (start_idx, end_idx)
        } else {
            (0, total_pages.saturating_sub(1))
        };

        tracing::info!("Processing pages {}-{} for OCR", start + 1, end + 1);

        for page_index in start..=end {
            tracing::debug!("Processing page {} for OCR", page_index + 1);

            // Get the page
            let page = document.pages().get(page_index).map_err(|e| {
                PdfError::OcrError(format!("Failed to get page {}: {}", page_index + 1, e))
            })?;

            // Render page to image at high DPI for better OCR quality
            let render_config = PdfRenderConfig::new()
                .set_target_width(2000) // High resolution for OCR
                .set_maximum_height(3000)
                .rotate_if_landscape(PdfPageRenderRotation::Degrees90, true);

            let image_buffer = page
                .render_with_config(&render_config)
                .map_err(|e| {
                    PdfError::OcrError(format!("Failed to render page {}: {}", page_index + 1, e))
                })?
                .as_image();

            // Convert to DynamicImage for OCR processing
            let dynamic_image = image::DynamicImage::ImageRgba8(image_buffer);

            // Extract text using OCR
            match ocr_engine.extract_text_from_pdf_page(dynamic_image) {
                Ok(page_text) => {
                    tracing::debug!(
                        "OCR extracted {} characters from page {}",
                        page_text.len(),
                        page_index + 1
                    );
                    all_ocr_text.push_str(&page_text);
                    all_ocr_text.push('\n'); // Add page separator
                }
                Err(e) => {
                    tracing::warn!("OCR failed on page {}: {}", page_index + 1, e);
                    // Continue with other pages
                }
            }
        }

        tracing::info!(
            "OCR extraction completed, total text: {} characters",
            all_ocr_text.len()
        );

        // Detect tables from OCR text
        if !all_ocr_text.trim().is_empty() {
            let tables = self.detector.detect_tables(&all_ocr_text);
            tracing::info!("Detected {} tables from OCR text", tables.len());
            Ok(tables)
        } else {
            tracing::warn!("No text extracted via OCR");
            Ok(Vec::new())
        }
    }
}
