use piptable_markdown::extract_tables;
use piptable_sheet::CellValue;

#[test]
fn test_multiple_tables() {
    let md = r#"
# Document with Tables

First table:

| Product | Price | Quantity |
|---------|-------|----------|
| Apple   | 1.50  | 10       |
| Banana  | 0.75  | 25       |

Some text between tables.

| Name    | Age | Active |
|---------|-----|--------|
| Alice   | 30  | true   |
| Bob     | 25  | false  |
"#;

    let sheets = extract_tables(md).expect("extract tables");
    assert_eq!(sheets.len(), 2);

    // First table
    let sheet1 = &sheets[0];
    assert_eq!(sheet1.row_count(), 3); // header + 2 rows
    assert_eq!(sheet1.col_count(), 3);

    // Second table
    let sheet2 = &sheets[1];
    assert_eq!(sheet2.row_count(), 3);
    assert_eq!(sheet2.col_count(), 3);
}

#[test]
fn test_type_inference() {
    let md = r#"
| String | Integer | Float | Bool | Null |
|--------|---------|-------|------|------|
| hello  | 42      | 3.14  | true | null |
| world  | -10     | 2.5   | false| N/A  |
"#;

    let sheets = extract_tables(md).expect("extract tables");
    let sheet = &sheets[0];

    // Check type inference for first data row
    let data = sheet.data();
    assert_eq!(data.len(), 3); // header + 2 rows

    let first_row = &data[1];
    assert!(matches!(first_row[0], CellValue::String(ref s) if s == "hello"));
    assert!(matches!(first_row[1], CellValue::Int(42)));
    assert!(matches!(first_row[2], CellValue::Float(f) if (f - 3.14).abs() < 0.001));
    assert!(matches!(first_row[3], CellValue::Bool(true)));
    assert!(matches!(first_row[4], CellValue::Null));
}

#[test]
fn test_inline_formatting() {
    let md = r#"
| Column | Value |
|--------|-------|
| **Bold** | *italic* |
| `code` | normal |
"#;

    let sheets = extract_tables(md).expect("extract tables");
    let sheet = &sheets[0];
    let data = sheet.data();

    // Text extraction should handle inline formatting
    let row1 = &data[1];
    assert!(matches!(row1[0], CellValue::String(ref s) if s == "Bold"));
    assert!(matches!(row1[1], CellValue::String(ref s) if s == "italic"));

    let row2 = &data[2];
    assert!(matches!(row2[0], CellValue::String(ref s) if s == "code"));
}

#[test]
fn test_no_tables_error() {
    let md = "# Just a heading\n\nNo tables here.";
    let result = extract_tables(md);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("No tables found"));
}
