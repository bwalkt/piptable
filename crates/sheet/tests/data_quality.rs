use piptable_sheet::{CleanOptions, NullStrategy, Sheet, ValidationRule};

#[test]
fn test_remove_duplicates_by_column() {
    let mut sheet = Sheet::from_data(vec![
        vec!["id", "email"],
        vec!["1", "alice@example.com"],
        vec!["1", "alice2@example.com"],
        vec!["2", "bob@example.com"],
    ]);
    sheet.name_columns_by_row(0).unwrap();

    let removed = sheet.remove_duplicates_by_columns(&["id"]).unwrap();
    assert_eq!(removed, 1);
    assert_eq!(sheet.row_count(), 3);
    assert_eq!(sheet.get_by_name(1, "id").unwrap().as_str(), "1");
    assert_eq!(sheet.get_by_name(2, "id").unwrap().as_str(), "2");
}

#[test]
fn test_validate_column_email() {
    let mut sheet = Sheet::from_data(vec![
        vec!["email"],
        vec!["valid@example.com"],
        vec!["not-an-email"],
    ]);
    sheet.name_columns_by_row(0).unwrap();

    let invalid = sheet
        .validate_column("email", ValidationRule::Email)
        .unwrap();
    assert_eq!(invalid, vec![2]);
}

#[test]
fn test_validate_column_range() {
    let mut sheet = Sheet::from_data(vec![
        vec!["age"],
        vec!["30"],
        vec!["10"],
        vec!["200"],
        vec![""],
    ]);
    sheet.name_columns_by_row(0).unwrap();

    let invalid = sheet
        .validate_column(
            "age",
            ValidationRule::Range {
                min: 18.0,
                max: 65.0,
            },
        )
        .unwrap();
    assert_eq!(invalid, vec![2, 3, 4]);
}

#[test]
fn test_clean_data_trim_lower_empty_to_null() {
    let mut sheet = Sheet::from_data(vec![vec!["name"], vec!["  Alice  "], vec![""]]);
    sheet.name_columns_by_row(0).unwrap();

    let mut options = CleanOptions::default();
    options.trim = true;
    options.lower = true;
    options.null_strategy = NullStrategy::EmptyToNull;

    sheet.clean_data(&options).unwrap();
    assert_eq!(sheet.get_by_name(1, "name").unwrap().as_str(), "alice");
    assert!(sheet.get_by_name(2, "name").unwrap().is_null());
}
