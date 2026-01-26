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

import "sales.csv" into sales

' Filter for high-value transactions and add calculated columns
dim with_margin: table = query(
  SELECT 
    *,
    amount * 0.15 as margin
  FROM "sales.csv"
  WHERE amount > 1000
)

export with_margin to "high_value_sales.csv"
```

### Aggregate and Summarize
```piptable
' @title Sales Summary by Category
' @description Group sales data and calculate totals

import "transactions.csv" into transactions

dim summary: table = query(
  SELECT 
    category,
    COUNT(*) as transaction_count,
    SUM(amount) as total_sales,
    AVG(amount) as avg_sale,
    MAX(amount) as largest_sale
  FROM "transactions.csv"
  GROUP BY category
  ORDER BY total_sales DESC
)

print "Sales Summary Generated"
export summary to "category_summary.csv"
```

### Data Validation
```piptable
' @title Data Validation Example
' @description Find and report data quality issues

import "customer_data.csv" into data

' Find records with missing required fields
dim missing_email: table = query(
  SELECT * FROM "customer_data.csv" WHERE email IS NULL OR email = ''
)

' Find duplicate records
dim duplicates: table = query(
  SELECT email, COUNT(*) as count 
  FROM "customer_data.csv" 
  GROUP BY email 
  HAVING COUNT(*) > 1
)

if len(missing_email) > 0 then
  print "Found " + str(len(missing_email)) + " records with missing emails"
  export missing_email to "data_quality_missing_emails.csv"
end if

if len(duplicates) > 0 then
  print "Found " + str(len(duplicates)) + " duplicate email addresses"
  export duplicates to "data_quality_duplicates.csv"
end if
```

## Next Steps

- [CSV Operations](csv.md) - Working with CSV files
- [Excel Processing](excel.md) - Reading and writing Excel files
- [JSON Transformation](json.md) - Processing JSON data
