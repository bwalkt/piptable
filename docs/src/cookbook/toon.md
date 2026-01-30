# TOON Processing

TOON (Typed Object-Oriented Notation) is a human-readable data format that combines the simplicity of CSV with type information. PipTable provides native support for reading and writing TOON files.

## What is TOON?

TOON is a columnar format that includes:
- Type information for each column
- Human-readable structure
- Compact representation
- Support for various data types (string, int, float, bool, null)

## Reading TOON Files

### Basic TOON Import
```piptable
' @title Read TOON File
' @description Import typed data from a TOON file

import "data.toon" into data
print "Loaded " + str(len(data)) + " typed records from TOON"
```

### TOON File Structure Example
```
# Example TOON file structure:
# name:string age:int salary:float active:bool
# 3
# Alice 30 75000.50 true
# Bob 25 65000.00 false
# Charlie 35 85000.75 true
```

### Process TOON with Type Safety
```piptable
' @title Type-Safe TOON Processing
' @description Leverage TOON's type information for safe processing

import "employees.toon" into employees

' TOON files can now be queried directly with SQL
dim high_earners: table = query(
  SELECT 
    name,
    age,
    salary,
    salary * 0.15 as bonus
  FROM employees
  WHERE salary > 70000 AND active = true
)

export high_earners to "high_earners.toon"
```

## SQL Queries on TOON Files

### Direct File Queries
```piptable
' @title Query TOON Files with SQL
' @description TOON files can be referenced directly in SQL FROM clauses

' Query a TOON file directly by filename
dim results: table = query(
  SELECT * FROM "products.toon"
  WHERE price > 20.00
  ORDER BY name
)
```

### Joining TOON with Other Formats
```piptable
' @title Cross-Format SQL Joins
' @description Join TOON files with CSV, Excel, or other formats

' Join TOON product data with CSV sales data
dim sales_analysis: table = query(
  SELECT 
    p.name as product_name,
    p.price,
    s.quantity,
    p.price * s.quantity as total_value
  FROM "products.toon" as p
  JOIN "sales.csv" as s ON p.id = s.product_id
  ORDER BY total_value DESC
)
```

### Variable-Based Queries
```piptable
' @title Query TOON Variables
' @description In-memory TOON data can be queried directly

import "inventory.toon" into inventory

' Query the imported variable directly
dim low_stock: table = query(
  SELECT name, quantity, reorder_point
  FROM inventory
  WHERE quantity < reorder_point
  ORDER BY quantity ASC
)

print "Found " + str(len(low_stock)) + " items below reorder point"
```

## Converting Between Formats

### CSV to TOON
```piptable
' @title Convert CSV to TOON
' @description Add type information to CSV data

import "untyped_data.csv" into csv_data

' Ensure correct types using file reference
dim typed_data: table = query(
  SELECT 
    CAST(id AS INT) as id,
    CAST(name AS TEXT) as name,
    CAST(amount AS FLOAT) as amount,
    CAST(is_active AS BOOL) as is_active,
    DATE(created_at) as created_date
  FROM "untyped_data.csv"
)

' Export with type information
export typed_data to "typed_data.toon"
print "Converted CSV to typed TOON format"
```

## WASM Formula Boundary

### Compile + Evaluate Formulas (JS + TOON)
```javascript
import {
  wasmCompileMany,
  wasmEvalMany,
  createSheetPayloadWithOptions,
  convertFromToonValue
} from "./spreadsheet-helpers.js";

const compileReq = {
  formulas: [
    { kind: "text", f: "=A1+B1" },
    { kind: "text", f: "=SUM(A1:B1)" }
  ]
};

const compiled = await wasmCompileMany(compileReq);

const sheet = createSheetPayloadWithOptions(
  [[1, 2]],
  0,
  0,
  { autoSparse: true }
);

const evalReq = { compiled: compiled.compiled, sheet };
const evalResp = await wasmEvalMany(evalReq);

const values = evalResp.results.map(convertFromToonValue);
console.log(values); // [3, 3]
```

### Sparse Sheet Payload (Manual)
```javascript
const sparseSheet = {
  range: { s: { r: 0, c: 0 }, e: { r: 0, c: 1 } },
  items: [
    { r: 0, c: 0, v: { t: "int", v: 10 } },
    { r: 0, c: 1, v: { t: "int", v: 5 } }
  ]
};
```
### TOON to JSON
```piptable
' @title Export TOON to JSON
' @description Convert typed TOON data to JSON

import "source.toon" into toon_data
export toon_data to "temp_source_toon.csv"

' Type information helps with JSON conversion - use file reference
dim json_ready: table = query(
  SELECT 
    JSON_OBJECT(
      'id', id,
      'name', name,
      'metadata', JSON_OBJECT(
        'age', age,
        'salary', salary,
        'active', active
      )
    ) as json_record
  FROM "temp_source_toon.csv"
)

export json_ready to "output.json"
```

