//! Demo script to test Sheet/Book module (Issues #65, #66)
//!
//! Run with: cargo run --example sheet_demo

use piptable_sheet::{Book, CsvOptions, Sheet, XlsxReadOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Sheet/Book Module Demo ===\n");

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

    println!("Created sheet with {} rows, {} cols", sheet.row_count(), sheet.col_count());

    // Name columns by first row
    sheet.name_columns_by_row(0)?;
    println!("Column names: {:?}", sheet.column_names());

    // Access by column name
    let ages = sheet.column_by_name("Age")?;
    println!("Ages column: {:?}", ages);

    // Filter rows (keep age > 25)
    sheet.filter_rows(|row| {
        row.get(1)
            .and_then(|c| c.as_str().parse::<i32>().ok())
            .map(|age| age > 25)
            .unwrap_or(false)
    });
    println!("After filter (age > 25): {} rows", sheet.row_count());

    // Save to CSV
    let csv_path = "/tmp/sheet_demo.csv";
    sheet.save_as_csv(csv_path)?;
    println!("Saved to: {}", csv_path);

    // Load from CSV
    let loaded = Sheet::from_csv(csv_path)?;
    println!("Loaded back: {} rows, {} cols", loaded.row_count(), loaded.col_count());

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
    let xlsx_path = "/tmp/sheet_demo.xlsx";
    typed_sheet.save_as_xlsx(xlsx_path)?;
    println!("Saved xlsx to: {}", xlsx_path);

    // Load from xlsx
    let xlsx_loaded = Sheet::from_xlsx(xlsx_path)?;
    println!("Loaded xlsx: {} rows, {} cols", xlsx_loaded.row_count(), xlsx_loaded.col_count());

    // Load with headers option
    let xlsx_with_headers = Sheet::from_xlsx_with_options(
        xlsx_path,
        XlsxReadOptions::default().with_headers(true),
    )?;
    println!("With headers - column names: {:?}", xlsx_with_headers.column_names());

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

    println!("Book has {} sheets: {:?}", book.sheet_count(), book.sheet_names());

    // Save book to xlsx
    let book_xlsx_path = "/tmp/book_demo.xlsx";
    book.save_as_xlsx(book_xlsx_path)?;
    println!("Saved book to: {}", book_xlsx_path);

    // Load book from xlsx
    let loaded_book = Book::from_xlsx(book_xlsx_path)?;
    println!("Loaded book: {} sheets", loaded_book.sheet_count());

    // Get sheet names without loading data
    let sheet_names = Book::xlsx_sheet_names(book_xlsx_path)?;
    println!("Sheet names in file: {:?}", sheet_names);

    // Access specific sheet
    let sales_sheet = loaded_book.get_sheet("Sales")?;
    println!("Sales sheet: {} rows", sales_sheet.row_count());

    println!("\n=== Demo Complete ===");
    Ok(())
}
