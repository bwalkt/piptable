use piptable_interpreter::Interpreter;
use piptable_parser::PipParser;
use piptable_sheet::{CellValue, CsvOptions, Sheet};
use std::fs;
use tempfile::tempdir;

#[tokio::test]
async fn test_csv_append_mode() {
    let dir = tempdir().unwrap();
    let csv_path = dir.path().join("data.csv");

    // First, create initial CSV file
    {
        let mut interp = Interpreter::new();
        let script = format!(
            r#"
            dim data = [
                {{"id": 1, "name": "Alice", "age": 30}},
                {{"id": 2, "name": "Bob", "age": 25}}
            ]
            export data to "{}"
            "#,
            csv_path.display()
        );

        let program = PipParser::parse_str(&script).unwrap();
        interp.eval(program).await.unwrap();
    }

    // Verify initial file contents
    let content = fs::read_to_string(&csv_path).unwrap();
    assert!(content.contains("Alice"));
    assert!(content.contains("Bob"));
    let line_count = content.lines().count();
    assert_eq!(line_count, 3); // header + 2 data rows

    // Now append more data
    {
        let mut interp = Interpreter::new();
        let script = format!(
            r#"
            dim new_data = [
                {{"id": 3, "name": "Charlie", "age": 35}},
                {{"id": 4, "name": "Diana", "age": 28}}
            ]
            export new_data to "{}" append
            "#,
            csv_path.display()
        );

        let program = PipParser::parse_str(&script).unwrap();
        interp.eval(program).await.unwrap();
    }

    // Verify appended file contents
    let final_content = fs::read_to_string(&csv_path).unwrap();
    assert!(final_content.contains("Alice"));
    assert!(final_content.contains("Bob"));
    assert!(final_content.contains("Charlie"));
    assert!(final_content.contains("Diana"));

    let final_line_count = final_content.lines().count();
    assert_eq!(final_line_count, 5); // header + 4 data rows

    // Verify header appears only once (note: columns may be in different order)
    let header_count = final_content
        .lines()
        .filter(|line| {
            line.contains("age")
                && line.contains("id")
                && line.contains("name")
                && !line.contains("Alice")
                && !line.contains("Bob")
        })
        .count();
    assert_eq!(header_count, 1, "Header should appear only once");
}

#[tokio::test]
async fn test_csv_append_creates_file_if_not_exists() {
    let dir = tempdir().unwrap();
    let csv_path = dir.path().join("new_file.csv");

    // Append to non-existent file (should create it)
    let mut interp = Interpreter::new();
    let script = format!(
        r#"
        dim data = [
            {{"x": 1, "y": 2}},
            {{"x": 3, "y": 4}}
        ]
        export data to "{}" append
        "#,
        csv_path.display()
    );

    let program = PipParser::parse_str(&script).unwrap();
    interp.eval(program).await.unwrap();

    // Verify file was created
    assert!(csv_path.exists());
    let content = fs::read_to_string(&csv_path).unwrap();
    assert!(content.contains("x,y"));
    assert!(content.contains("1,2"));
    assert!(content.contains("3,4"));
}

#[tokio::test]
async fn test_csv_append_column_mismatch_error() {
    let dir = tempdir().unwrap();
    let csv_path = dir.path().join("data.csv");

    // Create initial CSV
    {
        let mut interp = Interpreter::new();
        let script = format!(
            r#"
            dim data = [
                {{"id": 1, "name": "Alice"}},
                {{"id": 2, "name": "Bob"}}
            ]
            export data to "{}"
            "#,
            csv_path.display()
        );

        let program = PipParser::parse_str(&script).unwrap();
        interp.eval(program).await.unwrap();
    }

    // Try to append with different columns - should fail
    {
        let mut interp = Interpreter::new();
        let script = format!(
            r#"
            dim new_data = [
                {{"user_id": 3, "email": "charlie@example.com"}}
            ]
            export new_data to "{}" append
            "#,
            csv_path.display()
        );

        let program = PipParser::parse_str(&script).unwrap();
        let result = interp.eval(program).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Column mismatch"));
    }
}

