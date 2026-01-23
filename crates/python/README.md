# piptable

A fast, pyexcel-like library for working with tabular data (CSV, Excel) built in Rust.

## Installation

```bash
pip install piptable
```

## Quick Start

```python
from piptable import Sheet, Book

# Create a sheet from data
sheet = Sheet.from_data([
    ["Name", "Age", "City"],
    ["Alice", 30, "NYC"],
    ["Bob", 25, "LA"],
])

# Access data
print(sheet.row_count())  # 3
print(sheet.col_count())  # 3

# Name columns by first row
sheet.name_columns_by_row(0)

# Access by column name
ages = sheet.column_by_name("Age")
print(ages)  # [Age, 30, 25]

# Save to CSV
sheet.save_as_csv("output.csv")

# Save to Excel
sheet.save_as_xlsx("output.xlsx")
```

## Loading Data

### From CSV

```python
# Basic load
sheet = Sheet.from_csv("data.csv")

# With headers
sheet = Sheet.from_csv("data.csv", has_headers=True)

# With custom delimiter
sheet = Sheet.from_csv("data.tsv", delimiter="\t")
```

### From Excel

```python
# Load first sheet
sheet = Sheet.from_xlsx("workbook.xlsx")

# Load specific sheet
sheet = Sheet.from_xlsx_sheet("workbook.xlsx", "Sheet2")

# Load with headers
sheet = Sheet.from_xlsx("workbook.xlsx", has_headers=True)

# Load all sheets as a Book
book = Book.from_xlsx("workbook.xlsx")
```

## Sheet Operations

### Cell Access

```python
# By index (0-based)
value = sheet.get(0, 1)  # row 0, col 1
sheet.set(0, 1, "new value")

# By column name (after naming columns)
sheet.name_columns_by_row(0)
value = sheet.get_by_name(1, "Age")
```

### Row Operations

```python
# Get a row
row = sheet.row(0)

# Append a row
sheet.row_append(["Charlie", 35, "Chicago"])

# Insert a row
sheet.row_insert(1, ["Dave", 28, "Boston"])

# Delete a row
sheet.row_delete(2)
```

### Column Operations

```python
# Get a column
col = sheet.column(1)

# Get by name
ages = sheet.column_by_name("Age")

# Append a column
sheet.column_append([True, False, True])

# Delete a column
sheet.column_delete(2)
sheet.column_delete_by_name("City")
```

## Book Operations

```python
# Create a book
book = Book()

# Add sheets
book.add_sheet("Sales", sales_sheet)
book.add_sheet("Expenses", expenses_sheet)

# Get sheet names
names = book.sheet_names()  # ["Sales", "Expenses"]

# Get a sheet
sales = book.get_sheet("Sales")

# Remove a sheet
removed = book.remove_sheet("Expenses")

# Save to Excel
book.save_as_xlsx("report.xlsx")
```

## Conversion

```python
# To 2D list
data = sheet.to_list()

# To dictionary (requires named columns)
sheet.name_columns_by_row(0)
data = sheet.to_dict()  # {"Name": [...], "Age": [...], ...}

# To CSV string
csv_string = sheet.to_csv_string()
```

## License

MIT
