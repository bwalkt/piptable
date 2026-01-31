

  Your PDF module currently focuses on table extraction:
  - Uses lopdf and pdfium for PDF parsing
  - OCR support via Tesseract (109 languages)
  - Regex-based table detection
  - Converts tables to Sheet structures
  - JSON export already available for sheets

  MinerU's Approach (What We Could Add)

  MinerU provides comprehensive document understanding:
  1. Full document structure: Headers, paragraphs, lists
  2. Rich element extraction: Images, formulas (LaTeX), footnotes
  3. Semantic preservation: Reading order, document hierarchy
  4. Multiple outputs: Markdown + JSON with metadata

  Extension Possibilities

  1. Enhanced Document Parser (New Module)

  // crates/pdf/src/document.rs
  pub struct DocumentExtractor {
      extract_images: bool,
      extract_formulas: bool,
      preserve_layout: bool,
  }

  pub struct DocumentElement {
      pub element_type: ElementType,
      pub content: String,
      pub metadata: ElementMetadata,
  }

  pub enum ElementType {
      Heading(u8), // H1-H6
      Paragraph,
      Table(TableRegion),
      Image { alt: String, path: String },
      Formula { latex: String },
      List { ordered: bool },
  }

  2. Markdown/JSON Exporters

  // crates/pdf/src/export.rs
  impl Document {
      pub fn to_markdown(&self) -> String {
          // Convert document elements to markdown
          // Tables → markdown tables
          // Images → ![alt](path)
          // Formulas → $$latex$$
      }

      pub fn to_llm_json(&self) -> Value {
          // Structured JSON with metadata
          // Preserves reading order
          // Includes element types and positions
      }
  }

  3. Integration Points

  - Extend PdfExtractor to detect more element types
  - Add layout analysis using pdfium's text extraction
  - Enhance OCR to handle images and formulas
  - Create a Document struct that preserves structure

  4. Practical Implementation Path

  1. Phase 1: Add paragraph/heading detection alongside tables
  2. Phase 2: Export to markdown format
  3. Phase 3: Image extraction with OCR descriptions
  4. Phase 4: Formula detection and LaTeX conversion

  Example Usage

  // Future API
  let doc = PdfDocument::extract("paper.pdf", PdfOptions {
      extract_tables: true,
      extract_images: true,
      extract_formulas: true,
      ocr_enabled: true,
  })?;

  // LLM-ready markdown
  let markdown = doc.to_markdown();

  // Structured JSON for processing
  let json = doc.to_llm_json();

  Benefits for PipTable

  - Import full documents, not just tables
  - Better context for data analysis
  - Support research papers and reports
  - Enable RAG (Retrieval Augmented Generation) workflows

  The foundation is already there with your OCR and PDF parsing. The main additions would be:
  1. Document structure detection
  2. Markdown generation
  3. Richer metadata preservation

  This would make PipTable capable of ingesting complex documents and transforming them into formats that LLMs
  can effectively process, similar to MinerU but integrated with your existing data pipeline infrastructure.

> so what would
    1. Document structure detection entail

⏺ I'll break down what document structure detection would entail for PDF processing:

⏺ Document Structure Detection - Detailed Breakdown

  1. Text Extraction with Positioning

  Currently you extract raw text, but structure detection needs positional data:

  // Instead of just text, extract with coordinates
  struct TextBlock {
      text: String,
      bbox: BoundingBox,  // (x, y, width, height)
      page: usize,
      font_size: f32,
      font_name: String,
      is_bold: bool,
      is_italic: bool,
  }

  Using pdfium, you can get this:
  // pdfium provides character-level positioning
  let page = doc.pages().get(0)?;
  let text_page = page.text()?;

  for segment in text_page.segments() {
      let bounds = segment.bounds();
      let text = segment.text();
      let style = segment.style_info(); // font, size, weight
  }

  2. Layout Analysis (Clustering)

  Group text blocks into logical units based on:

  fn cluster_text_blocks(blocks: Vec<TextBlock>) -> Vec<DocumentRegion> {
      // Group by proximity
      let columns = detect_columns(&blocks);  // Multi-column layouts
      let paragraphs = group_by_proximity(&blocks, threshold: 10.0);

      // Vertical spacing analysis
      let line_spacing = calculate_median_line_spacing(&blocks);
      let paragraph_gaps = find_gaps_over(line_spacing * 1.5);

      DocumentRegion {
          blocks: grouped_blocks,
          region_type: RegionType::Paragraph,
      }
  }

⏺ 3. Element Classification Rules

  Heading Detection

  fn classify_heading(block: &TextBlock, doc_stats: &DocStats) -> Option<HeadingLevel> {
      // Font size relative to document average
      let size_ratio = block.font_size / doc_stats.avg_font_size;

      // Heuristics:
      if size_ratio > 1.5 && block.is_bold { return Some(H1); }
      if size_ratio > 1.3 && block.is_bold { return Some(H2); }
      if size_ratio > 1.2 { return Some(H3); }

      // Pattern matching
      if HEADING_PATTERN.is_match(&block.text) {  // "1. Introduction", "Chapter 2"
          return Some(classify_by_pattern(&block.text));
      }

      // Short lines with larger font
      if block.text.len() < 100 && size_ratio > 1.1 {
          return Some(H4);
      }
      None
  }

  List Detection

  fn detect_lists(blocks: &[TextBlock]) -> Vec<ListRegion> {
      let bullet_patterns = [
          r"^[•·▪▫◦‣⁃]",     // Bullet symbols
          r"^\d+\.",          // 1. 2. 3.
          r"^[a-z]\)",        // a) b) c)
          r"^[IVX]+\.",       // Roman numerals
      ];

      // Find sequences with consistent indentation
      let mut lists = Vec::new();
      for window in blocks.windows(3) {
          if same_indent(&window) && starts_with_bullet(&window[0]) {
              lists.push(ListRegion { items: window.to_vec() });
          }
      }
  }

