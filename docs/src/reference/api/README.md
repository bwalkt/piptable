# API Reference

Complete reference for PipTable's APIs and data structures.

## Core APIs

### [Sheet API](sheet.md)
The fundamental data structure for tabular data:
- Cell access and manipulation
- Row and column operations  
- Transformations (map, filter)
- Join operations (inner, left, right, full)
- Append and upsert operations

### [Book API](book.md)
Container for multiple sheets:
- Sheet management (add, remove, rename)
- Consolidation of multiple sheets
- Merging books
- Batch operations across sheets

### [Built-in Functions](functions.md)
Functions available in expressions:
- Type conversion (int, float, str)
- Math functions (sum, avg, min, max, abs)
- Object/array utilities (keys, values, len)
- Planned string and date functions

## Data Operations

### SQL Operations
Execute SQL queries on sheets and books:
```piptable
dim result = query("SELECT * FROM users WHERE age > 18")
```

Features:
- Standard SQL syntax
- Joins between sheets
- Aggregations and grouping
- Window functions (planned)

### HTTP Operations
Make HTTP requests and process responses:
```piptable
dim data = fetch("https://api.example.com/users")
dim json = data.json()
```

Features:
- GET, POST, PUT, DELETE methods
- JSON request/response handling
- Headers and authentication
- Async parallel requests

### AI Operations
Natural language data queries:
```piptable
dim insights = ask "What are the top selling products?" from sales_data
```

Features:
- Natural language to SQL
- Data summarization
- Pattern detection
- Multiple model support

## File Formats

### [Supported Formats](formats.md)
Import and export various file types:

| Format | Import | Export | Notes |
|--------|--------|--------|-------|
| CSV | ✅ | ✅ | Configurable delimiter |
| Excel (.xlsx) | ✅ | ✅ | Multi-sheet support |
| JSON | ✅ | ✅ | Array or object format |
| Parquet | ✅ | ✅ | Efficient columnar storage |
| TSV | ✅ | ✅ | Tab-delimited |

## Data Types

### CellValue
The basic unit of data in sheets:
- `String` - Text data
- `Int` - 64-bit integers
- `Float` - 64-bit floating point
- `Bool` - Boolean values
- `DateTime` - ISO 8601 timestamps
- `Null` - Missing/empty values

### Type Conversion
Automatic and explicit conversions:
```piptable
' Explicit conversion
dim num = int("42")
dim text = str(3.14)

' Automatic in contexts
if value then  ' Truthy/falsy conversion
    print("Has value")
end if
```

## Error Handling

PipTable uses Result types for fallible operations:

### Common Errors
- `IndexOutOfBounds` - Invalid row/column access
- `ColumnNotFound` - Named column doesn't exist
- `TypeMismatch` - Incompatible types in operation
- `ParseError` - Invalid syntax or format
- `IOError` - File read/write failures

### Error Patterns
```piptable
' Check for errors
dim result = sheet.get_by_name(0, "invalid")
if result.is_error() then
    print("Column not found")
end if

' Use default values
dim value = sheet.get(0, 0).unwrap_or(0)
```

## Performance Guidelines

### Memory Management
- Sheets store data row-major (Vec<Vec<CellValue>>)
- Books use IndexMap for O(1) sheet lookups
- Consolidation creates new sheets (no in-place modification)

### Optimization Tips
1. **Prefer row operations over column operations** - Row access is faster
2. **Use append_distinct/upsert for deduplication** - More efficient than manual checking
3. **Batch operations when possible** - Reduce iteration overhead
4. **Use appropriate file formats** - Parquet for large datasets, CSV for simplicity

### Async Operations
Use parallel execution for independent operations:
```piptable
dim results = parallel
    fetch(api1),
    fetch(api2),
    query("SELECT * FROM large_table")
end parallel
```

## Integration

### Python UDFs
Extend PipTable with Python functions:
```python
def custom_transform(value):
    return value.upper().strip()

register_python("transform", custom_transform)
```

### Command Line Interface
```bash
# Run a script
piptable script.pip

# Interactive REPL
piptable

# Process data pipeline
piptable transform.pip --input data.csv --output results.xlsx
```

## Version Compatibility

Current version: 0.1.0

### Breaking Changes
Track breaking API changes between versions:
- v0.1.0: Initial API release

### Deprecation Policy
- Deprecated APIs marked with warnings
- Minimum 2 version deprecation period
- Migration guides provided

## See Also

- [DSL Reference](../dsl/README.md) - Language syntax
- [Examples](../../examples/README.md) - Code examples
- [Guides](../../guide/README.md) - How-to guides