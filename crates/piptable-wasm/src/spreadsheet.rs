//! WASM APIs for spreadsheet operations
//!
//! Minimal, batch-oriented APIs using TOON for efficient data exchange

use piptable_formulas::FormulaEngine;
use piptable_primitives::toon::{
    CellUpdate, CompileError, CompileRequest, CompileResponse, EvalError, EvalRequest,
    EvalResponse, FormulaBytecode, FormulaText, RangeUpdateRequest, RangeUpdateResponse,
    SheetPayload, ToonValue,
};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

/// Compile multiple formulas in batch
///
/// Input: TOON-encoded CompileRequest
/// Output: TOON-encoded CompileResponse
#[wasm_bindgen]
pub fn compile_many(toon_bytes: &[u8]) -> Result<Vec<u8>, JsValue> {
    // Detect format by checking first byte
    let request: CompileRequest = if toon_bytes.starts_with(b"{") {
        // JSON format for debugging
        serde_json::from_slice(toon_bytes)
            .map_err(|e| JsValue::from_str(&format!("JSON parse error: {}", e)))?
    } else {
        // TOON binary format (using MessagePack for now as TOON implementation)
        rmp_serde::from_slice(toon_bytes)
            .map_err(|e| JsValue::from_str(&format!("TOON parse error: {}", e)))?
    };

    // Process compile request
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
                // Add placeholder for failed compilation
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

    // Serialize response in same format as request
    let response_bytes = if toon_bytes.starts_with(b"{") {
        serde_json::to_vec(&response)
            .map_err(|e| JsValue::from_str(&format!("JSON serialize error: {}", e)))?
    } else {
        rmp_serde::to_vec(&response)
            .map_err(|e| JsValue::from_str(&format!("TOON serialize error: {}", e)))?
    };

    Ok(response_bytes)
}

/// Evaluate multiple compiled formulas in batch
///
/// Input: TOON-encoded EvalRequest
/// Output: TOON-encoded EvalResponse
#[wasm_bindgen]
pub fn eval_many(toon_bytes: &[u8]) -> Result<Vec<u8>, JsValue> {
    // Detect format
    let request: EvalRequest = if toon_bytes.starts_with(b"{") {
        serde_json::from_slice(toon_bytes)
            .map_err(|e| JsValue::from_str(&format!("JSON parse error: {}", e)))?
    } else {
        rmp_serde::from_slice(toon_bytes)
            .map_err(|e| JsValue::from_str(&format!("TOON parse error: {}", e)))?
    };

    // Create evaluation context with sheet data
    let context = create_eval_context(&request.sheet, request.globals);

    let mut results = Vec::new();
    let mut errors = Vec::new();

    // Evaluate each compiled formula
    for (idx, bytecode) in request.compiled.iter().enumerate() {
        match evaluate_bytecode(&bytecode.b, &context) {
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

    // Serialize in same format
    let response_bytes = if toon_bytes.starts_with(b"{") {
        serde_json::to_vec(&response)
            .map_err(|e| JsValue::from_str(&format!("JSON serialize error: {}", e)))?
    } else {
        rmp_serde::to_vec(&response)
            .map_err(|e| JsValue::from_str(&format!("TOON serialize error: {}", e)))?
    };

    Ok(response_bytes)
}

/// Apply updates to a sheet range
///
/// Input: TOON-encoded RangeUpdateRequest
/// Output: TOON-encoded RangeUpdateResponse
#[wasm_bindgen]
pub fn apply_range(toon_bytes: &[u8]) -> Result<Vec<u8>, JsValue> {
    // Detect format
    let request: RangeUpdateRequest = if toon_bytes.starts_with(b"{") {
        serde_json::from_slice(toon_bytes)
            .map_err(|e| JsValue::from_str(&format!("JSON parse error: {}", e)))?
    } else {
        rmp_serde::from_slice(toon_bytes)
            .map_err(|e| JsValue::from_str(&format!("TOON parse error: {}", e)))?
    };

    // Apply updates to sheet
    let mut sheet = request.sheet;
    for update in request.updates {
        apply_cell_update(&mut sheet, update)?;
    }

    let response = RangeUpdateResponse::Updated(sheet);

    // Serialize in same format
    let response_bytes = if toon_bytes.starts_with(b"{") {
        serde_json::to_vec(&response)
            .map_err(|e| JsValue::from_str(&format!("JSON serialize error: {}", e)))?
    } else {
        rmp_serde::to_vec(&response)
            .map_err(|e| JsValue::from_str(&format!("TOON serialize error: {}", e)))?
    };

    Ok(response_bytes)
}

/// Validate a formula for syntax highlighting
///
/// Input: TOON-encoded FormulaText
/// Output: TOON-encoded validation result
#[wasm_bindgen]
pub fn validate_formula(toon_bytes: &[u8]) -> Result<Vec<u8>, JsValue> {
    let formula: FormulaText = if toon_bytes.starts_with(b"{") {
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
    let response_bytes = if toon_bytes.starts_with(b"{") {
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
    // TODO: Implement actual compilation
    // For now, just store the formula text as "bytecode"
    let _ = engine;
    Ok(formula.as_bytes().to_vec())
}

fn evaluate_bytecode(bytecode: &[u8], _context: &EvalContext) -> Result<ToonValue, String> {
    // TODO: Implement actual evaluation
    // For now, return a placeholder
    Ok(ToonValue::Str {
        v: format!(
            "=TODO({})",
            std::str::from_utf8(bytecode).unwrap_or("invalid")
        ),
    })
}

fn create_eval_context(
    sheet: &SheetPayload,
    globals: Option<HashMap<String, ToonValue>>,
) -> EvalContext {
    EvalContext {
        sheet: sheet.clone(),
        globals: globals.unwrap_or_default(),
    }
}

fn apply_cell_update(sheet: &mut SheetPayload, update: CellUpdate) -> Result<(), JsValue> {
    let row_offset = (update.addr.r - sheet.range.s.r) as usize;
    let col_offset = (update.addr.c - sheet.range.s.c) as usize;
    let cols = (sheet.range.e.c - sheet.range.s.c + 1) as usize;

    let index = row_offset * cols + col_offset;

    if index < sheet.values.len() {
        sheet.values[index] = update.value;
        Ok(())
    } else {
        Err(JsValue::from_str("Cell address out of range"))
    }
}

struct EvalContext {
    #[allow(dead_code)]
    sheet: SheetPayload,
    #[allow(dead_code)]
    globals: HashMap<String, ToonValue>,
}
