//! Tests for lambda expression support with arrow syntax

mod common {
    include!("common_impl.txt");
}
use common::*;

use piptable_core::Value;

#[tokio::test]
async fn test_lambda_single_param_arrow_syntax() {
    let (interp, _) = run_script(
        r#"
        dim double = x => x * 2
    "#,
    )
    .await;

    // First check if the lambda was stored correctly
    match interp.get_var("double").await {
        Some(Value::Lambda { params, .. }) => {
            assert_eq!(params.len(), 1);
            assert_eq!(params[0], "x");
        }
        other => panic!("Expected lambda, got: {:?}", other),
    }
    
    // Now test calling it
    let (interp2, _) = run_script(
        r#"
        dim double = x => x * 2
        dim result = double(5)
    "#,
    )
    .await;

    assert!(matches!(
        interp2.get_var("result").await,
        Some(Value::Int(10))
    ));
}

#[tokio::test]
async fn test_lambda_multiple_params_arrow_syntax() {
    let (interp, _) = run_script(
        r#"
        dim add = (x, y) => x + y
        dim result = add(3, 4)
    "#,
    )
    .await;

    assert!(matches!(
        interp.get_var("result").await,
        Some(Value::Int(7))
    ));
}

#[tokio::test]
async fn test_lambda_no_params_arrow_syntax() {
    let (interp, _) = run_script(
        r#"
        dim constant = () => 42
        dim result = constant()
    "#,
    )
    .await;

    assert!(matches!(
        interp.get_var("result").await,
        Some(Value::Int(42))
    ));
}

#[tokio::test]
#[ignore] // Lambda block expressions not yet supported
async fn test_lambda_with_sheet_map() {
    let (interp, _) = run_script(
        r#"
        import "test_data.csv" as data {
            headers: true,
            content: "Name,Age,Score\nAlice,25,85\nBob,30,92\nCharlie,28,78"
        }
        
        ' Transform ages using lambda
        dim transformed = data.map(cell => {
            if type(cell) = "int" then
                return cell + 10
            else
                return cell
            end if
        })
    "#,
    )
    .await;

    match interp.get_var("transformed").await {
        Some(Value::Sheet(sheet)) => {
            // Check that ages were incremented by 10
            let data = sheet.data();
            assert_eq!(data.len(), 4); // Header + 3 data rows
            
            // Check transformed ages (column 1)
            use piptable_sheet::CellValue;
            if let Some(row) = data.get(1) {
                assert!(matches!(row.get(1), Some(CellValue::Int(35)))); // Alice: 25 + 10
            }
            if let Some(row) = data.get(2) {
                assert!(matches!(row.get(1), Some(CellValue::Int(40)))); // Bob: 30 + 10
            }
            if let Some(row) = data.get(3) {
                assert!(matches!(row.get(1), Some(CellValue::Int(38)))); // Charlie: 28 + 10
            }
        }
        _ => panic!("Expected sheet result"),
    }
}

#[tokio::test]
async fn test_lambda_with_sheet_filter() {
    // Create temp file
    let temp_file = create_temp_csv("Name,Age,Score\nAlice,25,85\nBob,30,92\nCharlie,28,78\nDavid,35,88");
    let script = format!(
        r#"
        import "{}" into data
        
        ' Filter rows where age > 28 using lambda
        dim filtered = data.filter(row => row->Age > 28)
    "#,
        temp_file.path().display()
    );
    
    let (interp, _) = run_script(&script).await;

    match interp.get_var("filtered").await {
        Some(Value::Sheet(sheet)) => {
            let data = sheet.data();
            // Should have header + 2 rows (Bob and David)
            assert_eq!(data.len(), 3);
            
            // Check that we have Bob and David
            use piptable_sheet::CellValue;
            if let Some(row) = data.get(1) {
                assert!(matches!(row.get(0), Some(CellValue::String(s)) if s == "Bob"));
                assert!(matches!(row.get(1), Some(CellValue::Int(30))));
            }
            if let Some(row) = data.get(2) {
                assert!(matches!(row.get(0), Some(CellValue::String(s)) if s == "David"));
                assert!(matches!(row.get(1), Some(CellValue::Int(35))));
            }
        }
        _ => panic!("Expected sheet result"),
    }
}

#[tokio::test]
async fn test_lambda_closure_capture() {
    let (interp, _) = run_script(
        r#"
        dim factor = 10
        dim multiply = x => x * factor
        dim result = multiply(5)
    "#,
    )
    .await;

    assert!(matches!(
        interp.get_var("result").await,
        Some(Value::Int(50))
    ));
}

#[tokio::test]
async fn test_lambda_in_array_map() {
    let (interp, _) = run_script(
        r#"
        dim numbers = [1, 2, 3, 4, 5]
        
        ' Use lambda to transform each number
        dim square_fn = x => x * x
        dim result1 = square_fn(numbers[0])
        dim result2 = square_fn(numbers[1])
        dim result3 = square_fn(numbers[2])
        dim result4 = square_fn(numbers[3])
        dim result5 = square_fn(numbers[4])
        
        dim squares = [result1, result2, result3, result4, result5]
    "#,
    )
    .await;

    match interp.get_var("squares").await {
        Some(Value::Array(arr)) => {
            assert_eq!(arr.len(), 5);
            assert!(matches!(arr[0], Value::Int(1)));
            assert!(matches!(arr[1], Value::Int(4)));
            assert!(matches!(arr[2], Value::Int(9)));
            assert!(matches!(arr[3], Value::Int(16)));
            assert!(matches!(arr[4], Value::Int(25)));
        }
        _ => panic!("Expected array result"),
    }
}

#[tokio::test]
#[ignore] // Nested lambdas with closures not yet fully supported
async fn test_lambda_nested() {
    let (interp, _) = run_script(
        r#"
        dim outer = x => (y => x + y)
        dim add5 = outer(5)
        dim result = add5(3)
    "#,
    )
    .await;

    assert!(matches!(
        interp.get_var("result").await,
        Some(Value::Int(8))
    ));
}

#[tokio::test]
async fn test_lambda_with_string_operations() {
    let (interp, _) = run_script(
        r#"
        dim greet = name => "Hello, " + name + "!"
        dim message = greet("World")
    "#,
    )
    .await;

    assert!(matches!(
        interp.get_var("message").await,
        Some(Value::String(s)) if s == "Hello, World!"
    ));
}

#[tokio::test]
#[ignore] // Lambda block expressions not yet supported
async fn test_lambda_with_conditional() {
    let (interp, _) = run_script(
        r#"
        dim abs_value = x => {
            if x < 0 then
                return -x
            else
                return x
            end if
        }
        dim result1 = abs_value(-5)
        dim result2 = abs_value(3)
    "#,
    )
    .await;

    assert!(matches!(
        interp.get_var("result1").await,
        Some(Value::Int(5))
    ));
    
    assert!(matches!(
        interp.get_var("result2").await,
        Some(Value::Int(3))
    ));
}

#[tokio::test]
async fn test_lambda_with_simple_conditional() {
    let (interp, _) = run_script(
        r#"
        dim is_positive = x => x > 0
        dim result1 = is_positive(5)
        dim result2 = is_positive(-3)
    "#,
    )
    .await;

    assert!(matches!(
        interp.get_var("result1").await,
        Some(Value::Bool(true))
    ));
    
    assert!(matches!(
        interp.get_var("result2").await,
        Some(Value::Bool(false))
    ));
}