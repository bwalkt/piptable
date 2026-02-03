//! Interactive Formula Playground
//!
//! This example demonstrates an interactive REPL-style playground for testing
//! formulas in piptable-sheet.

use piptable_sheet::{CellValue, Sheet};
use std::io::{self, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔════════════════════════════════════════════════════════════════════╗");
    println!("║           Piptable Formula Playground - Interactive REPL          ║");
    println!("╚════════════════════════════════════════════════════════════════════╝");
    println!();
    println!("Commands:");
    println!("  set <cell> <value>    - Set a cell value (e.g., 'set A1 100')");
    println!("  formula <cell> <expr> - Set a formula (e.g., 'formula C1 =A1+B1')");
    println!("  eval                  - Evaluate all formulas");
    println!("  show                  - Display the spreadsheet");
    println!("  clear                 - Clear the spreadsheet");
    println!("  demo <name>           - Load a demo (basic, financial, stats, lookup)");
    println!("  help                  - Show this help message");
    println!("  quit                  - Exit the playground");
    println!();

    let mut sheet = create_default_sheet();
    println!("Initial spreadsheet (10x10 grid):");
    display_sheet(&sheet)?;

    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        let parts: Vec<&str> = input.split_whitespace().collect();
        match parts[0] {
            "quit" | "exit" => {
                println!("Goodbye!");
                break;
            }
            "help" => {
                print_help();
            }
            "show" => {
                display_sheet(&sheet)?;
            }
            "clear" => {
                sheet = create_default_sheet();
                println!("Spreadsheet cleared.");
            }
            "set" => {
                if parts.len() < 3 {
                    println!("Usage: set <cell> <value>");
                    continue;
                }
                handle_set_command(&mut sheet, parts[1], &parts[2..].join(" "));
            }
            "formula" | "f" => {
                if parts.len() < 3 {
                    println!("Usage: formula <cell> <expression>");
                    continue;
                }
                handle_formula_command(&mut sheet, parts[1], &parts[2..].join(" "));
            }
            "eval" | "e" => match sheet.evaluate_formulas() {
                Ok(_) => println!("Formulas evaluated successfully."),
                Err(e) => println!("Error evaluating formulas: {}", e),
            },
            "demo" => {
                if parts.len() < 2 {
                    println!("Available demos: basic, financial, stats, lookup");
                    continue;
                }
                match load_demo(parts[1]) {
                    Ok(demo_sheet) => sheet = demo_sheet,
                    Err(e) => println!("Error loading demo: {}", e),
                }
            }
            _ => {
                println!(
                    "Unknown command: {}. Type 'help' for available commands.",
                    parts[0]
                );
            }
        }
    }

    Ok(())
}

fn create_default_sheet() -> Sheet {
    let mut data = Vec::new();
    for _ in 0..10 {
        let row: Vec<CellValue> = (0..10).map(|_| CellValue::Null).collect();
        data.push(row);
    }
    Sheet::from_data(data)
}

fn display_sheet(sheet: &Sheet) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n     A      B      C      D      E      F      G      H      I      J");
    println!("   ┌──────┬──────┬──────┬──────┬──────┬──────┬──────┬──────┬──────┬──────┐");

    for row in 0..10 {
        print!("{:2} │", row + 1);
        for col in 0..10 {
            let value = sheet.get(row, col)?;
            let display = format_cell_value(&value);
            print!("{:^6}│", display);
        }
        println!();

        if row < 9 {
            println!("   ├──────┼──────┼──────┼──────┼──────┼──────┼──────┼──────┼──────┼──────┤");
        }
    }
    println!("   └──────┴──────┴──────┴──────┴──────┴──────┴──────┴──────┴──────┴──────┘");
    println!();

    Ok(())
}

