# Planned Features

This document summarizes all planned features mentioned throughout the PipTable documentation.

## Language Features

### Operators
- **Automatic type coercion for + operator**: Mixed types currently error, planned to auto-convert
- **String multiplication**: Support for `"=" * 40` style operations

### Statements & Expressions
- **Glob patterns in imports**: Currently only comma-separated lists, e.g., `import "*.csv"`
- **Import options**: Additional options like delimiter and encoding
- **Export options**: Support for `export data to "file.csv" with {"delimiter": "|"}`
- **Method calls on objects**: `data.sort()`, `sheet.filter(condition)`
- **Chained joins**: Direct chaining without intermediate variables

## Functions

### Type Conversion
- `bool(value)` - Convert to boolean

### String Functions
- `upper(str)` - Convert to uppercase
- `lower(str)` - Convert to lowercase  
- `trim(str)` - Remove whitespace
- `substr(str, start, len)` - Extract substring
- `concat(str1, str2, ...)` - Concatenate strings
- `split(str, delimiter)` - Split string
- `replace(str, old, new)` - Replace substring

### Math Functions
- `round(n, decimals)` - Round number
- `floor(n)` - Round down
- `ceil(n)` - Round up

### Array Functions
- `first(array)` - First element
- `last(array)` - Last element
- `push(array, item)` - Add to end
- `pop(array)` - Remove last
- `slice(array, start, end)` - Extract subarray
- `contains(array, item)` - Check membership
- `reverse(array)` - Reverse array
- `sort(array)` - Sort array

### Object Functions
- `has(object, key)` - Check key exists
- `merge(obj1, obj2)` - Merge objects

### Utility Functions
- `now()` - Current timestamp
- `uuid()` - Generate UUID
- `random()` - Random 0-1
- `random(min, max)` - Random in range

## Python Integration

All advanced functions listed in the Python UDFs section are available when Python integration is enabled, including:
- Regular expression operations
- Advanced date/time calculations
- Statistical functions
- Complex array operations

## Implementation Priority

Based on common use cases, suggested implementation order:

1. **High Priority** (Core functionality)
   - `bool()` conversion
   - `upper()`, `lower()`, `trim()`
   - `round()`, `floor()`, `ceil()`
   - `now()` for timestamps

2. **Medium Priority** (Common operations)
   - String functions: `substr()`, `split()`, `replace()`
   - Array functions: `first()`, `last()`, `contains()`
   - `random()` functions
   - Glob patterns for imports

3. **Low Priority** (Nice to have)
   - Method calls on objects
   - Export/import options
   - Chained joins
   - Advanced array operations

## Notes

- All planned features are marked with 📋 in the main documentation
- Python UDFs can provide many of these functions as a workaround
- The hybrid Rust/Python approach allows incremental implementation