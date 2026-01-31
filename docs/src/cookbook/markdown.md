# Markdown Table Import

This recipe shows how to extract tables from Markdown using the Rust API.

## Extract All Tables

```rust
use piptable_markdown::extract_tables;

let md = r#"
| Name | Score |
| ---- | ----- |
| Alice | 95 |
| Bob | 87 |
"#;

let sheets = extract_tables(md)?;
assert_eq!(sheets.len(), 1);
```

## Convert a Single Table

```rust
use piptable_markdown::MarkdownTables;

let md = r#"
| A | B |
|---|---|
| 1 | 2 |
"#;

let tables = MarkdownTables::from_markdown(md)?;
let sheet = tables.tables[0].to_sheet()?;
```

## Notes

- Markdown import is Rust API only for now.
- DSL import/export for Markdown is planned.
