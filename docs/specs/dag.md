# Dependency DAG (Internal)

## Purpose
The DAG tracks formula dependencies and provides recalculation order for dirty
cells. It supports cell, range, and static references and is used by
`piptable-formulas::FormulaEngine`.

## Key Types
- `piptable_dag::Dag` — stores nodes/edges and dirty tracking.
- `piptable_dag::NodeRef` — cell/range/static reference.
- `piptable_formulas::FormulaDependency` — dependency representation from formula AST.

## Usage (Formula Engine)
- `set_formula_with_sheet(sheet_id, cell, formula, resolver)` wires dependencies
  into the DAG.
- `mark_dirty_with_sheet(sheet_id, cell)` marks inputs dirty.
- `get_dirty_nodes_with_sheet()` returns the recalculation order.

## Notes
- `Dag::get_dirty_nodes()` is **non-mutating**; use `take_dirty_nodes()` when you
  want to clear the dirty set.
- Range dependencies are guarded by a max size (default 10,000 cells) to avoid
  excessive graph growth. Override with `Dag::with_max_range_cells`.
- Unresolved sheet names are stored as metadata (no DAG edge) unless a
  `SheetIdResolver` is provided.
