# ETL Pipelines

Build robust Extract, Transform, Load (ETL) pipelines using PipTable's powerful data processing capabilities. These examples show complete workflows from data ingestion to final output.

## Excel-Based ETL with SQL

### Process Multiple Excel Sources
```piptable
' @title ETL from Excel Sources
' @description Combine multiple Excel files with SQL

' Import Excel files
import "sales_2023.xlsx" into sales_2023
import "sales_2024.xlsx" into sales_2024
import "customer_master.xlsx" into customers

' Union sales data across years
dim all_sales = query(
    SELECT '2023' as year, * FROM sales_2023
    UNION ALL
    SELECT '2024' as year, * FROM sales_2024
)

' Enrich with customer data
dim enriched_sales = query(
    SELECT 
        s.*,
        c.customer_name,
        c.customer_segment,
        c.region
    FROM all_sales s
    LEFT JOIN customers c ON s.customer_id = c.id
)

' Calculate KPIs
dim kpis = query(
    SELECT 
        year,
        customer_segment,
        region,
        COUNT(*) as transaction_count,
        SUM(amount) as total_revenue,
        AVG(amount) as avg_transaction
    FROM enriched_sales
    GROUP BY year, customer_segment, region
    ORDER BY year, total_revenue DESC
)

' Export results
export kpis to "sales_kpis.xlsx"
```

## Complete ETL Pipeline

### Sales Data Pipeline
```piptable
' @title Complete Sales ETL Pipeline
' @description Extract from multiple sources, transform, and load to reporting database

' EXTRACT: Load data from multiple sources
import "sales_transactions.csv" into raw_sales
import "customers.csv" into customers
import "products.json" into products

' TRANSFORM: Clean and enrich data in single query
' Combine all transformations using file references
dim final_sales: table = query(
  SELECT
    s.transaction_id,
    s.transaction_date as sale_date,
    c.customer_name,
    c.customer_segment,
    p.product_name,
    p.category,
    cast(s.quantity as int) as quantity,
    cast(s.unit_price as float) as unit_price,
    cast(s.quantity as int) * cast(s.unit_price as float) as total_amount,
    cast(s.quantity as int) * cast(s.unit_price as float) * 0.15 as profit_margin
  FROM "sales_transactions.csv" s
  LEFT JOIN "customers.csv" c ON s.customer_id = c.id
  LEFT JOIN "products.json" p ON s.product_id = p.id
  WHERE cast(s.quantity as int) > 0 AND cast(s.unit_price as float) > 0
)

' LOAD: Export to different formats
export final_sales to "processed_sales.csv"
export final_sales to "sales_report.xlsx"

print "ETL Pipeline Complete: " + str(len(final_sales)) + " records processed"
```

## Daily Data Processing

### Incremental Data Load
```piptable
' @title Incremental Daily Load
' @description Process only new records since last run

' Load existing data
import "master_database.csv" into existing_data
import "daily_extract.csv" into todays_data

' Find new records using file references
dim new_records: table = query(
  SELECT d.*
  FROM "daily_extract.csv" d
  LEFT JOIN "master_database.csv" m ON d.id = m.id
  WHERE m.id IS NULL
)

if len(new_records) > 0 then
  ' Append new records to historical file
  export new_records to "all_historical_records.csv" append
  
  ' Also save today's new records separately
  export new_records to "new_records_today.csv"
  
  print "Added " + str(len(new_records)) + " new records"
else
  print "No new records to process today"
end if
```

## Data Quality Pipeline

