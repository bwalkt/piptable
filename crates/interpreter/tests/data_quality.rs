//! Data quality DSL tests.

mod common {
    include!("common_impl.txt");
}

use piptable_core::Value;
use piptable_interpreter::Interpreter;
use piptable_parser::PipParser;
use piptable_sheet::{CellValue, Sheet};

#[tokio::test]
async fn test_sheet_remove_duplicates_dsl() {
    let mut sheet = Sheet::from_data(vec![
        vec!["id", "name"],
        vec!["1", "alice"],
        vec!["1", "alice2"],
        vec!["2", "bob"],
    ]);
    sheet.name_columns_by_row(0).unwrap();

    let mut interp = Interpreter::new();
    interp
        .set_var("s", Value::Sheet(Box::new(sheet)))
        .await
        .expect("set sheet");

    let program = PipParser::parse_str(r#"dim out = sheet_remove_duplicates(s, ["id"])"#).unwrap();
    interp.eval(program).await.unwrap();
    let out = interp.get_var("out").await;
    match out {
        Some(Value::Sheet(result)) => assert_eq!(result.row_count(), 3),
        _ => panic!("Expected sheet result"),
    }
}

#[tokio::test]
async fn test_sheet_validate_column_dsl() {
    let mut sheet = Sheet::from_data(vec![
        vec!["email"],
        vec!["valid@example.com"],
        vec!["not-an-email"],
    ]);
    sheet.name_columns_by_row(0).unwrap();

    let mut interp = Interpreter::new();
    interp
        .set_var("s", Value::Sheet(Box::new(sheet)))
        .await
        .expect("set sheet");

    let program =
        PipParser::parse_str(r#"dim invalid = sheet_validate_column(s, "email", "email")"#)
            .unwrap();
    interp.eval(program).await.unwrap();
    let invalid = interp.get_var("invalid").await;
    match invalid {
        Some(Value::Array(rows)) => {
            assert_eq!(rows.len(), 1);
            assert!(matches!(rows[0], Value::Int(2)));
        }
        _ => panic!("Expected array result"),
    }
}

#[tokio::test]
async fn test_sheet_clean_data_dsl() {
    let mut sheet = Sheet::from_data(vec![vec!["name"], vec!["  Alice  "], vec![""]]);
    sheet.name_columns_by_row(0).unwrap();

    let mut interp = Interpreter::new();
    interp
        .set_var("s", Value::Sheet(Box::new(sheet)))
        .await
        .expect("set sheet");

    let program = PipParser::parse_str(
        r#"dim cleaned = sheet_clean_data(s, ["trim", "lower", "empty_to_null"])"#,
    )
    .unwrap();
    interp.eval(program).await.unwrap();
    let cleaned = interp.get_var("cleaned").await;
    match cleaned {
        Some(Value::Sheet(result)) => {
            assert_eq!(
                result.get_by_name(1, "name").unwrap(),
                &CellValue::String("alice".into())
            );
            assert!(result.get_by_name(2, "name").unwrap().is_null());
        }
        _ => panic!("Expected sheet result"),
    }
}
