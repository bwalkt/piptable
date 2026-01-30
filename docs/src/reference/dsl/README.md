# DSL Reference

Complete reference for the PipTable Domain-Specific Language.

## Overview

PipTable combines VBA-like syntax with SQL capabilities to create an intuitive data processing language. This reference covers all language constructs, operators, and built-in functionality.

## Language Structure

### Program Structure
- A PipTable program consists of a sequence of statements
- Comments start with `'` and continue to end of line
- Statements can span multiple lines
- Case-insensitive keywords (but case-sensitive identifiers)

### Type System
PipTable supports these data types:
- **Primitives**: `int`, `float`, `string`, `bool`, `null`
- **Collections**: `array`, `object`
- **Data**: `table` (sheets/dataframes)
- **Special**: `function`, `duration`, `timestamp`

## Categories

### [Statements](statements.md)
Control program flow and data operations:
- Variable declarations (`dim`)
- Control flow (`if`, `for`, `while`)
- Data operations (`import`, `export`, `append`, `upsert`)
- Functions

### [Expressions](expressions.md)
Compute and transform values:
- Arithmetic operations
- Logical operations
- SQL queries (`query()`)
- HTTP requests (`fetch()`)
- AI queries (`ask()`)

### [Operators](operators.md)
Combine and compare values:
- Arithmetic: `+`, `-`, `*`, `/`, `%`
- Comparison: `=`, `<>`, `<`, `>`, `<=`, `>=`
- Logical: `and`, `or`, `not`
- Special: `like`, `in`, `is null`

## Quick Reference

### Common Patterns

```vba
' Load data
dim data = import "file.csv" into sheet

' Transform with SQL
dim result = query("SELECT * FROM data WHERE value > 100")

' Join operations
dim combined = users join orders on "user_id"

' Export results
export result to "output.xlsx"
```

### Data Pipeline Example

```vba
' Import multiple files
dim sales = import "sales_*.csv" into book

' Consolidate sheets
dim all_sales = sales.consolidate()

' Append new data
all_sales append distinct new_sales on "id"

' Analyze
dim summary = query("
    SELECT product, SUM(amount) as total
    FROM all_sales
    GROUP BY product
")

' Export
export summary to "report.xlsx"
```

## See Also

- [Built-in Functions](../api/functions.md)
- [Sheet API](../api/sheet.md)
- [File Formats](../api/formats.md)
