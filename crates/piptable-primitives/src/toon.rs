//! TOON (Tagged Object Notation) schema definitions for spreadsheet data exchange
//!
//! Compact, efficient binary format for WASM boundary crossing

use crate::{CellAddress, CellRange, ErrorValue, Value as CellValue};
use serde::{Deserialize, Serialize};

/// TOON Value representation - compact tagged union
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "lowercase")]
pub enum ToonValue {
    #[serde(rename = "null")]
    Null,

    #[serde(rename = "bool")]
    Bool { v: u8 }, // 0 or 1 for compactness

    #[serde(rename = "int")]
    Int { v: i64 },

    #[serde(rename = "float")]
    Float { v: f64 },

    #[serde(rename = "str")]
    Str { v: String },

    #[serde(rename = "arr")]
    Array { v: Vec<ToonValue> },

    #[serde(rename = "obj")]
    Object {
        v: std::collections::HashMap<String, ToonValue>,
    },

    #[serde(rename = "date")]
    Date { v: i64 }, // Unix timestamp in ms

    #[serde(rename = "duration")]
    Duration { v: i64 }, // Duration in ms

    #[serde(rename = "error")]
    Error { code: String, msg: String },
}

/// Cell address in TOON format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToonCellAddr {
    pub r: u32, // row
    pub c: u32, // column
}

/// Range in TOON format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToonRange {
    pub s: ToonCellAddr, // start (inclusive)
    pub e: ToonCellAddr, // end (inclusive)
}

/// Sheet data payload (dense or sparse)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SheetPayload {
    /// Dense encoding (row-major values array)
    Dense {
        range: ToonRange,
        /// Row-major values, length = (rows * cols)
        values: Vec<ToonValue>,
    },
    /// Sparse encoding (only non-empty cells)
    Sparse {
        range: ToonRange,
        items: Vec<SparseCell>,
    },
}

/// Single cell in sparse encoding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SparseCell {
    pub r: u32,
    pub c: u32,
    pub v: ToonValue,
}

/// Formula as text
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormulaText {
    pub kind: String, // "text"
    pub f: String,    // formula string
}

/// Compiled formula bytecode (opaque)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormulaBytecode {
    pub kind: String, // "bc"
    pub b: Vec<u8>,   // bytecode bytes
}

/// Formula compile request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileRequest {
    pub formulas: Vec<FormulaText>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<CompileOptions>,
}

/// Compile options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decimal: Option<String>,
}

/// Formula compile response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileResponse {
    pub compiled: Vec<FormulaBytecode>,
    pub errors: Vec<CompileError>,
}

/// Compile error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileError {
    pub idx: u32,
    pub msg: String,
}

/// Formula evaluation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalRequest {
    pub compiled: Vec<FormulaBytecode>,
    pub sheet: SheetPayload,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub globals: Option<std::collections::HashMap<String, ToonValue>>,
}

/// Formula evaluation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalResponse {
    pub results: Vec<ToonValue>,
    pub errors: Vec<EvalError>,
}

/// Evaluation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalError {
    pub idx: u32,
    pub msg: String,
}

/// Range update request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangeUpdateRequest {
    pub sheet: SheetPayload,
    pub updates: Vec<CellUpdate>,
}

/// Single cell update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellUpdate {
    pub addr: ToonCellAddr,
    pub value: ToonValue,
}

/// Range update response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RangeUpdateResponse {
    Updated(SheetPayload),
    Success { ok: bool },
}

impl SheetPayload {
    /// Get value at specific cell (returns Null if missing in sparse)
    pub fn get_cell(&self, row: u32, col: u32) -> Option<ToonValue> {
        match self {
            SheetPayload::Dense { range, values } => {
                if row < range.s.r || row > range.e.r || col < range.s.c || col > range.e.c {
                    return None;
                }
                let row_offset = (row - range.s.r) as usize;
                let col_offset = (col - range.s.c) as usize;
                let cols = (range.e.c - range.s.c + 1) as usize;
                let index = row_offset * cols + col_offset;
                values.get(index).cloned()
            }
            SheetPayload::Sparse { range, items } => {
                if row < range.s.r || row > range.e.r || col < range.s.c || col > range.e.c {
                    return None;
                }
                items
                    .iter()
                    .find(|item| item.r == row && item.c == col)
                    .map(|item| item.v.clone())
                    .or(Some(ToonValue::Null))
            }
        }
    }
}

// Conversion implementations

