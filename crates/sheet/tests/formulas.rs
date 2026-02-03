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
