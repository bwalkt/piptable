//! WASM APIs for spreadsheet operations
//!
//! Minimal, batch-oriented APIs using TOON for efficient data exchange

use piptable_formulas::{CompiledFormula, FormulaEngine, ValueResolver};
use piptable_primitives::toon::{
    CellUpdate, CompileError, CompileRequest, CompileResponse, EvalError, EvalRequest,
    EvalResponse, FormulaBytecode, FormulaText, RangeUpdateRequest, RangeUpdateResponse,
    SheetPayload, ToonValue,
};
use piptable_primitives::{CellAddress, CellRange, Value};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

/// Compile multiple formulas in batch
///
/// Input: TOON-encoded CompileRequest
/// Output: TOON-encoded CompileResponse
#[wasm_bindgen]
pub fn compile_many(toon_bytes: &[u8]) -> Result<Vec<u8>, JsValue> {
    compile_many_bytes(toon_bytes).map_err(|e| JsValue::from_str(&e))
}

/// Evaluate multiple compiled formulas in batch
///
/// Input: TOON-encoded EvalRequest
/// Output: TOON-encoded EvalResponse
#[wasm_bindgen]
pub fn eval_many(toon_bytes: &[u8]) -> Result<Vec<u8>, JsValue> {
    eval_many_bytes(toon_bytes).map_err(|e| JsValue::from_str(&e))
}

/// Apply updates to a sheet range
///
/// Input: TOON-encoded RangeUpdateRequest
/// Output: TOON-encoded RangeUpdateResponse
#[wasm_bindgen]
pub fn apply_range(toon_bytes: &[u8]) -> Result<Vec<u8>, JsValue> {
    apply_range_bytes(toon_bytes).map_err(|e| JsValue::from_str(&e))
}

/// Validate a formula for syntax highlighting
///
/// Input: TOON-encoded FormulaText
/// Output: TOON-encoded validation result
#[wasm_bindgen]
pub fn validate_formula(toon_bytes: &[u8]) -> Result<Vec<u8>, JsValue> {
    let formula: FormulaText = if is_json_bytes(toon_bytes) {
        serde_json::from_slice(toon_bytes)
            .map_err(|e| JsValue::from_str(&format!("JSON parse error: {}", e)))?
    } else {
        rmp_serde::from_slice(toon_bytes)
            .map_err(|e| JsValue::from_str(&format!("TOON parse error: {}", e)))?
    };

    // Validate formula syntax
    let mut engine = FormulaEngine::new();
    let validation = match compile_formula(&mut engine, &formula.f) {
        Ok(_) => serde_json::json!({
            "valid": true,
            "msg": "Formula is valid"
        }),
        Err(e) => serde_json::json!({
            "valid": false,
            "msg": e.to_string()
        }),
    };

    // Return validation result
    let response_bytes = if is_json_bytes(toon_bytes) {
        serde_json::to_vec(&validation)
            .map_err(|e| JsValue::from_str(&format!("JSON serialize error: {}", e)))?
    } else {
        rmp_serde::to_vec(&validation)
            .map_err(|e| JsValue::from_str(&format!("TOON serialize error: {}", e)))?
    };

    Ok(response_bytes)
}

// Helper functions

fn compile_formula(engine: &mut FormulaEngine, formula: &str) -> Result<Vec<u8>, String> {
    let compiled = engine.compile(formula).map_err(|e| e.to_string())?;
    rmp_serde::to_vec(&compiled).map_err(|e| e.to_string())
}

fn evaluate_bytecode(
    engine: &FormulaEngine,
    bytecode: &[u8],
    context: &WasmEvalContext,
) -> Result<ToonValue, String> {
    let compiled: CompiledFormula = rmp_serde::from_slice(bytecode).map_err(|e| e.to_string())?;
    let value = engine
        .evaluate(&compiled, context)
        .map_err(|e| e.to_string())?;
    Ok(ToonValue::from(value))
}

fn create_eval_context(
    sheet: &SheetPayload,
    globals: Option<HashMap<String, ToonValue>>,
) -> WasmEvalContext {
    let sparse_index = match sheet {
        SheetPayload::Sparse { items, .. } => {
            let mut map = HashMap::with_capacity(items.len());
            for item in items {
                map.insert((item.r, item.c), item.v.clone());
            }
            Some(map)
        }
        _ => None,
    };

    WasmEvalContext {
        sheet: sheet.clone(),
        globals: globals.unwrap_or_default(),
        sparse_index,
    }
}

fn apply_cell_update(
    sheet: &mut SheetPayload,
    update: CellUpdate,
    compact: bool,
) -> Result<(), JsValue> {
    match sheet {
        SheetPayload::Dense { range, values } => {
            if update.addr.r < range.s.r
                || update.addr.r > range.e.r
                || update.addr.c < range.s.c
                || update.addr.c > range.e.c
            {
                return Err(JsValue::from_str("Cell address out of range"));
            }

            let row_offset = (update.addr.r - range.s.r) as usize;
            let col_offset = (update.addr.c - range.s.c) as usize;
            let rows = (range.e.r - range.s.r + 1) as usize;
            let cols = (range.e.c - range.s.c + 1) as usize;
            let expected_len = rows
                .checked_mul(cols)
                .ok_or_else(|| JsValue::from_str("Range too large"))?;
            if values.len() < expected_len {
                return Err(JsValue::from_str("Dense payload length mismatch"));
            }
            let index = row_offset * cols + col_offset;

            values[index] = update.value;
            Ok(())
        }
        SheetPayload::Sparse { range, items } => {
            if update.addr.r < range.s.r
                || update.addr.r > range.e.r
                || update.addr.c < range.s.c
                || update.addr.c > range.e.c
            {
                return Err(JsValue::from_str("Cell address out of range"));
            }

            let is_null = matches!(update.value, ToonValue::Null);
            if let Some(existing) = items
                .iter_mut()
                .find(|item| item.r == update.addr.r && item.c == update.addr.c)
            {
                if is_null {
                    // remove existing by marking for removal
                    existing.v = ToonValue::Null;
                } else {
                    existing.v = update.value;
                }
            } else if !is_null {
                items.push(piptable_primitives::toon::SparseCell {
                    r: update.addr.r,
                    c: update.addr.c,
                    v: update.value,
                });
            }

            if compact {
                items.retain(|item| !matches!(item.v, ToonValue::Null));
            }
            Ok(())
        }
    }
}

