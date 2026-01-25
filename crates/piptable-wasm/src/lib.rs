use piptable_parser::PipParser;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

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
        console_log!("Parsing code: {}", code);

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
                // Extract line and column from error message if possible
                let error_str = e.to_string();
                let errors = vec![serde_json::json!({
                    "line": 1,
                    "column": 1,
                    "message": error_str
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
