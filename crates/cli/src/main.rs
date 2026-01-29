//! # piptable-cli
//!
//! Command-line interface for the piptable DSL.

use anyhow::{Context, Result};
use arrow::util::pretty::pretty_format_batches;
use clap::Parser;
use colored::Colorize;
use piptable_core::Value;
use piptable_interpreter::Interpreter;
use piptable_parser::PipParser;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

/// piptable - A VBA-like DSL for data processing
#[derive(Parser)]
#[command(name = "pip")]
#[command(author, version, about = "VBA+SQL DSL for data processing", long_about = None)]
struct Cli {
    /// Script file to execute
    #[arg(value_name = "FILE")]
    file: Option<PathBuf>,

    /// Execute inline script
    #[arg(short = 'e', long = "execute")]
    execute: Option<String>,

    /// Start REPL mode
    #[arg(short = 'i', long = "interactive")]
    interactive: bool,

    /// Output format (json, csv, table)
    #[arg(short = 'f', long = "format", default_value = "table")]
    format: OutputFormat,

    /// Set variable (key=value)
    #[arg(short = 'D', long = "define", value_name = "KEY=VALUE")]
    vars: Vec<String>,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,
}

/// Output format for results.
#[derive(Clone, Copy, Default, clap::ValueEnum)]
enum OutputFormat {
    /// JSON output
    Json,
    /// CSV output
    Csv,
    /// Pretty table output (default)
    #[default]
    Table,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    if cli.verbose {
        tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
            )
            .init();
    }

    // Create interpreter
    let mut interpreter = Interpreter::new();

    // Set variables from CLI
    for var in &cli.vars {
        let (key, value) = var.split_once('=').with_context(|| {
            format!("Invalid variable format: '{var}'. Expected KEY=VALUE format")
        })?;
        interpreter.set_var(key, parse_cli_value(value)).await;
    }

    // Determine execution mode
    if cli.interactive {
        run_repl(&mut interpreter, cli.format).await
    } else if let Some(script) = cli.execute {
        run_script(&mut interpreter, &script, cli.format).await
    } else if let Some(file) = cli.file {
        let source = std::fs::read_to_string(&file)
            .with_context(|| format!("Failed to read file: {}", file.display()))?;
        run_script(&mut interpreter, &source, cli.format).await
    } else {
        // No arguments - show help
        Cli::parse_from(["pip", "--help"]);
        Ok(())
    }
}

/// Parse a CLI value string into a Value.
fn parse_cli_value(s: &str) -> Value {
    // Try to parse as different types
    if s.eq_ignore_ascii_case("null") {
        Value::Null
    } else if s.eq_ignore_ascii_case("true") {
        Value::Bool(true)
    } else if s.eq_ignore_ascii_case("false") {
        Value::Bool(false)
    } else if let Ok(n) = s.parse::<i64>() {
        Value::Int(n)
    } else if let Ok(f) = s.parse::<f64>() {
        Value::Float(f)
    } else {
        Value::String(s.to_string())
    }
}

/// Run a piptable script.
async fn run_script(
    interpreter: &mut Interpreter,
    source: &str,
    format: OutputFormat,
) -> Result<()> {
    let program = PipParser::parse_str(source).map_err(|e| anyhow::anyhow!("{e}"))?;
    let result = interpreter
        .eval(program)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    // Print output buffer
    for line in interpreter.output().await {
        println!("{line}");
    }

    // Print result if not null
    if !result.is_null() {
        print_value(&result, format)?;
    }

    Ok(())
}

