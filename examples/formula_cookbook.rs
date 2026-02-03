//! Formula Cookbook Examples
//!
//! This file contains practical examples of using formulas in piptable-sheet.

use piptable_sheet::{CellValue, Sheet};

macro_rules! row {
    ($($value:expr),* $(,)?) => {
        vec![$(CellValue::from($value)),*]
    };
}

/// Runs the formula cookbook example.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Piptable Formula Cookbook ===\n");

    basic_arithmetic_example()?;
    financial_calculations()?;
    data_analysis_example()?;
    string_manipulation_example()?;
    conditional_logic_example()?;
    dynamic_spreadsheet_example()?;

    Ok(())
}

/// Example: Basic Arithmetic Operations
fn basic_arithmetic_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("1. Basic Arithmetic Operations");
    println!("------------------------------");

    let mut sheet = Sheet::from_data(vec![
        row!["Price", "Quantity", "Total", "Tax Rate", "Tax", "Final"],
        row![100, 5, 0, 0.08, 0, 0],
        row![250, 2, 0, 0.08, 0, 0],
        row![75, 10, 0, 0.08, 0, 0],
    ]);

    // Calculate totals
    sheet.set_formula("C2", "=A2*B2")?;
    sheet.set_formula("C3", "=A3*B3")?;
    sheet.set_formula("C4", "=A4*B4")?;

    // Calculate tax
    sheet.set_formula("E2", "=C2*D2")?;
    sheet.set_formula("E3", "=C3*D3")?;
    sheet.set_formula("E4", "=C4*D4")?;

    // Final price with tax
    sheet.set_formula("F2", "=C2+E2")?;
    sheet.set_formula("F3", "=C3+E3")?;
    sheet.set_formula("F4", "=C4+E4")?;

    sheet.evaluate_formulas()?;

    // Print results
    for row in 0..sheet.row_count() {
        for col in 0..sheet.col_count() {
            let value = sheet.get(row, col)?;
            print!("{:>10} ", value.as_str());
        }
        println!();
    }
    println!();

    Ok(())
}

/// Example: Financial Calculations
fn financial_calculations() -> Result<(), Box<dyn std::error::Error>> {
    println!("2. Financial Calculations");
    println!("-------------------------");

    let mut sheet = Sheet::from_data(vec![
        row!["Month", "Revenue", "Expenses", "Profit", "Margin %"],
        row!["Jan", 10000, 7000, 0, 0],
        row!["Feb", 12000, 8000, 0, 0],
        row!["Mar", 15000, 9000, 0, 0],
        row!["Apr", 11000, 7500, 0, 0],
        row!["Total", 0, 0, 0, 0],
    ]);

    // Calculate profit for each month
    for row in 2..=5 {
        sheet.set_formula(&format!("D{}", row), &format!("=B{}-C{}", row, row))?;
        sheet.set_formula(&format!("E{}", row), &format!("=(D{}/B{})*100", row, row))?;
    }

    // Calculate totals
    sheet.set_formula("B6", "=SUM(B2:B5)")?;
    sheet.set_formula("C6", "=SUM(C2:C5)")?;
    sheet.set_formula("D6", "=B6-C6")?;
    sheet.set_formula("E6", "=(D6/B6)*100")?;

    sheet.evaluate_formulas()?;

    // Print financial summary
    sheet.name_columns_by_row(0)?;
    for row in 1..sheet.row_count() {
        let month = sheet.get(row, 0)?;
        let profit = sheet.get(row, 3)?.as_float().unwrap_or(0.0);
        let margin = sheet.get(row, 4)?.as_float().unwrap_or(0.0);
        println!(
            "{:>6}: Profit = ${:8.2}, Margin = {:.1}%",
            month.as_str(),
            profit,
            margin
        );
    }
    println!();

    Ok(())
}

