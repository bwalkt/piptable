# ETL Pipelines

Build robust Extract, Transform, Load (ETL) pipelines using PipTable's powerful data processing capabilities. These examples show complete workflows from data ingestion to final output.

## Complete ETL Pipeline

### Sales Data Pipeline
```piptable
' @title Complete Sales ETL Pipeline
' @description Extract from multiple sources, transform, and load to reporting database

' EXTRACT: Load data from multiple sources
DIM raw_sales AS SHEET = READ("sales_transactions.csv")
DIM customers AS SHEET = READ("customers.xlsx") 
DIM products AS SHEET = READ("products.json")

' TRANSFORM: Clean and enrich data
' Step 1: Clean sales data
DIM cleaned_sales AS SHEET = QUERY(raw_sales,
  "SELECT 
    transaction_id,
    UPPER(TRIM(customer_id)) as customer_id,
    product_id,
    CAST(quantity AS INT) as quantity,
    CAST(unit_price AS FLOAT) as unit_price,
    DATE(transaction_date) as sale_date
   FROM raw_sales
   WHERE quantity > 0 AND unit_price > 0")

' Step 2: Join with customer data
DIM sales_with_customer AS SHEET = JOIN LEFT cleaned_sales, customers
  ON cleaned_sales.customer_id = customers.id

' Step 3: Join with product data
DIM enriched_sales AS SHEET = JOIN LEFT sales_with_customer, products
  ON sales_with_customer.product_id = products.id

' Step 4: Calculate metrics
DIM final_sales AS SHEET = QUERY(enriched_sales,
  "SELECT 
    transaction_id,
    sale_date,
    customer_name,
    customer_segment,
    product_name,
    category,
    quantity,
    unit_price,
    quantity * unit_price as total_amount,
    quantity * unit_price * 0.15 as profit_margin
   FROM enriched_sales")

' LOAD: Export to different formats
WRITE(final_sales, "processed_sales.csv")
EXPORT final_sales TO "sales_report.xlsx"

PRINT "ETL Pipeline Complete: " + STR(LEN(final_sales)) + " records processed"
```

## Daily Data Processing

### Incremental Data Load
```piptable
' @title Incremental Daily Load
' @description Process only new records since last run

' Load existing data
DIM existing_data AS SHEET = READ("master_database.csv")

' Get today's new data
DIM todays_data AS SHEET = READ("daily_extract_" + STR(CURRENT_DATE) + ".csv")

' Find new records (not in existing)
DIM new_records AS SHEET = QUERY(todays_data,
  "SELECT t.* 
   FROM todays_data t
   LEFT JOIN existing_data e ON t.id = e.id
   WHERE e.id IS NULL")

IF LEN(new_records) > 0 THEN
  ' Process new records
  DIM processed_new AS SHEET = QUERY(new_records,
    "SELECT 
      *,
      CURRENT_DATE as processed_date,
      'PENDING' as status
     FROM new_records")
  
  ' Append to master
  existing_data APPEND processed_new
  
  ' Save updated master
  WRITE(existing_data, "master_database.csv")
  PRINT "Added " + STR(LEN(new_records)) + " new records"
ELSE
  PRINT "No new records to process today"
END IF
```

## Data Quality Pipeline