/// Run the REPL.
async fn run_repl(interpreter: &mut Interpreter, format: OutputFormat) -> Result<()> {
    println!(
        "{} {} - Interactive Mode",
        "piptable".cyan().bold(),
        env!("CARGO_PKG_VERSION")
    );
    println!(
        "Type {} for help, {} to exit\n",
        ":help".yellow(),
        ":quit".yellow()
    );

    let mut rl = DefaultEditor::new()?;
    let history_path = dirs_history_path();

    // Load history if available
    if let Some(ref path) = history_path {
        let _ = rl.load_history(path);
    }

    loop {
        let prompt = "pip> ".green().bold().to_string();

        match rl.readline(&prompt) {
            Ok(line) => {
                let line = line.trim();

                if line.is_empty() {
                    continue;
                }

                // Add to history
                let _ = rl.add_history_entry(line);

                // Handle REPL commands
                if line.starts_with(':') {
                    match line {
                        ":quit" | ":q" | ":exit" => break,
                        ":help" | ":h" | ":?" => {
                            print_help();
                            continue;
                        }
                        ":vars" => {
                            println!("{}", "Variables not yet implemented".yellow());
                            continue;
                        }
                        ":clear" => {
                            print!("\x1B[2J\x1B[1;1H");
                            continue;
                        }
                        _ => {
                            println!("{} Unknown command: {}", "Error:".red().bold(), line);
                            continue;
                        }
                    }
                }

                // Execute piptable code
                match PipParser::parse_str(line) {
                    Ok(program) => {
                        match interpreter.eval(program).await {
                            Ok(result) => {
                                // Print output buffer
                                for out in interpreter.output().await {
                                    println!("{out}");
                                }

                                // Print result if not null
                                if !result.is_null() {
                                    if let Err(e) = print_value(&result, format) {
                                        println!("{} {e}", "Error:".red().bold());
                                    }
                                }
                            }
                            Err(e) => {
                                println!("{} {e}", "Error:".red().bold());
                            }
                        }
                    }
                    Err(e) => {
                        println!("{} {e}", "Parse error:".red().bold());
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("^C");
                continue;
            }
            Err(ReadlineError::Eof) => {
                println!("Goodbye!");
                break;
            }
            Err(e) => {
                println!("{} {e}", "Error:".red().bold());
                break;
            }
        }
    }

    // Save history
    if let Some(ref path) = history_path {
        let _ = rl.save_history(path);
    }

    Ok(())
}

/// Get the history file path.
fn dirs_history_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|mut p| {
        p.push("piptable");
        let _ = std::fs::create_dir_all(&p);
        p.push("history.txt");
        p
    })
}

/// Print a value in the specified format.
fn print_value(value: &Value, format: OutputFormat) -> Result<()> {
    match value {
        Value::Table(batches) => {
            if batches.is_empty() {
                println!("(empty table)");
                return Ok(());
            }

            match format {
                OutputFormat::Table => {
                    // Clone batches from Arc for pretty printing
                    let batch_clones: Vec<_> = batches.iter().map(|b| (**b).clone()).collect();
                    let formatted = pretty_format_batches(&batch_clones)?;
                    println!("{formatted}");
                }
                OutputFormat::Json => {
                    // Convert table to JSON array of objects
                    let json = table_to_json(batches)?;
                    println!("{}", serde_json::to_string_pretty(&json)?);
                }
                OutputFormat::Csv => {
                    print_table_csv(batches)?;
                }
            }
        }
        Value::Array(items) => match format {
            OutputFormat::Json => {
                let json_items: Result<Vec<_>, _> = items.iter().map(|v| v.to_json()).collect();
                match json_items {
                    Ok(arr) => println!("{}", serde_json::to_string_pretty(&arr)?),
                    Err(e) => println!("{}", e),
                }
            }
            OutputFormat::Csv | OutputFormat::Table => {
                for item in items {
                    println!("{}", format_value(item));
                }
            }
        },
        Value::Object(obj) => match format {
            OutputFormat::Json => {
                let json_obj: Result<serde_json::Map<String, serde_json::Value>, _> = obj
                    .iter()
                    .map(|(k, v)| v.to_json().map(|jv| (k.clone(), jv)))
                    .collect();
                match json_obj {
                    Ok(map) => println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::Value::Object(map))?
                    ),
                    Err(e) => println!("{}", e),
                }
            }
            OutputFormat::Csv | OutputFormat::Table => {
                for (k, v) in obj {
                    println!("{}: {}", k, format_value(v));
                }
            }
        },
        _ => {
            println!("{}", format_value(value));
        }
    }

    Ok(())
}

