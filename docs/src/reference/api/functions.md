# Built-in Functions

PipTable uses a hybrid approach for built-in functions: core functions are implemented in Rust for performance, while complex/specialized functions can be provided via Python UDFs.

> **Note:** This documentation includes both currently implemented functions and planned functionality. Functions marked with 📋 are planned but not yet implemented.

## Currently Implemented Functions (Rust)

These functions are available in the current version of PipTable:

### Type Conversion

| Function | Description | Example | Status |
|----------|-------------|---------|--------|
| `int(value)` | Convert to integer | `int("42")` → `42` | ✅ Implemented |
| `float(value)` | Convert to float | `float("3.14")` → `3.14` | ✅ Implemented |
| `str(value)` | Convert to string | `str(42)` → `"42"` | ✅ Implemented |
| `bool(value)` | Convert to boolean | `bool(1)` → `true` | 📋 Planned |

### Core Functions

| Function | Description | Example | Status |
|----------|-------------|---------|--------|
| `len(value)` | Length of text or array | `len("hello")` → `5` | ✅ Implemented |
| `type(value)` | Get type name | `type(42)` → `"int"` | ✅ Implemented |
| `print(...)` | Output values | `print("Hello", name)` | ✅ Implemented |

Note: `len()` is formula-backed. For objects, use `len(keys(obj))` to count fields.

### String Functions (Planned) 📋

| Function | Description | Example | Status |
|----------|-------------|---------|--------|
| `upper(str)` | Convert to uppercase | `upper("hello")` → `"HELLO"` | 📋 Planned |
| `lower(str)` | Convert to lowercase | `lower("HELLO")` → `"hello"` | 📋 Planned |
| `trim(str)` | Remove whitespace | `trim(" hello ")` → `"hello"` | 📋 Planned |
| `substr(str, start, len)` | Extract substring | `substr("hello", 1, 3)` → `"ell"` | 📋 Planned |
| `concat(str1, str2, ...)` | Concatenate strings | `concat("a", "b", "c")` → `"abc"` | 📋 Planned |
| `split(str, delimiter)` | Split string | `split("a,b,c", ",")` → `["a", "b", "c"]` | 📋 Planned |
| `replace(str, old, new)` | Replace substring | `replace("hello", "l", "r")` → `"herro"` | 📋 Planned |

### Math Functions

| Function | Description | Example | Status |
|----------|-------------|---------|--------|
| `abs(n)` | Absolute value | `abs(-5)` → `5` | ✅ Implemented |
| `min(values...)` | Minimum value or sheet range | `min(3, 1, 5)` → `1.0` | ✅ Implemented |
| `max(values...)` | Maximum value or sheet range | `max(3, 1, 5)` → `5.0` | ✅ Implemented |
| `sum(values...)` | Sum of values/arrays or sheet range | `sum([1, 2, 3])` → `6.0` | ✅ Implemented |
| `avg(values...)` | Average of values/arrays or sheet range | `avg([1, 2, 3])` → `2.0` | ✅ Implemented |
| `count(values...)` | Count numeric values or sheet range | `count([1, 2, 3])` → `3` | ✅ Implemented |
| `counta(values...)` | Count non-empty values or sheet range | `counta([1, null, "x"])` → `2` | ✅ Implemented |
| `round(n, decimals)` | Round number | `round(3.14159, 2)` → `3.14` | 📋 Planned |
| `floor(n)` | Round down | `floor(3.9)` → `3` | 📋 Planned |
| `ceil(n)` | Round up | `ceil(3.1)` → `4` | 📋 Planned |

### Object/Array Functions

| Function | Description | Example | Status |
|----------|-------------|---------|--------|
| `keys(object)` | Get object keys | `keys({"a": 1, "b": 2})` → `["a", "b"]` | ✅ Implemented |
| `values(object)` | Get object values | `values({"a": 1, "b": 2})` → `[1, 2]` | ✅ Implemented |
| `consolidate(book)` | Consolidate book sheets | `consolidate(book)` | ✅ Implemented |

### Sheet Functions

| Function | Description | Example | Status |
|----------|-------------|---------|--------|
| `sheet_map(sheet, operation)` | Transform sheet cell values | `sheet_map(data, "upper")` | ✅ Implemented |
| `sheet_filter_rows(sheet, column, value)` | Filter sheet rows by column value | `sheet_filter_rows(data, "status", "active")` | ✅ Implemented |

**sheet_map operations:**
- `"upper"` - Convert string cells to uppercase
- `"lower"` - Convert string cells to lowercase  
- `"trim"` - Trim whitespace from string cells

