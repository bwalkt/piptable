# Working with Joins

This guide provides a comprehensive overview of using join operations in PipTable, from basic concepts to advanced patterns. Joins are one of the most powerful features for combining data from multiple sources.

## What are Joins?

Joins allow you to combine data from two or more tables (sheets) based on related columns. Think of it as matching records from different data sources to create a unified view.

### Real-World Analogy

Imagine you have two filing cabinets:
- **Cabinet A**: Employee records (name, ID, department code)  
- **Cabinet B**: Department information (code, name, budget)

A join operation lets you create a combined report showing employee names with their full department information by matching the department codes.

## Basic Join Concepts

### Join Types Overview

| Join Type | Description | Use Case |
|-----------|-------------|-----------|
| **Inner** | Only matching records | Complete data |
| **Left** | All from left + matches from right | Preserve all primary records |
| **Right** | All from right + matches from left | Rare, usually use left instead |
| **Full** | All records from both sides | Complete picture with gaps |

### Simple Example

```vba
' Load your data
dim employees = import "employees.csv" into sheet
dim departments = import "departments.csv" into sheet

' Basic inner join - only employees with valid departments
dim result = employees join departments on "dept_id"

' View the results  
print result
export result to "employee_report.xlsx"
```

## Getting Started with Joins

### Step 1: Understand Your Data

Before joining, examine your data structure:

```vba
' Check what columns are available
dim employees = import "employees.csv" into sheet
print "Employee columns:"
print sheet_col_count(employees)

dim departments = import "departments.csv" into sheet  
print "Department columns:"
print sheet_col_count(departments)
```

### Step 2: Identify Join Keys

Look for columns that connect your tables:

```vba
' Common join key patterns:
' - ID fields: customer_id, product_id, order_id
' - Reference codes: dept_code, category_code  
' - Natural keys: email, phone, sku
```

### Step 3: Choose the Right Join Type

Ask yourself:
- Do I need all records from the primary table? → Use **left join**
- Do I only want complete matches? → Use **inner join**  
- Do I need to see all data from both sources? → Use **full join**

### Step 4: Test with Sample Data

```vba
' Always test joins with a small sample first
dim sampleEmployees = query("SELECT * FROM employees LIMIT 10")
dim testResult = sampleEmployees join departments on "dept_id"
print "Sample result has " + str(sheet_row_count(testResult)) + " rows"
```

## Common Patterns

### Master-Detail Relationships

**Pattern**: One master record relates to many detail records.

```vba
' Customers (master) and Orders (detail)
dim customers = import "customers.csv" into sheet
dim orders = import "orders.csv" into sheet

' Get all customers with their order information
dim customerOrders = customers left join orders on "customer_id" = "id"

' Summary: customers with order counts
dim customerSummary = query("
    SELECT 
        customer_name,
        customer_email,
        COUNT(order_id) as order_count,
        COALESCE(SUM(order_total), 0) as total_spent
    FROM customerOrders 
    GROUP BY customer_id, customer_name, customer_email
")
```

### Reference Data Lookups

**Pattern**: Enrich transactional data with reference information.

```vba
' Sales transactions with product details
dim sales = import "daily_sales.csv" into sheet
dim products = import "product_catalog.csv" into sheet

' Enrich sales with product information
dim salesWithProducts = sales join products on "product_sku" = "sku"

' Add category analysis
dim categories = import "categories.csv" into sheet
dim fullSales = salesWithProducts join categories on "category_id" = "id"
```

### Data Validation and Quality

**Pattern**: Identify data quality issues through joins.

```vba
' Find orphaned records
dim orders = import "orders.csv" into sheet
dim customers = import "customers.csv" into sheet

' Orders without valid customers
dim orphanOrders = orders left join customers on "customer_id" = "id"
dim badOrders = query("
    SELECT order_id, customer_id, order_date, order_total
    FROM orphanOrders 
    WHERE customer_name IS NULL
")

if sheet_row_count(badOrders) > 0 then
    print "Found " + str(sheet_row_count(badOrders)) + " orders with invalid customer IDs"
    export badOrders to "data_quality_issues.csv"
else
    print "All orders have valid customer references"
end if
```

