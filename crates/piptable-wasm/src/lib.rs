use arrow::record_batch::RecordBatch;
use piptable_core::{Expr, PipError, Program, Statement, Value};
use piptable_interpreter::Interpreter;
use piptable_parser::PipParser;
use piptable_sheet::{CellValue, Sheet};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use wasm_bindgen::prelude::*;

pub mod spreadsheet;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
    console_log!("PipTable WASM initialized");
}

#[derive(Serialize, Deserialize)]
pub struct ParseResult {
    pub success: bool,
    pub ast: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ValidationError {
    pub line: usize,
    pub column: usize,
    pub message: String,
}

#[derive(Serialize)]
pub struct ExecResult {
    pub success: bool,
    pub output: Vec<String>,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

#[wasm_bindgen]
pub struct PipTableParser;

impl Default for PipTableParser {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl PipTableParser {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        console_log!("Creating PipTable parser");
        Self
    }

    #[wasm_bindgen]
    pub fn parse(&self, code: &str) -> Result<JsValue, JsValue> {
        #[cfg(debug_assertions)]
        console_log!("Parsing {} bytes of code", code.len());

        match PipParser::parse_str(code) {
            Ok(ast) => {
                let result = ParseResult {
                    success: true,
                    ast: Some(format!("{:#?}", ast)),
                    error: None,
                };
                serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
            }
            Err(e) => {
                let result = ParseResult {
                    success: false,
                    ast: None,
                    error: Some(e.to_string()),
                };
                serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
            }
        }
    }

    #[wasm_bindgen]
    pub fn validate(&self, code: &str) -> Result<JsValue, JsValue> {
        match PipParser::parse_str(code) {
            Ok(_) => {
                let result = serde_json::json!({
                    "valid": true,
                    "errors": []
                });
                serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
            }
            Err(e) => {
                // Extract line and column from PipError::Parse variant
                let (line, column, message) = match e {
                    PipError::Parse {
                        line,
                        column,
                        message,
                    } => (line, column, message),
                    // For other error types, default to line 1, column 1
                    other_error => (1, 1, other_error.to_string()),
                };

                let errors = vec![serde_json::json!({
                    "line": line,
                    "column": column,
                    "message": message
                })];

                let result = serde_json::json!({
                    "valid": false,
                    "errors": errors
                });
                serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
            }
        }
    }