/// Example: Data Analysis with Aggregation Functions
fn data_analysis_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("3. Data Analysis with Aggregation");
    println!("----------------------------------");

    let mut sheet = Sheet::from_data(vec![
        row!["Student", "Test1", "Test2", "Test3", "Average", "Grade"],
        row!["Alice", 85, 90, 88, 0, ""],
        row!["Bob", 78, 82, 85, 0, ""],
        row!["Carol", 92, 95, 94, 0, ""],
        row!["David", 70, 75, 72, 0, ""],
        row!["Statistics", 0, 0, 0, 0, ""],
        row!["Average", 0, 0, 0, 0, ""],
        row!["Max", 0, 0, 0, 0, ""],
        row!["Min", 0, 0, 0, 0, ""],
    ]);

    // Calculate student averages
    for row in 2..=5 {
        sheet.set_formula(
            &format!("E{}", row),
            &format!("=AVERAGE(B{}:D{})", row, row),
        )?;
        sheet.set_formula(
            &format!("F{}", row),
            &format!(
                "=IF(E{}>90,\"A\",IF(E{}>80,\"B\",IF(E{}>70,\"C\",\"D\")))",
                row, row, row
            ),
        )?;
    }

    // Calculate statistics for each test
    sheet.set_formula("B7", "=AVERAGE(B2:B5)")?;
    sheet.set_formula("C7", "=AVERAGE(C2:C5)")?;
    sheet.set_formula("D7", "=AVERAGE(D2:D5)")?;

    sheet.set_formula("B8", "=MAX(B2:B5)")?;
    sheet.set_formula("C8", "=MAX(C2:C5)")?;
    sheet.set_formula("D8", "=MAX(D2:D5)")?;

    sheet.set_formula("B9", "=MIN(B2:B5)")?;
    sheet.set_formula("C9", "=MIN(C2:C5)")?;
    sheet.set_formula("D9", "=MIN(D2:D5)")?;

    // Overall class average
    sheet.set_formula("E7", "=AVERAGE(E2:E5)")?;

    sheet.evaluate_formulas()?;

    // Print grade report
    println!("Grade Report:");
    for row in 1..=5 {
        if row == 1 {
            println!("Student    Average   Grade");
            println!("--------   -------   -----");
        } else {
            let name = sheet.get(row, 0)?;
            let avg = sheet.get(row, 4)?.as_float().unwrap_or(0.0);
            let grade = sheet.get(row, 5)?;
            println!("{:10} {:6.1}    {}", name.as_str(), avg, grade.as_str());
        }
    }

    println!("\nClass Statistics:");
    println!(
        "Test 1: Avg={:.1}, Max={:.0}, Min={:.0}",
        sheet.get_a1("B7")?.as_float().unwrap_or(0.0),
        sheet.get_a1("B8")?.as_float().unwrap_or(0.0),
        sheet.get_a1("B9")?.as_float().unwrap_or(0.0)
    );
    println!(
        "Overall Class Average: {:.1}",
        sheet.get_a1("E7")?.as_float().unwrap_or(0.0)
    );
    println!();

    Ok(())
}

/// Example: String Manipulation
fn string_manipulation_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("4. String Manipulation");
    println!("----------------------");

    let mut sheet = Sheet::from_data(vec![
        row!["First Name", "Last Name", "Full Name", "Email", "Username"],
        row!["john", "doe", "", "", ""],
        row!["jane", "smith", "", "", ""],
        row!["bob", "wilson", "", "", ""],
    ]);

    for row in 2..=4 {
        // Create full name with proper capitalization
        sheet.set_formula(
            &format!("C{}", row),
            &format!("=CONCATENATE(UPPER(LEFT(A{},1)), LOWER(RIGHT(A{},LEN(A{})-1)), \" \", UPPER(LEFT(B{},1)), LOWER(RIGHT(B{},LEN(B{})-1)))", 
                row, row, row, row, row, row),
        )?;

        // Generate email
        sheet.set_formula(
            &format!("D{}", row),
            &format!(
                "=CONCATENATE(LOWER(A{}), \".\", LOWER(B{}), \"@company.com\")",
                row, row
            ),
        )?;

        // Create username (first initial + last name)
        sheet.set_formula(
            &format!("E{}", row),
            &format!("=CONCATENATE(LEFT(A{},1), LOWER(B{}))", row, row),
        )?;
    }

    sheet.evaluate_formulas()?;

    // Print results
    println!("User Information:");
    println!(
        "{:<12} {:<12} {:<20} {:<25} {:<10}",
        "First", "Last", "Full Name", "Email", "Username"
    );
    println!("{}", "-".repeat(79));

    for row in 2..=4 {
        let first = sheet.get(row, 0)?;
        let last = sheet.get(row, 1)?;
        let full = sheet.get(row, 2)?;
        let email = sheet.get(row, 3)?;
        let username = sheet.get(row, 4)?;

        println!(
            "{:<12} {:<12} {:<20} {:<25} {:<10}",
            first.as_str(),
            last.as_str(),
            full.as_str(),
            email.as_str(),
            username.as_str()
        );
    }
    println!();

    Ok(())
}