fn format_cell_value(value: &CellValue) -> String {
    match value {
        CellValue::Null => "".to_string(),
        CellValue::Bool(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
        CellValue::Int(i) => format!("{}", i),
        CellValue::Float(f) => {
            if f.fract() == 0.0 && f.abs() < 1e10 {
                format!("{:.0}", f)
            } else {
                format!("{:.2}", f)
            }
        }
        CellValue::String(s) => {
            if s.chars().count() > 6 {
                let preview: String = s.chars().take(5).collect();
                format!("{}…", preview)
            } else {
                s.clone()
            }
        }
        CellValue::Formula(f) => {
            if let Some(cached) = &f.cached {
                format_cell_value(cached)
            } else {
                "=?".to_string()
            }
        }
    }
}

fn handle_set_command(sheet: &mut Sheet, cell: &str, value: &str) {
    // Try to parse as number first
    let value = value.trim();
    let value = value
        .strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .unwrap_or(value);

    let cell_value = if let Ok(int_val) = value.parse::<i64>() {
        CellValue::Int(int_val)
    } else if let Ok(float_val) = value.parse::<f64>() {
        CellValue::Float(float_val)
    } else if value == "true" || value == "TRUE" {
        CellValue::Bool(true)
    } else if value == "false" || value == "FALSE" {
        CellValue::Bool(false)
    } else {
        CellValue::String(value.to_string())
    };

    match sheet.set_a1(cell, cell_value) {
        Ok(_) => println!("Set {} = {}", cell, value),
        Err(e) => println!("Error setting cell: {}", e),
    }
}

fn handle_formula_command(sheet: &mut Sheet, cell: &str, formula: &str) {
    // Ensure formula starts with =
    let formula = if !formula.starts_with('=') {
        format!("={}", formula)
    } else {
        formula.to_string()
    };

    match sheet.set_formula(cell, &formula) {
        Ok(_) => println!("Set formula {} = {}", cell, formula),
        Err(e) => println!("Error setting formula: {}", e),
    }
}

fn print_help() {
    println!("\n=== Formula Playground Help ===");
    println!("\nBasic Commands:");
    println!("  set A1 42             - Set cell A1 to number 42");
    println!("  set B1 \"Hello\"        - Set cell B1 to string \"Hello\"");
    println!("  formula C1 =A1+B1     - Set formula in C1");
    println!("  eval                  - Evaluate all formulas");
    println!("  show                  - Display current sheet");
    println!("\nSupported Formula Functions:");
    println!("  Math: SUM, AVERAGE, MIN, MAX, ABS, ROUND, CEIL, FLOOR, POWER, SQRT");
    println!("  Logic: IF, AND, OR, NOT");
    println!("  String: CONCATENATE, UPPER, LOWER, LEFT, RIGHT, MID, LEN, TRIM");
    println!("  Lookup: VLOOKUP, HLOOKUP, INDEX, MATCH");
    println!("  Stats: COUNT, COUNTA, COUNTIF, STDEV, VAR");
    println!("  Date: DATE, NOW, TODAY, YEAR, MONTH, DAY");
    println!("\nExamples:");
    println!("  formula D1 =SUM(A1:C1)");
    println!("  formula E1 =IF(A1>50,\"High\",\"Low\")");
    println!("  formula F1 =CONCATENATE(A1,\" items\")");
    println!("  formula G1 =AVERAGE(A1:A10)");
    println!();
}

fn load_demo(name: &str) -> Result<Sheet, Box<dyn std::error::Error>> {
    match name {
        "basic" => {
            println!("Loading basic arithmetic demo...");
            let mut sheet = create_default_sheet();

            // Basic arithmetic demo
            sheet.set_a1("A1", 10)?;
            sheet.set_a1("B1", 20)?;
            sheet.set_a1("A2", 5)?;
            sheet.set_a1("B2", 15)?;

            sheet.set_formula("C1", "=A1+B1")?;
            sheet.set_formula("C2", "=A2*B2")?;
            sheet.set_formula("D1", "=C1-C2")?;
            sheet.set_formula("D2", "=SUM(A1:B2)")?;

            sheet.evaluate_formulas()?;

            println!("Demo loaded: A1=10, B1=20, C1=A1+B1, etc.");
            println!("Try 'show' to see the sheet, or modify values with 'set'");

            Ok(sheet)
        }
        "financial" => {
            println!("Loading financial calculations demo...");
            let mut sheet = create_default_sheet();

            // Financial calculations
            sheet.set_a1("A1", "Revenue")?;
            sheet.set_a1("A2", 10000)?;
            sheet.set_a1("A3", 12000)?;
            sheet.set_a1("A4", 15000)?;

            sheet.set_a1("B1", "Costs")?;
            sheet.set_a1("B2", 7000)?;
            sheet.set_a1("B3", 8000)?;
            sheet.set_a1("B4", 9000)?;

            sheet.set_formula("C1", "=\"Profit\"")?;
            sheet.set_formula("C2", "=A2-B2")?;
            sheet.set_formula("C3", "=A3-B3")?;
            sheet.set_formula("C4", "=A4-B4")?;

            sheet.set_formula("D1", "=\"Margin%\"")?;
            sheet.set_formula("D2", "=(C2/A2)*100")?;
            sheet.set_formula("D3", "=(C3/A3)*100")?;
            sheet.set_formula("D4", "=(C4/A4)*100")?;

            sheet.set_a1("A5", "Total")?;
            sheet.set_formula("A6", "=SUM(A2:A4)")?;
            sheet.set_formula("B6", "=SUM(B2:B4)")?;
            sheet.set_formula("C6", "=A6-B6")?;
            sheet.set_formula("D6", "=AVERAGE(D2:D4)")?;

            sheet.evaluate_formulas()?;

            println!("Financial demo loaded with Revenue, Costs, Profit, and Margin calculations");

            Ok(sheet)
        }
        "stats" => {
            println!("Loading statistics demo...");
            let mut sheet = create_default_sheet();

            // Sample data
            for i in 0..10 {
                sheet.set_a1(&format!("A{}", i + 1), (i + 1) * 10)?;
                sheet.set_a1(&format!("B{}", i + 1), 50 + i * 5)?;
            }

            sheet.set_formula("D1", "=\"Mean A\"")?;
            sheet.set_formula("E1", "=AVERAGE(A1:A10)")?;

            sheet.set_formula("D2", "=\"Max A\"")?;
            sheet.set_formula("E2", "=MAX(A1:A10)")?;

            sheet.set_formula("D3", "=\"Min A\"")?;
            sheet.set_formula("E3", "=MIN(A1:A10)")?;

            sheet.set_formula("D4", "=\"Count\"")?;
            sheet.set_formula("E4", "=COUNT(A1:A10)")?;

            sheet.set_formula("D5", "=\"Sum\"")?;
            sheet.set_formula("E5", "=SUM(A1:A10)")?;

            sheet.evaluate_formulas()?;

            println!("Statistics demo loaded with sample data and statistical functions");

            Ok(sheet)
        }
        "lookup" => {
            println!("Loading lookup functions demo...");
            let mut sheet = create_default_sheet();

            // Create a simple lookup table
            sheet.set_a1("A1", "ID")?;
            sheet.set_a1("B1", "Name")?;
            sheet.set_a1("C1", "Score")?;

            sheet.set_a1("A2", 101)?;
            sheet.set_a1("B2", "Alice")?;
            sheet.set_a1("C2", 85)?;

            sheet.set_a1("A3", 102)?;
            sheet.set_a1("B3", "Bob")?;
            sheet.set_a1("C3", 92)?;

            sheet.set_a1("A4", 103)?;
            sheet.set_a1("B4", "Carol")?;
            sheet.set_a1("C4", 78)?;

            // Lookup examples
            sheet.set_a1("E1", "Lookup ID:")?;
            sheet.set_a1("F1", 102)?;

            sheet.set_formula("E2", "=\"Name:\"")?;
            sheet.set_formula("F2", "=VLOOKUP(F1,A2:C4,2,FALSE)")?;

            sheet.set_formula("E3", "=\"Score:\"")?;
            sheet.set_formula("F3", "=VLOOKUP(F1,A2:C4,3,FALSE)")?;

            sheet.set_formula("E5", "=\"Max Score:\"")?;
            sheet.set_formula("F5", "=MAX(C2:C4)")?;

            sheet.set_formula("E6", "=\"Avg Score:\"")?;
            sheet.set_formula("F6", "=AVERAGE(C2:C4)")?;

            sheet.evaluate_formulas()?;

            println!("Lookup demo loaded with VLOOKUP examples");
            println!("Try changing F1 to different IDs (101, 102, 103) and run 'eval'");

            Ok(sheet)
        }
        _ => Err(format!(
            "Unknown demo '{}'. Available demos: basic, financial, stats, lookup",
            name
        )
        .into()),
    }
}