### Validation and Error Handling
```piptable
' @title Data Quality Pipeline
' @description Validate, clean, and separate good/bad records

import "raw_input.csv" into raw_data

' Define validation rules using file references
dim valid_records: table = query(
  SELECT * FROM "raw_input.csv"
  WHERE 
    email LIKE '%@%.%' AND
    age BETWEEN 18 AND 120 AND
    phone LIKE '__________' AND
    country IN ('US', 'CA', 'UK', 'AU')
)

dim invalid_records: table = query(
  SELECT * FROM "raw_input.csv"
  WHERE 
    email NOT LIKE '%@%.%' OR
    age NOT BETWEEN 18 AND 120 OR
    phone NOT LIKE '__________' OR
    country NOT IN ('US', 'CA', 'UK', 'AU')
)

' Process valid records using file reference
dim processed: table = query(
  SELECT 
    lower(email) as email,
    name,
    age,
    phone,
    country,
    'processed' as status
  FROM "raw_input.csv"
  WHERE 
    email LIKE '%@%.%' AND
    age BETWEEN 18 AND 120 AND
    phone LIKE '__________' AND
    country IN ('US', 'CA', 'UK', 'AU')
)

' Export results
export processed to "clean_data.csv"
export invalid_records to "rejected_records.csv"

' Generate quality report
print "Data Quality Check: " + str(len(valid_records)) + "/" + str(len(raw_data)) + " records passed"
```

## Multi-Source Integration

### Merge Heterogeneous Data
```piptable
' @title Multi-Source Data Integration
' @description Combine CSV, Excel, and JSON data (API data handled separately)

' Load from different sources
import "source1.csv" into csv_data
import "source2.xlsx" into excel_data
export excel_data to "temp_source2.csv"
import "source3.json" into json_data

' Standardize column names and add source tracking
dim std_csv: table = query(
  SELECT customer_id as id, customer_name as name, email_address as email, 'CSV' as source
  FROM "source1.csv"
)

dim std_excel: table = query(
  SELECT cust_id as id, full_name as name, email, 'EXCEL' as source
  FROM "temp_source2.csv"
)

dim std_json: table = query(
  SELECT id, name, contact_email as email, 'JSON' as source
  FROM "source3.json"
)

' Combine and deduplicate all sources
dim unique_customers: table = query(
  SELECT 
    MIN(id) as id,
    FIRST(name) as name,
    email,
    STRING_AGG(source, ',') as sources
  FROM (
    SELECT id, name, email, source FROM std_csv
    UNION ALL
    SELECT id, name, email, source FROM std_excel
    UNION ALL
    SELECT id, name, email, source FROM std_json
  )
  GROUP BY email
)

export unique_customers to "integrated_customers.csv"
print "Integrated " + str(len(unique_customers)) + " unique customers from 3 sources"
```

## Scheduled Reporting

### Automated Report Generation
```piptable
' @title Automated Daily Report
' @description Generate and distribute daily reports

' Load and filter today's data
import "transactions.csv" into transactions

dim todays_transactions: table = query(
  SELECT * FROM "transactions.csv" 
  WHERE transaction_date = CURRENT_DATE
)

' Generate all reports from file references
dim daily_summary: table = query(
  SELECT 
    CURRENT_DATE as report_date,
    COUNT(*) as total_transactions,
    COUNT(DISTINCT customer_id) as unique_customers,
    SUM(amount) as total_revenue,
    AVG(amount) as avg_transaction_value,
    MAX(amount) as largest_transaction
  FROM "transactions.csv"
  WHERE transaction_date = CURRENT_DATE
)

' Top products
dim top_products: table = query(
  SELECT 
    product_name,
    COUNT(*) as units_sold,
    SUM(amount) as revenue
  FROM "transactions.csv"
  WHERE transaction_date = CURRENT_DATE
  GROUP BY product_name
  ORDER BY revenue DESC
  LIMIT 10
)

' Hourly breakdown
dim hourly_sales: table = query(
  SELECT 
    EXTRACT(HOUR FROM transaction_time) as hour,
    COUNT(*) as transactions,
    SUM(amount) as revenue
  FROM "transactions.csv"
  WHERE transaction_date = CURRENT_DATE
  GROUP BY EXTRACT(HOUR FROM transaction_time)
  ORDER BY hour
)

' Export reports (Note: Multi-sheet workbooks not directly supported)
export daily_summary to "daily_summary.csv"
export top_products to "top_products.csv"
export hourly_sales to "hourly_sales.csv"

print "Daily report generated with " + str(len(todays_transactions)) + " transactions"
```

