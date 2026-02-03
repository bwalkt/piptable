use piptable_interpreter::Interpreter;
use piptable_parser::PipParser;
use piptable_sheet::{CellValue, CsvOptions, Sheet};
use serde_json::Value as JsonValue;
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
    assert!(content.contains('\t')); // Has tab separators
    assert!(content.contains("Widget"));
    assert!(content.contains("Gadget"));
    assert!(content.contains("Doohickey"));

    let lines: Vec<_> = content.lines().collect();
    assert_eq!(lines.len(), 4); // header + 3 data rows
}

/// Verifies distinct append mode removes duplicates.
#[test]
fn test_append_distinct_mode() {
    use piptable_interpreter::io::{export_sheet_with_mode, ExportMode};

    let dir = tempdir().unwrap();
    let csv_path = dir.path().join("distinct_test.csv");

    // Create initial CSV with some data
    let initial_data = "id,name,value\n1,Alice,100\n2,Bob,200\n3,Charlie,300";
    fs::write(&csv_path, initial_data).unwrap();

    // Create new sheet with overlapping and new data
    let mut new_sheet = Sheet::new();
    new_sheet.row_append(vec!["id", "name", "value"]).unwrap();
    new_sheet
        .row_append(vec!["2", "Bob Updated", "250"])
        .unwrap(); // Duplicate ID (should be skipped)
    new_sheet.row_append(vec!["4", "David", "400"]).unwrap(); // New ID (should be added)
    new_sheet
        .row_append(vec!["1", "Alice Updated", "150"])
        .unwrap(); // Duplicate ID (should be skipped)
    new_sheet.row_append(vec!["5", "Eve", "500"]).unwrap(); // New ID (should be added)
    new_sheet.name_columns_by_row(0).unwrap();

    // Append with distinct mode (key column 0 = id)
    export_sheet_with_mode(
        &new_sheet,
        &csv_path.display().to_string(),
        ExportMode::AppendDistinct {
            key_column: Some(0),
        },
    )
    .unwrap();

    // Read result and verify
    let content = fs::read_to_string(&csv_path).unwrap();
    let lines: Vec<_> = content.lines().collect();

    // Should have header + 3 original + 2 new = 6 rows
    assert_eq!(lines.len(), 6);
    assert!(content.contains("1,Alice,100")); // Original Alice
    assert!(content.contains("2,Bob,200")); // Original Bob
    assert!(content.contains("4,David,400")); // New David
    assert!(content.contains("5,Eve,500")); // New Eve
    assert!(!content.contains("Bob Updated")); // Should not have updated Bob
    assert!(!content.contains("Alice Updated")); // Should not have updated Alice
}

/// Verifies append-or-update behavior updates existing rows.
#[test]
fn test_append_or_update_mode() {
    use piptable_interpreter::io::{export_sheet_with_mode, ExportMode};

    let dir = tempdir().unwrap();
    let csv_path = dir.path().join("update_test.csv");

    // Create initial CSV with some data
    let initial_data = "id,name,value\n1,Alice,100\n2,Bob,200\n3,Charlie,300";
    fs::write(&csv_path, initial_data).unwrap();

    // Create new sheet with overlapping and new data
    let mut new_sheet = Sheet::new();
    new_sheet.row_append(vec!["id", "name", "value"]).unwrap();
    new_sheet
        .row_append(vec!["2", "Bob Updated", "250"])
        .unwrap(); // Update existing
    new_sheet.row_append(vec!["4", "David", "400"]).unwrap(); // New row
    new_sheet
        .row_append(vec!["1", "Alice Updated", "150"])
        .unwrap(); // Update existing
    new_sheet.name_columns_by_row(0).unwrap();

    // Append with update mode (key column 0 = id)
    export_sheet_with_mode(
        &new_sheet,
        &csv_path.display().to_string(),
        ExportMode::AppendOrUpdate {
            key_column: Some(0),
        },
    )
    .unwrap();

    // Read result and verify
    let content = fs::read_to_string(&csv_path).unwrap();
    let lines: Vec<_> = content.lines().collect();

    // Should have header + 3 original + 1 new = 5 rows (updates don't add rows)
    assert_eq!(lines.len(), 5);
    assert!(content.contains("1,Alice Updated,150")); // Updated Alice
    assert!(content.contains("2,Bob Updated,250")); // Updated Bob
    assert!(content.contains("3,Charlie,300")); // Unchanged Charlie
    assert!(content.contains("4,David,400")); // New David
    assert!(!content.contains("Alice,100")); // Should not have original Alice
    assert!(!content.contains("Bob,200")); // Should not have original Bob
}