## Advanced Techniques

### Multi-Table Joins

Combining data from multiple sources:

```vba
' E-commerce order analysis
dim orders = import "orders.csv" into sheet
dim customers = import "customers.csv" into sheet
dim products = import "products.csv" into sheet
dim orderItems = import "order_items.csv" into sheet

' Build comprehensive view step by step
dim step1 = orders join customers on "customer_id" = "id"
dim step2 = orderItems join products on "product_id" = "id"  
dim orderAnalysis = step1 join step2 on "order_id"

' Or use parentheses to control precedence
dim directJoin = (orders join customers on "customer_id" = "id") 
                join (orderItems join products on "product_id" = "id") on "order_id"
```

### Conditional Join Logic

Different join strategies based on data characteristics:

```vba
function smartJoin(leftSheet, rightSheet, joinKey)
    ' Check data quality first
    dim leftNullCount = query("SELECT COUNT(*) FROM leftSheet WHERE " + joinKey + " IS NULL")
    dim rightNullCount = query("SELECT COUNT(*) FROM rightSheet WHERE " + joinKey + " IS NULL")
    
    if leftNullCount > 0 or rightNullCount > 0 then
        print "Warning: Found null values in join keys"
        ' Clean data before joining
        dim cleanLeft = query("SELECT * FROM leftSheet WHERE " + joinKey + " IS NOT NULL")
        dim cleanRight = query("SELECT * FROM rightSheet WHERE " + joinKey + " IS NOT NULL")
        return cleanLeft join cleanRight on joinKey
    else
        ' Data is clean, proceed with normal join
        return leftSheet join rightSheet on joinKey
    end if
end function
```

### Performance Optimization

```vba
' Bad: Join large tables then filter
dim allSales = import "annual_sales.csv" into sheet    ' 1M rows
dim allCustomers = import "customers.csv" into sheet   ' 100K rows
dim bigJoin = allSales join allCustomers on "customer_id" = "id"
dim filtered = query("SELECT * FROM bigJoin WHERE region = 'West'")

' Good: Filter first, then join
dim westCustomers = query("SELECT * FROM allCustomers WHERE region = 'West'")
dim optimized = allSales join westCustomers on "customer_id" = "id"

' Even better: Filter both sides
dim recentSales = query("SELECT * FROM allSales WHERE sale_date >= '2023-01-01'")
dim activeCustomers = query("SELECT * FROM allCustomers WHERE status = 'active'")
dim efficient = recentSales join activeCustomers on "customer_id" = "id"
```

## Troubleshooting Common Issues

### Problem: No Results from Inner Join

**Symptom**: Expected matches but get empty result.

```vba
' Debug step by step
dim employees = import "employees.csv" into sheet
dim departments = import "departments.csv" into sheet

' Check data in both tables
print "Employee dept_ids:"
print query("SELECT DISTINCT dept_id FROM employees ORDER BY dept_id")

print "Department ids:"  
print query("SELECT DISTINCT id FROM departments ORDER BY id")

' Look for type mismatches
print "Employee dept_id types:"
print query("SELECT dept_id, typeof(dept_id) FROM employees LIMIT 5")

print "Department id types:"
print query("SELECT id, typeof(id) FROM departments LIMIT 5")
```

**Solutions**:
- Check for data type mismatches (string vs. number)
- Look for extra spaces or formatting differences
- Verify column names are correct

### Problem: Too Many Results

**Symptom**: Join produces more rows than expected.

```vba
' Check for duplicates in join keys
dim employees = import "employees.csv" into sheet
dim duplicateCheck = query("
    SELECT dept_id, COUNT(*) as count
    FROM employees 
    GROUP BY dept_id 
    HAVING count > 1
")

if sheet_row_count(duplicateCheck) > 0 then
    print "Found duplicate dept_ids in employees table"
    print duplicateCheck
end if
```