impl From<CellAddress> for ToonCellAddr {
    fn from(addr: CellAddress) -> Self {
        ToonCellAddr {
            r: addr.row,
            c: addr.col,
        }
    }
}

impl From<ToonCellAddr> for CellAddress {
    fn from(addr: ToonCellAddr) -> Self {
        CellAddress {
            row: addr.r,
            col: addr.c,
        }
    }
}

impl From<CellRange> for ToonRange {
    fn from(range: CellRange) -> Self {
        ToonRange {
            s: range.start.into(),
            e: range.end.into(),
        }
    }
}

impl From<ToonRange> for CellRange {
    fn from(range: ToonRange) -> Self {
        CellRange {
            start: range.s.into(),
            end: range.e.into(),
        }
    }
}

impl From<CellValue> for ToonValue {
    fn from(value: CellValue) -> Self {
        match value {
            CellValue::Empty => ToonValue::Null,
            CellValue::Bool(b) => ToonValue::Bool {
                v: if b { 1 } else { 0 },
            },
            CellValue::Int(i) => ToonValue::Int { v: i },
            CellValue::Float(f) => ToonValue::Float { v: f },
            CellValue::String(s) => ToonValue::Str { v: s },
            CellValue::Error(e) => ToonValue::Error {
                code: format!("{:?}", e),
                msg: error_message(&e),
            },
            CellValue::Array(arr) => ToonValue::Array {
                v: arr.into_iter().map(Into::into).collect(),
            },
        }
    }
}

impl From<ToonValue> for CellValue {
    fn from(value: ToonValue) -> Self {
        match value {
            ToonValue::Null => CellValue::Empty,
            ToonValue::Bool { v } => CellValue::Bool(v != 0),
            ToonValue::Int { v } => CellValue::Int(v),
            ToonValue::Float { v } => CellValue::Float(v),
            ToonValue::Str { v } => CellValue::String(v),
            ToonValue::Error { code, .. } => CellValue::Error(parse_error_code(&code)),
            ToonValue::Array { v } => CellValue::Array(v.into_iter().map(Into::into).collect()),
            ToonValue::Date { v } => {
                // Convert Unix timestamp to Excel date serial
                let excel_date = unix_to_excel_date(v);
                CellValue::Float(excel_date)
            }
            ToonValue::Duration { v } => CellValue::Int(v),
            ToonValue::Object { .. } => {
                // Objects aren't directly supported in cells
                CellValue::Error(ErrorValue::Value)
            }
        }
    }
}

fn error_message(e: &ErrorValue) -> String {
    match e {
        ErrorValue::Div0 => "#DIV/0!".to_string(),
        ErrorValue::Name => "#NAME?".to_string(),
        ErrorValue::Value => "#VALUE!".to_string(),
        ErrorValue::Ref => "#REF!".to_string(),
        ErrorValue::Null => "#NULL!".to_string(),
        ErrorValue::Num => "#NUM!".to_string(),
        ErrorValue::NA => "#N/A".to_string(),
    }
}

fn parse_error_code(code: &str) -> ErrorValue {
    match code {
        "Div0" => ErrorValue::Div0,
        "Name" => ErrorValue::Name,
        "Value" => ErrorValue::Value,
        "Ref" => ErrorValue::Ref,
        "Null" => ErrorValue::Null,
        "Num" => ErrorValue::Num,
        "NA" => ErrorValue::NA,
        _ => ErrorValue::Value,
    }
}

