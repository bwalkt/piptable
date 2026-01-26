# ETL Pipelines

Build robust Extract, Transform, Load (ETL) pipelines using PipTable's powerful data processing capabilities. These examples show complete workflows from data ingestion to final output.

## Complete ETL Pipeline

### Sales Data Pipeline
```piptable
' @title Complete Sales ETL Pipeline
' @description Extract from multiple sources, transform, and load to reporting database

' EXTRACT: Load data from multiple sources
dim raw_sales: table = import "sales_transactions.csv" into sheet
dim customers: table = import "customers.csv" into sheet  
dim products: table = import "products.json" into sheet

' TRANSFORM: Clean and enrich data
' Step 1: Clean sales data using query expression
dim cleaned_sales: table = query(
  SELECT 
    transaction_id,
    upper(trim(customer_id)) as customer_id,
    product_id,
    cast(quantity as int) as quantity,
    cast(unit_price as float) as unit_price,
    transaction_date as sale_date
  FROM "sales_transactions.csv"
  WHERE quantity > 0 AND unit_price > 0
)

' Step 2: Join with customer data
dim sales_with_customer: table = 
  cleaned_sales left join customers on "customer_id" = "id"

' Step 3: Join with product data
dim enriched_sales: table = 
  sales_with_customer left join products on "product_id" = "id"

' Step 4: Calculate metrics
dim final_sales: table = query(
  SELECT 
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
  FROM enriched_sales
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
dim existing_data: table = import "master_database.csv" into sheet

' Get today's new data (would need date formatting function)
dim todays_data: table = import "daily_extract.csv" into sheet

' Find new records (not in existing) using query
dim new_records: table = query(
  SELECT t.* 
  FROM todays_data t
  LEFT JOIN existing_data e ON t.id = e.id
  WHERE e.id IS NULL
)

if len(new_records) > 0 then
  ' Process new records
  dim processed_new: table = query(
    SELECT 
      *,
      'today' as processed_date,
      'PENDING' as status
    FROM new_records
  )
  
  ' Append to master
  existing_data append processed_new
  
  ' Save updated master
  export existing_data to "master_database.csv"
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

dim raw_data: table = import "raw_input.csv" into sheet

' Define validation rules using query
dim valid_records: table = query(
  SELECT * FROM raw_data
  WHERE 
    email LIKE '%@%.%' AND
    age BETWEEN 18 AND 120 AND
    phone LIKE '__________' AND
    country IN ('US', 'CA', 'UK', 'AU')
)

dim invalid_records: table = query(
  SELECT * FROM raw_data
  WHERE 
    email NOT LIKE '%@%.%' OR
    age NOT BETWEEN 18 AND 120 OR
    phone NOT LIKE '__________' OR
    country NOT IN ('US', 'CA', 'UK', 'AU')
)

' Process valid records
dim processed: table = query(
  SELECT 
    lower(email) as email,
    name,
    age,
    phone,
    country,
    'processed' as status
  FROM valid_records
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
' @description Combine CSV, Excel, JSON, and API data

' Load from different sources
dim csv_data: table = import "source1.csv" into sheet
dim excel_data: table = import "source2.xlsx" into sheet
dim json_data: table = import "source3.json" into sheet

' Fetch from API
dim api_data: table = fetch("https://api.example.com/data")

' Standardize column names
dim std_csv: table = query(
  SELECT 
    customer_id as id,
    customer_name as name,
    email_address as email,
    'CSV' as source
  FROM csv_data
)

dim std_excel: table = query(
  SELECT 
    cust_id as id,
    full_name as name,
    email,
    'EXCEL' as source
  FROM excel_data
)

dim std_json: table = query(
  SELECT 
    id,
    name,
    contact_email as email,
    'JSON' as source
  FROM json_data
)

dim std_api: table = query(
  SELECT 
    userId as id,
    displayName as name,
    emailAddress as email,
    'API' as source
  FROM api_data
)

' Combine all sources
dim all_sources: table = std_csv
all_sources append std_excel
all_sources append std_json
all_sources append std_api

' Deduplicate based on email
dim unique_customers: table = query(
  SELECT 
    MIN(id) as id,
    FIRST(name) as name,
    email,
    STRING_AGG(source, ',') as sources
  FROM all_sources
  GROUP BY email
)

export unique_customers to "integrated_customers.csv"
print "Integrated " + str(len(unique_customers)) + " unique customers from 4 sources"
```

## Scheduled Reporting

### Automated Report Generation
```piptable
' @title Automated Daily Report
' @description Generate and distribute daily reports

' Load and filter today's data
dim transactions: table = import "transactions.csv" into sheet

dim todays_transactions: table = query(
  SELECT * FROM transactions 
  WHERE transaction_date = 'today'
)

' Generate summary metrics
dim daily_summary: table = query(
  SELECT 
    'today' as report_date,
    COUNT(*) as total_transactions,
    COUNT(DISTINCT customer_id) as unique_customers,
    SUM(amount) as total_revenue,
    AVG(amount) as avg_transaction_value,
    MAX(amount) as largest_transaction
  FROM todays_transactions
)

' Top products
dim top_products: table = query(
  SELECT 
    product_name,
    COUNT(*) as units_sold,
    SUM(amount) as revenue
  FROM todays_transactions
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
  FROM todays_transactions
  GROUP BY hour
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

' Load data once
dim large_data: table = import "large_file.csv" into sheet

while offset < len(large_data)
  ' Process batch using query with LIMIT/OFFSET
  dim batch: table = query(
    SELECT * FROM large_data
    LIMIT batch_size
    OFFSET offset
  )
  
  ' Check if batch is empty
  if len(batch) = 0 then
    exit while
  end if
  
  ' Process batch
  dim processed_batch: table = query(
    SELECT 
      *,
      upper(name) as name_upper,
      lower(email) as email_lower
    FROM batch
  )
  
  ' Export batch (append mode would be ideal but not directly supported)
  if offset = 0 then
    export processed_batch to "output.csv"
  else
    ' This is inefficient - ideally would append directly
    dim existing: table = import "output.csv" into sheet
    existing append processed_batch
    export existing to "output.csv"
  end if
  
  total_processed = total_processed + len(batch)
  offset = offset + batch_size
  
  print "Processed " + str(total_processed) + " records..."
wend

print "Batch processing complete: " + str(total_processed) + " total records"
```

### Memory-Efficient Processing
```piptable
' @title Memory-Efficient Data Processing  
' @description Process large datasets without loading entire file repeatedly
' Note: Use file chunks or streaming approaches for very large datasets

' Method 1: Process pre-split file chunks
dim chunk_num: int = 1
dim total_processed: int = 0

' Process pre-split chunks (split externally: split -l 10000 large_file.csv chunk_)
while true
  ' Build chunk filename
  dim chunk_file: string = "chunk_" + str(chunk_num) + ".csv"
  
  ' Try to import chunk (will fail if file doesn't exist)
  ' Note: Need error handling which may not be available
  dim batch: table = import chunk_file into sheet
  
  if len(batch) = 0 then
    exit while
  end if
  
  ' Transform batch
  dim processed: table = query(
    SELECT 
      *,
      upper(name) as name_upper,
      lower(email) as email_lower,
      chunk_num as chunk_id
    FROM batch
  )
  
  ' Export chunk results
  dim output_file: string = "processed_chunk_" + str(chunk_num) + ".csv"
  export processed to output_file
  
  total_processed = total_processed + len(batch)
  chunk_num = chunk_num + 1
  
  print "Processed chunk " + str(chunk_num - 1) + ": " + str(total_processed) + " total records"
wend

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