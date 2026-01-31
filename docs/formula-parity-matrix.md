# Formula Parity Matrix: Piptable vs SheetXL

## Overview
This document tracks formula function parity between Piptable's implementation and SheetXL's comprehensive formula library.

## Implementation Status

### ✅ Implemented (23 functions)

| Category | Function | Piptable | SheetXL | Notes |
| -------- | -------- | -------- | ------- | ----- |
| **Math & Aggregation** | | | | |
| | SUM | ✅ | ✅ | Basic aggregation |
| | AVERAGE | ✅ | ✅ | Mean calculation |
| | COUNT | ✅ | ✅ | Count numeric values |
| | COUNTA | ✅ | ✅ | Count non-empty cells |
| | MAX | ✅ | ✅ | Maximum value |
| | MIN | ✅ | ✅ | Minimum value |
| **Logical** | | | | |
| | IF | ✅ | ✅ | Conditional logic |
| | AND | ✅ | ✅ | Logical AND |
| | OR | ✅ | ✅ | Logical OR |
| | NOT | ✅ | ✅ | Logical NOT |
| **Text** | | | | |
| | CONCAT | ✅ | ✅ | String concatenation |
| | LEN | ✅ | ✅ | String length |
| | LEFT | ✅ | ✅ | Left substring |
| | RIGHT | ✅ | ✅ | Right substring |
| **Date & Time** | | | | |
| | TODAY | ✅ | ✅ | Current date |
| | NOW | ✅ | ✅ | Current date/time |
| | DATE | ✅ | ✅ | Create date |
| **Lookup & Reference** | | | | |
| | VLOOKUP | ✅ | ✅ | Vertical lookup |
| | HLOOKUP | ✅ | ✅ | Horizontal lookup |
| | INDEX | ✅ | ✅ | Array indexing |
| | MATCH | ✅ | ✅ | Position matching |
| | XLOOKUP | ✅ | ✅ | Modern lookup |
| | OFFSET | ✅ | ✅ | Dynamic range reference |

### 🔄 Planned for Next Phase

| Category | Function | Priority | Notes |
| -------- | -------- | -------- | ----- |
| **Lookup & Reference** | | | |
| | XMATCH | Medium | XLOOKUP companion |
| **Math** | | | |
| | ROUND | High | Rounding |
| | FLOOR | Medium | Round down |
| | CEILING | Medium | Round up |
| | ABS | High | Absolute value |
| | SQRT | Medium | Square root |
| **Statistical** | | | |
| | MEDIAN | Medium | Middle value |
| | MODE | Low | Most frequent |
| | STDEV | Medium | Standard deviation |
| **Text** | | | |
| | MID | High | Substring |
| | UPPER | Medium | Uppercase |
| | LOWER | Medium | Lowercase |
| | TRIM | High | Remove spaces |
| | FIND | Medium | Find substring |
| | SUBSTITUTE | Medium | Replace text |

### 📊 SheetXL Categories Not Yet Addressed

| Category | Function Count | Examples | Priority |
| -------- | -------------- | -------- | -------- |
| Financial | ~50 | PV, FV, PMT, IRR, NPV | Low |
| Engineering | ~40 | CONVERT, BIN2DEC, COMPLEX | Low |
| Statistical | ~100+ | NORM.DIST, T.TEST, CORREL | Medium |
| Database | ~12 | DSUM, DCOUNT, DAVERAGE | Low |
| Cube | ~7 | CUBEVALUE, CUBESET | Very Low |
| Web | ~3 | WEBSERVICE, FILTERXML | Low |
| Information | ~20 | ISBLANK, ISERROR, ISNUMBER | High |

## Coverage Metrics

- **Current Coverage**: 23/400+ (~6%)
- **Core Functions**: 23/50 (46%)
- **Categories Covered**: 5/12 (41%)

## Performance Baselines

| Operation | Target | Current | Status |
| --------- | ------ | ------- | ------ |
| Parse formula (simple) | <1ms | TBD | 🔄 |
| Parse formula (complex) | <5ms | TBD | 🔄 |
| Evaluate SUM (100 cells) | <1ms | TBD | 🔄 |
| Evaluate SUM (10,000 cells) | <10ms | TBD | 🔄 |
| Compile formula | <2ms | TBD | 🔄 |
| Cache lookup | <0.1ms | TBD | 🔄 |

## Notes

1. **Core Focus**: We're prioritizing the most commonly used spreadsheet functions
2. **DSL Integration**: Functions are designed to work both in spreadsheet and DSL contexts
3. **WASM Optimization**: Batch operations via TOON format for efficient boundary crossing
4. **Extensibility**: Function registry allows easy addition of new functions

## Next Steps

1. Add ROUND, ABS, and other essential math functions
2. Implement IS* information functions for type checking
3. Create comprehensive benchmarks for all implemented functions
