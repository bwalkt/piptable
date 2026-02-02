# PDF Playground Examples

Copy and paste these examples into the PipTable playground to try PDF processing features.

## Quick Start Examples

### 1. Basic Table Workflow (Simulated)

```vba
' Simulate a PDF table import (PDF import is not supported in the playground)
create table data (
    Product varchar,
    Qty integer,
    Price decimal
)

insert into data values
('Widget', 10, 2.50),
('Gadget', 5, 4.00)

select * from data
```

### 2. Document Structure (Simulated)

```vba
' Simulate structured PDF output
dim doc = {
    "document": {
        "elements": [
            { "type": "heading", "level": 1, "content": "Overview", "page": 0 },
            { "type": "paragraph", "content": "Summary of results...", "page": 0 }
        ],
        "metadata": { "page_count": 1 }
    }
}

print(doc)
```

### 3. Page Range (Simulated)

```vba
' Simulate a page-range table
create table summary (
    Page integer,
    Metric varchar,
    Value decimal
)

insert into summary values
(1, 'Revenue', 1000),
(2, 'Revenue', 1200)

select * from summary
```

## Interactive Examples

### Example 1: Financial Report Analysis

```vba
' Simulate financial data (since playground may not have PDF)
create table financial_data (
    Year integer,
    Quarter varchar,
    Revenue decimal,
    Expenses decimal,
    NetIncome decimal
)

insert into financial_data values
(2024, 'Q1', 1000000, 750000, 250000),
(2024, 'Q2', 1100000, 780000, 320000),
(2024, 'Q3', 1050000, 800000, 250000),
(2024, 'Q4', 1200000, 850000, 350000)

' Calculate metrics
select 
    Year,
    Quarter,
    Revenue,
    NetIncome,
    round(NetIncome / Revenue * 100, 2) as "Profit Margin %"
from financial_data
order by Year, Quarter

' Quarterly comparison
select 
    Quarter,
    sum(Revenue) as TotalRevenue,
    avg(NetIncome) as AvgProfit
from financial_data
group by Quarter
```

### Example 2: Document Structure Processing

```vba
' Simulate document structure from PDF extraction
dim doc = {
    "document": {
        "elements": [
            {
                "type": "heading",
                "level": 1,
                "content": "Introduction",
                "page": 0
            },
            {
                "type": "paragraph",
                "content": "This document describes our Q3 results...",
                "page": 0
            },
            {
                "type": "heading",
                "level": 2,
                "content": "Revenue Analysis",
                "page": 1
            },
            {
                "type": "paragraph",
                "content": "Revenue increased by 15% year-over-year...",
                "page": 1
            },
            {
                "type": "heading",
                "level": 2,
                "content": "Cost Structure",
                "page": 2
            },
            {
                "type": "paragraph",
                "content": "Operating expenses remained flat...",
                "page": 2
            }
        ],
        "metadata": {
            "page_count": 3
        }
    }
}

' Extract all headings
dim headings = filter(doc.document.elements, e => e.type == "heading")

print("Document Structure:")
foreach h in headings {
    dim indent = repeat("  ", h.level - 1)
    print(indent + "- " + h.content + " (page " + (h.page + 1) + ")")
}

' Count elements by type
dim heading_count = count(filter(doc.document.elements, e => e.type == "heading"))
dim para_count = count(filter(doc.document.elements, e => e.type == "paragraph"))

print("")
print("Statistics:")
print("Total pages: " + doc.metadata.page_count)
print("Headings: " + heading_count)
print("Paragraphs: " + para_count)
```

### Example 3: Product Catalog Processing

```vba
' Simulate product catalog data from PDF
create table products (
    SKU varchar,
    ProductName varchar,
    Category varchar,
    Price decimal,
    Stock integer,
    OnSale boolean
)

insert into products values
('SKU001', 'Laptop Pro 15', 'Electronics', 1299.99, 45, true),
('SKU002', 'Wireless Mouse', 'Electronics', 29.99, 120, false),
('SKU003', 'USB-C Cable', 'Accessories', 19.99, 200, true),
('SKU004', 'Monitor 27"', 'Electronics', 399.99, 30, false),
('SKU005', 'Keyboard Mechanical', 'Electronics', 149.99, 65, true),
('SKU006', 'Laptop Stand', 'Accessories', 49.99, 80, false),
('SKU007', 'Webcam HD', 'Electronics', 79.99, 55, true)

' Find products on sale
print("🔥 Products on Sale:")
select 
    ProductName,
    Price as "Regular Price",
    round(Price * 0.8, 2) as "Sale Price",
    round(Price * 0.2, 2) as "You Save"
from products
where OnSale = true
order by Price desc

' Category summary
print("")
print("📊 Category Summary:")
select 
    Category,
    count(*) as Products,
    round(avg(Price), 2) as "Avg Price",
    sum(Stock) as "Total Stock"
from products
group by Category
```

