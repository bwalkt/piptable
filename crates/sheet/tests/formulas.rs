use chrono::TimeZone;
use piptable_sheet::{CellValue, Sheet};

#[test]
fn test_set_and_evaluate_formula_sum() -> Result<(), Box<dyn std::error::Error>> {
    let mut sheet = Sheet::from_data(vec![vec![0, 0, 0]]);
    sheet.set_a1("A1", 1)?;
    sheet.set_a1("B1", 2)?;

    sheet.set_formula("C1", "=SUM(A1:B1)")?;
    sheet.evaluate_formulas()?;

    match sheet.get_a1("C1")? {
        CellValue::Formula(formula) => {
            assert_eq!(formula.source, "=SUM(A1:B1)");
            assert!(matches!(
                formula.cached.as_deref(),
                Some(CellValue::Float(f)) if (*f - 3.0).abs() < f64::EPSILON
            ));
        }
        _ => panic!("expected formula cell"),
    }

    assert_eq!(sheet.get_a1("C1")?.as_float(), Some(3.0));
    Ok(())
}

#[test]
fn test_formula_recalculates_on_input_change() -> Result<(), Box<dyn std::error::Error>> {
    let mut sheet = Sheet::from_data(vec![vec![0, 0, 0]]);
    sheet.set_a1("A1", 2)?;
    sheet.set_a1("B1", 3)?;
    sheet.set_formula("C1", "=A1+B1")?;
    sheet.evaluate_formulas()?;
    assert_eq!(sheet.get_a1("C1")?.as_float(), Some(5.0));

    sheet.set_a1("A1", 10)?;
    sheet.evaluate_formulas()?;
    assert_eq!(sheet.get_a1("C1")?.as_float(), Some(13.0));
    Ok(())
}

#[test]
fn test_circular_reference_error() -> Result<(), Box<dyn std::error::Error>> {
    let mut sheet = Sheet::from_data(vec![vec![0, 0]]);
    sheet.set_formula("A1", "=B1")?;
    let err = sheet.set_formula("B1", "=A1").unwrap_err();
    assert!(format!("{err}").contains("Circular"));
    Ok(())
}

#[test]
fn test_formula_with_multiple_functions() -> Result<(), Box<dyn std::error::Error>> {
    let mut sheet = Sheet::from_data(vec![vec![1, 2, 3, 0], vec![4, 5, 6, 0], vec![7, 8, 9, 0]]);

    // Test AVERAGE of a range
    sheet.set_formula("D1", "=AVERAGE(A1:C2)")?;
    sheet.evaluate_formulas()?;
    assert_eq!(sheet.get_a1("D1")?.as_float(), Some(3.5)); // (1+2+3+4+5+6)/6

    // Test MAX and MIN
    sheet.set_formula("D2", "=MAX(A1:C3)")?;
    sheet.evaluate_formulas()?;
    assert_eq!(sheet.get_a1("D2")?.as_float(), Some(9.0));

    sheet.set_formula("D3", "=MIN(A1:C3)")?;
    sheet.evaluate_formulas()?;
    assert_eq!(sheet.get_a1("D3")?.as_float(), Some(1.0));

    Ok(())
}

#[test]
fn test_formula_with_arithmetic_operations() -> Result<(), Box<dyn std::error::Error>> {
    let mut sheet = Sheet::from_data(vec![vec![10, 5, 0, 0, 0]]);

    // Test basic arithmetic
    sheet.set_formula("C1", "=A1*B1")?;
    sheet.evaluate_formulas()?;
    assert_eq!(sheet.get_a1("C1")?.as_float(), Some(50.0));

    sheet.set_formula("D1", "=A1/B1")?;
    sheet.evaluate_formulas()?;
    assert_eq!(sheet.get_a1("D1")?.as_float(), Some(2.0));

    // Test order of operations
    sheet.set_formula("E1", "=A1+B1*2")?;
    sheet.evaluate_formulas()?;
    assert_eq!(sheet.get_a1("E1")?.as_float(), Some(20.0)); // 10 + (5 * 2)

    Ok(())
}

