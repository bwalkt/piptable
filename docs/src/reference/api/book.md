# Book API

The Book API provides methods for working with collections of sheets, similar to Excel workbooks or multi-sheet spreadsheets.

## Creating Books

### Constructors

```rust
Book::new() -> Book
Book::with_name(name: &str) -> Book
Book::from_files<P>(paths: &[P]) -> Result<Book>
Book::from_files_with_options<P>(paths: &[P], options: LoadOptions) -> Result<Book>
```

**Examples:**
```piptable
' Create empty book
dim workbook = new Book()

' Import multiple files into book
dim all_data = import "sales_2023.csv,sales_2024.csv" into book

' Load Excel file with multiple sheets
dim excel = import "data.xlsx" into book
```

## Basic Properties

### Information Methods

| Method | Description | Returns |
|--------|-------------|---------|
| `name()` | Get book name | `&str` |
| `sheet_count()` | Number of sheets | `usize` |
| `is_empty()` | Check if empty | `bool` |
| `sheet_names()` | List of sheet names | `Vec<&str>` |
| `has_sheet(name)` | Check sheet exists | `bool` |

**Examples:**
```piptable
dim count = book.sheet_count()
dim names = book.sheet_names()

if book.has_sheet("Summary") then
    print("Summary sheet found")
end if
```

## Sheet Management

### Accessing Sheets

```rust
get_sheet(name: &str) -> Result<&Sheet>
get_sheet_mut(name: &str) -> Result<&mut Sheet>
get_sheet_by_index(index: usize) -> Result<&Sheet>
get_sheet_by_index_mut(index: usize) -> Result<&mut Sheet>
active_sheet() -> Option<&Sheet>
active_sheet_mut() -> Option<&mut Sheet>
```

**Examples:**
```piptable
' Get sheet by name
dim sales = book.get_sheet("Sales")

' Get sheet by index (0-based)
dim first = book.get_sheet_by_index(0)

' Get active sheet
dim current = book.active_sheet()
```

### Adding and Removing Sheets

```rust
add_sheet(name: &str, sheet: Sheet) -> Result<()>
add_empty_sheet(name: &str) -> Result<&mut Sheet>
remove_sheet(name: &str) -> Result<Sheet>
rename_sheet(old_name: &str, new_name: &str) -> Result<()>
set_active_sheet(name: &str) -> Result<()>
```

**Examples:**
```piptable
' Add new empty sheet
dim new_sheet = book.add_empty_sheet("Analysis")

' Add existing sheet
book.add_sheet("Import", data_sheet)

' Remove sheet
dim removed = book.remove_sheet("Temp")

' Rename sheet
book.rename_sheet("Sheet1", "MainData")

' Set active sheet
book.set_active_sheet("Summary")
```

## Book Operations

### Merging Books

```rust
merge(other: Book)
```

Merges another book into this one, handling name conflicts by appending suffixes.

**Examples:**
```piptable
' Merge two books
book1.merge(book2)

' Merge multiple data files
dim combined = new Book()
for each file in files
    dim data = import file into book
    combined.merge(data)
end for
```

### Consolidating Sheets

```rust
consolidate() -> Result<Sheet>
consolidate_with_options(options: ConsolidateOptions) -> Result<Sheet>
```

Combines all sheets in a book into a single sheet by stacking rows.

**ConsolidateOptions:**
```rust
ConsolidateOptions {
    headers: bool,              // First row contains headers (default: true)
    source_column: Option<String>, // Add column with source sheet name
}
```

**Examples:**
```piptable
' Simple consolidation
dim all_data = book.consolidate()

' Consolidate with source tracking
dim options = ConsolidateOptions::new()
    .with_source_column("source_sheet")
    .with_headers(true)
    
dim combined = book.consolidate_with_options(options)
```

## Iteration

```rust
sheets() -> Iterator<Item = (&str, &Sheet)>
sheets_mut() -> Iterator<Item = (&str, &mut Sheet)>
```

**Examples:**
```piptable
' Process all sheets
for each (name, sheet) in book.sheets()
    print("Sheet: " + name + " has " + sheet.row_count() + " rows")
end for

' Modify all sheets
for each (name, sheet) in book.sheets_mut()
    sheet.column_append(["processed"])
end for
```

## Loading Options

When loading multiple files into a book:

```rust
LoadOptions {
    headers: bool,           // Treat first row as headers
    delimiter: Option<char>, // CSV delimiter
    sheet_name_from: enum {  // How to name sheets
        FileName,           // Use file name
        FirstColumn,        // Use first column value
        Custom(String),     // Use custom name
    }
}
```

**Examples:**
```piptable
' Load CSVs with custom delimiter
dim options = LoadOptions::new()
    .with_delimiter('|')
    .with_headers(true)

dim book = Book::from_files_with_options(files, options)
```

## Common Patterns

### Pattern: Monthly Data Files

```piptable
' Load monthly sales files
dim files = [
    "sales_jan_2024.csv",
    "sales_feb_2024.csv", 
    "sales_mar_2024.csv"
]

dim sales_book = import files into book

' Consolidate into single sheet with source column
dim all_sales = sales_book.consolidate_with_options({
    "source_column": "month"
})

' Analyze consolidated data
dim summary = query("
    SELECT month, SUM(amount) as total
    FROM all_sales
    GROUP BY month
")
```

### Pattern: Excel Multi-Sheet Processing

```piptable
' Load Excel file with multiple sheets
dim workbook = import "quarterly_report.xlsx" into book

' Process each sheet
for each (name, sheet) in workbook.sheets_mut()
    ' Calculate totals for new column
    dim quantity_col = sheet.column_by_name("quantity")
    dim price_col = sheet.column_by_name("price")
    dim totals_col = []
    for i = 0 to len(quantity_col) - 1
        totals_col.push(quantity_col[i] * price_col[i])
    end for
    sheet.column_append(totals_col)
    
    ' Add summary row
    dim summary = [
        "TOTAL",
        sum(quantity_col),
        avg(price_col),
        sum(totals_col)
    ]
    sheet.row_append(summary)
end for

' Export modified workbook
export workbook to "quarterly_report_processed.xlsx"
```

### Pattern: Data Validation Across Sheets

```piptable
' Load reference data and transactions
dim book = new Book()
book.add_sheet("products", import "products.csv" into sheet)
book.add_sheet("customers", import "customers.csv" into sheet)
book.add_sheet("orders", import "orders.csv" into sheet)

' Validate foreign keys
dim validation = query("
    SELECT o.*
    FROM book.orders o
    LEFT JOIN book.products p ON o.product_id = p.id
    LEFT JOIN book.customers c ON o.customer_id = c.id
    WHERE p.id IS NULL OR c.id IS NULL
")

if validation.row_count() > 0 then
    print("Found " + validation.row_count() + " invalid orders")
    export validation to "invalid_orders.csv"
end if
```

## Error Handling

Book operations return `Result<T>` types that can contain:
- `BookError::SheetNotFound` - Sheet name doesn't exist
- `BookError::DuplicateSheet` - Sheet name already exists
- `BookError::EmptyBook` - Operation requires non-empty book
- `BookError::InvalidIndex` - Sheet index out of bounds

## Performance Considerations

- Books store sheets in insertion order using `IndexMap`
- Sheet lookups by name are O(1)
- Consolidation creates a new sheet, original sheets unchanged
- Merging modifies the target book in-place

## See Also

- [Sheet API](sheet.md) - Working with individual sheets
- [File Formats](formats.md) - Supported import/export formats