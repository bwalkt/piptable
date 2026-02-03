//! WASM boundary types with proper Value mappings
//!
//! Maps between piptable_core::Value and TOON for safe WASM crossing

use crate::Value;
use piptable_primitives::toon::{ToonCellAddr, ToonValue};
use piptable_primitives::CellAddress;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Sheet payload with automatic sparse/dense encoding selection
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SheetPayload {
    /// Dense encoding (row-major values array)
    Dense {
        range: ToonRange,
        values: Vec<ToonValue>,
    },
    /// Sparse encoding (only non-empty cells)
    Sparse {
        range: ToonRange,
        items: Vec<SparseCell>,
    },
}

/// Range definition for TOON
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToonRange {
    pub s: ToonCellAddr, // start (inclusive)
    pub e: ToonCellAddr, // end (inclusive)
}

/// Single cell in sparse encoding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SparseCell {
    pub r: u32,
    pub c: u32,
    pub v: ToonValue,
}

impl SheetPayload {
    /// Create a sheet payload, automatically choosing encoding
    pub fn from_values(start: CellAddress, end: CellAddress, values: Vec<Vec<Value>>) -> Self {
        let rows = (end.row - start.row + 1) as usize;
        let cols = (end.col - start.col + 1) as usize;
        let total_cells = rows * cols;

        // Count non-empty cells
        let mut non_empty_count = 0;
        let mut sparse_items = Vec::new();

        for (r_idx, row) in values.iter().enumerate() {
            for (c_idx, value) in row.iter().enumerate() {
                if !matches!(value, Value::Null) {
                    non_empty_count += 1;
                    sparse_items.push(SparseCell {
                        r: start.row + r_idx as u32,
                        c: start.col + c_idx as u32,
                        v: value_to_toon(value),
                    });
                }
            }
        }

        // Choose encoding based on density
        if should_use_sparse(rows as u32, cols as u32, non_empty_count) {
            SheetPayload::Sparse {
                range: ToonRange {
                    s: start.into(),
                    e: end.into(),
                },
                items: sparse_items,
            }
        } else {
            // Dense encoding - flatten to row-major
            let mut flat_values = Vec::with_capacity(total_cells);
            for row in values {
                for value in row {
                    flat_values.push(value_to_toon(&value));
                }
            }

            SheetPayload::Dense {
                range: ToonRange {
                    s: start.into(),
                    e: end.into(),
                },
                values: flat_values,
            }
        }
    }

    /// Get dimensions of the sheet
    pub fn dimensions(&self) -> (u32, u32) {
        match self {
            SheetPayload::Dense { range, .. } | SheetPayload::Sparse { range, .. } => {
                let rows = range.e.r - range.s.r + 1;
                let cols = range.e.c - range.s.c + 1;
                (rows, cols)
            }
        }
    }

