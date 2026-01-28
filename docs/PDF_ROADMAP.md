# PDF Support Roadmap

**Tracking Issue**: [#92 - feat(pdf): add basic PDF table extraction support](https://github.com/bwalkt/piptable/issues/92)

## Overview
This document outlines the phased approach for implementing comprehensive PDF support in PipTable, enabling users to extract and manipulate tabular data from PDF documents.

## Phase 1: Foundation - Basic Text-Based Table Extraction 
**Status**: Partially Complete (Basic implementation merged, OCR pending)

### Phase 1A: Basic Text Extraction ✅ COMPLETED
**Merged to main (PR #166)**
- ✅ Basic PDF text extraction using pdf-extract and lopdf libraries
- ✅ Simple table detection using regex patterns
- ✅ Conversion of detected tables to Sheet format
- ✅ Support for page range selection
- ✅ Integration with io::import_sheet() function
- ✅ Error handling and validation
- ✅ Basic test coverage

### Phase 1B: Add OCR Support ✅ IMPLEMENTED
**Completed**:
- ✅ Added tesseract-rs and pdfium-render for OCR capabilities alongside existing text extraction
- ✅ Implemented automatic detection of scanned vs text PDFs
- ✅ Added OCR fallback when text extraction yields no results
- ✅ Kept existing pdf-extract/lopdf as primary method, added OCR as supplementary
- ✅ Full PDF page rendering to images for OCR processing
- ✅ Image preprocessing for better OCR accuracy

**System Dependencies Required**:
- Tesseract OCR library and language data
- Leptonica image processing library
- On macOS: `brew install tesseract leptonica`
- On Ubuntu/Debian: `apt-get install tesseract-ocr tesseract-ocr-eng libleptonica-dev`
- On other systems: Install Tesseract and Leptonica according to platform documentation

### Technical Implementation Plan:
- **Primary extraction**: Keep pdf-extract and lopdf (already working well)
- **OCR support**: ADD tesseract-rs for scanned/image-based PDFs
- **Hybrid approach**: Try text extraction first, fall back to OCR if needed
- **Table detection**: Keep regex patterns, enhance with OCR output processing

### Current Limitations:
- No DSL integration (not available via IMPORT command)
- Simple table detection (may miss complex layouts)
- No support for merged cells or nested tables
- Limited to regex-based pattern matching

## Phase 2: Enhanced Detection & DSL Integration 🚧 PLANNED

### Goals:
- **DSL Integration**: Enable `IMPORT "file.pdf" INTO sheet` syntax
- **Improved Detection**:
  - Machine learning-based table detection
  - Support for bordered tables
  - Handle merged cells and spanning columns
  - Multi-column layouts
  - Nested table structures

### Technical Approach:
- Integrate with interpreter's IMPORT command
- Add PDF-specific import options:
  ```piptable
  IMPORT "report.pdf" WITH {
    "pages": [1, 3, 5],
    "ocr": false,
    "merge_tables": true
  } INTO sheet
  ```
- Implement heuristics for:
  - Header row detection
  - Column type inference
  - Table boundary detection

### Dependencies:
- May require additional ML libraries for table detection
- Consider using existing solutions like Camelot or Tabula algorithms

## Phase 3: Advanced Features & Optimization 🚀 FUTURE

### Goals:
- **Form Data Extraction**: Extract data from PDF forms
- **Streaming Processing**: Handle very large PDFs efficiently
- **Batch Processing**: Process multiple PDFs in parallel
- **Template Matching**: Define templates for consistent PDF layouts
- **Export to PDF**: Generate PDFs with tables from Sheets

### Advanced Features:
1. **Template System**:
   ```piptable
   ' Define a template for invoice PDFs
   DIM template = PDFTemplate::new()
     .header_pattern("Invoice #\\d+")
     .table_region(x: 50, y: 200, width: 500, height: 400)
     .columns(["Item", "Quantity", "Price", "Total"])
   
   DIM data = IMPORT "invoice.pdf" WITH template INTO sheet
   ```

2. **Streaming API**:
   ```piptable
   ' Process large PDF without loading entire file
   DIM stream = PDFStream::open("large_report.pdf")
   FOR EACH page IN stream
     DIM tables = page.extract_tables()
     ' Process tables incrementally
   NEXT
   ```

3. **Batch Processing**:
   ```piptable
   ' Process multiple PDFs in parallel
   DIM results = IMPORT "invoices/*.pdf" WITH {
     "parallel": true,
     "max_workers": 4
   } INTO book
   ```

## Implementation Timeline

### Immediate (Next PR):
- [ ] DSL integration for basic PDF import
- [ ] Documentation for PDF support
- [ ] More comprehensive test suite

### Short-term (1-2 months):
- [ ] Improved table detection algorithms
- [ ] Support for bordered tables
- [ ] Header detection improvements

### Medium-term (3-6 months):
- [ ] Basic OCR support
- [ ] Form field extraction
- [ ] Performance optimizations

### Long-term (6+ months):
- [ ] Advanced ML-based detection
- [ ] Template system
- [ ] PDF generation from Sheets

## Testing Strategy

### Test Coverage Needed:
1. **Unit Tests**: Each component (extractor, detector, converter)
2. **Integration Tests**: End-to-end PDF import
3. **Performance Tests**: Large PDF handling
4. **Regression Tests**: Various PDF formats and generators

### Test PDFs Required:
- Simple text tables
- Bordered tables
- Multi-page documents
- Scanned documents
- Forms with fields
- Mixed content (text + tables + images)
- Different PDF versions (1.4, 1.5, 1.7, 2.0)

## Performance Considerations

### Optimization Opportunities:
- Lazy loading of PDF pages
- Parallel processing of multi-page documents
- Caching of extracted text
- Incremental table detection

### Benchmarks to Track:
- Pages per second processing rate
- Memory usage for large PDFs
- Accuracy of table detection
- OCR accuracy rates

## API Design

### Current API (Phase 1):
```rust
// Low-level API
let tables = extract_tables_from_pdf("file.pdf")?;
let sheets: Vec<Sheet> = tables.into_iter().map(convert_to_sheet).collect();

// With options
let options = PdfOptions {
    page_range: Some((1, 10)),
    min_table_rows: 2,
    min_table_cols: 2,
    ocr_enabled: false,
    ocr_language: "eng".to_string(),
};
let tables = extract_tables_with_options("file.pdf", options)?;
```

### Future DSL API:
```piptable
' Simple import
DIM data = IMPORT "report.pdf" INTO sheet

' With options
DIM data = IMPORT "report.pdf" WITH {
  "pages": [1, 2, 3],
  "ocr": true,
  "merge_tables": false
} INTO sheet

' Multiple PDFs into book
DIM all_reports = IMPORT "reports/*.pdf" INTO book

' With SQL
DIM results = QUERY(
  SELECT * FROM "report.pdf"
  WHERE amount > 1000
)
```

## Dependencies & Licensing

### Current Dependencies (Phase 1A):
- `pdf-extract` (MIT) - Text extraction
- `lopdf` (MIT) - PDF parsing
- `regex` (MIT/Apache) - Pattern matching
- `lazy_static` (MIT/Apache) - Regex compilation

### Phase 1B Dependencies (To Add):
- `tesseract-rs` - Rust bindings for Tesseract OCR (ADD to existing)
- `image` (MIT) - Image preprocessing for OCR
- Keep existing `pdf-extract` and `lopdf` - they work well for text PDFs

### Future Dependencies:
- ML libraries (TBD) - Advanced table detection
- `cairo-rs` - PDF generation capabilities

## Known Issues & Limitations

### Current Limitations:
1. Cannot handle encrypted/password-protected PDFs
2. No support for right-to-left languages
3. Complex layouts may not be detected correctly
4. No support for extracting images or charts
5. Cannot preserve formatting or styling

### Future Considerations:
- Security: Sanitize PDFs before processing
- Privacy: Handle sensitive data appropriately
- Accessibility: Ensure extracted data maintains accessibility info

## Success Metrics

### Phase 1 Metrics:
- ✅ Successfully extract simple tables
- ✅ Handle multi-page documents
- ✅ Proper error handling
- ✅ Basic test coverage

### Future Metrics:
- [ ] 90%+ accuracy on standard table layouts
- [ ] OCR accuracy > 95% on clean scans
- [ ] Process 100+ pages per second
- [ ] Support 95% of common PDF generators

## References & Resources

### Documentation:
- [PDF 1.7 Specification](https://www.adobe.com/devnet/pdf/pdf_reference.html)
- [Table Extraction Algorithms](https://github.com/camelot-dev/camelot)
- [Tesseract OCR Documentation](https://github.com/tesseract-ocr/tesseract)

### Similar Projects:
- **Camelot**: Python library for PDF table extraction
- **Tabula**: Java library for PDF tables
- **pdfplumber**: Python PDF processing library
- **PyPDF2**: Python PDF toolkit

## Contributing

To contribute to PDF support development:

1. Check this roadmap for planned features
2. Open an issue to discuss your proposed contribution
3. Follow the phased approach - don't skip ahead
4. Ensure comprehensive tests for new features
5. Update this roadmap document with progress

## Questions & Discussion

For questions or suggestions about PDF support:
- Open an issue with the `pdf` label
- Reference this roadmap in discussions
- Provide sample PDFs that demonstrate issues