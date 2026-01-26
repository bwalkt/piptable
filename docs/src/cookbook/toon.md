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

dim data: table = import "data.toon" into sheet
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

dim employees: table = import "employees.toon" into sheet

' Type information is preserved
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

## Converting Between Formats

### CSV to TOON
```piptable
' @title Convert CSV to TOON
' @description Add type information to CSV data

dim csv_data: table = import "untyped_data.csv" into sheet

' Ensure correct types
dim typed_data: table = query(
  SELECT 
    CAST(id AS INT) as id,
    CAST(name AS TEXT) as name,
    CAST(amount AS FLOAT) as amount,
    CAST(is_active AS BOOL) as is_active,
    DATE(created_at) as created_date
  FROM csv_data
)

' Export with type information
export typed_data to "typed_data.toon"
print "Converted CSV to typed TOON format"
```

### TOON to JSON
```piptable
' @title Export TOON to JSON
' @description Convert typed TOON data to JSON

dim toon_data: table = import "source.toon" into sheet

' Type information helps with JSON conversion
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
  FROM toon_data
)

export json_ready to "output.json"
```

## Working with Complex TOON Data

### Multi-Type Processing
```piptable
' @title Process Mixed Types in TOON
' @description Handle different data types from TOON files

dim mixed_data: table = import "mixed_types.toon" into sheet

' Process different types appropriately
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
  FROM mixed_data
)

export processed to "processed_types.csv"
```

### Aggregate Typed Data
```piptable
' @title Aggregate TOON Data
' @description Perform type-aware aggregations

dim sales: table = import "sales_data.toon" into sheet

' Type information ensures correct aggregations
dim summary: table = query(
  SELECT 
    product_category,
    COUNT(*) as transaction_count,
    SUM(quantity) as total_units,
    AVG(unit_price) as avg_price,
    SUM(quantity * unit_price) as revenue,
    COUNT(DISTINCT customer_id) as unique_customers
  FROM sales
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

dim raw_data: table = import "input.toon" into sheet

' Validate and report type issues
dim validation_report: table = query(
  SELECT 
    *,
    CASE 
      WHEN TYPE(age) != 'INT' THEN 'Invalid age type'
      WHEN TYPE(salary) != 'FLOAT' THEN 'Invalid salary type'
      WHEN TYPE(active) != 'BOOL' THEN 'Invalid active type'
      ELSE 'Valid'
    END as validation_status
  FROM raw_data
)

' Separate valid and invalid records
dim valid_records: table = query(
  SELECT * FROM validation_report 
  WHERE validation_status = 'Valid'
)

dim invalid_records: table = query(
  SELECT * FROM validation_report 
  WHERE validation_status != 'Valid'
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

dim large_dataset: table = import "big_data.toon" into sheet

' Types allow efficient operations
dim optimized: table = query(
  SELECT 
    int_id,
    float_value * 1.1 as adjusted,
    bool_flag
  FROM large_dataset
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