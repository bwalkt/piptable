use piptable_core::Value;
use piptable_interpreter::Interpreter;
use piptable_parser::PipParser;
use piptable_sheet::CellValue;
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
        import "{}" into data without headers
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

            // Now that headers are disabled, this should be parsed as Int(30)
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
async fn test_import_html_with_td_headers() {
    let temp_dir = tempdir().unwrap();
    let html_path = temp_dir.path().join("test_td_headers.html");

    let html_content = r#"<!DOCTYPE html>
<html>
<body>
    <table>
        <tr>
            <td>Product ID</td>
            <td>Product Name</td>
            <td>Price</td>
        </tr>
        <tr>
            <td>001</td>
            <td>Widget A</td>
            <td>10.50</td>
        </tr>
        <tr>
            <td>002</td>
            <td>Widget B</td>
            <td>15.75</td>
        </tr>
    </table>
</body>
</html>"#;
    fs::write(&html_path, html_content).unwrap();

    // Test with headers=true to verify td headers are treated as strings
    let script = format!(
        r#"
        import "{}" into products (headers = true)
        products
    "#,
        html_path.display()
    );

    let mut interp = Interpreter::new();
    let program = PipParser::parse_str(&script).unwrap();
    let result = interp.eval(program).await.unwrap();

    // Check that we got a sheet with properly named columns
    match result {
        Value::Sheet(sheet) => {
            assert_eq!(sheet.row_count(), 3, "Expected 3 rows including header");
            assert_eq!(sheet.col_count(), 3, "Expected 3 columns");

            // Verify column names were set
            let column_names = sheet.column_names().expect("Should have column names");
            assert_eq!(column_names[0], "Product ID");
            assert_eq!(column_names[1], "Product Name");
            assert_eq!(column_names[2], "Price");

            // Check that header row cells are strings (not parsed as numbers)
            if let Ok(first_header) = sheet.get(0, 0) {
                assert!(matches!(first_header, piptable_sheet::CellValue::String(_)));
            }

            // Check that data rows have correct types
            if let Ok(product_id) = sheet.get(1, 0) {
                assert!(matches!(product_id, piptable_sheet::CellValue::String(_)));
            }
            if let Ok(price) = sheet.get(1, 2) {
                assert!(matches!(price, piptable_sheet::CellValue::Float(_)));
            }
        }
        _ => panic!("Expected a Sheet value"),
    }
}

