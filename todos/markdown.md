# Markdown Import/Export for Document Structure

## Summary
Add Markdown import/export capabilities to support document structure conversion, enabling seamless transformation between Markdown and our `StructuredDocument` format. This complements the PDF structure detection feature (#234) and enables LLM-ready document processing.

## Approach

### Import (Markdown → StructuredDocument)
Use **Comrak** for parsing. It provides a rich, traversable AST and supports GitHub-style features (tables, task lists, strikethrough), which maps cleanly to our `StructuredDocument` model.

### Export (StructuredDocument → Markdown)
Generate markdown directly from our structured model for deterministic output. This keeps formatting stable and avoids coupling to renderer quirks.

### Optional Normalization
If we need to normalize external markdown (or round-trip), consider pulldown-cmark + pulldown-cmark-to-cmark later for event-based rendering.

**Rationale**: Backend service benefits from Comrak's richer AST; export can stay simple and controlled.

## Implementation

### 1. Dependencies

```toml
# Cargo.toml
[dependencies]
# For markdown parsing (import)
comrak = { version = "0.28", default-features = false }

# Later, if we need normalization (optional)
pulldown-cmark = { version = "0.11", optional = true }
pulldown-cmark-to-cmark = { version = "15", optional = true }

[features]
markdown-normalize = ["pulldown-cmark", "pulldown-cmark-to-cmark"]
```

**Note**: Skip `syntect` feature initially - it adds 6MB+ to binary size and is only needed for syntax highlighting.

### 2. Core Implementation

Create `crates/pdf/src/markdown.rs`:

```rust
use comrak::{Arena, ComrakOptions, parse_document};
use comrak::nodes::{AstNode, NodeValue, ListType};
use crate::structure::{StructuredDocument, Element, DocumentMetadata};
use crate::detector::TableRegion;
use crate::error::Result;

impl StructuredDocument {
    /// Import from markdown string
    pub fn from_markdown(md: &str) -> Result<Self> {
        let arena = Arena::new();
        let mut options = ComrakOptions::default();
        
        // Enable GitHub Flavored Markdown extensions
        options.extension.table = true;
        options.extension.strikethrough = true;
        options.extension.tasklist = true;
        options.extension.autolink = true;
        options.extension.footnotes = true;
        
        let root = parse_document(&arena, md, &options);
        let mut elements = Vec::new();
        
        Self::traverse_ast(root, &mut elements)?;
        
        Ok(StructuredDocument { 
            elements,
            metadata: DocumentMetadata::default(),
        })
    }
    
    /// Traverse Comrak AST and extract elements
    fn traverse_ast<'a>(node: &'a AstNode<'a>, elements: &mut Vec<Element>) -> Result<()> {
        for child in node.children() {
            match &child.data.borrow().value {
                NodeValue::Heading(h) => {
                    let text = Self::extract_text(child);
                    elements.push(Element::Heading(h.level as usize, text));
                }
                
                NodeValue::Paragraph => {
                    let text = Self::extract_text(child);
                    if !text.trim().is_empty() {
                        elements.push(Element::Paragraph(text));
                    }
                }
                
                NodeValue::List(list) => {
                    let items = Self::extract_list_items(child);
                    elements.push(Element::List {
                        ordered: list.list_type == ListType::Ordered,
                        items,
                    });
                }
                
                NodeValue::Table(_) => {
                    let table = Self::extract_table(child)?;
                    elements.push(Element::Table(table));
                }
                
                NodeValue::CodeBlock(code) => {
                    elements.push(Element::CodeBlock {
                        language: code.info.clone(),
                        content: code.literal.clone(),
                    });
                }
                
                NodeValue::BlockQuote => {
                    let content = Self::extract_text(child);
                    elements.push(Element::Quote(content));
                }
                
                NodeValue::Image(link) => {
                    elements.push(Element::Image {
                        alt: link.title.clone(),
                        url: link.url.clone(),
                    });
                }
                
                NodeValue::TaskItem(checked) => {
                    let text = Self::extract_text(child);
                    elements.push(Element::TaskItem {
                        checked: *checked,
                        text,
                    });
                }
                
                NodeValue::FootnoteDefinition(name) => {
                    let content = Self::extract_text(child);
                    elements.push(Element::Footnote {
                        label: name.clone(),
                        content,
                    });
                }
                
                NodeValue::Math(math) => {
                    elements.push(Element::Formula {
                        latex: math.literal.clone(),
                        display: math.display_math,
                    });
                }
                
                _ => {
                    // Recurse for container nodes
                    Self::traverse_ast(child, elements)?;
                }
            }
        }
        Ok(())
    }
    
    /// Extract text content from a node and its children
    fn extract_text<'a>(node: &'a AstNode<'a>) -> String {
        let mut text = String::new();
        Self::collect_text(node, &mut text);
        text
    }
    
    fn collect_text<'a>(node: &'a AstNode<'a>, output: &mut String) {
        match &node.data.borrow().value {
            NodeValue::Text(t) | NodeValue::Code(t) => {
                output.push_str(&t.literal);
            }
            NodeValue::SoftBreak | NodeValue::LineBreak => {
                output.push(' ');
            }
            _ => {
                for child in node.children() {
                    Self::collect_text(child, output);
                }
            }
        }
    }
    
    /// Extract list items from a list node
    fn extract_list_items<'a>(list_node: &'a AstNode<'a>) -> Vec<String> {
        let mut items = Vec::new();
        
        for item in list_node.children() {
            if let NodeValue::Item(_) = &item.data.borrow().value {
                items.push(Self::extract_text(item));
            }
        }
        
        items
    }
    
    /// Extract table from a table node
    fn extract_table<'a>(table_node: &'a AstNode<'a>) -> Result<TableRegion> {
        let mut rows = Vec::new();
        let mut headers = None;
        let mut in_header = false;
        
        for child in table_node.children() {
            match &child.data.borrow().value {
                NodeValue::TableHead => {
                    in_header = true;
                    for row in child.children() {
                        if let NodeValue::TableRow = &row.data.borrow().value {
                            let cells = Self::extract_table_row(row);
                            headers = Some(cells);
                        }
                    }
                    in_header = false;
                }
                NodeValue::TableRow => {
                    if !in_header {
                        let cells = Self::extract_table_row(child);
                        rows.push(cells);
                    }
                }
                _ => {}
            }
        }
        
        Ok(TableRegion {
            rows,
            headers,
            start_line: 0, // Will be set if needed
            end_line: 0,
        })
    }
    
    fn extract_table_row<'a>(row_node: &'a AstNode<'a>) -> Vec<String> {
        let mut cells = Vec::new();
        
        for cell in row_node.children() {
            if let NodeValue::TableCell = &cell.data.borrow().value {
                cells.push(Self::extract_text(cell));
            }
        }
        
        cells
    }
}
```

### 3. Export Implementation

```rust
impl StructuredDocument {
    /// Export to clean, LLM-ready markdown
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        let mut last_was_list = false;
        
        for element in &self.elements {
            // Add spacing between different element types
            if !md.is_empty() && !last_was_list {
                md.push_str("\n");
            }
            
            match element {
                Element::Heading(level, text) => {
                    md.push_str(&"#".repeat(*level));
                    md.push(' ');
                    md.push_str(text);
                    md.push_str("\n\n");
                    last_was_list = false;
                }
                
                Element::Paragraph(text) => {
                    md.push_str(text);
                    md.push_str("\n\n");
                    last_was_list = false;
                }
                
                Element::List { ordered, items } => {
                    for (i, item) in items.iter().enumerate() {
                        if *ordered {
                            md.push_str(&format!("{}. {}\n", i + 1, item));
                        } else {
                            md.push_str(&format!("- {}\n", item));
                        }
                    }
                    md.push('\n');
                    last_was_list = true;
                }
                
                Element::Table(table) => {
                    md.push_str(&Self::table_to_markdown(table));
                    md.push_str("\n\n");
                    last_was_list = false;
                }
                
                Element::CodeBlock { language, content } => {
                    md.push_str("```");
                    md.push_str(language);
                    md.push('\n');
                    md.push_str(content);
                    if !content.ends_with('\n') {
                        md.push('\n');
                    }
                    md.push_str("```\n\n");
                    last_was_list = false;
                }
                
                Element::Quote(text) => {
                    for line in text.lines() {
                        md.push_str("> ");
                        md.push_str(line);
                        md.push('\n');
                    }
                    md.push('\n');
                    last_was_list = false;
                }
                
                Element::Image { alt, url } => {
                    md.push_str(&format!("![{}]({})\n\n", alt, url));
                    last_was_list = false;
                }
                
                Element::Formula { latex, display } => {
                    if *display {
                        // Display math (block)
                        md.push_str("$$\n");
                        md.push_str(latex);
                        md.push_str("\n$$\n\n");
                    } else {
                        // Inline math
                        md.push_str(&format!("${}$", latex));
                    }
                    last_was_list = false;
                }
                
                Element::TaskItem { checked, text } => {
                    let checkbox = if *checked { "[x]" } else { "[ ]" };
                    md.push_str(&format!("- {} {}\n", checkbox, text));
                    last_was_list = false;
                }
                
                Element::Footnote { label, content } => {
                    md.push_str(&format!("[^{}]: {}\n\n", label, content));
                    last_was_list = false;
                }
            }
        }
        
        md.trim_end().to_string()
    }
    
    /// Convert table to GitHub Flavored Markdown table
    fn table_to_markdown(table: &TableRegion) -> String {
        let mut md = String::new();
        
        // Determine if we have headers
        if let Some(headers) = &table.headers {
            // Header row
            md.push('|');
            for header in headers {
                md.push_str(&format!(" {} |", Self::escape_table_cell(header)));
            }
            md.push('\n');
            
            // Separator row
            md.push('|');
            for _ in headers {
                md.push_str(" --- |");
            }
            md.push('\n');
        }
        
        // Data rows
        for row in &table.rows {
            md.push('|');
            for cell in row {
                md.push_str(&format!(" {} |", Self::escape_table_cell(cell)));
            }
            md.push('\n');
        }
        
        md
    }
    
    /// Escape special characters in table cells
    fn escape_table_cell(text: &str) -> String {
        text.replace('|', "\\|")
            .replace('\n', " ")
            .replace('\r', "")
    }
}
```

### 4. Extended Element Types

Update the `Element` enum to support richer content:

```rust
// crates/pdf/src/structure.rs
pub enum Element {
    Heading(usize, String),
    Paragraph(String),
    List { ordered: bool, items: Vec<String> },
    Table(TableRegion),
    Image { alt: String, url: String },
    Formula { latex: String, display: bool },
    CodeBlock { language: String, content: String },
    Quote(String),
    TaskItem { checked: bool, text: String },
    Footnote { label: String, content: String },
    HorizontalRule,
}
```

### 5. Integration with DSL

```rust
// Future DSL integration
import "document.md" into doc as markdown
export doc to "output.pdf"

