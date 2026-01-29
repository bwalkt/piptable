# Join Operations Cookbook

This cookbook provides practical examples of join operations in PipTable, demonstrating real-world scenarios and advanced techniques.

## Table of Contents

1. [Basic Join Examples](#basic-join-examples)
2. [Real-World Scenarios](#real-world-scenarios)
3. [Performance Optimization](#performance-optimization)
4. [Error Handling](#error-handling)
5. [Advanced Patterns](#advanced-patterns)

## Basic Join Examples

### Employee-Department Join

**Scenario**: Match employees with their department information.

```vba
' Sample data files:
' employees.csv: id, name, dept_id, salary
' departments.csv: id, name, budget, manager

dim employees = import "employees.csv" into sheet
dim departments = import "departments.csv" into sheet

' Inner join - only employees with valid departments
dim employeesWithDepts = employees join departments on "dept_id" = "id"

' Left join - all employees, including those without departments
dim allEmployees = employees left join departments on "dept_id" = "id"

' Export results
export employeesWithDepts to "employee_department_report.csv"
```

### Sales Data Analysis

**Scenario**: Analyze sales data across customers and products.

```vba
' Data files:
' sales.csv: id, customer_id, product_id, amount, date
' customers.csv: id, name, email, region
' products.csv: id, name, category, price

dim sales = import "sales.csv" into sheet
dim customers = import "customers.csv" into sheet  
dim products = import "products.csv" into sheet

' Multi-table join for complete sales analysis
dim salesAnalysis = (sales join customers on "customer_id" = "id") join products on "product_id" = "id"

' Filter for high-value sales
dim highValueSales = query("
    SELECT customer_name, product_name, amount, date, region
    FROM salesAnalysis 
    WHERE amount > 1000
    ORDER BY amount DESC
")

export highValueSales to "high_value_sales.xlsx"
```

## Real-World Scenarios

### E-commerce Order Processing

```vba
' Complete order processing pipeline
dim orders = import "orders.csv" into sheet          ' order_id, customer_id, date, status
dim orderItems = import "order_items.csv" into sheet ' order_id, product_id, quantity, price
dim customers = import "customers.csv" into sheet    ' id, name, email, address
dim products = import "products.csv" into sheet      ' id, name, sku, category

' Build comprehensive order view
dim orderDetails = orders join customers on "customer_id" = "id"
dim itemDetails = orderItems join products on "product_id" = "id"
dim fullOrders = orderDetails join itemDetails on "order_id"

' Generate customer order summary
dim customerSummary = query("
    SELECT 
        customer_name,
        customer_email,
        COUNT(*) as total_orders,
        SUM(quantity * price) as total_spent,
        AVG(quantity * price) as avg_order_value
    FROM fullOrders
    WHERE order_status = 'completed'
    GROUP BY customer_id, customer_name, customer_email
    HAVING total_spent > 500
    ORDER BY total_spent DESC
")

export customerSummary to "customer_analysis.xlsx"
```

### Financial Reporting

```vba
' Monthly financial reporting
dim transactions = import "transactions.csv" into sheet    ' id, account_id, amount, date, type
dim accounts = import "accounts.csv" into sheet          ' id, name, type, category
dim budgets = import "budgets.csv" into sheet           ' account_id, month, budgeted_amount

' Join transactions with account information
dim transactionDetails = transactions join accounts on "account_id" = "id"

' Add budget information
dim fullFinancials = transactionDetails left join budgets on "account_id" = "account_id"

' Generate monthly variance report
dim monthlyVariance = query("
    SELECT 
        account_name,
        account_category,
        strftime('%Y-%m', date) as month,
        SUM(amount) as actual_amount,
        AVG(budgeted_amount) as budget_amount,
        SUM(amount) - AVG(budgeted_amount) as variance,
        ROUND((SUM(amount) - AVG(budgeted_amount)) / AVG(budgeted_amount) * 100, 2) as variance_pct
    FROM fullFinancials
    WHERE date >= date('now', '-12 months')
    GROUP BY account_id, account_name, account_category, month
    ORDER BY month DESC, variance DESC
")

export monthlyVariance to "variance_report.xlsx"
```

### Inventory Management

```vba
' Inventory tracking and reorder analysis
dim inventory = import "inventory.csv" into sheet        ' product_id, quantity_on_hand, location
dim products = import "products.csv" into sheet         ' id, name, sku, category, reorder_level
dim suppliers = import "suppliers.csv" into sheet       ' id, name, contact, lead_time
dim productSuppliers = import "product_suppliers.csv" into sheet ' product_id, supplier_id, cost

' Build complete inventory view
dim inventoryDetails = inventory join products on "product_id" = "id"
dim supplierInfo = productSuppliers join suppliers on "supplier_id" = "id"
dim fullInventory = inventoryDetails left join supplierInfo on "product_id"

' Identify items needing reorder
dim reorderReport = query("
    SELECT 
        product_name,
        sku,
        category,
        quantity_on_hand,
        reorder_level,
        supplier_name,
        supplier_contact,
        cost,
        lead_time,
        (reorder_level * 2) - quantity_on_hand as suggested_order_qty
    FROM fullInventory
    WHERE quantity_on_hand <= reorder_level
    ORDER BY (quantity_on_hand / reorder_level) ASC
")

export reorderReport to "reorder_analysis.xlsx"
```

## Performance Optimization

### Pre-filtering for Large Datasets

```vba
' Optimize joins by filtering first
dim largeSalesData = import "annual_sales.csv" into sheet
dim customers = import "customers.csv" into sheet

' Bad: join then filter
dim slowResult = largeSalesData join customers on "customer_id" = "id"
dim filtered = query("SELECT * FROM slowResult WHERE amount > 1000")

' Good: filter then join
dim highValueSales = query("SELECT * FROM largeSalesData WHERE amount > 1000")
dim optimizedResult = highValueSales join customers on "customer_id" = "id"
```

### Index-Friendly Operations

```vba
' Ensure consistent data types in join columns
dim orders = import "orders.csv" into sheet
dim customers = import "customers.csv" into sheet

' Clean data before joining
dim cleanOrders = query("
    SELECT 
        order_id,
        CAST(customer_id AS INTEGER) as customer_id,
        amount,
        date
    FROM orders 
    WHERE customer_id IS NOT NULL 
      AND customer_id != ''
")

dim cleanCustomers = query("
    SELECT 
        CAST(id AS INTEGER) as id,
        name,
        email
    FROM customers
    WHERE id IS NOT NULL
")

' Now join with consistent integer types
dim result = cleanOrders join cleanCustomers on "customer_id" = "id"
```

### Memory-Efficient Chained Joins

```vba
' Process large datasets incrementally
dim orders = import "large_orders.csv" into sheet
dim customers = import "customers.csv" into sheet  
dim products = import "products.csv" into sheet

' Approach 1: Build incrementally
dim step1 = query("
    SELECT order_id, customer_id, product_id, amount, date
    FROM orders 
    WHERE date >= date('now', '-30 days')
")

dim step2 = step1 join customers on "customer_id" = "id"
dim finalResult = step2 join products on "product_id" = "id"

' Approach 2: Use SQL for complex joins
dim sqlResult = query("
    SELECT 
        o.order_id,
        c.name as customer_name,
        p.name as product_name,
        o.amount,
        o.date
    FROM (
        SELECT * FROM orders 
        WHERE date >= date('now', '-30 days')
    ) o
    JOIN customers c ON o.customer_id = c.id
    JOIN products p ON o.product_id = p.id
")
```

## Error Handling

### Defensive Join Programming

```vba
' Validate data before joining
function safeJoin(leftSheet, rightSheet, leftKey, rightKey)
    ' Check if sheets exist
    if leftSheet is nothing or rightSheet is nothing then
        print "Error: One or both sheets are null"
        return nothing
    end if
    
    ' Check if columns exist
    if not hasColumn(leftSheet, leftKey) then
        print "Error: Left sheet missing column: " + leftKey
        return nothing
    end if
    
    if not hasColumn(rightSheet, rightKey) then
        print "Error: Right sheet missing column: " + rightKey  
        return nothing
    end if
    
    ' Check for empty datasets
    if getRowCount(leftSheet) = 0 then
        print "Warning: Left sheet is empty"
        return rightSheet ' or return empty sheet
    end if
    
    if getRowCount(rightSheet) = 0 then
        print "Warning: Right sheet is empty"  
        return leftSheet ' or return empty sheet
    end if
    
    ' Perform the join
    return leftSheet join rightSheet on leftKey = rightKey
end function

' Usage
dim employees = import "employees.csv" into sheet
dim departments = import "departments.csv" into sheet
dim result = safeJoin(employees, departments, "dept_id", "id")
```

### Handling Missing Data

```vba
' Deal with null values in join columns
dim sales = import "sales_with_nulls.csv" into sheet
dim customers = import "customers.csv" into sheet

' Option 1: Filter out nulls before joining
dim cleanSales = query("
    SELECT * FROM sales 
    WHERE customer_id IS NOT NULL 
      AND customer_id != ''
      AND customer_id != '0'
")
dim result1 = cleanSales join customers on "customer_id" = "id"

' Option 2: Use left join and identify missing matches
dim allSales = sales left join customers on "customer_id" = "id"
dim orphanSales = query("
    SELECT * FROM allSales 
    WHERE customer_name IS NULL
")

print "Found " + str(getRowCount(orphanSales)) + " sales without customer data"
export orphanSales to "orphan_sales.csv"
```

## Advanced Patterns

### Self-Joins

```vba
' Employee hierarchy analysis
dim employees = import "employees.csv" into sheet ' id, name, manager_id, dept_id

' Join employees with their managers
dim employeeHierarchy = employees left join employees on "manager_id" = "id"

' Note: Need to alias columns to avoid conflicts
dim hierarchyReport = query("
    SELECT 
        e.name as employee_name,
        m.name as manager_name,
        e.dept_id,
        CASE WHEN m.name IS NULL THEN 'Top Level' ELSE 'Reports To: ' || m.name END as hierarchy_level
    FROM employees e
    LEFT JOIN employees m ON e.manager_id = m.id
    ORDER BY m.name, e.name
")

export hierarchyReport to "org_hierarchy.xlsx"
```

### Complex Multi-Table Joins

```vba
' Supply chain analysis with multiple relationships
dim purchases = import "purchases.csv" into sheet      ' id, supplier_id, product_id, qty, cost, date
dim suppliers = import "suppliers.csv" into sheet     ' id, name, country, rating
dim products = import "products.csv" into sheet       ' id, name, category, unit
dim categories = import "categories.csv" into sheet   ' name, department, margin_target

' Build comprehensive supply chain view
dim purchaseDetails = (purchases join suppliers on "supplier_id" = "id") 
                     join products on "product_id" = "id"
dim fullSupplyChain = purchaseDetails join categories on "category" = "name"

' Analyze supplier performance by category
dim supplierAnalysis = query("
    SELECT 
        supplier_name,
        supplier_country,
        supplier_rating,
        category,
        department,
        COUNT(*) as total_purchases,
        SUM(qty) as total_quantity,
        SUM(cost) as total_cost,
        AVG(cost / qty) as avg_unit_cost,
        margin_target,
        ROUND(AVG(cost / qty) * (1 + margin_target/100), 2) as suggested_sell_price
    FROM fullSupplyChain
    WHERE date >= date('now', '-12 months')
    GROUP BY supplier_id, supplier_name, supplier_country, supplier_rating, 
             category, department, margin_target
    HAVING total_purchases >= 5
    ORDER BY category, total_cost DESC
")

export supplierAnalysis to "supplier_performance.xlsx"
```

### Conditional Joins

```vba
' Different join strategies based on data characteristics
dim orders = import "orders.csv" into sheet
dim customers = import "customers.csv" into sheet

' Check data quality first
dim customerIdCount = query("SELECT COUNT(DISTINCT customer_id) FROM orders")
dim customerCount = query("SELECT COUNT(*) FROM customers")

if customerIdCount > customerCount then
    print "Warning: More unique customer IDs in orders than customers in master list"
    print "Using left join to preserve all order data"
    dim result = orders left join customers on "customer_id" = "id"
else
    print "Customer data appears complete, using inner join"
    dim result = orders join customers on "customer_id" = "id"
end if

' Add data quality metrics to output
dim qualityCheck = query("
    SELECT 
        *,
        CASE 
            WHEN customer_name IS NULL THEN 'Missing Customer Data'
            ELSE 'Complete'
        END as data_quality_flag
    FROM result
")

export qualityCheck to "orders_with_quality_flags.xlsx"
```

### Join with Aggregation

```vba
' Customer lifetime value calculation
dim orders = import "orders.csv" into sheet        ' id, customer_id, amount, date
dim customers = import "customers.csv" into sheet  ' id, name, email, signup_date

' First, calculate customer aggregates
dim customerMetrics = query("
    SELECT 
        customer_id,
        COUNT(*) as total_orders,
        SUM(amount) as total_spent,
        AVG(amount) as avg_order_value,
        MIN(date) as first_order,
        MAX(date) as last_order,
        ROUND((julianday(MAX(date)) - julianday(MIN(date))) / 365.25, 2) as customer_lifespan_years
    FROM orders
    GROUP BY customer_id
")

' Join with customer details
dim customerLTV = customers join customerMetrics on "id" = "customer_id"

' Calculate lifetime value segments
dim ltvAnalysis = query("
    SELECT 
        *,
        CASE 
            WHEN customer_lifespan_years > 0 THEN total_spent / customer_lifespan_years
            ELSE total_spent
        END as annual_value,
        CASE 
            WHEN total_spent >= 5000 THEN 'VIP'
            WHEN total_spent >= 1000 THEN 'High Value'
            WHEN total_spent >= 500 THEN 'Medium Value'
            ELSE 'Low Value'
        END as customer_segment
    FROM customerLTV
    ORDER BY total_spent DESC
")

export ltvAnalysis to "customer_ltv_analysis.xlsx"
```

## Best Practices Summary

1. **Always validate data before joining** - Check for required columns and data types
2. **Filter early** - Reduce dataset size before joins when possible
3. **Use appropriate join types** - Inner for strict matches, left/right for optional data
4. **Handle nulls explicitly** - Decide how to deal with missing join keys
5. **Test with sample data** - Validate join logic on small datasets first
6. **Monitor performance** - Use query plans and timing for large datasets
7. **Document join relationships** - Make business logic clear in comments
8. **Use meaningful variable names** - Make code self-documenting

These patterns and examples should cover most real-world join scenarios you'll encounter with PipTable. Remember that joins are powerful but can be memory and CPU intensive with large datasets, so always consider performance implications in production environments.