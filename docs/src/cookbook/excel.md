# Excel Processing

PipTable provides comprehensive support for Excel files, including both modern XLSX and legacy XLS formats. You can read, transform, and write Excel workbooks with multiple sheets.

## Reading Excel Files

### Basic Excel Import
```piptable
' @title Read Excel File
' @description Import data from an Excel file

DIM data AS SHEET = READ("report.xlsx")
PRINT "Loaded " + STR(LEN(data)) + " rows from Excel"
```

### Read Specific Sheet
```piptable
' @title Read Specific Excel Sheet
' @description Import a named sheet from an Excel workbook

' Read specific sheet by name
IMPORT "workbook.xlsx" WITH SHEET "Sales" INTO sales_data

' Or read by index (0-based)
IMPORT "workbook.xlsx" WITH SHEET 2 INTO third_sheet

PRINT "Loaded Sales sheet with " + STR(LEN(sales_data)) + " rows"
```

### Read All Sheets into Book
```piptable
' @title Import Entire Workbook
' @description Load all sheets from an Excel file

' Import all sheets into a book
IMPORT "financial_report.xlsx" INTO report_book

' Access individual sheets
DIM revenue AS SHEET = report_book.revenue
DIM expenses AS SHEET = report_book.expenses
DIM summary AS SHEET = report_book.summary

PRINT "Loaded " + STR(LEN(report_book)) + " sheets from workbook"
```

## Working with Legacy Excel (XLS)

### Read XLS Files
```piptable
' @title Read Legacy Excel Files
' @description Support for pre-2007 Excel formats

' Read old Excel format
DIM legacy_data AS SHEET = READ("old_report.xls")

' Convert to modern format
WRITE(legacy_data, "modernized_report.xlsx")
PRINT "Converted XLS to XLSX format"
```

## Processing Excel Data

### Merge Multiple Excel Files
```piptable
' @title Merge Multiple Excel Reports
' @description Combine data from multiple Excel files

' Import multiple Excel files
IMPORT "report_jan.xlsx,report_feb.xlsx,report_mar.xlsx" INTO quarterly

' Consolidate all sheets with same structure
DIM combined AS SHEET = CONSOLIDATE(quarterly)

' Add quarter column
DIM with_quarter AS SHEET = QUERY(combined,
  "SELECT *, 'Q1' as quarter FROM combined")

WRITE(with_quarter, "quarterly_report.xlsx")
```

### Cross-Sheet Analysis
```piptable
' @title Cross-Sheet Analysis
' @description Analyze data across multiple sheets

IMPORT "company_data.xlsx" INTO company

' Join data from different sheets
DIM employees AS SHEET = company.employees
DIM departments AS SHEET = company.departments

DIM analysis AS SHEET = JOIN INNER employees, departments 
  ON employees.dept_id = departments.id

' Calculate department summaries
DIM dept_summary AS SHEET = QUERY(analysis,
  "SELECT 
    department_name,
    COUNT(*) as employee_count,
    AVG(salary) as avg_salary,
    SUM(salary) as total_payroll
   FROM analysis
   GROUP BY department_name
   ORDER BY total_payroll DESC")

WRITE(dept_summary, "department_analysis.xlsx")
```

### Clean Excel Data
```piptable
' @title Clean Messy Excel Data
' @description Fix common Excel data issues

DIM raw AS SHEET = READ("messy_excel.xlsx")

' Clean common Excel issues
DIM cleaned AS SHEET = QUERY(raw,
  "SELECT 
    TRIM(name) as name,
    CASE 
      WHEN email LIKE '%@%' THEN LOWER(email)
      ELSE NULL 
    END as email,
    CAST(REPLACE(REPLACE(phone, '-', ''), ' ', '') AS TEXT) as phone,
    CAST(amount AS FLOAT) as amount,
    DATE(date_column) as clean_date
   FROM raw
   WHERE name IS NOT NULL")

WRITE(cleaned, "cleaned_data.xlsx")
```

## Writing Excel Files

### Create Multi-Sheet Workbook
```piptable
' @title Create Multi-Sheet Workbook
' @description Generate Excel file with multiple sheets

DIM sales AS SHEET = READ("sales.csv")
DIM customers AS SHEET = READ("customers.csv")

' Create summaries for different sheets
DIM monthly_summary AS SHEET = QUERY(sales,
  "SELECT 
    month,
    COUNT(*) as transactions,
    SUM(amount) as total_sales
   FROM sales
   GROUP BY month")

DIM top_customers AS SHEET = QUERY(sales,
  "SELECT 
    customer_id,
    COUNT(*) as purchase_count,
    SUM(amount) as total_spent
   FROM sales
   GROUP BY customer_id
   ORDER BY total_spent DESC
   LIMIT 100")

' Create a book with multiple sheets
DIM report_book AS BOOK = NEW BOOK()
report_book.monthly_summary = monthly_summary
report_book.top_customers = top_customers
report_book.all_sales = sales

WRITE(report_book, "sales_report.xlsx")
PRINT "Created Excel workbook with 3 sheets"
```

### Format Excel Output
```piptable
' @title Excel with Calculated Columns
' @description Add formulas and calculated fields

DIM data AS SHEET = READ("products.csv")

' Add calculated columns for Excel
DIM with_formulas AS SHEET = QUERY(data,
  "SELECT 
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
   FROM data
   ORDER BY total_value DESC")

WRITE(with_formulas, "inventory_report.xlsx")
```

## Excel Templates

### Report Generation
```piptable
' @title Generate Excel Report from Template
' @description Create formatted reports for distribution

' Load template structure
DIM template AS SHEET = READ("report_template.xlsx")
DIM current_data AS SHEET = READ("current_month.csv")

' Match template structure
DIM formatted AS SHEET = QUERY(current_data,
  "SELECT 
    department,
    category,
    SUM(revenue) as revenue,
    SUM(costs) as costs,
    SUM(revenue) - SUM(costs) as profit,
    ROUND((SUM(revenue) - SUM(costs)) / SUM(revenue) * 100, 2) as margin_pct
   FROM current_data
   GROUP BY department, category
   ORDER BY department, category")

' Add metadata
DIM with_metadata AS SHEET = QUERY(formatted,
  "SELECT 
    CURRENT_DATE as report_date,
    'Monthly Report' as report_type,
    *
   FROM formatted")

WRITE(with_metadata, "monthly_report_" + STR(CURRENT_DATE) + ".xlsx")
```

## Performance Considerations

1. **Use specific sheet names** when you only need one sheet
2. **Read XLS files sparingly** - convert to XLSX when possible
3. **Limit sheet size** - Excel has row limits (1,048,576 rows)
4. **Use appropriate data types** to preserve Excel formatting
5. **Consider memory usage** when working with large workbooks

## Next Steps

- [JSON Transformation](json.md) - Processing JSON data
- [Database Queries](database.md) - Working with databases
- [Report Generation](etl.md#automated-report-generation) - Building reports