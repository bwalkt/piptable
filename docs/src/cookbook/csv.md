# CSV Operations

CSV is the most common data format for data exchange. PipTable provides powerful tools for reading, transforming, and writing CSV files.

## Reading CSV Files

### Basic CSV Import
```piptable
' @title Basic CSV Import
' @description Import a CSV file with headers

DIM data AS SHEET = READ("data.csv")
PRINT "Loaded " + STR(LEN(data)) + " rows"
```

### Import Without Headers
```piptable
' @title CSV Without Headers
' @description Import CSV and manually assign column names

IMPORT "data.csv" WITHOUT HEADERS INTO raw_data

' Add column names using SQL
DIM data AS SHEET = QUERY(raw_data, 
  "SELECT 
    _1 as customer_id,
    _2 as name,
    _3 as email,
    _4 as amount
   FROM raw_data")
```

### Import Multiple CSV Files
```piptable
' @title Merge Multiple CSV Files
' @description Combine multiple CSV files into one dataset

' Import multiple files into a book
IMPORT "sales_jan.csv,sales_feb.csv,sales_mar.csv" INTO quarterly

' Consolidate into single sheet
DIM all_sales AS SHEET = CONSOLIDATE(quarterly)
PRINT "Combined " + STR(LEN(all_sales)) + " total records"

WRITE(all_sales, "sales_q1_combined.csv")
```

## Cleaning CSV Data

### Handle Missing Values
```piptable
' @title Clean Missing Values
' @description Replace nulls and clean data

DIM raw AS SHEET = READ("messy_data.csv")

' Replace nulls with defaults
DIM cleaned AS SHEET = QUERY(raw,
  "SELECT 
    COALESCE(name, 'Unknown') as name,
    COALESCE(email, 'no-email@example.com') as email,
    COALESCE(age, 0) as age,
    COALESCE(status, 'pending') as status
   FROM raw")

WRITE(cleaned, "cleaned_data.csv")
```

### Remove Duplicates
```piptable
' @title Remove Duplicate Records
' @description Keep only unique records based on key columns

DIM data AS SHEET = READ("customers.csv")

' Remove duplicates keeping most recent record per email
DIM unique_customers AS SHEET = QUERY(data,
  "SELECT * FROM (
     SELECT *,
       ROW_NUMBER() OVER (PARTITION BY email ORDER BY created_date DESC) as rn
     FROM data
   ) WHERE rn = 1")

PRINT "Removed " + STR(LEN(data) - LEN(unique_customers)) + " duplicates"
WRITE(unique_customers, "unique_customers.csv")
```

## Transforming CSV Data

### Pivot Data
```piptable
' @title Pivot Sales Data
' @description Transform rows to columns for reporting

DIM sales AS SHEET = READ("sales_data.csv")

' Pivot monthly sales by product
DIM pivoted AS SHEET = QUERY(sales,
  "SELECT 
    product,
    SUM(CASE WHEN month = 'Jan' THEN amount ELSE 0 END) as jan_sales,
    SUM(CASE WHEN month = 'Feb' THEN amount ELSE 0 END) as feb_sales,
    SUM(CASE WHEN month = 'Mar' THEN amount ELSE 0 END) as mar_sales,
    SUM(amount) as total_sales
   FROM sales
   GROUP BY product
   ORDER BY total_sales DESC")

WRITE(pivoted, "sales_by_product_month.csv")
```

### Split and Combine Columns
```piptable
' @title Split and Combine Columns
' @description Parse and restructure column data

DIM contacts AS SHEET = READ("contacts.csv")

' Split full name and combine address fields
DIM formatted AS SHEET = QUERY(contacts,
  "SELECT 
    SPLIT_PART(full_name, ' ', 1) as first_name,
    SPLIT_PART(full_name, ' ', -1) as last_name,
    street || ', ' || city || ', ' || state || ' ' || zip as full_address,
    email,
    phone
   FROM contacts")

WRITE(formatted, "contacts_formatted.csv")
```

## Writing CSV Files

### Export with Custom Delimiter
```piptable
' @title Export Tab-Delimited
' @description Export data as TSV (tab-separated values)

DIM data AS SHEET = READ("input.csv")

' Process data
DIM processed AS SHEET = QUERY(data, 
  "SELECT * FROM data WHERE status = 'active'")

' Export as TSV
EXPORT processed TO "output.tsv" WITH DELIMITER "\t"
```

### Export Subsets
```piptable
' @title Export Data Subsets
' @description Split data into multiple CSV files

DIM all_orders AS SHEET = READ("orders.csv")

' Get unique regions
DIM regions AS SHEET = QUERY(all_orders, 
  "SELECT DISTINCT region FROM all_orders")

' Export each region to separate file
FOR EACH row IN regions
  ' Use parameterized filtering to avoid SQL injection
  DIM region_data AS SHEET = QUERY(all_orders, 
    "SELECT * FROM all_orders WHERE region = ?", row.region)
  
  ' Alternative: Use FILTER for type-safe filtering
  ' DIM region_data AS SHEET = FILTER(all_orders, 
  '   WHERE all_orders.region = row.region)
  
  WRITE(region_data, "orders_" + row.region + ".csv")
  PRINT "Exported " + STR(LEN(region_data)) + " orders for " + row.region
NEXT
```

## Performance Tips

1. **Use WITHOUT HEADERS** for large files without headers to avoid parsing overhead
2. **Import multiple files at once** using comma-separated paths
3. **Use CONSOLIDATE** to efficiently combine sheets
4. **Filter early** in your queries to reduce memory usage
5. **Use appropriate data types** in queries for better performance

## Next Steps

- [Excel Processing](excel.md) - Working with Excel files
- [JSON Transformation](json.md) - Processing JSON data
- [ETL Pipelines](etl.md) - Building data pipelines