## Working with Complex TOON Data

### Multi-Type Processing
```piptable
' @title Process Mixed Types in TOON
' @description Handle different data types from TOON files

import "mixed_types.toon" into mixed_data
export mixed_data to "temp_mixed_types.csv"

' Process different types appropriately using file reference
dim processed: table = query(
  SELECT 
    string_col,
    int_col * 2 as doubled_int,
    ROUND(float_col, 2) as rounded_float,
    CASE 
      WHEN bool_col = true THEN 'Active'
      ELSE 'Inactive'
    END as status,
    COALESCE(nullable_col, 'N/A') as non_null
  FROM "temp_mixed_types.csv"
)

export processed to "processed_types.csv"
```

### Aggregate Typed Data
```piptable
' @title Aggregate TOON Data
' @description Perform type-aware aggregations

import "sales_data.toon" into sales
export sales to "temp_sales_data.csv"

' Type information ensures correct aggregations - use file reference
dim summary: table = query(
  SELECT 
    product_category,
    COUNT(*) as transaction_count,
    SUM(quantity) as total_units,
    AVG(unit_price) as avg_price,
    SUM(quantity * unit_price) as revenue,
    COUNT(DISTINCT customer_id) as unique_customers
  FROM "temp_sales_data.csv"
  GROUP BY product_category
  ORDER BY revenue DESC
)

export summary to "sales_summary.toon"
```

## TOON Schema Validation

### Validate Data Types
```piptable
' @title TOON Schema Validation
' @description Ensure data conforms to expected types

import "input.toon" into raw_data
export raw_data to "temp_input.csv"

' Validate and report issues (simplified without TYPE function)
dim validation_report: table = query(
  SELECT 
    *,
    CASE 
      WHEN age IS NULL THEN 'Missing age'
      WHEN salary IS NULL THEN 'Missing salary'
      WHEN active IS NULL THEN 'Missing active'
      ELSE 'Valid'
    END as validation_status
  FROM "temp_input.csv"
)

' Separate valid and invalid records
dim valid_records: table = query(
  SELECT * FROM "temp_input.csv"
  WHERE age IS NOT NULL 
    AND salary IS NOT NULL 
    AND active IS NOT NULL
)

dim invalid_records: table = query(
  SELECT * FROM "temp_input.csv"
  WHERE age IS NULL 
    OR salary IS NULL 
    OR active IS NULL
)

export valid_records to "valid_data.toon"
export invalid_records to "type_errors.csv"

print "Validation complete: " + str(len(valid_records)) + " valid, " + str(len(invalid_records)) + " invalid"
```

## Performance Benefits

### Type-Optimized Queries
```piptable
' @title Leverage TOON Types for Performance
' @description Type information enables query optimization

import "big_data.toon" into large_dataset
export large_dataset to "temp_big_data.csv"

' Types allow efficient operations - use file reference
dim optimized: table = query(
  SELECT 
    int_id,
    float_value * 1.1 as adjusted,
    bool_flag
  FROM "temp_big_data.csv"
  WHERE int_id > 1000000  -- Integer comparison is fast
    AND bool_flag = true   -- Boolean check is optimized
    AND float_value BETWEEN 100.0 AND 500.0
)

export optimized to "filtered.toon"
```

## TOON Best Practices

1. **Preserve Types** - Keep type information when converting between formats
2. **Validate Early** - Check types at import to catch issues
3. **Use for Exchange** - TOON is ideal for data exchange between systems
4. **Leverage Types** - Use type information for optimization
5. **Document Schema** - Include comments about expected types

## Format Comparison

| Feature | CSV | JSON | TOON |
|---------|-----|------|------|
| Human Readable | ✅ | ✅ | ✅ |
| Type Information | ❌ | Implicit | ✅ Explicit |
| Compact | ✅ | ❌ | ✅ |
| Schema | ❌ | ❌ | ✅ |
| Performance | Good | Fair | Excellent |

## Next Steps

- [CSV Operations](csv.md) - Working with untyped CSV data
- [JSON Transformation](json.md) - Processing JSON structures
- [Data Validation](data-processing.md#data-validation) - Data quality checks
