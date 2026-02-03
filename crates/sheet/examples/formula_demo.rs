//! Simple Formula Demo
//!
//! Run with: cargo run --example formula_demo -p piptable-sheet

use piptable_sheet::Sheet;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Formula Demo ===\n");

    // Create a simple spreadsheet with initial size
    let mut sheet = Sheet::from_data(vec![
        vec![""; 10]; // 10 columns
        10            // 10 rows
    ]);

    // Set some values
    sheet.set_a1("A1", 10)?;
    sheet.set_a1("B1", 20)?;
    sheet.set_a1("A2", 5)?;
    sheet.set_a1("B2", 15)?;

    println!("Initial values:");
    println!("A1 = 10, B1 = 20");
    println!("A2 = 5,  B2 = 15\n");

    // Add formulas
    sheet.set_formula("C1", "=A1+B1")?;
    sheet.set_formula("C2", "=A2*B2")?;
    sheet.set_formula("A3", "=SUM(A1:A2)")?;
    sheet.set_formula("B3", "=SUM(B1:B2)")?;
    sheet.set_formula("C3", "=SUM(C1:C2)")?;

    // Add more complex formulas
    sheet.set_formula("D1", "=AVERAGE(A1:B2)")?;
    sheet.set_formula("D2", "=MAX(A1:C2)")?;
    sheet.set_formula("D3", "=MIN(A1:C2)")?;

    println!("Formulas added:");
    println!("C1 = A1+B1");
    println!("C2 = A2*B2");
    println!("A3 = SUM(A1:A2)");
    println!("B3 = SUM(B1:B2)");
    println!("C3 = SUM(C1:C2)");
    println!("D1 = AVERAGE(A1:B2)");
    println!("D2 = MAX(A1:C2)");
    println!("D3 = MIN(A1:C2)\n");

    // Evaluate formulas
    sheet.evaluate_formulas()?;

    println!("Results after evaluation:");
    println!(
        "C1 = {} (should be 30)",
        sheet.get_a1("C1")?.as_float().unwrap_or(0.0)
    );
    println!(
        "C2 = {} (should be 75)",
        sheet.get_a1("C2")?.as_float().unwrap_or(0.0)
    );
    println!(
        "A3 = {} (should be 15)",
        sheet.get_a1("A3")?.as_float().unwrap_or(0.0)
    );
    println!(
        "B3 = {} (should be 35)",
        sheet.get_a1("B3")?.as_float().unwrap_or(0.0)
    );
    println!(
        "C3 = {} (should be 105)",
        sheet.get_a1("C3")?.as_float().unwrap_or(0.0)
    );
    println!(
        "D1 = {} (should be 12.5)",
        sheet.get_a1("D1")?.as_float().unwrap_or(0.0)
    );
    println!(
        "D2 = {} (should be 75)",
        sheet.get_a1("D2")?.as_float().unwrap_or(0.0)
    );
    println!(
        "D3 = {} (should be 5)",
        sheet.get_a1("D3")?.as_float().unwrap_or(0.0)
    );
    println!();

    // Test dynamic recalculation
    println!("Testing dynamic recalculation:");
    println!("Changing A1 from 10 to 100...");
    sheet.set_a1("A1", 100)?;
    sheet.evaluate_formulas()?;

    println!("New results:");
    println!(
        "C1 = {} (should be 120)",
        sheet.get_a1("C1")?.as_float().unwrap_or(0.0)
    );
    println!(
        "A3 = {} (should be 105)",
        sheet.get_a1("A3")?.as_float().unwrap_or(0.0)
    );
    println!(
        "C3 = {} (should be 195)",
        sheet.get_a1("C3")?.as_float().unwrap_or(0.0)
    );
    println!();

    // Test string functions
    println!("Testing string functions:");
    sheet.set_a1("E1", "hello")?;
    sheet.set_a1("E2", "world")?;
    sheet.set_formula("E3", "=CONCATENATE(E1, \" \", E2)")?;
    sheet.set_formula("E4", "=UPPER(E1)")?;
    sheet.set_formula("E5", "=LEN(E3)")?;

    sheet.evaluate_formulas()?;

    println!("E1 = 'hello'");
    println!("E2 = 'world'");
    println!(
        "E3 = CONCATENATE(E1, \" \", E2) = '{}'",
        sheet.get_a1("E3")?.as_str()
    );
    println!("E4 = UPPER(E1) = '{}'", sheet.get_a1("E4")?.as_str());
    println!(
        "E5 = LEN(E3) = {}",
        sheet.get_a1("E5")?.as_float().unwrap_or(0.0)
    );
    println!();

    // Test logical functions
    println!("Testing logical functions:");
    sheet.set_formula("F1", "=IF(A1>50, \"high\", \"low\")")?;
    sheet.set_formula("F2", "=AND(A1>50, B1<100)")?;
    sheet.set_formula("F3", "=OR(A1<10, B1>10)")?;

    sheet.evaluate_formulas()?;

    println!(
        "F1 = IF(A1>50, \"high\", \"low\") = '{}'",
        sheet.get_a1("F1")?.as_str()
    );
    println!(
        "F2 = AND(A1>50, B1<100) = {}",
        sheet.get_a1("F2")?.as_bool().unwrap_or(false)
    );
    println!(
        "F3 = OR(A1<10, B1>10) = {}",
        sheet.get_a1("F3")?.as_bool().unwrap_or(false)
    );

    Ok(())
}
