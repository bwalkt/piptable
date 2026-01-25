# First Script Tutorial

Let's build a complete data processing pipeline step by step. We'll analyze sales data, generate insights, and create reports.

## The Scenario

You have monthly sales data in CSV files and need to:
1. Combine multiple months of data
2. Calculate metrics by product and region  
3. Identify top performers
4. Generate an Excel report

## Step 1: Setup Sample Data

First, create sample data files:

`sales_jan.csv`:
```csv
date,product,region,quantity,price,total
2024-01-05,Widget A,North,10,25.00,250.00
2024-01-12,Widget B,South,5,50.00,250.00
2024-01-15,Widget A,East,8,25.00,200.00
2024-01-20,Widget C,North,12,15.00,180.00
2024-01-25,Widget B,West,7,50.00,350.00
```

`sales_feb.csv`:
```csv
date,product,region,quantity,price,total
2024-02-03,Widget A,South,15,25.00,375.00
2024-02-10,Widget C,East,20,15.00,300.00
2024-02-14,Widget B,North,9,50.00,450.00
2024-02-18,Widget A,West,11,25.00,275.00
2024-02-25,Widget C,South,18,15.00,270.00
```

## Step 2: Create the Script

Create `sales_analysis.pip`:

```vba
' sales_analysis.pip - Complete sales data analysis pipeline
' Author: Your Name
' Date: 2024

' ============================================
' STEP 1: Load and Combine Data
' ============================================

print("Loading sales data...")

' Import all sales CSV files into a book (multiple sheets)
dim salesBook = import "sales_*.csv" into book

' Consolidate all sheets into one
dim allSales = salesBook.consolidate()

print("Loaded " + str(len(allSales)) + " sales records")

' ============================================
' STEP 2: Basic Analysis
' ============================================

print("Analyzing sales data...")

' Calculate total revenue
dim totalRevenue = query("
    SELECT SUM(total) as revenue 
    FROM allSales
")
print("Total Revenue: $" + str(totalRevenue[0]["revenue"]))

' Sales by product
dim productSales = query("
    SELECT 
        product,
        SUM(quantity) as units_sold,
        SUM(total) as revenue,
        AVG(price) as avg_price
    FROM allSales
    GROUP BY product
    ORDER BY revenue DESC
")

print("\nSales by Product:")
print(productSales)

' ============================================
' STEP 3: Regional Analysis
' ============================================

dim regionalSales = query("
    SELECT 
        region,
        COUNT(*) as transactions,
        SUM(total) as revenue,
        AVG(total) as avg_transaction
    FROM allSales
    GROUP BY region
    ORDER BY revenue DESC
")

print("\nSales by Region:")
print(regionalSales)

' ============================================
' STEP 4: Find Top Performers
' ============================================

' Best selling product
dim topProduct = query("
    SELECT product, SUM(total) as revenue
    FROM allSales
    GROUP BY product
    ORDER BY revenue DESC
    LIMIT 1
")

print("\nTop Product: " + topProduct[0]["product"])

' Best performing region
dim topRegion = query("
    SELECT region, SUM(total) as revenue
    FROM allSales  
    GROUP BY region
    ORDER BY revenue DESC
    LIMIT 1
")

print("Top Region: " + topRegion[0]["region"])

' ============================================
' STEP 5: Time-based Analysis
' ============================================

' Extract month from date for monthly trends
dim monthlyTrends = query("
    SELECT 
        SUBSTR(date, 1, 7) as month,
        SUM(total) as revenue,
        COUNT(*) as transactions
    FROM allSales
    GROUP BY month
    ORDER BY month
")

print("\nMonthly Trends:")
print(monthlyTrends)

' ============================================
' STEP 6: Create Reports
' ============================================

print("\nGenerating reports...")

' Create a summary object for the report
dim summary = {
    "total_revenue": totalRevenue[0]["revenue"],
    "top_product": topProduct[0]["product"],
    "top_region": topRegion[0]["region"],
    "total_transactions": len(allSales)
}

' Export detailed reports
export productSales to "product_report.xlsx"
export regionalSales to "regional_report.xlsx"
export monthlyTrends to "monthly_trends.xlsx"

' Export raw data for reference
export allSales to "all_sales_combined.xlsx"

' Save summary as JSON
export summary to "summary.json"

print("Reports generated:")
print("  - product_report.xlsx")
print("  - regional_report.xlsx")
print("  - monthly_trends.xlsx")
print("  - all_sales_combined.xlsx")
print("  - summary.json")

' ============================================
' STEP 7: Data Quality Check
' ============================================

' Check for any data issues
dim nullCheck = query("
    SELECT COUNT(*) as nulls
    FROM allSales
    WHERE total IS NULL 
       OR quantity IS NULL
       OR price IS NULL
")

if nullCheck[0]["nulls"] > 0 then
    print("WARNING: Found " + str(nullCheck[0]["nulls"]) + " records with null values")
else
    print("\nData quality check passed!")
end if

print("\nAnalysis complete!")
```

