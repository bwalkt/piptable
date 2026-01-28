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
        // Extract text; use per-page extraction when OCR is enabled for mixed PDFs.
        let (text, content_needs_ocr) = if self.options.ocr_enabled {
            let page_texts = self.extract_text_per_page_with_lopdf(path)?;
            let mut combined = String::new();
            let mut pages_needing_ocr: Vec<u16> = Vec::new();

            for (page_num, page_text) in &page_texts {
                if OcrEngine::needs_ocr(page_text, None) {
                    match u16::try_from(page_num.saturating_sub(1)) {
                        Ok(index) => pages_needing_ocr.push(index),
                        Err(_) => {
                            tracing::warn!(
                                "Skipping OCR for page {}: page index exceeds PDFium limits",
                                page_num
                            );
                        }
                    }
                }
                combined.push_str(page_text);
                combined.push('\n');
            }

            let ocr_available = self.ocr_engine.is_some();
            let mut used_text = combined;

            if !pages_needing_ocr.is_empty() && ocr_available {
                tracing::info!(
                    "OCR candidate pages: {} (of {})",
                    pages_needing_ocr.len(),
                    page_texts.len()
                );

                match self.extract_text_with_ocr_for_pages(path, &pages_needing_ocr) {
                    Ok(ocr_pages) if !ocr_pages.is_empty() => {
                        let mut rebuilt = String::new();
                        for (page_num, page_text) in page_texts {
                            let page_index = u16::try_from(page_num.saturating_sub(1)).ok();
                            if let Some(page_index) = page_index {
                                if let Some(ocr_text) = ocr_pages.get(&page_index) {
                                    rebuilt.push_str(ocr_text);
                                } else {
                                    rebuilt.push_str(&page_text);
                                }
                            } else {
                                rebuilt.push_str(&page_text);
                            }
                            rebuilt.push('\n');
                        }
                        used_text = rebuilt;
                    }
                    Ok(_) => {
                        tracing::warn!("OCR completed but produced no text");
                    }
                    Err(e) => match e {
                        PdfError::OcrSetupError(_) => {
                            tracing::error!("OCR setup failed: {}", e);
                            return Err(e);
                        }
                        PdfError::OcrProcessingError(_) => {
                            tracing::warn!(
                                "OCR processing failed, falling back to text extraction: {}",
                                e
                            );
                        }
                        _ => {
                            tracing::warn!(
                                "OCR failed (unknown type), falling back to text extraction: {}",
                                e
                            );
                        }
                    },
                }
            }

            (used_text, !pages_needing_ocr.is_empty())
        } else {
            let text = self.extract_text_from_pdf(path)?;
            let content_needs_ocr = OcrEngine::needs_ocr(&text, None);
            (text, content_needs_ocr)
        };

        // Detect tables from extracted text (either original or after OCR fallback)
        let tables = self.detector.detect_tables(&text);

        if tables.is_empty() {
            if content_needs_ocr && !self.options.ocr_enabled {
                // Use OcrError (not OcrSetupError) since this is about user configuration,
                // not missing dependencies - OCR is simply disabled
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

    fn extract_text_per_page_with_lopdf(&self, path: &Path) -> Result<Vec<(usize, String)>> {
        let doc = Document::load(path)
            .map_err(|e| PdfError::ParseError(format!("Failed to load PDF: {}", e)))?;

        let pages = doc.get_pages();
        let (start, end) = if let Some((s, e)) = self.options.page_range {
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

        let mut results = Vec::new();
        let mut any_page_extracted = false;
        let mut last_error: Option<String> = None;

        for page_num in start..=end {
            match doc.extract_text(&[page_num as u32]) {
                Ok(content) => {
                    any_page_extracted = true;
                    results.push((page_num, content));
                }
                Err(e) => {
                    last_error = Some(e.to_string());
                    tracing::warn!("lopdf extract_text failed on page {}: {}", page_num, e);
                    results.push((page_num, String::new()));
                }
            }
        }

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

        Ok(results)
    }

    fn extract_text_with_ocr_for_pages(
        &self,
        path: &Path,
        page_indices: &[u16],
    ) -> Result<std::collections::HashMap<u16, String>> {
        let ocr_engine = self
            .ocr_engine
            .as_ref()
            .ok_or_else(|| PdfError::OcrSetupError("OCR engine not initialized".to_string()))?;

        tracing::info!("Attempting OCR extraction for {} pages", page_indices.len());

        let pdfium = Pdfium::new(
            Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
                .or_else(|_| Pdfium::bind_to_system_library())
                .map_err(|e| {
                    PdfError::OcrSetupError(format!("Failed to initialize PDFium: {}", e))
                })?,
        );

        let document = pdfium.load_pdf_from_file(path, None).map_err(|e| {
            PdfError::OcrProcessingError(format!("Failed to load PDF for OCR: {}", e))
        })?;

        let total_pages = document.pages().len();
        let mut results = std::collections::HashMap::new();

        for &page_index in page_indices {
            if page_index >= total_pages {
                continue;
            }

            tracing::debug!("Processing page {} for OCR", page_index + 1);
            let page = document.pages().get(page_index).map_err(|e| {
                PdfError::OcrProcessingError(format!(
                    "Failed to get page {}: {}",
                    page_index + 1,
                    e
                ))
            })?;

            let render_config = PdfRenderConfig::new()
                .set_target_width(2000)
                .set_maximum_height(3000)
                .rotate_if_landscape(PdfPageRenderRotation::Degrees90, true);

            let image_buffer = page
                .render_with_config(&render_config)
                .map_err(|e| {
                    PdfError::OcrProcessingError(format!(
                        "Failed to render page {}: {}",
                        page_index + 1,
                        e
                    ))
                })?
                .as_image();

            let dynamic_image: image::DynamicImage = image_buffer;

            match ocr_engine.extract_text_from_pdf_page(dynamic_image) {
                Ok(page_text) => {
                    results.insert(page_index, page_text);
                }
                Err(e) => {
                    if let PdfError::OcrSetupError(_) = e {
                        return Err(e);
                    }
                    tracing::warn!("OCR failed on page {}: {}", page_index + 1, e);
                }
            }
        }

        Ok(results)
    }
}
