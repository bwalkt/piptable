# Sheet Operations - Rust API & DSL

This page demonstrates the advanced Sheet operations available through both the Rust API and the piptable DSL, implementing pyexcel-like functionality.

## DSL Sheet Operations

The piptable DSL provides direct access to Sheet operations through built-in functions and A1 notation indexing.

### A1 Notation Access

Access cells and ranges using Excel-style notation:

```piptable
// Import a CSV file as a sheet (works with Excel files too)
import sales from 'sales.csv'

// Access individual cells using A1 notation
dim header = sales["A1"]         // Get header cell
dim value = sales["B2"]          // Get data cell

// Access ranges - returns a new sheet
dim range = sales["A1:C3"]       // Get a 3x3 range
dim column = sales["A:A"]        // Get entire first column
dim row = sales["1:1"]           // Get first row

// Use in expressions
print("Product name in B1: " + sales["B1"])
```

### Sheet Built-in Functions

The DSL provides these built-in functions for sheet manipulation:

```piptable
// Sheet information
dim rows = sheet_row_count(sales)    // Get number of rows
dim cols = sheet_col_count(sales)    // Get number of columns

// Cell access by A1 notation
dim cell_value = sheet_get_a1(sales, "B2")
sales = sheet_set_a1(sales, "B2", "New Value")

// Range operations
dim subsheet = sheet_get_range(sales, "A1:C5")

// Column operations (requires named columns)
sales = sheet_name_columns_by_row(sales, 0)  // Use first row as headers
dim name_column = sheet_column_by_name(sales, "Name")
dim price = sheet_get_by_name(sales, 2, "Price")  // Row 2, Price column
sales = sheet_set_by_name(sales, 2, "Price", 29.99)

// Sheet transformations
sales = sheet_transpose(sales)                    // Transpose rows/columns
sales = sheet_select_columns(sales, ["Name", "Price", "Stock"])
sales = sheet_remove_columns(sales, ["TempCol", "DebugInfo"])
sales = sheet_remove_empty_rows(sales)
```

### Formula Evaluation

Evaluate formulas against sheet data when needed:

```piptable
// Evaluate a formula stored in a cell (e.g., "=SUM(A1:A2)")
dim computed = sheet_get_cell_value(sales, "B2")

// Evaluate a formula string directly against the sheet
dim total = sheet_eval_formula(sales, "SUM(A1:A10)")
```

In Rust, you can store formulas directly on the sheet and compute cached results:

```rust
use piptable_sheet::Sheet;

let mut sheet = Sheet::from_data(vec![vec![0, 0, 0]]);
sheet.set_a1("A1", 1)?;
sheet.set_a1("B1", 2)?;

sheet.set_formula("C1", "=SUM(A1:B1)")?;
sheet.evaluate_formulas()?;

let cell = sheet.get_a1("C1")?;
assert_eq!(cell.as_float(), Some(3.0));
```

## Formula Engine (Rust)

Use the `piptable-formulas` crate directly for programmatic formula evaluation.
Lookup functions (VLOOKUP/HLOOKUP/INDEX/MATCH/XLOOKUP/OFFSET) are formula-backed and
use the same implementation as the DSL.

```rust
use piptable_formulas::{EvalContext, FormulaEngine};
use piptable_primitives::{CellAddress, CellRange, ErrorValue, Value};
use std::collections::HashMap;

fn eval_lookup() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = FormulaEngine::new();

    // Provide a 2x2 range for A1:B2 (row-major).
    let range = CellRange::new(CellAddress::new(0, 0), CellAddress::new(1, 1));
    let values = vec![
        Value::String("Apple".to_string()),
        Value::Float(1.5),
        Value::String("Banana".to_string()),
        Value::Float(0.75),
    ];

    let mut ranges = HashMap::new();
    ranges.insert(range, values);
    let ctx = EvalContext::with_ranges(ranges);

    let compiled = engine.compile(r#"VLOOKUP("Banana", A1:B2, 2, FALSE)"#)?;
    let result = engine.evaluate(&compiled, &ctx)?;
    assert!(matches!(result, Value::Float(f) if (f - 0.75).abs() < 1e-9));

    let compiled = engine.compile(r#"XLOOKUP("App*", A1:A2, B1:B2, "N/A", 2)"#)?;
    let result = engine.evaluate(&compiled, &ctx)?;
    assert!(matches!(result, Value::Float(_)));

    let compiled = engine.compile(r#"XLOOKUP(0.8, B1:B2, A1:A2, "N/A", 1, 2)"#)?;
    let result = engine.evaluate(&compiled, &ctx)?;
    assert!(matches!(result, Value::String(_)));

    let compiled = engine.compile("OFFSET(A1:B2, 1, 0, 1, 2)")?;
    let result = engine.evaluate(&compiled, &ctx)?;
    assert!(matches!(result, Value::Array(_)));

    // Not-found returns a formula error (#N/A).
    let compiled = engine.compile(r#"VLOOKUP("Pear", A1:B2, 2, FALSE)"#)?;
    let result = engine.evaluate(&compiled, &ctx)?;
    assert!(matches!(result, Value::Error(ErrorValue::NA)));

    Ok(())
}
```