    /// Get value at specific cell
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

/// Determine if sparse encoding should be used
pub fn should_use_sparse(rows: u32, cols: u32, non_empty_count: usize) -> bool {
    let total_cells = (rows * cols) as usize;

    if total_cells == 0 {
        return false;
    }

    let density = non_empty_count as f64 / total_cells as f64;

    // Use sparse if:
    // 1. Density < 20%
    // 2. Large grid (>10k cells) with < 50% density
    density < 0.2 || (total_cells > 10_000 && density < 0.5)
}

/// Convert piptable Value to TOON (WASM-safe)
pub fn value_to_toon(value: &Value) -> ToonValue {
    match value {
        Value::Null => ToonValue::Null,
        Value::Bool(b) => ToonValue::Bool {
            v: if *b { 1 } else { 0 },
        },
        Value::Int(i) => ToonValue::Int { v: *i },
        Value::Float(f) => ToonValue::Float { v: *f },
        Value::String(s) => ToonValue::Str { v: s.clone() },
        Value::Array(arr) => ToonValue::Array {
            v: arr.iter().map(value_to_toon).collect(),
        },
        Value::Object(map) => {
            let mut obj = HashMap::new();
            for (k, v) in map {
                obj.insert(k.clone(), value_to_toon(v));
            }
            ToonValue::Object { v: obj }
        }
        // Non-WASM-safe types
        Value::Table(_) => ToonValue::Error {
            code: "TABLE_UNSUPPORTED".to_string(),
            msg: "Table values cannot cross WASM boundary".to_string(),
        },
        Value::Sheet(sheet) => {
            // Convert to simplified representation
            // For now, return error - could implement sheet serialization
            ToonValue::Error {
                code: "SHEET_UNSUPPORTED".to_string(),
                msg: format!(
                    "Sheet {}x{} cannot cross boundary directly",
                    sheet.row_count(),
                    sheet.col_count()
                ),
            }
        }
        Value::Function { name, .. } => ToonValue::Error {
            code: "FUNCTION_UNSUPPORTED".to_string(),
            msg: format!("Function '{}' cannot cross WASM boundary", name),
        },
        Value::Lambda { .. } => ToonValue::Error {
            code: "LAMBDA_UNSUPPORTED".to_string(),
            msg: "Lambda expressions cannot cross WASM boundary".to_string(),
        },
    }
}

/// Convert TOON back to piptable Value
pub fn toon_to_value(toon: &ToonValue) -> Value {
    match toon {
        ToonValue::Null => Value::Null,
        ToonValue::Bool { v } => Value::Bool(*v != 0),
        ToonValue::Int { v } => Value::Int(*v),
        ToonValue::Float { v } => Value::Float(*v),
        ToonValue::Str { v } => Value::String(v.clone()),
        ToonValue::Array { v } => Value::Array(v.iter().map(toon_to_value).collect()),
        ToonValue::Object { v } => {
            let mut map = HashMap::new();
            for (k, val) in v {
                map.insert(k.clone(), toon_to_value(val));
            }
            Value::Object(map)
        }
        ToonValue::Date { v } => {
            // Convert Unix timestamp to string for now
            Value::String(format!("DATE({})", v))
        }
        ToonValue::Duration { v } => Value::Int(*v),
        ToonValue::Error { msg, .. } => {
            // Create string representation of error
            Value::String(format!("ERROR: {}", msg))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use piptable_sheet::Sheet;
    use piptable_types::{Expr, Literal, Param, ParamMode};
    use std::collections::HashMap;

    #[test]
    fn test_sparse_encoding_selection() {
        // Small dense grid should use dense
        assert!(!should_use_sparse(10, 10, 80)); // 80% density

        // Zero-sized grid should be dense (false)
        assert!(!should_use_sparse(0, 0, 0));

        // Large sparse grid should use sparse
        assert!(should_use_sparse(100, 100, 500)); // 5% density

        // Large grid with medium density should use sparse
        assert!(should_use_sparse(200, 200, 10000)); // 25% density

        // Small sparse grid should use sparse
        assert!(should_use_sparse(10, 10, 10)); // 10% density
    }

    #[test]
    fn test_value_to_toon_mapping() {
        // Basic types
        assert!(matches!(value_to_toon(&Value::Null), ToonValue::Null));
        assert!(matches!(
            value_to_toon(&Value::Bool(true)),
            ToonValue::Bool { v: 1 }
        ));
        assert!(matches!(
            value_to_toon(&Value::Int(42)),
            ToonValue::Int { v: 42 }
        ));

        // Unsupported types become errors
        let table_val = Value::Table(vec![]);
        let toon = value_to_toon(&table_val);
        assert!(matches!(toon, ToonValue::Error { code, .. } if code == "TABLE_UNSUPPORTED"));
    }

    #[test]
    fn test_value_to_toon_all_variants() {
        let mut obj = HashMap::new();
        obj.insert("a".to_string(), Value::Int(1));

        let values = vec![
            value_to_toon(&Value::Null),
            value_to_toon(&Value::Bool(false)),
            value_to_toon(&Value::Int(7)),
            value_to_toon(&Value::Float(2.5)),
            value_to_toon(&Value::String("x".to_string())),
            value_to_toon(&Value::Array(vec![Value::Int(1), Value::Bool(true)])),
            value_to_toon(&Value::Object(obj)),
        ];

        assert!(matches!(values[0], ToonValue::Null));
        assert!(matches!(values[1], ToonValue::Bool { v: 0 }));
        assert!(matches!(values[2], ToonValue::Int { v: 7 }));
        assert!(matches!(values[3], ToonValue::Float { v: f } if (f - 2.5).abs() < 0.0001));
        assert!(matches!(&values[4], ToonValue::Str { v } if v == "x"));
        assert!(matches!(values[5], ToonValue::Array { .. }));
        assert!(matches!(values[6], ToonValue::Object { .. }));

        let sheet = Sheet::from_data(vec![vec![1i64, 2i64]]);
        let toon = value_to_toon(&Value::Sheet(Box::new(sheet)));
        assert!(matches!(toon, ToonValue::Error { code, .. } if code == "SHEET_UNSUPPORTED"));

        let func = Value::Function {
            name: "f".to_string(),
            params: vec![Param {
                name: "x".to_string(),
                mode: ParamMode::ByVal,
                default: None,
                is_param_array: false,
            }],
            is_async: false,
        };
        let toon = value_to_toon(&func);
        assert!(matches!(toon, ToonValue::Error { code, .. } if code == "FUNCTION_UNSUPPORTED"));

        let lam = Value::Lambda {
            params: vec!["x".to_string()],
            body: Expr::Literal(Literal::Int(1)),
        };
        let toon = value_to_toon(&lam);
        assert!(matches!(toon, ToonValue::Error { code, .. } if code == "LAMBDA_UNSUPPORTED"));
    }

    #[test]
    fn test_toon_to_value_variants() {
        let toon = ToonValue::Null;
        assert!(matches!(toon_to_value(&toon), Value::Null));

        let toon = ToonValue::Bool { v: 1 };
        assert!(matches!(toon_to_value(&toon), Value::Bool(true)));

        let toon = ToonValue::Int { v: 3 };
        assert!(matches!(toon_to_value(&toon), Value::Int(3)));

        let toon = ToonValue::Float { v: 1.5 };
        assert!(matches!(toon_to_value(&toon), Value::Float(f) if (f - 1.5).abs() < 0.0001));

        let toon = ToonValue::Str { v: "s".to_string() };
        assert!(matches!(toon_to_value(&toon), Value::String(s) if s == "s"));

        let toon = ToonValue::Array {
            v: vec![
                ToonValue::Int { v: 1 },
                ToonValue::Str { v: "a".to_string() },
            ],
        };
        assert!(matches!(toon_to_value(&toon), Value::Array(v) if v.len() == 2));

        let mut obj = HashMap::new();
        obj.insert("k".to_string(), ToonValue::Int { v: 9 });
        let toon = ToonValue::Object { v: obj };
        assert!(matches!(toon_to_value(&toon), Value::Object(m) if m.contains_key("k")));

        let toon = ToonValue::Date { v: 0 };
        assert!(matches!(toon_to_value(&toon), Value::String(s) if s == "DATE(0)"));

        let toon = ToonValue::Duration { v: 42 };
        assert!(matches!(toon_to_value(&toon), Value::Int(42)));

        let toon = ToonValue::Error {
            code: "X".to_string(),
            msg: "bad".to_string(),
        };
        assert!(matches!(toon_to_value(&toon), Value::String(s) if s == "ERROR: bad"));
    }

    #[test]
    fn test_sheet_payload_dense_get_cell() {
        let start = CellAddress::new(0, 0);
        let end = CellAddress::new(1, 1);
        let values = vec![
            vec![Value::Null, Value::Int(1)],
            vec![Value::Bool(true), Value::Null],
        ];
        let payload = SheetPayload::from_values(start, end, values);
        assert!(matches!(&payload, SheetPayload::Dense { .. }));
        if let SheetPayload::Dense { range, values } = &payload {
            assert_eq!(range.s.r, 0);
            assert_eq!(range.e.c, 1);
            assert_eq!(values.len(), 4);
        }
        assert_eq!(payload.dimensions(), (2, 2));

        assert!(matches!(
            payload.get_cell(0, 1),
            Some(ToonValue::Int { v: 1 })
        ));
        assert!(matches!(payload.get_cell(0, 0), Some(ToonValue::Null)));
        assert!(payload.get_cell(2, 2).is_none());
    }

    #[test]
    fn test_sheet_payload_sparse_get_cell() {
        let start = CellAddress::new(0, 0);
        let end = CellAddress::new(9, 9);
        let mut values = vec![vec![Value::Null; 10]; 10];
        values[3][4] = Value::Int(7);

        let payload = SheetPayload::from_values(start, end, values);
        assert!(matches!(&payload, SheetPayload::Sparse { .. }));
        if let SheetPayload::Sparse { items, .. } = &payload {
            assert_eq!(items.len(), 1);
        }
        assert_eq!(payload.dimensions(), (10, 10));

        assert!(matches!(
            payload.get_cell(3, 4),
            Some(ToonValue::Int { v: 7 })
        ));
        assert!(matches!(payload.get_cell(0, 0), Some(ToonValue::Null)));
        assert!(payload.get_cell(20, 20).is_none());
    }
}