#[tokio::test]
async fn test_tsv_append_mode() {
    let dir = tempdir().unwrap();
    let tsv_path = dir.path().join("data.tsv");

    // Create initial TSV file
    {
        let mut interp = Interpreter::new();
        let script = format!(
            r#"
            dim data = [
                {{"product": "Widget", "price": 10.50}},
                {{"product": "Gadget", "price": 25.00}}
            ]
            export data to "{}"
            "#,
            tsv_path.display()
        );

        let program = PipParser::parse_str(&script).unwrap();
        interp.eval(program).await.unwrap();
    }

    // Append more data
    {
        let mut interp = Interpreter::new();
        let script = format!(
            r#"
            dim new_products = [
                {{"product": "Doohickey", "price": 15.75}}
            ]
            export new_products to "{}" append
            "#,
            tsv_path.display()
        );

        let program = PipParser::parse_str(&script).unwrap();
        interp.eval(program).await.unwrap();
    }

    // Verify TSV has tab separators and all data
    let content = fs::read_to_string(&tsv_path).unwrap();
    assert!(content.contains("\t")); // Has tab separators
    assert!(content.contains("Widget"));
    assert!(content.contains("Gadget"));
    assert!(content.contains("Doohickey"));

    let lines: Vec<_> = content.lines().collect();
    assert_eq!(lines.len(), 4); // header + 3 data rows
}

#[tokio::test]
async fn test_append_mode_not_supported_for_json() {
    let dir = tempdir().unwrap();
    let json_path = dir.path().join("data.json");

    let mut interp = Interpreter::new();
    let script = format!(
        r#"
        dim data = [{{"id": 1}}]
        export data to "{}" append
        "#,
        json_path.display()
    );

    let program = PipParser::parse_str(&script).unwrap();
    let result = interp.eval(program).await;

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Append mode is only supported for CSV and TSV"));
}

#[tokio::test]
async fn test_sheet_append_mode() {
    let dir = tempdir().unwrap();
    let csv_path = dir.path().join("sheet_data.csv");

    // Create initial sheet data
    {
        let mut interp = Interpreter::new();
        let _script = format!(
            r#"
            import "{}" into initial_data without headers
            export initial_data to "{}"
            "#,
            "tests/test_data/simple.csv",
            csv_path.display()
        );

        // First create a simple test CSV for importing
        let test_data_dir = dir.path().join("tests/test_data");
        fs::create_dir_all(&test_data_dir).unwrap();
        let test_csv = test_data_dir.join("simple.csv");
        fs::write(&test_csv, "A,B,C\n1,2,3\n4,5,6\n").unwrap();

        let script = format!(
            r#"
            import "{}" into initial_data
            export initial_data to "{}"
            "#,
            test_csv.display(),
            csv_path.display()
        );

        let program = PipParser::parse_str(&script).unwrap();
        interp.eval(program).await.unwrap();
    }

    // Append more sheet data
    {
        let mut interp = Interpreter::new();

        // Create a sheet with matching columns
        let mut sheet = Sheet::from_data(vec![
            vec![
                CellValue::String("A".to_string()),
                CellValue::String("B".to_string()),
                CellValue::String("C".to_string()),
            ],
            vec![CellValue::Int(7), CellValue::Int(8), CellValue::Int(9)],
            vec![CellValue::Int(10), CellValue::Int(11), CellValue::Int(12)],
        ]);
        sheet.name_columns_by_row(0).unwrap();

        interp
            .set_var("append_data", piptable_core::Value::Sheet(sheet))
            .await;

        let script = format!(
            r#"
            export append_data to "{}" append
            "#,
            csv_path.display()
        );

        let program = PipParser::parse_str(&script).unwrap();
        interp.eval(program).await.unwrap();
    }

    // Verify appended data
    let content = fs::read_to_string(&csv_path).unwrap();
    let lines: Vec<_> = content.lines().collect();
    assert_eq!(lines.len(), 5); // header + 2 original + 2 appended
    assert!(content.contains("1,2,3"));
    assert!(content.contains("4,5,6"));
    assert!(content.contains("7,8,9"));
    assert!(content.contains("10,11,12"));
}

