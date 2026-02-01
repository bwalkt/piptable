# Working with PDF Documents

PipTable can extract tables from PDF documents, making it easy to analyze data from reports, invoices, and other structured documents.

## Importing PDF Tables

### Basic Import

```piptable
' Single table -> sheet
dim report = import "report.pdf" into report

' Multiple tables -> book
dim tables = import "report.pdf" into tables
dim summary = tables["table_1"]
dim details = tables["table_2"]
```

### With Headers

```piptable
' Import with header detection (book)
dim report = import "financial_report.pdf" with {"has_headers": true} into book

' First row of each table becomes column names
dim quarterly = report["table_1"]
print(quarterly.column_names())  ' ["Quarter", "Revenue", "Profit"]
```

## Examples

### Financial Report Analysis

Extract and analyze tables from financial PDFs:

```piptable
' Import quarterly report
dim report = import "Q4_2024.pdf" into book
dim financials = report["table_1"]

' Calculate totals and growth
dim totals = query("
    SELECT 
        SUM(revenue) as total_revenue,
        SUM(expenses) as total_expenses,
        SUM(revenue - expenses) as net_profit
    FROM financials
")

' Export analysis to Excel
export totals to "financial_analysis.xlsx"
```

### Invoice Processing

Process multiple invoice PDFs:

```piptable
function process_invoice(pdf_path)
    dim tables = import pdf_path into book
    
    ' Usually first table has invoice details
    dim invoice_details = tables["table_1"]
    
    ' Extract key information
    dim total = query("
        SELECT SUM(quantity * unit_price) as total
        FROM invoice_details
    ")
    
    return total[0][0]
end function

' Process batch of invoices
dim invoices = ["inv001.pdf", "inv002.pdf", "inv003.pdf"]
dim grand_total = 0

for invoice in invoices
    dim amount = process_invoice(invoice)
    print(invoice + ": $" + str(amount))
    grand_total = grand_total + amount
next

print("Grand Total: $" + str(grand_total))
```

### Research Paper Data Extraction

Extract data tables from academic papers:

```piptable
' Import research paper tables
dim paper = import "research_paper.pdf" into book

' Find results table (often has specific patterns)
dim results = paper["table_3"]  ' Adjust based on your PDF

' Analyze experimental data
dim analysis = query("
    SELECT 
        experiment_id,
        AVG(measurement) as avg_result,
        STDDEV(measurement) as std_dev
    FROM results
    GROUP BY experiment_id
    ORDER BY avg_result DESC
")

' Create visualization data
export analysis to "experiment_results.json"
```

## PDF Table Features

### Type Inference

PDF tables automatically detect data types:

| PDF Content | Detected Type | Example |
|-------------|---------------|---------|
| Numbers | Int/Float | `42`, `3.14` |
| Percentages | String/Float | `"85%"` → can parse |
| Currency | String | `"$1,234.56"` |
| Dates | String | `"2024-01-01"` |
| Text | String | `"Product Name"` |

### Multiple Table Handling

PDFs often contain multiple tables:

```piptable
' Process all tables in a PDF
dim document = import "complex_report.pdf" into book

' Iterate through all tables
for i = 1 to 10  ' Check up to 10 tables
    dim table_name = "table_" + str(i)
    
    if document.has_key(table_name) then
        dim table = document[table_name]
        print("Table " + str(i) + " has " + str(table.row_count()) + " rows")
        
        ' Process based on content
        if table.has_column("Revenue") then
            print("  -> Financial table detected")
        elseif table.has_column("Name") then
            print("  -> Personnel table detected")
        end if
    end if
next
```

### OCR Support

For scanned PDFs (when OCR is available):

```piptable
' Import scanned document
dim scanned = import "scanned_document.pdf" into book

' OCR will be automatically applied if:
' - Text extraction yields minimal content
' - Tesseract is available on the system
' - Image quality is sufficient

dim data = scanned["table_1"]
' Clean up OCR artifacts if needed
dim cleaned = query("
    SELECT 
        TRIM(column1) as clean_col1,
        CAST(REPLACE(column2, 'O', '0') AS INT) as numeric_col2
    FROM data
")
```

