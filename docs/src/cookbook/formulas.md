# Formulas in the DSL

This page shows how to call spreadsheet-style formula functions from the piptable DSL and how to evaluate formulas against sheet data.

## Call Formula Functions

Formula functions are callable directly from the DSL. Names are case-insensitive
and resolve to the same implementation:

```piptable
dim total = SUM(1, 2, 3)
dim label = IF(1, "yes", "no")
dim left = LEFT("hello", 2)
dim joined = CONCAT("a", "b", "c")
```

Formula function names are case-insensitive (`sum`, `Sum`, and `SUM` all work).
When a DSL function name matches a formula function, the formula implementation
is used.

Aggregate functions like `SUM`, `AVERAGE`, `MIN`, and `MAX` accept arrays or
ranges.

## Evaluate Formulas Against Sheets

Retrieve raw formula text or computed values:

```piptable
dim raw = sheet_get_cell(sales, "B1")       ' returns "=SUM(A1:A2)" if that's stored
dim value = sheet_get_cell_value(sales, "B1") ' returns the evaluated result
dim is_formula = is_sheet_cell_formula(sales, "B1")
```

Use these helpers to evaluate formulas against sheet data:

```piptable
import sales from "sales.csv"

dim a1 = sheet_get_cell_value(sales, "B1")
dim total = sheet_eval_formula(sales, "SUM(A1:A10)")
dim total_short = sum(sales, "A1:A10")
dim average_short = avg(sales, "A1:A10")
dim min_short = min(sales, "A1:A10")
dim max_short = max(sales, "A1:A10")
dim count_short = count(sales, "A1:A10")
dim counta_short = counta(sales, "A1:A10")
dim cached = sheet_get_a1_eval(sales, "B1")
```

Notes:
- `sheet_get_cell_value` evaluates formulas stored as strings in cells (e.g., `"=SUM(A1:A2)"`).
- When a formula references a cell that contains another formula string, it is treated as a string value (no recursive evaluation yet).