// Or in Rust API
let doc = StructuredDocument::from_markdown(md_content)?;
let pdf = doc.to_pdf()?;
```

### 6. Utility Functions

```rust
impl StructuredDocument {
    /// Extract all tables from the document
    pub fn extract_tables(&self) -> Vec<&TableRegion> {
        self.elements
            .iter()
            .filter_map(|e| match e {
                Element::Table(t) => Some(t),
                _ => None
            })
            .collect()
    }
    
    /// Extract all headings with their hierarchy
    pub fn extract_outline(&self) -> Vec<(usize, &str)> {
        self.elements
            .iter()
            .filter_map(|e| match e {
                Element::Heading(level, text) => Some((*level, text.as_str())),
                _ => None
            })
            .collect()
    }
    
    /// Convert to plain text (for search/indexing)
    pub fn to_plain_text(&self) -> String {
        let mut text = String::new();
        
        for element in &self.elements {
            match element {
                Element::Heading(_, t) | 
                Element::Paragraph(t) |
                Element::Quote(t) => {
                    text.push_str(t);
                    text.push_str("\n\n");
                }
                Element::List { items, .. } => {
                    for item in items {
                        text.push_str(item);
                        text.push('\n');
                    }
                    text.push('\n');
                }
                Element::CodeBlock { content, .. } => {
                    text.push_str(content);
                    text.push_str("\n\n");
                }
                _ => {}
            }
        }
        
        text
    }
}
```

### 7. Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_markdown_import() {
        let md = r#"# Title
        
This is a paragraph with **bold** and *italic* text.

## Section 1

- Item 1
- Item 2
- Item 3

### Subsection

| Column 1 | Column 2 |
| -------- | -------- |
| Cell 1   | Cell 2   |
| Cell 3   | Cell 4   |

```rust
fn main() {
    println!("Hello, world!");
}
```"#;
        
        let doc = StructuredDocument::from_markdown(md).unwrap();
        
        // Verify structure
        assert!(matches!(&doc.elements[0], Element::Heading(1, t) if t == "Title"));
        assert!(matches!(&doc.elements[1], Element::Paragraph(_)));
        assert!(matches!(&doc.elements[2], Element::Heading(2, _)));
        assert!(matches!(&doc.elements[3], Element::List { ordered: false, .. }));
        assert!(matches!(&doc.elements[4], Element::Heading(3, _)));
        assert!(matches!(&doc.elements[5], Element::Table(_)));
        assert!(matches!(&doc.elements[6], Element::CodeBlock { language, .. } if language == "rust"));
    }
    
    #[test]
    fn test_markdown_export() {
        let mut doc = StructuredDocument::default();
        doc.elements.push(Element::Heading(1, "Test Document".to_string()));
        doc.elements.push(Element::Paragraph("This is a test.".to_string()));
        doc.elements.push(Element::List {
            ordered: true,
            items: vec!["First".to_string(), "Second".to_string()],
        });
        
        let md = doc.to_markdown();
        
        assert!(md.contains("# Test Document"));
        assert!(md.contains("This is a test."));
        assert!(md.contains("1. First"));
        assert!(md.contains("2. Second"));
    }
    
    #[test]
    fn test_markdown_round_trip() {
        let original = r#"# Title

This is a paragraph.

## Section

- Item 1
- Item 2

| Col1 | Col2 |
| --- | --- |
| A | B |
"#;

        let doc = StructuredDocument::from_markdown(original).unwrap();
        let exported = doc.to_markdown();
        
        // Should preserve structure (not necessarily exact formatting)
        assert!(exported.contains("# Title"));
        assert!(exported.contains("This is a paragraph"));
        assert!(exported.contains("## Section"));
        assert!(exported.contains("- Item 1"));
        assert!(exported.contains("- Item 2"));
        assert!(exported.contains("| Col1 | Col2 |"));
        assert!(exported.contains("| A | B |"));
    }
    
    #[test]
    fn test_github_flavored_markdown() {
        let gfm = r#"# Tasks

- [x] Completed task
- [ ] Pending task

~~strikethrough~~

```python
def hello():
    print("world")
