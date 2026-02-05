# Planned: Parameter Validation Rules

Note: This document describes planned validation rules. The syntax and behavior may change.

This document describes the validation rules for function parameters in PipTable DSL.

## Parameter Syntax

### Basic Parameter Declaration

```text
param = { param_modifier? ~ ident }
param_modifier = { ^"byval" | ^"byref" }
```

Parameters consist of an optional modifier followed by an identifier (parameter name).

### Supported Modifiers

- **ByVal** (default): Parameter passed by value - modifications don't affect the original
- **ByRef**: Parameter passed by reference - modifications affect the original variable

### Examples

```vb
' Basic parameters (ByVal by default)
function add(a, b)
    return a + b
end function

' Explicit ByVal and ByRef
function process(ByVal input, ByRef result)
    result = input * 2
    return result
end function

' Mixed parameter modes
function update_data(ByVal multiplier, ByRef data)
    data = data * multiplier
end function
```text

## Validation Rules

### 1. Parameter Names

- Must be valid identifiers (start with letter or underscore, followed by letters, digits, or underscores)
- Cannot be language keywords (e.g., `if`, `then`, `end`, `function`)
- Must be unique within the function parameter list

**Valid Examples:**
```vb
function example(x, y, user_id, _temp)
```

**Invalid Examples:**
```vb
function bad(if, then)      ' Keywords not allowed
function bad2(x, x)         ' Duplicate names not allowed
```text

### 2. Parameter Modifiers

- Only `ByVal` and `ByRef` are supported
- Modifiers are case-insensitive (`byval`, `ByVal`, `BYVAL` are all valid)
- If no modifier is specified, `ByVal` is assumed

**Valid Examples:**
```vb
function test(ByVal a, ByRef b, c)  ' c defaults to ByVal
function test2(byval x, BYREF y)    ' Case insensitive
```

### 3. ByRef Restrictions

ByRef parameters have additional validation at call time:

- **Must be lvalues**: Only variables, array elements, or object fields can be passed to ByRef parameters
- **Cannot pass literals**: Literal values (numbers, strings, etc.) cannot be passed to ByRef parameters

**Valid ByRef Arguments:**
```vb
dim x = 10
dim arr = [1, 2, 3]
dim obj = { value: 42 }

call increment(x)           ' Variable
call increment(arr[0])      ' Array element  
call increment(obj->value)  ' Object field
```text

**Invalid ByRef Arguments:**
```vb
call increment(5)           ' Literal number - ERROR
call increment("hello")     ' Literal string - ERROR
call increment(x + 1)       ' Expression result - ERROR
```

### 4. Parameter Lists

- Parameter lists are comma-separated
- Empty parameter lists are allowed: `function test()`
- No trailing commas allowed

**Valid Examples:**
```vb
function no_params()
function one_param(x)
function multi_params(a, b, c)
```text

**Invalid Examples:**
```vb
function bad(a, b,)        ' Trailing comma not allowed
function bad2(, a, b)      ' Leading comma not allowed
```

## Error Handling

### Parse-Time Errors

These errors are detected during parsing:

1. **Invalid parameter syntax**
   ```vb
   function bad(123abc)    ' Invalid identifier
```text

2. **Invalid modifier**
   ```vb
   function bad(byvalue x) ' Unknown modifier
```

3. **Missing parameter name**
   ```vb
   function bad(ByVal)     ' No parameter name
```text

### Runtime Errors

These errors are detected when the function is called:

1. **ByRef argument must be lvalue**
   ```vb
   function increment(ByRef x)
       x = x + 1
   end function
   call increment(5)       ' Error: ByRef parameter expects variable
```

2. **Wrong number of arguments**
   ```vb
   function add(a, b)
       return a + b
   end function
   dim result = add(1)     ' Error: Missing argument for parameter b
```text

## Implementation Notes

### AST Representation

Parameters are represented in the AST as:

```text
pub struct Param {
    pub name: String,
    pub mode: ParamMode,
}

pub enum ParamMode {
    ByVal,
    ByRef,
}
```

### Parser Implementation

The parser builds parameters using the `build_param` function which:

1. Defaults to `ByVal` mode
2. Extracts the parameter name from the identifier token
3. Updates the mode if a modifier is present
4. Validates that a name is provided

### Interpreter Execution

During function calls, the interpreter:

1. Validates argument count matches parameter count
2. For ByRef parameters, validates arguments are lvalues
3. Creates appropriate variable bindings based on parameter mode
4. Manages reference semantics for ByRef parameters

## Future Enhancements

Potential future parameter features:

1. **Optional parameters with default values**
   ```vb
   function greet(Optional name = "World")
```text

2. **Parameter arrays (variadic parameters)**
   ```vb
   function sum(ParamArray values)
```

3. **Type hints for parameters**
   ```vb
   function calculate(x: Int, y: Float)
```text

These features would require additional validation rules and AST modifications.
