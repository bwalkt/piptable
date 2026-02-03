//! PDF document structure extraction module.
//!
//! This module provides functionality to extract structured content from PDF documents,
//! including headings, paragraphs, and document hierarchy. It uses font analysis and
//! pattern matching to identify document elements.
//!
//! # Example
//!
//! ```rust,no_run
//! use piptable_pdf::structure::StructureDetector;
//! use pdfium_render::prelude::*;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let detector = StructureDetector::default();
//!     let pdfium = Pdfium::new(Pdfium::bind_to_system_library()?);
//!     let document = pdfium.load_pdf_from_file("paper.pdf", None)?;
//!     let structured_doc = detector.analyze_document(&document, None)?;
//!     let markdown = structured_doc.to_markdown();
//!     println!("{}", markdown);
//!     Ok(())
//! }
//! ```

use crate::error::{PdfError, Result};
use pdfium_render::prelude::*;
use serde_json::json;

/// Represents a bounding box for text elements in PDF coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingBox {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl BoundingBox {
    pub fn from_pdf_rect(rect: PdfRect) -> Self {
        Self {
            left: rect.left().value,
            right: rect.right().value,
            top: rect.top().value,
            bottom: rect.bottom().value,
        }
    }

    pub fn merge(&self, other: &BoundingBox) -> BoundingBox {
        BoundingBox {
            left: self.left.min(other.left),
            right: self.right.max(other.right),
            top: self.top.max(other.top),
            bottom: self.bottom.min(other.bottom),
        }
    }

    pub fn height(&self) -> f32 {
        (self.top - self.bottom).abs()
    }
}

/// Represents a text block extracted from a PDF with styling information.
#[derive(Debug, Clone)]
pub struct TextBlock {
    pub text: String,
    pub bbox: BoundingBox,
    pub page: usize,
    pub font_size: f32,
    pub font_name: String,
    pub is_bold: bool,
    pub is_italic: bool,
}

/// Represents a structured document element.
#[derive(Debug, Clone)]
pub enum DocumentElement {
    Heading {
        level: u8,
        text: String,
        page: usize,
        bbox: BoundingBox,
    },
    Paragraph {
        text: String,
        page: usize,
        bbox: BoundingBox,
    },
}

/// Represents a complete structured document extracted from a PDF.
#[derive(Debug, Clone)]
pub struct StructuredDocument {
    /// List of document elements in reading order
    pub elements: Vec<DocumentElement>,
    /// Total number of pages in the source PDF
    pub page_count: usize,
}

impl StructuredDocument {
    /// Converts the structured document to Markdown format.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use piptable_pdf::StructuredDocument;
    /// # fn demo(doc: &StructuredDocument) -> Result<(), Box<dyn std::error::Error>> {
    /// let markdown = doc.to_markdown();
    /// std::fs::write("output.md", markdown)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        for element in &self.elements {
            match element {
                DocumentElement::Heading { level, text, .. } => {
                    let hashes = "#".repeat(*level as usize);
                    out.push_str(&hashes);
                    out.push(' ');
                    out.push_str(text);
                    out.push_str("\n\n");
                }
                DocumentElement::Paragraph { text, .. } => {
                    out.push_str(text);
                    out.push_str("\n\n");
                }
            }
        }
        out
    }

    /// Converts the structured document to JSON format optimized for LLM processing.
    ///
    /// The JSON structure includes top-level `elements` and `metadata` keys.
    /// Each element contains type, content, page numbers, and bounding boxes.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use piptable_pdf::StructuredDocument;
    /// # fn demo(doc: &StructuredDocument) -> Result<(), Box<dyn std::error::Error>> {
    /// let json = doc.to_llm_json();
    /// let json_str = serde_json::to_string_pretty(&json)?;
    /// println!("{}", json_str);
    /// # Ok(())
    /// # }
    /// ```
    pub fn to_llm_json(&self) -> serde_json::Value {
        let elements = self
            .elements
            .iter()
            .map(|element| match element {
                DocumentElement::Heading {
                    level,
                    text,
                    page,
                    bbox,
                } => json!({
                    "type": "heading",
                    "level": level,
                    "content": text,
                    "page": page,
                    "bbox": {
                        "left": bbox.left,
                        "top": bbox.top,
                        "right": bbox.right,
                        "bottom": bbox.bottom,
                    }
                }),
                DocumentElement::Paragraph { text, page, bbox } => json!({
                    "type": "paragraph",
                    "content": text,
                    "page": page,
                    "bbox": {
                        "left": bbox.left,
                        "top": bbox.top,
                        "right": bbox.right,
                        "bottom": bbox.bottom,
                    }
                }),
            })
            .collect::<Vec<_>>();

        json!({
            "elements": elements,
            "metadata": {
                "page_count": self.page_count,
            }
        })
    }
}