### Dependency Tracking (Rust)

The formula engine can track dependencies and compute recalculation order. For
sheet-aware workflows, use the `*_with_sheet` APIs along with a `SheetIdResolver`.

```rust
use piptable_formulas::{FormulaEngine, SheetIdResolver};
use piptable_primitives::CellAddress;

struct Resolver;
impl SheetIdResolver for Resolver {
    fn sheet_id(&self, sheet_name: &str) -> Option<u32> {
        match sheet_name {
            "Sheet1" => Some(1),
            _ => None,
        }
    }
}

fn track_dependencies() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = FormulaEngine::new();
    let a1 = CellAddress::new(0, 0);
    let b1 = CellAddress::new(0, 1);

    engine.set_formula_with_sheet(1, b1, "=Sheet1!A1+1", Some(&Resolver))?;
    engine.mark_dirty_with_sheet(1, &a1);

    let dirty = engine.get_dirty_nodes_with_sheet()?;
    assert!(dirty.iter().any(|cell| cell.sheet_id == 1 && cell.addr == b1));
    Ok(())
}
```

### SQL Integration

Sheets automatically convert to tables for SQL queries:

```piptable
// Import data as sheet
import products from 'products.csv'
import orders from 'orders.csv'

// Use sheets directly in SQL - they're automatically converted to tables
dim top_products = query(
    SELECT product_name, SUM(quantity) as total_sold
    FROM orders 
    JOIN products ON orders.product_id = products.id
    GROUP BY product_name 
    ORDER BY total_sold DESC 
    LIMIT 10
)

// Mix sheet operations with SQL
products = sheet_name_columns_by_row(products, 0)
dim expensive = query(SELECT * FROM products WHERE price > 100)
```

### Complete DSL Example

Here's a complete data processing workflow using DSL sheet operations:

```piptable
@title "Sales Analysis with DSL Sheet Operations"
@description "Process sales data using sheet functions and SQL integration"

// Import raw sales data
import raw_sales from 'monthly_sales.csv'

// Set up column headers
raw_sales = sheet_name_columns_by_row(raw_sales, 0)

// Clean up data - remove any empty rows
raw_sales = sheet_remove_empty_rows(raw_sales)

// Keep only the columns we need
raw_sales = sheet_select_columns(raw_sales, [
    "Date", "Product", "Quantity", "Price", "Customer"
])

// Add calculated total column using cell access
// Note: row 0 is headers, data starts at row 1
// DSL for loops use inclusive bounds, so "1 to row_count - 1" processes all data rows
dim row_count = sheet_row_count(raw_sales)
for i in 1 to row_count - 1 {
    dim qty = sheet_get_by_name(raw_sales, i, "Quantity")
    dim price = sheet_get_by_name(raw_sales, i, "Price")
    dim total = qty * price
    raw_sales = sheet_set_by_name(raw_sales, i, "Total", total)
}

// Now analyze with SQL
dim monthly_summary = query(
    SELECT 
        strftime('%Y-%m', "Date") as month,
        COUNT(*) as order_count,
        SUM("Total") as revenue,
        AVG("Total") as avg_order
    FROM raw_sales 
    GROUP BY month 
    ORDER BY month
)

// Export results
export monthly_summary to 'monthly_summary.csv'

// Get top customers using sheet operations
dim customer_totals = query(
    SELECT "Customer", SUM("Total") as total_spent
    FROM raw_sales 
    GROUP BY "Customer" 
    ORDER BY total_spent DESC
    LIMIT 10
)

print("Analysis complete! Top customers and monthly trends exported.")
```

## DSL Recipes

Short, copy-pasteable patterns for common sheet + formula tasks.

### Read Formula Text vs Computed Value

```piptable
// Raw cell value (formula text if stored)
dim raw = sheet_get_cell(sales, "B2")

// Computed value if formula, otherwise raw value
dim value = sheet_get_cell_value(sales, "B2")

// Check if a cell holds a formula string
dim is_formula = is_sheet_cell_formula(sales, "B2")
```

### Compute a Column With Formulas

