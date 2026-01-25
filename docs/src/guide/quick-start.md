# Quick Start

Get up and running with PipTable in 5 minutes!

## Interactive Mode

The fastest way to try PipTable is using interactive mode:

```bash
pip -i
```

This starts a REPL where you can type commands and see results immediately:

```vba
PipTable vX.Y.Z
Type 'exit' to quit, 'help' for assistance

> dim x = 42
> print(x)
42

> dim data = [1, 2, 3, 4, 5]
> dim sum = 0
> for each item in data
    sum = sum + item
  next
> print(sum)
15
```

## Your First Script

Create a file called `hello.pip`:

```vba
' hello.pip - My first PipTable script

' Variables and printing
dim name = "World"
print("Hello, " + name + "!")

' Working with data
dim numbers = [1, 2, 3, 4, 5]
dim total = sum(numbers)
print("Sum of numbers: " + str(total))

' Simple data processing
dim data = {
    "name": "Alice",
    "age": 30,
    "city": "NYC"
}
print("User: " + data.name + " from " + data.city)
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