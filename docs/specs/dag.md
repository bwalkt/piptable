# Dependency DAG (Implementation Requirements)

## Purpose
Provide a dependency graph for formulas that:
- Records edges between formula cells and their inputs (cells, ranges, statics).
- Computes recalculation order from dirty inputs.
- Supports sheet-aware dependencies.
- Guards against cycles and pathological ranges.

## Scope (Non-user-facing)
This is an internal implementation spec for `piptable-dag` and its integration
with `piptable-formulas`.

## Functional Requirements
1) Node types
- Cells (sheet_id + row + col).
- Ranges (sheet_id + start/end coordinates).
- Static references (named identifiers).

2) Edge management
- Add edges from formula cell -> input node.
- Remove edges for a given formula cell.
- Support delete semantics (clear inputs, detach node, or delete node entirely).

3) Dirty tracking
- Mark cells dirty.
- Return dependents in recalculation order.
- Provide both non-mutating and mutating accessors for dirty nodes.

4) Sheet awareness
- Store sheet ID on all nodes and ranges.
- Unresolved sheet names in formulas are stored as metadata only and do not
  produce DAG edges unless a resolver maps names to IDs.

5) Range support
- Range nodes must participate in dependency ordering.
- Range dependencies must be guarded by a maximum cell count.

6) Cycle detection
- Detect and reject circular dependencies when adding edges.
- Surface cycle details for error reporting.

## Integration Requirements (Formula Engine)
- Compilation emits `FormulaDependency` items for cells/ranges and sheet-qualified
  references.
- The engine wires dependencies into the DAG via `set_formula_with_sheet`.
- Dirty propagation uses the sheet-aware DAG APIs.

## Constraints
- Default max range size: 10,000 cells (configurable).
- Non-mutating dirty reads must not alter the dirty set.
- Operations must be safe against out-of-bounds sheet or cell values.

## Error Handling
- Cycle detection errors must be surfaced as formula errors.
- Range-too-large errors must include requested size and max.
- Deleting a sheet must preserve static nodes.

## Performance Requirements
- Range lookups should be indexed by row for faster dependent discovery.
- Avoid redundant graph traversals when possible.

## Testing Requirements
- Cycle detection (simple and multi-node).
- Range dependency limits.
- Dirty node ordering with ranges.
- Sheet deletion does not remove static nodes.
- JSON round-trip for DAG persistence.

## Out of Scope (for now)
- Spatial indexing beyond row buckets.
- External sheet dependency resolution beyond provided resolver.