/// Ensures append-or-update works without header rows.
#[test]
fn test_append_or_update_with_column_names_no_header_row() {
    use piptable_interpreter::io::{export_sheet_with_mode, ExportMode};

    let dir = tempdir().unwrap();
    let csv_path = dir.path().join("update_no_header_test.csv");

    // Create initial CSV without a header row
    let initial_data = "1,Alice,100\n2,Bob,200\n3,Charlie,300";
    fs::write(&csv_path, initial_data).unwrap();

    // Create new sheet with column names but data that should update first row
    let mut new_sheet = Sheet::new();
    new_sheet.row_append(vec!["id", "name", "value"]).unwrap();
    new_sheet
        .row_append(vec!["1", "Alice Updated", "150"])
        .unwrap(); // Should update first data row
    new_sheet.name_columns_by_row(0).unwrap();

    // Append with update mode
    export_sheet_with_mode(
        &new_sheet,
        &csv_path.display().to_string(),
        ExportMode::AppendOrUpdate {
            key_column: Some(0),
        },
    )
    .unwrap();

    // Read result and verify
    let content = fs::read_to_string(&csv_path).unwrap();
    let lines: Vec<_> = content.lines().collect();

    // Should have updated the first row (Alice)
    assert_eq!(lines.len(), 3); // Same number of rows
    assert!(content.contains("1,Alice Updated,150")); // Updated Alice
    assert!(content.contains("2,Bob,200")); // Unchanged Bob
    assert!(content.contains("3,Charlie,300")); // Unchanged Charlie
    assert!(!content.contains("1,Alice,100")); // Should not have original Alice
}

/// Ensures header detection works for string-only CSVs.
#[test]
fn test_string_only_csv_header_detection() {
    use piptable_interpreter::io::{export_sheet_with_mode, ExportMode};

    let dir = tempdir().unwrap();
    let csv_path = dir.path().join("string_only_test.csv");

    // Create initial CSV with string-only data but clear header pattern
    let initial_data = "Product,Category,Description\nWidget,Tools,A small useful tool\nGadget,Electronics,An electronic device that does things";
    fs::write(&csv_path, initial_data).unwrap();

    // Create new sheet to append
    let mut new_sheet = Sheet::new();
    new_sheet
        .row_append(vec!["Product", "Category", "Description"])
        .unwrap();
    new_sheet
        .row_append(vec!["Doohickey", "Misc", "Something mysterious and useful"])
        .unwrap();
    new_sheet.name_columns_by_row(0).unwrap();

    // Append normally (should detect headers correctly)
    export_sheet_with_mode(
        &new_sheet,
        &csv_path.display().to_string(),
        ExportMode::Append,
    )
    .unwrap();

    // Read result and verify
    let content = fs::read_to_string(&csv_path).unwrap();
    let lines: Vec<_> = content.lines().collect();

    // Should have header + 2 original + 1 new = 4 rows (no duplicate header)
    assert_eq!(lines.len(), 4);
    assert!(content.contains("Product,Category,Description")); // Header present once
    assert!(content.contains("Widget,Tools,A small useful tool")); // Original data
    assert!(content.contains("Doohickey,Misc,Something mysterious and useful")); // New data

    // Count header occurrences - should only appear once
    let header_count = content.matches("Product,Category,Description").count();
    assert_eq!(header_count, 1, "Header should only appear once");
}

