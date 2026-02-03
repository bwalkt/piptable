//! Sql tests for the PipTable interpreter.

#![allow(clippy::needless_raw_string_hashes)]

/// Shared test helpers.
mod common {
    include!("common_impl.txt");
}
use common::*;

use piptable_core::Value;

#[tokio::test]
async fn test_simple_query() {
    let (interp, _) = run_script("dim result = query(SELECT 1 + 1 as sum)").await;
    match interp.get_var("result").await {
        Some(Value::Table(batches)) => {
            assert!(!batches.is_empty(), "Query should return results");
            let batch = &batches[0];
            assert_eq!(batch.num_columns(), 1);
            let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
            assert_eq!(total_rows, 1);
        }
        _ => panic!("Expected Table result"),
    }
}

#[tokio::test]
async fn test_query_multiple_columns() {
    let (interp, _) = run_script("dim result = query(SELECT 1 as a, 2 as b, 3 as c)").await;
    match interp.get_var("result").await {
        Some(Value::Table(batches)) => {
            assert!(!batches.is_empty());
            let batch = &batches[0];
            assert_eq!(batch.num_columns(), 3);
            let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
            assert_eq!(total_rows, 1);
        }
        _ => panic!("Expected table"),
    }
}

// TODO: SQL variable tests moved to lib.rs due to parser limitations
// The parser currently has issues with certain SQL aliases and keywords
// Tests for SQL queries on in-memory variables are in src/lib.rs:
// - test_sql_query_on_table_variable
// - test_sql_join_on_table_variables

// TODO: WHERE clause parsing has issues with the current grammar
/// Verifies that a CSV-backed SQL query filters rows correctly using a WHERE clause.
///
/// Creates a temporary CSV file, runs a `SELECT * FROM '<path>' WHERE value > 150` query
/// through the interpreter, and asserts the resulting table contains the expected two rows.
///
/// # Examples
///
/// ```
/// let csv_content = "id,name,value\n1,foo,100\n2,bar,200\n3,baz,300";
/// let file = create_temp_csv(csv_content);
/// let path = file.path().to_string_lossy().replace('\\', "/");
/// let script = format!(r#"dim result = query(SELECT * FROM '{}' WHERE value > 150)"#, path);
/// let (interp, _) = run_script(&script).await;
/// match interp.get_var("result").await {
///     Some(Value::Table(batches)) => {
///         let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
///         assert_eq!(total_rows, 2);
///     }
///     _ => panic!("Expected table"),
/// }
/// ```
#[tokio::test]
#[ignore = "SQL WHERE clause parsing issue"]
async fn test_csv_query() {
    let csv_content = "id,name,value\n1,foo,100\n2,bar,200\n3,baz,300";
    let file = create_temp_csv(csv_content);
    let path = file.path().to_string_lossy().replace('\\', "/");
    let path = path.replace('\'', "''"); // Escape single quotes for SQL

    let script = format!(
        r#"dim result = query(SELECT * FROM '{}' WHERE value > 150)"#,
        path
    );
    let (interp, _) = run_script(&script).await;

    match interp.get_var("result").await {
        Some(Value::Table(batches)) => {
            let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
            assert_eq!(total_rows, 2); // bar and baz
        }
        _ => panic!("Expected table"),
    }
}

// TODO: GROUP BY parsing has issues with the current grammar
/// Verifies that a CSV query with `GROUP BY` and `SUM` produces a table with one row per group.
///
/// This integration test writes a temporary CSV, runs a SQL-like query that groups by `category`
/// and sums `amount`, and asserts the resulting table contains two aggregated rows.
///
/// # Examples
///
/// ```
/// // Creates a temp CSV and runs: SELECT category, SUM(amount) as total FROM '<path>' GROUP BY category
/// let csv_content = "category,amount\nA,100\nB,200\nA,150\nB,50";
/// let file = create_temp_csv(csv_content);
/// let path = file.path().to_string_lossy().replace('\\', "/");
/// let script = format!(r#"dim result = query(SELECT category, SUM(amount) as total FROM '{}' GROUP BY category ORDER BY category)"#, path);
/// let (interp, _) = run_script(&script).await;
/// match interp.get_var("result").await {
///     Some(Value::Table(batches)) => {
///         let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
///         assert_eq!(total_rows, 2);
///     }
///     _ => panic!("Expected table"),
/// }
/// ```
#[tokio::test]
#[ignore = "SQL GROUP BY parsing issue"]
async fn test_csv_aggregation() {
    let csv_content = "category,amount\nA,100\nB,200\nA,150\nB,50";
    let file = create_temp_csv(csv_content);
    let path = file.path().to_string_lossy().replace('\\', "/");
    let path = path.replace('\'', "''"); // Escape single quotes for SQL

    let script = format!(
        r#"dim result = query(SELECT category, SUM(amount) as total FROM '{}' GROUP BY category ORDER BY category)"#,
        path
    );
    let (interp, _) = run_script(&script).await;

    match interp.get_var("result").await {
        Some(Value::Table(batches)) => {
            let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
            assert_eq!(total_rows, 2);
        }
        _ => panic!("Expected table"),
    }
}

