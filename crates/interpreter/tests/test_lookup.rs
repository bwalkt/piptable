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
    let err = run_script_err(
        r#"
        products = [
            ["Apple", 1.50, 100],
            ["Banana", 0.75, 200]
        ]
        missing = vlookup("Grape", products, 2, false)
    "#,
    )
    .await;
    assert!(err.contains("Formula error: #N/A"));
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
    let err = run_script_err(
        r#"
        quarterly = [
            ["Product", "Q1", "Q2", "Q3", "Q4"],
            ["Sales", 100, 150, 120, 180]
        ]
        missing = hlookup("Q5", quarterly, 2, false)
    "#,
    )
    .await;
    assert!(err.contains("Formula error: #N/A"));
}

#[tokio::test]
async fn test_hlookup_approximate_match() {
    let (interp, _) = run_script(
        r"
        salary_table = [
            [0, 30000, 40000, 50000, 60000, 70000],
            [10000, 35000, 45000, 55000, 65000, 75000],
            [1000, 2000, 3000, 4000, 5000, 6000]
        ]
        salary1 = hlookup(35000, salary_table, 2, true)
        salary2 = hlookup(45000, salary_table, 2, true)
        bonus1 = hlookup(55000, salary_table, 3, true)
        bonus2 = hlookup(65000, salary_table, 3, true)
    ",
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
        r"
        data = [
            [10, 20, 30],
            [40, 50, 60],
            [70, 80, 90]
        ]
        val1 = index(data, 2, 3)
        val2 = index(data, 3, 1)
        row2 = index(data, 2)
    ",
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
    "#,
    )
    .await;

    assert!(matches!(interp.get_var("pos1").await, Some(Value::Int(2))));

    assert!(matches!(interp.get_var("pos2").await, Some(Value::Int(4))));

    assert!(matches!(interp.get_var("pos3").await, Some(Value::Int(3))));
    let err = run_script_err(
        r#"
        fruits = ["Apple", "Banana", "Cherry", "Date"]
        missing = match("Grape", fruits, 0)
    "#,
    )
    .await;
    assert!(err.contains("Formula error: #N/A"));
}

#[tokio::test]
async fn test_match_less_than_or_equal() {
    let (interp, _) = run_script(
        r"
        sorted_nums = [10, 20, 30, 40, 50]
        pos1 = match(25, sorted_nums, 1)
        pos2 = match(30, sorted_nums, 1)
        pos3 = match(55, sorted_nums, 1)
    ",
    )
    .await;

    assert!(matches!(interp.get_var("pos1").await, Some(Value::Int(2))));

    assert!(matches!(interp.get_var("pos2").await, Some(Value::Int(3))));

    assert!(matches!(interp.get_var("pos3").await, Some(Value::Int(5))));
    let err = run_script_err(
        r#"
        sorted_nums = [10, 20, 30, 40, 50]
        missing = match(5, sorted_nums, 1)
    "#,
    )
    .await;
    assert!(err.contains("Formula error: #N/A"));
}

#[tokio::test]
async fn test_match_greater_than_or_equal() {
    let (interp, _) = run_script(
        r"
        descending = [50, 40, 30, 20, 10]
        pos1 = match(35, descending, -1)
        pos2 = match(50, descending, -1)
    ",
    )
    .await;

    assert!(matches!(interp.get_var("pos1").await, Some(Value::Int(2))));
    assert!(matches!(interp.get_var("pos2").await, Some(Value::Int(1))));
    let err = run_script_err(
        r#"
        descending = [50, 40, 30, 20, 10]
        missing = match(60, descending, -1)
    "#,
    )
    .await;
    assert!(err.contains("Formula error: #N/A"));
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
        single = [["Apple", 1.50]]
        result2 = vlookup("Apple", single, 2, false)
        data = [["Apple", 1.50], ["Banana", 0.75]]
    "#,
    )
    .await;

    assert!(matches!(
        interp.get_var("result2").await,
        Some(Value::Float(f)) if (f - 1.5).abs() < 0.001
    ));

    let err = run_script_err(
        r#"
        empty = []
        result1 = vlookup("test", empty, 1, false)
    "#,
    )
    .await;
    assert!(err.contains("Formula error: #N/A"));
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
async fn test_xlookup_wildcard_matching() {
    let (interp, _) = run_script(
        r#"
        names = ["Apple", "Application", "Cat", "Bat", "Data*"]
        prices = [1, 2, 3, 4, 5]
        result1 = xlookup("App*", names, prices, "N/A", 2)
        result2 = xlookup("?at", names, prices, "N/A", 2)
        result3 = xlookup("Data\\*", names, prices, "N/A", 2)
        result4 = xlookup("app*", names, prices, "N/A", 2, 1, true)
    "#,
    )
    .await;

    assert!(matches!(
        interp.get_var("result1").await,
        Some(Value::Int(1))
    ));
    assert!(matches!(
        interp.get_var("result2").await,
        Some(Value::Int(3))
    ));
    assert!(matches!(
        interp.get_var("result3").await,
        Some(Value::Int(5))
    ));
    assert!(matches!(
        interp.get_var("result4").await,
        Some(Value::Int(1))
    ));
}

