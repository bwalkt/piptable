# PDF Processing Cookbook

Real-world examples and recipes for PDF processing with PipTable.

## Table of Contents

1. [Research Paper to Markdown](#research-paper-to-markdown)
2. [Financial Report Analysis](#financial-report-analysis)
3. [Multi-PDF Consolidation](#multi-pdf-consolidation)
4. [Catalog Data Extraction](#catalog-data-extraction)
5. [Legal Document Processing](#legal-document-processing)
6. [Scientific Data Tables](#scientific-data-tables)
7. [Invoice Processing](#invoice-processing)
8. [Academic Transcript Analysis](#academic-transcript-analysis)

## Research Paper to Markdown

**Goal**: Convert a research paper PDF to Markdown for blog posts or documentation.

```vba
' Extract paper structure
import "paper.pdf" into paper with { "extract_structure": true }

' Export to JSON for downstream use
export paper to "paper.json"

' Or process headings first
dim elements = paper.document.elements
dim abstract_found = false

for each elem in elements
    if elem.type == "heading" and elem.content contains "Abstract" {
        abstract_found = true
    }
    if abstract_found and elem.type == "paragraph" {
        print("Abstract: " + elem.content)
        break
    }
next
```

## Financial Report Analysis

**Goal**: Extract financial tables and calculate key metrics.

```vba
' Import income statement tables
import "annual_report.pdf" into financials with {
    page_range: "45-55",
    min_table_rows: 5,
    detect_headers: true
}

' Calculate financial ratios
dim table = financials.table_1
select 
    Year,
    Revenue,
    NetIncome,
    (NetIncome / Revenue * 100) as ProfitMargin,
    case 
        when NetIncome > 0 then 'Profitable'
        else 'Loss'
    end as Status
from table
where Year >= 2020
order by Year desc

' Export analysis
export analysis to "financial_analysis.xlsx"
```

## Multi-PDF Consolidation

**Goal**: Combine data from multiple PDF reports into a single dataset.

```vba
' Initialize results table
create table consolidated (
    source varchar,
    date date,
    category varchar,
    amount decimal
)

' Process each monthly report
dim months = ["jan", "feb", "mar", "apr", "may", "jun"]

for each month in months
    dim filename = "report_2024_" + month + ".pdf"
    
    ' Import tables from each PDF
    import filename into monthly_data with {
        page_range: "2-5",
        detect_headers: true
    }
    
    ' Insert into consolidated table
    insert into consolidated
    select 
        filename as source,
        Date,
        Category,
        Amount
    from monthly_data
    where Amount > 0
next

' Generate summary
select 
    Category,
    count(*) as Transactions,
    sum(Amount) as Total,
    avg(Amount) as Average
from consolidated
group by Category
order by Total desc

export summary to "quarterly_summary.csv"
```

## Catalog Data Extraction

**Goal**: Extract product information from a PDF catalog.

```vba
' Import product tables
import "catalog.pdf" into products with {
    min_table_cols: 4,
    detect_headers: true
}

' Clean and standardize data
dim table = products.table_1
select 
    upper(trim(SKU)) as ProductCode,
    ProductName,
    replace(Price, '$', '') as Price,
    case
        when Availability = 'Y' then 'In Stock'
        when Availability = 'N' then 'Out of Stock'
        else 'Check Availability'
    end as StockStatus
from table
where ProductName is not null

' Find products on sale
select * from table
where Price < OriginalPrice
order by (OriginalPrice - Price) desc
limit 20

export results to "sale_items.json"
```

## Legal Document Processing

**Goal**: Extract and analyze contract clauses from legal PDFs.

```vba
' Extract document structure
import "contract.pdf" into contract with { "extract_structure": true }

' Find specific sections
dim elements = contract.document.elements
dim clauses = []

for each elem in elements
    if elem.type == "heading" {
        ' Track section numbers like "3.1", "3.2"
        if elem.content matches "^\\d+\\.\\d+" {
            clauses.push({
                section: elem.content,
                page: elem.page + 1,
                content: ""
            })
        }
    } elif elem.type == "paragraph" and len(clauses) > 0 {
        ' Add paragraph to current clause
        clauses[-1].content += elem.content + " "
    }
next

' Search for key terms
dim important_terms = ["liability", "termination", "payment", "confidential"]

for each clause in clauses
    for each term in important_terms
        if lower(clause.content) contains term {
            print("Section " + clause.section + " (page " + clause.page + ") mentions: " + term)
        }
    next
next
```

## Scientific Data Tables

**Goal**: Extract and analyze experimental data from research papers.

```vba
' Import data tables from methods section
import "research_data.pdf" into experiments with {
    page_range: "8-15",
    min_table_rows: 3,
    detect_headers: true
}

' Statistical analysis
dim table = experiments.table_1
select 
    Condition,
    count(*) as Samples,
    avg(Measurement) as Mean,
    stdev(Measurement) as StdDev,
    min(Measurement) as Min,
    max(Measurement) as Max
from table
group by Condition

' Find significant results
select * from table
where PValue < 0.05
order by PValue asc

' Export for further analysis
export results to "experimental_results.csv"
```

## Invoice Processing

**Goal**: Extract invoice data for accounting systems.

```vba
' Process batch of invoice PDFs
dim invoice_files = glob("invoices/*.pdf")
create table all_invoices (
    invoice_number varchar,
    date date,
    vendor varchar,
    amount decimal,
    tax decimal,
    total decimal
)

for each file in invoice_files
    try {
        import file into invoice_data with {
            page_range: "1-1",  ' Invoices are usually 1 page
            min_table_rows: 1
        }
        
        ' Extract invoice details
        ' Assuming standard invoice format
        insert into all_invoices
        select 
            InvoiceNo,
            InvoiceDate,
            Vendor,
            Subtotal,
            Tax,
            Total
        from invoice_data
        where Total > 0
        
    } catch error {
        print("Failed to process: " + file)
        ' Log failed files for manual review
    }
next

' Monthly summary
select 
    month(date) as Month,
    count(*) as InvoiceCount,
    sum(total) as TotalSpent,
    avg(total) as AverageInvoice
from all_invoices
group by month(date)
order by Month

export data to "accounts_payable.xlsx"
```

## Academic Transcript Analysis

**Goal**: Extract and analyze grades from academic transcripts.

```vba
' Import transcript tables
import "transcript.pdf" into grades with {
    detect_headers: true,
    min_table_cols: 4  ' Course, Credits, Grade, Points
}

' Calculate GPA
dim table = grades.table_1
select 
    Semester,
    sum(Credits) as TotalCredits,
    sum(Credits * GradePoints) / sum(Credits) as GPA,
    count(*) as Courses
from table
where Grade != 'W'  ' Exclude withdrawals
group by Semester
order by Semester

' Find best performance
select 
    Course,
    Grade,
    Credits,
    GradePoints
from table
where GradePoints = 4.0
order by Credits desc

' Degree progress
select 
    Department,
    sum(Credits) as CompletedCredits,
    avg(GradePoints) as DeptGPA
from table
group by Department
having sum(Credits) >= 3
order by DeptGPA desc

export results to "academic_analysis.json"
```

## Advanced Techniques

### Combining Structure and Tables

```vba
' Extract both structure and tables
import "report.pdf" into structure with { "extract_structure": true }
import "report.pdf" into tables

' Match tables to their sections
dim current_section = "Unknown"
dim table_sections = []

for each elem in structure.document.elements
    if elem.type == "heading" {
        current_section = elem.content
    }
    ' Table pages are not tracked yet; associate headings manually as needed
next

' Example manual association
table_sections.push({
    section: current_section,
    table: "table_1"
})
```

### Error Recovery

```vba
' Robust PDF processing with fallbacks
function process_pdf(filename) {
    try {
        ' Try with structure first
        import filename into doc with { "extract_structure": true }
        return doc
    } catch {
        try {
            ' Fall back to table extraction
            import filename into tables
            return tables
        } catch {
            print("Failed to process: " + filename)
            return null
        }
    }
}
```

### Performance Optimization

```vba
' Process large PDFs in chunks
dim total_pages = 500
dim chunk_size = 50

for start in range(1, total_pages, chunk_size) {
    dim end = min(start + chunk_size - 1, total_pages)
    
    import "large_doc.pdf" into chunk with {
        page_range: start + "-" + end,
        extract_structure: true
    }
    
    ' Process chunk
    process_chunk(chunk)
    
    ' Free memory in your host environment as needed
}
```

## Tips and Best Practices

1. **Always specify page ranges** for large PDFs to improve performance
2. **Use try-catch blocks** for batch processing to handle corrupted PDFs
3. **Check for null values** when processing tables with inconsistent formats
4. **Export intermediate results** when processing many files
5. **Use structure extraction** for text-heavy documents, tables for data-heavy ones
6. **Validate extracted data** by checking row/column counts and data types
7. **Log processing errors** for manual review of problematic PDFs
