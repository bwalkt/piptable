# Core Concepts

Understanding these fundamental concepts will help you write effective PipTable scripts.

## Variables and Types

### Variable Declaration

Variables are declared with the `dim` keyword:

```vba
dim name = "Alice"           ' String
dim age = 30                 ' Integer  
dim price = 19.99           ' Float
dim isActive = true         ' Boolean
dim data = null             ' Null
```

### Type Hints (Optional)

Add type hints for clarity and validation:

```vba
dim name: string = "Alice"
dim age: int = 30
dim price: float = 19.99
dim isActive: bool = true
dim records: array = []
dim config: object = {}
dim sales: table = import "data.csv" into sheet
```

### Type Conversion

Convert between types using built-in functions:

```vba
dim x = "42"
dim num = int(x)        ' String to integer
dim text = str(num)     ' Integer to string
dim decimal = float(x)  ' String to float
```

## Data Structures

### Arrays

Ordered collections of values:

```vba
' Create arrays
dim numbers = [1, 2, 3, 4, 5]
dim mixed = ["text", 42, true, null]

' Access elements
dim first = numbers[0]     ' 1
dim last = numbers[-1]      ' 5

' Array operations
numbers[2] = 10            ' Update element
dim length = len(numbers)  ' Get length
```

### Objects

Key-value pairs (dictionaries):

```vba
' Create objects
dim person = {
    "name": "Alice",
    "age": 30,
    "city": "NYC"
}

' Access properties
dim name = person.name       ' Dot notation
dim age = person["age"]      ' Bracket notation

' Update properties
person.email = "alice@example.com"
person["phone"] = "555-0123"
```

### Tables (DataFrames)

Tables are the primary data structure for tabular data:

```vba
' Create from import
dim data = import "file.csv" into sheet

' Create from query
dim result = query("SELECT * FROM data WHERE value > 100")

' Access table data
dim rowCount = len(data)
dim firstRow = data[0]
```

## Control Flow

### If Statements

```vba
if age >= 18 then
    print("Adult")
elseif age >= 13 then
    print("Teenager")
else
    print("Child")
end if
```

### For Loops

```vba
' Traditional for loop
for i = 1 to 10
    print(i)
next

' With step
for i = 0 to 100 step 10
    print(i)
next

' Reverse
for i = 10 to 1 step -1
    print(i)
next
```

### For Each Loops

```vba
' Iterate over array
dim items = ["apple", "banana", "orange"]
for each item in items
    print(item)
next

' Iterate over object
dim config = {"host": "localhost", "port": 3000}
for each key in keys(config)
    print(key + ": " + str(config[key]))
next
```

### While Loops

```vba
dim count = 0
while count < 10
    print(count)
    count = count + 1
wend
```

## Functions

### Defining Functions

```vba
' Simple function
function greet(name)
    return "Hello, " + name + "!"
end function

' Function with multiple parameters
function calculateTotal(price, quantity, taxRate)
    dim subtotal = price * quantity
    dim tax = subtotal * taxRate
    return subtotal + tax
end function

' Using functions
dim message = greet("Alice")
dim total = calculateTotal(19.99, 3, 0.08)
```

### Subroutines

Procedures that don't return values:

```vba
sub logMessage(level, message)
    dim timestamp = now()
    print("[" + timestamp + "] " + level + ": " + message)
end sub

' Call subroutine
call logMessage("INFO", "Process started")
```

## SQL Queries

The `query()` function enables SQL operations on data:

```vba
' Basic query on variables
dim result = query("SELECT * FROM data")

' Query directly from files (CSV, JSON, Excel, Parquet)
dim excel_data = query("SELECT * FROM 'sales.xlsx' WHERE amount > 100")
dim csv_data = query("SELECT * FROM 'users.csv' WHERE active = true")

' With conditions
dim filtered = query("
    SELECT name, age 
    FROM users 
    WHERE age >= 21
    ORDER BY name
")

' Aggregations
dim summary = query("
    SELECT 
        department,
        COUNT(*) as count,
        AVG(salary) as avg_salary
    FROM employees
    GROUP BY department
")

' Joins between different sources
dim combined = query("
    SELECT 
        o.order_id,
        c.name,
        o.total
    FROM orders o
    JOIN customers c ON o.customer_id = c.id
")

' Mix files and variables in queries
import "customers.xlsx" into customers
dim sales = query("
    SELECT c.name, s.amount
    FROM customers c
    JOIN 'sales.csv' s ON c.id = s.customer_id
")
```

