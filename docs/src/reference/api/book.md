# Book API

The Book API provides methods for working with collections of sheets, similar to Excel workbooks or multi-sheet spreadsheets.

> Note: The DSL supports book objects (imported from multi-sheet files, PDFs, and Markdown tables) plus a set of book helpers and methods.

## Creating Books

### Constructors

```text
Book::new() -> Book
Book::with_name(name: &str) -> Book
Book::from_files<P>(paths: &[P]) -> Result<Book>
Book::from_files_with_options<P>(paths: &[P], options: LoadOptions) -> Result<Book>
Book::from_dict<T>(sheets: IndexMap<String, Vec<Vec<T>>>) -> Result<Book>
```

**Examples:**
```piptable
' Create empty book
dim workbook = book_from_dict({})

' Import multiple files into book
dim all_data = import "sales_2023.csv,sales_2024.csv" into book

' Load Excel file with multiple sheets
dim excel = import "data.xlsx" into book

' Create from a dictionary (DSL)
dim manual = book_from_dict({ "Sheet1": [[1, 2], [3, 4]] })
```

## DSL Book Functions

These helpers are available in the DSL:

| Function | Description | Example |
|----------|-------------|---------|
| `book_sheet_names(book)` | List sheet names | `book_sheet_names(excel)` |
| `book_sheet_count(book)` | Count sheets | `book_sheet_count(excel)` |
| `book_has_sheet(book, name)` | Check for a sheet | `book_has_sheet(excel, "Summary")` |
| `book_get_sheet(book, name)` | Get a sheet by name | `book_get_sheet(excel, "Data")` |
| `book_get_sheet_by_index(book, idx)` | Get a sheet by index | `book_get_sheet_by_index(excel, 0)` |
| `book_active_sheet(book)` | Get active sheet | `book_active_sheet(excel)` |
| `book_set_active_sheet(book, name)` | Set active sheet | `book_set_active_sheet(excel, "Summary")` |
| `book_add_sheet(book, name, sheet)` | Add a sheet | `book_add_sheet(excel, "Extra", data)` |
| `book_remove_sheet(book, name)` | Remove a sheet | `book_remove_sheet(excel, "Temp")` |
| `book_rename_sheet(book, old, new)` | Rename a sheet | `book_rename_sheet(excel, "Sheet1", "Main")` |
| `book_merge(book, other)` | Merge books | `book_merge(book1, book2)` |
| `book_to_dict(book)` | Convert to a dictionary | `book_to_dict(excel)` |
| `book_from_dict(map)` | Create a book from a dictionary | `book_from_dict({ "Sheet1": [[1,2]] })` |
| `book_sheets(book)` | Get all sheets as an array | `book_sheets(excel)` |
| `book_add_empty_sheet(book, name)` | Add an empty sheet | `book_add_empty_sheet(excel, "Temp")` |
| `book_consolidate(book)` | Consolidate sheets | `book_consolidate(excel)` |
| `book_consolidate_with_options(book, options)` | Consolidate with options | `book_consolidate_with_options(excel, {"add_source_column": true})` |
| `book_from_files(paths)` | Load multiple files into a book | `book_from_files(["a.csv", "b.csv"])` |
| `book_from_files_with_options(paths, options)` | Load files with options | `book_from_files_with_options(["a.csv"], {"has_headers": false})` |

Note: DSL helpers that mutate a book (add/remove/rename/merge/set_active) return a new book value.

## DSL Method Calls

Book methods can also be called directly:

```piptable
dim names = excel.sheet_names()
dim first = excel.get_sheet_by_index(0)
dim renamed = excel.rename_sheet("Sheet1", "Main")

' Indexing by name or index
dim data = excel["Data"]
dim first_sheet = excel[0]
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
| `to_dict()` | Convert to sheet-name map | `IndexMap<String, Vec<Vec<CellValue>>>` |

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

```text
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

```text
add_sheet(name: &str, sheet: Sheet) -> Result<()>
add_empty_sheet(name: &str) -> Result<&mut Sheet>
remove_sheet(name: &str) -> Result<Sheet>
rename_sheet(old_name: &str, new_name: &str) -> Result<()>
set_active_sheet(name: &str) -> Result<()>
```

