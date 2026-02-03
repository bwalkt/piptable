use piptable_primitives::toon::{
    CellUpdate, CompileRequest, EvalRequest, FormulaText, RangeUpdateRequest, SheetPayload,
    ToonCellAddr, ToonRange, ToonValue,
};
use piptable_wasm::spreadsheet::{apply_range_bytes, compile_many_bytes, eval_many_bytes};

/// Builds a JSON-encoded request payload.
fn json_bytes<T: serde::Serialize>(value: &T) -> Vec<u8> {
    serde_json::to_vec(value).expect("json encode")
}

/// Builds a msgpack-encoded request payload.
fn msgpack_bytes<T: serde::Serialize>(value: &T) -> Vec<u8> {
    rmp_serde::to_vec_named(value).expect("msgpack encode")
}

/// Decodes a JSON response payload.
fn parse_json<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> T {
    serde_json::from_slice(bytes).expect("json decode")
}

/// Decodes a msgpack response payload.
fn parse_msgpack<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> T {
    rmp_serde::from_slice(bytes).expect("msgpack decode")
}

/// Verifies JSON eval for dense sheets.
#[test]
fn test_compile_and_eval_json_dense() {
    let compile_req = CompileRequest {
        formulas: vec![
            FormulaText {
                kind: "text".to_string(),
                f: "=A1+B1".to_string(),
            },
            FormulaText {
                kind: "text".to_string(),
                f: "=SUM(A1:B1)".to_string(),
            },
        ],
        options: None,
    };

    let compile_resp: piptable_primitives::toon::CompileResponse =
        parse_json(&compile_many_bytes(&json_bytes(&compile_req)).expect("compile"));
    assert!(compile_resp.errors.is_empty());
    assert_eq!(compile_resp.compiled.len(), 2);

    let sheet = SheetPayload::Dense {
        range: ToonRange {
            s: ToonCellAddr { r: 0, c: 0 },
            e: ToonCellAddr { r: 0, c: 1 },
        },
        values: vec![ToonValue::Int { v: 1 }, ToonValue::Int { v: 2 }],
    };

    let eval_req = EvalRequest {
        compiled: compile_resp.compiled,
        sheet,
        globals: None,
    };

    let eval_resp: piptable_primitives::toon::EvalResponse =
        parse_json(&eval_many_bytes(&json_bytes(&eval_req)).expect("eval"));
    assert!(eval_resp.errors.is_empty());
    assert_eq!(eval_resp.results.len(), 2);
    assert!(matches!(eval_resp.results[0], ToonValue::Float { v } if (v - 3.0).abs() < 0.001));
    assert!(matches!(eval_resp.results[1], ToonValue::Float { v } if (v - 3.0).abs() < 0.001));
}

/// Verifies msgpack eval for sparse sheets.
#[test]
fn test_compile_and_eval_msgpack_sparse() {
    let compile_req = CompileRequest {
        formulas: vec![FormulaText {
            kind: "text".to_string(),
            f: "=A1+B1".to_string(),
        }],
        options: None,
    };

    let compile_resp: piptable_primitives::toon::CompileResponse =
        parse_msgpack(&compile_many_bytes(&msgpack_bytes(&compile_req)).expect("compile"));
    assert!(compile_resp.errors.is_empty());

    let sheet = SheetPayload::Sparse {
        range: ToonRange {
            s: ToonCellAddr { r: 0, c: 0 },
            e: ToonCellAddr { r: 0, c: 1 },
        },
        items: vec![
            piptable_primitives::toon::SparseCell {
                r: 0,
                c: 0,
                v: ToonValue::Int { v: 10 },
            },
            piptable_primitives::toon::SparseCell {
                r: 0,
                c: 1,
                v: ToonValue::Int { v: 5 },
            },
        ],
    };

    let eval_req = EvalRequest {
        compiled: compile_resp.compiled,
        sheet,
        globals: None,
    };

    let eval_resp: piptable_primitives::toon::EvalResponse =
        parse_msgpack(&eval_many_bytes(&msgpack_bytes(&eval_req)).expect("eval"));
    assert!(eval_resp.errors.is_empty());
    assert!(matches!(eval_resp.results[0], ToonValue::Float { v } if (v - 15.0).abs() < 0.001));
}

/// Verifies range updates for sparse sheets.
#[test]
fn test_apply_range_sparse_updates() {
    let sheet = SheetPayload::Sparse {
        range: ToonRange {
            s: ToonCellAddr { r: 0, c: 0 },
            e: ToonCellAddr { r: 0, c: 1 },
        },
        items: vec![piptable_primitives::toon::SparseCell {
            r: 0,
            c: 0,
            v: ToonValue::Int { v: 1 },
        }],
    };

    let update_req = RangeUpdateRequest {
        sheet,
        updates: vec![
            CellUpdate {
                addr: ToonCellAddr { r: 0, c: 1 },
                value: ToonValue::Int { v: 2 },
            },
            CellUpdate {
                addr: ToonCellAddr { r: 0, c: 0 },
                value: ToonValue::Null,
            },
        ],
    };

    let response: piptable_primitives::toon::RangeUpdateResponse =
        parse_json(&apply_range_bytes(&json_bytes(&update_req)).expect("apply range"));

    match response {
        piptable_primitives::toon::RangeUpdateResponse::Updated(SheetPayload::Sparse {
            items,
            ..
        }) => {
            assert_eq!(items.len(), 1);
            assert!(matches!(items[0].v, ToonValue::Int { v: 2 }));
        }
        _ => panic!("expected sparse payload"),
    }
}