#[tokio::test]
async fn test_xlookup_binary_search_modes() {
    let (interp, _) = run_script(
        r#"
        values_asc = [1, 3, 5, 7, 9]
        results_asc = ["A", "B", "C", "D", "E"]
        exact_asc = xlookup(5, values_asc, results_asc, "N/A", 0, 2)
        next_small_asc = xlookup(6, values_asc, results_asc, "N/A", -1, 2)
        next_large_asc = xlookup(6, values_asc, results_asc, "N/A", 1, 2)

        values_desc = [9, 7, 5, 3, 1]
        results_desc = ["E", "D", "C", "B", "A"]
        exact_desc = xlookup(5, values_desc, results_desc, "N/A", 0, -2)
        next_small_desc = xlookup(6, values_desc, results_desc, "N/A", -1, -2)
        next_large_desc = xlookup(6, values_desc, results_desc, "N/A", 1, -2)
        next_large_desc_small = xlookup(0, values_desc, results_desc, "N/A", 1, -2)
    "#,
    )
    .await;

    assert!(matches!(
        interp.get_var("exact_asc").await,
        Some(Value::String(s)) if s == "C"
    ));
    assert!(matches!(
        interp.get_var("next_small_asc").await,
        Some(Value::String(s)) if s == "C"
    ));
    assert!(matches!(
        interp.get_var("next_large_asc").await,
        Some(Value::String(s)) if s == "D"
    ));

    assert!(matches!(
        interp.get_var("exact_desc").await,
        Some(Value::String(s)) if s == "C"
    ));
    assert!(matches!(
        interp.get_var("next_small_desc").await,
        Some(Value::String(s)) if s == "C"
    ));
    assert!(matches!(
        interp.get_var("next_large_desc").await,
        Some(Value::String(s)) if s == "D"
    ));
    assert!(matches!(
        interp.get_var("next_large_desc_small").await,
        Some(Value::String(s)) if s == "A"
    ));
}

#[tokio::test]
async fn test_xlookup_binary_search_requires_sorted() {
    let err = run_script_err(
        r#"
        values = [1, 3, 2]
        results = ["A", "B", "C"]
        result = xlookup(2, values, results, "N/A", 0, 2)
    "#,
    )
    .await;

    assert!(err.contains("Formula error: #VALUE"));
}