**sheet_filter_rows:**
- Filters rows where the specified column matches the given value
- Preserves the header row
- Returns a new sheet with only matching rows

### Lookup Functions

| Function | Description | Example | Status |
| ---------- | ------------- | --------- | -------- |
| `vlookup(lookup_value, table, col_index, [exact])` | Vertical lookup | `vlookup("Apple", data, 2, false)` | ✅ Implemented |
| `hlookup(lookup_value, table, row_index, [exact])` | Horizontal lookup | `hlookup("Q1", data, 2, false)` | ✅ Implemented |
| `index(array, row_num, [col_num])` | Return value at position | `index(data, 2, 3)` | ✅ Implemented |
| `match(lookup_value, array, [match_type])` | Find position of value | `match("Apple", fruits, 0)` | ✅ Implemented |
| `xlookup(lookup, array, return_array, [if_not_found], [match_mode], [search_mode])` | Extended lookup | `xlookup("Apple", names, prices)` | ✅ Implemented |

#### Excel/PipTable Parity Matrix

| Feature | Excel | PipTable | Notes |
| --------- | ------- | ---------- | ------- |
| **VLOOKUP** | | | |
| Exact match (FALSE/0) | ✅ | ✅ | Identical behavior |
| Approximate match (TRUE/1) | ✅ | ✅ | Requires sorted data |
| Default range_lookup | TRUE | TRUE | Approximate match by default |
| #N/A on not found | ✅ | ✅ | Returns "#N/A" string |
| Type coercion | Partial | ✅ | PipTable coerces numeric strings |
| **HLOOKUP** | | | |
| Exact match | ✅ | ✅ | Identical to VLOOKUP logic |
| Approximate match | ✅ | ✅ | Requires sorted data |
| **INDEX** | | | |
| 1D array indexing | ✅ | ✅ | Single index parameter |
| 2D array indexing | ✅ | ✅ | Row and column indices |
| Negative indices | ❌ | ❌ | Returns error |
| **MATCH** | | | |
| Exact match (0) | ✅ | ✅ | Case-sensitive |
| Less than (1) | ✅ | ✅ | Finds largest ≤ value |
| Greater than (-1) | ✅ | ✅ | Finds smallest ≥ value |
| Wildcard support | ✅ | ❌ | Not yet implemented |
| **XLOOKUP** | | | |
| Basic lookup | ✅ | ✅ | Core functionality identical |
| if_not_found parameter | ✅ | ✅ | Custom default value |
| Match modes (0-2) | ✅ | ✅ | Exact, next smaller, next larger |
| Wildcard match mode | ✅ | ❌ | Not yet implemented |
| Search modes | ✅ | Partial | First-to-last, last-to-first |
| Binary search modes | ✅ | ❌ | Not yet implemented |

#### Type Coercion Rules

PipTable follows these rules for type comparisons in lookups:

1. **Numeric equality**: `1` (int) equals `1.0` (float)
2. **String to number**: `"123"` matches `123` when appropriate
3. **Case sensitivity**: String matches are case-sensitive
4. **Null handling**: Null values never match anything except explicit null checks

#### Common Examples

```piptable
# VLOOKUP Examples
dim products = [
    ["Apple", 1.50, 100],
    ["Banana", 0.75, 200],
    ["Cherry", 2.00, 150]
]

# Exact match lookup
dim price = vlookup("Banana", products, 2, false)  # Returns 0.75

# Approximate match (requires sorted first column)
dim sorted_data = [
    [10, "Low"],
    [50, "Medium"],
    [100, "High"]
]
dim category = vlookup(75, sorted_data, 2, true)  # Returns "Medium"

# HLOOKUP Example
dim quarterly = [
    ["Product", "Q1", "Q2", "Q3", "Q4"],
    ["Sales", 100, 150, 120, 180],
    ["Costs", 80, 100, 90, 120]
]
dim q2_sales = hlookup("Q2", quarterly, 2, false)  # Returns 150

# INDEX/MATCH Combination (like VLOOKUP but more flexible)
dim fruits = ["Apple", "Banana", "Cherry"]
dim prices = [1.50, 0.75, 2.00]
dim position = match("Banana", fruits, 0)  # Returns 2
dim price = index(prices, position)  # Returns 0.75

# XLOOKUP (modern replacement for VLOOKUP)
dim result = xlookup("Cherry", fruits, prices, "Not found", 0, 1)  # Returns 2.00
```

#### Error Handling

All lookup functions return `"#N/A"` when a lookup value is not found (matching Excel behavior). For custom error handling, use XLOOKUP with the `if_not_found` parameter:

```piptable
# Standard VLOOKUP returns #N/A
dim result1 = vlookup("Grape", products, 2, false)  # Returns "#N/A"

# XLOOKUP with custom not-found value
dim result2 = xlookup("Grape", fruits, prices, 0.00)  # Returns 0.00
```

### Array Functions (Planned) 📋

| Function | Description | Example | Status |
|----------|-------------|---------|--------|
| `first(array)` | First element | `first([1, 2, 3])` → `1` | 📋 Planned |
| `last(array)` | Last element | `last([1, 2, 3])` → `3` | 📋 Planned |
| `push(array, item)` | Add to end | `push([1, 2], 3)` → `[1, 2, 3]` | 📋 Planned |
| `pop(array)` | Remove last | `pop([1, 2, 3])` → `[1, 2]` | 📋 Planned |
| `slice(array, start, end)` | Extract subarray | `slice([1, 2, 3, 4], 1, 3)` → `[2, 3]` | 📋 Planned |
| `contains(array, item)` | Check membership | `contains([1, 2, 3], 2)` → `true` | 📋 Planned |
| `reverse(array)` | Reverse array | `reverse([1, 2, 3])` → `[3, 2, 1]` | 📋 Planned |
| `sort(array)` | Sort array | `sort([3, 1, 2])` → `[1, 2, 3]` | 📋 Planned |
| `has(object, key)` | Check key exists | `has({"a": 1}, "a")` → `true` | 📋 Planned |
| `merge(obj1, obj2)` | Merge objects | `merge({"a": 1}, {"b": 2})` → `{"a": 1, "b": 2}` | 📋 Planned |

### Utility Functions (Planned) 📋

| Function | Description | Example | Status |
|----------|-------------|---------|--------|
| `now()` | Current timestamp | `now()` → `2024-01-15 10:30:00` | 📋 Planned |
| `uuid()` | Generate UUID | `uuid()` → `"550e8400-e29b-..."` | 📋 Planned |
| `random()` | Random 0-1 | `random()` → `0.7264` | 📋 Planned |
| `random(min, max)` | Random in range | `random(1, 10)` → `7` | 📋 Planned |

## Advanced Functions (Python UDFs)

These functions require Python integration and provide more complex functionality.

### String Functions (Advanced)

```python
# Regular expression operations
def regex_match(pattern: str, text: str) -> bool:
    """Check if text matches pattern"""
    import re
    return bool(re.match(pattern, text))

def regex_extract(pattern: str, text: str) -> str:
    """Extract matching group from text"""
    import re
    match = re.search(pattern, text)
    return match.group(1) if match else None

# Advanced string manipulation  
def left(text: str, n: int) -> str:
    """Get leftmost n characters"""
    return text[:n]

def right(text: str, n: int) -> str:
    """Get rightmost n characters"""
    return text[-n:]

def pad_left(text: str, width: int, char: str = ' ') -> str:
    """Pad string on left to width"""
    return text.rjust(width, char)

def levenshtein(s1: str, s2: str) -> int:
    """Calculate edit distance between strings"""
    if len(s1) < len(s2):
        return levenshtein(s2, s1)
    
    if len(s2) == 0:
        return len(s1)
    
    previous_row = range(len(s2) + 1)
    for i, c1 in enumerate(s1):
        current_row = [i + 1]
        for j, c2 in enumerate(s2):
            # j+1 instead of j since previous_row and current_row are one character longer
            insertions = previous_row[j + 1] + 1
            deletions = current_row[j] + 1
            substitutions = previous_row[j] + (c1 != c2)
            current_row.append(min(insertions, deletions, substitutions))
        previous_row = current_row
    
    return previous_row[-1]
```

### Date/Time Functions (Advanced)

```python
from datetime import datetime, timedelta
from dateutil.parser import parse

def parse_date(date_str: str) -> datetime:
    """Parse date from string"""
    return parse(date_str)

def format_date(dt: datetime, format: str) -> str:
    """Format date as string"""
    return dt.strftime(format)

def date_add(dt: datetime, interval: str, amount: int) -> datetime:
    """Add interval to date"""
    intervals = {
        'days': timedelta(days=amount),
        'weeks': timedelta(weeks=amount),
        'months': timedelta(days=amount * 30),  # Approximate
        'years': timedelta(days=amount * 365)   # Approximate
    }
    return dt + intervals.get(interval, timedelta())

def date_diff(dt1: datetime, dt2: datetime, unit: str = 'days') -> int:
    """Calculate difference between dates"""
    delta = dt2 - dt1
    if unit == 'days':
        return delta.days
    elif unit == 'hours':
        return delta.total_seconds() // 3600
    elif unit == 'minutes':
        return delta.total_seconds() // 60
    return int(delta.total_seconds())

def weekday(dt: datetime) -> str:
    """Get day of week name"""
    return dt.strftime('%A')

def quarter(dt: datetime) -> int:
    """Get quarter (1-4)"""
    return (dt.month - 1) // 3 + 1
```

