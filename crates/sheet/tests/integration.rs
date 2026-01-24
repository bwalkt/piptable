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
    let mut sheet = Sheet::from_data(vec![vec!["A", "B", "C"], vec!["1", "2", "3"]]);

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
    assert!(matches!(result, Err(SheetError::ColumnsNotNamed(_))));
}

#[test]
fn test_duplicate_column_names_error() {
    let mut sheet = Sheet::from_data(vec![
        vec!["A", "B", "A"], // Duplicate "A"
        vec!["1", "2", "3"],
    ]);

    let result = sheet.name_columns_by_row(0);
    assert!(matches!(
        result,
        Err(SheetError::DuplicateColumnName { .. })
    ));
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
        .column_map(0, |cell| CellValue::Int(cell.as_int().unwrap_or(0) * 2))
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
    let original = Sheet::from_data(vec![vec!["name", "value"], vec!["test", "42"]]);

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
    let mut sheet = Sheet::from_data(vec![vec!["A", "B"], vec!["1", "2"], vec!["3", "4"]]);

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

// ===== Join Operations Tests =====

fn create_employees_sheet() -> Sheet {
    let mut sheet = Sheet::from_data(vec![
        vec!["name", "salary"],
        vec!["joe", "100"],
        vec!["alice", "150"],
    ]);
    sheet.name_columns_by_row(0).unwrap();
    sheet
}

fn create_titles_sheet() -> Sheet {
    let mut sheet = Sheet::from_data(vec![
        vec!["name", "title"],
        vec!["joe", "developer"],
        vec!["bob", "analyst"],
    ]);
    sheet.name_columns_by_row(0).unwrap();
    sheet
}

#[test]
fn test_inner_join() {
    let employees = create_employees_sheet();
    let titles = create_titles_sheet();

    let result = employees.inner_join(&titles, "name").unwrap();

    // Only joe matches in both
    assert_eq!(result.row_count(), 2); // header + 1 data row
    assert_eq!(result.col_count(), 3); // name, salary, title

    // Check the joined row
    assert_eq!(
        result.get_by_name(1, "name").unwrap(),
        &CellValue::String("joe".to_string())
    );
    assert_eq!(
        result.get_by_name(1, "salary").unwrap(),
        &CellValue::String("100".to_string())
    );
    assert_eq!(
        result.get_by_name(1, "title").unwrap(),
        &CellValue::String("developer".to_string())
    );
}

#[test]
fn test_left_join() {
    let employees = create_employees_sheet();
    let titles = create_titles_sheet();

    let result = employees.left_join(&titles, "name").unwrap();

    // Both employees included
    assert_eq!(result.row_count(), 3); // header + 2 data rows

    // joe has title
    assert_eq!(
        result.get_by_name(1, "title").unwrap(),
        &CellValue::String("developer".to_string())
    );

    // alice has null title (no match)
    assert_eq!(result.get_by_name(2, "title").unwrap(), &CellValue::Null);
}

#[test]
fn test_right_join() {
    let employees = create_employees_sheet();
    let titles = create_titles_sheet();

    let result = employees.right_join(&titles, "name").unwrap();

    // Both title holders included
    assert_eq!(result.row_count(), 3); // header + 2 data rows

    // joe has salary
    // bob has null salary (no match in employees)
    let has_null_salary = (1..result.row_count())
        .any(|i| result.get_by_name(i, "salary").unwrap() == &CellValue::Null);
    assert!(has_null_salary);
}

#[test]
fn test_full_join() {
    let employees = create_employees_sheet();
    let titles = create_titles_sheet();

    let result = employees.full_join(&titles, "name").unwrap();

    // joe, alice, bob all included
    assert_eq!(result.row_count(), 4); // header + 3 data rows
}

#[test]
fn test_join_key_not_found() {
    let employees = create_employees_sheet();
    let titles = create_titles_sheet();

    let result = employees.inner_join(&titles, "nonexistent");
    assert!(result.is_err());
}

#[test]
fn test_join_columns_not_named() {
    let sheet1 = Sheet::from_data(vec![vec![1, 2], vec![3, 4]]);
    let sheet2 = Sheet::from_data(vec![vec![5, 6], vec![7, 8]]);

    let result = sheet1.inner_join(&sheet2, "key");
    assert!(result.is_err());
}

#[test]
fn test_join_one_to_many() {
    // Test duplicate keys producing cartesian product
    let mut orders = Sheet::from_data(vec![
        vec!["customer", "order_id"],
        vec!["alice", "1"],
        vec!["alice", "2"],
        vec!["bob", "3"],
    ]);
    orders.name_columns_by_row(0).unwrap();

    let mut customers = Sheet::from_data(vec![
        vec!["customer", "city"],
        vec!["alice", "NYC"],
        vec!["bob", "LA"],
    ]);
    customers.name_columns_by_row(0).unwrap();

    let result = orders.inner_join(&customers, "customer").unwrap();

    // alice has 2 orders, bob has 1 = 3 data rows + header
    assert_eq!(result.row_count(), 4);
}

#[test]
fn test_join_with_different_key_names() {
    let mut employees = Sheet::from_data(vec![vec!["emp_name", "salary"], vec!["joe", "100"]]);
    employees.name_columns_by_row(0).unwrap();

    let mut titles = Sheet::from_data(vec![vec!["person", "title"], vec!["joe", "developer"]]);
    titles.name_columns_by_row(0).unwrap();

    let result = employees
        .inner_join_on(&titles, "emp_name", "person")
        .unwrap();

    assert_eq!(result.row_count(), 2); // header + 1 data row
    assert_eq!(
        result.get_by_name(1, "title").unwrap(),
        &CellValue::String("developer".to_string())
    );
}

#[test]
fn test_join_empty_left_sheet() {
    let mut empty = Sheet::from_data(vec![vec!["name", "value"]]);
    empty.name_columns_by_row(0).unwrap();

    let titles = create_titles_sheet();

    let result = empty.inner_join(&titles, "name").unwrap();
    assert_eq!(result.row_count(), 1); // header only, no data rows
}

#[test]
fn test_join_empty_right_sheet() {
    let employees = create_employees_sheet();

    let mut empty = Sheet::from_data(vec![vec!["name", "title"]]);
    empty.name_columns_by_row(0).unwrap();

    let result = employees.inner_join(&empty, "name").unwrap();
    assert_eq!(result.row_count(), 1); // header only, no data rows
}

#[test]
fn test_left_join_empty_right() {
    let employees = create_employees_sheet();

    let mut empty = Sheet::from_data(vec![vec!["name", "title"]]);
    empty.name_columns_by_row(0).unwrap();

    let result = employees.left_join(&empty, "name").unwrap();
    // All left rows preserved with null right values
    assert_eq!(result.row_count(), 3); // header + 2 employees
}

#[test]
fn test_join_data_value_matches_column_name() {
    // Edge case: first data row has a value that matches a column name
    // The stricter header detection should NOT skip this row
    let mut left = Sheet::from_data(vec![
        vec!["id", "name"],
        vec!["name", "alice"], // "name" value matches column name
        vec!["2", "bob"],
    ]);
    left.name_columns_by_row(0).unwrap();

    let mut right = Sheet::from_data(vec![
        vec!["id", "title"],
        vec!["name", "manager"], // "name" value matches column name
        vec!["2", "developer"],
    ]);
    right.name_columns_by_row(0).unwrap();

    let result = left.inner_join(&right, "id").unwrap();

    // Header + 2 data rows (not 1 - both rows should be included)
    assert_eq!(result.row_count(), 3);
    assert_eq!(
        result.get_by_name(1, "name").unwrap(),
        &CellValue::String("alice".to_string())
    );
    assert_eq!(
        result.get_by_name(1, "title").unwrap(),
        &CellValue::String("manager".to_string())
    );
}

// ===== Append/Upsert Operations Tests =====

#[test]
fn test_append_basic() {
    let mut sheet1 = Sheet::from_data(vec![vec![1, 2], vec![3, 4]]);
    let sheet2 = Sheet::from_data(vec![vec![5, 6], vec![7, 8]]);

    sheet1.append(&sheet2).unwrap();

    assert_eq!(sheet1.row_count(), 4);
    assert_eq!(sheet1.get(2, 0).unwrap(), &CellValue::Int(5));
    assert_eq!(sheet1.get(3, 1).unwrap(), &CellValue::Int(8));
}

#[test]
fn test_append_with_named_columns() {
    let mut sheet1 = Sheet::from_data(vec![vec!["id", "name"], vec!["1", "alice"]]);
    sheet1.name_columns_by_row(0).unwrap();

    let mut sheet2 = Sheet::from_data(vec![vec!["id", "name"], vec!["2", "bob"]]);
    sheet2.name_columns_by_row(0).unwrap();

    sheet1.append(&sheet2).unwrap();

    // Header + alice + bob
    assert_eq!(sheet1.row_count(), 3);
}

#[test]
fn test_append_column_mismatch_error() {
    let mut sheet1 = Sheet::from_data(vec![vec![1, 2, 3]]);
    let sheet2 = Sheet::from_data(vec![vec![4, 5]]);

    let result = sheet1.append(&sheet2);
    assert!(result.is_err());
}

#[test]
fn test_append_distinct() {
    let mut sheet1 = Sheet::from_data(vec![
        vec!["id", "name"],
        vec!["1", "alice"],
        vec!["2", "bob"],
    ]);
    sheet1.name_columns_by_row(0).unwrap();

    let mut sheet2 = Sheet::from_data(vec![
        vec!["id", "name"],
        vec!["2", "bob_updated"], // duplicate id=2
        vec!["3", "charlie"],     // new
    ]);
    sheet2.name_columns_by_row(0).unwrap();

    sheet1.append_distinct(&sheet2, "id").unwrap();

    // Header + alice + bob + charlie (bob_updated skipped)
    assert_eq!(sheet1.row_count(), 4);

    // Original bob unchanged
    assert_eq!(
        sheet1.get_by_name(2, "name").unwrap(),
        &CellValue::String("bob".to_string())
    );
}

#[test]
fn test_append_distinct_duplicates_in_other() {
    let mut sheet1 = Sheet::from_data(vec![vec!["id", "name"], vec!["1", "alice"]]);
    sheet1.name_columns_by_row(0).unwrap();

    // other has duplicate keys (two rows with id=2)
    let mut sheet2 = Sheet::from_data(vec![
        vec!["id", "name"],
        vec!["2", "bob"],
        vec!["2", "bob_duplicate"], // same key, should be skipped
        vec!["3", "charlie"],
    ]);
    sheet2.name_columns_by_row(0).unwrap();

    sheet1.append_distinct(&sheet2, "id").unwrap();

    // Header + alice + bob + charlie = 4 (bob_duplicate skipped)
    assert_eq!(sheet1.row_count(), 4);

    // First bob was added
    assert_eq!(
        sheet1.get_by_name(2, "name").unwrap(),
        &CellValue::String("bob".to_string())
    );
}

#[test]
fn test_upsert() {
    let mut sheet1 = Sheet::from_data(vec![
        vec!["id", "name", "salary"],
        vec!["1", "joe", "100"],
        vec!["2", "alice", "150"],
    ]);
    sheet1.name_columns_by_row(0).unwrap();

    let mut sheet2 = Sheet::from_data(vec![
        vec!["id", "name", "salary"],
        vec!["2", "alice", "200"], // update alice's salary
        vec!["3", "bob", "120"],   // insert bob
    ]);
    sheet2.name_columns_by_row(0).unwrap();

    sheet1.upsert(&sheet2, "id").unwrap();

    // Header + joe + alice (updated) + bob (inserted)
    assert_eq!(sheet1.row_count(), 4);

    // Alice's salary updated
    assert_eq!(
        sheet1.get_by_name(2, "salary").unwrap(),
        &CellValue::String("200".to_string())
    );

    // Bob inserted
    assert_eq!(
        sheet1.get_by_name(3, "name").unwrap(),
        &CellValue::String("bob".to_string())
    );
}

#[test]
fn test_upsert_key_not_found() {
    let mut sheet1 = Sheet::from_data(vec![vec!["id", "name"], vec!["1", "alice"]]);
    sheet1.name_columns_by_row(0).unwrap();

    let mut sheet2 = Sheet::from_data(vec![vec!["other_id", "name"], vec!["2", "bob"]]);
    sheet2.name_columns_by_row(0).unwrap();

    let result = sheet1.upsert(&sheet2, "id");
    assert!(result.is_err());
}

// ===== Parquet Tests =====

#[test]
fn test_parquet_file_io() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.parquet");

    let mut sheet = Sheet::from_data(vec![
        vec!["id", "name", "score"],
        vec!["1", "Alice", "95.5"],
        vec!["2", "Bob", "87.3"],
    ]);
    sheet.name_columns_by_row(0).unwrap();

    sheet.save_as_parquet(&file_path).unwrap();

    let loaded = Sheet::from_parquet(&file_path).unwrap();
    assert_eq!(loaded.row_count(), sheet.row_count());
    assert_eq!(loaded.col_count(), sheet.col_count());
    assert!(loaded.column_names().is_some());
}

