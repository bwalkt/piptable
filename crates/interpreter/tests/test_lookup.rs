//! Tests for lookup functions (VLOOKUP, HLOOKUP, INDEX, MATCH, XLOOKUP)

mod common {
    include!("common_impl.txt");
}
use common::*;

use piptable_core::Value;

#[tokio::test]
async fn test_vlookup_exact_match() {
    let (interp, _) = run_script(
        r#"
        products = [
            ["Apple", 1.50, 100],
            ["Banana", 0.75, 200],
            ["Cherry", 2.00, 150],
            ["Date", 3.50, 50]
        ]
        price = vlookup("Banana", products, 2, false)
        quantity = vlookup("Cherry", products, 3, false)
        not_found = vlookup("Grape", products, 2, false)
    "#,
    )
    .await;

    assert!(matches!(
        interp.get_var("price").await,
        Some(Value::Float(f)) if (f - 0.75).abs() < 0.001
    ));

    assert!(matches!(
        interp.get_var("quantity").await,
        Some(Value::Int(150))
    ));

    assert!(matches!(
        interp.get_var("not_found").await,
        Some(Value::String(s)) if s == "#N/A"
    ));
}

#[tokio::test]
async fn test_hlookup_exact_match() {
    let (interp, _) = run_script(
        r#"
        quarterly = [
            ["Product", "Q1", "Q2", "Q3", "Q4"],
            ["Sales", 100, 150, 120, 180],
            ["Costs", 80, 100, 90, 120]
        ]
        q2_sales = hlookup("Q2", quarterly, 2, false)
        q4_costs = hlookup("Q4", quarterly, 3, false)
        not_found = hlookup("Q5", quarterly, 2, false)
    "#,
    )
    .await;

    assert!(matches!(
        interp.get_var("q2_sales").await,
        Some(Value::Int(150))
    ));

    assert!(matches!(
        interp.get_var("q4_costs").await,
        Some(Value::Int(120))
    ));

    assert!(matches!(
        interp.get_var("not_found").await,
        Some(Value::String(s)) if s == "#N/A"
    ));
}

#[tokio::test]
async fn test_hlookup_approximate_match() {
    let (interp, _) = run_script(
        r#"
        salary_table = [
            [0, 30000, 40000, 50000, 60000, 70000],
            [10000, 35000, 45000, 55000, 65000, 75000],
            [1000, 2000, 3000, 4000, 5000, 6000]
        ]
        salary1 = hlookup(35000, salary_table, 2, true)
        salary2 = hlookup(45000, salary_table, 2, true)
        bonus1 = hlookup(55000, salary_table, 3, true)
        bonus2 = hlookup(65000, salary_table, 3, true)
    "#,
    )
    .await;

    assert!(matches!(
        interp.get_var("salary1").await,
        Some(Value::Int(35000))
    ));

    assert!(matches!(
        interp.get_var("salary2").await,
        Some(Value::Int(45000))
    ));

    assert!(matches!(
        interp.get_var("bonus1").await,
        Some(Value::Int(4000))
    ));

    assert!(matches!(
        interp.get_var("bonus2").await,
        Some(Value::Int(5000))
    ));
}

#[tokio::test]
async fn test_index_function() {
    let (interp, _) = run_script(
        r#"
        data = [
            [10, 20, 30],
            [40, 50, 60],
            [70, 80, 90]
        ]
        val1 = index(data, 2, 3)
        val2 = index(data, 3, 1)
        row2 = index(data, 2)
    "#,
    )
    .await;

    assert!(matches!(interp.get_var("val1").await, Some(Value::Int(60))));

    assert!(matches!(interp.get_var("val2").await, Some(Value::Int(70))));

    if let Some(Value::Array(row)) = interp.get_var("row2").await {
        assert_eq!(row.len(), 3);
        assert!(matches!(&row[0], Value::Int(40)));
        assert!(matches!(&row[1], Value::Int(50)));
        assert!(matches!(&row[2], Value::Int(60)));
    } else {
        panic!("Expected row2 to be an array");
    }
}