#[tokio::test]
async fn test_vlookup_approximate_match() {
    let (interp, _) = run_script(
        r"
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
    ",
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

// TODO: Enable these tests when type coercion is fully implemented
#[tokio::test]
#[ignore = "type coercion not fully implemented yet"]
async fn test_vlookup_type_coercion() {
    let (interp, _) = run_script(
        r#"
        ' Test numeric string to number coercion
        data = [
            ["1", "One"],
            [2, "Two"],
            ["3.0", "Three"],
            [4.0, "Four"]
        ]

        ' Looking up with different numeric types
        result1 = vlookup(1, data, 2, false)      ' Int looking for string "1"
        result2 = vlookup("2", data, 2, false)    ' String looking for int 2
        result3 = vlookup(3, data, 2, false)      ' Int looking for string "3.0"
        result4 = vlookup("4", data, 2, false)    ' String looking for float 4.0
    "#,
    )
    .await;

    assert!(matches!(
        interp.get_var("result1").await,
        Some(Value::String(s)) if s == "One"
    ));

    assert!(matches!(
        interp.get_var("result2").await,
        Some(Value::String(s)) if s == "Two"
    ));

    assert!(matches!(
        interp.get_var("result3").await,
        Some(Value::String(s)) if s == "Three"
    ));

    assert!(matches!(
        interp.get_var("result4").await,
        Some(Value::String(s)) if s == "Four"
    ));
}

#[tokio::test]
#[ignore = "type coercion not fully implemented yet"]
async fn test_match_type_coercion() {
    let (interp, _) = run_script(
        r#"
        ' Test MATCH with mixed types
        mixed_array = ["1", 2, "3.0", 4.0, 5]

        pos1 = match(1, mixed_array, 0)     ' Int matches string "1"
        pos2 = match("2", mixed_array, 0)   ' String matches int 2
        pos3 = match(3.0, mixed_array, 0)   ' Float matches string "3.0"
        pos4 = match("4", mixed_array, 0)   ' String matches float 4.0
        pos5 = match("5", mixed_array, 0)   ' String matches int 5
    "#,
    )
    .await;

    assert!(matches!(interp.get_var("pos1").await, Some(Value::Int(1))));
    assert!(matches!(interp.get_var("pos2").await, Some(Value::Int(2))));
    assert!(matches!(interp.get_var("pos3").await, Some(Value::Int(3))));
    assert!(matches!(interp.get_var("pos4").await, Some(Value::Int(4))));
    assert!(matches!(interp.get_var("pos5").await, Some(Value::Int(5))));
}

#[tokio::test]
#[ignore = "xlookup search direction not fully implemented yet"]
async fn test_xlookup_duplicates_with_search_direction() {
    let (interp, _) = run_script(
        r#"
        ' Test XLOOKUP with duplicates and different search modes
        names = ["Apple", "Banana", "Apple", "Cherry", "Banana"]
        prices = [1.50, 0.75, 1.75, 2.00, 0.80]

        ' Search mode 1: first to last (default)
        first_apple = xlookup("Apple", names, prices, "N/A", 0, 1)
        first_banana = xlookup("Banana", names, prices, "N/A", 0, 1)

        ' Search mode -1: last to first
        last_apple = xlookup("Apple", names, prices, "N/A", 0, -1)
        last_banana = xlookup("Banana", names, prices, "N/A", 0, -1)
    "#,
    )
    .await;

    // First occurrence matches
    assert!(matches!(
        interp.get_var("first_apple").await,
        Some(Value::Float(f)) if (f - 1.50).abs() < 0.001
    ));

    assert!(matches!(
        interp.get_var("first_banana").await,
        Some(Value::Float(f)) if (f - 0.75).abs() < 0.001
    ));

    // Last occurrence
    assert!(matches!(
        interp.get_var("last_apple").await,
        Some(Value::Float(f)) if (f - 1.75).abs() < 0.001
    ));

    assert!(matches!(
        interp.get_var("last_banana").await,
        Some(Value::Float(f)) if (f - 0.80).abs() < 0.001
    ));
}

#[tokio::test]
#[ignore = "index error handling not fully implemented yet"]
async fn test_index_invalid_inputs() {
    let (interp, _) = run_script(
        r"
        data = [
            [1, 2, 3],
            [4, 5, 6],
            [7, 8, 9]
        ]

        ' Invalid indices
        result1 = index(data, 0, 1)     ' Row index 0 (invalid)
        result2 = index(data, 4, 1)     ' Row index out of bounds
        result3 = index(data, 1, 0)     ' Column index 0 (invalid)
        result4 = index(data, 1, 4)     ' Column index out of bounds
        result5 = index(data, -1, 1)    ' Negative row index
        result6 = index(data, 1, -1)    ' Negative column index
    ",
    )
    .await;

    // All invalid indices should return error
    assert!(matches!(
        interp.get_var("result1").await,
        Some(Value::String(s)) if s.starts_with('#')
    ));

    assert!(matches!(
        interp.get_var("result2").await,
        Some(Value::String(s)) if s.starts_with('#')
    ));

    assert!(matches!(
        interp.get_var("result3").await,
        Some(Value::String(s)) if s.starts_with('#')
    ));

    assert!(matches!(
        interp.get_var("result4").await,
        Some(Value::String(s)) if s.starts_with('#')
    ));

    assert!(matches!(
        interp.get_var("result5").await,
        Some(Value::String(s)) if s.starts_with('#')
    ));

    assert!(matches!(
        interp.get_var("result6").await,
        Some(Value::String(s)) if s.starts_with('#')
    ));
}

#[tokio::test]
#[ignore = "edge case handling not fully implemented yet"]
async fn test_vlookup_additional_edge_cases() {
    let (interp, _) = run_script(
        r#"
        ' Edge cases for VLOOKUP
        empty_table = []
        single_row = [["Apple", 1.50, 100]]
        single_col = [["Apple"], ["Banana"], ["Cherry"]]

        ' Empty table
        result1 = vlookup("Apple", empty_table, 1, false)

        ' Single row table
        result2 = vlookup("Apple", single_row, 2, false)
        result3 = vlookup("Banana", single_row, 2, false)

        ' Single column table (col_index out of bounds)
        result4 = vlookup("Apple", single_col, 2, false)

        ' Null lookup value
        null_data = [
            [null, "Null row"],
            ["Apple", "Apple row"]
        ]
        result5 = vlookup(null, null_data, 2, false)
    "#,
    )
    .await;

    assert!(matches!(
        interp.get_var("result1").await,
        Some(Value::String(s)) if s == "#N/A"
    ));

    assert!(matches!(
        interp.get_var("result2").await,
        Some(Value::Float(f)) if (f - 1.50).abs() < 0.001
    ));

    assert!(matches!(
        interp.get_var("result3").await,
        Some(Value::String(s)) if s == "#N/A"
    ));

    assert!(matches!(
        interp.get_var("result4").await,
        Some(Value::String(s)) if s.starts_with('#')
    ));

    assert!(matches!(
        interp.get_var("result5").await,
        Some(Value::String(s)) if s == "Null row"
    ));
}