## Performance Optimization

### Batch Processing
```piptable
' @title Query-Based Batch Processing
' @description Process datasets using SQL LIMIT/OFFSET batching
' WARNING: This example loads the entire file into memory!
' For truly large files, consider splitting files externally first

' Define batch parameters
dim batch_size: int = 10000
dim offset: int = 0
dim total_processed: int = 0
dim continue_processing: bool = true

' Note: Must work with file references for SQL
while continue_processing
  ' Process batch using file reference with LIMIT/OFFSET
  dim batch: table = query(
    SELECT * FROM "large_file.csv"
    LIMIT batch_size
    OFFSET offset
  )
  
  ' Check if batch is empty
  if len(batch) = 0 then
    continue_processing = false
  else
    ' Process batch (transform the already-loaded batch)
    export batch to "temp_batch.csv"
    dim processed_batch: table = query(
      SELECT 
        *,
        upper(name) as name_upper,
        lower(email) as email_lower
      FROM "temp_batch.csv"
    )
    
    ' Export batch using append mode for efficiency
    export processed_batch to "output.csv" append
    
    total_processed = total_processed + len(batch)
    offset = offset + batch_size
    
    print "Processed " + str(total_processed) + " records..."
  end if
wend

print "Batch processing complete: " + str(total_processed) + " total records"
```

### Memory-Efficient Processing
```piptable
' @title Memory-Efficient Data Processing  
' @description Process large datasets without loading entire file repeatedly
' Note: Use file chunks or streaming approaches for very large datasets

' Method 1: Process pre-split file chunks
dim total_processed: int = 0

' Process pre-split chunks (split externally: split -l 10000 large_file.csv chunk_)
' Note: Files must be explicitly referenced - no dynamic file paths
' Example assumes files: chunk_01.csv, chunk_02.csv, chunk_03.csv

' Process chunk 1
import "chunk_01.csv" into chunk1
dim processed1: table = query(
  SELECT 
    *,
    upper(name) as name_upper,
    lower(email) as email_lower,
    '1' as chunk_id
  FROM "chunk_01.csv"
)
export processed1 to "processed_chunk_01.csv"
total_processed = total_processed + len(processed1)
print "Processed chunk 1: " + str(len(processed1)) + " records"

' Process chunk 2  
import "chunk_02.csv" into chunk2
dim processed2: table = query(
  SELECT 
    *,
    upper(name) as name_upper,
    lower(email) as email_lower,
    '2' as chunk_id
  FROM "chunk_02.csv"
)
export processed2 to "processed_chunk_02.csv"
total_processed = total_processed + len(processed2)
print "Processed chunk 2: " + str(len(processed2)) + " records"

' Process chunk 3
import "chunk_03.csv" into chunk3
dim processed3: table = query(
  SELECT 
    *,
    upper(name) as name_upper,
    lower(email) as email_lower,
    '3' as chunk_id
  FROM "chunk_03.csv"
)
export processed3 to "processed_chunk_03.csv"
total_processed = total_processed + len(processed3)
print "Processed chunk 3: " + str(len(processed3)) + " records"

print "Memory-efficient processing complete: " + str(total_processed) + " records"
```

## Best Practices

1. **Validate Early** - Check data quality at the extract phase
2. **Handle Errors Gracefully** - Separate good and bad records
3. **Use Incremental Loads** - Process only new/changed data when possible
4. **Log Everything** - Track record counts and processing times
5. **Test with Samples** - Validate pipeline logic with small datasets first
6. **Monitor Performance** - Watch memory usage with large files
7. **Document Transformations** - Comment complex business logic

## Next Steps

- [Data Import](import.md) - Advanced import techniques
- [Data Transformation](transform.md) - Complex transformations
- [Data Export](export.md) - Export strategies
