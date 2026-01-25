# Built-in Functions

PipTable uses a hybrid approach for built-in functions: core functions are implemented in Rust for performance, while complex/specialized functions can be provided via Python UDFs.

## Core Functions (Rust)

These essential functions are always available and implemented in Rust for maximum performance.

### Type Conversion

| Function | Description | Example |
|----------|-------------|---------|
| `int(value)` | Convert to integer | `int("42")` → `42` |
| `float(value)` | Convert to float | `float("3.14")` → `3.14` |
| `str(value)` | Convert to string | `str(42)` → `"42"` |
| `bool(value)` | Convert to boolean | `bool(1)` → `true` |

### String Functions (Basic)

| Function | Description | Example |
|----------|-------------|---------|
| `len(str)` | String length | `len("hello")` → `5` |
| `upper(str)` | Convert to uppercase | `upper("hello")` → `"HELLO"` |
| `lower(str)` | Convert to lowercase | `lower("HELLO")` → `"hello"` |
| `trim(str)` | Remove whitespace | `trim(" hello ")` → `"hello"` |
| `substr(str, start, len)` | Extract substring | `substr("hello", 1, 3)` → `"ell"` |
| `concat(str1, str2, ...)` | Concatenate strings | `concat("a", "b", "c")` → `"abc"` |
| `split(str, delimiter)` | Split string | `split("a,b,c", ",")` → `["a", "b", "c"]` |
| `replace(str, old, new)` | Replace substring | `replace("hello", "l", "r")` → `"herro"` |

### Math Functions (Basic)

| Function | Description | Example |
|----------|-------------|---------|
| `abs(n)` | Absolute value | `abs(-5)` → `5` |
| `round(n, decimals)` | Round number | `round(3.14159, 2)` → `3.14` |
| `floor(n)` | Round down | `floor(3.9)` → `3` |
| `ceil(n)` | Round up | `ceil(3.1)` → `4` |
| `min(a, b, ...)` | Minimum value | `min(3, 1, 5)` → `1` |
| `max(a, b, ...)` | Maximum value | `max(3, 1, 5)` → `5` |
| `sum(array)` | Sum of array | `sum([1, 2, 3])` → `6` |
| `avg(array)` | Average of array | `avg([1, 2, 3])` → `2` |

### Array Functions (Basic)

| Function | Description | Example |
|----------|-------------|---------|
| `len(array)` | Array length | `len([1, 2, 3])` → `3` |
| `first(array)` | First element | `first([1, 2, 3])` → `1` |
| `last(array)` | Last element | `last([1, 2, 3])` → `3` |
| `push(array, item)` | Add to end | `push([1, 2], 3)` → `[1, 2, 3]` |
| `pop(array)` | Remove last | `pop([1, 2, 3])` → `[1, 2]` |
| `slice(array, start, end)` | Extract subarray | `slice([1, 2, 3, 4], 1, 3)` → `[2, 3]` |
| `contains(array, item)` | Check membership | `contains([1, 2, 3], 2)` → `true` |
| `reverse(array)` | Reverse array | `reverse([1, 2, 3])` → `[3, 2, 1]` |
| `sort(array)` | Sort array | `sort([3, 1, 2])` → `[1, 2, 3]` |

### Object Functions

| Function | Description | Example |
|----------|-------------|---------|
| `keys(object)` | Get keys | `keys({"a": 1, "b": 2})` → `["a", "b"]` |
| `values(object)` | Get values | `values({"a": 1, "b": 2})` → `[1, 2]` |
| `has(object, key)` | Check key exists | `has({"a": 1}, "a")` → `true` |
| `merge(obj1, obj2)` | Merge objects | `merge({"a": 1}, {"b": 2})` → `{"a": 1, "b": 2}` |

### Utility Functions

| Function | Description | Example |
|----------|-------------|---------|
| `type(value)` | Get type name | `type(42)` → `"int"` |
| `print(value, ...)` | Output values | `print("Hello", name)` |
| `now()` | Current timestamp | `now()` → `2024-01-15 10:30:00` |
| `uuid()` | Generate UUID | `uuid()` → `"550e8400-e29b-..."` |
| `random()` | Random 0-1 | `random()` → `0.7264` |
| `random(min, max)` | Random in range | `random(1, 10)` → `7` |

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
    # Implementation here
    pass
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

```
' Type conversion
dim age = int("25")
dim price = float("19.99")
dim text = str(42)

' String manipulation
dim name = upper("john")
dim parts = split("a,b,c", ",")

' Math operations
dim total = sum([10, 20, 30])
dim average = avg(scores)

' Array operations
dim first_item = first(items)
dim sorted_list = sort(numbers)
```

### Using Python UDFs (When Python Available)

```
' Import Python functions module
use python "functions.py"

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