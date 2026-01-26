# CSV Operations

CSV is the most common data format for data exchange. PipTable provides powerful tools for reading, transforming, and writing CSV files.

## Reading CSV Files

### Basic CSV Import
```piptable
' @title Basic CSV Import
' @description Import a CSV file with headers

import "data.csv" into data
print "Loaded " + str(len(data)) + " rows"
```

### Import Without Headers
```piptable
' @title CSV Without Headers
' @description Import CSV and manually assign column names

import "data.csv" into raw_data without headers

' Add column names using SQL with file reference
dim data: table = query(
  SELECT 
    _1 as customer_id,
    _2 as name,
    _3 as email,
    _4 as amount
  FROM "data.csv"
)
```

### Import Multiple CSV Files
```piptable
' @title Merge Multiple CSV Files
' @description Combine multiple CSV files into one dataset

' Import multiple files into a book
import "sales_jan.csv", "sales_feb.csv", "sales_mar.csv" into quarterly

' Consolidate into single sheet
dim all_sales: table = consolidate(quarterly)
print "Combined " + str(len(all_sales)) + " total records"

export all_sales to "sales_q1_combined.csv"
```

## Cleaning CSV Data

### Handle Missing Values
```piptable
' @title Clean Missing Values
' @description Replace nulls and clean data

import "messy_data.csv" into raw

' Replace nulls with defaults using file reference
dim cleaned: table = query(
  SELECT 
    COALESCE(name, 'Unknown') as name,
    COALESCE(email, 'no-email@example.com') as email,
    COALESCE(age, 0) as age,
    COALESCE(status, 'pending') as status
  FROM "messy_data.csv"
)

export cleaned to "cleaned_data.csv"
```

### Remove Duplicates
```piptable
' @title Remove Duplicate Records
' @description Keep only unique records based on key columns

import "customers.csv" into data

' Remove duplicates keeping most recent record per email using file reference
dim unique_customers: table = query(
  SELECT * FROM (
    SELECT *,
      ROW_NUMBER() OVER (PARTITION BY email ORDER BY created_date DESC) as rn
    FROM "customers.csv"
  ) WHERE rn = 1
)

print "Removed " + str(len(data) - len(unique_customers)) + " duplicates"
export unique_customers to "unique_customers.csv"
```

## Transforming CSV Data

### Pivot Data
```piptable
' @title Pivot Sales Data
' @description Transform rows to columns for reporting

import "sales_data.csv" into sales

' Pivot monthly sales by product using file reference
dim pivoted: table = query(
  SELECT 
    product,
    SUM(CASE WHEN month = 'Jan' THEN amount ELSE 0 END) as jan_sales,
    SUM(CASE WHEN month = 'Feb' THEN amount ELSE 0 END) as feb_sales,
    SUM(CASE WHEN month = 'Mar' THEN amount ELSE 0 END) as mar_sales,
    SUM(amount) as total_sales
  FROM "sales_data.csv"
  GROUP BY product
  ORDER BY total_sales DESC
)

export pivoted to "sales_by_product_month.csv"
```

### Split and Combine Columns
```piptable
' @title Split and Combine Columns
' @description Parse and restructure column data

import "contacts.csv" into contacts

' Split full name and combine address fields using file reference
dim formatted: table = query(
  SELECT 
    SPLIT_PART(full_name, ' ', 1) as first_name,
    SPLIT_PART(full_name, ' ', -1) as last_name,
    street || ', ' || city || ', ' || state || ' ' || zip as full_address,
    email,
    phone
  FROM "contacts.csv"
)

export formatted to "contacts_formatted.csv"
```

## Writing CSV Files

### Export with Custom Delimiter
```piptable
' @title Export Tab-Delimited
' @description Export data as TSV (tab-separated values)

import "input.csv" into data

' Process data using file reference
dim processed: table = query(
  SELECT * FROM "input.csv" WHERE status = 'active'
)

' Export as TSV (Note: delimiter option may not be supported)
export processed to "output.tsv"
```

### Export Subsets
```piptable
' @title Export Data Subsets
' @description Split data into multiple CSV files

import "orders.csv" into all_orders

' Export predefined regions to separate files
' East region
dim east_orders: table = query(
  SELECT * FROM "orders.csv" WHERE region = 'East'
)
export east_orders to "orders_East.csv"
print "Exported " + str(len(east_orders)) + " orders for East"

' West region  
dim west_orders: table = query(
  SELECT * FROM "orders.csv" WHERE region = 'West'
)
export west_orders to "orders_West.csv"
print "Exported " + str(len(west_orders)) + " orders for West"

' North region
dim north_orders: table = query(
  SELECT * FROM "orders.csv" WHERE region = 'North'
)
export north_orders to "orders_North.csv"
print "Exported " + str(len(north_orders)) + " orders for North"

' South region
dim south_orders: table = query(
  SELECT * FROM "orders.csv" WHERE region = 'South'
)
export south_orders to "orders_South.csv"
print "Exported " + str(len(south_orders)) + " orders for South"
```

## Performance Tips

1. **Use without headers** for large files without headers to avoid parsing overhead
2. **Import multiple files at once** using comma-separated paths
3. **Use consolidate()** to efficiently combine sheets
4. **Filter early** in your queries to reduce memory usage
5. **Use appropriate data types** in queries for better performance

## Next Steps

- [Excel Processing](excel.md) - Working with Excel files
- [JSON Transformation](json.md) - Processing JSON data
- [ETL Pipelines](etl.md) - Building data pipelines