#[tokio::test]
async fn test_import_html_with_colspan() {
    let temp_dir = tempdir().unwrap();
    let html_path = temp_dir.path().join("test_colspan.html");

    let html_content = r#"<!DOCTYPE html>
<html>
<body>
    <table>
        <tr>
            <th colspan="2">Name</th>
            <th>Age</th>
        </tr>
        <tr>
            <td>John</td>
            <td>Doe</td>
            <td>30</td>
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

    // Check that colspan is handled correctly
    match result {
        Value::Sheet(sheet) => {
            assert_eq!(sheet.row_count(), 2);
            assert_eq!(
                sheet.col_count(),
                3,
                "Expected 3 columns after colspan expansion"
            );

            // Check that the header with colspan="2" appears in first two columns
            assert!(matches!(
                sheet.get(0, 0).unwrap(),
                piptable_sheet::CellValue::String(_)
            ));
            assert!(matches!(
                sheet.get(0, 1).unwrap(),
                piptable_sheet::CellValue::String(_)
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

#[tokio::test]
async fn test_import_htm_extension() {
    let temp_dir = tempdir().unwrap();
    let htm_path = temp_dir.path().join("test.htm"); // Note: .htm extension

    let html_content = r#"<!DOCTYPE html>
<html>
<body>
    <table>
        <tr>
            <th>Name</th>
            <th>Value</th>
        </tr>
        <tr>
            <td>Test</td>
            <td>123</td>
        </tr>
    </table>
</body>
</html>"#;
    fs::write(&htm_path, html_content).unwrap();

    let script = format!(
        r#"
        import "{}" into data
        data
    "#,
        htm_path.display()
    );

    let mut interp = Interpreter::new();
    let program = PipParser::parse_str(&script).unwrap();
    let result = interp.eval(program).await.unwrap();

    // Check that .htm files are handled the same as .html files
    match result {
        Value::Sheet(sheet) => {
            assert_eq!(sheet.row_count(), 2, "Expected 2 rows");
            assert_eq!(sheet.col_count(), 2, "Expected 2 columns");

            // Check that data was parsed correctly
            assert!(matches!(
                sheet.get(0, 0).unwrap(),
                piptable_sheet::CellValue::String(_)
            ));
            assert!(matches!(
                sheet.get(1, 1).unwrap(),
                piptable_sheet::CellValue::Int(123)
            ));
        }
        _ => panic!("Expected a Sheet value"),
    }
}

#[tokio::test]
async fn test_import_html_uneven_header_row() {
    let temp_dir = tempdir().unwrap();
    let html_path = temp_dir.path().join("test_uneven_header.html");

    let html_content = r#"<!DOCTYPE html>
<html>
<body>
    <table>
        <tr>
            <th>Name</th>
            <th>Age</th>
            <!-- Header row has only 2 columns -->
        </tr>
        <tr>
            <td>Alice</td>
            <td>30</td>
            <td>Engineer</td>
            <!-- Data row has 3 columns -->
        </tr>
        <tr>
            <td>Bob</td>
            <td>25</td>
            <td>Designer</td>
        </tr>
    </table>
</body>
</html>"#;
    fs::write(&html_path, html_content).unwrap();

    let script = format!(
        r#"
        import "{}" into data (headers = true)
        data
    "#,
        html_path.display()
    );

    let mut interp = Interpreter::new();
    let program = PipParser::parse_str(&script).unwrap();
    let result = interp.eval(program).await;

    // This should handle uneven headers gracefully
    match result {
        Ok(Value::Sheet(sheet)) => {
            assert_eq!(sheet.row_count(), 3, "Expected 3 rows");
            assert_eq!(sheet.col_count(), 3, "Expected 3 columns");

            // Check that column names were set, with generated names for missing headers
            if let Some(column_names) = sheet.column_names() {
                assert_eq!(column_names[0], "Name");
                assert_eq!(column_names[1], "Age");
                // The third column should have a generated name since header was missing
                assert_eq!(column_names[2], "Column_3");
            }
        }
        Err(e) => {
            panic!(
                "Uneven headers should now be handled gracefully, got error: {}",
                e
            );
        }
        Ok(other) => {
            panic!("Expected Sheet, got: {:?}", other);
        }
    }
}

#[tokio::test]
async fn test_import_html_rowspan_support() {
    // This test verifies that rowspan handling now works correctly
    let temp_dir = tempdir().unwrap();
    let html_path = temp_dir.path().join("test_rowspan.html");

    let html_content = r#"<!DOCTYPE html>
<html>
<body>
    <table>
        <tr>
            <th>Name</th>
            <th>Details</th>
        </tr>
        <tr>
            <td rowspan="2">Alice</td>
            <td>Engineer</td>
        </tr>
        <tr>
            <td>Senior</td>
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

    match result {
        Value::Sheet(sheet) => {
            // Verify table structure with rowspan support
            assert_eq!(sheet.row_count(), 3, "Should have header + 2 data rows");
            assert_eq!(sheet.col_count(), 2, "Should have 2 columns");

            // Check header row
            assert_eq!(
                sheet.get(0, 0).unwrap(),
                &CellValue::String("Name".to_string())
            );
            assert_eq!(
                sheet.get(0, 1).unwrap(),
                &CellValue::String("Details".to_string())
            );

            // Check first data row
            assert_eq!(
                sheet.get(1, 0).unwrap(),
                &CellValue::String("Alice".to_string())
            );
            assert_eq!(
                sheet.get(1, 1).unwrap(),
                &CellValue::String("Engineer".to_string())
            );

            // Check second data row - Alice should be duplicated due to rowspan
            assert_eq!(
                sheet.get(2, 0).unwrap(),
                &CellValue::String("Alice".to_string())
            );
            assert_eq!(
                sheet.get(2, 1).unwrap(),
                &CellValue::String("Senior".to_string())
            );
        }
        _ => panic!("Expected a Sheet value"),
    }
}

#[tokio::test]
async fn test_import_html_th_in_body_rows() {
    let temp_dir = tempdir().unwrap();
    let html_path = temp_dir.path().join("test_th_body.html");

    let html_content = r#"<!DOCTYPE html>
<html>
<body>
    <table>
        <tr>
            <th>Category</th>
            <th>Value</th>
        </tr>
        <tr>
            <th colspan="2">Row Header</th>
            <!-- This th in a data row should not get suffixed -->
        </tr>
        <tr>
            <td>Item</td>
            <td>100</td>
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

    // Verify that th elements in data rows don't get suffixed
    match result {
        Value::Sheet(sheet) => {
            assert_eq!(sheet.row_count(), 3);
            assert_eq!(sheet.col_count(), 2);

            // Check that the th in row 1 is just duplicated, not suffixed
            if let Ok(cell_0_0) = sheet.get(1, 0) {
                assert!(matches!(cell_0_0, piptable_sheet::CellValue::String(_)));
                if let piptable_sheet::CellValue::String(s) = cell_0_0 {
                    assert_eq!(s, "Row Header", "th in body row should not be suffixed");
                }
            }
            if let Ok(cell_0_1) = sheet.get(1, 1) {
                assert!(matches!(cell_0_1, piptable_sheet::CellValue::String(_)));
                if let piptable_sheet::CellValue::String(s) = cell_0_1 {
                    assert_eq!(s, "Row Header", "th in body row should not be suffixed");
                }
            }
        }
        _ => panic!("Expected a Sheet value"),
    }
}