/// Ensures distinct append ignores header rows as keys.
#[test]
fn test_append_distinct_ignores_header_keys() {
    use piptable_interpreter::io::{export_sheet_with_mode, ExportMode};

    let dir = tempdir().unwrap();
    let csv_path = dir.path().join("distinct_header_test.csv");

    // Create initial CSV where header value matches a potential data key
    let initial_data = "id,name,value\n1,Alice,100\nid,HeaderMatch,999"; // "id" appears as both header and data
    fs::write(&csv_path, initial_data).unwrap();

    // Create new sheet with "id" as a data value (should not be blocked by header)
    let mut new_sheet = Sheet::new();
    new_sheet.row_append(vec!["id", "name", "value"]).unwrap();
    new_sheet
        .row_append(vec!["id", "New Entry", "200"])
        .unwrap(); // This should be blocked by existing data, not header
    new_sheet.row_append(vec!["2", "Bob", "300"]).unwrap(); // This should be added
    new_sheet.name_columns_by_row(0).unwrap();

    // Append with distinct mode
    export_sheet_with_mode(
        &new_sheet,
        &csv_path.display().to_string(),
        ExportMode::AppendDistinct {
            key_column: Some(0),
        },
    )
    .unwrap();

    // Read result and verify
    let content = fs::read_to_string(&csv_path).unwrap();
    let lines: Vec<_> = content.lines().collect();

    // Should have header + 2 original + 1 new = 4 rows
    assert_eq!(lines.len(), 4);
    assert!(content.contains("1,Alice,100")); // Original Alice
    assert!(content.contains("id,HeaderMatch,999")); // Original data with "id" key
    assert!(content.contains("2,Bob,300")); // New Bob
    assert!(!content.contains("id,New Entry,200")); // Should be blocked by existing "id" data, not header
}

#[tokio::test]
async fn test_json_append_mode() {
    let dir = tempdir().unwrap();
    let json_path = dir.path().join("data.json");

    // Initial JSON data (note: keys will be sorted alphabetically when loaded)
    let initial_json = r#"[{"name":"Alice","age":30},{"name":"Bob","age":25}]"#;
    std::fs::write(&json_path, initial_json).unwrap();

    let mut interp = Interpreter::new();

    // Create sheet to append (columns in alphabetical order to match JSON)
    let mut append_sheet = Sheet::new();
    append_sheet.row_append(vec!["age", "name"]).unwrap();
    append_sheet.row_append(vec!["35", "Charlie"]).unwrap();
    append_sheet.row_append(vec!["28", "David"]).unwrap();
    append_sheet.name_columns_by_row(0).unwrap();

    interp
        .set_var(
            "new_data",
            piptable_core::Value::Sheet(Box::new(append_sheet)),
        )
        .await
        .unwrap();

    // Append more data using DSL
    let script = format!(
        r#"
        export new_data to "{}" append
        "#,
        json_path.display()
    );

    let program = PipParser::parse_str(&script).unwrap();
    let _result: piptable_core::Value = interp.eval(program).await.unwrap();

    // Load and verify
    let appended_sheet = Sheet::from_json(&json_path).unwrap();

    // Should have header + 4 data rows (2 original + 2 appended)
    assert_eq!(appended_sheet.row_count(), 5);
    assert_eq!(appended_sheet.col_count(), 2);

    // Verify data (columns are in alphabetical order: age, name)
    assert_eq!(appended_sheet.get(1, 1).unwrap().as_str(), "Alice");
    assert_eq!(appended_sheet.get(2, 1).unwrap().as_str(), "Bob");
    assert_eq!(appended_sheet.get(3, 1).unwrap().as_str(), "Charlie");
    assert_eq!(appended_sheet.get(4, 1).unwrap().as_str(), "David");
    assert_eq!(appended_sheet.get(1, 0).unwrap().as_int(), Some(30));
    assert_eq!(appended_sheet.get(2, 0).unwrap().as_int(), Some(25));
    assert_eq!(appended_sheet.get(3, 0).unwrap().as_int(), Some(35));
    assert_eq!(appended_sheet.get(4, 0).unwrap().as_int(), Some(28));
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
            .set_var("append_data", piptable_core::Value::Sheet(Box::new(sheet)))
            .await
            .unwrap();

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
            .set_var(
                "new_data",
                piptable_core::Value::Sheet(Box::new(append_sheet)),
            )
            .await
            .unwrap();

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

    // Try to append headerless data - should fail due to header mismatch
    {
        let mut interp = Interpreter::new();

        // Create headerless sheet (but with matching structure)
        let headerless_sheet = Sheet::from_data(vec![vec![
            CellValue::Int(3),
            CellValue::String("Charlie".to_string()),
        ]]);

        interp
            .set_var(
                "new_data",
                piptable_core::Value::Sheet(Box::new(headerless_sheet)),
            )
            .await
            .unwrap();

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
            .set_var(
                "new_data",
                piptable_core::Value::Sheet(Box::new(append_sheet)),
            )
            .await
            .unwrap();

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
    assert!(content.contains('\t')); // Has tab separators
    assert!(content.contains("Widget"));
    assert!(content.contains("Gadget"));
    assert!(content.contains("Doohickey"));

    let lines: Vec<_> = content.lines().collect();
    assert_eq!(lines.len(), 3); // 3 data rows, no headers
}

#[tokio::test]
async fn test_jsonl_append_mode() {
    let dir = tempdir().unwrap();
    let jsonl_path = dir.path().join("data.jsonl");

    // First, create initial JSONL file
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
            jsonl_path.display()
        );

        let program = PipParser::parse_str(&script).unwrap();
        interp.eval(program).await.unwrap();
    }

    // Verify initial file contents
    let content = fs::read_to_string(&jsonl_path).unwrap();
    let lines: Vec<_> = content.lines().collect();
    assert_eq!(lines.len(), 2);

    // Parse and verify first line
    let first: JsonValue = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(first["id"], 1);
    assert_eq!(first["name"], "Alice");

    // Now append more data
    {
        let mut interp = Interpreter::new();
        let script = format!(
            r#"
            dim new_data = [
                {{"id": 3, "name": "Charlie"}},
                {{"id": 4, "name": "Diana"}}
            ]
            export new_data to "{}" append
            "#,
            jsonl_path.display()
        );

        let program = PipParser::parse_str(&script).unwrap();
        interp.eval(program).await.unwrap();
    }

    // Verify appended file contents
    let final_content = fs::read_to_string(&jsonl_path).unwrap();
    let final_lines: Vec<_> = final_content.lines().collect();
    assert_eq!(final_lines.len(), 4);

    // Verify all records are present
    let third: JsonValue = serde_json::from_str(final_lines[2]).unwrap();
    assert_eq!(third["id"], 3);
    assert_eq!(third["name"], "Charlie");

    let fourth: JsonValue = serde_json::from_str(final_lines[3]).unwrap();
    assert_eq!(fourth["id"], 4);
    assert_eq!(fourth["name"], "Diana");
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
            .set_var(
                "new_data",
                piptable_core::Value::Sheet(Box::new(append_sheet)),
            )
            .await
            .unwrap();

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
            .set_var("data", piptable_core::Value::Sheet(Box::new(data_sheet)))
            .await
            .unwrap();

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

