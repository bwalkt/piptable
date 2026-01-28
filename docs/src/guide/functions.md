# Functions

Creating and using functions in PipTable, including lambda expressions for functional programming.

## Function Definitions

PipTable supports both named functions and anonymous lambda expressions.

### Named Functions

```pip
function add(a, b)
    return a + b
end function
```

### Lambda Expressions

Lambda expressions provide a concise way to create anonymous functions:

```pip
# Simple lambda with one parameter
dim add_one = |x| x + 1

# Lambda with multiple parameters
dim multiply = |a, b| a * b

# Lambda with no parameters
dim get_random = || random()
```

## Using Lambdas with Sheet Operations

Lambda expressions are particularly useful for data transformation operations on sheets:

### Map Operation

Transform every row in a sheet:

```pip
# Transform each row object (for sheets with named columns)
dim processed = data.map(|row| {
    ...row,
    name: upper(row.name),
    age: row.age + 1
})

# Transform each row array (for sheets without named columns)
dim adjusted = numbers.map(|row| [row[0], row[1] + 10, row[2] * 2])
```

### Filter Operation

Filter rows based on a condition:

```pip
# Keep only rows where age is greater than 18
dim adults = people.filter(|row| row.age > 18)

# Filter based on multiple conditions
dim filtered = data.filter(|row| row.status = "active" and row.score > 50)
```

## Lambda Syntax

Lambda expressions use the `|param1, param2, ...| expression` syntax:

- `|x| x * 2` - Single parameter lambda
- `|a, b| a + b` - Multiple parameters
- `|| 42` - No parameters
- `|x| x > 0 and x < 100` - Complex expression body

## Built-in Functions

PipTable provides many built-in functions for common operations:

### String Functions
- `upper(str)` - Convert to uppercase
- `lower(str)` - Convert to lowercase  
- `trim(str)` - Remove whitespace

### Math Functions
- `round(n, decimals)` - Round number
- `floor(n)` - Round down
- `ceil(n)` - Round up

### Sheet Functions
- `sheet_row_count(sheet)` - Get number of rows
- `sheet_col_count(sheet)` - Get number of columns
- `sheet_transpose(sheet)` - Transpose rows/columns

## Examples

### Data Transformation Pipeline

```pip
# Load data and apply transformations
import "sales_data.csv" into sales

# Clean and transform the data using lambdas
dim clean_data = sales
    .map(|cell| trim(cell))                    # Remove whitespace
    .filter(|row| row.amount > 0)              # Keep positive amounts
    .map(|row| {...row, amount: round(row.amount, 2)})  # Round amounts

export clean_data to "processed_sales.csv"
```

### Custom Calculations

```pip
# Apply calculations directly in the lambda
dim results = data.map(|row| {
    ...row,
    tax: row.subtotal * 0.08,
    total: "$" + string(round(row.subtotal * 1.08, 2))
})

# Or define reusable calculation logic inline
dim with_tax = data.map(|row| {
    dim tax_rate = 0.08
    dim tax_amount = row.subtotal * tax_rate
    return {...row, tax: tax_amount, total: row.subtotal + tax_amount}
})
```

## Functional Programming Patterns

Lambda expressions enable functional programming patterns in PipTable:

### Composition
```pip
# Compose multiple transformations
dim transform = |x| round(x * 1.1, 2)
dim result = data.map(transform)
```

### Chaining Operations
```pip
# Chain multiple transformations using map and filter
dim processed = data
    .filter(|row| row.status = "active")
    .map(|row| {...row, price: row.price * 1.1})
    .filter(|row| row.price > 100)

# Or combine transformations in a single map
dim result = data.map(|row| {
    dim new_price = row.price * 1.1
    return row.status = "active" and new_price > 100 
        ? {...row, price: new_price, eligible: true}
        : {...row, eligible: false}
})
```