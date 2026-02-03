//! Named row/column assignment tests for the DSL runtime.

use piptable_core::Value;
use piptable_interpreter::Interpreter;
use piptable_parser::PipParser;
use piptable_sheet::{CellValue, Sheet};

#[tokio::test]
async fn test_sheet_set_column_by_name() {
    let mut sheet = Sheet::from_data(vec![
        vec!["id", "salary"],
        vec!["E1", "100"],
        vec!["E2", "120"],
    ]);
    sheet.name_columns_by_row(0).unwrap();

    let mut interp = Interpreter::new();
    interp
        .set_var("s", Value::Sheet(Box::new(sheet)))
        .await
        .expect("set sheet");

    let program = PipParser::parse_str(
        r#"
        dim updated = sheet_set_column_by_name(s, "salary", ["salary", 110, 130])
    "#,
    )
    .unwrap();
    interp.eval(program).await.unwrap();
    let updated = interp.get_var("updated").await;

    match updated {
        Some(Value::Sheet(result)) => {
            assert_eq!(
                result.get_by_name(1, "salary").unwrap(),
                &CellValue::Int(110)
            );
            assert_eq!(
                result.get_by_name(2, "salary").unwrap(),
                &CellValue::Int(130)
            );
        }
        _ => panic!("Expected sheet result"),
    }
}

#[tokio::test]
async fn test_sheet_set_row_by_name() {
    let mut sheet = Sheet::from_data(vec![
        vec!["EMP001", "Alice", "30"],
        vec!["EMP002", "Bob", "25"],
    ]);
    sheet.name_rows_by_column(0).unwrap();

    let mut interp = Interpreter::new();
    interp
        .set_var("s", Value::Sheet(Box::new(sheet)))
        .await
        .expect("set sheet");

    let program = PipParser::parse_str(
        r#"
        dim updated = sheet_set_row_by_name(s, "EMP002", ["EMP002", "Bobby", 26])
    "#,
    )
    .unwrap();
    interp.eval(program).await.unwrap();
    let updated = interp.get_var("updated").await;

    match updated {
        Some(Value::Sheet(result)) => {
            let row = result.row_by_name("EMP002").unwrap();
            assert_eq!(row[1], CellValue::String("Bobby".to_string()));
            assert_eq!(row[2], CellValue::Int(26));
        }
        _ => panic!("Expected sheet result"),
    }
}
