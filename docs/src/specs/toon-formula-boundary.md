# TOON Formula Boundary Schema (Draft)

This document defines the proposed TOON payloads used at the WASM boundary for
formula compilation and evaluation. It complements `docs/specs/toon-formula-boundary.toon`.

## 1) Core Types

### Value
Supported value envelope:

- `null` -> `{ t: "null" }`
- `bool` -> `{ t: "bool", v: true|false }`
- `int` -> `{ t: "int", v: i64 }`
- `float` -> `{ t: "float", v: f64 }`
- `str` -> `{ t: "str", v: string }`
- `arr` -> `{ t: "arr", v: [Value] }`
- `obj` -> `{ t: "obj", v: {string: Value} }`
- `error` -> `{ t: "error", code: string, msg: string }`

### CellAddr
Zero-based coordinates:

```
{ r: u32, c: u32 }
```

### Range
Inclusive bounds:

```
{ s: CellAddr, e: CellAddr }
```

## 2) Sheet Payload Encodings

### Dense (row-major)
Use when the range is small or mostly populated.

```
{
  range: { s: {r,c}, e: {r,c} },
  values: [Value]  // length = rows * cols, row-major
}
```

### Sparse
Use when most cells are empty/null.

```
{
  range: { s: {r,c}, e: {r,c} },
  items: [ { r: u32, c: u32, v: Value } ]
}
```

### Heuristic
- Use sparse if `(non_null / total) < 0.2` or `rows*cols > 10_000` with density `< 0.5`.
- Otherwise use dense.

## 3) Compile + Eval Requests

### CompileRequest
```
{
  formulas: [ { kind: "text", f: string } ],
  options?: { locale?: string, decimal?: string }
}
```

### CompileResponse
```
{
  compiled: [ { kind: "bc", b: bytes } ],
  errors: [ { idx: u32, msg: string } ]
}
```

### EvalRequest
```
{
  compiled: [ { kind: "bc", b: bytes } ],
  sheet: (dense | sparse),
  globals?: { string: Value }
}
```

### EvalResponse
```
{
  results: [Value],
  errors: [ { idx: u32, msg: string } ]
}
```

## 4) Mapping from piptable `Value`
Source: `crates/core/src/value.rs`

Supported boundary types:
- `Value::Null` -> `null`
- `Value::Bool` -> `bool`
- `Value::Int` -> `int`
- `Value::Float` -> `float`
- `Value::String` -> `str`
- `Value::Array` -> `arr`
- `Value::Object` -> `obj`

Unsupported or special handling:
- `Value::Sheet` -> serialize to `SheetPayload` (dense or sparse)
- `Value::Table` -> not supported at WASM boundary by default
- `Value::Function` / `Value::Lambda` -> not supported at WASM boundary

## 5) Notes

- The `.toon` file is a schema descriptor, not the official TOON format.
- This boundary spec is intentionally minimal; add types only when needed.
