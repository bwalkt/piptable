# JSON Transformation

PipTable provides powerful JSON processing capabilities, supporting both standard JSON and JSONL (JSON Lines) formats. Handle complex nested structures, API responses, and streaming data with ease.

## Reading JSON Files

### Basic JSON Import
```piptable
' @title Read JSON File
' @description Import data from a JSON file

DIM data AS SHEET = READ("data.json")
PRINT "Loaded " + STR(LEN(data)) + " records from JSON"
```

### Read JSONL (JSON Lines)
```piptable
' @title Process JSON Lines Format
' @description Read streaming JSON data line by line

' JSONL: One JSON object per line
DIM stream_data AS SHEET = READ("events.jsonl")

' Process streaming events
DIM processed AS SHEET = QUERY(stream_data,
  "SELECT 
    event_id,
    event_type,
    JSON_EXTRACT(payload, '$.user_id') as user_id,
    JSON_EXTRACT(payload, '$.amount') as amount,
    timestamp
   FROM stream_data
   WHERE event_type IN ('purchase', 'refund')")

WRITE(processed, "processed_events.csv")
```

### Parse Nested JSON
```piptable
' @title Extract Nested JSON Data
' @description Handle complex nested JSON structures

DIM nested_data AS SHEET = READ("nested.json")

' Extract nested fields using JSON functions
DIM flattened AS SHEET = QUERY(nested_data,
  "SELECT 
    id,
    JSON_EXTRACT(data, '$.user.name') as user_name,
    JSON_EXTRACT(data, '$.user.email') as user_email,
    JSON_EXTRACT(data, '$.address.street') as street,
    JSON_EXTRACT(data, '$.address.city') as city,
    JSON_EXTRACT(data, '$.address.zip') as zip,
    JSON_ARRAY_LENGTH(JSON_EXTRACT(data, '$.orders')) as order_count
   FROM nested_data")

WRITE(flattened, "flattened_data.csv")
```

## Working with API Responses

### Process API JSON
```piptable
' @title Fetch and Process API Data
' @description Get data from API and transform JSON response

' Fetch from REST API
DIM api_response AS SHEET = FETCH("https://api.example.com/users")

' Process API response
DIM users AS SHEET = QUERY(api_response,
  "SELECT 
    JSON_EXTRACT(data, '$.id') as user_id,
    JSON_EXTRACT(data, '$.attributes.name') as name,
    JSON_EXTRACT(data, '$.attributes.email') as email,
    JSON_EXTRACT(data, '$.attributes.created_at') as created_date,
    JSON_EXTRACT(data, '$.relationships.team.id') as team_id
   FROM api_response")

' Get related data
DIM teams AS SHEET = FETCH("https://api.example.com/teams")

' Join user and team data
DIM enriched AS SHEET = JOIN LEFT users, teams
  ON users.team_id = teams.id

WRITE(enriched, "api_data.csv")
```

### Handle Pagination
```piptable
' @title API Pagination Handler
' @description Fetch all pages from paginated API

DIM all_data AS SHEET = NEW SHEET()
DIM page AS INT = 1
DIM has_more AS BOOL = true

WHILE has_more
  ' Fetch page
  DIM page_data AS SHEET = FETCH(
    "https://api.example.com/data?page=" + STR(page))
  
  ' Extract records array and flatten it
  DIM records AS SHEET = QUERY(page_data,
    "SELECT record FROM (
       SELECT JSON_ARRAY_ELEMENTS(JSON_EXTRACT(data, '$.records')) as record
     )") 
  
  ' Extract pagination metadata
  DIM meta AS SHEET = QUERY(page_data,
    "SELECT JSON_EXTRACT(data, '$.has_more') as has_more")
  
  ' Append records to accumulated data
  IF page = 1 THEN
    all_data = records
  ELSE
    all_data APPEND records
  END IF
  
  ' Check for more pages (accessing first row's field)
  DIM meta_row AS ROW = meta[0]
  has_more = meta_row.has_more
  page = page + 1
  
  PRINT "Fetched page " + STR(page - 1) + ", total: " + STR(LEN(all_data))
WEND

WRITE(all_data, "complete_dataset.json")
```

## Transform JSON Structures

