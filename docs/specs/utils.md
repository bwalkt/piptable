# Spreadsheet Utils (Internal)

## Source of truth
- Core address/column utilities live in `piptable-primitives::address`.
- `piptable-utils` re-exports these helpers for internal consumption.

## Guidance
- Do not duplicate A1 parsing, column conversions, or sheet-name sanitation in other crates.
- If you need a new low-level helper, add it to `piptable-primitives::address` and re-export it from `piptable-utils`.
- Keep DSL/API-facing behavior aligned with primitives; avoid reimplementing logic in higher layers.

## Scope
- This file is for internal development guidance only.
- User-facing docs (DSL, cookbook, playground) should not document these helpers directly.
