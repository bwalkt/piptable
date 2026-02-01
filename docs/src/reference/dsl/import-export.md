# Import/Export Statements

PipTable provides comprehensive import and export capabilities for working with various file formats.

## Import Statement

### Syntax

```piptable
import <source> [with <options>] into <target>
```

- `<source>`: File path, URL, or expression
- `<options>`: Optional configuration object
- `<target>`: `sheet` or `book`

### Basic Import

```piptable
' Import single file
dim data = import "data.csv" into sheet
dim workbook = import "report.xlsx" into book

' Import from URL
dim remote = import "https://example.com/data.json" into sheet

' Import multiple files
dim all_data = import "file1.csv,file2.csv,file3.csv" into book
```

### Import with Options

```piptable
' CSV with custom delimiter
dim data = import "data.txt" with {
    "delimiter": "|",
    "has_headers": true
} into sheet

' Excel specific sheet
dim sheet1 = import "workbook.xlsx" with {
    "sheet": "Sheet1"
} into sheet

' JSON with specific format
dim records = import "data.json" with {
    "format": "records"
} into sheet
```

### Markdown Import

```piptable
' Import all tables from a Markdown file into a book
dim tables = import "README.md" into book

' Access tables by name (table_1, table_2, ...)
dim first = tables["table_1"]

' Import from GitHub README
dim readme_url = "https://raw.githubusercontent.com/org/repo/main/README.md"
dim github_tables = import readme_url into book
```

## Export Statement

### Syntax

```piptable
export <source> to <destination> [with <options>]
```

- `<source>`: Sheet or Book variable
- `<destination>`: File path
- `<options>`: Optional configuration object

### Basic Export

```piptable
' Export to different formats
export sheet to "output.csv"
export sheet to "output.xlsx"
export sheet to "output.json"
export sheet to "output.parquet"
export sheet to "output.toon"

' Export entire book
export book to "workbook.xlsx"
```

### Export with Options

```piptable
' CSV with custom settings
export sheet to "output.csv" with {
    "delimiter": "\t",
    "quote": "'",
    "headers": false
}

' Pretty JSON
export sheet to "output.json" with {
    "indent": 2,
    "format": "records"
}

' Compressed Parquet
export sheet to "output.parquet" with {
    "compression": "snappy"
}
```

## Format Detection

File format is automatically detected from extension:

| Extension | Format | Target |
| ----------- | -------- | -------- |
| .csv, .tsv | CSV/TSV | sheet |
| .xlsx, .xls | Excel | sheet/book |
| .json, .jsonl | JSON | sheet |
| .parquet | Parquet | sheet |
| .toon | TOON | sheet |
| .md | Markdown | book |
| .pdf | PDF | sheet/book |

Markdown and PDF imports accept optional table extraction options:
`has_headers`, `detect_headers`, `min_table_rows`, `min_table_cols`, `min_table_size`.
`page_range` applies to PDFs only and is ignored for Markdown.

## Dynamic Import/Export

### Variable Paths

```piptable
dim filename = "data_" + format_date(now(), "YYYY-MM-DD") + ".csv"
dim data = import filename into sheet

dim output_dir = "/exports/"
export sheet to output_dir + "report.xlsx"
```

### Conditional Import

```piptable
dim format = get_env("DATA_FORMAT", "csv")
dim path = "data." + format

dim data = import path into sheet
```

### Batch Processing

```piptable
' Process multiple files
dim files = ["jan.csv", "feb.csv", "mar.csv"]
dim all_data = new Book()

for file in files
    dim month_data = import file into sheet
    all_data.add_sheet(month_data, file.replace(".csv", ""))
next

export all_data to "quarterly_report.xlsx"
```

## Import Sources

### File System

```piptable
' Relative path
dim data = import "data/sales.csv" into sheet

' Absolute path
dim data = import "/usr/share/data/report.xlsx" into book

' Home directory
dim data = import "~/Documents/data.json" into sheet
```

### URLs

```piptable
' HTTP/HTTPS
dim remote = import "https://api.example.com/data.json" into sheet

' With authentication (future)
dim secured = import "https://api.example.com/data" with {
    "headers": {
        "Authorization": "Bearer " + token
    }
} into sheet
```

### Inline Data

