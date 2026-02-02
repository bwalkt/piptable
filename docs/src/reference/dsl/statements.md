# Statements

Statements control program flow and perform operations in PipTable.

## Variable Declaration

### dim

Declares and initializes a variable.

```piptable
dim name = value
dim name: type = value  ' With type hint
```

**Examples:**
```piptable
dim count = 0
dim name: string = "Alice"
dim data: table = import "file.csv" into sheet
dim numbers: array = [1, 2, 3, 4, 5]
```

## Assignment

Updates the value of an existing variable.

```piptable
variable = value
object.field = value
array[index] = value
```

**Examples:**
```piptable
count = count + 1
user.age = 30
scores[0] = 100
```

## Control Flow

### if/then/else

Conditional execution based on boolean expressions.

```piptable
if condition then
    ' statements
elseif other_condition then
    ' statements
else
    ' statements
end if
```

**Examples:**
```piptable
if age >= 18 then
    print("Adult")
elseif age >= 13 then
    print("Teenager")
else
    print("Child")
end if
```

### for

Traditional loop with counter.

```piptable
for variable = start to end [step value]
    ' statements
next [variable]
```

**Examples:**
```piptable
' Count from 1 to 10
for i = 1 to 10
    print(i)
next

' Count by 5s
for i = 0 to 100 step 5
    print(i)
next

' Countdown
for i = 10 to 1 step -1
    print(i)
next
```

### for each

Iterate over collections.

```piptable
for each item in collection
    ' statements
next [item]
```

**Examples:**
```piptable
dim fruits = ["apple", "banana", "orange"]
for each fruit in fruits
    print(fruit)
next

dim user = {"name": "Alice", "age": 30}
for each key in keys(user)
    print(key + ": " + str(user[key]))
next
```

### while

Loop while condition is true.

```piptable
while condition
    ' statements
wend
```

**Examples:**
```piptable
dim count = 0
while count < 10
    print(count)
    count = count + 1
wend
```

## Functions

### function

Define a function that returns a value.

Parameters can be annotated with:
- `ByVal` (pass by value) or `ByRef` (pass by reference). `ByVal` is the default.
- `Optional` for default values (optional parameters must include a default and appear after required parameters; `Optional` is `ByVal` only).
- `ParamArray` for variadic arguments (must be the last parameter; receives remaining args as an array; `ParamArray` is `ByVal` only).

```piptable
[async] function name(parameters)
    ' statements
    return value
end function
```

**Examples:**
```piptable
function add(a, b)
    return a + b
end function

function increment(ByRef x)
    x = x + 1
end function

function add_one(ByVal x)
    x = x + 1
    return x
end function

function greet(name, Optional suffix = "!")
    return "Hello " + name + suffix
end function

function sum_all(ParamArray nums)
    return sum(nums)
end function

function factorial(n)
    if n <= 1 then
        return 1
    end if
    return n * factorial(n - 1)
end function

async function fetchData(url)
    dim response = await fetch(url)
    return response.json()
end function
```

### return

Exit function with optional return value.

```piptable
return [value]
```

**Examples:**
```piptable
function isPositive(n)
    if n > 0 then
        return true
    end if
    return false
end function
```

### call

Invoke a function (optional keyword).

```piptable
call myFunc(arguments)
myFunc(arguments)  ' call is optional
```

**Examples:**
```piptable
call printHeader("Report")
logError("File not found")
```

## Data Operations

### import

Load data from files.

```piptable
import file_pattern into sheet [options]
import file_pattern into book [options]
```

**Options:**
- `without headers` - First row is data, not headers
- `sheet "name"` - Specific sheet from Excel
- `with { ... }` - Import options (e.g., PDF page range, structure extraction)

**Examples:**
```piptable
' Single CSV file
dim data = import "sales.csv" into sheet

' Multiple files into book (exact paths only, glob patterns planned)
dim all_data = import "sales_2023.csv,sales_2024.csv" into book

' Without headers
dim raw = import "data.csv" into sheet without headers

' Specific Excel sheet
dim report = import "workbook.xlsx" sheet "Report" into sheet

' PDF structure extraction (returns JSON object)
dim doc = import "paper.pdf" into sheet with { "extract_structure": true }

' Note: Additional options like delimiter and encoding are planned features
```

### export

Save data to files, with optional append mode for incremental data building.

```piptable
export data to file [append]
```

**Examples:**
```piptable
' Various formats (format detected from extension)
export data to "output.csv"
export data to "output.xlsx"
export data to "output.json"
export data to "output.parquet"

' Append mode (currently supported for CSV and TSV)
export new_data to "existing.csv" append
export log_entries to "events.tsv" append

' Append creates file if it doesn't exist
export first_batch to "new_file.csv" append

' Note: Export options are planned but not yet implemented
' Future: export data to "output.csv" with {"delimiter": "|"}
```

**Append Mode Notes:**
- Currently supported only for CSV and TSV files
- Automatically validates column compatibility when appending
- Creates the file if it doesn't exist
- Preserves the existing data and adds new rows at the end

### append

Add rows to existing data.

```piptable
target append source
target append distinct source on key
```

**Examples:**
```piptable
' Basic append
users append new_users

' Append unique rows only
users append distinct new_users on "email"

' Append with duplicate check on multiple columns
orders append distinct new_orders on "order_id"
```

### upsert

Update existing rows or insert new ones.

```piptable
target upsert source on key
```

**Examples:**
```piptable
' Update or insert based on ID
users upsert updates on "user_id"

' Upsert with email as key
customers upsert new_data on "email"
```

## Visualization

### chart

Create data visualizations.

```piptable
chart type title
    option: value
    ' more options
end chart
```

**Types:** `bar`, `line`, `pie`, `scatter`, `area`

**Examples:**
```piptable
chart bar "Sales by Region"
    data: regional_sales
    x: "region"
    y: "total"
    color: "blue"
end chart

chart line "Monthly Trend"
    data: monthly_data
    x: "month"
    y: "revenue"
    title: "Revenue Over Time"
end chart
```

## Expression Statements

Any expression can be used as a statement.

```piptable
expression
```

**Examples:**
```piptable
' Function calls
print("Hello, World!")
len(data)

' SQL queries
query("UPDATE users SET active = true")

' Note: Method calls on objects are planned features
' Future: data.sort(), sheet.filter(condition)
```

## Comments

Document your code with comments.

```piptable
' This is a comment
dim x = 42  ' Inline comment
```

**Examples:**
```piptable
' ============================================
' Data Processing Pipeline
' Author: Alice Smith
' Date: 2024-01-15
' ============================================

' Load sales data from multiple sources
' NOTE: Glob patterns are planned; currently use comma-separated list
dim sales = import "sales_2023.csv,sales_2024.csv" into book

' TODO: Add validation for negative values
' NOTE: This assumes USD currency
```

## See Also

- [Expressions](expressions.md) - Computing values
- [Operators](operators.md) - Combining expressions
- [Built-in Functions](../api/functions.md) - Available functions