/// Format a simple value for display.
fn format_value(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Int(n) => n.to_string(),
        Value::Float(f) => f.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(items) => {
            let formatted: Vec<_> = items.iter().map(format_value).collect();
            format!("[{}]", formatted.join(", "))
        }
        Value::Object(obj) => {
            let formatted: Vec<_> = obj
                .iter()
                .map(|(k, v)| format!("{k}: {}", format_value(v)))
                .collect();
            format!("{{{}}}", formatted.join(", "))
        }
        Value::Table(batches) => {
            let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
            format!("<Table: {} rows>", total_rows)
        }
        Value::Function { name, params, .. } => {
            let param_list = params
                .iter()
                .map(|param| param.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            format!("<Function: {}({})>", name, param_list)
        }
        Value::Sheet(sheet) => {
            format!("<Sheet: {}x{}>", sheet.row_count(), sheet.col_count())
        }
        Value::Lambda { params, .. } => {
            format!("<Lambda: |{}|>", params.join(", "))
        }
    }
}

/// Convert Arrow batches to JSON.
fn table_to_json(
    batches: &[std::sync::Arc<arrow::array::RecordBatch>],
) -> Result<Vec<serde_json::Value>> {
    let mut rows = Vec::new();

    for batch in batches {
        let schema = batch.schema();

        for row_idx in 0..batch.num_rows() {
            let mut row = serde_json::Map::new();

            for (col_idx, field) in schema.fields().iter().enumerate() {
                let col = batch.column(col_idx);
                let value = array_value_to_json(col.as_ref(), row_idx);
                row.insert(field.name().clone(), value);
            }

            rows.push(serde_json::Value::Object(row));
        }
    }

    Ok(rows)
}

/// Convert an Arrow array value at index to JSON.
fn array_value_to_json(array: &dyn arrow::array::Array, idx: usize) -> serde_json::Value {
    use arrow::array::*;
    use arrow::datatypes::DataType;

    if array.is_null(idx) {
        return serde_json::Value::Null;
    }

    match array.data_type() {
        DataType::Boolean => {
            let arr = array.as_any().downcast_ref::<BooleanArray>().unwrap();
            serde_json::Value::Bool(arr.value(idx))
        }
        DataType::Int8 => {
            let arr = array.as_any().downcast_ref::<Int8Array>().unwrap();
            serde_json::Value::Number(arr.value(idx).into())
        }
        DataType::Int16 => {
            let arr = array.as_any().downcast_ref::<Int16Array>().unwrap();
            serde_json::Value::Number(arr.value(idx).into())
        }
        DataType::Int32 => {
            let arr = array.as_any().downcast_ref::<Int32Array>().unwrap();
            serde_json::Value::Number(arr.value(idx).into())
        }
        DataType::Int64 => {
            let arr = array.as_any().downcast_ref::<Int64Array>().unwrap();
            serde_json::Value::Number(arr.value(idx).into())
        }
        DataType::UInt8 => {
            let arr = array.as_any().downcast_ref::<UInt8Array>().unwrap();
            serde_json::Value::Number(arr.value(idx).into())
        }
        DataType::UInt16 => {
            let arr = array.as_any().downcast_ref::<UInt16Array>().unwrap();
            serde_json::Value::Number(arr.value(idx).into())
        }
        DataType::UInt32 => {
            let arr = array.as_any().downcast_ref::<UInt32Array>().unwrap();
            serde_json::Value::Number(arr.value(idx).into())
        }
        DataType::UInt64 => {
            let arr = array.as_any().downcast_ref::<UInt64Array>().unwrap();
            serde_json::Value::Number(arr.value(idx).into())
        }
        DataType::Float32 => {
            let arr = array.as_any().downcast_ref::<Float32Array>().unwrap();
            let val = arr.value(idx);
            serde_json::Number::from_f64(f64::from(val))
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null)
        }
        DataType::Float64 => {
            let arr = array.as_any().downcast_ref::<Float64Array>().unwrap();
            let val = arr.value(idx);
            serde_json::Number::from_f64(val)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null)
        }
        DataType::Utf8 => {
            let arr = array.as_any().downcast_ref::<StringArray>().unwrap();
            serde_json::Value::String(arr.value(idx).to_string())
        }
        DataType::LargeUtf8 => {
            let arr = array.as_any().downcast_ref::<LargeStringArray>().unwrap();
            serde_json::Value::String(arr.value(idx).to_string())
        }
        _ => serde_json::Value::String(format!("<unsupported type: {:?}>", array.data_type())),
    }
}

