# Sheet Operations - DSL Integration Status

## Current State

The new Sheet operations implemented in issue #137 are currently **Rust API only** and not yet exposed to the piptable DSL.

### What's Available Now

In Rust code, you can use:
```rust
// A1-style notation
let value = sheet.get_a1("A1")?;
sheet.set_a1("B2", "value")?;
let range = sheet.get_range("A1:C3")?;

// Named access
sheet.name_columns_by_row(0)?;
let col = sheet.column_by_name("Age")?;
sheet.set_by_name(row, "Name", value)?;

// Bulk operations
sheet.map(|cell| transform(cell));
sheet.filter_rows(|i, row| condition);
sheet.transpose();
```

### Current DSL Limitations

In the piptable DSL, sheets are automatically converted to Tables (Arrow RecordBatches) for SQL compatibility. The DSL currently:

1. **Does NOT support** direct cell access like `sheet["A1"]`
2. **Does NOT expose** Sheet methods like `map`, `filter_rows`, `transpose`
3. **DOES support** SQL queries on imported data
4. **DOES support** basic import/export operations

### Why These Limitations Exist

1. **Type System**: The DSL's `Value` enum includes `Table` (Arrow format) but not `Sheet`
2. **SQL Integration**: Tables are converted to Arrow format for SQL engine compatibility
3. **Design Philosophy**: DSL focuses on SQL-based transformations rather than cell-level operations

## Workarounds in Current DSL

### 1. Use SQL for Transformations
```piptable
import "data.csv" into data
dim transformed = query "
    SELECT 
        UPPER(name) as name,
        age * 1.1 as adjusted_age
    FROM data
"
export transformed to "output.csv"
```

### 2. Use Python Integration
```piptable
' Requires python feature
dim df = python("
import pandas as pd
df = pd.read_csv('data.csv')
df['A1'] = 'New Value'  # Cell-like access
df.iloc[0, 1] = 42       # Position access
df
")
```

### 3. Process Multiple Sheets
```piptable
' Load Excel with specific sheet
import "data.xlsx" sheet "Sheet1" into sheet1
import "data.xlsx" sheet "Sheet2" into sheet2

' Join sheets
dim combined = sheet1 join sheet2 on "id"
```

## What Would Be Needed for Full DSL Support

To expose the new Sheet operations to DSL users, we would need:

### 1. Extend Value Enum
```rust
pub enum Value {
    // ... existing variants ...
    Sheet(Sheet),  // New variant
}
```

### 2. Add ArrayIndex Support for Sheets
```rust
// In interpreter's eval_expr for ArrayIndex
(Value::Sheet(sheet), Value::String(notation)) => {
    // Handle A1 notation: sheet["A1"]
    sheet.get_a1(&notation)
        .map(|cell| cell_to_value(cell))
}
```

### 3. Add Sheet Methods as Functions
```piptable
' Proposed syntax
dim data = import("data.csv")
data.name_columns_by_row(0)
dim ages = data.column_by_name("Age")
data.set_a1("B2", "New Value")
data.transpose()
```

### 4. Maintain SQL Compatibility
- Auto-convert Sheet to Table when used in SQL queries
- Preserve backward compatibility with existing scripts

## Recommendation

For now, users should:

1. **Use Rust API** for complex sheet manipulations requiring cell-level access
2. **Use SQL in DSL** for data transformations 
3. **Use Python integration** if cell-level access is critical in DSL
4. **Wait for future DSL enhancements** tracked in a separate issue

## Future Enhancement Issue

DSL integration of Sheet operations is tracked in issue #138, which covers:

- Design decisions on syntax
- Performance implications
- Backward compatibility strategy
- Implementation roadmap