### Supported Data Sources

SQL queries can operate on:
- **Variables**: Any variable containing table data (automatically registered in SQL engine)
- **CSV files**: `FROM 'file.csv'`
- **Excel files**: `FROM 'file.xlsx'` or `FROM 'file.xls'`
- **JSON files**: `FROM 'file.json'`
- **Parquet files**: `FROM 'file.parquet'`

### Variable Registration

Variables containing table or sheet data are automatically registered in the SQL engine, eliminating the need to export to temporary files:

```vba
' Import data into variables
import "customers.xlsx" into customers
import "sales.csv" into sales

' Variables are automatically available in SQL queries
dim report = query("
    SELECT 
        c.name,
        COUNT(*) as order_count,
        SUM(s.amount) as total_spent
    FROM customers c
    JOIN sales s ON c.id = s.customer_id
    GROUP BY c.name
    ORDER BY total_spent DESC
")

' You can also alias variables in queries
dim top_customers = query("
    SELECT * FROM customers AS c
    WHERE c.lifetime_value > 10000
")
```

## Data Operations

### Import

Load data from files:

```vba
' Single file into sheet
dim data = import "data.csv" into sheet

' Multiple files into book
dim allData = import "*.csv" into book

' With options
dim noHeaders = import "data.csv" into sheet without headers
```

### Export

Save data to files:

```vba
' Export to different formats
export data to "output.csv"
export data to "output.xlsx"
export data to "output.json"
export data to "output.parquet"
```

### Join Operations

Combine data using DSL syntax:

```vba
' Inner join
dim result = users join orders on "id" = "user_id"

' Left join
dim result = users left join orders on "id"

' Right join
dim result = users right join orders on "id"

' Full join
dim result = users full join orders on "id"
```

### Append and Upsert

```vba
' Append new rows
users append newUsers

' Append unique rows only
users append distinct newUsers on "email"

' Update or insert
users upsert updates on "id"
```

## Error Handling

### Try-Catch Pattern

While not yet implemented, planned syntax:

```vba
try
    dim data = import "risky_file.csv" into sheet
catch error
    print("Error: " + error.message)
    dim data = []  ' Default value
end try
```

### Validation

```vba
' Check before operations
if fileExists("data.csv") then
    dim data = import "data.csv" into sheet
else
    print("File not found")
end if

' Validate data
if len(data) > 0 then
    ' Process data
else
    print("No data to process")
end if
```

## Best Practices

1. **Use meaningful variable names**
   ```vba
   ' Good
   dim customerOrders = import "orders.csv" into sheet
   
   ' Avoid
   dim d = import "orders.csv" into sheet
   ```

2. **Add comments for complex logic**
   ```vba
   ' Calculate compound interest
   ' Formula: A = P(1 + r/n)^(nt)
   dim amount = principal * pow(1 + rate/periods, periods * time)
   ```

3. **Break complex queries into steps**
   ```vba
   ' Step 1: Filter active users
   dim activeUsers = query("SELECT * FROM users WHERE active = true")
   
   ' Step 2: Get their orders
   dim orders = query("SELECT * FROM orders WHERE user_id IN ...")
   ```

4. **Use functions for reusable code**
   ```vba
   function formatCurrency(amount)
       return "$" + str(round(amount, 2))
   end function
   ```

5. **Validate inputs**
   ```vba
   if age < 0 or age > 150 then
       print("Invalid age value")
       return
   end if
   ```

## Next Steps

- [Variables and Types](variables-types.md) - Detailed type system
- [Data Structures](data-structures.md) - Arrays, objects, and tables
- [Control Flow](control-flow.md) - Conditionals and loops
- [Functions](functions.md) - Creating reusable code