### Validation and Error Handling
```piptable
' @title Data Quality Pipeline
' @description Validate, clean, and separate good/bad records

DIM raw_data AS SHEET = READ("raw_input.csv")

' Define validation rules
DIM valid_records AS SHEET = QUERY(raw_data,
  "SELECT * FROM raw_data
   WHERE 
    email LIKE '%@%.%' AND
    age BETWEEN 18 AND 120 AND
    phone REGEXP '^[0-9]{10}$' AND
    country IN ('US', 'CA', 'UK', 'AU')")

DIM invalid_records AS SHEET = QUERY(raw_data,
  "SELECT * FROM raw_data
   WHERE 
    email NOT LIKE '%@%.%' OR
    age NOT BETWEEN 18 AND 120 OR
    phone NOT REGEXP '^[0-9]{10}$' OR
    country NOT IN ('US', 'CA', 'UK', 'AU')")

' Process valid records
DIM processed AS SHEET = QUERY(valid_records,
  "SELECT 
    LOWER(email) as email,
    INITCAP(name) as name,
    age,
    phone,
    country,
    CURRENT_TIMESTAMP as processed_at
   FROM valid_records")

' Export results
WRITE(processed, "clean_data.csv")
WRITE(invalid_records, "rejected_records.csv")

' Generate quality report
DIM quality_report AS SHEET = QUERY(raw_data,
  "SELECT 
    COUNT(*) as total_records,
    " + STR(LEN(valid_records)) + " as valid_count,
    " + STR(LEN(invalid_records)) + " as invalid_count,
    ROUND(" + STR(LEN(valid_records)) + " * 100.0 / COUNT(*), 2) as success_rate
   FROM raw_data")

WRITE(quality_report, "quality_report.csv")
PRINT "Data Quality Check: " + STR(LEN(valid_records)) + "/" + STR(LEN(raw_data)) + " records passed"
```

## Multi-Source Integration

### Merge Heterogeneous Data
```piptable
' @title Multi-Source Data Integration
' @description Combine CSV, Excel, JSON, and API data

' Load from different sources
DIM csv_data AS SHEET = READ("source1.csv")
DIM excel_data AS SHEET = READ("source2.xlsx")
DIM json_data AS SHEET = READ("source3.json")

' Fetch from API
DIM api_data AS SHEET = FETCH("https://api.example.com/data")

' Standardize column names
DIM std_csv AS SHEET = QUERY(csv_data,
  "SELECT 
    customer_id as id,
    customer_name as name,
    email_address as email,
    'CSV' as source
   FROM csv_data")

DIM std_excel AS SHEET = QUERY(excel_data,
  "SELECT 
    cust_id as id,
    full_name as name,
    email,
    'EXCEL' as source
   FROM excel_data")

DIM std_json AS SHEET = QUERY(json_data,
  "SELECT 
    id,
    name,
    contact_email as email,
    'JSON' as source
   FROM json_data")

DIM std_api AS SHEET = QUERY(api_data,
  "SELECT 
    userId as id,
    displayName as name,
    emailAddress as email,
    'API' as source
   FROM api_data")

' Combine all sources
DIM all_sources AS SHEET = std_csv
all_sources APPEND std_excel
all_sources APPEND std_json
all_sources APPEND std_api

' Deduplicate based on email
DIM unique_customers AS SHEET = QUERY(all_sources,
  "SELECT 
    MIN(id) as id,
    FIRST(name) as name,
    email,
    STRING_AGG(source, ',') as sources
   FROM all_sources
   GROUP BY email")

WRITE(unique_customers, "integrated_customers.csv")
PRINT "Integrated " + STR(LEN(unique_customers)) + " unique customers from 4 sources"
```

## Scheduled Reporting

