use piptable_sheet::{Book, CellValue, Sheet, SheetError};
use tempfile::tempdir;

// ===== Sheet Creation Tests =====

#[test]
fn test_sheet_from_data() {
    let sheet = Sheet::from_data(vec![vec![1, 2, 3], vec![4, 5, 6]]);

    assert_eq!(sheet.row_count(), 2);
    assert_eq!(sheet.col_count(), 3);
    assert_eq!(sheet.get(0, 0).unwrap(), &CellValue::Int(1));
    assert_eq!(sheet.get(1, 2).unwrap(), &CellValue::Int(6));
}

#[test]
fn test_sheet_from_strings() {
    let sheet = Sheet::from_data(vec![
        vec!["Name", "Age", "City"],
        vec!["Alice", "30", "NYC"],
    ]);

    assert_eq!(sheet.row_count(), 2);
    assert_eq!(
        sheet.get(0, 0).unwrap(),
        &CellValue::String("Name".to_string())
    );
}

// ===== Row Operations Tests =====

#[test]
fn test_row_crud() {
    let mut sheet = Sheet::from_data(vec![vec![1, 2], vec![3, 4]]);

    // Append
    sheet.row_append(vec![5, 6]).unwrap();
    assert_eq!(sheet.row_count(), 3);
    assert_eq!(sheet.get(2, 0).unwrap(), &CellValue::Int(5));

    // Insert
    sheet.row_insert(1, vec![7, 8]).unwrap();
    assert_eq!(sheet.row_count(), 4);
    assert_eq!(sheet.get(1, 0).unwrap(), &CellValue::Int(7));

    // Update
    sheet.row_update(0, vec![10, 20]).unwrap();
    assert_eq!(sheet.get(0, 0).unwrap(), &CellValue::Int(10));

    // Delete
    let deleted = sheet.row_delete(1).unwrap();
    assert_eq!(deleted[0], CellValue::Int(7));
    assert_eq!(sheet.row_count(), 3);
}

#[test]
fn test_row_length_mismatch() {
    let mut sheet = Sheet::from_data(vec![vec![1, 2, 3]]);

    let result = sheet.row_append(vec![1, 2]); // Wrong length
    assert!(matches!(result, Err(SheetError::LengthMismatch { .. })));
}

#[test]
fn test_row_delete_where() {
    let mut sheet = Sheet::from_data(vec![vec![1], vec![2], vec![3], vec![4], vec![5]]);

    let deleted = sheet.row_delete_where(|row| row[0].as_int().unwrap_or(0) % 2 == 0);

    assert_eq!(deleted, 2); // Removed 2 and 4
    assert_eq!(sheet.row_count(), 3);
}

// ===== Column Operations Tests =====

#[test]
fn test_column_crud() {
    let mut sheet = Sheet::from_data(vec![vec![1, 2], vec![3, 4]]);

    // Append
    sheet.column_append(vec![5, 6]).unwrap();
    assert_eq!(sheet.col_count(), 3);
    assert_eq!(sheet.get(0, 2).unwrap(), &CellValue::Int(5));

    // Insert
    sheet.column_insert(1, vec![7, 8]).unwrap();
    assert_eq!(sheet.col_count(), 4);
    assert_eq!(sheet.get(0, 1).unwrap(), &CellValue::Int(7));

    // Update
    sheet.column_update(0, vec![10, 30]).unwrap();
    assert_eq!(sheet.get(0, 0).unwrap(), &CellValue::Int(10));

    // Delete
    let deleted = sheet.column_delete(1).unwrap();
    assert_eq!(deleted[0], CellValue::Int(7));
    assert_eq!(sheet.col_count(), 3);
}

#[test]
fn test_column_by_name() {
    let mut sheet = Sheet::from_data(vec![
        vec!["Name", "Age", "City"],
        vec!["Alice", "30", "NYC"],
        vec!["Bob", "25", "LA"],
    ]);

    sheet.name_columns_by_row(0).unwrap();

    let ages = sheet.column_by_name("Age").unwrap();
    assert_eq!(ages.len(), 3);
    assert_eq!(ages[1], CellValue::String("30".to_string()));

    // Delete by name
    sheet.column_delete_by_name("City").unwrap();
    assert_eq!(sheet.col_count(), 2);
}

// ===== Named Access Tests =====

