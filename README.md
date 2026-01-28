# piptable

## OCR Support for PDF Processing (Phase 1B)

PipTable now includes OCR (Optical Character Recognition) support for extracting tables from scanned or image-based PDFs. This feature requires additional system dependencies.

### System Dependencies

Before building or running PipTable with OCR support, install the required system libraries:

#### macOS (using Homebrew)
```bash
brew install tesseract leptonica
```

#### Ubuntu/Debian
```bash
sudo apt-get update
sudo apt-get install tesseract-ocr tesseract-ocr-eng libleptonica-dev pkg-config
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
