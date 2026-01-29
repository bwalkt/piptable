//! Integration tests for the FILTER function

mod common {
    include!("common_impl.txt");
}
use common::*;

use piptable_core::Value;

#[tokio::test]
async fn test_filter_basic_dsl() {
    let (interp, _) = run_script(
        r#"
        dim data = [10, 20, 30, 40, 50]
        dim criteria = [true, false, true, false, true]
        dim result = filter(data, criteria)
    "#,
    )
    .await;

    match interp.get_var("result").await {
        Some(Value::Array(arr)) => {
            assert_eq!(arr.len(), 3);
            assert!(matches!(arr[0], Value::Int(10)));
            assert!(matches!(arr[1], Value::Int(30)));
            assert!(matches!(arr[2], Value::Int(50)));
        }
        _ => panic!("Expected array result"),
    }
}

#[tokio::test]
async fn test_filter_2d_array_dsl() {
    let (interp, _) = run_script(
        r#"
        dim sales = [
            ["Alice", 100],
            ["Bob", 50],
            ["Charlie", 150],
            ["David", 75]
        ]
        dim high_sales = [true, false, true, false]
        dim result = filter(sales, high_sales)
    "#,
    )
    .await;

    match interp.get_var("result").await {
        Some(Value::Array(arr)) => {
            assert_eq!(arr.len(), 2);
            // Check first row
            match &arr[0] {
                Value::Array(row) => {
                    match &row[0] {
                        Value::String(s) => assert_eq!(s, "Alice"),
                        _ => panic!("Expected string"),
                    }
                    assert!(matches!(row[1], Value::Int(100)));
                }
                _ => panic!("Expected array row"),
            }
            // Check second row
            match &arr[1] {
                Value::Array(row) => {
                    match &row[0] {
                        Value::String(s) => assert_eq!(s, "Charlie"),
                        _ => panic!("Expected string"),
                    }
                    assert!(matches!(row[1], Value::Int(150)));
                }
                _ => panic!("Expected array row"),
            }
        }
        _ => panic!("Expected array result"),
    }
}

#[tokio::test]
async fn test_filter_empty_result_dsl() {
    let (interp, _) = run_script(
        r#"
        dim data = [1, 2, 3]
        dim criteria = [false, false, false]
        dim result = filter(data, criteria, "No matches")
    "#,
    )
    .await;

    match interp.get_var("result").await {
        Some(Value::String(s)) => assert_eq!(s, "No matches"),
        _ => panic!("Expected string result"),
    }
}

#[tokio::test]
async fn test_filter_with_numeric_criteria() {
    let (interp, _) = run_script(
        r#"
        dim names = ["Alice", "Bob", "Charlie", "David"]
        dim scores = [85, 0, 92, 67]
        dim result = filter(names, scores)  ' Non-zero values are truthy
    "#,
    )
    .await;

    match interp.get_var("result").await {
        Some(Value::Array(arr)) => {
            assert_eq!(arr.len(), 3);  // Only 0 is filtered out
            match &arr[0] {
                Value::String(s) => assert_eq!(s, "Alice"),
                _ => panic!("Expected string"),
            }
            match &arr[1] {
                Value::String(s) => assert_eq!(s, "Charlie"),
                _ => panic!("Expected string"),
            }
            match &arr[2] {
                Value::String(s) => assert_eq!(s, "David"),
                _ => panic!("Expected string"),
            }
        }
        _ => panic!("Expected array result"),
    }
}

#[tokio::test]
async fn test_filter_chained_operations() {
    let (interp, _) = run_script(
        r#"
        dim data = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
        dim criteria1 = [true, false, true, false, true, false, true, false, true, false]
        dim intermediate = filter(data, criteria1)
        ' intermediate should be [1, 3, 5, 7, 9]
        
        dim criteria2 = [false, true, false, true, false]
        dim final_result = filter(intermediate, criteria2)
        ' final should be [3, 7]
    "#,
    )
    .await;

    match interp.get_var("final_result").await {
        Some(Value::Array(arr)) => {
            assert_eq!(arr.len(), 2);
            assert!(matches!(arr[0], Value::Int(3)));
            assert!(matches!(arr[1], Value::Int(7)));
        }
        _ => panic!("Expected array result"),
    }
}