/// PDF document structure detector with configurable parameters.
///
/// The detector analyzes PDF text blocks to identify document structure including
/// headings and paragraphs. It uses both font-based and pattern-based detection methods.
#[derive(Debug, Clone)]
pub struct StructureDetector {
    heading_ratio_h1: f32,
    heading_ratio_h2: f32,
    heading_ratio_h3: f32,
    heading_ratio_h4: f32,
    heading_short_line_limit: usize,
    line_merge_threshold: f32,
    paragraph_gap_multiplier: f32,
}

impl Default for StructureDetector {
    fn default() -> Self {
        Self {
            heading_ratio_h1: 1.5,
            heading_ratio_h2: 1.3,
            heading_ratio_h3: 1.2,
            heading_ratio_h4: 1.1,
            heading_short_line_limit: 100,
            line_merge_threshold: 2.0,
            paragraph_gap_multiplier: 1.5,
        }
    }
}

impl StructureDetector {
    /// Analyzes a PDF document and extracts its structure.
    ///
    /// # Arguments
    /// * `document` - The PDF document to analyze
    /// * `page_range` - Optional page range to process (1-indexed, inclusive)
    ///
    /// # Returns
    /// A `StructuredDocument` containing extracted headings and paragraphs in reading order.
    pub fn analyze_document(
        &self,
        document: &PdfDocument,
        page_range: Option<(usize, usize)>,
    ) -> Result<StructuredDocument> {
        let total_pages = usize::from(document.pages().len());
        let (start, end) = resolve_page_range(page_range, total_pages)?;

        let mut blocks = Vec::new();
        for page_index in start..=end {
            let page_index_u16 = u16::try_from(page_index).map_err(|_| {
                PdfError::ExtractionError(format!("Page index {} out of range", page_index))
            })?;
            let page = document.pages().get(page_index_u16).map_err(|e| {
                PdfError::ExtractionError(format!(
                    "Failed to load page {}: {}",
                    page_index_u16 + 1,
                    e
                ))
            })?;
            blocks.extend(self.extract_text_blocks(&page, page_index)?);
        }

        Ok(self.analyze_blocks(blocks, total_pages))
    }

    fn extract_text_blocks(&self, page: &PdfPage, page_index: usize) -> Result<Vec<TextBlock>> {
        let text = page
            .text()
            .map_err(|e| PdfError::ExtractionError(format!("Failed to read page text: {}", e)))?;
        let mut blocks = Vec::new();

        for segment in text.segments().iter() {
            let raw_text = segment.text();
            let trimmed = raw_text.trim();
            if trimmed.is_empty() {
                continue;
            }

            let bbox = BoundingBox::from_pdf_rect(segment.bounds());
            let (font_size, font_name, is_bold, is_italic) = segment_style(&segment);

            blocks.push(TextBlock {
                text: trimmed.to_string(),
                bbox,
                page: page_index + 1,
                font_size,
                font_name,
                is_bold,
                is_italic,
            });
        }

        Ok(blocks)
    }