/// Print table as CSV.
fn print_table_csv(batches: &[std::sync::Arc<arrow::array::RecordBatch>]) -> Result<()> {
    if batches.is_empty() {
        return Ok(());
    }

    // Print header
    let schema = batches[0].schema();
    let headers: Vec<_> = schema.fields().iter().map(|f| f.name().clone()).collect();
    println!("{}", headers.join(","));

    // Print rows
    for batch in batches {
        for row_idx in 0..batch.num_rows() {
            let values: Vec<String> = (0..batch.num_columns())
                .map(|col_idx| {
                    let col = batch.column(col_idx);
                    csv_value(col.as_ref(), row_idx)
                })
                .collect();
            println!("{}", values.join(","));
        }
    }

    Ok(())
}

/// Format an Arrow value for CSV output.
fn csv_value(array: &dyn arrow::array::Array, idx: usize) -> String {
    let json = array_value_to_json(array, idx);
    match json {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(s) => {
            // Escape quotes and wrap in quotes if contains comma or quote
            if s.contains(',') || s.contains('"') || s.contains('\n') {
                format!("\"{}\"", s.replace('"', "\"\""))
            } else {
                s
            }
        }
        v => v.to_string(),
    }
}

/// Print REPL help.
fn print_help() {
    println!("{}", "piptable REPL Commands:".cyan().bold());
    println!("  {}    Show this help", ":help, :h, :?".yellow());
    println!("  {}  Exit the REPL", ":quit, :q, :exit".yellow());
    println!("  {}          List variables", ":vars".yellow());
    println!("  {}         Clear screen", ":clear".yellow());
    println!();
    println!("{}", "Examples:".cyan().bold());
    println!("  dim x = 10");
    println!("  dim data = query(SELECT * FROM \"data.csv\" LIMIT 5)");
    println!("  print(x * 2)");
}

#[cfg(test)]
mod tests {
    #![allow(clippy::approx_constant)]
    use super::*;
    use arrow::array::RecordBatch;
    use std::collections::HashMap;

    // ========================================================================
    // parse_cli_value tests
    // ========================================================================

    #[test]
    fn test_parse_cli_value_null() {
        assert!(matches!(parse_cli_value("null"), Value::Null));
        assert!(matches!(parse_cli_value("NULL"), Value::Null));
        assert!(matches!(parse_cli_value("Null"), Value::Null));
    }

    #[test]
    fn test_parse_cli_value_bool() {
        assert!(matches!(parse_cli_value("true"), Value::Bool(true)));
        assert!(matches!(parse_cli_value("TRUE"), Value::Bool(true)));
        assert!(matches!(parse_cli_value("false"), Value::Bool(false)));
        assert!(matches!(parse_cli_value("FALSE"), Value::Bool(false)));
    }

    #[test]
    fn test_parse_cli_value_int() {
        assert!(matches!(parse_cli_value("42"), Value::Int(42)));
        assert!(matches!(parse_cli_value("-10"), Value::Int(-10)));
        assert!(matches!(parse_cli_value("0"), Value::Int(0)));
    }

    #[test]
    fn test_parse_cli_value_float() {
        match parse_cli_value("3.14") {
            Value::Float(f) => assert!((f - 3.14).abs() < f64::EPSILON),
            _ => panic!("Expected Float"),
        }
        match parse_cli_value("-2.5") {
            Value::Float(f) => assert!((f - (-2.5)).abs() < f64::EPSILON),
            _ => panic!("Expected Float"),
        }
    }