## Step 3: Run the Script

Execute the analysis:

```bash
pip sales_analysis.pip
```

Expected output:

```
Loading sales data...
Loaded 10 sales records

Analyzing sales data...
Total Revenue: $2900.00

Sales by Product:
product    units_sold  revenue  avg_price
Widget B   21          1050.00  50.00
Widget A   44          1100.00  25.00
Widget C   50          750.00   15.00

Sales by Region:
region  transactions  revenue  avg_transaction
North   3            880.00   293.33
South   3            895.00   298.33
East    2            500.00   250.00
West    2            625.00   312.50

Top Product: Widget A
Top Region: South

Monthly Trends:
month     revenue  transactions
2024-01   1230.00  5
2024-02   1670.00  5

Generating reports...
Reports generated:
  - product_report.xlsx
  - regional_report.xlsx
  - monthly_trends.xlsx
  - all_sales_combined.xlsx
  - summary.json

Data quality check passed!

Analysis complete!
```

## Step 4: Extend the Script

Add visualization and advanced features:

```vba
' Add after Step 6 in the script

' ============================================
' STEP 8: Advanced Analysis
' ============================================

' Calculate growth rate
dim janSales = query("
    SELECT SUM(total) as revenue
    FROM allSales
    WHERE date LIKE '2024-01%'
")[0]["revenue"]

dim febSales = query("
    SELECT SUM(total) as revenue
    FROM allSales
    WHERE date LIKE '2024-02%'
")[0]["revenue"]

dim growthRate = ((febSales - janSales) / janSales) * 100
print("\nMonth-over-month growth: " + str(int(growthRate)) + "%")

' Find cross-selling opportunities
dim productPairs = query("
    SELECT 
        a.product as product1,
        b.product as product2,
        COUNT(*) as frequency
    FROM allSales a
    JOIN allSales b ON a.region = b.region
        AND a.date = b.date
        AND a.product < b.product
    GROUP BY product1, product2
    ORDER BY frequency DESC
")

if len(productPairs) > 0 then
    print("\nTop product combinations by region/date:")
    print(productPairs)
end if

' ============================================
' STEP 9: Alerts and Notifications
' ============================================

' Check for low performing products
dim lowPerformers = query("
    SELECT product, SUM(total) as revenue
    FROM allSales
    GROUP BY product
    HAVING revenue < 500
")

if len(lowPerformers) > 0 then
    print("\nALERT: Low performing products detected:")
    for each product in lowPerformers
        print("  - " + product["product"] + ": $" + str(product["revenue"]))
    next
end if
```

## Key Concepts Demonstrated

This script showcases:

1. **File Import**: Loading multiple CSV files
2. **Data Consolidation**: Combining sheets into one
3. **SQL Queries**: Complex aggregations and joins
4. **Variables**: Storing and using results
5. **Control Flow**: If statements and loops
6. **Data Export**: Multiple output formats
7. **Error Handling**: Data quality checks
8. **Comments**: Documenting code sections

## Exercises

Try modifying the script to:

1. Add a filter for minimum transaction amount
2. Calculate the best day of the week for sales
3. Create a function to format currency values
4. Add email notification for low performers
5. Generate a chart using the `chart` statement

## Next Steps

- [Core Concepts](core-concepts.md) - Deep dive into PipTable features
- [DSL Reference](../reference/dsl/README.md) - Complete syntax guide
- [Cookbook](../cookbook/data-processing.md) - More real-world examples