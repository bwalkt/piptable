# PDF Documentation Summary

This summary describes the PDF documentation files and their scope.

## Files

- `crates/pdf/README.md`
  - Rust API overview for table extraction and structure extraction.
  - Includes `PdfOptions`, `StructuredDocument`, and heading detection notes.

- `docs/PDF_DSL.md`
  - DSL-only usage for PDF import.
  - Uses real DSL syntax: `with { ... }` object options.
  - Notes that PDF table import returns `table_1`, `table_2`, etc.

- `docs/PDF_COOKBOOK.md`
  - Recipes and workflows using the DSL.
  - Examples use the correct PDF table access pattern (`table_1`).

- `docs/playground/PDF_EXAMPLES.md`
  - Playground-friendly examples only (simulated data).
  - PDF import is not supported in wasm; examples avoid real PDF imports.

## Notes

- DSL export syntax is `export <expr> to <path>`; format is inferred from extension.
- Structure extraction in DSL uses `with { "extract_structure": true }`.
