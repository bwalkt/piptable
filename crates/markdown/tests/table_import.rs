use piptable_markdown::{extract_tables, MarkdownTables};

#[test]
fn markdown_table_extracts_headers_and_rows() {
    let md = r#"| Name | Qty |
| --- | --- |
| Apple | 10 |
| Banana | 20 |"#;

    let tables = MarkdownTables::from_markdown(md).expect("parse markdown");
    assert_eq!(tables.tables.len(), 1);
    let table = &tables.tables[0];

    assert_eq!(table.headers.as_ref().unwrap(), &vec!["Name", "Qty"]);
    assert_eq!(table.rows.len(), 2);
    assert_eq!(table.rows[0], vec!["Apple", "10"]);
}

#[test]
fn markdown_tables_to_sheets() {
    let md = r#"| A | B |
| --- | --- |
| 1 | 2 |"#;

    let sheets = extract_tables(md).expect("extract tables");
    assert_eq!(sheets.len(), 1);
    let sheet = &sheets[0];
    assert_eq!(sheet.row_count(), 2);
    assert_eq!(sheet.col_count(), 2);
}