### Math Functions (Advanced)

```python
import math
import statistics

def sqrt(x: float) -> float:
    """Square root"""
    return math.sqrt(x)

def pow(x: float, y: float) -> float:
    """Power function"""
    return math.pow(x, y)

def log(x: float, base: float = math.e) -> float:
    """Logarithm"""
    return math.log(x, base)

def sin(x: float) -> float:
    """Sine"""
    return math.sin(x)

def cos(x: float) -> float:
    """Cosine"""  
    return math.cos(x)

def median(data: list) -> float:
    """Median of array"""
    return statistics.median(data)

def stdev(data: list) -> float:
    """Standard deviation"""
    return statistics.stdev(data)

def percentile(data: list, p: float) -> float:
    """Calculate percentile"""
    return statistics.quantiles(data, n=100)[int(p)]
```

### Array Functions (Advanced)

```python
def unique(arr: list) -> list:
    """Get unique elements"""
    return list(set(arr))

def flatten(arr: list) -> list:
    """Flatten nested arrays"""
    result = []
    for item in arr:
        if isinstance(item, list):
            result.extend(flatten(item))
        else:
            result.append(item)
    return result

def group_by(arr: list, key_func) -> dict:
    """Group array by key function"""
    from itertools import groupby
    return {k: list(v) for k, v in groupby(arr, key_func)}

def zip_arrays(*arrays) -> list:
    """Zip multiple arrays together"""
    return list(zip(*arrays))
```

## Usage Examples

### Using Core Functions (Always Available)

```piptable
' Type conversion
dim age = int("25")
dim price = float("19.99")
dim text = str(42)

' Get type information
dim data_type = type(scores)
dim count = len(items)

' Math operations
dim total = sum([10, 20, 30])
dim average = avg(scores)
dim absolute = abs(-42)
dim minimum = min(3, 1, 5)
dim maximum = max(3, 1, 5)

' Object operations
dim object_keys = keys(data)
dim object_values = values(data)

' Sheet operations
import "data.csv" as data has_headers
dim upper_data = sheet_map(data, "upper")  ' Convert all text to uppercase
dim clean_data = sheet_map(data, "trim")   ' Trim whitespace from all cells
dim active_only = sheet_filter_rows(data, "status", "active")  ' Filter to active rows
```

### Using Python UDFs (When Python Available)

```piptable
' Register Python functions
register_python("regex_match", "import re; regex_match = lambda pattern, text: bool(re.match(pattern, text))")
register_python("regex_extract", "functions.py", "regex_extract")

' Advanced string operations
dim is_email = regex_match("^[a-zA-Z0-9+_.-]+@[a-zA-Z0-9.-]+$", email)
dim area_code = regex_extract("\\((\\d{3})\\)", phone_number)

' Date/time calculations
dim next_month = date_add(now(), "months", 1)
dim days_until = date_diff(now(), deadline, "days")
dim day_name = weekday(now())

' Statistical functions
dim mid_value = median(scores)
dim std_deviation = stdev(measurements)
dim p90 = percentile(response_times, 90)

' Advanced array operations
dim unique_ids = unique(all_ids)
dim flat_data = flatten(nested_arrays)
```

## Implementation Strategy

### Phase 1: Core Functions (Rust)
1. Implement essential type conversions
2. Add basic string operations
3. Include fundamental math functions
4. Provide simple array/object utilities

### Phase 2: Python Integration
1. Create Python binding interface
2. Define standard Python function library
3. Allow custom Python function imports
4. Handle type conversions between Rust/Python

### Phase 3: Performance Optimization
1. Cache Python function references
2. Batch Python calls when possible
3. Consider moving hot Python functions to Rust

## Function Resolution Order

When a function is called, PipTable resolves it in this order:

1. **Built-in Rust functions** - Fastest, always available
2. **Python standard library** - If Python integration enabled
3. **User-defined Python functions** - Custom implementations
4. **User-defined PipTable functions** - Written in DSL

This ensures maximum performance for common operations while allowing extensibility for complex use cases.

## See Also

- [Python Integration](../guide/python-integration.md) - Setting up Python UDFs
- [DSL Reference](../dsl/README.md) - Language syntax
- [Performance Guide](../guide/performance.md) - Optimization tips