/// Validates key column bounds for append/update modes.
#[test]
fn test_key_column_bounds_validation() {
    use piptable_interpreter::io::{export_sheet_with_mode, ExportMode};

    let dir = tempdir().unwrap();
    let csv_path = dir.path().join("bounds_test.csv");

    // Create initial CSV with 2 columns
    let initial_data = "1,Alice\n2,Bob";
    fs::write(&csv_path, initial_data).unwrap();

    // Create new sheet with 2 columns
    let new_sheet = Sheet::from_data(vec![vec!["3", "Charlie"]]);

    // Test with key column index 2 (out of bounds for 2 columns)
    let result = export_sheet_with_mode(
        &new_sheet,
        &csv_path.display().to_string(),
        ExportMode::AppendDistinct {
            key_column: Some(2), // This should fail - column index 2 is out of bounds
        },
    );

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .contains("Key column index 2 is out of bounds"));

    // Test with valid key column index
    let result = export_sheet_with_mode(
        &new_sheet,
        &csv_path.display().to_string(),
        ExportMode::AppendDistinct {
            key_column: Some(1), // This should work - column index 1 is valid
        },
    );

    assert!(result.is_ok());
}

/// Validates key column bounds for append-or-update mode.
#[test]
fn test_key_column_bounds_validation_update_mode() {
    use piptable_interpreter::io::{export_sheet_with_mode, ExportMode};

    let dir = tempdir().unwrap();
    let csv_path = dir.path().join("bounds_update_test.csv");

    // Create initial CSV with 2 columns
    let initial_data = "1,Alice\n2,Bob";
    fs::write(&csv_path, initial_data).unwrap();

    // Create new sheet with 2 columns
    let new_sheet = Sheet::from_data(vec![vec!["3", "Charlie"]]);

    // Test with key column index 3 (out of bounds for 2 columns)
    let result = export_sheet_with_mode(
        &new_sheet,
        &csv_path.display().to_string(),
        ExportMode::AppendOrUpdate {
            key_column: Some(3), // This should fail - column index 3 is out of bounds
        },
    );

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .contains("Key column index 3 is out of bounds"));

    // Test with valid key column index
    let result = export_sheet_with_mode(
        &new_sheet,
        &csv_path.display().to_string(),
        ExportMode::AppendOrUpdate {
            key_column: Some(1), // This should work - column index 1 is valid
        },
    );

    assert!(result.is_ok());
}