#[tokio::test]
async fn test_match_exact() {
    let (interp, _) = run_script(
        r#"
        fruits = ["Apple", "Banana", "Cherry", "Date"]
        numbers = [10, 20, 30, 40, 50]
        pos1 = match("Banana", fruits, 0)
        pos2 = match("Date", fruits, 0)
        pos3 = match(30, numbers, 0)
        not_found = match("Grape", fruits, 0)
    "#,
    )
    .await;

    assert!(matches!(interp.get_var("pos1").await, Some(Value::Int(2))));

    assert!(matches!(interp.get_var("pos2").await, Some(Value::Int(4))));

    assert!(matches!(interp.get_var("pos3").await, Some(Value::Int(3))));

    assert!(matches!(
        interp.get_var("not_found").await,
        Some(Value::String(s)) if s == "#N/A"
    ));
}

#[tokio::test]
async fn test_match_less_than_or_equal() {
    let (interp, _) = run_script(
        r#"
        sorted_nums = [10, 20, 30, 40, 50]
        pos1 = match(25, sorted_nums, 1)
        pos2 = match(30, sorted_nums, 1)
        pos3 = match(55, sorted_nums, 1)
        pos4 = match(5, sorted_nums, 1)
    "#,
    )
    .await;

    assert!(matches!(interp.get_var("pos1").await, Some(Value::Int(2))));

    assert!(matches!(interp.get_var("pos2").await, Some(Value::Int(3))));

    assert!(matches!(interp.get_var("pos3").await, Some(Value::Int(5))));

    assert!(matches!(
        interp.get_var("pos4").await,
        Some(Value::String(s)) if s == "#N/A"
    ));
}

#[tokio::test]
async fn test_match_greater_than_or_equal() {
    let (interp, _) = run_script(
        r#"
        descending = [50, 40, 30, 20, 10]
        pos1 = match(35, descending, -1)
        pos2 = match(50, descending, -1)
        pos3 = match(60, descending, -1)
    "#,
    )
    .await;

    assert!(matches!(interp.get_var("pos1").await, Some(Value::Int(2))));
    assert!(matches!(interp.get_var("pos2").await, Some(Value::Int(1))));
    assert!(matches!(
        interp.get_var("pos3").await,
        Some(Value::String(s)) if s == "#N/A"
    ));
}

#[tokio::test]
async fn test_xlookup_basic() {
    let (interp, _) = run_script(
        r#"
        product_names = ["Apple", "Banana", "Cherry", "Date"]
        product_prices = [1.50, 0.75, 2.00, 3.50]
        price1 = xlookup("Banana", product_names, product_prices)
        price2 = xlookup("Date", product_names, product_prices)
        price3 = xlookup("Grape", product_names, product_prices, "Not Found")
    "#,
    )
    .await;

    assert!(matches!(
        interp.get_var("price1").await,
        Some(Value::Float(f)) if (f - 0.75).abs() < 0.001
    ));

    assert!(matches!(
        interp.get_var("price2").await,
        Some(Value::Float(f)) if (f - 3.5).abs() < 0.001
    ));

    assert!(matches!(
        interp.get_var("price3").await,
        Some(Value::String(s)) if s == "Not Found"
    ));
}

#[tokio::test]
async fn test_index_match_combination() {
    let (interp, _) = run_script(
        r#"
        products = [
            ["Apple", 1.50, 100],
            ["Banana", 0.75, 200],
            ["Cherry", 2.00, 150],
            ["Date", 3.50, 50]
        ]
        product_names = [products[0][0], products[1][0], products[2][0], products[3][0]]
        banana_pos = match("Banana", product_names, 0)
        banana_price = index(products, banana_pos, 2)
        banana_quantity = index(products, banana_pos, 3)
    "#,
    )
    .await;

    assert!(matches!(
        interp.get_var("banana_pos").await,
        Some(Value::Int(2))
    ));

    assert!(matches!(
        interp.get_var("banana_price").await,
        Some(Value::Float(f)) if (f - 0.75).abs() < 0.001
    ));

    assert!(matches!(
        interp.get_var("banana_quantity").await,
        Some(Value::Int(200))
    ));
}