pub fn compile_many_bytes(toon_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let request: CompileRequest = decode_request(toon_bytes)?;

    let mut engine = FormulaEngine::new();
    let mut compiled = Vec::new();
    let mut errors = Vec::new();

    for (idx, formula_text) in request.formulas.iter().enumerate() {
        match compile_formula(&mut engine, &formula_text.f) {
            Ok(bytecode) => {
                compiled.push(FormulaBytecode {
                    kind: "bc".to_string(),
                    b: bytecode,
                });
            }
            Err(e) => {
                compiled.push(FormulaBytecode {
                    kind: "bc".to_string(),
                    b: vec![],
                });
                errors.push(CompileError {
                    idx: idx as u32,
                    msg: e.to_string(),
                });
            }
        }
    }

    let response = CompileResponse { compiled, errors };
    encode_response(&response, toon_bytes)
}

pub fn eval_many_bytes(toon_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let request: EvalRequest = decode_request(toon_bytes)?;
    let context = create_eval_context(&request.sheet, request.globals);
    let engine = FormulaEngine::new();

    let mut results = Vec::new();
    let mut errors = Vec::new();

    for (idx, bytecode) in request.compiled.iter().enumerate() {
        match evaluate_bytecode(&engine, &bytecode.b, &context) {
            Ok(value) => results.push(value),
            Err(e) => {
                results.push(ToonValue::Error {
                    code: "EVAL".to_string(),
                    msg: e.to_string(),
                });
                errors.push(EvalError {
                    idx: idx as u32,
                    msg: e.to_string(),
                });
            }
        }
    }

    let response = EvalResponse { results, errors };
    encode_response(&response, toon_bytes)
}

pub fn apply_range_bytes(toon_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let request: RangeUpdateRequest = decode_request(toon_bytes)?;

    let mut sheet = request.sheet;
    let total = request.updates.len();
    for (idx, update) in request.updates.into_iter().enumerate() {
        let is_last = idx + 1 == total;
        apply_cell_update(&mut sheet, update, is_last)
            .map_err(|e| e.as_string().unwrap_or_default())?;
    }

    let response = RangeUpdateResponse::Updated(sheet);
    encode_response(&response, toon_bytes)
}

fn is_json_bytes(bytes: &[u8]) -> bool {
    let mut first = None;
    for b in bytes {
        if !b.is_ascii_whitespace() {
            first = Some(*b);
            break;
        }
    }
    matches!(first, Some(b'{') | Some(b'['))
}

fn decode_request<T: serde::de::DeserializeOwned>(toon_bytes: &[u8]) -> Result<T, String> {
    if is_json_bytes(toon_bytes) {
        serde_json::from_slice(toon_bytes).map_err(|e| format!("JSON parse error: {}", e))
    } else {
        rmp_serde::from_slice(toon_bytes).map_err(|e| format!("TOON parse error: {}", e))
    }
}

fn encode_response<T: serde::Serialize>(
    response: &T,
    toon_bytes: &[u8],
) -> Result<Vec<u8>, String> {
    if is_json_bytes(toon_bytes) {
        serde_json::to_vec(response).map_err(|e| format!("JSON serialize error: {}", e))
    } else {
        rmp_serde::to_vec(response).map_err(|e| format!("TOON serialize error: {}", e))
    }
}

struct WasmEvalContext {
    sheet: SheetPayload,
    #[allow(dead_code)]
    // Reserved for future global resolution (named values/functions).
    globals: HashMap<String, ToonValue>,
    sparse_index: Option<HashMap<(u32, u32), ToonValue>>,
}

impl ValueResolver for WasmEvalContext {
    fn get_cell(&self, addr: &CellAddress) -> Value {
        let toon = match &self.sheet {
            SheetPayload::Sparse { .. } => self
                .sparse_index
                .as_ref()
                .and_then(|map| map.get(&(addr.row, addr.col)).cloned())
                .unwrap_or(ToonValue::Null),
            _ => self
                .sheet
                .get_cell(addr.row, addr.col)
                .unwrap_or(ToonValue::Null),
        };
        toon.into()
    }

    fn get_range(&self, range: &CellRange) -> Vec<Value> {
        let normalized = range.normalized();
        let rows = normalized.rows() as usize;
        let cols = normalized.cols() as usize;
        let mut values = Vec::with_capacity(rows);
        for r in 0..rows {
            let mut row = Vec::with_capacity(cols);
            for c in 0..cols {
                let addr =
                    CellAddress::new(normalized.start.row + r as u32, normalized.start.col + c as u32);
                row.push(self.get_cell(&addr));
            }
            values.push(Value::Array(row));
        }
        values
    }

    fn get_sheet_cell(&self, _sheet: &str, addr: &CellAddress) -> Value {
        self.get_cell(addr)
    }

    fn get_sheet_range(&self, _sheet: &str, range: &CellRange) -> Vec<Value> {
        self.get_range(range)
    }
}