```piptable
sales = sheet_name_columns_by_row(sales, 0)

dim rows = sheet_row_count(sales)
for i in 1 to rows - 1 {
    dim qty = sheet_get_by_name(sales, i, "Quantity")
    dim price = sheet_get_by_name(sales, i, "Price")
    dim total = qty * price
    sales = sheet_set_by_name(sales, i, "Total", total)
}
```

### Evaluate a Formula Against a Sheet

```piptable
dim total = sheet_eval_formula(sales, "SUM(A1:A10)")
```

## A1-Style Cell Access

Access and modify cells using Excel-style notation:

```rust
use piptable_sheet::{Sheet, CellValue};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut sheet = Sheet::from_csv("sales.csv")?;
    
    // Get cell value
    let value = sheet.get_a1("A1")?;
    println!("A1 = {}", value.as_str().unwrap_or(""));
    
    // Set cell value
    sheet.set_a1("B2", "Updated Value")?;
    sheet.set_a1("C3", 42)?;
    
    // Get a range as sub-sheet
    let range = sheet.get_range("A1:C3")?;
    println!("Range has {} rows", range.row_count());
    
    // Get mutable reference for in-place modification
    let cell = sheet.get_a1_mut("D4")?;
    *cell = CellValue::Float(99.99);
    
    Ok(())
}
```

## Named Column Operations

Use column names for easier data manipulation:

```rust
use piptable_sheet::{Sheet, CellValue};

fn process_employee_data() -> Result<(), Box<dyn std::error::Error>> {
    let mut sheet = Sheet::from_csv("employees.csv")?;
    
    // Use first row as column headers
    sheet.name_columns_by_row(0)?;
    
    // Access entire column by name
    let salaries = sheet.column_by_name("Salary")?;
    
    // Calculate average salary
    let total: f64 = salaries.iter()
        .skip(1)  // Skip header
        .filter_map(|cell| cell.as_float())
        .sum();
    let count = salaries.len().saturating_sub(1);
    let avg = if count > 0 { total / count as f64 } else { 0.0 };
    println!("Average salary: ${:.2}", avg);
    
    // Update specific cells by column name
    for row in 1..sheet.row_count() {
        let current = sheet.get_by_name(row, "Salary")?;
        if let Some(sal) = current.as_float() {
            // Give 10% raise
            sheet.set_by_name(row, "Salary", sal * 1.1)?;
        }
    }
    
    // Format a column
    sheet.format_column_by_name("HireDate", |cell| {
        // Standardize date format
        match cell.as_str() {
            s if s.contains("/") => {
                // Convert MM/DD/YYYY to YYYY-MM-DD
                let parts: Vec<&str> = s.split('/').collect();
                if parts.len() == 3 {
                    CellValue::String(format!("{}-{:02}-{:02}", 
                        parts[2], 
                        parts[0].parse::<u32>().unwrap_or(1),
                        parts[1].parse::<u32>().unwrap_or(1)
                    ))
                } else {
                    cell.clone()
                }
            }
            _ => cell.clone()
        }
    })?;
    
    sheet.save_as_csv("employees_processed.csv")?;
    Ok(())
}
```

## Bulk Operations

Apply transformations to entire sheets:

```rust
use piptable_sheet::{Sheet, CellValue};

fn bulk_transformations() -> Result<(), Box<dyn std::error::Error>> {
    let mut sheet = Sheet::from_csv("inventory.csv")?;
    
    // Apply function to all cells
    sheet.map(|cell| {
        match cell {
            CellValue::String(s) => CellValue::String(s.trim().to_string()),
            CellValue::Int(n) if *n < 0 => CellValue::Int(0),  // No negative inventory
            v => v.clone()
        }
    });
    
    // Filter rows based on condition
    sheet.name_columns_by_row(0)?;
    
    // Find the Stock column index
    let stock_col_idx = sheet.column_names()
        .and_then(|names| names.iter().position(|n| n == "Stock"))
        .unwrap_or(2);  // Assume Stock is in column 2 if not found
    
    sheet.filter_rows(|idx, row| {
        // Keep header and rows with stock > 0
        idx == 0 || row.get(stock_col_idx)
            .and_then(|cell| cell.as_int())
            .map(|stock| stock > 0)
            .unwrap_or(false)
    });
    
    // Remove empty rows
    sheet.remove_empty_rows();
    
    // Transpose for pivot-like view
    sheet.transpose();
    
    Ok(())
}
```

## Column Selection and Manipulation

Cherry-pick and reorganize columns:

```rust
use piptable_sheet::Sheet;

fn reorganize_data() -> Result<(), Box<dyn std::error::Error>> {
    let mut sheet = Sheet::from_excel("report.xlsx")?;
    sheet.name_columns_by_row(0)?;
    
    // Keep only specific columns in order
    sheet.select_columns(&["CustomerID", "OrderDate", "Total", "Status"])?;
    
    // Or remove unwanted columns
    // sheet.remove_columns(&["InternalNotes", "TempData", "Debug"])?;
    
    // Filter columns by condition
    sheet.filter_columns(|_idx, name| {
        // Keep only columns that don't start with underscore
        !name.starts_with('_')
    })?;
    
    Ok(())
}
```

## Complete Sales Analysis Example

A comprehensive example combining multiple operations:

```rust
use piptable_sheet::{Sheet, CellValue};
use std::collections::HashMap;

fn analyze_sales() -> Result<(), Box<dyn std::error::Error>> {
    // Load and prepare data
    let mut sales = Sheet::from_csv("sales.csv")?;
    sales.name_columns_by_row(0)?;
    
    // Clean and format data (keep header for now)
    sales.format_column_by_name("Price", |cell| {
        match cell.as_str() {
            s if s.starts_with('$') => {
                let cleaned = s.trim_start_matches('$').replace(",", "");
                cleaned.parse::<f64>()
                    .map(CellValue::Float)
                    .unwrap_or(CellValue::Null)
            }
            s => s.parse::<f64>()
                .map(CellValue::Float)
                .unwrap_or(CellValue::Null)
        }
    })?;
    
    // Calculate totals using named access (safer than A1 notation after row operations)
    for row_idx in 1..sales.row_count() {  // Skip header row
        let price = sales.get_by_name(row_idx, "Price")?;
        let qty = sales.get_by_name(row_idx, "Quantity")?;
        let total = price.as_float().unwrap_or(0.0) * qty.as_int().unwrap_or(0) as f64;
        sales.set_by_name(row_idx, "Total", total)?;
    }
    
    // Filter out invalid entries
    sales.filter_rows(|_, row| {
        row.iter().any(|cell| !matches!(cell, CellValue::Null))
    });
    
    // Extract top products
    let products_range = sales.get_range("A1:E10")?;  // Top 10 products
    
    // Group by category using named rows
    sales.name_rows_by_column(0)?;  // Use first column as row names
    
    // Export results
    sales.save_as_excel("sales_analysis.xlsx")?;
    products_range.save_as_csv("top_products.csv")?;
    
    Ok(())
}
```

## Migration from pyexcel

If you're coming from pyexcel, here's how to translate common operations:

```rust
use piptable_sheet::{Sheet, CellValue};

fn pyexcel_migration() -> Result<(), Box<dyn std::error::Error>> {
    let mut sheet = Sheet::from_csv("data.csv")?;
    
    // pyexcel: sheet['A1']
    let value = sheet.get_a1("A1")?;
    
    // pyexcel: sheet['A1'] = 'value'
    sheet.set_a1("A1", "value")?;
    
    // pyexcel: sheet['A1:C3']
    let range = sheet.get_range("A1:C3")?;
    
    // pyexcel: sheet.column['Name']
    sheet.name_columns_by_row(0)?;
    let column = sheet.column_by_name("Name")?;
    
    // pyexcel: sheet.row['Row1']
    sheet.name_rows_by_column(0)?;
    let row = sheet.row_by_name("Row1")?;
    
    // pyexcel: sheet.map(lambda x: x.upper())
    sheet.map(|cell| {
        match cell {
            CellValue::String(s) => CellValue::String(s.to_uppercase()),
            v => v.clone()
        }
    });
    
    // pyexcel: sheet.filter(condition)
    sheet.filter_rows(|idx, row| {
        // your condition here
        true
    });
    
    // pyexcel: del sheet.column['a', 'c']
    sheet.remove_columns(&["a", "c"])?;
    
    // pyexcel: sheet.transpose()
    sheet.transpose();
    
    Ok(())
}
```

## Performance Tips

1. **Use bulk operations** instead of cell-by-cell updates when possible
2. **Name columns once** at the beginning if you'll access by name multiple times
3. **Filter early** to reduce data size before expensive operations
4. **Use references** (`get_a1`) for reading, mutations (`get_a1_mut`) only when modifying
5. **Consider memory** when transposing large sheets

## When to Use Rust API vs DSL

Use the **Rust API** when you need:
- Cell-level access (A1 notation)
- Complex data transformations
- Custom formatting logic
- Performance-critical operations
- Integration with other Rust code

Use the **DSL** when you need:
- SQL-based queries
- Simple import/export
- Joins between tables
- Integration with HTTP APIs
- Quick scripting without compilation

## See Also

- [Sheet API Reference](../reference/api/sheet.md)
- [Sheet Advanced Operations](../reference/api/sheet_advanced.md)
- [DSL Integration Status](../reference/api/sheet_dsl_integration.md)
