use crate::detector::{TableDetector, TableRegion};
use crate::error::{PdfError, Result};
use crate::ocr::OcrEngine;
use lopdf::Document;
use pdf_extract::extract_text;
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
        
        if text.trim().is_empty() && self.ocr_engine.is_some() {
            // If no text found and OCR is enabled, try OCR
            tracing::info!("No text found in PDF, attempting OCR extraction");
            self.extract_tables_with_ocr(path)
        } else {
            // Detect tables from extracted text
            let tables = self.detector.detect_tables(&text);
            
            if tables.is_empty() {
                Err(PdfError::NoTablesFound)
            } else {
                Ok(tables)
            }
        }
    }
    
    fn extract_text_from_pdf(&self, path: &Path) -> Result<String> {
        // Use pdf-extract for text extraction
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
        let pages = doc.get_pages();
        
        let (start, end) = if let Some((s, e)) = self.options.page_range {
            (s.max(1), e.min(pages.len()))
        } else {
            (1, pages.len())
        };
        
        for page_num in start..=end {
            // Extract text from page using lopdf
            // lopdf expects page numbers directly
            if let Ok(content) = doc.extract_text(&[page_num as u32]) {
                all_text.push_str(&content);
                all_text.push('\n');
            }
        }
        
        Ok(all_text)
    }
    
    fn extract_tables_with_ocr(&self, _path: &Path) -> Result<Vec<TableRegion>> {
        // OCR implementation would go here
        // For Phase 1, we're focusing on text-based extraction
        Err(PdfError::OcrError("OCR extraction not fully implemented in Phase 1".to_string()))
    }
}