# JSON Transformation

PipTable provides powerful JSON processing capabilities, supporting both standard JSON and JSONL (JSON Lines) formats. Handle complex nested structures, API responses, and streaming data with ease.

## Reading JSON Files

### Basic JSON Import
```piptable
' @title Read JSON File
' @description Import data from a JSON file

dim data: table = import "data.json" into sheet
print "Loaded " + str(len(data)) + " records from JSON"
```

### Read JSONL (JSON Lines)
```piptable
' @title Process JSON Lines Format
' @description Read streaming JSON data line by line

' JSONL: One JSON object per line
dim stream_data: table = import "events.jsonl" into sheet

' Process streaming events
dim processed: table = query(
  SELECT 
    event_id,
    event_type,
    JSON_EXTRACT(payload, '$.user_id') as user_id,
    JSON_EXTRACT(payload, '$.amount') as amount,
    timestamp
  FROM stream_data
  WHERE event_type IN ('purchase', 'refund')
)

export processed to "processed_events.csv"
```

### Parse Nested JSON
```piptable
' @title Extract Nested JSON Data
' @description Handle complex nested JSON structures

dim nested_data: table = import "nested.json" into sheet

' Extract nested fields using JSON functions
dim flattened: table = query(
  SELECT 
    id,
    JSON_EXTRACT(data, '$.user.name') as user_name,
    JSON_EXTRACT(data, '$.user.email') as user_email,
    JSON_EXTRACT(data, '$.address.street') as street,
    JSON_EXTRACT(data, '$.address.city') as city,
    JSON_EXTRACT(data, '$.address.zip') as zip,
    JSON_ARRAY_LENGTH(JSON_EXTRACT(data, '$.orders')) as order_count
  FROM nested_data
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

' Process API response
dim users: table = query(
  SELECT 
    JSON_EXTRACT(data, '$.id') as user_id,
    JSON_EXTRACT(data, '$.attributes.name') as name,
    JSON_EXTRACT(data, '$.attributes.email') as email,
    JSON_EXTRACT(data, '$.attributes.created_at') as created_date,
    JSON_EXTRACT(data, '$.relationships.team.id') as team_id
  FROM api_response
)

' Get related data
dim teams: table = fetch("https://api.example.com/teams")

' Join user and team data
dim enriched: table = users left join teams on "team_id" = "id"

export enriched to "api_data.csv"
```

### Handle Pagination
```piptable
' @title API Pagination Handler
' @description Fetch all pages from paginated API

dim all_data: table = import "empty.csv" into sheet
dim page: int = 1
dim has_more: bool = true

while has_more
  ' Fetch page
  dim page_data: table = fetch(
    "https://api.example.com/data?page=" + str(page))
  
  ' Extract records array and flatten it
  dim records: table = query(
    SELECT record FROM (
      SELECT JSON_ARRAY_ELEMENTS(JSON_EXTRACT(data, '$.records')) as record
    )
  ) 
  
  ' Extract pagination metadata
  dim meta: table = query(
    SELECT JSON_EXTRACT(data, '$.has_more') as has_more
    FROM page_data
  )
  
  ' Append records to accumulated data
  if page = 1 then
    all_data = records
  else
    all_data append records
  end if
  
  ' Check for more pages (accessing first row's field)
  dim meta_row: row = meta[0]
  has_more = meta_row.has_more
  page = page + 1
  
  print "Fetched page " + str(page - 1) + ", total: " + str(len(all_data))
wend

export all_data to "complete_dataset.json"
```

## Transform JSON Structures

### JSON to Relational
```piptable
' @title Convert JSON to Relational Tables
' @description Normalize JSON into multiple related tables

dim json_data: table = import "orders.json" into sheet

' Extract main order data
dim orders: table = query(
  SELECT 
    order_id,
    customer_id,
    order_date,
    status,
    total_amount
  FROM json_data
)

' Extract order items as separate table
dim order_items: table = query(
  SELECT 
    order_id,
    JSON_EXTRACT(item, '$.product_id') as product_id,
    JSON_EXTRACT(item, '$.quantity') as quantity,
    JSON_EXTRACT(item, '$.price') as price
  FROM json_data,
  JSON_EACH(JSON_EXTRACT(json_data.data, '$.items')) as item
)

' Extract customer data
dim customers: table = query(
  SELECT DISTINCT
    customer_id,
    JSON_EXTRACT(data, '$.customer.name') as name,
    JSON_EXTRACT(data, '$.customer.email') as email
  FROM json_data
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

dim orders: table = import "orders.csv" into sheet
dim items: table = import "order_items.csv" into sheet

' Join and nest data
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
        FROM items
        WHERE items.order_id = orders.order_id
      )
    ) as order_json
  FROM orders
)

export nested_orders to "nested_orders.json"
```

## JSON Analytics

### Analyze JSON Logs
```piptable
' @title Analyze Application Logs
' @description Process JSON log files for insights

dim logs: table = import "app_logs.jsonl" into sheet

' Parse log entries
dim parsed_logs: table = query(
  SELECT 
    JSON_EXTRACT(data, '$.timestamp') as timestamp,
    JSON_EXTRACT(data, '$.level') as log_level,
    JSON_EXTRACT(data, '$.service') as service,
    JSON_EXTRACT(data, '$.message') as message,
    JSON_EXTRACT(data, '$.duration_ms') as duration_ms,
    JSON_EXTRACT(data, '$.user_id') as user_id
  FROM logs
)

' Error analysis
dim error_summary: table = query(
  SELECT 
    service,
    COUNT(*) as error_count,
    AVG(CAST(duration_ms AS FLOAT)) as avg_duration,
    COUNT(DISTINCT user_id) as affected_users
  FROM parsed_logs
  WHERE log_level = 'ERROR'
  GROUP BY service
  ORDER BY error_count DESC
)

' Performance analysis
dim slow_requests: table = query(
  SELECT * FROM parsed_logs
  WHERE CAST(duration_ms AS INT) > 1000
  ORDER BY duration_ms DESC
)

export error_summary to "error_analysis.csv"
export slow_requests to "performance_issues.csv"
```

### JSON Metrics Aggregation
```piptable
' @title Aggregate JSON Metrics
' @description Calculate statistics from JSON data

dim metrics: table = import "metrics.jsonl" into sheet

' Time-series aggregation
dim hourly_stats: table = query(
  SELECT 
    DATE_TRUNC('hour', JSON_EXTRACT(data, '$.timestamp')) as hour,
    JSON_EXTRACT(data, '$.metric_name') as metric,
    AVG(CAST(JSON_EXTRACT(data, '$.value') AS FLOAT)) as avg_value,
    MIN(CAST(JSON_EXTRACT(data, '$.value') AS FLOAT)) as min_value,
    MAX(CAST(JSON_EXTRACT(data, '$.value') AS FLOAT)) as max_value,
    COUNT(*) as sample_count
  FROM metrics
  GROUP BY hour, metric
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