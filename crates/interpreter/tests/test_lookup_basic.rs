//! Basic tests for lookup functions

mod common {
    include!("common_impl.txt");
}
use common::*;

use piptable_core::Value;

#[tokio::test]
async fn test_vlookup_basic() {
    let (interp, _) = run_script(
        r#"
        products = [["A", 10], ["B", 20], ["C", 30]]
        result = vlookup("B", products, 2, false)
    "#,
    )
    .await;
    assert!(matches!(interp.get_var("result").await, Some(Value::Int(20))));
}

#[tokio::test]
async fn test_index_basic() {
    let (interp, _) = run_script(
        r#"
        data = [[1, 2], [3, 4]]
        result = index(data, 2, 2)
    "#,
    )
    .await;
    assert!(matches!(interp.get_var("result").await, Some(Value::Int(4))));
}

#[tokio::test]
async fn test_match_basic() {
    let (interp, _) = run_script(
        r#"
        arr = ["A", "B", "C"]
        result = match("B", arr, 0)
    "#,
    )
    .await;
    assert!(matches!(interp.get_var("result").await, Some(Value::Int(2))));
}