use piptable_core::Value;
use piptable_interpreter::Interpreter;
use piptable_parser::PipParser;
use std::fs;
use tempfile::tempdir;

#[tokio::test]
async fn test_import_html_file() {
    let temp_dir = tempdir().unwrap();
    let html_path = temp_dir.path().join("test.html");

    let html_content = r#"<!DOCTYPE html>
<html>
<body>
    <table>
        <tr>
            <th>Name</th>
            <th>Age</th>
            <th>City</th>
        </tr>
        <tr>
            <td>Alice</td>
            <td>30</td>
            <td>New York</td>
        </tr>
        <tr>
            <td>Bob</td>
            <td>25</td>
            <td>Los Angeles</td>
        </tr>
    </table>
</body>
</html>"#;
    fs::write(&html_path, html_content).unwrap();

    let script = format!(
        r#"
        import "{}" into data
        data
    "#,
        html_path.display()
    );

    let mut interp = Interpreter::new();
    let program = PipParser::parse_str(&script).unwrap();
    let result = interp.eval(program).await.unwrap();

    // Check that we got a sheet
    match result {
        Value::Sheet(sheet) => {
            assert!(sheet.row_count() >= 3, "Expected at least 3 rows");
            assert!(sheet.col_count() >= 3, "Expected at least 3 columns");
        }
        _ => panic!("Expected a Sheet value"),
    }
}

#[tokio::test]
async fn test_import_html_with_headers() {
    let temp_dir = tempdir().unwrap();
    let html_path = temp_dir.path().join("test_headers.html");

    let html_content = r#"<!DOCTYPE html>
<html>
<body>
    <table>
        <tr>
            <th>Product</th>
            <th>Price</th>
            <th>Quantity</th>
        </tr>
        <tr>
            <td>Widget A</td>
            <td>10.50</td>
            <td>100</td>
        </tr>
        <tr>
            <td>Widget B</td>
            <td>15.75</td>
            <td>50</td>
        </tr>
    </table>
</body>
</html>"#;
    fs::write(&html_path, html_content).unwrap();

    let script = format!(
        r#"
        import "{}" into products
        products
    "#,
        html_path.display()
    );

    let mut interp = Interpreter::new();
    let program = PipParser::parse_str(&script).unwrap();
    let result = interp.eval(program).await.unwrap();

    // Check that we got a sheet with the expected data
    match result {
        Value::Sheet(sheet) => {
            assert_eq!(
                sheet.row_count(),
                3,
                "Expected 3 rows (header + 2 data rows)"
            );
            assert_eq!(sheet.col_count(), 3, "Expected 3 columns");
            // Check that first row contains header-like strings
            if let Ok(first_cell) = sheet.get(0, 0) {
                match first_cell {
                    piptable_sheet::CellValue::String(s) => {
                        assert_eq!(s, "Product", "Expected first header to be 'Product'");
                    }
                    _ => panic!("Expected header cell to be a string"),
                }
            } else {
                panic!("Could not get first cell");
            }
        }
        _ => panic!("Expected a Sheet value"),
    }
}

#[tokio::test]
async fn test_import_html_mixed_data_types() {
    let temp_dir = tempdir().unwrap();
    let html_path = temp_dir.path().join("test_mixed.html");

    let html_content = r#"<!DOCTYPE html>
<html>
<body>
    <table>
        <tr>
            <td>Alice</td>
            <td>30</td>
            <td>true</td>
            <td>3.14</td>
        </tr>
        <tr>
            <td>Bob</td>
            <td>25</td>
            <td>false</td>
            <td>2.71</td>
        </tr>
    </table>
</body>
</html>"#;

    fs::write(&html_path, html_content).unwrap();

    let script = format!(
        r#"
        import "{}" into data
        data
    "#,
        html_path.display()
    );

    let mut interp = Interpreter::new();
    let program = PipParser::parse_str(&script).unwrap();
    let result = interp.eval(program).await.unwrap();

    // Check that numeric values were properly parsed
    match result {
        Value::Sheet(sheet) => {
            assert_eq!(sheet.row_count(), 2);
            assert_eq!(sheet.col_count(), 4);

            // Check that types were correctly parsed
            // Row 0: Alice, 30, true, 3.14
            assert!(matches!(
                sheet.get(0, 0).unwrap(),
                piptable_sheet::CellValue::String(_)
            ));
            assert!(matches!(
                sheet.get(0, 1).unwrap(),
                piptable_sheet::CellValue::Int(30)
            ));
            assert!(matches!(
                sheet.get(0, 2).unwrap(),
                piptable_sheet::CellValue::Bool(true)
            ));
            assert!(matches!(
                sheet.get(0, 3).unwrap(),
                piptable_sheet::CellValue::Float(_)
            ));

            // Row 1: Bob, 25, false, 2.71
            assert!(matches!(
                sheet.get(1, 0).unwrap(),
                piptable_sheet::CellValue::String(_)
            ));
            assert!(matches!(
                sheet.get(1, 1).unwrap(),
                piptable_sheet::CellValue::Int(25)
            ));
            assert!(matches!(
                sheet.get(1, 2).unwrap(),
                piptable_sheet::CellValue::Bool(false)
            ));
            assert!(matches!(
                sheet.get(1, 3).unwrap(),
                piptable_sheet::CellValue::Float(_)
            ));
        }
        _ => panic!("Expected a Sheet value"),
    }
}

#[tokio::test]
async fn test_import_html_no_table() {
    let temp_dir = tempdir().unwrap();
    let html_path = temp_dir.path().join("no_table.html");

    let html_content = r#"<!DOCTYPE html>
<html>
<body>
    <div>No table here, just some text</div>
</body>
</html>"#;

    fs::write(&html_path, html_content).unwrap();

    let script = format!(
        r#"
        import "{}" into data
        data
    "#,
        html_path.display()
    );

    let mut interp = Interpreter::new();
    let program = PipParser::parse_str(&script).expect("Script should parse successfully");
    let result = interp.eval(program).await;

    // Should fail gracefully when no table is found
    assert!(
        result.is_err(),
        "Expected error when no table found in HTML"
    );
    let error_msg = format!("{}", result.unwrap_err());
    assert!(
        error_msg.contains("No table") || error_msg.contains("Failed to import HTML"),
        "Expected error message about missing table, got: {}",
        error_msg
    );
}