### JSON to Relational
```piptable
' @title Convert JSON to Relational Tables
' @description Normalize JSON into multiple related tables

DIM json_data AS SHEET = READ("orders.json")

' Extract main order data
DIM orders AS SHEET = QUERY(json_data,
  "SELECT 
    order_id,
    customer_id,
    order_date,
    status,
    total_amount
   FROM json_data")

' Extract order items as separate table
DIM order_items AS SHEET = QUERY(json_data,
  "SELECT 
    order_id,
    JSON_EXTRACT(item, '$.product_id') as product_id,
    JSON_EXTRACT(item, '$.quantity') as quantity,
    JSON_EXTRACT(item, '$.price') as price
   FROM json_data,
   JSON_EACH(JSON_EXTRACT(json_data.data, '$.items')) as item")

' Extract customer data
DIM customers AS SHEET = QUERY(json_data,
  "SELECT DISTINCT
    customer_id,
    JSON_EXTRACT(data, '$.customer.name') as name,
    JSON_EXTRACT(data, '$.customer.email') as email
   FROM json_data")

' Save normalized tables
WRITE(orders, "orders.csv")
WRITE(order_items, "order_items.csv") 
WRITE(customers, "customers.csv")
```

### Build JSON from Tables
```piptable
' @title Create JSON from Relational Data
' @description Combine tables into nested JSON structure

DIM orders AS SHEET = READ("orders.csv")
DIM items AS SHEET = READ("order_items.csv")

' Join and nest data
DIM nested_orders AS SHEET = QUERY(orders,
  "SELECT 
    order_id,
    customer_id,
    order_date,
    JSON_OBJECT(
      'order_id', order_id,
      'customer_id', customer_id,
      'date', order_date,
      'items', (
        SELECT JSON_GROUP_ARRAY(
          JSON_OBJECT(
            'product_id', product_id,
            'quantity', quantity,
            'price', price
          )
        )
        FROM items
        WHERE items.order_id = orders.order_id
      )
    ) as order_json
   FROM orders")

EXPORT nested_orders TO "nested_orders.json"
```

## JSON Analytics

### Analyze JSON Logs
```piptable
' @title Analyze Application Logs
' @description Process JSON log files for insights

DIM logs AS SHEET = READ("app_logs.jsonl")

' Parse log entries
DIM parsed_logs AS SHEET = QUERY(logs,
  "SELECT 
    JSON_EXTRACT(data, '$.timestamp') as timestamp,
    JSON_EXTRACT(data, '$.level') as log_level,
    JSON_EXTRACT(data, '$.service') as service,
    JSON_EXTRACT(data, '$.message') as message,
    JSON_EXTRACT(data, '$.duration_ms') as duration_ms,
    JSON_EXTRACT(data, '$.user_id') as user_id
   FROM logs")

' Error analysis
DIM error_summary AS SHEET = QUERY(parsed_logs,
  "SELECT 
    service,
    COUNT(*) as error_count,
    AVG(CAST(duration_ms AS FLOAT)) as avg_duration,
    COUNT(DISTINCT user_id) as affected_users
   FROM parsed_logs
   WHERE log_level = 'ERROR'
   GROUP BY service
   ORDER BY error_count DESC")

' Performance analysis
DIM slow_requests AS SHEET = QUERY(parsed_logs,
  "SELECT * FROM parsed_logs
   WHERE CAST(duration_ms AS INT) > 1000
   ORDER BY duration_ms DESC")

WRITE(error_summary, "error_analysis.csv")
WRITE(slow_requests, "performance_issues.csv")
```

### JSON Metrics Aggregation
```piptable
' @title Aggregate JSON Metrics
' @description Calculate statistics from JSON data

DIM metrics AS SHEET = READ("metrics.jsonl")

' Time-series aggregation
DIM hourly_stats AS SHEET = QUERY(metrics,
  "SELECT 
    DATE_TRUNC('hour', JSON_EXTRACT(data, '$.timestamp')) as hour,
    JSON_EXTRACT(data, '$.metric_name') as metric,
    AVG(CAST(JSON_EXTRACT(data, '$.value') AS FLOAT)) as avg_value,
    MIN(CAST(JSON_EXTRACT(data, '$.value') AS FLOAT)) as min_value,
    MAX(CAST(JSON_EXTRACT(data, '$.value') AS FLOAT)) as max_value,
    COUNT(*) as sample_count
   FROM metrics
   GROUP BY hour, metric
   ORDER BY hour, metric")

WRITE(hourly_stats, "metrics_summary.csv")
```

## Performance Tips

1. **Use JSONL for large datasets** - Streaming format is more memory efficient
2. **Extract only needed fields** - Don't parse entire JSON if you only need specific fields
3. **Cache parsed results** - Save extracted data to avoid re-parsing
4. **Use JSON functions** - Leverage SQL JSON functions for complex operations
5. **Validate JSON structure** - Check for expected fields before processing

## Next Steps

- [HTTP APIs](http-api.md) - Working with REST APIs
- [Database Queries](database.md) - Database integration
- [Python Integration](python.md) - Advanced processing with Python