#[test]
fn test_named_columns() {
    let mut sheet = Sheet::from_data(vec![
        vec!["A", "B", "C"],
        vec!["1", "2", "3"],
    ]);

    sheet.name_columns_by_row(0).unwrap();

    assert!(sheet.column_names().is_some());
    assert_eq!(sheet.column_names().unwrap(), &vec!["A", "B", "C"]);

    // Access by name
    let val = sheet.get_by_name(1, "B").unwrap();
    assert_eq!(val, &CellValue::String("2".to_string()));
}

#[test]
fn test_columns_not_named_error() {
    let sheet = Sheet::from_data(vec![vec![1, 2]]);

    let result = sheet.column_by_name("A");
    assert!(matches!(result, Err(SheetError::ColumnsNotNamed)));
}

#[test]
fn test_duplicate_column_names_error() {
    let mut sheet = Sheet::from_data(vec![
        vec!["A", "B", "A"], // Duplicate "A"
        vec!["1", "2", "3"],
    ]);

    let result = sheet.name_columns_by_row(0);
    assert!(matches!(result, Err(SheetError::DuplicateColumnName { .. })));
}

// ===== Transformation Tests =====

#[test]
fn test_map_all_cells() {
    let mut sheet = Sheet::from_data(vec![vec![1, 2], vec![3, 4]]);

    sheet.map(|cell| {
        if let Some(i) = cell.as_int() {
            CellValue::Int(i * 10)
        } else {
            cell.clone()
        }
    });

    assert_eq!(sheet.get(0, 0).unwrap(), &CellValue::Int(10));
    assert_eq!(sheet.get(1, 1).unwrap(), &CellValue::Int(40));
}

#[test]
fn test_column_map() {
    let mut sheet = Sheet::from_data(vec![vec![1, 2], vec![3, 4]]);

    sheet
        .column_map(0, |cell| {
            CellValue::Int(cell.as_int().unwrap_or(0) * 2)
        })
        .unwrap();

    assert_eq!(sheet.get(0, 0).unwrap(), &CellValue::Int(2));
    assert_eq!(sheet.get(1, 0).unwrap(), &CellValue::Int(6));
    // Column 1 unchanged
    assert_eq!(sheet.get(0, 1).unwrap(), &CellValue::Int(2));
}

#[test]
fn test_filter_rows() {
    let mut sheet = Sheet::from_data(vec![vec![1], vec![2], vec![3], vec![4]]);

    sheet.filter_rows(|row| row[0].as_int().unwrap_or(0) > 2);

    assert_eq!(sheet.row_count(), 2);
    assert_eq!(sheet.get(0, 0).unwrap(), &CellValue::Int(3));
}

// ===== CSV Tests =====

#[test]
fn test_csv_roundtrip() {
    let original = Sheet::from_data(vec![
        vec!["name", "value"],
        vec!["test", "42"],
    ]);

    let csv = original.to_csv_string().unwrap();
    let restored = Sheet::from_csv_str(&csv).unwrap();

    assert_eq!(original.row_count(), restored.row_count());
    assert_eq!(original.col_count(), restored.col_count());
}

#[test]
fn test_csv_type_inference() {
    let csv = "str,int,float,bool,null\nhello,42,3.14,true,";
    let sheet = Sheet::from_csv_str(csv).unwrap();

    assert_eq!(
        sheet.get(1, 0).unwrap(),
        &CellValue::String("hello".to_string())
    );
    assert_eq!(sheet.get(1, 1).unwrap(), &CellValue::Int(42));
    assert_eq!(sheet.get(1, 2).unwrap(), &CellValue::Float(3.14));
    assert_eq!(sheet.get(1, 3).unwrap(), &CellValue::Bool(true));
    assert_eq!(sheet.get(1, 4).unwrap(), &CellValue::Null);
}

#[test]
fn test_csv_file_io() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.csv");

    let sheet = Sheet::from_data(vec![vec![1, 2, 3], vec![4, 5, 6]]);
    sheet.save_as_csv(&path).unwrap();

    let loaded = Sheet::from_csv(&path).unwrap();
    assert_eq!(loaded.row_count(), 2);
    assert_eq!(loaded.col_count(), 3);
}

#[test]
fn test_tsv() {
    let sheet = Sheet::from_data(vec![vec!["a", "b"], vec!["c", "d"]]);
    let tsv = sheet.to_tsv_string().unwrap();

    assert!(tsv.contains('\t'));
    assert!(!tsv.contains(','));
}

// ===== Book Tests =====

