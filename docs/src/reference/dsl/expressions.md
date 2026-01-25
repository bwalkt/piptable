# Expressions

Expressions compute values in PipTable. They can be used in variable assignments, conditions, and as arguments to functions.

## Literals

### Primitive Literals

Basic values that can be written directly in code.

```vba
42              ' Integer
3.14159         ' Float
"Hello"         ' String
true            ' Boolean
false           ' Boolean
null            ' Null value
```

**String Escapes:**
```vba
"Line 1\nLine 2"     ' Newline
"Tab\there"          ' Tab
"Quote: \"Hi\""      ' Escaped quote
"Path: C:\\Users"    ' Escaped backslash
```

### Array Literals

Ordered collections of values.

```vba
[]                          ' Empty array
[1, 2, 3]                  ' Number array
["a", "b", "c"]            ' String array
[1, "two", true, null]     ' Mixed types
[[1, 2], [3, 4]]          ' Nested arrays
```

### Object Literals

Key-value pairs (dictionaries/maps).

```vba
{}                          ' Empty object
{"name": "Alice"}          ' Single property
{
    "name": "Bob",
    "age": 30,
    "active": true
}                          ' Multiple properties
{"key": [1, 2, 3]}        ' Nested structures
```

### Interval Literals

Time duration values.

```vba
interval 5 seconds
interval 30 minutes
interval 2 hours
interval 7 days
interval 1 month
interval 1 year
```

## Field Access

Access properties and elements of objects and arrays.

### Dot Notation

Access object properties.

```vba
object.property
object.nested.property
```

**Examples:**
```vba
user.name
config.database.host
response.data.items[0].id
```

### Bracket Notation

Access with computed keys or array indices.

```vba
object["property"]
object[variable]
array[index]
array[-1]  ' Negative indexing from end
```

**Examples:**
```vba
user["full name"]      ' Property with space
data[key]             ' Dynamic property
scores[0]             ' First element
items[-1]             ' Last element
matrix[i][j]          ' 2D array
```

## Function Calls

Invoke functions with arguments.

```vba
function()
function(arg1)
function(arg1, arg2, ...)
```

**Examples:**
```vba
print("Hello")
len(array)
sum([1, 2, 3])
max(scores)
substr(text, 0, 10)
```

## SQL Queries

### query()

Execute SQL on data.

```vba
query("SQL statement")
```

**Examples:**
```vba
' Simple select
dim result = query("SELECT * FROM users")

' With conditions
dim adults = query("
    SELECT name, age 
    FROM users 
    WHERE age >= 18
")

' Aggregation
dim stats = query("
    SELECT 
        department,
        COUNT(*) as count,
        AVG(salary) as avg_salary
    FROM employees
    GROUP BY department
")

' Join
dim report = query("
    SELECT 
        u.name,
        o.order_date,
        o.total
    FROM users u
    JOIN orders o ON u.id = o.user_id
    WHERE o.total > 100
")
```

## HTTP Operations

### fetch()

Make HTTP requests.

```vba
fetch(url)
fetch(url, options)
```

**Examples:**
```vba
' GET request
dim data = fetch("https://api.example.com/users")

' POST with body
dim response = fetch("https://api.example.com/users", {
    "method": "POST",
    "headers": {"Content-Type": "application/json"},
    "body": {"name": "Alice", "age": 30}
})

' Parse JSON response
dim json = fetch(url).json()
```

## AI Operations

### ask()

Query data using natural language.

```vba
ask "question" from data [using model "name"]
```

**Examples:**
```vba
' Basic question
dim answer = ask "What is the average salary?" from employees

' Specific model
dim insight = ask "Find trends in sales data" from sales using model "gpt-4"

' Complex analysis
dim report = ask "
    Summarize the top 3 products by revenue
    and explain their growth patterns
" from sales_data
```

## Join Operations

Combine data from multiple sources.

```vba
left_source join right_source on condition
left_source left join right_source on condition
left_source right join right_source on condition  
left_source full join right_source on condition
```

**Examples:**
```vba
' Inner join
dim result = users join orders on "id" = "user_id"

' Left join with same column
dim all_users = users left join profiles on "id"

' Different column names
dim data = customers join purchases on "cust_id" = "customer_id"

' Note: Chained joins require intermediate variables (planned feature)
dim user_orders = users join orders on "id" = "user_id"
dim report = user_orders join products on "product_id" = "id"
```

## Async Operations

### parallel

Execute multiple operations concurrently.

```vba
parallel
    expression1,
    expression2,
    ...
end parallel
```

**Examples:**
```vba
' Fetch multiple APIs
dim results = parallel
    fetch("https://api1.example.com/data"),
    fetch("https://api2.example.com/data"),
    fetch("https://api3.example.com/data")
end parallel

' Multiple queries
dim reports = parallel
    query("SELECT * FROM sales"),
    query("SELECT * FROM inventory"),
    query("SELECT * FROM customers")
end parallel
```

### async for

Asynchronous iteration.

```vba
async for each item in collection
    ' async operations
end async
```

**Examples:**
```vba
' Process URLs concurrently
dim urls = ["url1", "url2", "url3"]
dim results = async for each url in urls
    fetch(url)
end async
```

### await

Wait for async operation.

```vba
await async_expression
```

**Examples:**
```vba
dim data = await fetch(url)
dim result = await async_function()
```

## Type Assertions

Explicitly specify or convert types.

```vba
expression::type
```

**Examples:**
```vba
"42"::int           ' String to integer
3.14::string        ' Float to string
data::table         ' Assert as table
result::array       ' Assert as array
```

## Parentheses

Group expressions to control evaluation order.

```vba
(expression)
```

**Examples:**
```vba
(a + b) * c         ' Addition before multiplication
(x > 0) and (y > 0) ' Clear boolean grouping
query("SELECT * FROM (" + subquery + ")")
```

## See Also

- [Operators](operators.md) - Combining expressions
- [Built-in Functions](../api/functions.md) - Available functions
- [SQL Reference](query.md) - SQL query syntax