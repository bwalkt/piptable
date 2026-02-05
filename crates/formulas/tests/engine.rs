use std::collections::HashMap;

use piptable_formulas::{EvalContext, FormulaEngine};
use piptable_primitives::{CellAddress, CellRange, Value};

#[test]
fn test_engine_compile_and_evaluate_sum() {
    let mut engine = FormulaEngine::new();
    let compiled = engine.compile("=SUM(A1:A2)").expect("compile formula");

    let mut ranges = HashMap::new();
    ranges.insert(
        CellRange::new(CellAddress::new(0, 0), CellAddress::new(1, 0)),
        vec![Value::Int(1), Value::Int(2)],
    );
    let ctx = EvalContext::with_ranges(ranges);

    let value = engine.evaluate(&compiled, &ctx).expect("evaluate");
    match value {
        Value::Float(f) if (f - 3.0).abs() < 1e-9 => {}
        Value::Int(3) => {}
        other => panic!("expected 3, got {other:?}"),
    }
}

#[test]
fn test_engine_compile_invalid_formula() {
    let mut engine = FormulaEngine::new();
    assert!(engine.compile("=SUM(").is_err());
}
