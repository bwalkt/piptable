# piptable

## OCR Support for PDF Processing (Phase 1B)

PipTable now includes OCR (Optical Character Recognition) support for extracting tables from scanned or image-based PDFs. This feature requires additional system dependencies.

### System Dependencies

Before building or running PipTable with OCR support, install the required system libraries:

#### macOS (using Homebrew)
```bash
# OCR dependencies
brew install tesseract leptonica

# PDF rendering for OCR (required for scanned PDFs)
# PDFium will be dynamically loaded - ensure it's available via system package manager
# or the pdfium-render crate will attempt to download/bundle it automatically
```

#### Ubuntu/Debian
```bash
sudo apt-get update
sudo apt-get install tesseract-ocr tesseract-ocr-eng libleptonica-dev pkg-config

# Note: PDFium library is dynamically loaded by pdfium-render crate
# If OCR fails at runtime, you may need to install libpdfium or 
# allow the crate to download/bundle PDFium automatically
```

#### Fedora/CentOS/RHEL
```bash
sudo dnf install tesseract tesseract-devel leptonica-devel pkgconfig
```

#### Windows
1. Download and install Tesseract from: https://github.com/UB-Mannheim/tesseract/wiki
2. Install Microsoft Visual C++ Build Tools
3. Ensure Tesseract is in your PATH

### OCR Language Support

By default, PipTable uses English (`eng`) for OCR. To add support for additional languages:

#### macOS
```bash
# Install additional languages (example: French and Spanish)
brew install tesseract-lang
```

#### Ubuntu/Debian  
```bash
sudo apt-get install tesseract-ocr-fra tesseract-ocr-spa
```

### Runtime Dependencies

**PDFium for PDF Rendering:**
- The `pdfium-render` crate dynamically loads PDFium library for PDF-to-image conversion
- If OCR fails with errors about PDFium initialization, ensure PDFium is available:
  - The crate attempts to auto-download/bundle PDFium on first use
  - Alternative: Install system PDFium package if available
  - Error messages will indicate PDFium loading failures

**Troubleshooting OCR Issues:**
- "Failed to initialize PDFium" → PDFium library not found or loadable
- "Failed to initialize Tesseract" → Tesseract not installed or not in PATH
- "OCR extraction failed" → PDF rendering or OCR processing error

### HTML Import Support

PipTable can import HTML tables directly from HTML files. This feature automatically extracts table data from HTML documents and converts them to sheets for processing.

```bash
# Import HTML table data
import "report.html" into sales
import "catalog.html" into products with headers
```

The HTML import feature:
- Extracts all `<table>` elements from HTML files
- Automatically detects header rows (`<th>` elements)
- Preserves data types (integers, floats, booleans, strings)
- Supports multiple tables per HTML file (returns the first by default)

### Usage

OCR is automatically enabled when processing PDFs that contain minimal extractable text. You can also explicitly enable OCR:

```rust
use piptable_pdf::{PdfOptions, extract_tables_with_options};

let options = PdfOptions {
    ocr_enabled: true,
    ocr_language: "eng".to_string(),
    page_range: Some((1, 5)),
    min_table_rows: 2,
    min_table_cols: 2,
};

let tables = extract_tables_with_options("scanned_document.pdf", options)?;
```

For more details, see [docs/PDF_ROADMAP.md](docs/PDF_ROADMAP.md).
