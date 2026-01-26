# Quick Start

Get up and running with PipTable in 5 minutes!

## Interactive Mode

The fastest way to try PipTable is using interactive mode:

```bash
pip -i
```

This starts a REPL where you can type commands and see results immediately:

```piptable
' @title Interactive REPL Session
' @description Example of using PipTable in interactive mode
' @readonly
' @height 250px

DIM x AS INT = 42
PRINT x

DIM data AS ARRAY = [1, 2, 3, 4, 5]
DIM sum AS INT = 0
FOR i = 1 TO 5
    sum = sum + data[i - 1]
NEXT
PRINT sum
```

## Your First Script

Create a file called `hello.pip`:

```piptable
' @title My First PipTable Script
' @description A simple example showing variables, functions, and output

' Variables and printing
DIM name AS STRING = "World"
PRINT "Hello, " + name + "!"

' Working with data
DIM numbers AS ARRAY = [1, 2, 3, 4, 5]
DIM total AS INT = 15
PRINT "Sum of numbers: " + STR(total)
```

Try editing the code above and clicking "Run" to see how it works!

```piptable
' @title Data Processing Example
' @description Working with objects and conditional logic
' @height 350px

DIM userName AS STRING = "Alice"
DIM userAge AS INT = 30
DIM userCity AS STRING = "NYC"

PRINT "User: " + userName + " from " + userCity

IF userAge >= 18 THEN
    PRINT "User is an adult"
ELSE
    PRINT "User is a minor"
END IF
```

Run it:

```bash
pip hello.pip
```

Output:
```
Hello, World!
Sum of numbers: 15
User: Alice from NYC
```

## Processing CSV Data

Create a sample CSV file `data.csv`:

```csv
name,age,department,salary
Alice,30,Engineering,75000
Bob,25,Marketing,60000
Charlie,35,Engineering,85000
Diana,28,Sales,65000
```

Process it with `process.pip`:

```vba
' Load the CSV file
dim employees = import "data.csv" into sheet

' Find high earners using SQL
dim highEarners = query("
    SELECT name, salary 
    FROM employees 
    WHERE salary > 70000
    ORDER BY salary DESC
")

' Print results
print("High Earners:")
print(highEarners)

' Export to Excel
export highEarners to "high_earners.xlsx"
print("Results saved to high_earners.xlsx")
```

## Command-Line Usage

PipTable supports several command-line options:

```bash
# Run a script
pip script.pip

# Run with variables
pip script.pip --var name=Alice --var age=30

# Execute inline code
pip -e "print('Hello from command line!')"

# Output as JSON
pip script.pip -f json

# Output as CSV
pip script.pip -f csv

# Verbose mode for debugging
pip script.pip -v
```

## Working with Multiple Files

Process multiple CSV files at once:

```vba
' Import multiple files
dim sales = import "sales_*.csv" into book

' Consolidate into single sheet
dim allSales = sales.consolidate()

' Analyze
dim summary = query("
    SELECT 
        product,
        SUM(quantity) as total_qty,
        AVG(price) as avg_price
    FROM allSales
    GROUP BY product
")

export summary to "sales_summary.xlsx"
```

## HTTP API Integration

Fetch data from APIs:

```vba
' Fetch JSON data
dim response = fetch("https://api.example.com/data")
dim data = response.json()

' Process API data
for each item in data.items
    print(item.name + ": $" + str(item.price))
next
```

## Error Handling

PipTable provides helpful error messages:

```vba
' This will show a clear error
dim data = import "missing.csv" into sheet
' Error: File not found: missing.csv

' Type checking helps catch issues
dim x: int = "not a number"
' Error: Type mismatch: expected int, got string
```

## Tips for Success

1. **Use Comments**: Start lines with `'` for documentation
2. **Type Hints**: Add `: type` for clarity (optional)
3. **SQL Power**: Use `query()` for complex data operations
4. **File Formats**: Import/export CSV, JSON, Excel, Parquet
5. **Debugging**: Use `-v` flag for verbose output

## What's Next?

- [First Script Tutorial](first-script.md) - Build a complete data pipeline
- [Core Concepts](core-concepts.md) - Understand variables, types, and control flow
- [DSL Reference](../reference/dsl/README.md) - Complete syntax documentation
- [Cookbook](../cookbook/data-processing.md) - Real-world examples