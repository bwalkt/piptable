# PipTable PDF Module

The PDF module provides two main capabilities:

- Table extraction into `Sheet` values
- Document structure extraction (headings + paragraphs) for LLM-friendly output

## Features

- **Table Detection**: Extract tables from PDFs with configurable thresholds
- **Structure Extraction**: Headings and paragraphs with bounding boxes
- **OCR Support**: Optional OCR fallback for scanned PDFs (Rust API only)
- **Markdown/JSON Export**: Helpers for structured documents
- **Page Ranges**: Limit work to specific pages

## Usage

### Table Extraction

```rust
use piptable_pdf::{extract_tables_from_pdf, extract_tables_with_options, PdfOptions};

// Simple table extraction
let tables = extract_tables_from_pdf("document.pdf")?;

// With custom options
let options = PdfOptions {
    page_range: Some((1, 10)),
    min_table_rows: 3,
    min_table_cols: 2,
    ocr_enabled: true,
    ..Default::default()
};
let tables = extract_tables_with_options("document.pdf", options)?;
```

### Structure Extraction

```rust
use piptable_pdf::{extract_structure_from_pdf, extract_structure_with_options, PdfOptions};

let doc = extract_structure_from_pdf("document.pdf")?;

let markdown = doc.to_markdown();
let json = doc.to_llm_json();

let options = PdfOptions {
    page_range: Some((1, 5)),
    ..Default::default()
};
let doc = extract_structure_with_options("document.pdf", options)?;
```

## API Highlights

### PdfOptions

```rust
pub struct PdfOptions {
    pub page_range: Option<(usize, usize)>,
    pub ocr_enabled: bool,
    pub ocr_language: String,
    pub min_table_rows: usize,
    pub min_table_cols: usize,
    pub extract_structure: bool,
}
```

Notes:
- `page_range` is 1-indexed and inclusive.
- `extract_structure` is used by the DSL import path; Rust APIs call
  structure extraction directly and do not rely on this flag.

### StructuredDocument

```rust
pub struct StructuredDocument {
    pub elements: Vec<DocumentElement>,
    pub page_count: usize,
}

pub enum DocumentElement {
    Heading { level: u8, text: String, page: usize, bbox: BoundingBox },
    Paragraph { text: String, page: usize, bbox: BoundingBox },
}
```

`page` is 0-indexed.

## Heading Detection

Two mechanisms are used:

- **Pattern-based**
  - `Chapter 1` → H1
  - `1. Title`, `1.2. Subtitle` → H2/H3
  - `IV. Title` → H2
- **Font-based**
  - Larger font ratio + bold → H1/H2
  - Larger font ratio → H3
  - Slightly larger + short line → H4

## Limitations (Phase 1)

- Single-column layout assumption
- Lists are not detected as separate elements
- Tables are extracted separately from structure
- Images are not extracted

## System Requirements

- PDFium is required for structure extraction
- OCR requires Tesseract + dependencies (if enabled)