#[tokio::test]
async fn test_headerless_csv_append() {
    let dir = tempdir().unwrap();
    let csv_path = dir.path().join("headerless.csv");

    // Create initial headerless CSV
    {
        let sheet = Sheet::from_data(vec![
            vec![CellValue::Int(1), CellValue::String("Alice".to_string())],
            vec![CellValue::Int(2), CellValue::String("Bob".to_string())],
        ]);
        sheet.save_as_csv(&csv_path).unwrap();
    }

    // Verify initial file has no headers (just data)
    let content = fs::read_to_string(&csv_path).unwrap();
    assert!(content.contains("1,Alice"));
    assert!(content.contains("2,Bob"));
    let line_count = content.lines().count();
    assert_eq!(line_count, 2);

    // Append more headerless data
    {
        let mut interp = Interpreter::new();

        // Create headerless sheet to append
        let append_sheet = Sheet::from_data(vec![
            vec![CellValue::Int(3), CellValue::String("Charlie".to_string())],
            vec![CellValue::Int(4), CellValue::String("Diana".to_string())],
        ]);

        interp
            .set_var("new_data", piptable_core::Value::Sheet(append_sheet))
            .await;

        let script = format!(
            r#"
            export new_data to "{}" append
            "#,
            csv_path.display()
        );

        let program = PipParser::parse_str(&script).unwrap();
        interp.eval(program).await.unwrap();
    }

    // Verify appended data
    let final_content = fs::read_to_string(&csv_path).unwrap();
    assert!(final_content.contains("1,Alice"));
    assert!(final_content.contains("2,Bob"));
    assert!(final_content.contains("3,Charlie"));
    assert!(final_content.contains("4,Diana"));

    let final_line_count = final_content.lines().count();
    assert_eq!(final_line_count, 4); // All data rows, no headers
}

#[tokio::test]
async fn test_mixed_header_append_error() {
    let dir = tempdir().unwrap();
    let csv_path = dir.path().join("mixed.csv");

    // Create CSV with headers
    {
        let mut interp = Interpreter::new();
        let script = format!(
            r#"
            dim data = [
                {{"id": 1, "name": "Alice"}},
                {{"id": 2, "name": "Bob"}}
            ]
            export data to "{}"
            "#,
            csv_path.display()
        );

        let program = PipParser::parse_str(&script).unwrap();
        interp.eval(program).await.unwrap();
    }

    // Try to append headerless data - should work as it detects headers exist
    {
        let mut interp = Interpreter::new();

        // Create headerless sheet (but with matching structure)
        let headerless_sheet = Sheet::from_data(vec![vec![
            CellValue::Int(3),
            CellValue::String("Charlie".to_string()),
        ]]);

        interp
            .set_var("new_data", piptable_core::Value::Sheet(headerless_sheet))
            .await;

        let script = format!(
            r#"
            export new_data to "{}" append
            "#,
            csv_path.display()
        );

        let program = PipParser::parse_str(&script).unwrap();
        let result = interp.eval(program).await;

        // This should fail due to column name mismatch
        assert!(result.is_err());
    }
}