#[tokio::test]
async fn test_vlookup_edge_cases() {
    let (interp, _) = run_script(
        r#"
        empty = []
        result1 = vlookup("test", empty, 1, false)
        single = [["Apple", 1.50]]
        result2 = vlookup("Apple", single, 2, false)
        data = [["Apple", 1.50], ["Banana", 0.75]]
    "#,
    )
    .await;

    assert!(matches!(
        interp.get_var("result1").await,
        Some(Value::String(s)) if s == "#N/A"
    ));

    assert!(matches!(
        interp.get_var("result2").await,
        Some(Value::Float(f)) if (f - 1.5).abs() < 0.001
    ));
}

#[tokio::test]
async fn test_xlookup_advanced_modes() {
    let (interp, _) = run_script(
        r#"
        values = [10, 20, 30, 40, 50]
        results = ["A", "B", "C", "D", "E"]
        result1 = xlookup(25, values, results, "None", -1)
        result2 = xlookup(25, values, results, "None", 1)
        duplicate_values = [10, 20, 20, 30]
        duplicate_results = ["First", "Second", "Third", "Fourth"]
        result3 = xlookup(20, duplicate_values, duplicate_results, "None", 0, -1)
    "#,
    )
    .await;

    assert!(matches!(
        interp.get_var("result1").await,
        Some(Value::String(s)) if s == "B"
    ));

    assert!(matches!(
        interp.get_var("result2").await,
        Some(Value::String(s)) if s == "C"
    ));

    assert!(matches!(
        interp.get_var("result3").await,
        Some(Value::String(s)) if s == "Third"
    ));
}

#[tokio::test]
async fn test_xlookup_search_mode_with_duplicates() {
    let (interp, _) = run_script(
        r#"
        lkp = [10, 20, 30, 20, 40]
        ret = ["A", "B", "C", "D", "E"]
        
        result1 = xlookup(20, lkp, ret, "Not Found", 0, 1)
        result2 = xlookup(20, lkp, ret, "Not Found", 0, -1)
    "#,
    )
    .await;

    // Forward search (search_mode=1) should return first match "B"
    if let Some(Value::String(s)) = interp.get_var("result1").await {
        assert_eq!(s, "B");
    } else {
        panic!("result1 should be string B");
    }

    // Backward search (search_mode=-1) should return last match "D"
    if let Some(Value::String(s)) = interp.get_var("result2").await {
        assert_eq!(s, "D");
    } else {
        panic!("result2 should be string D");
    }
}

#[tokio::test]
async fn test_vlookup_approximate_match() {
    let (interp, _) = run_script(
        r#"
        tax_table = [
            [0, 0.10],
            [10000, 0.12],
            [40000, 0.22],
            [85000, 0.24],
            [160000, 0.32]
        ]
        rate1 = vlookup(5000, tax_table, 2, true)
        rate2 = vlookup(25000, tax_table, 2, true)
        rate3 = vlookup(50000, tax_table, 2, true)
        rate4 = vlookup(100000, tax_table, 2, true)
    "#,
    )
    .await;

    assert!(matches!(
        interp.get_var("rate1").await,
        Some(Value::Float(f)) if (f - 0.10).abs() < 0.001
    ));

    assert!(matches!(
        interp.get_var("rate2").await,
        Some(Value::Float(f)) if (f - 0.12).abs() < 0.001
    ));

    assert!(matches!(
        interp.get_var("rate3").await,
        Some(Value::Float(f)) if (f - 0.22).abs() < 0.001
    ));

    assert!(matches!(
        interp.get_var("rate4").await,
        Some(Value::Float(f)) if (f - 0.24).abs() < 0.001
    ));
}