**Solutions**:
- Remove duplicates before joining
- Use GROUP BY to aggregate duplicate records
- Consider if you need a many-to-many relationship

### Problem: Missing Data After Join

**Symptom**: Expected data disappears after join.

```vba
' Compare row counts
dim employees = import "employees.csv" into sheet
dim departments = import "departments.csv" into sheet

print "Employees before join: " + str(sheet_row_count(employees))

dim innerResult = employees join departments on "dept_id" = "id"
print "After inner join: " + str(sheet_row_count(innerResult))

dim leftResult = employees left join departments on "dept_id" = "id"  
print "After left join: " + str(sheet_row_count(leftResult))

' Find unmatched records
dim unmatched = query("
    SELECT * FROM leftResult 
    WHERE department_name IS NULL
")
print "Unmatched employees: " + str(sheet_row_count(unmatched))
```

**Solutions**:
- Use left join to preserve all primary records
- Check for missing reference data
- Clean join keys before joining

## Best Practices

### 1. Plan Your Join Strategy

```vba
' Document your join logic
' Purpose: Create monthly sales report with customer and product details
' Strategy: 
' 1. Start with sales (primary data)
' 2. Left join customers (preserve all sales)
' 3. Inner join products (skip sales without valid products)

dim monthlySales = import "sales_2023_12.csv" into sheet
dim customers = import "customers.csv" into sheet  
dim products = import "products.csv" into sheet

dim step1 = monthlySales left join customers on "customer_id" = "id"
dim finalReport = step1 join products on "product_sku" = "sku"
```

### 2. Validate Data Quality

```vba
function validateJoinData(sheet, keyColumn)
    ' Check for nulls
    dim nulls = query("SELECT COUNT(*) FROM sheet WHERE " + keyColumn + " IS NULL")
    if nulls > 0 then
        print "Warning: " + str(nulls) + " null values in " + keyColumn
    end if
    
    ' Check for duplicates
    dim dupes = query("
        SELECT " + keyColumn + ", COUNT(*) as count
        FROM sheet 
        GROUP BY " + keyColumn + " 
        HAVING count > 1
    ")
    if sheet_row_count(dupes) > 0 then
        print "Warning: Duplicate values in " + keyColumn
        print dupes
    end if
    
    ' Check data distribution  
    dim stats = query("
        SELECT 
            COUNT(*) as total_rows,
            COUNT(DISTINCT " + keyColumn + ") as unique_values,
            MIN(" + keyColumn + ") as min_value,
            MAX(" + keyColumn + ") as max_value
        FROM sheet
    ")
    print "Data stats for " + keyColumn + ":"
    print stats
end function
```

### 3. Handle Null Values Appropriately

```vba
' Decide how to handle nulls before joining
dim sales = import "sales.csv" into sheet
dim customers = import "customers.csv" into sheet

' Option 1: Exclude nulls
dim cleanSales = query("SELECT * FROM sales WHERE customer_id IS NOT NULL")
dim result1 = cleanSales join customers on "customer_id" = "id"

' Option 2: Include nulls but mark them
dim allSales = sales left join customers on "customer_id" = "id"  
dim marked = query("
    SELECT *,
        CASE 
            WHEN customer_name IS NULL THEN 'Unknown Customer'
            ELSE customer_name 
        END as display_name
    FROM allSales
")
```

### 4. Test with Representative Data

```vba
' Create test scenarios
function testJoinScenario(name, leftData, rightData, joinKey)
    print "Testing scenario: " + name
    
    dim result = leftData join rightData on joinKey
    print "- Result rows: " + str(sheet_row_count(result))
    
    dim leftCount = sheet_row_count(leftData)
    dim rightCount = sheet_row_count(rightData)
    dim resultCount = sheet_row_count(result)
    
    if resultCount = 0 and leftCount > 0 and rightCount > 0 then
        print "- WARNING: No matches found!"
    elseif resultCount > leftCount + rightCount then
        print "- WARNING: Result larger than input!"
    else
        print "- Join completed successfully"
    end if
    
    print ""
end function

' Test different scenarios
testJoinScenario("Normal case", employees, departments, "dept_id")
testJoinScenario("Empty left", query("SELECT * FROM employees WHERE 1=0"), departments, "dept_id")
testJoinScenario("Empty right", employees, query("SELECT * FROM departments WHERE 1=0"), "dept_id")
```

