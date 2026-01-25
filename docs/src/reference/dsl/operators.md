# Operators

Operators combine values and expressions in PipTable.

## Arithmetic Operators

Perform mathematical calculations on numeric values.

| Operator | Description | Example | Result |
|----------|-------------|---------|--------|
| `+` | Addition | `5 + 3` | `8` |
| `-` | Subtraction | `10 - 4` | `6` |
| `*` | Multiplication | `3 * 4` | `12` |
| `/` | Division | `15 / 3` | `5` |
| `%` | Modulo (remainder) | `10 % 3` | `1` |
| `-` | Negation (unary) | `-5` | `-5` |

**Examples:**
```piptable
dim total = price * quantity
dim average = sum / count
dim remainder = value % 10
dim negative = -amount
```

## String Operators

Combine and manipulate string values.

| Operator | Description | Example | Result |
|----------|-------------|---------|--------|
| `+` | Concatenation | `"Hello" + " " + "World"` | `"Hello World"` |

**Examples:**
```piptable
dim fullName = firstName + " " + lastName
dim message = "Total: $" + str(amount)
dim path = folder + "/" + filename
```

## Comparison Operators

Compare values and return boolean results.

| Operator | Description | Example | Result |
|----------|-------------|---------|--------|
| `=` | Equal to | `5 = 5` | `true` |
| `==` | Equal to (alternative) | `5 == 5` | `true` |
| `<>` | Not equal to | `5 <> 3` | `true` |
| `!=` | Not equal to (alternative) | `5 != 3` | `true` |
| `<` | Less than | `3 < 5` | `true` |
| `>` | Greater than | `5 > 3` | `true` |
| `<=` | Less than or equal | `3 <= 3` | `true` |
| `>=` | Greater than or equal | `5 >= 5` | `true` |

**Examples:**
```piptable
if age >= 18 then
    print("Adult")
end if

while count < limit
    count = count + 1
wend

dim isValid = value > 0 and value <= 100
```

## Logical Operators

Combine boolean expressions.

| Operator | Description | Example | Result |
|----------|-------------|---------|--------|
| `and` | Logical AND | `true and false` | `false` |
| `or` | Logical OR | `true or false` | `true` |
| `not` | Logical NOT | `not true` | `false` |

**Truth Tables:**

### AND
| A | B | A and B |
|---|---|---------|
| true | true | true |
| true | false | false |
| false | true | false |
| false | false | false |

### OR
| A | B | A or B |
|---|---|---------|
| true | true | true |
| true | false | true |
| false | true | true |
| false | false | false |

**Examples:**
```piptable
' Complex conditions
if age >= 18 and hasLicense then
    print("Can drive")
end if

' Alternative conditions
if status = "active" or status = "pending" then
    process()
end if

' Negation
if not isValid then
    print("Invalid input")
end if

' Short-circuit evaluation
if obj <> null and obj.value > 0 then
    ' Safe to access obj.value
end if
```

## Special Operators

### LIKE

Pattern matching for strings (SQL-style).

```piptable
expression LIKE pattern
```

**Wildcards:**
- `%` - Matches any sequence of characters
- `_` - Matches any single character

**Examples:**
```piptable
' Match email pattern
if email like "%@%.%" then
    print("Valid email format")
end if

' Match specific pattern
if code like "ABC-___" then
    print("Valid code format")
end if

' In queries
dim results = query("SELECT * FROM users WHERE name LIKE 'John%'")
```

### IN

Check if value exists in a list or array.

```piptable
value IN (list)
value IN array
```

**Examples:**
```piptable
' Check membership
if status in ("active", "pending", "approved") then
    process()
end if

' With arrays
dim validCodes = [100, 200, 300]
if code in validCodes then
    print("Valid code")
end if

' In queries
dim results = query("
    SELECT * FROM orders 
    WHERE status IN ('shipped', 'delivered')
")
```

### IS NULL / IS NOT NULL

Check for null values.

```piptable
expression IS NULL
expression IS NOT NULL
```

**Examples:**
```piptable
' Check for null
if value is null then
    value = defaultValue
end if

' Check for not null
if result is not null then
    print(result)
end if

' In queries
dim results = query("
    SELECT * FROM users 
    WHERE email IS NOT NULL
")
```

## Access Operators

Access members of objects and arrays.

| Operator | Description | Example |
|----------|-------------|---------|
| `.` | Object property access | `user.name` |
| `->` | Object property access (alternative) | `user->name` |
| `[]` | Array/object index | `array[0]`, `obj["key"]` |

**Examples:**
```piptable
' Object access
dim name = user.name
dim city = user.address.city
dim value = data->field

' Array access
dim first = items[0]
dim last = items[-1]  ' Negative index from end

' Dynamic property access
dim key = "email"
dim email = user[key]

' Nested access
dim value = data.users[0].profile.settings["theme"]
```

## Type Operators

### Type Assertion (::)

Assert or convert types.

```piptable
expression::type
```

**Examples:**
```piptable
' Type conversion
dim num = "42"::int
dim text = 3.14::string

' Type assertion
dim data = result::table
dim items = response::array
```

## Operator Precedence

Operators are evaluated in this order (highest to lowest):

1. **Parentheses** `()`
2. **Member access** `.`, `->`, `[]`
3. **Function calls** `()`
4. **Type assertion** `::`
5. **Unary** `-`, `+`, `not`
6. **Multiplicative** `*`, `/`, `%`
7. **Additive** `+`, `-`
8. **Comparison** `<`, `>`, `<=`, `>=`, `=`, `<>`, `like`, `in`, `is null`
9. **Logical AND** `and`
10. **Logical OR** `or`
11. **Join** `join`, `left join`, `right join`, `full join`

**Examples:**
```piptable
' Multiplication before addition
dim result = 2 + 3 * 4        ' = 14, not 20

' Parentheses override precedence  
dim result = (2 + 3) * 4      ' = 20

' Comparison before logical
if x > 0 and y > 0 then       ' Compares first, then ANDs

' Complex expression
dim value = -array[index].field + base * rate / 100
' Evaluates as: (-(array[index].field)) + ((base * rate) / 100)
```

## Associativity

Most binary operators are left-associative (evaluate left to right).

```piptable
a + b + c   ' Evaluates as (a + b) + c
a / b / c   ' Evaluates as (a / b) / c
```

## Type Coercion

PipTable performs automatic type conversion in some cases:

### String Concatenation
Any value concatenated with a string becomes a string:
```piptable
dim text = "Value: " + 42           ' "Value: 42"
dim msg = "Active: " + true         ' "Active: true"
```

### Numeric Operations
Strings are converted to numbers if possible:
```piptable
dim result = "10" + 5               ' 15 (numeric)
dim value = "3.14" * 2              ' 6.28
```

### Boolean Context
Values are truthy or falsy in boolean contexts:
- **Falsy**: `null`, `false`, `0`, `""`, `[]`, `{}`
- **Truthy**: Everything else

```piptable
if array then                       ' True if non-empty
    print("Array has items")
end if

if text then                        ' True if non-empty string
    print("Text exists")
end if
```

## See Also

- [Expressions](expressions.md) - Building expressions
- [Statements](statements.md) - Using operators in statements
- [SQL Reference](query.md) - SQL operators in queries