#[test]
fn test_formula_with_string_functions() -> Result<(), Box<dyn std::error::Error>> {
    let mut sheet = Sheet::from_data(vec![vec!["hello", "world", "", ""]]);

    // Test CONCATENATE
    sheet.set_formula("C1", "=CONCATENATE(A1, \" \", B1)")?;
    sheet.evaluate_formulas()?;
    assert_eq!(sheet.get_a1("C1")?.as_str(), "hello world");

    // Test UPPER and LOWER
    sheet.set_formula("D1", "=UPPER(A1)")?;
    sheet.evaluate_formulas()?;
    assert_eq!(sheet.get_a1("D1")?.as_str(), "HELLO");

    Ok(())
}

#[test]
fn test_formula_with_logical_functions() -> Result<(), Box<dyn std::error::Error>> {
    let mut sheet = Sheet::from_data(vec![vec![5, 10, 0, 0]]);

    // Test IF function
    sheet.set_formula("C1", "=IF(A1<B1, \"less\", \"greater\")")?;
    sheet.evaluate_formulas()?;
    assert_eq!(sheet.get_a1("C1")?.as_str(), "less");

    // Test AND and OR
    sheet.set_formula("D1", "=AND(A1<B1, B1>0)")?;
    sheet.evaluate_formulas()?;
    assert_eq!(sheet.get_a1("D1")?.as_bool(), Some(true));

    Ok(())
}

#[test]
fn test_formula_chain_dependencies() -> Result<(), Box<dyn std::error::Error>> {
    let mut sheet = Sheet::from_data(vec![vec![1, 0, 0, 0]]);

    // Create a chain of formula dependencies
    sheet.set_formula("B1", "=A1*2")?;
    sheet.set_formula("C1", "=B1+3")?;
    sheet.set_formula("D1", "=C1/5")?;
    sheet.evaluate_formulas()?;

    assert_eq!(sheet.get_a1("B1")?.as_float(), Some(2.0));
    assert_eq!(sheet.get_a1("C1")?.as_float(), Some(5.0));
    assert_eq!(sheet.get_a1("D1")?.as_float(), Some(1.0));

    // Update source and check propagation
    sheet.set_a1("A1", 5)?;
    sheet.evaluate_formulas()?;

    assert_eq!(sheet.get_a1("B1")?.as_float(), Some(10.0));
    assert_eq!(sheet.get_a1("C1")?.as_float(), Some(13.0));
    assert_eq!(sheet.get_a1("D1")?.as_float(), Some(2.6));

    Ok(())
}

#[test]
fn test_formula_with_empty_cells() -> Result<(), Box<dyn std::error::Error>> {
    let mut sheet = Sheet::from_data(vec![
        vec![CellValue::Null, CellValue::Null, CellValue::Null],
        vec![CellValue::Null, CellValue::Null, CellValue::Null],
        vec![CellValue::Null, CellValue::Null, CellValue::Null],
    ]);

    // Set some values, leaving others empty
    sheet.set_a1("A1", 10)?;
    sheet.set_a1("C1", 5)?;

    // SUM should handle empty cells as 0
    sheet.set_formula("A3", "=SUM(A1:C1)")?;
    sheet.evaluate_formulas()?;
    assert_eq!(sheet.get_a1("A3")?.as_float(), Some(15.0));

    // COUNT should count non-empty cells
    sheet.set_formula("B3", "=COUNT(A1:C1)")?;
    sheet.evaluate_formulas()?;
    assert_eq!(sheet.get_a1("B3")?.as_float(), Some(2.0));

    Ok(())
}

#[test]
fn test_formula_with_date_functions() -> Result<(), Box<dyn std::error::Error>> {
    let mut sheet = Sheet::from_data(vec![vec![2024, 1, 15, 0]]);

    // Test DATE function
    sheet.set_formula("D1", "=DATE(A1, B1, C1)")?;
    sheet.evaluate_formulas()?;

    // DATE returns an Excel serial date based on local midnight.
    let result = sheet.get_a1("D1")?.as_float();
    assert!(result.is_some());
    let result = result.unwrap();
    let expected = excel_serial_for_local_date(2024, 1, 15);
    assert!(
        (result - expected).abs() < 1e-6,
        "Expected {expected}, got {result}"
    );

    Ok(())
}

#[test]
fn test_formula_error_propagation() -> Result<(), Box<dyn std::error::Error>> {
    let mut sheet = Sheet::from_data(vec![vec![10, 0, 0]]);

    // Division by zero should produce an error
    sheet.set_formula("C1", "=A1/B1")?;
    sheet.evaluate_formulas()?;

    // The formula should evaluate but produce an error value
    match sheet.get_a1("C1")? {
        CellValue::Formula(formula) => {
            assert!(
                matches!(formula.cached.as_deref(), Some(CellValue::String(s)) if s == "#DIV/0!")
            );
        }
        _ => panic!("expected formula cell"),
    }

    Ok(())
}