    #[test]
    fn test_parse_cli_value_string() {
        assert!(matches!(parse_cli_value("hello"), Value::String(s) if s == "hello"));
        assert!(matches!(parse_cli_value("hello world"), Value::String(s) if s == "hello world"));
        assert!(matches!(parse_cli_value(""), Value::String(s) if s.is_empty()));
    }

    // ========================================================================
    // format_value tests
    // ========================================================================

    #[test]
    fn test_format_value_primitives() {
        assert_eq!(format_value(&Value::Null), "null");
        assert_eq!(format_value(&Value::Bool(true)), "true");
        assert_eq!(format_value(&Value::Bool(false)), "false");
        assert_eq!(format_value(&Value::Int(42)), "42");
        assert_eq!(format_value(&Value::Float(3.14)), "3.14");
        assert_eq!(format_value(&Value::String("hello".to_string())), "hello");
    }

    #[test]
    fn test_format_value_array() {
        let arr = Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
        assert_eq!(format_value(&arr), "[1, 2, 3]");

        let empty = Value::Array(vec![]);
        assert_eq!(format_value(&empty), "[]");
    }

    #[test]
    fn test_format_value_object() {
        let mut map = HashMap::new();
        map.insert("key".to_string(), Value::String("value".to_string()));
        let obj = Value::Object(map);
        assert_eq!(format_value(&obj), "{key: value}");
    }

    #[test]
    fn test_format_value_function() {
        let func = Value::Function {
            name: "add".to_string(),
            params: vec![
                piptable_core::Param {
                    name: "a".to_string(),
                    mode: piptable_core::ParamMode::ByVal,
                },
                piptable_core::Param {
                    name: "b".to_string(),
                    mode: piptable_core::ParamMode::ByVal,
                },
            ],
            is_async: false,
        };
        assert_eq!(format_value(&func), "<Function: add(a, b)>");
    }

    // ========================================================================
    // CSV escaping tests
    // ========================================================================

    #[test]
    fn test_csv_escape_simple() {
        use arrow::array::{Int64Array, StringArray};

        // Test integer
        let int_arr = Int64Array::from(vec![42]);
        assert_eq!(csv_value(&int_arr, 0), "42");

        // Test simple string (no escaping needed)
        let str_arr = StringArray::from(vec!["hello"]);
        assert_eq!(csv_value(&str_arr, 0), "hello");
    }

    #[test]
    fn test_csv_escape_special_chars() {
        use arrow::array::StringArray;

        // String with comma needs quoting
        let arr = StringArray::from(vec!["hello,world"]);
        assert_eq!(csv_value(&arr, 0), "\"hello,world\"");

        // String with quote needs escaping
        let arr = StringArray::from(vec!["say \"hello\""]);
        assert_eq!(csv_value(&arr, 0), "\"say \"\"hello\"\"\"");

        // String with newline needs quoting
        let arr = StringArray::from(vec!["line1\nline2"]);
        assert_eq!(csv_value(&arr, 0), "\"line1\nline2\"");
    }

    // ========================================================================
    // CLI argument parsing tests
    // ========================================================================

    #[test]
    fn test_cli_parse_file() {
        let cli = Cli::parse_from(["pip", "script.pip"]);
        assert_eq!(cli.file, Some(PathBuf::from("script.pip")));
        assert!(!cli.interactive);
        assert!(cli.execute.is_none());
    }

    #[test]
    fn test_cli_parse_execute() {
        let cli = Cli::parse_from(["pip", "-e", "dim x = 42"]);
        assert_eq!(cli.execute, Some("dim x = 42".to_string()));
        assert!(cli.file.is_none());
    }

    #[test]
    fn test_cli_parse_interactive() {
        let cli = Cli::parse_from(["pip", "-i"]);
        assert!(cli.interactive);
    }