⏺ 4. Reading Order Determination

  PDFs store text in rendering order, not reading order. You need to sort:

  fn determine_reading_order(regions: Vec<DocumentRegion>) -> Vec<DocumentRegion> {
      // For multi-column layouts
      let columns = detect_column_boundaries(&regions);

      // Sort algorithm:
      regions.sort_by(|a, b| {
          // 1. Page number first
          if a.page != b.page { return a.page.cmp(&b.page); }

          // 2. Column order (left to right)
          let a_col = get_column_index(&a.bbox, &columns);
          let b_col = get_column_index(&b.bbox, &columns);
          if a_col != b_col { return a_col.cmp(&b_col); }

          // 3. Top to bottom within column
          a.bbox.top.cmp(&b.bbox.top)
      });

      regions
  }

  5. Special Element Detection

  Footnotes/Headers/Footers

  fn detect_page_elements(page_blocks: &[TextBlock]) -> PageElements {
      // Headers: repeated text at top across pages
      let top_10_percent = filter_by_position(page_blocks, 0.0..0.1);

      // Footers: page numbers, repeated text at bottom
      let bottom_10_percent = filter_by_position(page_blocks, 0.9..1.0);

      // Footnotes: small font at bottom, often with superscript references
      let footnotes = bottom_10_percent.iter()
          .filter(|b| b.font_size < avg_font_size * 0.8)
          .filter(|b| FOOTNOTE_PATTERN.is_match(&b.text))
  }

  Image/Figure Regions

  fn detect_figures(page: &PdfPage) -> Vec<FigureRegion> {
      let images = page.objects()
          .filter(|obj| obj.is_image())
          .map(|img| {
              FigureRegion {
                  bbox: img.bounds(),
                  caption: find_nearby_caption(&img, page_blocks),
                  image_data: img.data(),
              }
          })
  }

⏺ 6. Practical Implementation Approach

  Given your existing codebase, here's how to add structure detection:

  // crates/pdf/src/structure.rs

  use pdfium_render::prelude::*;

  pub struct StructureDetector {
      min_heading_size_ratio: f32,
      column_gap_threshold: f32,
      paragraph_gap_multiplier: f32,
  }

  impl StructureDetector {
      pub fn analyze_document(&self, pdfium: &Pdfium, doc: &PdfDocument)
          -> Result<StructuredDocument> {

          let mut elements = Vec::new();

          for (page_num, page) in doc.pages().iter().enumerate() {
              // 1. Extract all text with positioning
              let text_blocks = self.extract_text_blocks(&page)?;

              // 2. Detect layout (columns, regions)
              let layout = self.analyze_layout(&text_blocks)?;

              // 3. Classify elements
              for region in layout.regions {
                  let element = self.classify_region(&region, &layout.stats)?;
                  elements.push(element);
              }

              // 4. Extract non-text elements
              elements.extend(self.extract_images(&page)?);
              elements.extend(self.extract_tables(&page)?); // Your existing code
          }

          // 5. Sort by reading order
          elements.sort_by(|a, b| self.compare_reading_order(a, b));

          Ok(StructuredDocument { elements })
      }
  }

  7. Integration with Your Existing Code

  Modify your PdfExtractor:

  // crates/pdf/src/extractor.rs
  pub struct PdfOptions {
      // Existing options...
      pub extract_structure: bool,  // New!
  }

  impl PdfExtractor {
      pub fn extract_document(&self, path: &Path) -> Result<Document> {
          let pdfium = Pdfium::new()?;
          let doc = pdfium.load_pdf_from_file(path)?;

          if self.options.extract_structure {
              // Full document structure
              let structure = StructureDetector::new()
                  .analyze_document(&pdfium, &doc)?;

              // Combine with existing table detection
              let tables = self.detector.detect_tables(&text)?;

              Document::from_structure(structure, tables)
          } else {
              // Current behavior - tables only
              self.extract_tables_from_path(path)
          }
      }
  }

  8. Key Challenges & Solutions

  | Challenge       | Solution                                                 |
  |-----------------|----------------------------------------------------------|
  | Scanned PDFs    | Use OCR with layout preservation (Tesseract HOCR output) |
  | Complex layouts | Start with simple docs, add complexity gradually         |
  | Performance     | Cache analysis results, process pages in parallel        |
  | Accuracy        | Use ML models (like LayoutLM) for better classification  |

  9. Incremental Development Path

  // Phase 1: Basic structure (1-2 weeks)
  struct BasicDocument {
      headings: Vec<(usize, String)>,  // (level, text)
      paragraphs: Vec<String>,
      tables: Vec<TableRegion>,        // Already have this!
  }

  // Phase 2: Positioned elements (2-3 weeks)
  struct PositionedDocument {
      elements: Vec<Element>,
      reading_order: Vec<usize>,
  }

  // Phase 3: Rich elements (3-4 weeks)
  struct RichDocument {
      elements: Vec<Element>,
      images: Vec<Image>,
      formulas: Vec<Formula>,
      metadata: DocumentMetadata,
  }

  The beauty is you already have:
  - ✅ PDF parsing (lopdf, pdfium)
  - ✅ Table detection
  - ✅ OCR pipeline
  - ✅ Text extraction

  You just need to add the positioning data and classification logic to transform raw text into structured
  documents!


