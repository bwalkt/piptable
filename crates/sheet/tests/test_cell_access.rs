use piptable_sheet::{CellValue, Result, Sheet};

#[test]
fn test_a1_cell_access() -> Result<()> {
    let mut sheet = Sheet::from_data(vec![
        vec!["Name", "Age", "City"],
        vec!["Alice", "30", "NYC"],
        vec!["Bob", "25", "LA"],
    ]);

    // Test get
    assert_eq!(sheet.get_a1("A1")?.as_str(), "Name");
    assert_eq!(sheet.get_a1("B2")?.as_str(), "30");
    assert_eq!(sheet.get_a1("C3")?.as_str(), "LA");

    // Test set
    sheet.set_a1("B2", "31")?;
    assert_eq!(sheet.get_a1("B2")?.as_str(), "31");

    // Test case insensitive
    assert_eq!(sheet.get_a1("a1")?.as_str(), "Name");

    Ok(())
}

#[test]
fn test_a1_range_access() -> Result<()> {
    let sheet = Sheet::from_data(vec![
        vec!["A", "B", "C", "D"],
        vec!["1", "2", "3", "4"],
        vec!["5", "6", "7", "8"],
        vec!["9", "10", "11", "12"],
    ]);

    // Get sub-sheet
    let sub = sheet.get_range("B2:C3")?;
    assert_eq!(sub.row_count(), 2);
    assert_eq!(sub.col_count(), 2);
    assert_eq!(sub.get(0, 0)?.as_str(), "2");
    assert_eq!(sub.get(0, 1)?.as_str(), "3");
    assert_eq!(sub.get(1, 0)?.as_str(), "6");
    assert_eq!(sub.get(1, 1)?.as_str(), "7");

    // Single cell range
    let single = sheet.get_range("B2")?;
    assert_eq!(single.row_count(), 1);
    assert_eq!(single.col_count(), 1);
    assert_eq!(single.get(0, 0)?.as_str(), "2");

    Ok(())
}

#[test]
fn test_named_row_access() -> Result<()> {
    let mut sheet = Sheet::from_data(vec![
        vec!["EMP001", "Alice", "30"],
        vec!["EMP002", "Bob", "25"],
        vec!["EMP003", "Charlie", "35"],
    ]);

    // Name rows by first column
    sheet.name_rows_by_column(0)?;

    // Access by row name
    let row = sheet.row_by_name("EMP002")?;
    assert_eq!(row[1].as_str(), "Bob");
    assert_eq!(row[2].as_str(), "25");

    Ok(())
}

#[test]
fn test_named_column_access() -> Result<()> {
    let mut sheet = Sheet::from_data(vec![
        vec!["Name", "Age", "City"],
        vec!["Alice", "30", "NYC"],
        vec!["Bob", "25", "LA"],
    ]);

    // Name columns by first row
    sheet.name_columns_by_row(0)?;

    // Access by column name
    let ages = sheet.column_by_name("Age")?;
    assert_eq!(ages[1].as_str(), "30");
    assert_eq!(ages[2].as_str(), "25");

    // Set by column name
    sheet.set_by_name(1, "Age", "31")?;
    assert_eq!(sheet.get_by_name(1, "Age")?.as_str(), "31");

    Ok(())
}

#[test]
fn test_map_operation() -> Result<()> {
    let mut sheet = Sheet::from_data(vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]]);

    // Double all numbers
    sheet.map(|cell| match cell {
        CellValue::Int(n) => CellValue::Int(n * 2),
        v => v.clone(),
    });

    assert_eq!(sheet.get(0, 0)?.as_int().unwrap(), 2);
    assert_eq!(sheet.get(1, 1)?.as_int().unwrap(), 10);
    assert_eq!(sheet.get(2, 2)?.as_int().unwrap(), 18);

    Ok(())
}

#[test]
fn test_filter_rows() -> Result<()> {
    let mut sheet = Sheet::from_data(vec![
        vec![1, 2, 3],
        vec![4, 5, 6],
        vec![7, 8, 9],
        vec![10, 11, 12],
    ]);

    // Keep only rows where first column > 5
    sheet.filter_rows(|_, row| row[0].as_int().unwrap_or(0) > 5);

    assert_eq!(sheet.row_count(), 2);
    assert_eq!(sheet.get(0, 0)?.as_int().unwrap(), 7);
    assert_eq!(sheet.get(1, 0)?.as_int().unwrap(), 10);

    Ok(())
}