    #[test]
    fn test_cli_parse_format() {
        let cli = Cli::parse_from(["pip", "-f", "json", "-e", "dim x = 1"]);
        assert!(matches!(cli.format, OutputFormat::Json));

        let cli = Cli::parse_from(["pip", "--format", "csv", "-e", "dim x = 1"]);
        assert!(matches!(cli.format, OutputFormat::Csv));

        let cli = Cli::parse_from(["pip", "-e", "dim x = 1"]);
        assert!(matches!(cli.format, OutputFormat::Table));
    }

    #[test]
    fn test_cli_parse_vars() {
        let cli = Cli::parse_from(["pip", "-D", "KEY=value", "-D", "NUM=42", "-e", "dim x = 1"]);
        assert_eq!(cli.vars.len(), 2);
        assert_eq!(cli.vars[0], "KEY=value");
        assert_eq!(cli.vars[1], "NUM=42");
    }

    #[test]
    fn test_cli_parse_verbose() {
        let cli = Cli::parse_from(["pip", "-v", "-e", "dim x = 1"]);
        assert!(cli.verbose);
    }

    // ========================================================================
    // Integration tests
    // ========================================================================

    #[tokio::test]
    async fn test_run_script_parses_valid_code() {
        let mut interpreter = Interpreter::new();
        let result = run_script(&mut interpreter, "dim x = 42", OutputFormat::Table).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[allow(unused_mut)] // interpreter needs to be mut for run_script signature
    async fn test_run_script_fails_on_invalid_code() {
        let mut interpreter = Interpreter::new();
        let result = run_script(&mut interpreter, "!@#invalid", OutputFormat::Table).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_variable_injection() {
        let interpreter = Interpreter::new();
        interpreter.set_var("x", Value::Int(42)).await;
        let value = interpreter.get_var("x").await;
        assert!(matches!(value, Some(Value::Int(42))));
    }

    // ========================================================================
    // array_value_to_json tests
    // ========================================================================

    #[test]
    fn test_array_value_to_json_boolean() {
        use arrow::array::BooleanArray;
        let arr = BooleanArray::from(vec![Some(true), Some(false), None]);
        assert_eq!(array_value_to_json(&arr, 0), serde_json::Value::Bool(true));
        assert_eq!(array_value_to_json(&arr, 1), serde_json::Value::Bool(false));
        assert_eq!(array_value_to_json(&arr, 2), serde_json::Value::Null);
    }

    #[test]
    fn test_array_value_to_json_integers() {
        use arrow::array::{Int16Array, Int32Array, Int64Array, Int8Array};

        let arr = Int8Array::from(vec![42i8]);
        assert_eq!(array_value_to_json(&arr, 0), serde_json::json!(42));

        let arr = Int16Array::from(vec![1000i16]);
        assert_eq!(array_value_to_json(&arr, 0), serde_json::json!(1000));

        let arr = Int32Array::from(vec![100_000_i32]);
        assert_eq!(array_value_to_json(&arr, 0), serde_json::json!(100_000));

        let arr = Int64Array::from(vec![9_999_999_999_i64]);
        assert_eq!(
            array_value_to_json(&arr, 0),
            serde_json::json!(9_999_999_999_i64)
        );
    }

    #[test]
    fn test_array_value_to_json_unsigned() {
        use arrow::array::{UInt16Array, UInt32Array, UInt64Array, UInt8Array};

        let arr = UInt8Array::from(vec![255u8]);
        assert_eq!(array_value_to_json(&arr, 0), serde_json::json!(255));

        let arr = UInt16Array::from(vec![65535u16]);
        assert_eq!(array_value_to_json(&arr, 0), serde_json::json!(65535));

        let arr = UInt32Array::from(vec![4_294_967_295_u32]);
        assert_eq!(
            array_value_to_json(&arr, 0),
            serde_json::json!(4_294_967_295_u64)
        );

        let arr = UInt64Array::from(vec![18_446_744_073_709_551_615_u64]);
        assert_eq!(
            array_value_to_json(&arr, 0),
            serde_json::json!(18_446_744_073_709_551_615_u64)
        );
    }

    #[test]
    fn test_array_value_to_json_floats() {
        use arrow::array::{Float32Array, Float64Array};

        let arr = Float32Array::from(vec![3.14f32]);
        let result = array_value_to_json(&arr, 0);
        if let serde_json::Value::Number(n) = result {
            assert!((n.as_f64().unwrap() - 3.14).abs() < 0.001);
        } else {
            panic!("Expected Number");
        }

        let arr = Float64Array::from(vec![2.71828f64]);
        let result = array_value_to_json(&arr, 0);
        if let serde_json::Value::Number(n) = result {
            assert!((n.as_f64().unwrap() - 2.71828).abs() < 0.00001);
        } else {
            panic!("Expected Number");
        }
    }

    #[test]
    fn test_array_value_to_json_float_nan() {
        use arrow::array::Float64Array;
        let arr = Float64Array::from(vec![f64::NAN]);
        // NaN should become null in JSON
        assert_eq!(array_value_to_json(&arr, 0), serde_json::Value::Null);
    }

    #[test]
    fn test_array_value_to_json_strings() {
        use arrow::array::{LargeStringArray, StringArray};

        let arr = StringArray::from(vec!["hello", "world"]);
        assert_eq!(
            array_value_to_json(&arr, 0),
            serde_json::Value::String("hello".to_string())
        );
        assert_eq!(
            array_value_to_json(&arr, 1),
            serde_json::Value::String("world".to_string())
        );

        let arr = LargeStringArray::from(vec!["large string"]);
        assert_eq!(
            array_value_to_json(&arr, 0),
            serde_json::Value::String("large string".to_string())
        );
    }

    // ========================================================================
    // table_to_json tests
    // ========================================================================

    #[test]
    fn test_table_to_json_empty() {
        let result = table_to_json(&[]).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_table_to_json_with_data() {
        use arrow::array::{Int64Array, StringArray};
        use arrow::datatypes::{DataType, Field, Schema};

        let schema = Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("name", DataType::Utf8, false),
        ]);

        let batch = RecordBatch::try_new(
            std::sync::Arc::new(schema),
            vec![
                std::sync::Arc::new(Int64Array::from(vec![1, 2])),
                std::sync::Arc::new(StringArray::from(vec!["alice", "bob"])),
            ],
        )
        .unwrap();

        let batches = vec![std::sync::Arc::new(batch)];
        let result = table_to_json(&batches).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0]["id"], serde_json::json!(1));
        assert_eq!(result[0]["name"], serde_json::json!("alice"));
        assert_eq!(result[1]["id"], serde_json::json!(2));
        assert_eq!(result[1]["name"], serde_json::json!("bob"));
    }

    // ========================================================================
    // print_table_csv tests
    // ========================================================================

    #[test]
    fn test_print_table_csv_empty() {
        let result = print_table_csv(&[]);
        assert!(result.is_ok());
    }

    // ========================================================================
    // format_value Table variant test
    // ========================================================================

    #[test]
    fn test_format_value_table() {
        use arrow::array::Int64Array;
        use arrow::datatypes::{DataType, Field, Schema};

        let schema = Schema::new(vec![Field::new("id", DataType::Int64, false)]);

        let batch = RecordBatch::try_new(
            std::sync::Arc::new(schema),
            vec![std::sync::Arc::new(Int64Array::from(vec![1, 2, 3]))],
        )
        .unwrap();

        let table = Value::Table(vec![std::sync::Arc::new(batch)]);
        assert_eq!(format_value(&table), "<Table: 3 rows>");
    }

    #[test]
    fn test_format_value_empty_table() {
        let table = Value::Table(vec![]);
        assert_eq!(format_value(&table), "<Table: 0 rows>");
    }

    // ========================================================================
    // dirs_history_path test
    // ========================================================================

    #[test]
    fn test_dirs_history_path() {
        // Just verify it doesn't panic - result depends on system
        let _path = dirs_history_path();
    }
}
