use piptable_sheet::{CellValue, Sheet};

#[test]
fn test_set_and_evaluate_formula_sum() -> Result<(), Box<dyn std::error::Error>> {
    let mut sheet = Sheet::from_data(vec![vec![0, 0, 0]]);
    sheet.set_a1("A1", 1)?;
    sheet.set_a1("B1", 2)?;

    sheet.set_formula("C1", "=SUM(A1:B1)")?;
    sheet.evaluate_formulas()?;

    assert_eq!(sheet.get_a1("C1")?, &CellValue::Float(3.0));
    Ok(())
}

#[test]
fn test_formula_recalculates_on_input_change() -> Result<(), Box<dyn std::error::Error>> {
    let mut sheet = Sheet::from_data(vec![vec![0, 0, 0]]);
    sheet.set_a1("A1", 2)?;
    sheet.set_a1("B1", 3)?;
    sheet.set_formula("C1", "=A1+B1")?;
    sheet.evaluate_formulas()?;
    assert_eq!(sheet.get_a1("C1")?, &CellValue::Float(5.0));

    sheet.set_a1("A1", 10)?;
    sheet.evaluate_formulas()?;
    assert_eq!(sheet.get_a1("C1")?, &CellValue::Float(13.0));
    Ok(())
}
