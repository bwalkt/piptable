# PDF Import DSL Documentation

PipTable provides comprehensive PDF import capabilities through its DSL, supporting both table extraction and document structure extraction.

## Basic PDF Import

### Import Tables from PDF

```vba
' Import all tables from a PDF (returns a book-style object)
import "report.pdf" into tables

' Import with specific page range
import "report.pdf" into tables with { "page_range": "1-10" }

' Import with multiple options
import "financial.pdf" into data with {
    "page_range": "5-20",
    "min_table_rows": 3,
    "min_table_cols": 2,
    "detect_headers": true
}
```

### Import Document Structure

```vba
' Extract document structure (headings, paragraphs)
import "paper.pdf" into doc with { "extract_structure": true }

' Alias for extract_structure
import "research.pdf" into content with { "structure": true }

' Combine with page range
import "book.pdf" into chapters with {
    "extract_structure": true,
    "page_range": "1-50"
}
```

## Import Options

All PDF import options can be specified using the `with` clause:

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `page_range` | String | All pages | Pages to process (e.g., "1-10", "5-5") |
| `min_table_rows` | Integer | 2 | Minimum rows for table detection |
| `min_table_cols` | Integer | 2 | Minimum columns for table detection |
| `detect_headers` | Boolean | false | Auto-detect table headers |
| `extract_structure` | Boolean | false | Extract document structure instead of tables |
| `structure` | Boolean | false | Alias for extract_structure |

## Table Extraction Examples

### Financial Reports

```vba
' Import quarterly report tables
import "Q3_2024.pdf" into quarterly with {
    "page_range": "10-25",
    "min_table_rows": 4,
    "detect_headers": true
}

' Process the first table
dim table = quarterly.table_1
select 
    Revenue,
    Expenses,
    Revenue - Expenses as Profit
from table
where Revenue > 0
```

### Data Sheets

```vba
' Import product specifications
import "specs.pdf" into products with {
    "min_table_cols": 3,
    "detect_headers": true
}

' Filter and export
dim table = products.table_1
dim results = select * from table
where Price < 100
export results to "affordable_products.csv"
```

## Document Structure Extraction

### Research Papers

```vba
' Extract paper structure
import "research_paper.pdf" into paper with { "extract_structure": true }

' The result is a JSON structure with elements:
' {
'   "elements": [
'     {
'       "type": "heading",
'       "level": 1,
'       "content": "Introduction",
'       "page": 1,
'       "bbox": { ... }
'     },
'     {
'       "type": "paragraph",
'       "content": "This paper presents...",
'       "page": 1,
'       "bbox": { ... }
'     }
'   ],
'   "metadata": {
'     "page_count": 20
'   }
' }

```

To export the structured document to JSON, use a `.json` target:

```vba
export paper to "paper.json"
```

### Technical Documentation

```vba
' Extract first 3 chapters
import "manual.pdf" into docs with {
    "extract_structure": true,
    "page_range": "1-100"
}

' Process the structured content
dim headings = filter(docs.elements, e => e.type == "heading")
for each h in headings
    print("Level " + h.level + ": " + h.content)
next
```

## Combined Workflows

### Extract Tables and Structure

```vba
' First extract tables
import "report.pdf" into tables

' Then extract structure separately
import "report.pdf" into structure with { "extract_structure": true }

' Process both
dim first_table = tables.table_1
dim heading_count = count(filter(structure.elements, e => e.type == "heading"))
print("Found table_1 and " + heading_count + " headings")
```

### Multi-File Processing

```vba
' Process multiple PDFs
dim files = ["report1.pdf", "report2.pdf", "report3.pdf"]

for each file in files
    import file into data
    
    ' Process each file's tables
    dim table = data.table_1
    dim results = select 
        file as source_file,
        sum(Revenue) as total_revenue
    from table
    group by source_file
    export results to "consolidated.csv" append
next
```

## Error Handling

```vba
try
    import "document.pdf" into data with {
        "page_range": "1-1000"  ' May exceed actual pages
    }
catch error
    print("Import failed: " + error.message)
    ' Fallback to first 10 pages
    import "document.pdf" into data with { "page_range": "1-10" }
end try
```

## Performance Tips

1. **Page Ranges**: Always specify page ranges for large PDFs to improve performance
2. **Structure vs Tables**: Use `extract_structure` for document text, regular import for tables
3. **OCR**: OCR is not exposed in the DSL; enable it via Rust API if needed
4. **Memory**: Large PDFs with structure extraction may use significant memory

## Output Formats

### Table Import Result
Returns a book-style object of tables (`table_1`, `table_2`, ...).

### Structure Import Result
Returns a JSON object with:
- `elements`: Array of heading and paragraph elements
- `metadata.page_count`: Total pages in document
- Each element includes:
  - `type`: "heading" or "paragraph"
  - `content`: The text content
  - `page`: Page number (1-indexed)
  - `bbox`: Bounding box coordinates
  - `level`: (headings only) 1-4 for H1-H4

## Integration with Export

```vba
' Import PDF structure and export to JSON
import "report.pdf" into doc with { "extract_structure": true }
export doc to "report.json"

' Import tables and export to Excel
import "data.pdf" into tables
export tables.table_1 to "data.xlsx"
```

## Limitations

### Current Limitations (Phase 1)
- Structure extraction assumes single-column layout
- Lists are not yet detected as separate elements
- Tables and structure are extracted separately
- Images are not extracted

### Planned Features
- Multi-column layout support
- List detection (bullets, numbered)
- Integrated table and structure extraction
- Image and caption extraction
- Formula/equation detection