#[tokio::test]
async fn test_headerless_tsv_append() {
    let dir = tempdir().unwrap();
    let tsv_path = dir.path().join("headerless.tsv");

    // Create initial headerless TSV
    {
        let sheet = Sheet::from_data(vec![
            vec![
                CellValue::String("Widget".to_string()),
                CellValue::Float(10.50),
            ],
            vec![
                CellValue::String("Gadget".to_string()),
                CellValue::Float(25.00),
            ],
        ]);
        sheet
            .save_as_csv_with_options(&tsv_path, CsvOptions::tsv())
            .unwrap();
    }

    // Append more headerless data
    {
        let mut interp = Interpreter::new();

        let append_sheet = Sheet::from_data(vec![vec![
            CellValue::String("Doohickey".to_string()),
            CellValue::Float(15.75),
        ]]);

        interp
            .set_var("new_data", piptable_core::Value::Sheet(append_sheet))
            .await;

        let script = format!(
            r#"
            export new_data to "{}" append
            "#,
            tsv_path.display()
        );

        let program = PipParser::parse_str(&script).unwrap();
        interp.eval(program).await.unwrap();
    }

    // Verify TSV has tab separators and all data
    let content = fs::read_to_string(&tsv_path).unwrap();
    assert!(content.contains("\t")); // Has tab separators
    assert!(content.contains("Widget"));
    assert!(content.contains("Gadget"));
    assert!(content.contains("Doohickey"));

    let lines: Vec<_> = content.lines().collect();
    assert_eq!(lines.len(), 3); // 3 data rows, no headers
}

#[tokio::test]
async fn test_string_data_header_detection() {
    let dir = tempdir().unwrap();
    let csv_path = dir.path().join("string_data.csv");

    // Create CSV with all string data (not headers, just string data)
    {
        let sheet = Sheet::from_data(vec![
            vec![
                CellValue::String("Alice".to_string()),
                CellValue::String("Smith".to_string()),
                CellValue::String("Engineer".to_string()),
            ],
            vec![
                CellValue::String("Bob".to_string()),
                CellValue::String("Jones".to_string()),
                CellValue::String("Manager".to_string()),
            ],
        ]);
        sheet.save_as_csv(&csv_path).unwrap();
    }

    // Try to append more string data - should work as both are headerless
    {
        let mut interp = Interpreter::new();

        let append_sheet = Sheet::from_data(vec![vec![
            CellValue::String("Charlie".to_string()),
            CellValue::String("Brown".to_string()),
            CellValue::String("Analyst".to_string()),
        ]]);

        interp
            .set_var("new_data", piptable_core::Value::Sheet(append_sheet))
            .await;

        let script = format!(
            r#"
            export new_data to "{}" append
            "#,
            csv_path.display()
        );

        let program = PipParser::parse_str(&script).unwrap();
        interp.eval(program).await.unwrap();
    }

    // Verify data was appended correctly
    let content = fs::read_to_string(&csv_path).unwrap();
    assert!(content.contains("Alice,Smith,Engineer"));
    assert!(content.contains("Bob,Jones,Manager"));
    assert!(content.contains("Charlie,Brown,Analyst"));
    let lines: Vec<_> = content.lines().collect();
    assert_eq!(lines.len(), 3); // All data rows, no headers
}

#[tokio::test]
async fn test_empty_sheet_append() {
    let dir = tempdir().unwrap();
    let csv_path = dir.path().join("empty_append.csv");

    // Create empty CSV file
    {
        let sheet = Sheet::new();
        sheet.save_as_csv(&csv_path).unwrap();
    }

    // Append data to empty file - should work
    {
        let mut interp = Interpreter::new();

        let data_sheet = Sheet::from_data(vec![vec![
            CellValue::Int(1),
            CellValue::String("Test".to_string()),
        ]]);

        interp
            .set_var("data", piptable_core::Value::Sheet(data_sheet))
            .await;

        let script = format!(
            r#"
            export data to "{}" append
            "#,
            csv_path.display()
        );

        let program = PipParser::parse_str(&script).unwrap();
        interp.eval(program).await.unwrap();
    }

    // Verify data was added
    let content = fs::read_to_string(&csv_path).unwrap();
    assert!(content.contains("1,Test"));
}
