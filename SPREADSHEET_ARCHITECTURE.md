# Spreadsheet Engine Architecture

## Overview
Following Codex's recommendations, we've created a clean separation of concerns with three new crates that will serve as the foundation for spreadsheet functionality in both WASM (browser) and native (Tauri) contexts.

## Crate Structure

### 1. `crates/piptable-primitives/`
**Purpose**: Core spreadsheet primitives
- Cell addresses (A1, R1C1 notation)
- Cell ranges and references
- Value types (Empty, Bool, Int, Float, String, Error, Array)
- Error types for cell values
- Address parsing and formatting

### 2. `crates/piptable-formulas/`
**Purpose**: Formula engine
- Formula AST and expression types
- Formula compilation and caching
- Function registry (SUM, VLOOKUP, etc.)
- Dependency tracking for recalculation
- Standard Excel-compatible functions

### 3. `crates/piptable-utils/`
**Purpose**: Utility functions
- Number formatting (thousands separators, percentages)
- Date/time handling (Excel serial dates)
- Column index ↔ letter conversions
- Value parsing from strings
- Format string parsing

### 4. `crates/piptable-wasm/` (existing, to be enhanced)
**Purpose**: WASM bindings
- Expose compile/eval APIs
- Data marshaling (JSON → MessagePack/TOON later)
- Batch operations to minimize boundary crossings

### 5. `crates/sheet/` (existing, to be integrated)
**Purpose**: Sheet data structure
- Integrate with formula engine
- Implement as data source for formula evaluation
- Cell storage and retrieval

## Data Exchange Format Strategy

Per Codex's recommendation:
1. **Phase 1** (Current): JSON for formulas + small ranges
   - Easy to debug and integrate
   - Good for initial implementation

2. **Phase 2** (Performance): MessagePack or TOON
   - Smaller payloads
   - Faster parsing
   - Still easy to use from JS

3. **Phase 3** (Optional): Arrow IPC
   - Only if moving large columnar datasets
   - May be overkill for typical spreadsheet ranges

## Key Design Principles

### 1. Coarse-Grained WASM Boundary
```rust
// Good: Batch operations
pub fn evaluate_range(range: &str, formulas: Vec<String>) -> Vec<Value>

// Bad: Per-cell calls
pub fn evaluate_cell(formula: &str) -> Value
```

### 2. Formula Caching
- Compile formulas once, cache the AST
- Only recompile when formula text changes
- Track dependencies for smart recalculation

### 3. Engine-Agnostic Core
- Same engine serves both DSL and spreadsheet
- Shared evaluation logic
- Different contexts (DSLContext vs SpreadsheetContext)

## Integration Points

### With Existing Piptable
- Reuse existing `Expr` AST from parser
- Extend interpreter for cell references
- Leverage DataFusion for SQL operations on ranges
- Use Arrow for efficient data representation

### Browser (WASM)
```typescript
import { FormulaEngine } from '@piptable/wasm';

const engine = new FormulaEngine();
const results = await engine.evaluateBatch(sheet, formulas);
```

### Desktop (Tauri)
```rust
// Direct Rust usage, no WASM overhead
let engine = FormulaEngine::new();
let results = engine.evaluate_range(&sheet, &formulas)?;
```

## Next Steps

1. Implement A1 notation parsing in primitives
2. Complete formula parser using Pest
3. Implement basic functions (SUM, AVERAGE, etc.)
4. Create WASM bindings with batch API
5. Integrate with existing sheet crate
6. Add comprehensive tests

## Benefits

- **Single implementation**: One codebase for all platforms
- **Performance**: Rust speed for calculations
- **Type safety**: Rust's type system prevents bugs
- **Clean boundaries**: Easy to target both WASM and native
- **Maintainability**: Clear separation of concerns

## References

- Issue #204: Unified spreadsheet engine implementation
- SheetXL packages for feature parity
- Existing piptable DSL and interpreter