```

> This is a quote
> spanning multiple lines

[^1]: This is a footnote

$$
E = mc^2
$$"#;

        let doc = StructuredDocument::from_markdown(gfm).unwrap();
        
        // Find task items
        let has_task = doc.elements.iter().any(|e| 
            matches!(e, Element::TaskItem { .. })
        );
        assert!(has_task);
        
        // Find code block
        let has_python = doc.elements.iter().any(|e|
            matches!(e, Element::CodeBlock { language, .. } if language == "python")
        );
        assert!(has_python);
        
        // Find quote
        let has_quote = doc.elements.iter().any(|e|
            matches!(e, Element::Quote(_))
        );
        assert!(has_quote);
        
        // Find formula
        let has_formula = doc.elements.iter().any(|e|
            matches!(e, Element::Formula { .. })
        );
        assert!(has_formula);
    }
}
```

### 8. Error Handling

```rust
// crates/pdf/src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum MarkdownError {
    #[error("Failed to parse markdown: {0}")]
    ParseError(String),
    
    #[error("Invalid table structure")]
    InvalidTable,
    
    #[error("Unsupported markdown element: {0}")]
    UnsupportedElement(String),
}
```

### 9. Benchmarks

```rust
#[cfg(all(test, not(target_env = "msvc")))]
mod bench {
    use super::*;
    use test::Bencher;
    
    #[bench]
    fn bench_parse_large_markdown(b: &mut Bencher) {
        let md = std::fs::read_to_string("tests/fixtures/large.md").unwrap();
        b.iter(|| {
            StructuredDocument::from_markdown(&md).unwrap()
        });
    }
    
    #[bench]
    fn bench_export_large_document(b: &mut Bencher) {
        let md = std::fs::read_to_string("tests/fixtures/large.md").unwrap();
        let doc = StructuredDocument::from_markdown(&md).unwrap();
        b.iter(|| {
            doc.to_markdown()
        });
    }
}
```

