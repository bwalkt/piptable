# Join Operations

Join operations in PipTable allow you to combine data from multiple sheets (tables) based on common columns. The language supports all standard SQL join types with a natural, VBA-like syntax.

## Overview

PipTable provides four types of joins:

- **Inner Join**: Returns only matching rows from both sheets
- **Left Join**: Returns all rows from the left sheet, plus matching rows from the right
- **Right Join**: Returns all rows from the right sheet, plus matching rows from the left  
- **Full Join**: Returns all rows from both sheets

## Basic Syntax

### Same Column Join

When joining on columns with the same name:

```vba
result = leftSheet join rightSheet on "columnName"
result = leftSheet left join rightSheet on "columnName"
result = leftSheet right join rightSheet on "columnName"
result = leftSheet full join rightSheet on "columnName"
```

### Different Column Join

When joining on columns with different names:

```vba
result = leftSheet join rightSheet on "leftColumn" = "rightColumn"
result = leftSheet left join rightSheet on "leftColumn" = "rightColumn"
```

## Join Types

### Inner Join

Returns only rows where the join condition matches in both sheets.

```vba
dim employees = import "employees.csv" into sheet
dim departments = import "departments.csv" into sheet

' Only employees with valid department IDs
dim result = employees join departments on "dept_id"
```

**Result**: Only employees who have a corresponding department record.

### Left Join (Left Outer Join)

Returns all rows from the left sheet, with matching rows from the right sheet. Missing matches are filled with null values.

```vba
' All employees, with department info where available
dim result = employees left join departments on "dept_id"
```

**Result**: All employees, including those without departments (department fields will be null).

### Right Join (Right Outer Join)

Returns all rows from the right sheet, with matching rows from the left sheet. Missing matches are filled with null values.

```vba
' All departments, with employee info where available
dim result = employees right join departments on "dept_id"
```

**Result**: All departments, including those with no employees (employee fields will be null).

### Full Join (Full Outer Join)

Returns all rows from both sheets, filling missing matches with null values.

```vba
' Complete picture: all employees and all departments
dim result = employees full join departments on "dept_id"
```

**Result**: Every employee and every department, with nulls where there's no match.

## Column Handling

### Automatic Column Management

PipTable automatically handles column conflicts:

```vba
' If both sheets have "name" column
dim result = customers join orders on "customer_id"
' Result columns: customer_id, name, email, name_right, amount, date
```

Duplicate column names from the right sheet get a `_right` suffix.

### Header Detection

PipTable automatically detects and handles header rows:

```vba
' Works with CSV files that have headers
dim sales = import "sales.csv" into sheet
dim products = import "products.csv" into sheet
dim result = sales join products on "product_id"
```

## Advanced Examples

### Different Column Names

Join on columns with different names:

```vba
dim users = import "users.csv" into sheet
dim profiles = import "profiles.csv" into sheet

' Join users.id with profiles.user_id
dim result = users join profiles on "id" = "user_id"
```

### Chained Joins

Combine data from multiple sheets:

```vba
dim orders = import "orders.csv" into sheet
dim customers = import "customers.csv" into sheet  
dim products = import "products.csv" into sheet

' Join orders with both customers and products
dim result = (orders join customers on "customer_id") join products on "product_id"
```

### Join with Variables

Use variables in join conditions:

```vba
dim leftKey = "employee_id"
dim rightKey = "emp_id"
dim result = employees join timesheets on leftKey = rightKey
```

### Combining with SQL Queries

Use joins within SQL query expressions:

```vba
dim orderDetails = query("
    SELECT oc.order_id, c.name as customer_name, oc.total
    FROM (orders join customers on customer_id) as oc
    WHERE oc.total > 1000
")
```

## Performance Considerations

### Indexing Strategy

- Joins are implemented using hash-based algorithms
- Performance is generally O(n + m) for the join operation
- Ensure join columns contain consistent data types

### Memory Usage

- Large datasets may require significant memory for hash table construction
- Consider filtering data before joining when possible

```vba
' Filter before joining for better performance
dim recentOrders = query("SELECT * FROM orders WHERE date > '2023-01-01'")
dim result = recentOrders join customers on "customer_id"
```

## Error Handling

### Common Errors

1. **Missing Join Column**: Specified column doesn't exist in one or both sheets
2. **Empty Column Values**: Join columns contain null/empty values
3. **Type Mismatches**: Join columns have incompatible data types

### Validation Examples

```vba
' Check if column exists before joining
dim employees = import "employees.csv" into sheet
dim _ = sheet_column_by_name(employees, "department_id") ' Errors if column missing
dim result = employees join departments on "department_id"
```

## Best Practices

### 1. Use Meaningful Variable Names

```vba
' Good: descriptive names
dim employeeWithDepartments = employees left join departments on "dept_id"

' Avoid: generic names
dim result = employees left join departments on "dept_id"
```

### 2. Validate Data Before Joining

```vba
' Check for required columns (errors if missing)
dim _ = sheet_column_by_name(orders, "customer_id")
dim orderDetails = orders join customers on "customer_id"
```

### 3. Handle Null Values

```vba
' Clean data before joining
dim cleanOrders = query("SELECT * FROM orders WHERE customer_id IS NOT NULL")
dim result = cleanOrders join customers on "customer_id"
```

### 4. Use Parentheses for Complex Joins

```vba
' Clear precedence with parentheses
dim result = (orders join customers on "customer_id") join products on "product_id"
```

### 5. Filter Early for Performance

```vba
' Filter before joining when possible
dim activeCustomers = query("SELECT * FROM customers WHERE status = 'active'")
dim result = orders join activeCustomers on "customer_id"
```

## Integration Examples

### CSV File Joins

```vba
' Join multiple CSV files
dim sales = import "2023_sales.csv" into sheet
dim territories = import "territories.csv" into sheet
dim salesWithTerritory = sales left join territories on "region_id"

export salesWithTerritory to "sales_analysis.xlsx"
```

### Excel Sheet Joins

```vba
' Join Excel sheets
dim workbook = import "quarterly_data.xlsx" into book
dim q1Data = getSheet(workbook, "Q1")
dim q2Data = getSheet(workbook, "Q2")

dim combinedQuarters = q1Data full join q2Data on "product_id"
```

### API Data Joins

```vba
' Join with data from HTTP API
dim localUsers = import "users.csv" into sheet
dim apiData = fetch("https://api.example.com/user-profiles")
dim userProfiles = parse(apiData) into sheet

dim enrichedUsers = localUsers left join userProfiles on "user_id"
```

## Comparison with SQL

PipTable join syntax maps directly to SQL:

| PipTable               | SQL                                   |
| ---------------------- | ------------------------------------- |
| `a join b on "id"`     | `a INNER JOIN b ON a.id = b.id`       |
| `a left join b on "id"`| `a LEFT JOIN b ON a.id = b.id`        |
| `a right join b on "id"`| `a RIGHT JOIN b ON a.id = b.id`      |
| `a full join b on "id"`| `a FULL OUTER JOIN b ON a.id = b.id`  |
| `a join b on "x" = "y"`| `a INNER JOIN b ON a.x = b.y`         |

This makes it easy to understand join behavior if you're familiar with SQL, while providing a more readable syntax for those coming from VBA or similar backgrounds.

## See Also

- [Query Expressions](query.md) - Using joins within SQL queries
- [Data Structures](../../guide/data-structures.md) - Understanding sheets and tables
- [Import/Export](import-export.md) - Loading data for joins
- [Append/Upsert](append-upsert.md) - Other data combination operations
