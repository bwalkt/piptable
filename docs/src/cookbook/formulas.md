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
Use the formula function names consistently in scripts; there is no separate
DSL implementation for lookup formulas.

Aggregate functions like `SUM`, `AVERAGE`, `MIN`, and `MAX` accept arrays or
ranges.

## Lookup and Reference Formulas

Lookup formulas operate on arrays or sheet ranges:

```piptable
dim products = [
  ["Apple", 1.50, 100],
  ["Banana", 0.75, 200],
  ["Cherry", 2.00, 150]
]

dim price = vlookup("Banana", products, 2, false)
dim row = match("Cherry", products, 0)
dim qty = index(products, row, 3)

dim names = ["Apple", "Banana", "Cherry"]
dim prices = [1.50, 0.75, 2.00]
dim safe_price = xlookup("Date", names, prices, 0.0)
dim wildcard_price = xlookup("App*", names, prices, 0.0, 2)
dim ci_wildcard = xlookup("app*", names, prices, 0.0, 2, 1, true)
```

`OFFSET` builds a subrange from a range or 2D array:

```piptable
dim block = offset(products, 1, 0, 1, 2)  ' returns [["Banana", 0.75]]
```

`XLOOKUP` supports binary search for sorted arrays with `search_mode = 2` (ascending) or `-2` (descending).

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

R1C1 notation is also supported in formulas:

```piptable
dim total_r1c1 = sheet_eval_formula(sales, "SUM(R1C1:R10C1)")
dim relative = sheet_get_cell_value(sales, "B2")  ' "=R[-1]C[-1]" in B2
```

Notes:
- `sheet_get_cell_value` evaluates formulas stored as strings in cells (e.g., `"=SUM(A1:A2)"`).
- When a formula references a cell that contains another formula string, it is treated as a string value (no recursive evaluation yet).
- Formula errors include context and the original formula text (e.g., `Formula error in sheet_eval_formula: ... (formula: "...")`).
  For lookups, a not-found result is a formula error (e.g., `#N/A`) unless you pass `if_not_found` to `xlookup`.