fn unix_to_excel_date(unix_ms: i64) -> f64 {
    // Excel epoch is 1900-01-01, Unix epoch is 1970-01-01
    // Difference is 25569 days
    const EXCEL_EPOCH_OFFSET: f64 = 25569.0;
    let unix_days = (unix_ms as f64) / (1000.0 * 86400.0);
    unix_days + EXCEL_EPOCH_OFFSET
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_addr_conversion() {
        let addr = CellAddress { row: 2, col: 3 };
        let toon: ToonCellAddr = addr.into();
        assert_eq!(toon.r, 2);
        assert_eq!(toon.c, 3);

        let back: CellAddress = toon.into();
        assert_eq!(back.row, 2);
        assert_eq!(back.col, 3);
    }

    #[test]
    fn test_range_conversion() {
        let range = CellRange {
            start: CellAddress { row: 1, col: 1 },
            end: CellAddress { row: 2, col: 3 },
        };
        let toon: ToonRange = range.into();
        assert_eq!(toon.s.r, 1);
        assert_eq!(toon.e.c, 3);

        let back: CellRange = toon.into();
        assert_eq!(back.start.row, 1);
        assert_eq!(back.end.col, 3);
    }

    #[test]
    fn test_value_to_toon_and_back() {
        let value = CellValue::Empty;
        let toon: ToonValue = value.clone().into();
        assert!(matches!(toon, ToonValue::Null));
        let back: CellValue = toon.into();
        assert!(matches!(back, CellValue::Empty));

        let value = CellValue::Bool(true);
        let toon: ToonValue = value.clone().into();
        assert!(matches!(toon, ToonValue::Bool { v: 1 }));
        let back: CellValue = toon.into();
        assert!(matches!(back, CellValue::Bool(true)));

        let value = CellValue::Int(42);
        let toon: ToonValue = value.clone().into();
        assert!(matches!(toon, ToonValue::Int { v: 42 }));

        let back: CellValue = toon.into();
        assert!(matches!(back, CellValue::Int(42)));

        let value = CellValue::Float(3.5);
        let toon: ToonValue = value.clone().into();
        assert!(matches!(toon, ToonValue::Float { v } if (v - 3.5).abs() < 0.0001));
        let back: CellValue = toon.into();
        assert!(matches!(back, CellValue::Float(f) if (f - 3.5).abs() < 0.0001));

        let value = CellValue::String("x".to_string());
        let toon: ToonValue = value.clone().into();
        assert!(matches!(toon, ToonValue::Str { ref v } if v == "x"));
        let back: CellValue = toon.into();
        assert!(matches!(back, CellValue::String(s) if s == "x"));

        let value = CellValue::Error(ErrorValue::Div0);
        let toon: ToonValue = value.into();
        assert!(matches!(toon, ToonValue::Error { ref code, .. } if code == "Div0"));

        let back: CellValue = toon.into();
        assert!(matches!(back, CellValue::Error(ErrorValue::Div0)));

        let value = CellValue::Array(vec![CellValue::Int(1), CellValue::String("a".to_string())]);
        let toon: ToonValue = value.clone().into();
        assert!(matches!(toon, ToonValue::Array { ref v } if v.len() == 2));

        let back: CellValue = toon.into();
        assert!(matches!(back, CellValue::Array(v) if v.len() == 2));
    }

    #[test]
    fn test_toon_object_to_cell_error() {
        let mut obj = std::collections::HashMap::new();
        obj.insert("a".to_string(), ToonValue::Int { v: 1 });
        let toon = ToonValue::Object { v: obj };
        let value: CellValue = toon.into();
        assert!(matches!(value, CellValue::Error(ErrorValue::Value)));
    }

    #[test]
    fn test_unix_to_excel_date() {
        let excel = unix_to_excel_date(0);
        assert!((excel - 25569.0).abs() < 0.0001);
    }

    #[test]
    fn test_toon_date_and_duration() {
        let date = ToonValue::Date { v: 0 };
        let value: CellValue = date.into();
        assert!(matches!(value, CellValue::Float(f) if (f - 25569.0).abs() < 0.0001));

        let duration = ToonValue::Duration { v: 60000 };
        let value: CellValue = duration.into();
        assert!(matches!(value, CellValue::Int(60000)));
    }

    #[test]
    fn test_parse_error_code_unknown() {
        assert!(matches!(parse_error_code("Unknown"), ErrorValue::Value));
    }

    #[test]
    fn test_error_message_and_codes() {
        assert_eq!(error_message(&ErrorValue::Div0), "#DIV/0!");
        assert_eq!(error_message(&ErrorValue::Name), "#NAME?");
        assert_eq!(error_message(&ErrorValue::Value), "#VALUE!");
        assert_eq!(error_message(&ErrorValue::Ref), "#REF!");
        assert_eq!(error_message(&ErrorValue::Null), "#NULL!");
        assert_eq!(error_message(&ErrorValue::Num), "#NUM!");
        assert_eq!(error_message(&ErrorValue::NA), "#N/A");

        assert!(matches!(parse_error_code("Div0"), ErrorValue::Div0));
        assert!(matches!(parse_error_code("Name"), ErrorValue::Name));
        assert!(matches!(parse_error_code("Value"), ErrorValue::Value));
        assert!(matches!(parse_error_code("Ref"), ErrorValue::Ref));
        assert!(matches!(parse_error_code("Null"), ErrorValue::Null));
        assert!(matches!(parse_error_code("Num"), ErrorValue::Num));
        assert!(matches!(parse_error_code("NA"), ErrorValue::NA));
    }
}