#[test]
fn test_filter_columns() -> Result<()> {
    let mut sheet = Sheet::from_data(vec![
        vec!["Name", "Age", "City", "Country"],
        vec!["Alice", "30", "NYC", "USA"],
        vec!["Bob", "25", "LA", "USA"],
    ]);

    sheet.name_columns_by_row(0)?;

    // Keep only Name and City columns
    sheet.filter_columns(|_, name| name == "Name" || name == "City")?;

    assert_eq!(sheet.col_count(), 2);
    assert_eq!(
        sheet.column_names().unwrap(),
        &vec!["Name".to_string(), "City".to_string()]
    );

    Ok(())
}

#[test]
fn test_format_column() -> Result<()> {
    let mut sheet = Sheet::from_data(vec![
        vec!["Name", "Age"],
        vec!["Alice", "30"],
        vec!["Bob", "25"],
    ]);

    sheet.name_columns_by_row(0)?;

    // Convert Age column to integers
    sheet.format_column_by_name("Age", |cell| match cell.as_str().parse::<i64>() {
        Ok(n) => CellValue::Int(n),
        Err(_) => cell.clone(),
    })?;

    assert!(matches!(sheet.get_by_name(1, "Age")?, CellValue::Int(30)));
    assert!(matches!(sheet.get_by_name(2, "Age")?, CellValue::Int(25)));

    Ok(())
}

#[test]
fn test_remove_empty_rows() -> Result<()> {
    let mut sheet = Sheet::from_data(vec![
        vec![
            CellValue::String("Name".to_string()),
            CellValue::String("Age".to_string()),
        ],
        vec![
            CellValue::String("Alice".to_string()),
            CellValue::String("30".to_string()),
        ],
        vec![CellValue::Null, CellValue::Null],
        vec![
            CellValue::String("".to_string()),
            CellValue::String("".to_string()),
        ],
        vec![
            CellValue::String("Bob".to_string()),
            CellValue::String("25".to_string()),
        ],
    ]);

    sheet.remove_empty_rows();

    assert_eq!(sheet.row_count(), 3);
    assert_eq!(sheet.get(0, 0)?.as_str(), "Name");
    assert_eq!(sheet.get(1, 0)?.as_str(), "Alice");
    assert_eq!(sheet.get(2, 0)?.as_str(), "Bob");

    Ok(())
}

#[test]
fn test_transpose() -> Result<()> {
    let mut sheet = Sheet::from_data(vec![
        vec!["A", "B", "C"],
        vec!["1", "2", "3"],
        vec!["4", "5", "6"],
    ]);

    sheet.transpose();

    assert_eq!(sheet.row_count(), 3);
    assert_eq!(sheet.col_count(), 3);

    assert_eq!(sheet.get(0, 0)?.as_str(), "A");
    assert_eq!(sheet.get(0, 1)?.as_str(), "1");
    assert_eq!(sheet.get(0, 2)?.as_str(), "4");

    assert_eq!(sheet.get(1, 0)?.as_str(), "B");
    assert_eq!(sheet.get(2, 0)?.as_str(), "C");

    Ok(())
}

#[test]
fn test_select_columns() -> Result<()> {
    let mut sheet = Sheet::from_data(vec![
        vec!["Name", "Age", "City", "Country"],
        vec!["Alice", "30", "NYC", "USA"],
        vec!["Bob", "25", "LA", "USA"],
    ]);

    sheet.name_columns_by_row(0)?;

    // Cherry-pick columns
    sheet.select_columns(&["Name", "City"])?;

    assert_eq!(sheet.col_count(), 2);
    assert_eq!(
        sheet.column_names().unwrap(),
        &vec!["Name".to_string(), "City".to_string()]
    );
    assert_eq!(sheet.get(1, 0)?.as_str(), "Alice");
    assert_eq!(sheet.get(1, 1)?.as_str(), "NYC");

    Ok(())
}

#[test]
fn test_remove_columns() -> Result<()> {
    let mut sheet = Sheet::from_data(vec![
        vec!["Name", "Age", "City", "Country"],
        vec!["Alice", "30", "NYC", "USA"],
        vec!["Bob", "25", "LA", "USA"],
    ]);

    sheet.name_columns_by_row(0)?;

    // Remove Age and Country columns
    sheet.remove_columns(&["Age", "Country"])?;

    assert_eq!(sheet.col_count(), 2);
    assert_eq!(
        sheet.column_names().unwrap(),
        &vec!["Name".to_string(), "City".to_string()]
    );
    assert_eq!(sheet.get(1, 0)?.as_str(), "Alice");
    assert_eq!(sheet.get(1, 1)?.as_str(), "NYC");

    Ok(())
}