#[test]
fn test_book_sheet_management() {
    let mut book = Book::new();

    book.add_sheet("Sheet1", Sheet::new()).unwrap();
    book.add_sheet("Sheet2", Sheet::new()).unwrap();

    assert_eq!(book.sheet_count(), 2);
    assert!(book.has_sheet("Sheet1"));
    assert!(book.has_sheet("Sheet2"));
    assert_eq!(book.sheet_names(), vec!["Sheet1", "Sheet2"]);
}

#[test]
fn test_book_active_sheet() {
    let mut book = Book::new();

    book.add_sheet("First", Sheet::new()).unwrap();
    book.add_sheet("Second", Sheet::new()).unwrap();

    // First added is active by default
    assert_eq!(book.active_sheet().unwrap().name(), "First");

    book.set_active_sheet("Second").unwrap();
    assert_eq!(book.active_sheet().unwrap().name(), "Second");
}

#[test]
fn test_book_remove_sheet() {
    let mut book = Book::new();
    book.add_sheet("ToRemove", Sheet::new()).unwrap();
    book.add_sheet("ToKeep", Sheet::new()).unwrap();

    book.remove_sheet("ToRemove").unwrap();

    assert_eq!(book.sheet_count(), 1);
    assert!(!book.has_sheet("ToRemove"));
}

#[test]
fn test_book_rename_sheet() {
    let mut book = Book::new();
    book.add_sheet("Old", Sheet::new()).unwrap();

    book.rename_sheet("Old", "New").unwrap();

    assert!(!book.has_sheet("Old"));
    assert!(book.has_sheet("New"));
}

#[test]
fn test_book_merge() {
    let mut book1 = Book::new();
    book1.add_sheet("Sheet1", Sheet::new()).unwrap();

    let mut book2 = Book::new();
    book2.add_sheet("Sheet1", Sheet::new()).unwrap(); // Conflict
    book2.add_sheet("Sheet2", Sheet::new()).unwrap();

    book1.merge(book2);

    assert_eq!(book1.sheet_count(), 3);
    assert!(book1.has_sheet("Sheet1"));
    assert!(book1.has_sheet("Sheet1_1")); // Renamed due to conflict
    assert!(book1.has_sheet("Sheet2"));
}

#[test]
fn test_book_csv_dir() {
    let dir = tempdir().unwrap();

    // Create CSV files
    let sheet1 = Sheet::from_data(vec![vec![1, 2]]);
    let sheet2 = Sheet::from_data(vec![vec![3, 4]]);

    sheet1.save_as_csv(dir.path().join("data1.csv")).unwrap();
    sheet2.save_as_csv(dir.path().join("data2.csv")).unwrap();

    // Load as book
    let book = Book::from_csv_dir(dir.path()).unwrap();

    assert_eq!(book.sheet_count(), 2);
    assert!(book.has_sheet("data1"));
    assert!(book.has_sheet("data2"));
}

// ===== Conversion Tests =====

#[test]
fn test_to_array() {
    let sheet = Sheet::from_data(vec![vec![1, 2], vec![3, 4]]);
    let arr = sheet.to_array();

    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0].len(), 2);
}

#[test]
fn test_to_dict() {
    let mut sheet = Sheet::from_data(vec![
        vec!["A", "B"],
        vec!["1", "2"],
        vec!["3", "4"],
    ]);

    // Without naming, returns None
    assert!(sheet.to_dict().is_none());

    // With naming
    sheet.name_columns_by_row(0).unwrap();
    let dict = sheet.to_dict().unwrap();

    assert!(dict.contains_key("A"));
    assert!(dict.contains_key("B"));
    assert_eq!(dict["A"].len(), 3);
}

// ===== CellValue Tests =====

#[test]
fn test_cell_value_conversions() {
    assert_eq!(CellValue::Int(42).as_float(), Some(42.0));
    assert_eq!(CellValue::Float(3.14).as_int(), Some(3));
    assert_eq!(CellValue::Bool(true).as_int(), Some(1));
    assert_eq!(CellValue::String("42".to_string()).as_int(), Some(42));
    assert_eq!(CellValue::Null.as_int(), None);
}

#[test]
fn test_cell_value_parse() {
    assert_eq!(CellValue::parse(""), CellValue::Null);
    assert_eq!(CellValue::parse("true"), CellValue::Bool(true));
    assert_eq!(CellValue::parse("42"), CellValue::Int(42));
    assert_eq!(CellValue::parse("3.14"), CellValue::Float(3.14));
    assert_eq!(
        CellValue::parse("hello"),
        CellValue::String("hello".to_string())
    );
}
