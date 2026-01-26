# Sheet Advanced Operations

This document covers advanced sheet operations added in issue #137, providing pyexcel-like functionality.

## A1-Style Cell Notation

Access and modify cells using Excel-style notation.

### Basic Cell Access

```rust
// Get cell value
let value = sheet.get_a1("A1")?;
let value = sheet.get_a1("B2")?;

// Set cell value
sheet.set_a1("A1", "New Value")?;
sheet.set_a1("C3", 42)?;

// Get mutable reference
let cell = sheet.get_a1_mut("B2")?;
*cell = CellValue::String("Modified".to_string());
```

### Range Access

```rust
// Get a sub-sheet from a range
let sub_sheet = sheet.get_range("A1:C3")?;
let sub_sheet = sheet.get_range("B2:D5")?;

// Single cell as range
let single = sheet.get_range("B2")?;
```

**Examples in piptable DSL:**

```piptable
' Future syntax (to be implemented in interpreter)
dim value = sheet["A1"]
sheet["B2"] = "New Value"
dim range = sheet["A1:C3"]
```

## Named Row and Column Access

Name rows and columns for easier access.

### Named Columns

```rust
// Use first row as column names
sheet.name_columns_by_row(0)?;

// Access by column name
let ages = sheet.column_by_name("Age")?;
let value = sheet.get_by_name(row_index, "Name")?;

// Set by column name  
sheet.set_by_name(row_index, "City", "NYC")?;

// Format column by name
sheet.format_column_by_name("Age", |cell| {
    // Convert string to integer
    match cell.as_str().parse::<i64>() {
        Ok(n) => CellValue::Int(n),
        Err(_) => cell.clone(),
    }
})?;
```

### Named Rows

```rust
// Use first column as row names
sheet.name_rows_by_column(0)?;

// Access by row name
let employee = sheet.row_by_name("Employee001")?;
```

**Example:**

```piptable
import "data.csv" into sheet

' Name columns by first row
sheet.name_columns_by_row(0)

' Access by column name
dim ages = sheet.column["Age"]
sheet.column["Salary"] = sheet.column["Salary"].map(x => x * 1.1)
```

## Bulk Operations

Apply functions to entire sheets or specific regions.

### Map Operations

```rust
// Apply function to all cells
sheet.map(|cell| {
    match cell {
        CellValue::Int(n) => CellValue::Int(n * 2),
        CellValue::String(s) => CellValue::String(s.to_uppercase()),
        v => v.clone(),
    }
});

// Consuming version (returns self)
let sheet = sheet.map_into(|cell| transform(cell));

// Map specific column
sheet.column_map(2, |cell| cell.clone())?;
sheet.column_map_by_name("Price", |cell| {
    // Add 10% markup
    match cell {
        CellValue::Float(f) => CellValue::Float(f * 1.1),
        CellValue::Int(n) => CellValue::Float(n as f64 * 1.1),
        v => v.clone(),
    }
})?;
```

### Filter Operations

```rust
// Filter rows with index
sheet.filter_rows(|idx, row| {
    // Keep rows where first column > 5
    row[0].as_int().unwrap_or(0) > 5
});

// Filter columns
sheet.filter_columns(|idx, name| {
    // Keep only Name and Age columns
    name == "Name" || name == "Age"
})?;

// Remove empty rows
sheet.remove_empty_rows();
```

### Format Operations

```rust
// Format specific column
sheet.format_column(0, |cell| {
    CellValue::String(cell.as_str().to_uppercase())
})?;

// Format column by name
sheet.format_column_by_name("Date", |cell| {
    // Parse and reformat date
    parse_date(cell.as_str())
})?;
```

## Column Selection and Manipulation

### Cherry-Pick Columns

```rust
// Keep only specified columns
sheet.select_columns(&["Name", "Age", "City"])?;

// Remove specific columns (opposite of select)
sheet.remove_columns(&["TempColumn", "Debug"])?;
```

### Column Operations

```rust
// Remove columns at specific indices
sheet.remove_columns_at(&[2, 4, 5])?;

// Append column
sheet.column_append(vec!["Header", "Value1", "Value2"])?;

// Insert column at index
sheet.column_insert(1, vec!["New", "Column", "Data"])?;
```

## Transformation Operations

### Transpose

```rust
// Swap rows and columns
sheet.transpose();
```

### Remove Empty Data

```rust
// Remove rows where all cells are null or empty
sheet.remove_empty_rows();
```

## Complete Example

```rust
use piptable_sheet::{Sheet, CellValue};

fn process_sales_data() -> Result<()> {
    // Load data
    let mut sheet = Sheet::from_csv("sales.csv")?;
    
    // Use first row as column names
    sheet.name_columns_by_row(0)?;
    
    // Remove the header row from data
    sheet.row_delete(0)?;
    
    // Convert Price column to float
    sheet.format_column_by_name("Price", |cell| {
        match cell.as_str().parse::<f64>() {
            Ok(f) => CellValue::Float(f),
            Err(_) => CellValue::Null,
        }
    })?;
    
    // Filter out rows with null prices
    sheet.filter_rows(|_, row| {
        !matches!(row[2], CellValue::Null)
    });
    
    // Apply 10% discount using A1 notation
    for i in 1..=sheet.row_count() {
        let notation = format!("C{}", i);
        if let Ok(cell) = sheet.get_a1_mut(&notation) {
            if let CellValue::Float(price) = cell {
                *cell = CellValue::Float(*price * 0.9);
            }
        }
    }
    
    // Cherry-pick final columns
    sheet.select_columns(&["Product", "Price", "Quantity"])?;
    
    // Save result
    sheet.save_as_csv("processed_sales.csv")?;
    
    Ok(())
}
```

## Performance Considerations

1. **Map operations** iterate through all cells, so they can be expensive on large sheets
2. **Filter operations** create new data structures, so memory usage may temporarily double
3. **Named access** uses HashMaps for O(1) lookups after initial setup
4. **Transpose** creates a completely new data grid
5. **A1 notation parsing** has a small overhead compared to direct index access

## Migration from pyexcel

| pyexcel | piptable |
|---------|----------|
| `sheet['A1']` | `sheet.get_a1("A1")` |
| `sheet['A1'] = value` | `sheet.set_a1("A1", value)` |
| `sheet['A1:C3']` | `sheet.get_range("A1:C3")` |
| `sheet.column["Name"]` | `sheet.column_by_name("Name")` |
| `sheet.row["Row1"]` | `sheet.row_by_name("Row1")` |
| `sheet.map(func)` | `sheet.map(func)` |
| `sheet.filter()` | `sheet.filter_rows()` |
| `del sheet.column['a', 'c']` | `sheet.remove_columns(&["a", "c"])` |
| `sheet.transpose()` | `sheet.transpose()` |

## Next Steps

Future enhancements planned:
- Region-specific operations (apply to sub-ranges)
- Arithmetic operations on sheets (concatenation with +, |)
- More sophisticated filtering with multiple conditions
- Cell validation functions
- Conditional formatting support