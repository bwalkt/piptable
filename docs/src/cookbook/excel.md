# Excel Processing

PipTable provides comprehensive support for Excel files, including both modern XLSX and legacy XLS formats. You can read, transform, and write Excel workbooks with multiple sheets.

## Reading Excel Files

### Basic Excel Import
```piptable
' @title Read Excel File
' @description Import data from an Excel file

import "report.xlsx" into data
print "Loaded " + str(len(data)) + " rows from Excel"
```

### Read Specific Sheet
```piptable
' @title Read Specific Excel Sheet
' @description Import a named sheet from an Excel workbook

' Read specific sheet by name
import "workbook.xlsx" sheet "Sales" into sales_data

' Process the imported sheet
print "Loaded sheet with " + str(len(sales_data)) + " rows"
```

### Read All Sheets into Book
```piptable
' @title Import Entire Workbook
' @description Load all sheets from an Excel file

' Import all sheets into a book
import "financial_report.xlsx" into report_book

' Access sheets using Book API methods
' Note: Direct sheet access from workbook variables is planned but not yet available
' Currently, you can:
' - Import specific sheets by name: import "file.xlsx" sheet "SheetName" into data
' - Get sheet names: report_book.sheet_names()
' - Access sheets: report_book.get_sheet("SheetName") or report_book.sheets()
print "Sheet names: " + str(report_book.sheet_names())
print "Loaded workbook with " + str(len(report_book.sheets())) + " sheets"
```

## SQL Queries on Excel Files

### Important: Excel Table Naming in SQL

When using Excel files in SQL queries:
- **Via import**: When you import an Excel file into a variable (e.g., `import "data.xlsx" into sales`), use the variable name in SQL: `FROM sales`. The system handles the internal registration transparently.
- **Direct file reference**: When referencing Excel files directly in SQL (e.g., `FROM 'data.xlsx'`):
  - The table name is derived from the filename without extension (e.g., 'data.xlsx' becomes table `data`)
  - Only the first/default sheet is loaded (for specific sheets, use import with sheet parameter first)
  - Headers are assumed to be present in the first row
- **Best Practice**: Always import Excel files first for better control over sheet selection, naming, and header handling.

### Query Excel Data with SQL
```piptable
' @title SQL Query on Excel File
' @description Run SQL queries on imported Excel data

' Import Excel file into a variable
import "sales_data.xlsx" into sales

' Query the data using the variable name
dim high_value_sales = query(
    SELECT * FROM sales 
    WHERE amount > 10000 
    ORDER BY date DESC
)

' Export results
export high_value_sales to "high_value_sales.csv"
```

### Join Excel with CSV
```piptable
' @title Join Excel with CSV Data
' @description Combine data from Excel and CSV files using SQL

' Import data sources
import "customers.xlsx" into customers
import "orders.csv" into orders

' Join the tables
dim customer_orders = query(
    SELECT 
        c.customer_name,
        c.email,
        o.order_id,
        o.amount,
        o.order_date
    FROM customers c
    JOIN orders o ON c.customer_id = o.customer_id
    WHERE o.amount > 100
    ORDER BY o.order_date DESC
)

' Export results to new Excel file
export customer_orders to "customer_orders.xlsx"
```

### Aggregate Excel Data
```piptable
' @title Aggregate Excel Data with SQL
' @description Calculate summaries from Excel data

' Import sales data from Excel
import "monthly_sales.xlsx" into sales

' Calculate monthly summaries
dim monthly_summary = query(
    SELECT 
        month,
        COUNT(*) as transaction_count,
        SUM(amount) as total_sales,
        AVG(amount) as avg_sale,
        MAX(amount) as largest_sale
    FROM sales
    GROUP BY month
    ORDER BY month
)

' Display results
print monthly_summary
```

### Cross-Sheet Analysis
```piptable
' @title Cross-Sheet SQL Analysis
' @description Analyze data across multiple Excel sheets

' Import different sheets
import "data.xlsx" sheet "Q1_Sales" into q1
import "data.xlsx" sheet "Q2_Sales" into q2

' Union data from multiple quarters
dim half_year_sales = query(
    SELECT 'Q1' as quarter, * FROM q1
    UNION ALL
    SELECT 'Q2' as quarter, * FROM q2
)

' Analyze combined data
dim product_summary = query(
    SELECT 
        product_id,
        COUNT(*) as total_transactions,
        SUM(amount) as total_revenue
    FROM half_year_sales
    GROUP BY product_id
    ORDER BY total_revenue DESC
)

export product_summary to "half_year_summary.xlsx"
```

## Working with Legacy Excel (XLS)

### Read XLS Files
```piptable
' @title Read Legacy Excel Files
' @description Support for pre-2007 Excel formats

' Read old Excel format
import "old_report.xls" into legacy_data

' Convert to modern format
export legacy_data to "modernized_report.xlsx"
print "Converted XLS to XLSX format"
```

## Processing Excel Data

