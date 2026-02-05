# Sheet API

The Sheet API provides methods for working with 2D tabular data. A Sheet represents a single table or dataframe with rows and columns.

## Creating Sheets

### Constructors

```text
Sheet::new() -> Sheet
Sheet::with_name(name: &str) -> Sheet
Sheet::from_data<T>(data: Vec<Vec<T>>) -> Sheet
Sheet::from_records(records: Vec<IndexMap<String, CellValue>>) -> Result<Sheet>
```

**Examples:**
```piptable
' Create empty sheet
dim data = new Sheet()

' Import from file creates a sheet
dim sales = import "sales.csv" into sheet

' Query results create sheets
dim results = query("SELECT * FROM users")
```

## Basic Properties

### Information Methods

| Method | Description | Returns |
|--------|-------------|---------|
| `name()` | Get sheet name | `&str` |
| `row_count()` | Number of rows | `usize` |
| `col_count()` | Number of columns | `usize` |
| `is_empty()` | Check if empty | `bool` |
| `column_names()` | Get column headers | `Option<&Vec<String>>` |

**Examples:**
```piptable
dim rows = sheet.row_count()
dim cols = sheet.col_count()
dim headers = sheet.column_names()

if sheet.is_empty() then
    print("No data")
end if
```

## Cell Operations

### Accessing Cells

```text
get(row: usize, col: usize) -> Result<&CellValue>
get_by_name(row: usize, col_name: &str) -> Result<&CellValue>
set<T>(row: usize, col: usize, value: T) -> Result<()>
set_by_name<T>(row: usize, col_name: &str, value: T) -> Result<()>
```

**Examples:**
```piptable
' Get cell at row 0, column 2
dim value = sheet.get(0, 2)

' Get by column name
dim price = sheet.get_by_name(0, "price")

' Set cell value
sheet.set(0, 2, 42.50)
sheet.set_by_name(0, "price", 42.50)
```

## Row Operations

### Accessing Rows

```text
row(index: usize) -> Result<&Vec<CellValue>>
row_by_name(name: &str) -> Result<&Vec<CellValue>>
rows() -> Iterator<Item = &Vec<CellValue>>
rows_mut() -> Iterator<Item = &mut Vec<CellValue>>
```

### Modifying Rows

```text
row_append<T>(data: Vec<T>) -> Result<()>
row_insert<T>(index: usize, data: Vec<T>) -> Result<()>
row_update<T>(index: usize, data: Vec<T>) -> Result<()>
row_update_by_name<T>(name: &str, data: Vec<T>) -> Result<()>
row_delete(index: usize) -> Result<Vec<CellValue>>
row_delete_multi(indices: Vec<usize>) -> Result<()>
row_delete_where<F>(predicate: F) -> usize
```

**Examples:**
```piptable
' Append a new row
sheet.row_append([1, "Product", 29.99, true])

' Insert at specific position
sheet.row_insert(0, ["ID", "Name", "Price", "Active"])

' Delete row
sheet.row_delete(5)

' Delete rows matching condition  
sheet.row_delete_where(|row| row[2].as_float().unwrap_or(0.0) < 10.0)
```

## Column Operations

### Accessing Columns

```text
column(index: usize) -> Result<Vec<CellValue>>
column_by_name(name: &str) -> Result<Vec<CellValue>>
```

### Modifying Columns

```text
column_append<T>(data: Vec<T>) -> Result<()>
column_insert<T>(index: usize, data: Vec<T>, name: Option<String>) -> Result<()>
column_update<T>(index: usize, data: Vec<T>) -> Result<()>
column_update_by_name<T>(name: &str, data: Vec<T>) -> Result<()>
column_delete(index: usize) -> Result<Vec<CellValue>>
column_delete_by_name(name: &str) -> Result<Vec<CellValue>>
column_delete_multi_by_name(names: &[&str]) -> Result<()>
```

**Examples:**
```piptable
' Get column by index or name
dim prices = sheet.column(2)
dim names = sheet.column_by_name("product_name")

' Add new column
sheet.column_append([100, 200, 300, 400])

' Delete columns
sheet.column_delete_by_name("temp_column")
sheet.column_delete_multi_by_name(["col1", "col2"])
```

## Transformations

### Map Operations

```text
map<F>(f: F)  // Apply function to all cells
map_range<F>(range: &str, f: F) -> Result<()>  // Apply function to a range (A1 or R1C1)
column_map<F>(col_index: usize, f: F) -> Result<()>
column_map_by_name<F>(name: &str, f: F) -> Result<()>
```

**Examples:**
```piptable
' Transform all cells to uppercase (for string cells)
sheet.map(|cell| match cell {
    CellValue::String(s) => CellValue::String(s.to_uppercase()),
    other => other
})

' Increase prices by 10%
sheet.column_map_by_name("price", |cell| match cell {
    CellValue::Float(f) => CellValue::Float(f * 1.1),
    CellValue::Int(i) => CellValue::Float(i as f64 * 1.1),
    other => other
})
```

### Data Cleaning