### 5. Document Your Joins

```vba
' Use clear variable names and comments
' BAD: Generic names
dim r1 = a join b on "id"
dim r2 = r1 join c on "x" 

' GOOD: Descriptive names  
dim employeesWithDepartments = employees join departments on "dept_id" = "id"
dim completeEmployeeView = employeesWithDepartments join locations on "location_id" = "id"

' Document complex join logic
' Join sales with customers and products to create analysis dataset
' - Use left join for customers to preserve all sales (even with missing customer data)
' - Use inner join for products to exclude sales with invalid product codes
' - Result: All valid sales with customer info where available
dim salesWithCustomers = sales left join customers on "customer_id" = "id"
dim salesAnalysisDataset = salesWithCustomers join products on "product_sku" = "sku"
```

## Integration with Other Features

### Joins with SQL Queries

```vba
' Combine joins with complex SQL logic
dim orders = import "orders.csv" into sheet
dim customers = import "customers.csv" into sheet

' Simple join first
dim orderData = orders join customers on "customer_id" = "id"

' Then complex analysis with SQL
dim monthlyTrends = query("
    SELECT 
        strftime('%Y-%m', order_date) as month,
        customer_segment,
        COUNT(*) as order_count,
        SUM(order_total) as total_revenue,
        AVG(order_total) as avg_order_value,
        COUNT(DISTINCT customer_id) as unique_customers
    FROM orderData
    WHERE order_date >= date('now', '-12 months')
    GROUP BY month, customer_segment
    ORDER BY month, customer_segment
")
```

### Joins with HTTP Data

```vba
' Join local data with API responses
dim localCustomers = import "customers.csv" into sheet

' Fetch additional data from API
dim apiResponse = fetch("https://api.example.com/customer-metrics")
dim apiData = parse(apiResponse) into sheet

' Join local and remote data
dim enrichedCustomers = localCustomers left join apiData on "customer_id" = "id"
```

### Joins in ETL Pipelines

```vba
' Complete ETL pipeline with multiple joins
sub processMonthlyReport()
    ' Extract
    dim rawSales = import "sales/" + getCurrentMonth() + ".csv" into sheet
    dim customers = import "master_data/customers.csv" into sheet
    dim products = import "master_data/products.csv" into sheet
    dim territories = import "master_data/territories.csv" into sheet
    
    ' Transform with joins
    dim step1 = rawSales join customers on "customer_id" = "id"
    dim step2 = step1 join products on "product_id" = "id" 
    dim finalData = step2 left join territories on "territory_id" = "id"
    
    ' Clean and validate
    dim cleanData = query("
        SELECT *
        FROM finalData
        WHERE sales_amount > 0 
          AND customer_name IS NOT NULL
          AND product_name IS NOT NULL
    ")
    
    ' Load
    export cleanData to "reports/monthly_sales_" + getCurrentMonth() + ".xlsx"
    
    print "Monthly report generated with " + str(sheet_row_count(cleanData)) + " records"
end sub
```

This guide should give you a solid foundation for using joins effectively in PipTable. Remember that joins are powerful but can be complex - always test with small datasets first and validate your results.

## Next Steps

- Read the [Join Operations Reference](../reference/dsl/joins.md) for complete syntax details
- Explore the [Join Cookbook](../cookbook/joins.md) for advanced examples  
- Learn about [SQL Query Integration](../reference/dsl/query.md) for complex analysis
- Review [Performance Best Practices](../guide/performance.md) for large datasets
