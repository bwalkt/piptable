//! Demo script to test Sheet/Book module (Issues #65, #66, #79)
//!
//! Run with: cargo run --example sheet_demo -p piptable-sheet

use piptable_sheet::{Book, Sheet, XlsxReadOptions};
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use indexmap::IndexMap;
    use piptable_sheet::CellValue;

    println!("=== Sheet/Book Module Demo ===\n");

    // Use platform-appropriate temp directory
    let temp_dir = env::temp_dir();

    // =========================================================================
    // Issue #66: CSV Support
    // =========================================================================
    println!("--- Issue #66: CSV Support ---\n");

    // Create a sheet from data
    let mut sheet = Sheet::from_data(vec![
        vec!["Name", "Age", "City"],
        vec!["Alice", "30", "NYC"],
        vec!["Bob", "25", "LA"],
        vec!["Charlie", "35", "Chicago"],
    ]);

    println!(
        "Created sheet with {} rows, {} cols",
        sheet.row_count(),
        sheet.col_count()
    );

    // Name columns by first row
    sheet.name_columns_by_row(0)?;
    println!("Column names: {:?}", sheet.column_names());

    // Access by column name
    let ages = sheet.column_by_name("Age")?;
    println!("Ages column: {:?}", ages);

    // Filter rows (keep age > 25)
    sheet.filter_rows(|_idx, row| {
        row.get(1)
            .and_then(|c: &CellValue| c.as_str().parse::<i32>().ok())
            .map(|age| age > 25)
            .unwrap_or(false)
    });
    println!("After filter (age > 25): {} rows", sheet.row_count());

    // Save to CSV
    let csv_path = temp_dir.join("sheet_demo.csv");
    sheet.save_as_csv(&csv_path)?;
    println!("Saved to: {}", csv_path.display());

    // Load from CSV
    let loaded = Sheet::from_csv(&csv_path)?;
    println!(
        "Loaded back: {} rows, {} cols",
        loaded.row_count(),
        loaded.col_count()
    );

    // Convert to CSV string
    let csv_string = sheet.to_csv_string()?;
    println!("\nCSV output:\n{}", csv_string);

    // =========================================================================
    // Issue #65: xlsx Support
    // =========================================================================
    println!("\n--- Issue #65: xlsx Support ---\n");

    // Create a sheet with various types
    let mut typed_sheet = Sheet::new();
    typed_sheet.row_append(vec!["Product", "Price", "InStock", "Rating"])?;
    typed_sheet.row_append(vec!["Widget", "19.99", "true", "4.5"])?;
    typed_sheet.row_append(vec!["Gadget", "29.99", "false", "3.8"])?;
    typed_sheet.row_append(vec!["Gizmo", "9.99", "true", "4.9"])?;

    // Save to xlsx
    let xlsx_path = temp_dir.join("sheet_demo.xlsx");
    typed_sheet.save_as_xlsx(&xlsx_path)?;
    println!("Saved xlsx to: {}", xlsx_path.display());

    // Load from xlsx
    let xlsx_loaded = Sheet::from_xlsx(&xlsx_path)?;
    println!(
        "Loaded xlsx: {} rows, {} cols",
        xlsx_loaded.row_count(),
        xlsx_loaded.col_count()
    );

    // Load with headers option
    let xlsx_with_headers =
        Sheet::from_xlsx_with_options(&xlsx_path, XlsxReadOptions::default().with_headers(true))?;
    println!(
        "With headers - column names: {:?}",
        xlsx_with_headers.column_names()
    );

    // =========================================================================
    // Book operations (multiple sheets)
    // =========================================================================
    println!("\n--- Book Operations ---\n");

    let mut book = Book::new();

    // Add sheets
    let sales = Sheet::from_data(vec![
        vec!["Month", "Revenue"],
        vec!["Jan", "10000"],
        vec!["Feb", "12000"],
        vec!["Mar", "15000"],
    ]);

    let expenses = Sheet::from_data(vec![
        vec!["Category", "Amount"],
        vec!["Rent", "2000"],
        vec!["Utilities", "500"],
        vec!["Salaries", "8000"],
    ]);

    book.add_sheet("Sales", sales)?;
    book.add_sheet("Expenses", expenses)?;

    println!(
        "Book has {} sheets: {:?}",
        book.sheet_count(),
        book.sheet_names()
    );

    // Save book to xlsx
    let book_xlsx_path = temp_dir.join("book_demo.xlsx");
    book.save_as_xlsx(&book_xlsx_path)?;
    println!("Saved book to: {}", book_xlsx_path.display());

    // Load book from xlsx
    let loaded_book = Book::from_xlsx(&book_xlsx_path)?;
    println!("Loaded book: {} sheets", loaded_book.sheet_count());

    // Get sheet names without loading data
    let sheet_names = Book::xlsx_sheet_names(&book_xlsx_path)?;
    println!("Sheet names in file: {:?}", sheet_names);

    // Access specific sheet
    let sales_sheet = loaded_book.get_sheet("Sales")?;
    println!("Sales sheet: {} rows", sales_sheet.row_count());

    // =========================================================================
    // Issue #79: to_records / from_records
    // =========================================================================
    println!("\n--- Issue #79: to_records / from_records ---\n");

    // Create a sheet with named columns
    let mut records_sheet = Sheet::from_data(vec![
        vec!["id", "name", "score"],
        vec!["1", "Alice", "95"],
        vec!["2", "Bob", "87"],
        vec!["3", "Charlie", "92"],
    ]);
    records_sheet.name_columns_by_row(0)?;

    // Convert to records (list of dictionaries)
    let records = records_sheet.to_records();
    println!("to_records() output:");
    if let Some(recs) = &records {
        for (i, rec) in recs.iter().enumerate() {
            println!("  Row {}: {:?}", i, rec);
        }
    }

    // Create a sheet from records
    let mut rec1 = IndexMap::new();
    rec1.insert(
        "product".to_string(),
        CellValue::String("Widget".to_string()),
    );
    rec1.insert("price".to_string(), CellValue::Float(19.99));
    rec1.insert("qty".to_string(), CellValue::Int(100));

    let mut rec2 = IndexMap::new();
    rec2.insert(
        "product".to_string(),
        CellValue::String("Gadget".to_string()),
    );
    rec2.insert("price".to_string(), CellValue::Float(29.99));
    rec2.insert("qty".to_string(), CellValue::Int(50));

    let from_recs = Sheet::from_records(vec![rec1, rec2])?;
    println!("\nfrom_records() created sheet:");
    println!("  Columns: {:?}", from_recs.column_names());
    println!("  Rows: {}", from_recs.row_count());
    println!("  CSV:\n{}", from_recs.to_csv_string()?);

    // =========================================================================
    // Issue #78: JSON, JSONL, and TOON Support
    // =========================================================================
    println!("\n--- Issue #78: JSON, JSONL, and TOON Support ---\n");

    // Create a sheet for JSON/JSONL/TOON demo
    let mut data_sheet = Sheet::from_data(vec![
        vec!["name", "age", "city"],
        vec!["Alice", "30", "NYC"],
        vec!["Bob", "25", "LA"],
    ]);
    data_sheet.name_columns_by_row(0)?;

    // JSON output
    let json = data_sheet.to_json_string_pretty()?;
    println!("JSON output:");
    println!("{}", json);

    // JSONL output
    let jsonl = data_sheet.to_jsonl_string()?;
    println!("\nJSONL output:");
    println!("{}", jsonl);

    // TOON output (Token-Oriented Object Notation - LLM friendly)
    let toon = data_sheet.to_toon_string()?;
    println!("TOON output:");
    println!("{}", toon);

    // Roundtrip: JSON -> Sheet -> JSON
    let json_sheet = Sheet::from_json_str(&data_sheet.to_json_string()?)?;
    println!("JSON roundtrip: {} rows", json_sheet.row_count());

    // Roundtrip: JSONL -> Sheet -> JSONL
    let jsonl_sheet = Sheet::from_jsonl_str(&data_sheet.to_jsonl_string()?)?;
    println!("JSONL roundtrip: {} rows", jsonl_sheet.row_count());

    // Roundtrip: TOON -> Sheet -> TOON
    let toon_sheet = Sheet::from_toon_str(&data_sheet.to_toon_string()?)?;
    println!("TOON roundtrip: {} rows", toon_sheet.row_count());

    // Save to files
    let json_path = temp_dir.join("demo.json");
    let jsonl_path = temp_dir.join("demo.jsonl");
    let toon_path = temp_dir.join("demo.toon");

    data_sheet.save_as_json_pretty(&json_path)?;
    data_sheet.save_as_jsonl(&jsonl_path)?;
    data_sheet.save_as_toon(&toon_path)?;
    println!("\nSaved to:");
    println!("  JSON:  {}", json_path.display());
    println!("  JSONL: {}", jsonl_path.display());
    println!("  TOON:  {}", toon_path.display());

    println!("\n=== Demo Complete ===");
    Ok(())
}
