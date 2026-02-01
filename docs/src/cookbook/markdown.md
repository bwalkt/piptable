# Working with Markdown

PipTable supports importing tables from Markdown documents, making it easy to work with documentation, README files, and other markdown-formatted data.

## Importing Markdown Tables

### Basic Import

```piptable
' Import all tables from a markdown file
dim tables = import "README.md" into book

' Access individual tables
dim first_table = tables["table_1"]
dim second_table = tables["table_2"]
```

### With Headers

```piptable
' Import with header detection
dim docs = import "documentation.md" with {"has_headers": true} into book

' First row becomes column names
dim features = docs["table_1"]
print(features.column_names())  ' ["Feature", "Status", "Description"]
```

## Examples

### Documentation to Data

Extract data tables from documentation:

```piptable
' Import feature comparison from docs
dim features = import "docs/features.md" into book
dim comparison = features["table_1"]

' Analyze feature coverage
dim coverage = query("
    SELECT feature, 
           SUM(CASE WHEN status = '✅' THEN 1 ELSE 0 END) as implemented,
           SUM(CASE WHEN status = '🚧' THEN 1 ELSE 0 END) as in_progress,
           SUM(CASE WHEN status = '❌' THEN 1 ELSE 0 END) as not_started
    FROM comparison
    GROUP BY feature
")

export coverage to "feature_status.xlsx"
```

### GitHub README Analysis

Process tables from GitHub README files:

```piptable
' Import README
dim readme = import "README.md" into book

' Common README table patterns
dim contributors = readme["table_1"]  ' Often contributor table
dim badges = readme["table_2"]        ' Status badges table

' Analyze contributors
dim top_contributors = query("
    SELECT name, commits, additions, deletions
    FROM contributors
    ORDER BY commits DESC
    LIMIT 10
")
```

### Report Generation from Markdown

Convert markdown reports to structured data:

```piptable
' Import weekly report
dim report = import "reports/week-45.md" into book

' Extract metrics table
dim metrics = report["table_1"]

' Generate summary with comparisons
dim summary = query("
    SELECT 
        metric,
        current_value,
        previous_value,
        ROUND((current_value - previous_value) / previous_value * 100, 2) as change_pct,
        CASE 
            WHEN (current_value - previous_value) > 0 THEN '📈'
            WHEN (current_value - previous_value) < 0 THEN '📉'
            ELSE '➡️'
        END as trend
    FROM metrics
    ORDER BY ABS(change_pct) DESC
")

export summary to "metrics-summary.xlsx"
```

### API Documentation Parser

Extract API endpoints from markdown docs:

```piptable
' Import API documentation
dim api_docs = import "api-reference.md" into book

' Process endpoint table
dim endpoints = api_docs["table_1"]

' Generate OpenAPI spec data
dim api_spec = query("
    SELECT 
        endpoint,
        method,
        description,
        'application/json' as content_type,
        CASE method
            WHEN 'GET' THEN 200
            WHEN 'POST' THEN 201
            WHEN 'PUT' THEN 200
            WHEN 'DELETE' THEN 204
        END as success_code
    FROM endpoints
    WHERE deprecated != 'true'
")

export api_spec to "openapi-endpoints.json"
```

## Markdown Table Features

### Type Inference

Markdown tables automatically detect data types:

| Feature | Example | Detected Type |
|---------|---------|---------------|
| Integers | `42` | Int |
| Decimals | `3.14` | Float |
| Booleans | `true` | Bool |
| Nulls | `null`, `N/A` | Null |
| Text | `hello` | String |
| Emojis | `✅`, `❌` | String |

### Inline Formatting

Markdown formatting is preserved as text:

```piptable
' Markdown with formatting
' | Style | **Bold** | *Italic* | `Code` |
' Results in: ["Style", "Bold", "Italic", "Code"]

dim formatted = import "styled.md" into book
dim data = formatted["table_1"]

' Clean formatting if needed
dim clean = data.map_column("Description", lambda(x) {
    ' Remove markdown formatting
    x.replace("**", "").replace("*", "").replace("`", "")
})
```

### Multiple Tables

Documents can contain multiple tables:

```piptable
' Process all tables in a document
dim doc = import "analysis.md" into book

' Count and process each table
dim table_count = 0
for i = 1 to 100  ' Check up to 100 tables
    dim table_name = "table_" + str(i)
    if doc.has_key(table_name) then
        table_count = table_count + 1
        dim table = doc[table_name]
        print("Table " + str(i) + ": " + str(table.row_count()) + " rows, " + str(table.col_count()) + " columns")
    else
        break  ' No more tables
    end if
next