### Example 4: Multi-Table Analysis

```vba
' Simulate multiple tables from PDF pages
create table sales_q1 (
    Product varchar,
    Units integer,
    Revenue decimal
)

create table sales_q2 (
    Product varchar,
    Units integer,
    Revenue decimal
)

insert into sales_q1 values
('Product A', 100, 5000),
('Product B', 150, 7500),
('Product C', 80, 4000)

insert into sales_q2 values
('Product A', 120, 6000),
('Product B', 140, 7000),
('Product C', 95, 4750)

' Compare quarters
select 
    q1.Product,
    q1.Units as "Q1 Units",
    q2.Units as "Q2 Units",
    q2.Units - q1.Units as "Change",
    case 
        when q2.Units > q1.Units then '📈'
        when q2.Units < q1.Units then '📉'
        else '→'
    end as Trend
from sales_q1 q1
join sales_q2 q2 on q1.Product = q2.Product
order by q1.Product
```

### Example 5: Data Validation and Cleaning

```vba
' Simulate messy data from PDF extraction
create table raw_data (
    ID varchar,
    Name varchar,
    Email varchar,
    Amount varchar
)

insert into raw_data values
('001', 'John Doe', 'john@example.com', '$1,234.56'),
('002', 'Jane Smith', 'jane@invalid', '2345.67'),
('003', '', 'bob@example.com', '$500'),
('004', 'Alice Brown', 'alice@example.com', 'N/A'),
('005', 'Charlie', 'charlie@example.com', '1000.00')

' Clean and validate data
select 
    ID,
    coalesce(nullif(Name, ''), 'Unknown') as Name,
    case 
        when Email like '%@%.%' then Email
        else 'Invalid Email'
    end as Email,
    case 
        when Amount = 'N/A' then 0
        else cast(replace(replace(Amount, '$', ''), ',', '') as decimal)
    end as CleanAmount
from raw_data

' Find data quality issues
print("⚠️ Data Quality Issues:")
select 
    'Missing Name' as Issue,
    count(*) as Count
from raw_data
where Name = '' or Name is null
union all
select 
    'Invalid Email' as Issue,
    count(*) as Count
from raw_data
where Email not like '%@%.%'
union all
select 
    'Invalid Amount' as Issue,
    count(*) as Count
from raw_data
where Amount = 'N/A'
```

### Example 6: Text Search in Structure

```vba
' Simulate searching through document structure
dim doc = {
    "document": {
        "elements": [
            {"type": "heading", "content": "Revenue Report 2024"},
            {"type": "paragraph", "content": "Total revenue reached $5.2M in Q3."},
            {"type": "heading", "content": "Expense Analysis"},
            {"type": "paragraph", "content": "Operating expenses were reduced by 10%."},
            {"type": "heading", "content": "Profit Margins"},
            {"type": "paragraph", "content": "Net profit margin improved to 22%."}
        ]
    }
}

' Search for keywords
dim keywords = ["revenue", "profit", "expense"]

print("🔍 Keyword Search Results:")
foreach keyword in keywords {
    print("")
    print("Searching for: " + upper(keyword))
    dim found = false
    
    foreach elem in doc.document.elements {
        if lower(elem.content) contains lower(keyword) {
            print("  ✓ Found in " + elem.type + ": " + elem.content)
            found = true
        }
    }
    
    if not found {
        print("  ✗ Not found")
    }
}
```

## Try It Yourself

1. **Upload a PDF**: Use the file upload button in the playground
2. **Modify examples**: Change the queries to explore your data
3. **Combine features**: Mix structure extraction with SQL queries
4. **Export results**: Use `export to` commands to save your analysis

## Tips for Playground

- Start with small PDFs (< 10 pages) for faster processing
- Use `limit` clause to preview large datasets
- Check the console for any error messages
- Export results as CSV or JSON for external analysis

## Next Steps

After trying these examples:
1. Upload your own PDF files
2. Combine multiple examples for complex workflows
3. Save successful scripts for reuse
4. Share interesting patterns with the community