    fn analyze_blocks(&self, blocks: Vec<TextBlock>, page_count: usize) -> StructuredDocument {
        if blocks.is_empty() {
            return StructuredDocument {
                elements: Vec::new(),
                page_count,
            };
        }

        let avg_font_size = average_font_size(&blocks).max(1.0);
        let mut elements = Vec::new();

        let mut blocks_by_page: std::collections::HashMap<usize, Vec<TextBlock>> =
            std::collections::HashMap::new();
        for block in blocks {
            blocks_by_page.entry(block.page).or_default().push(block);
        }

        let mut pages: Vec<usize> = blocks_by_page.keys().copied().collect();
        pages.sort_unstable();

        for page_index in pages {
            if let Some(page_blocks) = blocks_by_page.get(&page_index) {
                let lines = self.build_lines(page_blocks, avg_font_size);
                elements.extend(self.build_paragraphs(&lines, avg_font_size));
            }
        }

        StructuredDocument {
            elements,
            page_count,
        }
    }

    fn build_lines(&self, blocks: &[TextBlock], avg_font_size: f32) -> Vec<Line> {
        let mut sorted = blocks.to_vec();
        sorted.sort_by(|a, b| {
            b.bbox
                .top
                .partial_cmp(&a.bbox.top)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    a.bbox
                        .left
                        .partial_cmp(&b.bbox.left)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });

        let mut lines: Vec<Line> = Vec::new();
        let merge_threshold = self.line_merge_threshold.max(avg_font_size * 0.5);

        for block in sorted {
            let mid_y = (block.bbox.top + block.bbox.bottom) / 2.0;
            if let Some(last) = lines.last_mut() {
                if (last.mid_y - mid_y).abs() <= merge_threshold && last.page == block.page {
                    last.push(block);
                    continue;
                }
            }
            lines.push(Line::from_block(block));
        }

        lines
    }

    fn build_paragraphs(&self, lines: &[Line], avg_font_size: f32) -> Vec<DocumentElement> {
        if lines.is_empty() {
            return Vec::new();
        }

        let line_spacing = median_line_gap(lines).unwrap_or(avg_font_size * 1.2);
        let paragraph_gap = line_spacing * self.paragraph_gap_multiplier;

        let mut elements = Vec::new();
        let mut current: Vec<Line> = Vec::new();

        for (idx, line) in lines.iter().enumerate() {
            if current.is_empty() {
                current.push(line.clone());
                continue;
            }

            let prev = &lines[idx - 1];
            let gap = (prev.bbox.bottom - line.bbox.top).max(0.0);
            if gap > paragraph_gap || prev.page != line.page {
                elements.push(self.classify_paragraph(&current, avg_font_size));
                current.clear();
            }

            current.push(line.clone());
        }

        if !current.is_empty() {
            elements.push(self.classify_paragraph(&current, avg_font_size));
        }

        elements
    }

    fn classify_paragraph(&self, lines: &[Line], avg_font_size: f32) -> DocumentElement {
        let text = join_lines(lines);
        let bbox = lines
            .iter()
            .fold(lines[0].bbox, |acc, line| acc.merge(&line.bbox));
        let page = lines[0].page;

        if lines.len() == 1 {
            let line = &lines[0];
            if let Some(level) = self.heading_level(line, avg_font_size) {
                return DocumentElement::Heading {
                    level,
                    text,
                    page,
                    bbox,
                };
            }
        }

        DocumentElement::Paragraph { text, page, bbox }
    }

    fn heading_level(&self, line: &Line, avg_font_size: f32) -> Option<u8> {
        if let Some(level) = heading_level_by_pattern(&line.text) {
            return Some(level);
        }
        if avg_font_size <= 0.0 {
            return None;
        }

        let ratio = line.font_size / avg_font_size;
        if ratio >= self.heading_ratio_h1 && line.is_bold {
            return Some(1);
        }
        if ratio >= self.heading_ratio_h2 && line.is_bold {
            return Some(2);
        }
        if ratio >= self.heading_ratio_h3 {
            return Some(3);
        }
        if ratio >= self.heading_ratio_h4 && line.text.len() <= self.heading_short_line_limit {
            return Some(4);
        }
        None
    }
}

#[derive(Debug, Clone)]
struct Line {
    text: String,
    bbox: BoundingBox,
    page: usize,
    font_size: f32,
    is_bold: bool,
    is_italic: bool,
    mid_y: f32,
}

