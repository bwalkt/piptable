# piptable CLI Examples

piptable uses a VBA-like DSL with SQL integration. Run scripts with:

```bash
# Execute inline script
pip -e 'print("Hello")'

# Execute script file
pip script.pip

# Start REPL
pip -i

# With variables
pip -D name=Alice -e 'print(name)'
```

## Basic Syntax

### Variables (dim)

```vba
dim x = 5
dim name = "Alice"
dim active = true
dim data = null
```

### Arithmetic

```vba
dim a = 10
dim b = 3

print(a + b)   ' 13
print(a - b)   ' 7
print(a * b)   ' 30
print(a / b)   ' 3.333...
print(a % b)   ' 1 (modulo)
print(-a)      ' -10
```

### Arrays

```vba
dim arr = [1, 2, 3, 4, 5]

print(arr[0])      ' First element: 1
print(arr[-1])     ' Last element: 5
print(len(arr))    ' Length: 5

arr[0] = 10        ' Modify element
```

### Objects

```vba
dim person = {"name": "Alice", "age": 30, "city": "NYC"}

print(person.name)       ' Alice
print(person["age"])     ' 30

person.age = 31          ' Modify field
```

## Control Flow

### If-Else

```vba
dim x = 15

if x > 10 then
  print("large")
elseif x > 5 then
  print("medium")
else
  print("small")
end if
```

### For Loop

```vba
' Count 1 to 5
for i = 1 to 5
  print(i)
next

' Count with step
for i = 0 to 10 step 2
  print(i)
next

' Count down
for i = 5 to 1 step -1
  print(i)
next
```

### For Each Loop

```vba
dim fruits = ["apple", "banana", "cherry"]

for each fruit in fruits
  print(fruit)
next
```

### While Loop

```vba
dim i = 0
while i < 5
  print(i)
  i = i + 1
wend
```

## Functions

### Define and Call

```vba
function add(a, b)
  return a + b
end function

print(add(3, 4))   ' 7
```

### Recursive Function

```vba
function factorial(n)
  if n <= 1 then
    return 1
  end if
  return n * factorial(n - 1)
end function

print(factorial(5))   ' 120
```

### Sub Procedures (no return)

```vba
function greet(name)
  print("Hello, " + name + "!")
end function

greet("World")
```

## Built-in Functions

### Math Functions

```vba
print(abs(-42))           ' 42
print(sum([1, 2, 3]))     ' 6
print(avg([1, 2, 3]))     ' 2.0
print(min([5, 2, 8]))     ' 2
print(max([5, 2, 8]))     ' 8
```

### Type Functions

```vba
print(type(42))           ' Int
print(type(3.14))         ' Float
print(type("hi"))         ' String
print(type([1,2]))        ' Array

print(int("42"))          ' 42
print(float("3.14"))      ' 3.14
print(str(42))            ' "42"
```

### Collection Functions

```vba
dim arr = [1, 2, 3]
print(len(arr))           ' 3
print(len("hello"))       ' 5

dim obj = {"a": 1, "b": 2}
print(keys(obj))          ' ["a", "b"]
print(values(obj))        ' [1, 2]
```

## HTTP Fetch

```vba
' Fetch JSON data
dim data = fetch("https://api.github.com/users/octocat")
print(data.name)
print(data.login)

' Iterate over array response
dim users = fetch("https://jsonplaceholder.typicode.com/users")
for each user in users
  print(user.name)
next
```

## SQL Queries

```vba
' Query from array data
dim people = [
  {"name": "Alice", "age": 30},
  {"name": "Bob", "age": 25},
  {"name": "Charlie", "age": 35}
]

dim result = query(select name, age from people where age > 28)
print(result)
```

## Output Formats

```bash
# JSON output
pip -f json -e 'dim x = {"a": 1, "b": 2}
x'

# CSV output
pip -f csv -e 'dim data = [["name","age"],["Alice","30"]]
data'

# Table output (default)
pip -f table -e 'dim data = [[1,2,3],[4,5,6]]
data'
```

## Sample Scripts

### fibonacci.pip

```vba
' Calculate Fibonacci sequence
function fib(n)
  if n <= 1 then
    return n
  end if
  return fib(n - 1) + fib(n - 2)
end function

for i = 0 to 10
  print(fib(i))
next
```

### fizzbuzz.pip

```vba
' Classic FizzBuzz
for i = 1 to 20
  if i % 15 == 0 then
    print("FizzBuzz")
  elseif i % 3 == 0 then
    print("Fizz")
  elseif i % 5 == 0 then
    print("Buzz")
  else
    print(i)
  end if
next
```

### sum_array.pip

```vba
' Sum numbers in an array
dim numbers = [10, 20, 30, 40, 50]

dim total = 0
for each n in numbers
  total = total + n
next

print("Sum: " + str(total))
print("Average: " + str(total / len(numbers)))
```

### object_manipulation.pip

```vba
' Work with objects
dim users = [
  {"name": "Alice", "score": 85},
  {"name": "Bob", "score": 92},
  {"name": "Charlie", "score": 78}
]

dim highest = 0
dim top_user = ""

for each user in users
  if user.score > highest then
    highest = user.score
    top_user = user.name
  end if
next

print("Top scorer: " + top_user + " with " + str(highest))
```

## Comments

```vba
' This is a single-line comment
dim x = 5  ' Inline comment

' Multi-line comments use multiple single quotes
' Line 1
' Line 2
```

## Running Scripts

Save any example to a `.pip` file and run:

```bash
pip fibonacci.pip
pip fizzbuzz.pip
pip -v script.pip  # verbose mode
```