## Workflow Example

```rust
// Complete workflow example
use piptable_pdf::{PdfDocument, StructuredDocument, PdfOptions};

// 1. Import from PDF
let pdf_doc = PdfDocument::extract("paper.pdf", PdfOptions {
    extract_structure: true,
    ocr_enabled: true,
    ..Default::default()
})?;

// 2. Convert to structured document
let structured = pdf_doc.to_structured_document();

// 3. Export to markdown for LLM
let markdown = structured.to_markdown();
std::fs::write("paper.md", markdown)?;

// 4. Or import existing markdown
let existing_md = std::fs::read_to_string("notes.md")?;
let md_doc = StructuredDocument::from_markdown(&existing_md)?;

// 5. Extract specific elements
let tables = md_doc.extract_tables();
let outline = md_doc.extract_outline();

// 6. Convert to other formats
let json = md_doc.to_llm_json();
let plain = md_doc.to_plain_text();
```

## Benefits

1. **Bidirectional conversion**: Import and export markdown seamlessly
2. **LLM optimization**: Clean, well-structured markdown output
3. **GitHub compatibility**: Full GFM support (tables, task lists, etc.)
4. **Performance**: Comrak is fast and battle-tested
5. **Extensibility**: Easy to add new element types
6. **Integration**: Works with existing PDF structure detection

## Dependencies Summary

- **comrak**: Markdown parsing (import) - rich AST, GFM support
- **No syntect**: Avoiding 6MB+ binary size increase
- **Optional normalization**: pulldown-cmark can be added later if needed

## Testing Strategy

1. Unit tests for each element type
2. Round-trip tests (markdown → document → markdown)
3. GitHub Flavored Markdown features
4. Large document benchmarks
5. Edge cases (empty tables, nested lists, etc.)

## References

- [Comrak Documentation](https://docs.rs/comrak/)
- [CommonMark Spec](https://spec.commonmark.org/)
- [GitHub Flavored Markdown Spec](https://github.github.com/gfm/)
- [pulldown-cmark](https://docs.rs/pulldown-cmark/) (for future normalization)
- [GitHub Issue #235](https://github.com/bwalkt/piptable/issues/235)