impl Line {
    fn from_block(block: TextBlock) -> Self {
        let mid_y = (block.bbox.top + block.bbox.bottom) / 2.0;
        Self {
            text: block.text,
            bbox: block.bbox,
            page: block.page,
            font_size: block.font_size,
            is_bold: block.is_bold,
            is_italic: block.is_italic,
            mid_y,
        }
    }

    fn push(&mut self, block: TextBlock) {
        if !self.text.is_empty() {
            self.text.push(' ');
        }
        self.text.push_str(block.text.trim());
        self.bbox = self.bbox.merge(&block.bbox);
        self.font_size = self.font_size.max(block.font_size);
        self.is_bold = self.is_bold || block.is_bold;
        self.is_italic = self.is_italic || block.is_italic;
        self.mid_y = (self.bbox.top + self.bbox.bottom) / 2.0;
    }
}

fn segment_style(segment: &PdfPageTextSegment<'_>) -> (f32, String, bool, bool) {
    let Ok(chars) = segment.chars() else {
        return (0.0, String::new(), false, false);
    };

    let Some(first) = chars.iter().next() else {
        return (0.0, String::new(), false, false);
    };

    let font_size = first.scaled_font_size().value;
    let font_name = first.font_name();
    let weight_is_bold = match first.font_weight() {
        Some(PdfFontWeight::Weight600)
        | Some(PdfFontWeight::Weight700Bold)
        | Some(PdfFontWeight::Weight800)
        | Some(PdfFontWeight::Weight900) => true,
        Some(PdfFontWeight::Custom(weight)) => weight >= 600,
        _ => false,
    };
    let is_bold = weight_is_bold || first.font_is_bold_reenforced();
    let is_italic = first.font_is_italic();

    (font_size, font_name, is_bold, is_italic)
}

fn average_font_size(blocks: &[TextBlock]) -> f32 {
    let mut sum = 0.0;
    let mut count = 0.0;
    for block in blocks {
        if block.font_size > 0.0 {
            sum += block.font_size;
            count += 1.0;
        }
    }
    if count == 0.0 {
        0.0
    } else {
        sum / count
    }
}

fn median_line_gap(lines: &[Line]) -> Option<f32> {
    if lines.len() < 2 {
        return None;
    }

    let mut gaps: Vec<f32> = Vec::new();
    for pair in lines.windows(2) {
        let prev = &pair[0];
        let current = &pair[1];
        if prev.page != current.page {
            continue;
        }
        let gap = (prev.bbox.bottom - current.bbox.top).max(0.0);
        if gap > 0.0 {
            gaps.push(gap);
        }
    }

    if gaps.is_empty() {
        return None;
    }

    gaps.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    Some(gaps[gaps.len() / 2])
}

fn join_lines(lines: &[Line]) -> String {
    let mut out = String::new();
    for (idx, line) in lines.iter().enumerate() {
        if idx > 0 {
            out.push(' ');
        }
        out.push_str(line.text.trim());
    }
    out
}

fn resolve_page_range(
    page_range: Option<(usize, usize)>,
    total_pages: usize,
) -> Result<(usize, usize)> {
    let (start, end) = if let Some((s, e)) = page_range {
        if s == 0 || e == 0 {
            return Err(PdfError::InvalidPageRange(
                "Page numbers must be >= 1".to_string(),
            ));
        }
        if s > e {
            return Err(PdfError::InvalidPageRange(format!(
                "Start page {} is greater than end page {}",
                s, e
            )));
        }
        let clamped_end = e.min(total_pages);
        if s > clamped_end {
            return Err(PdfError::InvalidPageRange(format!(
                "Start page {} exceeds document length of {} pages",
                s, total_pages
            )));
        }
        (s.saturating_sub(1), clamped_end.saturating_sub(1))
    } else {
        (0, total_pages.saturating_sub(1))
    };

    Ok((start, end))
}

fn heading_level_by_pattern(text: &str) -> Option<u8> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    let chapter_re = heading_chapter_regex();
    if chapter_re.is_match(trimmed) {
        return Some(1);
    }

    let numbered_re = heading_numbered_regex();
    if let Some(caps) = numbered_re.captures(trimmed) {
        let marker = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let dot_count = marker.matches('.').count();
        return Some(if dot_count >= 2 { 3 } else { 2 });
    }

    let roman_re = heading_roman_regex();
    if roman_re.is_match(trimmed) {
        return Some(2);
    }

    None
}