/// Example: Conditional Logic
fn conditional_logic_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("5. Conditional Logic");
    println!("--------------------");

    let mut sheet = Sheet::from_data(vec![
        row![
            "Product",
            "Stock",
            "Reorder Level",
            "Status",
            "Order Quantity"
        ],
        row!["Widget A", 50, 100, "", 0],
        row!["Widget B", 150, 100, "", 0],
        row!["Widget C", 25, 50, "", 0],
        row!["Widget D", 200, 150, "", 0],
    ]);

    for row in 2..=5 {
        // Check stock status
        sheet.set_formula(
            &format!("D{}", row),
            &format!(
                "=IF(B{}<C{}, \"REORDER\", IF(B{}<C{}*1.5, \"LOW\", \"OK\"))",
                row, row, row, row
            ),
        )?;

        // Calculate order quantity if needed
        sheet.set_formula(
            &format!("E{}", row),
            &format!("=IF(B{}<C{}, C{}*2-B{}, 0)", row, row, row, row),
        )?;
    }

    sheet.evaluate_formulas()?;

    // Print inventory status
    println!("Inventory Status Report:");
    println!(
        "{:<12} {:>8} {:>14} {:<10} {:>15}",
        "Product", "Stock", "Reorder Level", "Status", "Order Quantity"
    );
    println!("{}", "-".repeat(65));

    for row in 2..=5 {
        let product = sheet.get(row, 0)?;
        let stock = sheet.get(row, 1)?.as_int().unwrap_or(0);
        let reorder = sheet.get(row, 2)?.as_int().unwrap_or(0);
        let status = sheet.get(row, 3)?;
        let order_qty = sheet.get(row, 4)?.as_float().unwrap_or(0.0) as i64;

        println!(
            "{:<12} {:>8} {:>14} {:<10} {:>15}",
            product.as_str(),
            stock,
            reorder,
            status.as_str(),
            if order_qty > 0 {
                order_qty.to_string()
            } else {
                "-".to_string()
            }
        );
    }
    println!();

    Ok(())
}

/// Example: Dynamic Spreadsheet with Auto-recalculation
fn dynamic_spreadsheet_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("6. Dynamic Spreadsheet with Auto-recalculation");
    println!("-----------------------------------------------");

    let mut sheet = Sheet::from_data(vec![
        row!["Metric", "Q1", "Q2", "Q3", "Q4", "Total", "Average"],
        row!["Sales", 1000, 1200, 1500, 1800, 0, 0],
        row!["Costs", 600, 700, 800, 900, 0, 0],
        row!["Profit", 0, 0, 0, 0, 0, 0],
    ]);

    // Set up profit calculations
    sheet.set_formula("B4", "=B2-B3")?;
    sheet.set_formula("C4", "=C2-C3")?;
    sheet.set_formula("D4", "=D2-D3")?;
    sheet.set_formula("E4", "=E2-E3")?;

    // Set up totals
    sheet.set_formula("F2", "=SUM(B2:E2)")?;
    sheet.set_formula("F3", "=SUM(B3:E3)")?;
    sheet.set_formula("F4", "=SUM(B4:E4)")?;

    // Set up averages
    sheet.set_formula("G2", "=AVERAGE(B2:E2)")?;
    sheet.set_formula("G3", "=AVERAGE(B3:E3)")?;
    sheet.set_formula("G4", "=AVERAGE(B4:E4)")?;

    println!("Initial values:");
    sheet.evaluate_formulas()?;
    print_quarterly_data(&sheet)?;

    // Simulate Q2 adjustment
    println!("\nAfter Q2 sales increase to 1500:");
    sheet.set_a1("C2", 1500)?;
    sheet.evaluate_formulas()?;
    print_quarterly_data(&sheet)?;

    // Simulate cost reduction in Q4
    println!("\nAfter Q4 cost reduction to 750:");
    sheet.set_a1("E3", 750)?;
    sheet.evaluate_formulas()?;
    print_quarterly_data(&sheet)?;

    Ok(())
}

/// Helper function to print quarterly data
fn print_quarterly_data(sheet: &Sheet) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{:<8} {:>6} {:>6} {:>6} {:>6} {:>8} {:>8}",
        "Metric", "Q1", "Q2", "Q3", "Q4", "Total", "Average"
    );
    println!("{}", "-".repeat(50));

    for row in 2..=4 {
        let metric = sheet.get(row, 0)?;
        print!("{:<8}", metric.as_str());
        for col in 1..=6 {
            let value = sheet.get(row, col)?.as_float().unwrap_or(0.0);
            print!(" {:>7.0}", value);
        }
        println!();
    }

    Ok(())
}