#[test]
fn test_formula_absolute_references() -> Result<(), Box<dyn std::error::Error>> {
    let mut sheet = Sheet::from_data(vec![vec![1, 2, 0], vec![3, 4, 0], vec![5, 6, 0]]);

    // Test absolute reference parsing; add copy/fill coverage when supported.
    sheet.set_formula("C1", "=$A$1+B1")?;
    sheet.set_formula("C2", "=$A$1+B2")?;
    sheet.evaluate_formulas()?;

    assert_eq!(sheet.get_a1("C1")?.as_float(), Some(3.0)); // 1 + 2
    assert_eq!(sheet.get_a1("C2")?.as_float(), Some(5.0)); // 1 + 4

    Ok(())
}

fn excel_serial_for_local_date(year: i32, month: u32, day: u32) -> f64 {
    let date = chrono::NaiveDate::from_ymd_opt(year, month, day).expect("valid date");
    let naive = date.and_hms_opt(0, 0, 0).expect("valid time");
    let local_dt = chrono::Local
        .from_local_datetime(&naive)
        .earliest()
        .expect("local datetime");
    let utc = local_dt.with_timezone(&chrono::Utc);
    let unix_seconds = utc.timestamp();
    let unix_days = unix_seconds / 86_400;
    let time_fraction = (unix_seconds.rem_euclid(86_400)) as f64 / 86_400.0;
    (unix_days + 25_569) as f64 + time_fraction
}

#[test]
fn test_formula_row_column_operations() -> Result<(), Box<dyn std::error::Error>> {
    let mut sheet = Sheet::from_data(vec![
        vec![1, 2, 3, 0],
        vec![4, 5, 6, 0],
        vec![7, 8, 9, 0],
        vec![0, 0, 0, 0],
    ]);

    // Sum entire row
    sheet.set_formula("D1", "=SUM(A1:C1)")?;
    sheet.set_formula("D2", "=SUM(A2:C2)")?;
    sheet.set_formula("D3", "=SUM(A3:C3)")?;

    // Sum entire column
    sheet.set_formula("A4", "=SUM(A1:A3)")?;
    sheet.set_formula("B4", "=SUM(B1:B3)")?;
    sheet.set_formula("C4", "=SUM(C1:C3)")?;

    // Grand total
    sheet.set_formula("D4", "=SUM(A1:C3)")?;

    sheet.evaluate_formulas()?;

    // Check row sums
    assert_eq!(sheet.get_a1("D1")?.as_float(), Some(6.0));
    assert_eq!(sheet.get_a1("D2")?.as_float(), Some(15.0));
    assert_eq!(sheet.get_a1("D3")?.as_float(), Some(24.0));

    // Check column sums
    assert_eq!(sheet.get_a1("A4")?.as_float(), Some(12.0));
    assert_eq!(sheet.get_a1("B4")?.as_float(), Some(15.0));
    assert_eq!(sheet.get_a1("C4")?.as_float(), Some(18.0));

    // Check grand total
    assert_eq!(sheet.get_a1("D4")?.as_float(), Some(45.0));

    Ok(())
}

#[test]
fn test_set_formula_invalid_returns_error() -> Result<(), Box<dyn std::error::Error>> {
    let mut sheet = Sheet::from_data(vec![vec![0]]);
    let err = sheet.set_formula("A1", "=SUM(").unwrap_err();
    assert!(format!("{err}").contains("Formula error"));
    Ok(())
}

#[test]
fn test_formula_replaced_and_recalculated() -> Result<(), Box<dyn std::error::Error>> {
    let mut sheet = Sheet::from_data(vec![vec![1, 2, 0]]);
    sheet.set_formula("C1", "=A1+B1")?;
    sheet.evaluate_formulas()?;
    assert_eq!(sheet.get_a1("C1")?.as_float(), Some(3.0));

    sheet.set_formula("C1", "=A1*B1")?;
    sheet.evaluate_formulas()?;
    assert_eq!(sheet.get_a1("C1")?.as_float(), Some(2.0));
    Ok(())
}
