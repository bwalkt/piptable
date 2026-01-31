//! Formula integration tests for the DSL runtime.

mod common {
    include!("common_impl.txt");
}
use common::*;

use piptable_core::Value;
use piptable_interpreter::Interpreter;
use piptable_parser::PipParser;
use piptable_sheet::{CellValue, Sheet};

#[tokio::test]
async fn test_formula_functions_in_dsl() {
    let (interp, _) = run_script(
        r#"
        dim a = IF(1, "yes", "no")
        dim b = CONCAT("a", "b", "c")
        dim c = LEFT("hello", 2)
        dim d = RIGHT("world", 3)
        dim total = SUM(1, 2, 3)
        dim total_lower = sum(1, 2, 3)
        dim total_mixed = Sum(1, 2, 3)
    "#,
    )
    .await;

    assert!(matches!(
        interp.get_var("a").await,
        Some(Value::String(ref s)) if s == "yes"
    ));
    assert!(matches!(
        interp.get_var("b").await,
        Some(Value::String(ref s)) if s == "abc"
    ));
    assert!(matches!(
        interp.get_var("c").await,
        Some(Value::String(ref s)) if s == "he"
    ));
    assert!(matches!(
        interp.get_var("d").await,
        Some(Value::String(ref s)) if s == "rld"
    ));
    assert!(matches!(
        interp.get_var("total").await,
        Some(Value::Float(f)) if (f - 6.0).abs() < 1e-9
    ));
    assert!(matches!(
        interp.get_var("total_lower").await,
        Some(Value::Float(f)) if (f - 6.0).abs() < 1e-9
    ));
    assert!(matches!(
        interp.get_var("total_mixed").await,
        Some(Value::Float(f)) if (f - 6.0).abs() < 1e-9
    ));
}

#[tokio::test]
async fn test_sheet_formula_eval_helpers() {
    let mut interp = Interpreter::new();
    let sheet = Sheet::from_data(vec![
        vec![
            CellValue::Int(1),
            CellValue::String("=SUM(A1:A2)".to_string()),
        ],
        vec![CellValue::Int(2), CellValue::Null],
    ]);
    interp
        .set_var("s", Value::Sheet(sheet))
        .await
        .expect("set sheet");

    let script = r#"
        dim total = sheet_get_cell_value(s, "B1")
        dim direct = sheet_eval_formula(s, "SUM(A1:A2)")
        dim direct_short = sum(s, "A1:A2")
        dim avg_short = avg(s, "A1:A2")
        dim min_short = min(s, "A1:A2")
        dim max_short = max(s, "A1:A2")
        dim count_short = count(s, "A1:A2")
        dim counta_short = counta(s, "A1:A2")
        dim is_formula = is_sheet_cell_formula(s, "B1")
        dim is_not_formula = is_sheet_cell_formula(s, "A1")
    "#;
    let program = PipParser::parse_str(script).expect("parse script");
    interp.eval(program).await.expect("eval script");

    assert!(matches!(
        interp.get_var("total").await,
        Some(Value::Float(f)) if (f - 3.0).abs() < 1e-9
    ));
    assert!(matches!(
        interp.get_var("direct").await,
        Some(Value::Float(f)) if (f - 3.0).abs() < 1e-9
    ));
    assert!(matches!(
        interp.get_var("direct_short").await,
        Some(Value::Float(f)) if (f - 3.0).abs() < 1e-9
    ));
    assert!(matches!(
        interp.get_var("avg_short").await,
        Some(Value::Float(f)) if (f - 1.5).abs() < 1e-9
    ));
    assert!(matches!(
        interp.get_var("min_short").await,
        Some(Value::Float(f)) if (f - 1.0).abs() < 1e-9
    ));
    assert!(matches!(
        interp.get_var("max_short").await,
        Some(Value::Float(f)) if (f - 2.0).abs() < 1e-9
    ));
    assert!(matches!(
        interp.get_var("count_short").await,
        Some(Value::Int(2))
    ));
    assert!(matches!(
        interp.get_var("counta_short").await,
        Some(Value::Int(2))
    ));
    assert!(matches!(
        interp.get_var("is_formula").await,
        Some(Value::Bool(true))
    ));
    assert!(matches!(
        interp.get_var("is_not_formula").await,
        Some(Value::Bool(false))
    ));
}