fn heading_chapter_regex() -> &'static regex::Regex {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"(?i)^chapter\s+\d+\b").expect("valid chapter regex"))
}

fn heading_numbered_regex() -> &'static regex::Regex {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(r"^(\d+(?:\.\d+)*\.)\s+\S").expect("valid numbered heading regex")
    })
}

fn heading_roman_regex() -> &'static regex::Regex {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(r"(?i)^([IVXLCDM]+)\.\s+\S").expect("valid roman heading regex")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heading_detection() {
        let detector = StructureDetector::default();
        let blocks = vec![
            TextBlock {
                text: "Report Title".to_string(),
                bbox: BoundingBox {
                    left: 0.0,
                    top: 700.0,
                    right: 200.0,
                    bottom: 680.0,
                },
                page: 1,
                font_size: 24.0,
                font_name: "TestBold".to_string(),
                is_bold: true,
                is_italic: false,
            },
            TextBlock {
                text: "Body line".to_string(),
                bbox: BoundingBox {
                    left: 0.0,
                    top: 600.0,
                    right: 200.0,
                    bottom: 590.0,
                },
                page: 2,
                font_size: 6.0,
                font_name: "Test".to_string(),
                is_bold: false,
                is_italic: false,
            },
        ];

        let doc = detector.analyze_blocks(blocks, 2);
        let heading = doc.elements.iter().find_map(|element| {
            if let DocumentElement::Heading { level, text, .. } = element {
                Some((level, text))
            } else {
                None
            }
        });
        let (level, text) = heading.expect("expected heading");
        assert!((1..=4).contains(level));
        assert_eq!(text, "Report Title");
    }

    #[test]
    fn test_paragraph_grouping() {
        let detector = StructureDetector::default();
        let blocks = vec![
            TextBlock {
                text: "Line one".to_string(),
                bbox: BoundingBox {
                    left: 0.0,
                    top: 700.0,
                    right: 200.0,
                    bottom: 690.0,
                },
                page: 1,
                font_size: 12.0,
                font_name: "Test".to_string(),
                is_bold: false,
                is_italic: false,
            },
            TextBlock {
                text: "Line two".to_string(),
                bbox: BoundingBox {
                    left: 0.0,
                    top: 685.0,
                    right: 200.0,
                    bottom: 675.0,
                },
                page: 1,
                font_size: 12.0,
                font_name: "Test".to_string(),
                is_bold: false,
                is_italic: false,
            },
        ];

        let doc = detector.analyze_blocks(blocks, 1);
        assert_eq!(doc.elements.len(), 1);
        match &doc.elements[0] {
            DocumentElement::Paragraph { text, .. } => {
                assert_eq!(text, "Line one Line two");
            }
            _ => panic!("expected paragraph"),
        }
    }

    #[test]
    fn test_heading_pattern_detection() {
        let detector = StructureDetector::default();
        let blocks = vec![
            TextBlock {
                text: "1. Introduction".to_string(),
                bbox: BoundingBox {
                    left: 0.0,
                    top: 700.0,
                    right: 200.0,
                    bottom: 690.0,
                },
                page: 1,
                font_size: 12.0,
                font_name: "Test".to_string(),
                is_bold: false,
                is_italic: false,
            },
            TextBlock {
                text: "Body line".to_string(),
                bbox: BoundingBox {
                    left: 0.0,
                    top: 600.0,
                    right: 200.0,
                    bottom: 590.0,
                },
                page: 2,
                font_size: 12.0,
                font_name: "Test".to_string(),
                is_bold: false,
                is_italic: false,
            },
        ];

        let doc = detector.analyze_blocks(blocks, 2);
        match &doc.elements[0] {
            DocumentElement::Heading { level, text, .. } => {
                assert_eq!(*level, 2);
                assert_eq!(text, "1. Introduction");
            }
            _ => panic!("expected heading"),
        }
    }
}
