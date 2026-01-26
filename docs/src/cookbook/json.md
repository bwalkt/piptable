# JSON Transformation

PipTable provides powerful JSON processing capabilities, supporting both standard JSON and JSONL (JSON Lines) formats. Handle complex nested structures, API responses, and streaming data with ease.

## Reading JSON Files

### Basic JSON Import
```piptable
' @title Read JSON File
' @description Import data from a JSON file

import "data.json" into data
print "Loaded " + str(len(data)) + " records from JSON"
```

### Read JSONL (JSON Lines)
```piptable
' @title Process JSON Lines Format
' @description Read streaming JSON data line by line

' JSONL: One JSON object per line
import "events.jsonl" into stream_data

' Process streaming events using file reference
dim processed: table = query(
  SELECT 
    event_id,
    event_type,
    JSON_EXTRACT(payload, '$.user_id') as user_id,
    JSON_EXTRACT(payload, '$.amount') as amount,
    timestamp
  FROM "events.jsonl"
  WHERE event_type IN ('purchase', 'refund')
)

export processed to "processed_events.csv"
```

### Parse Nested JSON
```piptable
' @title Extract Nested JSON Data
' @description Handle complex nested JSON structures

import "nested.json" into nested_data

' Extract nested fields using JSON functions with file reference
dim flattened: table = query(
  SELECT 
    id,
    JSON_EXTRACT(data, '$.user.name') as user_name,
    JSON_EXTRACT(data, '$.user.email') as user_email,
    JSON_EXTRACT(data, '$.address.street') as street,
    JSON_EXTRACT(data, '$.address.city') as city,
    JSON_EXTRACT(data, '$.address.zip') as zip,
    JSON_ARRAY_LENGTH(JSON_EXTRACT(data, '$.orders')) as order_count
  FROM "nested.json"
)

export flattened to "flattened_data.csv"
```

## Working with API Responses

### Process API JSON
```piptable
' @title Fetch and Process API Data
' @description Get data from API and transform JSON response

' Fetch from REST API
dim api_response: table = fetch("https://api.example.com/users")
dim teams: table = fetch("https://api.example.com/teams")

' Join user and team data (fetch results can be joined directly)
dim enriched: table = api_response left join teams on "team_id" = "id"

export enriched to "api_data.csv"
```

### Handle Pagination
```piptable
' @title API Pagination Handler
' @description Fetch all pages from paginated API

' Pagination example - fetch first 3 pages
dim page1: table = fetch("https://api.example.com/data?page=1")
dim page2: table = fetch("https://api.example.com/data?page=2") 
dim page3: table = fetch("https://api.example.com/data?page=3")

' Combine pages
page1 append page2
page1 append page3

print "Fetched 3 pages, total: " + str(len(page1)) + " records"

export all_data to "complete_dataset.json"
```

## Transform JSON Structures

### JSON to Relational
```piptable
' @title Convert JSON to Relational Tables
' @description Normalize JSON into multiple related tables

import "orders.json" into json_data

' Extract main order data using file reference
dim orders: table = query(
  SELECT 
    order_id,
    customer_id,
    order_date,
    status,
    total_amount
  FROM "orders.json"
)

' Extract order items as separate table
dim order_items: table = query(
  SELECT 
    order_id,
    JSON_EXTRACT(item, '$.product_id') as product_id,
    JSON_EXTRACT(item, '$.quantity') as quantity,
    JSON_EXTRACT(item, '$.price') as price
  FROM "orders.json",
  JSON_EACH(JSON_EXTRACT(data, '$.items')) as item
)

' Extract customer data
dim customers: table = query(
  SELECT DISTINCT
    customer_id,
    JSON_EXTRACT(data, '$.customer.name') as name,
    JSON_EXTRACT(data, '$.customer.email') as email
  FROM "orders.json"
)

' Save normalized tables
export orders to "orders.csv"
export order_items to "order_items.csv" 
export customers to "customers.csv"
```

### Build JSON from Tables
```piptable
' @title Create JSON from Relational Data
' @description Combine tables into nested JSON structure

import "orders.csv" into orders
import "order_items.csv" into items

' Join and nest data using file references
dim nested_orders: table = query(
  SELECT 
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
        FROM "order_items.csv" i
        WHERE i.order_id = o.order_id
      )
    ) as order_json
  FROM "orders.csv" o
)

export nested_orders to "nested_orders.json"
```

## JSON Analytics

### Analyze JSON Logs
```piptable
' @title Analyze Application Logs
' @description Process JSON log files for insights

import "app_logs.jsonl" into logs

' Parse and analyze log entries using file references
dim error_summary: table = query(
  SELECT 
    JSON_EXTRACT(data, '$.service') as service,
    COUNT(*) as error_count,
    AVG(CAST(JSON_EXTRACT(data, '$.duration_ms') AS FLOAT)) as avg_duration,
    COUNT(DISTINCT JSON_EXTRACT(data, '$.user_id')) as affected_users
  FROM "app_logs.jsonl"
  WHERE JSON_EXTRACT(data, '$.level') = 'ERROR'
  GROUP BY JSON_EXTRACT(data, '$.service')
  ORDER BY error_count DESC
)

' Performance analysis
dim slow_requests: table = query(
  SELECT 
    JSON_EXTRACT(data, '$.timestamp') as timestamp,
    JSON_EXTRACT(data, '$.service') as service,
    JSON_EXTRACT(data, '$.message') as message,
    JSON_EXTRACT(data, '$.duration_ms') as duration_ms
  FROM "app_logs.jsonl"
  WHERE CAST(JSON_EXTRACT(data, '$.duration_ms') AS INT) > 1000
  ORDER BY JSON_EXTRACT(data, '$.duration_ms') DESC
)

export error_summary to "error_analysis.csv"
export slow_requests to "performance_issues.csv"
```

### JSON Metrics Aggregation
```piptable
' @title Aggregate JSON Metrics
' @description Calculate statistics from JSON data

import "metrics.jsonl" into metrics

' Time-series aggregation using file reference
dim hourly_stats: table = query(
  SELECT 
    DATE_TRUNC('hour', JSON_EXTRACT(data, '$.timestamp')) as hour,
    JSON_EXTRACT(data, '$.metric_name') as metric,
    AVG(CAST(JSON_EXTRACT(data, '$.value') AS FLOAT)) as avg_value,
    MIN(CAST(JSON_EXTRACT(data, '$.value') AS FLOAT)) as min_value,
    MAX(CAST(JSON_EXTRACT(data, '$.value') AS FLOAT)) as max_value,
    COUNT(*) as sample_count
  FROM "metrics.jsonl"
  GROUP BY DATE_TRUNC('hour', JSON_EXTRACT(data, '$.timestamp')), JSON_EXTRACT(data, '$.metric_name')
  ORDER BY hour, metric
)

export hourly_stats to "metrics_summary.csv"
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