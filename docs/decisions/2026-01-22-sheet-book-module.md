# Decision: Sheet/Book Module - Move Away from VBA

**Date**: 2026-01-22
**Status**: Approved

## Context

While implementing VBA built-in functions (#48-54), we questioned:
> "Why reinvent the wheel when Python has excellent string/array support?"

Explored alternatives:
- **pyexcel** - Good API but doesn't work on Mac
- **xlwings** - Requires Excel installed, 1-based indexing
- **openpyxl** - Low-level cell access

## Decision

**Move away from VBA approach**. Instead:

1. Create **pyexcel-like Sheet/Book API in Rust** (`crates/sheet/`)
2. Expose via **PyO3 bindings** (Phase 2)
3. Let **Python handle string/array/data operations** natively
4. **CSV first**, xlsx via calamine later

## Architecture

```
Python User Code
    ↓
PyO3 Bindings
    ↓
┌─────────────────────────────────┐
│  crates/sheet/ (Rust)           │
│  - Sheet/Book data structures   │
│  - CSV/xlsx I/O                 │
│  - Core operations              │
└─────────────────────────────────┘
```

## API Design (pyexcel-inspired)

### Sheet Operations
```python
# Python (via PyO3)
sheet = Sheet.from_csv("file.csv")
sheet.name_columns_by_row(0)  # First row as headers

# Access
sheet.get(row, col)           # Cell value (0-based)
sheet.row(idx)                # Row as list
sheet.column("ColName")       # Column by name

# CRUD
sheet.row_append([1, 2, 3])
sheet.column_delete("ColName")
sheet.filter_rows(lambda r: r[0] > 5)

# Export
sheet.save_as("output.csv")
sheet.to_dict()  # {col_name: [values]}
```

### Book Operations
```python
book = Book()
book.add_sheet("Sheet1", sheet)
book.get_sheet("Sheet1")
book.sheet_names()
```

## Comparison: pyexcel vs xlwings

| Aspect | pyexcel | xlwings | Our Approach |
|--------|---------|---------|--------------|
| Indexing | 0-based | 1-based | 0-based |
| Named access | Built-in | Via named ranges | Built-in |
| Bulk delete | `del sheet.row[a,b,c]` | Manual loops | Supported |
| Platform | Cross-platform | Needs Excel | Cross-platform |
| File support | Many formats | Excel only | CSV first, xlsx later |

## Impact on Existing Issues

- **#48-54 (VBA Built-in Functions)**: Deprioritize or close
- **#59 (PyO3 bindings)**: Higher priority now
- **New issues needed**: Sheet module implementation

## References

- Roadmap: #64
- pyexcel docs: https://docs.pyexcel.org/en/latest/
- calamine (Rust xlsx): https://github.com/tafia/calamine
- python-calamine: Python bindings for calamine

## Implementation Plan

See: `/Users/umam3/.claude/plans/purrfect-painting-crayon.md`

1. Create `crates/sheet/` with Cargo.toml
2. Implement CellValue and Sheet struct
3. Implement CRUD operations
4. Implement CSV I/O
5. Implement Book struct
6. Add tests
7. (Phase 2) PyO3 bindings
