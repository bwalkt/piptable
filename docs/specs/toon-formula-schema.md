# TOON Formula Schema Specification

## Overview
This document specifies the TOON (Tagged Object Notation) schema for the formula engine WASM boundary in piptable. It defines how data is exchanged between Rust and JavaScript for spreadsheet operations.

## Value Type Mappings

### Core Value Types (WASM-safe)
These types from `crates/core/src/value.rs` map directly to TOON:

| Rust Type | TOON Format | JavaScript | Example |
|-----------|-------------|------------|---------|
| `Value::Null` | `{t: "null"}` | `null` | Empty cell |
| `Value::Bool(b)` | `{t: "bool", v: 0\|1}` | `boolean` | `{t: "bool", v: 1}` |
| `Value::Int(i64)` | `{t: "int", v: i}` | `number` | `{t: "int", v: 42}` |
| `Value::Float(f64)` | `{t: "float", v: f}` | `number` | `{t: "float", v: 3.14}` |
| `Value::String(s)` | `{t: "str", v: s}` | `string` | `{t: "str", v: "hello"}` |
| `Value::Array(vec)` | `{t: "arr", v: [...]}` | `Array` | `{t: "arr", v: [{t:"int",v:1}]}` |
| `Value::Object(map)` | `{t: "obj", v: {...}}` | `Object` | `{t: "obj", v: {a:{t:"int",v:1}}}` |

### Unsupported Types
These types cannot cross the WASM boundary directly:

| Rust Type | Handling | Reason |
|-----------|----------|---------|
| `Value::Table` | Convert to error or stringify | Too heavy for serialization |
| `Value::Sheet` | Convert to `SheetPayload` | Use dense/sparse encoding |
| `Value::Function` | Error | Non-serializable |
| `Value::Lambda` | Error | Non-serializable |

### Extended Types (Optional)
Additional types for future use:

```typescript
// Date (Unix timestamp in milliseconds)
{ t: "date", v: 1234567890000 }

// Duration (milliseconds)
{ t: "duration", v: 3600000 }

// Error
{ t: "error", code: "DIV0", msg: "Division by zero" }
```

## Sheet Encoding Strategies

### Dense Encoding (Default)
Best for typical spreadsheets with >20% cell occupancy:

```typescript
interface SheetPayloadDense {
  range: {
    s: { r: 0, c: 0 },    // Start cell (A1)
    e: { r: 99, c: 25 }   // End cell (Z100)
  },
  values: [...]  // 2600 values in row-major order
}
```

**Row-major order example (3x3 range):**
```
Cells: A1 B1 C1
       A2 B2 C2
       A3 B3 C3

Values array: [A1, B1, C1, A2, B2, C2, A3, B3, C3]
```

### Sparse Encoding
Best for large, mostly empty ranges:

```typescript
interface SheetPayloadSparse {
  range: {
    s: { r: 0, c: 0 },
    e: { r: 999, c: 999 }
  },
  items: [
    { r: 0, c: 0, v: { t: "int", v: 1 } },
    { r: 5, c: 10, v: { t: "str", v: "data" } },
    // Only non-empty cells
  ]
}
```

### Encoding Selection Algorithm

```rust
fn should_use_sparse(rows: u32, cols: u32, non_null_count: usize) -> bool {
    let total_cells = (rows * cols) as usize;
    let density = non_null_count as f64 / total_cells as f64;
    
    // Use sparse if:
    // 1. Density < 20%
    // 2. Large grid (>10k cells) with many empties
    density < 0.2 || (total_cells > 10_000 && density < 0.5)
}
```

## API Message Formats

### Formula Compilation

**Request:**
```javascript
{
  formulas: [
    { kind: "text", f: "=A1+B1" },
    { kind: "text", f: "=SUM(A:A)" }
  ],
  options: {
    locale: "en-US",      // Optional
    decimal: "."          // Optional
  }
}
```

**Response:**
```javascript
{
  compiled: [
    { kind: "bc", b: [/* bytecode */] },
    { kind: "bc", b: [/* bytecode */] }
  ],
  errors: []  // Or [{idx: 0, msg: "Parse error"}]
}
```

### Formula Evaluation

**Request:**
```javascript
{
  compiled: [/* from compile response */],
  sheet: {/* SheetPayload */},
  globals: {  // Optional named values
    "TAX_RATE": { t: "float", v: 0.08 }
  }
}
```

**Response:**
```javascript
{
  results: [
    { t: "float", v: 42.5 },
    { t: "int", v: 100 }
  ],
  errors: []
}
```

### Range Updates

**Request:**
```javascript
{
  sheet: {/* current SheetPayload */},
  updates: [
    { 
      addr: { r: 0, c: 0 },
      value: { t: "int", v: 123 }
    }
  ]
}
```

**Response (one of):**
```javascript
// Full updated sheet
{ /* Updated SheetPayload */ }

// Or simple success
{ ok: true }
```

## Wire Format Selection

The WASM functions auto-detect the wire format:

1. **JSON** (first byte = `{`):
   - Used in debug mode
   - Payloads < 4KB
   - Human-readable

2. **MessagePack/TOON** (other first bytes):
   - Production mode
   - Payloads > 4KB
   - ~3-5x smaller than JSON
   - Faster parsing

## Performance Guidelines

### Batch Operations
Always batch operations to minimize WASM boundary crossings:

```javascript
// ❌ Bad: Multiple WASM calls
for (let i = 0; i < 100; i++) {
  evalCell(formulas[i]);
}

// ✅ Good: Single batched call
evalMany({ compiled: formulas, sheet: data });
```

### Caching
Compiled formulas should be cached:

```javascript
const cache = new Map();

function getCompiled(formula) {
  if (!cache.has(formula)) {
    const result = compileMany({ formulas: [{ kind: "text", f: formula }] });
    cache.set(formula, result.compiled[0]);
  }
  return cache.get(formula);
}
```

### Memory Management
- Reuse `SheetPayload` objects when possible
- Clear large arrays after use
- Use sparse encoding for large, mostly empty sheets

## Error Handling

All errors include an index and message:

```javascript
{
  errors: [
    { idx: 0, msg: "Unknown function: FOOBAR" },
    { idx: 2, msg: "Circular reference detected" }
  ]
}
```

Error codes for cell values:
- `DIV0` - Division by zero
- `NAME` - Unknown name/function
- `VALUE` - Type mismatch
- `REF` - Invalid reference
- `NULL` - Null intersection
- `NUM` - Invalid number
- `NA` - Not available