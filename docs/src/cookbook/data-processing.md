# Data Processing Recipes

This section contains practical examples for common data processing tasks using PipTable's DSL. Each example includes runnable code that you can test in the playground.

## Overview

PipTable excels at data processing tasks that traditionally require complex Python or R scripts. With its SQL-native approach and simple VBA-like syntax, you can:

- Clean and transform messy data
- Aggregate and summarize datasets
- Merge multiple data sources
- Generate reports and analytics

## Common Patterns

### Filter and Transform
```piptable
' @title Filter and Transform Example
' @description Clean data by filtering rows and transforming columns

DIM sales AS SHEET = READ("sales.csv")

' Filter for high-value transactions
DIM high_value AS SHEET = QUERY(sales, 
  "SELECT * FROM sales WHERE amount > 1000")

' Add calculated columns
DIM with_margin AS SHEET = QUERY(high_value,
  "SELECT *, amount * 0.15 as margin FROM high_value")

WRITE(with_margin, "high_value_sales.csv")
```

### Aggregate and Summarize
```piptable
' @title Sales Summary by Category
' @description Group sales data and calculate totals

DIM transactions AS SHEET = READ("transactions.csv")

DIM summary AS SHEET = QUERY(transactions, 
  "SELECT 
    category,
    COUNT(*) as transaction_count,
    SUM(amount) as total_sales,
    AVG(amount) as avg_sale,
    MAX(amount) as largest_sale
   FROM transactions 
   GROUP BY category
   ORDER BY total_sales DESC")

PRINT "Sales Summary Generated"
WRITE(summary, "category_summary.csv")
```

### Data Validation
```piptable
' @title Data Validation Example
' @description Find and report data quality issues

DIM data AS SHEET = READ("customer_data.csv")

' Find records with missing required fields
DIM missing_email AS SHEET = QUERY(data, 
  "SELECT * FROM data WHERE email IS NULL OR email = ''")

' Find duplicate records
DIM duplicates AS SHEET = QUERY(data,
  "SELECT email, COUNT(*) as count 
   FROM data 
   GROUP BY email 
   HAVING COUNT(*) > 1")

IF LEN(missing_email) > 0 THEN
  PRINT "Found " + STR(LEN(missing_email)) + " records with missing emails"
  WRITE(missing_email, "data_quality_missing_emails.csv")
END IF

IF LEN(duplicates) > 0 THEN
  PRINT "Found " + STR(LEN(duplicates)) + " duplicate email addresses"
  WRITE(duplicates, "data_quality_duplicates.csv")
END IF
```

## Next Steps

- [CSV Operations](csv.md) - Working with CSV files
- [Excel Processing](excel.md) - Reading and writing Excel files
- [JSON Transformation](json.md) - Processing JSON data