### Automated Report Generation
```piptable
' @title Automated Daily Report
' @description Generate and distribute daily reports

' Load today's data
DIM todays_transactions AS SHEET = QUERY(
  READ("transactions.csv"),
  "SELECT * FROM transactions 
   WHERE DATE(transaction_date) = CURRENT_DATE")

' Generate summary metrics
DIM daily_summary AS SHEET = QUERY(todays_transactions,
  "SELECT 
    CURRENT_DATE as report_date,
    COUNT(*) as total_transactions,
    COUNT(DISTINCT customer_id) as unique_customers,
    SUM(amount) as total_revenue,
    AVG(amount) as avg_transaction_value,
    MAX(amount) as largest_transaction
   FROM todays_transactions")

' Top products
DIM top_products AS SHEET = QUERY(todays_transactions,
  "SELECT 
    product_name,
    COUNT(*) as units_sold,
    SUM(amount) as revenue
   FROM todays_transactions
   GROUP BY product_name
   ORDER BY revenue DESC
   LIMIT 10")

' Hourly breakdown
DIM hourly_sales AS SHEET = QUERY(todays_transactions,
  "SELECT 
    EXTRACT(HOUR FROM transaction_date) as hour,
    COUNT(*) as transactions,
    SUM(amount) as revenue
   FROM todays_transactions
   GROUP BY hour
   ORDER BY hour")

' Create report workbook
DIM report AS BOOK = NEW BOOK()
report.summary = daily_summary
report.top_products = top_products
report.hourly_breakdown = hourly_sales
report.all_transactions = todays_transactions

' Save report
WRITE(report, "daily_report_" + STR(CURRENT_DATE) + ".xlsx")
PRINT "Daily report generated with " + STR(LEN(todays_transactions)) + " transactions"
```

## Performance Optimization

### Batch Processing
```piptable
' @title Query-Based Batch Processing
' @description Process datasets using SQL LIMIT/OFFSET batching
' WARNING: This example loads the entire file into memory on each iteration!
' See "Memory-Efficient Processing" section below for better approaches

' Define batch size
DIM batch_size AS INT = 10000
DIM offset AS INT = 0
DIM total_processed AS INT = 0

' Process in batches
WHILE true
  ' Read batch
  DIM batch AS SHEET = QUERY(
    READ("large_file.csv"),
    "SELECT * FROM large_file 
     LIMIT " + STR(batch_size) + " 
     OFFSET " + STR(offset))
  
  ' Check if batch is empty
  IF LEN(batch) = 0 THEN
    EXIT WHILE
  END IF
  
  ' Process batch
  DIM processed_batch AS SHEET = QUERY(batch,
    "SELECT 
      *,
      UPPER(name) as name_upper,
      LOWER(email) as email_lower
     FROM batch")
  
  ' Append to output
  IF offset = 0 THEN
    WRITE(processed_batch, "output.csv")
  ELSE
    DIM existing AS SHEET = READ("output.csv")
    existing APPEND processed_batch
    WRITE(existing, "output.csv")
  END IF
  
  total_processed = total_processed + LEN(batch)
  offset = offset + batch_size
  
  PRINT "Processed " + STR(total_processed) + " records..."
WEND

PRINT "Batch processing complete: " + STR(total_processed) + " total records"
```

### Memory-Efficient Processing
```piptable
' @title Memory-Efficient Data Processing  
' @description Process large datasets without loading entire file repeatedly
' Note: Use file chunks or streaming approaches for very large datasets

' Method 1: Process pre-split file chunks
DIM chunk_num AS INT = 1
DIM total_processed AS INT = 0
DIM all_results AS SHEET = NEW SHEET()

' Process pre-split chunks (split externally: split -l 10000 large_file.csv chunk_)
WHILE true
  ' Read next chunk file
  DIM chunk_file AS TEXT = "chunk_" + STR(chunk_num) + ".csv"
  DIM batch AS SHEET = read(chunk_file)
  
  IF LEN(batch) = 0 THEN
    EXIT WHILE
  END IF
  
  ' Transform batch
  DIM processed AS SHEET = QUERY(batch,
    "SELECT 
      *,
      UPPER(name) as name_upper,
      LOWER(email) as email_lower,
      " + STR(chunk_num) + " as chunk_id
     FROM batch")
  
  ' Accumulate results or write to separate files
  IF chunk_num = 1 THEN
    all_results = processed
  ELSE
    all_results APPEND processed
  END IF
  
  total_processed = total_processed + LEN(batch)
  chunk_num = chunk_num + 1
  
  PRINT "Processed chunk " + STR(chunk_num - 1) + ": " + STR(total_processed) + " total records"
WEND

WRITE(all_results, "final_processed.csv")
PRINT "Memory-efficient processing complete: " + STR(total_processed) + " records"
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