// TODO: JOIN parsing has issues with the current grammar
/// Integration test that verifies a SQL JOIN across two CSV files produces the expected combined rows.
///
/// This test creates two temporary CSV files (users and orders), runs a `query` that joins them on user ID,
/// and asserts the resulting table contains the combined rows (three total for the sample data).
///
/// # Examples
///
/// ```no_run
/// // Create CSV files, run the JOIN query, and check the combined row count.
/// let users_csv = "id,name\n1,alice\n2,bob";
/// let orders_csv = "user_id,amount\n1,100\n1,200\n2,50";
/// let users_file = create_temp_csv(users_csv);
/// let orders_file = create_temp_csv(orders_csv);
/// let users_path = users_file.path().to_string_lossy().replace('\\', "/");
/// let orders_path = orders_file.path().to_string_lossy().replace('\\', "/");
/// let script = format!(r#"dim result = query(
///     SELECT u.name, o.amount
///     FROM '{}' as u
///     JOIN '{}' as o ON u.id = o.user_id
///     ORDER BY u.name, o.amount
/// )"#, users_path, orders_path);
/// let (interp, _) = run_script(&script).await;
/// match interp.get_var("result").await {
///     Some(Value::Table(batches)) => {
///         let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
///         assert_eq!(total_rows, 3);
///     }
///     _ => panic!("Expected table"),
/// }
/// ```
#[tokio::test]
#[ignore = "SQL JOIN parsing issue"]
async fn test_csv_join() {
    let users_csv = "id,name\n1,alice\n2,bob";
    let orders_csv = "user_id,amount\n1,100\n1,200\n2,50";

    let users_file = create_temp_csv(users_csv);
    let orders_file = create_temp_csv(orders_csv);

    let users_path = users_file.path().to_string_lossy().replace('\\', "/");
    let users_path = users_path.replace('\'', "''"); // Escape single quotes for SQL
    let orders_path = orders_file.path().to_string_lossy().replace('\\', "/");
    let orders_path = orders_path.replace('\'', "''"); // Escape single quotes for SQL

    let script = format!(
        r#"dim result = query(
            SELECT u.name, o.amount
            FROM '{}' as u
            JOIN '{}' as o ON u.id = o.user_id
            ORDER BY u.name, o.amount
        )"#,
        users_path, orders_path
    );
    let (interp, _) = run_script(&script).await;

    match interp.get_var("result").await {
        Some(Value::Table(batches)) => {
            let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
            assert_eq!(total_rows, 3);
        }
        _ => panic!("Expected table"),
    }
}

#[tokio::test]
async fn test_xlsx_query() {
    use rust_xlsxwriter::Workbook;
    use tempfile::NamedTempFile;

    // Create a temporary XLSX file with .xlsx extension
    let file = NamedTempFile::with_suffix(".xlsx")
        .expect("Failed to create temp file")
        .into_temp_path();
    let path = file.to_path_buf();

    // Create XLSX with sample data
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    // Add headers
    worksheet.write_string(0, 0, "id").unwrap();
    worksheet.write_string(0, 1, "name").unwrap();
    worksheet.write_string(0, 2, "value").unwrap();

    // Add data rows
    worksheet.write_number(1, 0, 1).unwrap();
    worksheet.write_string(1, 1, "foo").unwrap();
    worksheet.write_number(1, 2, 100).unwrap();

    worksheet.write_number(2, 0, 2).unwrap();
    worksheet.write_string(2, 1, "bar").unwrap();
    worksheet.write_number(2, 2, 200).unwrap();

    worksheet.write_number(3, 0, 3).unwrap();
    worksheet.write_string(3, 1, "baz").unwrap();
    worksheet.write_number(3, 2, 300).unwrap();

    workbook.save(path.to_str().unwrap()).unwrap();

    // Test SELECT query on XLSX file - use import then query
    let path_str = path.to_string_lossy().replace('\\', "/");
    let script = format!(
        r#"
        import "{}" into xlsx_data
        dim result = query(SELECT * FROM xlsx_data)
    "#,
        path_str
    );
    let (interp, _) = run_script(&script).await;

    match interp.get_var("result").await {
        Some(Value::Table(batches)) => {
            assert!(!batches.is_empty());
            let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
            assert_eq!(total_rows, 3); // foo, bar, baz
            assert_eq!(batches[0].num_columns(), 3); // id, name, value
        }
        _ => panic!("Expected table"),
    }
}

