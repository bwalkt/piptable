# Formulas in the DSL

This page shows how to call spreadsheet-style formula functions from the piptable DSL and how to evaluate formulas against sheet data.

## Call Formula Functions

The DSL exposes the formula registry, so you can call formula functions directly:

```piptable
dim total = SUM(1, 2, 3)
dim label = IF(1, "yes", "no")
dim left = LEFT("hello", 2)
dim joined = CONCAT("a", "b", "c")
```

Formula functions follow Excel-style casing. Use uppercase names to avoid collisions
with DSL built-ins like `sum`, `avg/average`, `min`, `max`, and `len`.

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
dim cached = sheet_get_a1_eval(sales, "B1")
```

Notes:
- `sheet_get_cell_value` evaluates formulas stored as strings in cells (e.g., `"=SUM(A1:A2)"`).
- When a formula references a cell that contains another formula string, it is treated as a string value (no recursive evaluation yet).