**Examples:**
```piptable
' Add new sheet
book = book.add_sheet("Analysis", [["col1"], ["value"]])

' Add existing sheet
book = book.add_sheet("Import", data_sheet)

' Remove sheet
book = book.remove_sheet("Temp")

' Rename sheet
book = book.rename_sheet("Sheet1", "MainData")

' Set active sheet
book = book.set_active_sheet("Summary")
```

## Book Operations

### Merging Books

```text
merge(other: Book)
book1 + book2  // Merge books (same as merge)
```

Merges another book into this one, handling name conflicts by appending suffixes.
The `+` operator returns a new book.

**Examples:**
```piptable
' Merge two books
dim merged = book1.merge(book2)

' Merge multiple data files
dim combined = book_from_dict({})
for each file in files
    dim data = import file into book
    combined = combined.merge(data)
end for
```

### Consolidating Sheets

```text
consolidate() -> Result<Sheet>
consolidate_with_options(options: ConsolidateOptions) -> Result<Sheet>
```

Combines all sheets in a book into a single sheet by stacking rows.

**ConsolidateOptions:**
```text
ConsolidateOptions {
    add_source_column: bool,     // Add column with source sheet name (default: false)
    source_column_name: String,  // Source column name (default: "_source")
}
```

### Bulk Sheet Operations

```text
for_each_sheet<F>(&self, f: F)
for_each_sheet_mut<F>(&mut self, f: F)
try_for_each_sheet_mut<F, E>(&mut self, f: F) -> Result<(), E>
```

**Examples (DSL):**
```piptable
' Collect row counts
dim f = sheet => sheet.row_count()
dim counts = book.for_each_sheet(f)

' Clean each sheet (returns a new book)
dim clean = sheet => sheet_map(sheet, "trim")
dim cleaned = book.for_each_sheet_mut(clean)
```

### Dictionary Conversion

```text
let book_dict = IndexMap::from([
    ("Sheet1".to_string(), vec![vec![1, 2], vec![3, 4]]),
    ("Sheet2".to_string(), vec![vec![5, 6], vec![7, 8]]),
]);

let book = Book::from_dict(book_dict)?;
let roundtrip = book.to_dict();
```

**Examples:**
```piptable
' Simple consolidation
dim all_data = book_consolidate(book)

' Consolidate with source tracking
dim combined = book_consolidate_with_options(book, {
    "add_source_column": true,
    "source_column_name": "source_sheet"
})

' Create a book from a dictionary (DSL)
dim book = book_from_dict({
    "Sheet1": [["A", "B"], [1, 2]],
    "Sheet2": [["C", "D"], [3, 4]]
})
dim dict = book_to_dict(book)
```

## Iteration

```text
sheets() -> Iterator<Item = (&str, &Sheet)>
sheets_mut() -> Iterator<Item = (&str, &mut Sheet)>
```

**Examples:**
```piptable
' Process all sheets
for each name in book_sheet_names(book)
    dim sheet = book_get_sheet(book, name)
    print("Sheet: " + name + " has " + str(sheet.row_count()) + " rows")
next

' Modify all sheets into a new book
dim cleaned = book_from_dict({})
for each name in book_sheet_names(book)
    dim sheet = book_get_sheet(book, name)
    sheet = sheet_map(sheet, "trim")
    cleaned = book_add_sheet(cleaned, name, sheet)
next
```

## Loading Options

When loading multiple files into a book:

```text
FileLoadOptions {
    has_headers: bool, // Treat first row as headers (default: true)
}
```

**Examples:**
```piptable
' Load CSVs without headers
dim book = book_from_files_with_options(
    ["raw1.csv", "raw2.csv"],
    { "has_headers": false }
)
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
dim all_sales = book_consolidate_with_options(sales_book, {
    "add_source_column": true,
    "source_column_name": "month"
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
dim book = book_from_dict({})
book = book.add_sheet("products", import "products.csv" into sheet)
book = book.add_sheet("customers", import "customers.csv" into sheet)
book = book.add_sheet("orders", import "orders.csv" into sheet)

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
