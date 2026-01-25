# File Formats

PipTable supports importing and exporting various file formats for data interchange.

## Supported Formats

| Format | Extension | Import | Export | Multi-Sheet | Notes |
|--------|-----------|--------|--------|-------------|-------|
| CSV | .csv | ✅ | ✅ | ❌ | Configurable delimiter |
| TSV | .tsv | ✅ | ✅ | ❌ | Tab-delimited CSV |
| Excel | .xlsx | ✅ | ✅ | ✅ | Full workbook support |
| JSON | .json | ✅ | ✅ | ❌ | Array or object format |
| Parquet | .parquet | ✅ | ✅ | ❌ | Columnar storage |
| TOON | .toon | ✅ | ✅ | ❌ | PipTable native format |

## CSV/TSV Format

### Import Options

```piptable
' Basic CSV import
dim data = import "data.csv" into sheet

' Multiple CSV files into book
dim all_data = import "file1.csv,file2.csv,file3.csv" into book

' With custom delimiter (planned)
dim data = import "data.txt" with {"delimiter": "|"} into sheet
```

### Export Options

```piptable
' Basic CSV export
export sheet to "output.csv"

' With options (planned)
export sheet to "output.csv" with {
    "delimiter": "|",
    "headers": false,
    "quote": "'"
}
```

### CSV Features
- **Headers**: First row treated as column names by default
- **Delimiter**: Comma (,) default, configurable
- **Quoting**: RFC 4180 compliant quote handling
- **Encoding**: UTF-8 by default
- **Large files**: Streaming for memory efficiency

## Excel Format (.xlsx)

### Import Options

```piptable
' Import all sheets
dim workbook = import "data.xlsx" into book

' Import specific sheet (planned)
dim sheet1 = import "data.xlsx" sheet "Sheet1" into sheet

' Import with options
dim data = import "data.xlsx" with {
    "has_headers": true,
    "sheet": "Data"
} into sheet
```

### Export Options

```piptable
' Export single sheet to Excel
export sheet to "output.xlsx"

' Export entire book
export book to "workbook.xlsx"

' With formatting (planned)
export sheet to "styled.xlsx" with {
    "freeze_panes": "A2",
    "autofilter": true,
    "column_widths": "auto"
}
```

### Excel Features
- **Multi-sheet**: Full workbook support via Book
- **Cell types**: All CellValue types preserved
- **Formulas**: Read as calculated values
- **Date handling**: DateTime cells preserved
- **Large files**: Memory-efficient streaming

### Excel Limitations
- Formulas not preserved on export (values only)
- Formatting/styles not preserved
- Charts and images not supported
- Macros not supported

## JSON Format

### Import Options

```piptable
' Array of objects (records)
dim data = import "data.json" into sheet
' [{"name": "Alice", "age": 30}, {"name": "Bob", "age": 25}]

' Nested JSON (flattened)
dim flat = import "nested.json" into sheet
```

### Export Options

```piptable
' Export as array of objects
export sheet to "output.json"

' Export with formatting (planned)
export sheet to "pretty.json" with {
    "indent": 2,
    "format": "records"  ' or "columns"
}
```

### JSON Formats

#### Records Format (Default)
```json
[
  {"id": 1, "name": "Alice", "score": 95},
  {"id": 2, "name": "Bob", "score": 87}
]
```

#### Columns Format
```json
{
  "id": [1, 2],
  "name": ["Alice", "Bob"],
  "score": [95, 87]
}
```

#### Split Format (Pandas-style)
```json
{
  "columns": ["id", "name", "score"],
  "data": [[1, "Alice", 95], [2, "Bob", 87]]
}
```

## Parquet Format

### Import Options

```piptable
' Basic Parquet import
dim data = import "data.parquet" into sheet

' With schema inference
dim typed_data = import "data.parquet" with {
    "infer_types": true
} into sheet
```

### Export Options

```piptable
' Basic Parquet export
export sheet to "output.parquet"

' With compression
export sheet to "compressed.parquet" with {
    "compression": "snappy"  ' or "gzip", "lz4", "zstd"
}
```