#[tokio::test]
async fn test_xlsx_query_with_filter() {
    use rust_xlsxwriter::Workbook;
    use tempfile::NamedTempFile;

    // Create a temporary XLSX file with .xlsx extension
    let file = NamedTempFile::with_suffix(".xlsx")
        .expect("Failed to create temp file")
        .into_temp_path();
    let path = file.to_path_buf();

    // Create XLSX with sample data
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    // Add headers
    worksheet.write_string(0, 0, "category").unwrap();
    worksheet.write_string(0, 1, "amount").unwrap();

    // Add data rows
    worksheet.write_string(1, 0, "A").unwrap();
    worksheet.write_number(1, 1, 100).unwrap();

    worksheet.write_string(2, 0, "B").unwrap();
    worksheet.write_number(2, 1, 200).unwrap();

    worksheet.write_string(3, 0, "A").unwrap();
    worksheet.write_number(3, 1, 150).unwrap();

    worksheet.write_string(4, 0, "B").unwrap();
    worksheet.write_number(4, 1, 50).unwrap();

    workbook.save(path.to_str().unwrap()).unwrap();

    // Test SELECT with simple filter on XLSX file - use import then query
    let path_str = path.to_string_lossy().replace('\\', "/");
    let script = format!(
        r#"
        import "{}" into xlsx_data
        dim result = query(SELECT * FROM xlsx_data WHERE amount > 100)
    "#,
        path_str
    );
    let (interp, _) = run_script(&script).await;

    match interp.get_var("result").await {
        Some(Value::Table(batches)) => {
            assert!(!batches.is_empty());
            let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
            assert_eq!(total_rows, 2); // Rows with amount > 100: B,200 and A,150
        }
        _ => panic!("Expected table"),
    }
}

#[tokio::test]
async fn test_xlsx_join_with_csv() {
    use rust_xlsxwriter::Workbook;
    use tempfile::NamedTempFile;

    // Create a temporary XLSX file for users with .xlsx extension
    let xlsx_file = NamedTempFile::with_suffix(".xlsx")
        .expect("Failed to create temp file")
        .into_temp_path();
    let xlsx_path = xlsx_file.to_path_buf();

    // Create XLSX with user data
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    worksheet.write_string(0, 0, "id").unwrap();
    worksheet.write_string(0, 1, "username").unwrap();

    worksheet.write_number(1, 0, 1).unwrap();
    worksheet.write_string(1, 1, "alice").unwrap();

    worksheet.write_number(2, 0, 2).unwrap();
    worksheet.write_string(2, 1, "bob").unwrap();

    workbook.save(xlsx_path.to_str().unwrap()).unwrap();

    // Create a CSV file for orders
    let orders_csv = "user_id,amount\n1,100\n1,200\n2,50";
    let csv_file = create_temp_csv(orders_csv);

    let xlsx_str = xlsx_path.to_string_lossy().replace('\\', "/");
    let csv_str = csv_file.path().to_string_lossy().replace('\\', "/");

    // Test JOIN between XLSX and CSV - use import then query
    let script = format!(
        r#"
        import "{}" into xlsx_users
        import "{}" into csv_orders
        dim result = query(
            SELECT u.username, o.amount
            FROM xlsx_users as u
            JOIN csv_orders as o ON u.id = o.user_id
            ORDER BY u.username, o.amount
        )"#,
        xlsx_str, csv_str
    );
    let (interp, _) = run_script(&script).await;

    match interp.get_var("result").await {
        Some(Value::Table(batches)) => {
            assert!(!batches.is_empty());
            let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
            assert_eq!(total_rows, 3); // alice:100, alice:200, bob:50
        }
        _ => panic!("Expected table"),
    }
}