## Best Practices

### 1. Table Identification

Not all PDF structures are tables:

```piptable
' Check if extracted table is valid
function is_valid_table(sheet)
    ' Tables should have consistent column count
    if sheet.col_count() < 2 then
        return false
    end if
    
    ' Tables should have multiple rows
    if sheet.row_count() < 2 then
        return false
    end if
    
    return true
end function

dim pdf_data = import "document.pdf" into book
for key in pdf_data.keys()
    dim table = pdf_data[key]
    if is_valid_table(table) then
        ' Process valid table
        print("Processing " + key)
    end if
next
```

### 2. Data Cleaning

PDF extraction may need cleanup:

```piptable
' Clean extracted PDF data
dim raw = import "messy_report.pdf" into book
dim data = raw["table_1"]

' Remove empty rows
dim cleaned = query("
    SELECT * FROM data
    WHERE NOT (col1 IS NULL AND col2 IS NULL AND col3 IS NULL)
")

' Fix common OCR/extraction issues
dim normalized = cleaned.map_column("Amount", lambda(x) {
    ' Remove currency symbols and commas
    dim clean = x.replace("$", "").replace(",", "")
    return float(clean)
})
```

### 3. Error Handling

```piptable
try
    dim tables = import "report.pdf" into book
    
    if tables.sheet_count() == 0 then
        print("No tables found in PDF")
        exit
    end if
    
    ' Process tables...
    
catch error
    print("Failed to extract PDF tables: " + error.message)
    
    ' Fallback to manual entry or alternative source
    dim manual_data = import "report_backup.csv" into sheet
end try
```

## Integration Examples

### With SQL Queries

```piptable
' Import and join PDF tables
dim report = import "annual_report.pdf" into book
dim revenue = report["table_1"]
dim expenses = report["table_2"]

' Complex analysis with SQL
dim analysis = query("
    WITH profit_calc AS (
        SELECT 
            r.month,
            r.revenue,
            e.expenses,
            (r.revenue - e.expenses) as profit
        FROM revenue r
        JOIN expenses e ON r.month = e.month
    )
    SELECT 
        month,
        profit,
        SUM(profit) OVER (ORDER BY month) as cumulative_profit
    FROM profit_calc
")
```

### Export Pipeline

```piptable
' PDF to multiple formats pipeline
dim source = import "data_tables.pdf" into book

' Export each table to different format
for key in source.keys()
    dim table = source[key]
    
    ' CSV for data analysis
    export table to key + ".csv"
    
    ' JSON for web API
    export table to key + ".json"
    
    ' Parquet for data warehouse
    export table to key + ".parquet"
next
```

## Limitations

- **Layout Detection**: Complex layouts may not extract perfectly
- **Merged Cells**: May be split or duplicated
- **Formatting**: Colors, fonts, and styles are not preserved
- **Images**: Charts and images within tables are not extracted
- **Password Protection**: Encrypted PDFs require decryption first

## Troubleshooting

### No Tables Found

```piptable
dim tables = import "document.pdf" into book

if tables.sheet_count() == 0 then
    print("No tables detected. This could mean:")
    print("- The PDF contains only text (no tabular data)")
    print("- Tables are embedded as images (try OCR)")
    print("- Complex layout preventing detection")
end if
```

### Incomplete Extraction

```piptable
' If tables are cut off or incomplete
dim options = {
    "page_range": "1-5",     ' Limit to specific pages
    "min_table_size": 3,     ' Minimum rows to consider as table
    "detect_headers": true    ' Try to identify headers
}

dim tables = import "report.pdf" with options into book
```

## See Also

- [File Formats](../reference/api/formats.md) - All supported formats
- [Import/Export](../reference/dsl/import-export.md) - DSL syntax
- [Sheet API](../reference/api/sheet.md) - Working with extracted data
- [SQL Queries](./sql.md) - Analyzing imported data