```text
clean_data(options: &CleanOptions) -> Result<()>
clean_data_range(range: &str, options: &CleanOptions) -> Result<()>  // A1 or R1C1
```

**Examples:**
```text
let mut options = CleanOptions::default();
options.trim = true;
options.lower = true;
options.null_strategy = NullStrategy::EmptyToNull;

sheet.clean_data(&options)?;
sheet.clean_data_range("A2:C10", &options)?;
```

## Sheet Merge Operations

```text
append(other: &Sheet) -> Result<()>
concat_columns(other: &Sheet) -> Result<Sheet>
sheet1 + sheet2  // Append rows (Result<Sheet>)
sheet1 | sheet2  // Concatenate columns (Result<Sheet>)
```

**Notes:**
- `append` stacks rows (aligns by column names when available).
- `concat_columns` requires equal row counts; duplicate column names are suffixed.
- Operators are available in the Rust API.

**Examples (Rust):**
```text
let combined_rows = (&sheet1 + &sheet2)?;
let combined_cols = (&sheet1 | &sheet2)?;
```

### Filter Operations

```text
filter_rows<F>(predicate: F)  // Keep rows matching predicate
remove_columns_at(indices: &[usize]) -> Result<()>
```

**Examples:**
```piptable
' Keep only active products
sheet.filter_rows(|row| row[3].as_bool().unwrap_or(false))

' Remove columns by index
sheet.remove_columns_at([0, 4, 5])
```

## Join Operations

PipTable supports SQL-style joins between sheets:

```text
inner_join(other: &Sheet, key: &str) -> Result<Sheet>
inner_join_on(other: &Sheet, left_key: &str, right_key: &str) -> Result<Sheet>
left_join(other: &Sheet, key: &str) -> Result<Sheet>
left_join_on(other: &Sheet, left_key: &str, right_key: &str) -> Result<Sheet>
right_join(other: &Sheet, key: &str) -> Result<Sheet>
right_join_on(other: &Sheet, left_key: &str, right_key: &str) -> Result<Sheet>
full_join(other: &Sheet, key: &str) -> Result<Sheet>
full_join_on(other: &Sheet, left_key: &str, right_key: &str) -> Result<Sheet>
```

**Examples:**
```piptable
' Inner join on same column name
dim result = users.inner_join(orders, "user_id")

' Join with different column names
dim result = customers.inner_join_on(purchases, "id", "customer_id")

' Left join to keep all users
dim result = users.left_join(profiles, "id")

' Full outer join
dim result = products.full_join(inventory, "product_id")
```

### Join Semantics

- **Inner Join**: Returns only rows with matching keys in both sheets
- **Left Join**: Returns all rows from left sheet, with nulls for unmatched right rows
- **Right Join**: Returns all rows from right sheet, with nulls for unmatched left rows
- **Full Join**: Returns all rows from both sheets, with nulls for unmatched rows

## Append and Upsert Operations

### Append

Append adds rows from another sheet:

```text
append(other: &Sheet) -> Result<()>
append_distinct(other: &Sheet, key: &str) -> Result<()>
```

**Examples:**
```piptable
' Simple append - adds all rows
sheet1 append sheet2

' Append only unique rows based on key
users append distinct new_users on "email"
```

### Upsert

Upsert updates existing rows or inserts new ones based on a key:

```text
upsert(other: &Sheet, key: &str) -> Result<()>
```

**Examples:**
```piptable
' Update existing products, insert new ones
products upsert new_products on "sku"
```

## Export Operations

### Conversion Methods

```text
to_array() -> Vec<Vec<CellValue>>
to_dict() -> Option<IndexMap<String, Vec<CellValue>>>
to_records() -> Option<Vec<IndexMap<String, CellValue>>>
```

**Examples:**
```piptable
' Convert to array
dim array = sheet.to_array()

' Convert to dictionary (column name -> values)
dim dict = sheet.to_dict()

' Convert to records (array of row objects)
dim records = sheet.to_records()
```

## Column and Row Naming

```text
name_columns_by_row(row_index: usize) -> Result<()>
name_rows_by_column(col_index: usize) -> Result<()>
```

**Examples:**
```piptable
' Use first row as column headers
sheet.name_columns_by_row(0)

' Use first column as row names
sheet.name_rows_by_column(0)
```

## Error Handling

Sheet operations return `Result<T>` types that can contain:
- `SheetError::IndexOutOfBounds` - Invalid row/column index
- `SheetError::ColumnNotFound` - Column name doesn't exist
- `SheetError::DuplicateColumn` - Column name already exists
- `SheetError::IncompatibleShapes` - Mismatched dimensions
- `SheetError::InvalidKey` - Key column not found for join/append

## Performance Considerations

- Sheets use row-major storage (`Vec<Vec<CellValue>>`)
- Column operations may be slower than row operations
- Joins create new sheets rather than modifying in-place
- Use `append_distinct` and `upsert` for deduplication

## See Also

- [Book API](book.md) - Working with multiple sheets
- [Variables and Types](../../guide/variables-types.md) - Data types including CellValue
- [SQL Operations](../dsl/query.md) - Using SQL with sheets