    #[wasm_bindgen]
    pub fn format(&self, code: &str) -> Result<String, JsValue> {
        // For now, just return the code as-is
        // In the future, we could implement proper formatting
        Ok(code.to_string())
    }
}

#[wasm_bindgen]
pub fn get_examples() -> Result<JsValue, JsValue> {
    let examples = serde_json::json!({
        "hello": {
            "name": "Hello World",
            "code": "print \"Hello, World!\""
        },
        "variables": {
            "name": "Variables",
            "code": "' Variable assignment\nx = 42\ny = \"hello\"\nprint x\nprint y"
        },
        "sheet": {
            "name": "Sheet Operations",
            "code": r#"' Create a sheet
data = sheet([
    ["Name", "Age", "City"],
    ["Alice", 30, "NYC"],
    ["Bob", 25, "LA"]
])

' Filter data
adults = data | filter(Age >= 18)
print adults"#
        },
        "import": {
            "name": "Import CSV",
            "code": r#"' Import CSV file
import "data.csv" as sales

' Process the data
summary = sales | group_by(Category) | sum(Amount)
print summary"#
        },
        "sql": {
            "name": "SQL Query",
            "code": r#"' SQL query on sheet
result = query(
    SELECT Name, Age 
    FROM data 
    WHERE Age > 25
    ORDER BY Age DESC
)
print result"#
        },
        "functions": {
            "name": "Functions",
            "code": r#"' Define a function
function greet(name)
    return "Hello, " & name & "!"
end function

' Use the function
message = greet("World")
print message"#
        }
    });

    serde_wasm_bindgen::to_value(&examples).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen]
pub fn get_sample_data() -> Result<JsValue, JsValue> {
    let data = serde_json::json!({
        "csv": r#"Name,Age,City,Department
Alice,30,New York,Engineering
Bob,25,Los Angeles,Marketing
Charlie,35,Chicago,Sales
Diana,28,Houston,Engineering
Eve,32,Seattle,Marketing"#,
        "json": serde_json::json!([
            {"Name": "Alice", "Age": 30, "City": "New York", "Department": "Engineering"},
            {"Name": "Bob", "Age": 25, "City": "Los Angeles", "Department": "Marketing"},
            {"Name": "Charlie", "Age": 35, "City": "Chicago", "Department": "Sales"},
            {"Name": "Diana", "Age": 28, "City": "Houston", "Department": "Engineering"},
            {"Name": "Eve", "Age": 32, "City": "Seattle", "Department": "Marketing"}
        ])
    });

    serde_wasm_bindgen::to_value(&data).map_err(|e| JsValue::from_str(&e.to_string()))
}

fn cell_to_json(cell: &CellValue) -> serde_json::Value {
    match cell {
        CellValue::Null => serde_json::Value::Null,
        CellValue::Bool(b) => serde_json::Value::Bool(*b),
        CellValue::Int(i) => serde_json::Value::Number((*i).into()),
        CellValue::Float(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        CellValue::String(s) => serde_json::Value::String(s.clone()),
    }
}

fn sheet_to_json(sheet: &Sheet) -> serde_json::Value {
    let rows: Vec<serde_json::Value> = sheet
        .data()
        .iter()
        .map(|row: &Vec<CellValue>| {
            let cells: Vec<serde_json::Value> = row.iter().map(cell_to_json).collect();
            serde_json::Value::Array(cells)
        })
        .collect();
    serde_json::Value::Array(rows)
}

fn table_to_json(batches: &[Arc<RecordBatch>]) -> serde_json::Value {
    let mut total_rows: usize = 0;
    for batch in batches {
        total_rows = total_rows.saturating_add(batch.num_rows());
    }

    let columns: Vec<serde_json::Value> = batches
        .first()
        .map(|batch| {
            batch
                .schema()
                .fields()
                .iter()
                .map(|field| serde_json::Value::String(field.name().clone()))
                .collect::<Vec<serde_json::Value>>()
        })
        .unwrap_or_default();

    let mut out = serde_json::Map::new();
    out.insert(
        "type".to_string(),
        serde_json::Value::String("table".to_string()),
    );
    out.insert(
        "rows".to_string(),
        serde_json::Value::Number(serde_json::Number::from(total_rows as u64)),
    );
    out.insert("columns".to_string(), serde_json::Value::Array(columns));
    serde_json::Value::Object(out)
}

fn value_to_json(value: &Value) -> serde_json::Value {
    match value {
        Value::Null => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Int(i) => serde_json::Value::Number((*i).into()),
        Value::Float(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        Value::String(s) => serde_json::Value::String(s.clone()),
        Value::Array(items) => {
            let values = items.iter().map(value_to_json).collect();
            serde_json::Value::Array(values)
        }
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (k, v) in map {
                out.insert(k.clone(), value_to_json(v));
            }
            serde_json::Value::Object(out)
        }
        Value::Sheet(sheet) => sheet_to_json(sheet),
        Value::Table(batches) => table_to_json(batches),
        Value::Function { name, .. } => serde_json::Value::String(format!("<function {}>", name)),
        Value::Lambda { .. } => serde_json::Value::String("<lambda>".to_string()),
    }
}

fn validate_statement(stmt: &Statement) -> Result<(), String> {
    match stmt {
        Statement::Import { line, .. } => Err(format!(
            "Line {}: import is not supported in the playground",
            line
        )),
        Statement::Export { line, .. } => Err(format!(
            "Line {}: export is not supported in the playground",
            line
        )),
        Statement::Dim { value, .. } => validate_expr(value),
        Statement::Assignment { target, value, .. } => {
            validate_lvalue(target)?;
            validate_expr(value)
        }
        Statement::If {
            condition,
            then_body,
            elseif_clauses,
            else_body,
            ..
        } => {
            validate_expr(condition)?;
            for stmt in then_body {
                validate_statement(stmt)?;
            }
            for clause in elseif_clauses {
                validate_expr(&clause.condition)?;
                for stmt in &clause.body {
                    validate_statement(stmt)?;
                }
            }
            if let Some(body) = else_body {
                for stmt in body {
                    validate_statement(stmt)?;
                }
            }
            Ok(())
        }
        Statement::ForEach { iterable, body, .. } => {
            validate_expr(iterable)?;
            for stmt in body {
                validate_statement(stmt)?;
            }
            Ok(())
        }
        Statement::For {
            start,
            end,
            step,
            body,
            ..
        } => {
            validate_expr(start)?;
            validate_expr(end)?;
            if let Some(step) = step {
                validate_expr(step)?;
            }
            for stmt in body {
                validate_statement(stmt)?;
            }
            Ok(())
        }
        Statement::While {
            condition, body, ..
        } => {
            validate_expr(condition)?;
            for stmt in body {
                validate_statement(stmt)?;
            }
            Ok(())
        }
        Statement::Function { body, .. } => {
            for stmt in body {
                validate_statement(stmt)?;
            }
            Ok(())
        }
        Statement::Return { value, .. } => {
            if let Some(value) = value {
                validate_expr(value)?;
            }
            Ok(())
        }
        Statement::Call { args, .. } => {
            for arg in args {
                validate_expr(arg)?;
            }
            Ok(())
        }
        Statement::Chart { .. } => Ok(()),
        Statement::Append { source, .. } => validate_expr(source),
        Statement::Upsert { source, .. } => validate_expr(source),
        Statement::Expr { expr, .. } => validate_expr(expr),
        Statement::ExitFunction { .. }
        | Statement::ExitFor { .. }
        | Statement::ExitWhile { .. } => Ok(()),
    }
}

fn validate_expr(expr: &Expr) -> Result<(), String> {
    match expr {
        Expr::Fetch { .. } => Err("fetch is not supported in the playground".to_string()),
        Expr::Ask { .. } => Err("ask is not supported in the playground".to_string()),
        Expr::Binary { left, right, .. } => {
            validate_expr(left)?;
            validate_expr(right)
        }
        Expr::Unary { operand, .. } => validate_expr(operand),
        Expr::FieldAccess { object, .. } => validate_expr(object),
        Expr::ArrayIndex { array, index, .. } => {
            validate_expr(array)?;
            validate_expr(index)
        }
        Expr::TypeAssertion { expr, .. } => validate_expr(expr),
        Expr::Call { args, .. } => {
            for arg in args {
                validate_expr(arg)?;
            }
            Ok(())
        }
        Expr::CallExpr { callee, args } => {
            validate_expr(callee)?;
            for arg in args {
                validate_expr(arg)?;
            }
            Ok(())
        }
        Expr::Query(_) => Err("SQL is not supported in the playground".to_string()),
        Expr::AsyncForEach { iterable, body, .. } => {
            validate_expr(iterable)?;
            for stmt in body {
                validate_statement(stmt)?;
            }
            Ok(())
        }
        Expr::Parallel { expressions } => {
            for expr in expressions {
                validate_expr(expr)?;
            }
            Ok(())
        }
        Expr::Await(expr) => validate_expr(expr),
        Expr::Array(items) => {
            for item in items {
                validate_expr(item)?;
            }
            Ok(())
        }
        Expr::Object(items) => {
            for (_, value) in items {
                validate_expr(value)?;
            }
            Ok(())
        }
        Expr::Join { left, right, .. } => {
            validate_expr(left)?;
            validate_expr(right)
        }
        Expr::MethodCall { object, args, .. } => {
            validate_expr(object)?;
            for arg in args {
                validate_expr(arg)?;
            }
            Ok(())
        }
        Expr::Lambda { body, .. } => validate_expr(body),
        Expr::Literal(_) | Expr::Variable(_) => Ok(()),
    }
}

fn validate_lvalue(target: &piptable_core::LValue) -> Result<(), String> {
    match target {
        piptable_core::LValue::Variable(_) => Ok(()),
        piptable_core::LValue::Field { object, .. } => validate_lvalue(object),
        piptable_core::LValue::Index { array, index } => {
            validate_lvalue(array)?;
            validate_expr(index)
        }
    }
}

fn validate_program(program: &Program) -> Result<(), String> {
    for stmt in &program.statements {
        validate_statement(stmt)?;
    }
    Ok(())
}

/// Execute PipTable source code (parse, validate, and run) and return a JSON-serializable execution result.
///
/// The returned value encodes the execution outcome and any runtime output or error. On success the payload contains
/// `success: true`, an `output` array of log lines, and an optional `result` value; on failure it contains
/// `success: false` and an `error` message describing the problem (parse, validation, or runtime).
///
/// # Examples
///
/// ```no_run
/// # async fn example() {
/// let js: wasm_bindgen::JsValue = run_code("dim x = 1".into()).await.unwrap();
/// // `js` is a JsValue holding the ExecResult JSON object described above.
/// # }
/// ```
#[wasm_bindgen]
pub async fn run_code(code: String) -> Result<JsValue, JsValue> {
    let result = run_code_inner(&code).await;
    serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Executes PipTable code end-to-end: parses, validates, interprets, and collects output.
///
/// On parse failure or validation failure returns an `ExecResult` with `success = false` and an `error` describing the problem. On successful execution returns `success = true`, `output` containing captured interpreter logs, and `result` containing the evaluated value converted to JSON.
///
/// # Examples
///
/// ```
/// use futures::executor::block_on;
///
/// // Example usage: run a simple program and inspect success flag.
/// let code = "dim x = 1\nx";
/// let res = block_on(crate::run_code_inner(code));
/// assert!(res.success || res.error.is_some());
/// ```
async fn run_code_inner(code: &str) -> ExecResult {
    let program = match PipParser::parse_str(code) {
        Ok(program) => program,
        Err(e) => {
            return ExecResult {
                success: false,
                output: Vec::new(),
                result: None,
                error: Some(format!("Parse error: {}", e)),
            };
        }
    };

    if let Err(err) = validate_program(&program) {
        return ExecResult {
            success: false,
            output: Vec::new(),
            result: None,
            error: Some(err),
        };
    }

    let mut interp = Interpreter::new();
    let eval_result = interp.eval(program).await;
    let output = interp.output().await;

    match eval_result {
        Ok(value) => ExecResult {
            success: true,
            output,
            result: Some(value_to_json(&value)),
            error: None,
        },
        Err(e) => ExecResult {
            success: false,
            output,
            result: None,
            error: Some(e.to_string()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::run_code_inner;
    use futures::executor::block_on;

    #[test]
    fn run_code_reports_parse_errors() {
        let result = block_on(run_code_inner("dim x ="));
        assert!(!result.success);
        assert!(result.output.is_empty());
        assert!(result.result.is_none());
        let error = result.error.expect("error should be present");
        assert!(error.contains("Parse error"));
    }
}