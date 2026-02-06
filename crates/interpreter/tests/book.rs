//! Book-related tests for the PipTable interpreter.

#![allow(clippy::needless_raw_string_hashes)]

/// Shared test helpers.
mod common {
    include!("common_impl.txt");
}
use common::*;

use piptable_core::Value;

#[tokio::test]
async fn test_book_from_dict_and_names() {
    let script = r#"
        dim book = book_from_dict({
            "Sheet1": [["A", "B"], [1, 2]],
            "Sheet2": [["C", "D"], [3, 4]]
        })
        dim names = book_sheet_names(book)
        dim count = book_sheet_count(book)
    "#;

    let (interp, _) = run_script(script).await;

    match interp.get_var("names").await {
        Some(Value::Array(items)) => assert_eq!(items.len(), 2),
        _ => panic!("Expected names array"),
    }

    assert!(matches!(interp.get_var("count").await, Some(Value::Int(2))));
}

#[tokio::test]
async fn test_book_get_sheet() {
    let script = r#"
        dim book = book_from_dict({
            "Sheet1": [["A"], [1]]
        })
        dim s1 = book_get_sheet(book, "Sheet1")
        dim s0 = book_get_sheet_by_index(book, 0)
    "#;

    let (interp, _) = run_script(script).await;

    assert!(matches!(interp.get_var("s1").await, Some(Value::Sheet(_))));
    assert!(matches!(interp.get_var("s0").await, Some(Value::Sheet(_))));
}

#[tokio::test]
async fn test_book_merge_operator() {
    let script = r#"
        dim a = book_from_dict({
            "A": [["x"], [1]]
        })
        dim b = book_from_dict({
            "B": [["y"], [2]]
        })
        dim c = a + b
        dim count = book_sheet_count(c)
    "#;

    let (interp, _) = run_script(script).await;
    assert!(matches!(interp.get_var("count").await, Some(Value::Int(2))));
}

#[tokio::test]
async fn test_book_add_empty_sheet() {
    let script = r#"
        dim book = book_from_dict({})
        book = book_add_empty_sheet(book, "Empty")
        dim count = book_sheet_count(book)
    "#;

    let (interp, _) = run_script(script).await;
    assert!(matches!(interp.get_var("count").await, Some(Value::Int(1))));
}

#[tokio::test]
async fn test_book_consolidate() {
    let script = r#"
        dim book = book_from_dict({
            "One": [["id", "val"], [1, "a"]],
            "Two": [["id", "val"], [2, "b"]]
        })

        dim one = book_get_sheet(book, "One")
        one = sheet_name_columns_by_row(one, 0)
        dim two = book_get_sheet(book, "Two")
        two = sheet_name_columns_by_row(two, 0)

        dim normalized = book_from_dict({})
        normalized = book_add_sheet(normalized, "One", one)
        normalized = book_add_sheet(normalized, "Two", two)

        dim combined = book_consolidate(normalized)
        dim rows = sheet_row_count(combined)
    "#;

    let (interp, _) = run_script(script).await;
    assert!(matches!(interp.get_var("rows").await, Some(Value::Int(3))));
}

#[tokio::test]
async fn test_book_for_each_sheet() {
    let script = r#"
        dim book = book_from_dict({
            "A": [["col"], [1]],
            "B": [["col"], [2], [3]]
        })
        dim f = sheet => sheet.row_count()
        dim counts = book.for_each_sheet(f)
    "#;

    let (interp, _) = run_script(script).await;
    match interp.get_var("counts").await {
        Some(Value::Array(items)) => assert_eq!(items.len(), 2),
        _ => panic!("Expected counts array"),
    }
}

#[tokio::test]
async fn test_book_for_each_sheet_mut() {
    let script = r#"
        dim book = book_from_dict({
            "A": [["name"], [" Alice "]],
            "B": [["name"], [" Bob "]]
        })
        dim f = sheet => sheet_map(sheet, "trim")
        dim cleaned = book.for_each_sheet_mut(f)
        dim count = book_sheet_count(cleaned)
    "#;

    let (interp, _) = run_script(script).await;
    assert!(matches!(interp.get_var("count").await, Some(Value::Int(2))));
}

#[tokio::test]
async fn test_book_get_sheet_by_index_negative() {
    let script = r#"
        dim book = book_from_dict({
            "A": [["col"], [1]],
            "B": [["col"], [2]]
        })
        dim last = book_get_sheet_by_index(book, -1)
    "#;

    let (interp, _) = run_script(script).await;
    assert!(matches!(interp.get_var("last").await, Some(Value::Sheet(_))));
}

#[tokio::test]
async fn test_book_get_sheet_by_index_out_of_bounds() {
    let script = r#"
        dim book = book_from_dict({
            "A": [["col"], [1]]
        })
        dim s = book_get_sheet_by_index(book, 5)
    "#;
    let err = run_script_err(script).await;
    assert!(err.contains("index"));
}

#[tokio::test]
async fn test_book_add_sheet_invalid_data() {
    let script = r#"
        dim book = book_from_dict({})
        book = book_add_sheet(book, "Bad", [1, 2, 3])
    "#;
    let err = run_script_err(script).await;
    assert!(err.contains("Invalid sheet data"));
}

#[tokio::test]
async fn test_book_from_dict_invalid_value() {
    let script = r#"
        dim book = book_from_dict({
            "Bad": 123
        })
    "#;
    let err = run_script_err(script).await;
    assert!(err.contains("Invalid sheet data"));
}