### Parquet Features
- **Columnar storage**: Efficient for analytics
- **Compression**: Multiple algorithms supported
- **Type preservation**: Strong typing maintained
- **Schema evolution**: Compatible schema changes
- **Performance**: Fast for large datasets

## TOON Format

PipTable's native format for preserving all data types and metadata.

```piptable
' Save with full fidelity
export sheet to "data.toon"

' Load preserving all types
dim data = import "data.toon" into sheet
```

### TOON Features
- **Full type preservation**: All CellValue types
- **Metadata**: Column names, sheet names preserved
- **Compact**: Binary serialization
- **Version compatible**: Forward/backward compatibility

## Format Selection Guide

### When to use CSV
- Simple data exchange
- Human-readable output
- Wide compatibility
- Single table data

### When to use Excel
- Business users
- Multiple related tables
- Need formatting (future)
- Microsoft ecosystem

### When to use JSON
- Web APIs
- JavaScript integration
- Document stores
- Nested structures

### When to use Parquet
- Large datasets
- Analytics workloads
- Data warehouses
- Columnar operations

### When to use TOON
- PipTable-to-PipTable transfer
- Type preservation critical
- Internal storage
- Backup/archive

## Type Mapping

How CellValue types map to different formats:

| CellValue | CSV | Excel | JSON | Parquet |
|-----------|-----|-------|------|---------|
| String | Text | Text | string | STRING |
| Int | Number text | Number | number | INT64 |
| Float | Number text | Number | number | DOUBLE |
| Bool | "true"/"false" | Boolean | boolean | BOOLEAN |
| DateTime | ISO 8601 text | Date/Time | string (ISO) | TIMESTAMP |
| Null | Empty | Empty cell | null | NULL |

## Performance Considerations

### Memory Usage
- **Streaming formats**: CSV, TSV (low memory)
- **In-memory formats**: JSON, small Excel files
- **Chunked processing**: Parquet, large Excel files

### Speed Comparison (relative)
| Operation | CSV | Excel | JSON | Parquet |
|-----------|-----|-------|------|---------|
| Read small | Fast | Medium | Fast | Medium |
| Read large | Medium | Slow | Slow | Fast |
| Write small | Fast | Medium | Fast | Medium |
| Write large | Medium | Slow | Slow | Fast |

## Error Handling

Common format-specific errors:

### CSV Errors
- `InvalidDelimiter`: Delimiter not found
- `InconsistentColumns`: Row column count mismatch
- `EncodingError`: Invalid UTF-8

### Excel Errors
- `SheetNotFound`: Specified sheet doesn't exist
- `CorruptedFile`: Invalid Excel structure
- `UnsupportedVersion`: Old Excel format (.xls)

### JSON Errors
- `InvalidJSON`: Malformed JSON syntax
- `UnexpectedStructure`: Not array or object format
- `TypeMismatch`: Inconsistent types in array

### Parquet Errors
- `SchemaIncompatible`: Type mismatch with schema
- `CompressionError`: Unsupported compression
- `VersionMismatch`: Unsupported Parquet version

## Examples

### Data Pipeline with Multiple Formats

```piptable
' Import from JSON API
dim api_data = fetch("https://api.example.com/data").json()

' Convert to sheet
dim sheet = Sheet::from_records(api_data)

' Process data
dim processed = query("
    SELECT category, SUM(amount) as total
    FROM sheet
    GROUP BY category
")

' Export to multiple formats
export processed to "report.xlsx"  ' For business users
export processed to "report.parquet"  ' For data warehouse
export processed to "report.json"  ' For web dashboard
```

### Format Conversion Utility

```piptable
' Generic format converter
sub convert_file(input_path, output_path)
    ' Detect format by extension
    dim data = import input_path into sheet
    
    ' Add metadata
    data.set_name("Converted_" + now())
    
    ' Export to target format
    export data to output_path
    
    print("Converted: " + input_path + " -> " + output_path)
end sub

' Usage
convert_file("data.csv", "data.parquet")
convert_file("report.xlsx", "report.json")
```

## See Also

- [Sheet API](sheet.md) - Working with imported data
- [Book API](book.md) - Multi-sheet file handling
- [Import/Export Statements](../dsl/statements.md#import-export) - DSL syntax