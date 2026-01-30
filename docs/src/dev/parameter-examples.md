# Planned: Complex Parameter Scenarios - Examples

Note: This document describes planned parameter scenarios. The examples and behaviors may change.

This document provides comprehensive examples of complex parameter passing scenarios in PipTable DSL.

## Table of Contents

1. [Basic Parameter Modes](#basic-parameter-modes)
2. [Mixed Parameter Scenarios](#mixed-parameter-scenarios)  
3. [Array and Object Parameter Passing](#array-and-object-parameter-passing)
4. [Error Scenarios](#error-scenarios)
5. [Best Practices](#best-practices)

## Basic Parameter Modes

### ByVal Example - Value Copying

```vb
function double_value(ByVal x)
    x = x * 2
    return x
end function

' Usage
dim original = 10
dim result = double_value(original)
' original = 10 (unchanged)
' result = 20
```

### ByRef Example - Reference Modification

```vb
function increment(ByRef counter)
    counter = counter + 1
end function

' Usage  
dim count = 5
call increment(count)
' count = 6 (modified)
```

## Mixed Parameter Scenarios

### Function with Mixed Parameter Modes

```vb
function process_data(ByVal multiplier, ByRef result, ByVal input_data)
    ' multiplier is copied, changes don't affect original
    multiplier = multiplier + 0.1
    
    ' result is a reference, changes affect original
    result = input_data * multiplier
    
    ' return computed value
    return result + 100
end function

' Usage
dim factor = 2.0
dim output = 0
dim data = 50
dim return_value = process_data(factor, output, data)

' factor = 2.0 (unchanged)
' output = 105 (modified: 50 * 2.1 = 105)  
' return_value = 205 (105 + 100)
```

### Multiple ByRef Parameters

```vb
function swap(ByRef a, ByRef b)
    dim temp = a
    a = b
    b = temp
end function

' Usage
dim x = 10
dim y = 20
call swap(x, y)
' x = 20, y = 10 (values swapped)
```

## Array and Object Parameter Passing

### ByRef with Array Elements

```vb
function update_array_element(ByRef element)
    element = element * 10
end function

' Usage
dim numbers = [1, 2, 3, 4, 5]
call update_array_element(numbers[2])  ' Update index 2
' numbers = [1, 2, 30, 4, 5]
```

### ByRef with Object Fields

```vb
function update_score(ByRef score_field)
    score_field = score_field + 100
end function

' Usage
dim player = { name: "Alice", score: 150 }
call update_score(player->score)
' player = { name: "Alice", score: 250 }
```

### Complex Object Manipulation

```vb
function calculate_stats(ByVal data, ByRef summary)
    ' data is copied, original unchanged
    dim total = 0
    dim count = 0
    
    for each item in data
        total = total + item
        count = count + 1
    next
    
    ' summary is modified by reference
    summary = {
        total: total,
        count: count,
        average: total / count
    }
    
    return summary->average
end function

' Usage
dim scores = [85, 92, 78, 96, 88]
dim stats = null
dim avg = calculate_stats(scores, stats)

' scores = [85, 92, 78, 96, 88] (unchanged)
' stats = { total: 439, count: 5, average: 87.8 }
' avg = 87.8
```

## Error Scenarios

### ByRef Parameter Validation Errors

```vb
function modify_value(ByRef x)
    x = x * 2
end function

' Valid calls
dim variable = 10
call modify_value(variable)        ' ✓ Variable

dim arr = [1, 2, 3]
call modify_value(arr[0])         ' ✓ Array element

dim obj = { value: 5 }
call modify_value(obj->value)     ' ✓ Object field

' Invalid calls - These will generate errors:
call modify_value(5)              ' ❌ Literal value
call modify_value("hello")        ' ❌ String literal  
call modify_value(variable + 1)   ' ❌ Expression result
call modify_value(some_function()) ' ❌ Function return value
```

### Parameter Count Mismatches

```vb
function add_three(a, b, c)
    return a + b + c
end function

' Valid call
dim result = add_three(1, 2, 3)   ' ✓

' Invalid calls:
dim result1 = add_three(1, 2)     ' ❌ Too few arguments
dim result2 = add_three(1, 2, 3, 4) ' ❌ Too many arguments
```

## Best Practices

### 1. Consistent Parameter Naming

```vb
' Good: Clear, descriptive names
function calculate_discount(ByVal price, ByVal discount_rate, ByRef final_price)
    final_price = price * (1 - discount_rate)
    return final_price
end function

' Avoid: Unclear names
function calc(ByVal p, ByVal d, ByRef f)
    f = p * (1 - d)
    return f
end function
```

### 2. Minimize ByRef Usage

Use ByRef only when you need to modify the original variable:

```vb
' Good: Use return value for single result
function double_number(ByVal x)
    return x * 2
end function

' Less preferred: ByRef for single modification
function double_number_byref(ByRef x)
    x = x * 2
end function

' Good use of ByRef: Multiple outputs
function divide_with_remainder(ByVal dividend, ByVal divisor, ByRef remainder)
    remainder = dividend % divisor
    return dividend / divisor
end function
```

### 3. Document Parameter Expectations

```vb
' Function that processes data and updates statistics
' Parameters:
'   input_data (ByVal) - Array of numbers to process
'   stats (ByRef) - Object to store calculated statistics
'   options (ByVal) - Processing options object
' Returns: Processed data array
function process_with_stats(ByVal input_data, ByRef stats, ByVal options)
    ' Implementation...
end function
```

### 4. Validate ByRef Arguments at Runtime

```vb
function update_config(ByRef config_object)
    ' The interpreter automatically validates that config_object
    ' is a valid lvalue (variable, array element, or object field)
    ' No manual validation needed
    
    config_object->last_updated = now()
    config_object->version = config_object->version + 1
end function
```

### 5. Complex Nested Scenarios

```vb
' Advanced example: Function that processes a list of objects,
' updating each object and tracking overall statistics
function process_items(ByVal items, ByRef total_processed, ByRef error_count)
    dim processed = 0
    dim errors = 0
    
    for each item in items
        if validate_item(item) then
            call update_item(item->data)  ' ByRef modification of nested field
            processed = processed + 1
        else
            errors = errors + 1
        end if
    next
    
    total_processed = processed
    error_count = errors
    
    return processed > 0
end function

function validate_item(ByVal item)
    return item != null and item->data != null
end function

function update_item(ByRef data)
    data->processed = true
    data->timestamp = now()
end function

' Usage
dim item_list = [
    { data: { value: 100, processed: false } },
    { data: { value: 200, processed: false } }
]
dim total = 0
dim errors = 0

dim success = process_items(item_list, total, errors)
' total = 2, errors = 0, success = true
' Each item's data->processed = true and has timestamp
```

This comprehensive set of examples demonstrates the flexibility and power of PipTable's parameter passing system while highlighting important validation rules and best practices.