### Merge Multiple Excel Files
```piptable
' @title Merge Multiple Excel Reports
' @description Combine data from multiple Excel files

' Import multiple Excel files
import "report_jan.xlsx", "report_feb.xlsx", "report_mar.xlsx" into quarterly

' Consolidate all sheets with same structure
dim combined: table = consolidate(quarterly)

' Add quarter column 
' SQL queries can reference table variables directly (auto-registered).
' Export to a temp file only if you specifically need file-based workflows.
dim with_quarter: table = query(
  SELECT *, 'Q1' as quarter FROM combined
)

export with_quarter to "quarterly_report.xlsx"
```

### Cross-Sheet Analysis
```piptable
' @title Cross-Sheet Analysis
' @description Analyze data across multiple sheets

' Load data from separate Excel files
import "employees.xlsx" into employees
import "departments.xlsx" into departments

' Join data from different sheets
' Note: Join operations can work directly with table variables
dim analysis: table = employees inner join departments on "dept_id" = "id"

' Calculate department summaries
' SQL SELECT queries can use table variables directly.
' Export only when you need a file-backed workflow.
dim dept_summary: table = query(
  SELECT 
    department_name,
    COUNT(*) as employee_count,
    AVG(salary) as avg_salary,
    SUM(salary) as total_payroll
  FROM analysis
  GROUP BY department_name
  ORDER BY total_payroll DESC
)

export dept_summary to "department_analysis.xlsx"
```

### Clean Excel Data
```piptable
' @title Clean Messy Excel Data
' @description Fix common Excel data issues

import "messy_excel.xlsx" into raw
export raw to "temp_messy_excel.csv"

' Clean common Excel issues using file reference
dim cleaned: table = query(
  SELECT 
    TRIM(name) as name,
    CASE 
      WHEN email LIKE '%@%' THEN LOWER(email)
      ELSE NULL 
    END as email,
    CAST(REPLACE(REPLACE(phone, '-', ''), ' ', '') AS TEXT) as phone,
    CAST(amount AS FLOAT) as amount,
    DATE(date_column) as clean_date
  FROM "temp_messy_excel.csv"
  WHERE name IS NOT NULL
)

export cleaned to "cleaned_data.xlsx"
```

## Writing Excel Files

### Create Multi-Sheet Workbook
```piptable
' @title Create Multi-Sheet Workbook
' @description Generate Excel file with multiple sheets

import "sales.csv" into sales
import "customers.csv" into customers

' Create summaries for different sheets
' SQL queries can use variable names (sales) directly now.
dim monthly_summary: table = query(
  SELECT 
    month,
    COUNT(*) as transactions,
    SUM(amount) as total_sales
  FROM sales
  GROUP BY month
)

dim top_customers: table = query(
  SELECT 
    customer_id,
    COUNT(*) as purchase_count,
    SUM(amount) as total_spent
  FROM "sales.csv"
  GROUP BY customer_id
  ORDER BY total_spent DESC
  LIMIT 100
)

' Export individual sheets (Note: multi-sheet workbook creation may require special syntax)
export monthly_summary to "monthly_summary.xlsx"
export top_customers to "top_customers.xlsx"
export sales to "all_sales.xlsx"
print "Created Excel reports"
```

### Format Excel Output
```piptable
' @title Excel with Calculated Columns
' @description Add formulas and calculated fields

import "products.csv" into data

' Add calculated columns for Excel using file reference
dim with_formulas: table = query(
  SELECT 
    product_name,
    unit_price,
    quantity,
    unit_price * quantity as total_value,
    CASE 
      WHEN quantity < 10 THEN 'Low Stock'
      WHEN quantity < 50 THEN 'Medium Stock'
      ELSE 'In Stock'
    END as stock_status,
    unit_price * quantity * 0.15 as estimated_profit
  FROM "products.csv"
  ORDER BY total_value DESC
)

export with_formulas to "inventory_report.xlsx"
```

## Excel Templates

### Report Generation
```piptable
' @title Generate Excel Report from Template
' @description Create formatted reports for distribution

' Load template structure
import "report_template.xlsx" into template
import "current_month.csv" into current_data

' Match template structure using file reference
dim formatted: table = query(
  SELECT 
    department,
    category,
    SUM(revenue) as revenue,
    SUM(costs) as costs,
    SUM(revenue) - SUM(costs) as profit,
    ROUND((SUM(revenue) - SUM(costs)) / SUM(revenue) * 100, 2) as margin_pct
  FROM "current_month.csv"
  GROUP BY department, category
  ORDER BY department, category
)

' Add metadata (export and query)
export formatted to "temp_formatted.csv"
dim with_metadata: table = query(
  SELECT 
    CURRENT_DATE as report_date,
    'Monthly Report' as report_type,
    *
  FROM "temp_formatted.csv"
)

' Export with date in filename (Note: date formatting may vary)
export with_metadata to "monthly_report.xlsx"
```

## Performance Considerations

1. **Import specific sheets** when you only need one sheet
   - Use `import "file.xlsx" sheet "SheetName" into data` for direct sheet import
   - This is more efficient than importing the entire workbook
2. **Read XLS files sparingly** - convert to XLSX when possible
3. **Limit sheet size** - Excel has row limits (1,048,576 rows)
4. **Use appropriate data types** to preserve Excel formatting
5. **Consider memory usage** when working with large workbooks

## Next Steps

- [JSON Transformation](json.md) - Processing JSON data
- [Database Queries](database.md) - Working with databases
- [Report Generation](etl.md#automated-report-generation) - Building reports