print("Total tables found: " + str(table_count))
```

## Best Practices

### 1. Table Detection

Markdown tables must have proper formatting:

```markdown
| Column A | Column B | Column C |
|----------|----------|----------|
| Data 1   | Data 2   | Data 3   |
```

Requirements:
- Pipe delimiters (`|`)
- Header separator row with dashes (`---`)
- Consistent column count

### 2. Data Cleaning

```piptable
' Clean imported markdown data
dim raw = import "data.md" into book
dim table = raw["table_1"]

' Remove empty rows
dim cleaned = query("
    SELECT * FROM table 
    WHERE NOT (col1 IS NULL AND col2 IS NULL AND col3 IS NULL)
")

' Standardize values
dim normalized = cleaned.map_column("Status", lambda(x) {
    if x == "✅" or x == "Done" or x == "Complete" then
        return "Complete"
    elseif x == "❌" or x == "Failed" or x == "Incomplete" then
        return "Incomplete"
    elseif x == "🚧" or x == "WIP" or x == "In Progress" then
        return "In Progress"
    else
        return x
    end if
})
```

### 3. Header Detection

First row is automatically used as headers when specified:

```piptable
dim data = import "table.md" with {"has_headers": true} into book
dim sheet = data["table_1"]

' Access by column name
dim names = sheet.column("Name")
dim values = sheet.column("Value")
```

## Integration Examples

### With SQL Queries

```piptable
' Import and query markdown tables
dim docs = import "documentation.md" into book
dim api_table = docs["table_1"]

' Complex SQL analysis
dim analysis = query("
    WITH endpoint_stats AS (
        SELECT 
            endpoint,
            method,
            COUNT(*) OVER (PARTITION BY method) as method_count,
            LENGTH(endpoint) as path_length
        FROM api_table
    )
    SELECT 
        method,
        COUNT(*) as endpoint_count,
        AVG(path_length) as avg_path_length,
        MAX(path_length) as max_path_length
    FROM endpoint_stats
    GROUP BY method
    ORDER BY endpoint_count DESC
")
```

### With Formulas

```piptable
' Apply formulas to markdown data
dim prices = import "pricing.md" into book
dim price_table = prices["table_1"]

' Add calculated columns
price_table.add_column("Total", formula("Quantity * Price"))
price_table.add_column("Tax", formula("Total * 0.08"))
price_table.add_column("Final", formula("Total + Tax"))

' Export enhanced table
export price_table to "pricing_calculated.xlsx"
```

### Batch Processing

```piptable
' Process multiple markdown files
dim files = ["jan.md", "feb.md", "mar.md", "apr.md"]
dim all_data = new Book()

for file in files
    dim month_data = import file into book
    
    ' Assume first table contains metrics
    dim metrics = month_data["table_1"]
    
    ' Add month identifier
    dim month = file.replace(".md", "")
    metrics.add_column("Month", month)
    
    ' Add to combined book
    all_data.add_sheet(metrics, month)
next

' Export combined data
export all_data to "quarterly_metrics.xlsx"
```

## Error Handling

```piptable
try
    dim tables = import "document.md" into book
    
    if tables.sheet_count() == 0 then
        print("No tables found in document")
        ' Try alternative source
        dim backup = import "document.csv" into sheet
    else
        print("Found " + str(tables.sheet_count()) + " tables")
    end if
    
catch error
    print("Failed to parse markdown: " + error.message)
    
    ' Log error for debugging
    dim error_log = [
        ["timestamp", "file", "error"],
        [now(), "document.md", error.message]
    ]
    export error_log to "import_errors.csv" with {"append": true}
end try
```

## Advanced Techniques

### Dynamic Column Detection

```piptable
' Handle tables with varying columns
dim doc = import "dynamic.md" into book
dim table = doc["table_1"]

' Detect column patterns
dim columns = table.column_names()
dim has_status = false
dim has_date = false

for col in columns
    if col.contains("Status") or col.contains("State") then
        has_status = true
    end if
    if col.contains("Date") or col.contains("Time") then
        has_date = true
    end if
next

' Process based on detected columns
if has_status then
    dim status_summary = query("
        SELECT Status, COUNT(*) as count
        FROM table
        GROUP BY Status
    ")
end if
```

### Markdown Generation

While export is pending, you can prepare data for markdown:

```piptable
' Prepare data for markdown export
dim data = import "source.csv" into sheet

' Format for markdown (manual for now)
dim formatted = query("
    SELECT 
        '| ' || name || ' | ' || 
        status || ' | ' || 
        description || ' |' as markdown_row
    FROM data
")

' Headers and separator
dim header = "| Name | Status | Description |"
dim separator = "|------|--------|-------------|"

' Would export when available:
' export formatted to "output.md"
```

## See Also

- [File Formats](../reference/api/formats.md) - All supported formats
- [Import/Export](../reference/dsl/import-export.md) - DSL syntax  
- [Sheet API](../reference/api/sheet.md) - Working with imported data
- [PDF Import](./pdf.md) - Extract tables from PDFs
- [SQL Queries](./sql.md) - Analyze imported data
