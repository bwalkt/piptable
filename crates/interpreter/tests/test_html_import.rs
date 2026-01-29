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
        import data from "{}"
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
        import products from "{}" with headers
        products.columns
    "#,
        html_path.display()
    );

    let mut interp = Interpreter::new();
    let program = PipParser::parse_str(&script).unwrap();
    let result = interp.eval(program).await.unwrap();

    // Check that columns were named from the header row
    match result {
        Value::Object(cols) => {
            assert!(!cols.is_empty(), "Expected named columns");
            // Check that we have column names
            if let Some((first_col, _)) = cols.iter().next() {
                assert!(
                    first_col == "Product",
                    "Expected first column to be 'Product', got: {}",
                    first_col
                );
            }
        }
        _ => panic!("Expected an Object with column names"),
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
        import data from "{}"
        data[0][1]
    "#,
        html_path.display()
    );

    let mut interp = Interpreter::new();
    let program = PipParser::parse_str(&script).unwrap();
    let result = interp.eval(program).await.unwrap();

    // Check that numeric value was properly parsed
    match result {
        Value::Int(i) => {
            assert_eq!(i, 30, "Expected integer value 30");
        }
        _ => panic!("Expected an Int value"),
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
        import data from "{}"
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