```piptable
' CSV string
dim csv_text = "name,age\nAlice,30\nBob,25"
dim data = import csv_text as csv into sheet

' JSON string
dim json_text = '[{"name": "Alice", "age": 30}]'
dim data = import json_text as json into sheet

```

## Error Handling

```piptable
' Basic error handling
try
    dim data = import "missing.csv" into sheet
catch error
    print("Import failed: " + error.message)
    ' Use fallback
    dim data = create_empty_sheet()
end try

' Validation after import
dim data = import "data.csv" into sheet
if data.row_count() == 0 then
    error("No data found in file")
end if

' Check for required columns
dim required = ["id", "name", "value"]
for col in required
    if not data.has_column(col) then
        error("Missing required column: " + col)
    end if
next
```

## Advanced Examples

### ETL Pipeline

```piptable
' Extract from multiple sources
dim sales = import "sales.csv" into sheet
dim products = import "products.xlsx" with {"sheet": "Products"} into sheet
dim customers = import "https://api.example.com/customers" into sheet

' Transform with SQL
dim report = query("
    SELECT c.name as customer,
           p.product_name,
           s.quantity,
           s.quantity * p.price as total
    FROM sales s
    JOIN products p ON s.product_id = p.id
    JOIN customers c ON s.customer_id = c.id
")

' Load to multiple formats
export report to "report.xlsx"     ' For Excel users
export report to "report.parquet"  ' For data warehouse
export report to "report.json"     ' For web API
```

### Format Conversion

```piptable
function convert_file(input_file, output_file)
    ' Auto-detect input format
    dim data = import input_file into sheet
    
    ' Add metadata
    data.set_metadata("converted_at", now())
    data.set_metadata("source_file", input_file)
    
    ' Auto-detect output format from extension
    export data to output_file
    
    return data.row_count()
end function

' Usage
dim rows = convert_file("legacy.csv", "modern.parquet")
print("Converted " + str(rows) + " rows")
```

### Markdown Report Processing

```piptable
dim report = import "weekly_report.md" into book

' Process each table
for i = 0 to report.sheet_count() - 1
    dim table = report.get_sheet(i)
    
    ' Skip small tables (likely formatting)
    if table.row_count() < 3 then
        continue
    end if
    
    ' Detect metrics table by headers
    if table.has_column("Metric") and table.has_column("Value") then
        ' Export metrics to dashboard
        export table to "metrics_week_" + str(week_number()) + ".json"
    end if
next
```

## Best Practices

### 1. Always Validate Imports

```piptable
function safe_import(file_path)
    try
        dim data = import file_path into sheet
        
        ' Validate
        if data.row_count() == 0 then
            error("Empty file")
        end if
        
        return data
    catch e
        log_error("Import failed: " + e.message)
        return null
    end try
end function
```

### 2. Use Appropriate Formats

- **CSV**: Simple data exchange, maximum compatibility
- **Excel**: Business users, formatted reports
- **JSON**: Web APIs, nested data
- **Parquet**: Large datasets, analytics
- **TOON**: Internal storage, type preservation
- **Markdown**: Documentation extraction

### 3. Handle Missing Files

```piptable
function import_or_create(file_path, default_headers)
    if file_exists(file_path) then
        return import file_path into sheet
    else
        ' Create new sheet with headers
        dim sheet = new Sheet()
        sheet.row_append(default_headers)
        return sheet
    end if
end function
```

### 4. Batch Operations

```piptable
' Import directory of files
function import_directory(dir_path, pattern)
    dim files = list_files(dir_path, pattern)
    dim book = new Book()
    
    for file in files
        dim sheet = import file into sheet
        dim name = file_basename(file).replace(file_extension(file), "")
        book.add_sheet(sheet, name)
    next
    
    return book
end function

' Usage
dim all_reports = import_directory("./reports/", "*.csv")
export all_reports to "consolidated_reports.xlsx"
```

## See Also

- [File Formats](../api/formats.md) - Detailed format specifications
- [Sheet API](../api/sheet.md) - Working with imported data
- [Book API](../api/book.md) - Multi-sheet operations
- [Cookbook: Import](../../cookbook/import.md) - Import recipes
- [Cookbook: Export](../../cookbook/export.md) - Export recipes
- [Cookbook: Markdown](../../cookbook/markdown.md) - Markdown table extraction