#[test]
fn test_parquet_typed_data() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("typed.parquet");

    // Create sheet with typed data
    let mut sheet = Sheet::new();
    sheet.data_mut().push(vec![
        CellValue::String("id".to_string()),
        CellValue::String("score".to_string()),
        CellValue::String("active".to_string()),
    ]);
    sheet.data_mut().push(vec![
        CellValue::Int(1),
        CellValue::Float(95.5),
        CellValue::Bool(true),
    ]);
    sheet.data_mut().push(vec![
        CellValue::Int(2),
        CellValue::Float(87.3),
        CellValue::Bool(false),
    ]);
    sheet.name_columns_by_row(0).unwrap();

    sheet.save_as_parquet(&file_path).unwrap();

    let loaded = Sheet::from_parquet(&file_path).unwrap();

    // Types should be preserved
    assert!(matches!(loaded.get(1, 0).unwrap(), CellValue::Int(1)));
    assert!(matches!(loaded.get(1, 2).unwrap(), CellValue::Bool(true)));
}

#[test]
fn test_parquet_roundtrip_with_nulls() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("nulls.parquet");

    let mut sheet = Sheet::new();
    sheet.data_mut().push(vec![
        CellValue::String("a".to_string()),
        CellValue::String("b".to_string()),
    ]);
    sheet
        .data_mut()
        .push(vec![CellValue::Int(1), CellValue::Null]);
    sheet.data_mut().push(vec![
        CellValue::Null,
        CellValue::String("hello".to_string()),
    ]);
    sheet.name_columns_by_row(0).unwrap();

    sheet.save_as_parquet(&file_path).unwrap();
    let loaded = Sheet::from_parquet(&file_path).unwrap();

    // Nulls should be preserved
    assert!(loaded.get(1, 1).unwrap().is_null());
    assert!(loaded.get(2, 0).unwrap().is_null());
}
