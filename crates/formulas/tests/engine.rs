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

#[test]
fn test_engine_sum_with_multiple_ranges() {
    let mut engine = FormulaEngine::new();
    let compiled = engine
        .compile("=SUM(A1:A2, B1:B2)")
        .expect("compile formula");

    let mut ranges = HashMap::new();
    ranges.insert(
        CellRange::new(CellAddress::new(0, 0), CellAddress::new(1, 0)),
        vec![Value::Int(1), Value::Int(2)],
    );
    ranges.insert(
        CellRange::new(CellAddress::new(0, 1), CellAddress::new(1, 1)),
        vec![Value::Int(3), Value::Int(4)],
    );
    let ctx = EvalContext::with_ranges(ranges);

    let value = engine.evaluate(&compiled, &ctx).expect("evaluate");
    match value {
        Value::Float(f) if (f - 10.0).abs() < 1e-9 => {}
        other => panic!("expected 10, got {other:?}"),
    }
}

#[test]
fn test_engine_average_basic() {
    let mut engine = FormulaEngine::new();
    let compiled = engine.compile("=AVERAGE(A1:A3)").expect("compile formula");

    let mut ranges = HashMap::new();
    ranges.insert(
        CellRange::new(CellAddress::new(0, 0), CellAddress::new(2, 0)),
        vec![Value::Int(10), Value::Int(20), Value::Int(30)],
    );
    let ctx = EvalContext::with_ranges(ranges);

    let value = engine.evaluate(&compiled, &ctx).expect("evaluate");
    match value {
        Value::Float(f) if (f - 20.0).abs() < 1e-9 => {}
        other => panic!("expected 20, got {other:?}"),
    }
}

#[test]
fn test_engine_average_empty_range() {
    let mut engine = FormulaEngine::new();
    let compiled = engine.compile("=AVERAGE(A1:A1)").expect("compile formula");

    let mut ranges = HashMap::new();
    ranges.insert(
        CellRange::new(CellAddress::new(0, 0), CellAddress::new(0, 0)),
        vec![Value::Empty],
    );
    let ctx = EvalContext::with_ranges(ranges);

    let value = engine.evaluate(&compiled, &ctx).expect("evaluate");
    assert!(matches!(value, Value::Error(_)));
}

#[test]
fn test_engine_max_basic() {
    let mut engine = FormulaEngine::new();
    let compiled = engine.compile("=MAX(A1:A4)").expect("compile formula");

    let mut ranges = HashMap::new();
    ranges.insert(
        CellRange::new(CellAddress::new(0, 0), CellAddress::new(3, 0)),
        vec![Value::Int(5), Value::Int(3), Value::Int(9), Value::Int(1)],
    );
    let ctx = EvalContext::with_ranges(ranges);

    let value = engine.evaluate(&compiled, &ctx).expect("evaluate");
    match value {
        Value::Float(f) if (f - 9.0).abs() < 1e-9 => {}
        other => panic!("expected 9, got {other:?}"),
    }
}

#[test]
fn test_engine_min_basic() {
    let mut engine = FormulaEngine::new();
    let compiled = engine.compile("=MIN(A1:A4)").expect("compile formula");

    let mut ranges = HashMap::new();
    ranges.insert(
        CellRange::new(CellAddress::new(0, 0), CellAddress::new(3, 0)),
        vec![Value::Int(5), Value::Int(3), Value::Int(9), Value::Int(1)],
    );
    let ctx = EvalContext::with_ranges(ranges);

    let value = engine.evaluate(&compiled, &ctx).expect("evaluate");
    match value {
        Value::Float(f) if (f - 1.0).abs() < 1e-9 => {}
        other => panic!("expected 1, got {other:?}"),
    }
}

#[test]
fn test_engine_count_basic() {
    let mut engine = FormulaEngine::new();
    let compiled = engine.compile("=COUNT(A1:A4)").expect("compile formula");

    let mut ranges = HashMap::new();
    ranges.insert(
        CellRange::new(CellAddress::new(0, 0), CellAddress::new(3, 0)),
        vec![
            Value::Int(1),
            Value::String("text".to_string()),
            Value::Float(2.0),
            Value::Empty,
        ],
    );
    let ctx = EvalContext::with_ranges(ranges);

    let value = engine.evaluate(&compiled, &ctx).expect("evaluate");
    match value {
        Value::Int(2) => {}
        other => panic!("expected 2, got {other:?}"),
    }
}

#[test]
fn test_engine_abs_positive() {
    let mut engine = FormulaEngine::new();
    let compiled = engine.compile("=ABS(5)").expect("compile formula");
    let ctx = EvalContext::default();

    let value = engine.evaluate(&compiled, &ctx).expect("evaluate");
    match value {
        Value::Float(f) if (f - 5.0).abs() < 1e-9 => {}
        other => panic!("expected 5, got {other:?}"),
    }
}

#[test]
fn test_engine_abs_negative() {
    let mut engine = FormulaEngine::new();
    let compiled = engine.compile("=ABS(-5)").expect("compile formula");
    let ctx = EvalContext::default();

    let value = engine.evaluate(&compiled, &ctx).expect("evaluate");
    match value {
        Value::Float(f) if (f - 5.0).abs() < 1e-9 => {}
        other => panic!("expected 5, got {other:?}"),
    }
}

#[test]
fn test_engine_abs_cell_reference() {
    let mut engine = FormulaEngine::new();
    let compiled = engine.compile("=ABS(A1)").expect("compile formula");

    let mut cells = HashMap::new();
    cells.insert(CellAddress::new(0, 0), Value::Int(-42));
    let ctx = EvalContext::with_cells(cells);

    let value = engine.evaluate(&compiled, &ctx).expect("evaluate");
    match value {
        Value::Float(f) if (f - 42.0).abs() < 1e-9 => {}
        other => panic!("expected 42, got {other:?}"),
    }
}

#[test]
fn test_engine_nested_functions() {
    let mut engine = FormulaEngine::new();
    let compiled = engine
        .compile("=SUM(ABS(-1), ABS(-2), ABS(-3))")
        .expect("compile formula");
    let ctx = EvalContext::default();

    let value = engine.evaluate(&compiled, &ctx).expect("evaluate");
    match value {
        Value::Float(f) if (f - 6.0).abs() < 1e-9 => {}
        other => panic!("expected 6, got {other:?}"),
    }
}

#[test]
fn test_engine_max_with_mixed_types() {
    let mut engine = FormulaEngine::new();
    let compiled = engine.compile("=MAX(A1:A3)").expect("compile formula");

    let mut ranges = HashMap::new();
    ranges.insert(
        CellRange::new(CellAddress::new(0, 0), CellAddress::new(2, 0)),
        vec![
            Value::Int(5),
            Value::String("text".to_string()),
            Value::Int(10),
        ],
    );
    let ctx = EvalContext::with_ranges(ranges);

    let value = engine.evaluate(&compiled, &ctx).expect("evaluate");
    match value {
        Value::Float(f) if (f - 10.0).abs() < 1e-9 => {}
        other => panic!("expected 10, got {other:?}"),
    }
}

#[test]
fn test_engine_min_with_negative() {
    let mut engine = FormulaEngine::new();
    let compiled = engine.compile("=MIN(A1:A3)").expect("compile formula");

    let mut ranges = HashMap::new();
    ranges.insert(
        CellRange::new(CellAddress::new(0, 0), CellAddress::new(2, 0)),
        vec![Value::Int(-5), Value::Int(10), Value::Int(-15)],
    );
    let ctx = EvalContext::with_ranges(ranges);

    let value = engine.evaluate(&compiled, &ctx).expect("evaluate");
    match value {
        Value::Float(f) if (f - (-15.0)).abs() < 1e-9 => {}
        other => panic!("expected -15, got {other:?}"),
    }
}

#[test]
fn test_engine_sum_with_floats() {
    let mut engine = FormulaEngine::new();
    let compiled = engine.compile("=SUM(A1:A3)").expect("compile formula");

    let mut ranges = HashMap::new();
    ranges.insert(
        CellRange::new(CellAddress::new(0, 0), CellAddress::new(2, 0)),
        vec![Value::Float(1.5), Value::Float(2.5), Value::Float(3.0)],
    );
    let ctx = EvalContext::with_ranges(ranges);

    let value = engine.evaluate(&compiled, &ctx).expect("evaluate");
    match value {
        Value::Float(f) if (f - 7.0).abs() < 1e-9 => {}
        other => panic!("expected 7, got {other:?}"),
    }
}

#[test]
fn test_engine_average_with_single_value() {
    let mut engine = FormulaEngine::new();
    let compiled = engine.compile("=AVERAGE(A1)").expect("compile formula");

    let mut cells = HashMap::new();
    cells.insert(CellAddress::new(0, 0), Value::Int(42));
    let ctx = EvalContext::with_cells(cells);

    let value = engine.evaluate(&compiled, &ctx).expect("evaluate");
    match value {
        Value::Float(f) if (f - 42.0).abs() < 1e-9 => {}
        other => panic!("expected 42, got {other:?}"),
    }
}

#[test]
fn test_engine_count_ignores_text_and_empty() {
    let mut engine = FormulaEngine::new();
    let compiled = engine.compile("=COUNT(A1:A5)").expect("compile formula");

    let mut ranges = HashMap::new();
    ranges.insert(
        CellRange::new(CellAddress::new(0, 0), CellAddress::new(4, 0)),
        vec![
            Value::Int(1),
            Value::Empty,
            Value::String("text".to_string()),
            Value::Float(2.0),
            Value::Int(3),
        ],
    );
    let ctx = EvalContext::with_ranges(ranges);

    let value = engine.evaluate(&compiled, &ctx).expect("evaluate");
    match value {
        Value::Int(3) => {}
        other => panic!("expected 3, got {other:?}"),
    }
}

#[test]
fn test_engine_sum_with_empty_cells() {
    let mut engine = FormulaEngine::new();
    let compiled = engine.compile("=SUM(A1:A3)").expect("compile formula");

    let mut ranges = HashMap::new();
    ranges.insert(
        CellRange::new(CellAddress::new(0, 0), CellAddress::new(2, 0)),
        vec![Value::Int(1), Value::Empty, Value::Int(2)],
    );
    let ctx = EvalContext::with_ranges(ranges);

    let value = engine.evaluate(&compiled, &ctx).expect("evaluate");
    match value {
        Value::Float(f) if (f - 3.0).abs() < 1e-9 => {}
        other => panic!("expected 3, got {other:?}"),
    }
}

#[test]
fn test_engine_max_single_value() {
    let mut engine = FormulaEngine::new();
    let compiled = engine.compile("=MAX(A1)").expect("compile formula");

    let mut cells = HashMap::new();
    cells.insert(CellAddress::new(0, 0), Value::Int(42));
    let ctx = EvalContext::with_cells(cells);

    let value = engine.evaluate(&compiled, &ctx).expect("evaluate");
    match value {
        Value::Float(f) if (f - 42.0).abs() < 1e-9 => {}
        other => panic!("expected 42, got {other:?}"),
    }
}

#[test]
fn test_engine_min_single_value() {
    let mut engine = FormulaEngine::new();
    let compiled = engine.compile("=MIN(A1)").expect("compile formula");

    let mut cells = HashMap::new();
    cells.insert(CellAddress::new(0, 0), Value::Int(42));
    let ctx = EvalContext::with_cells(cells);

    let value = engine.evaluate(&compiled, &ctx).expect("evaluate");
    match value {
        Value::Float(f) if (f - 42.0).abs() < 1e-9 => {}
        other => panic!("expected 42, got {other:?}"),
    }
}

#[test]
fn test_engine_abs_with_zero() {
    let mut engine = FormulaEngine::new();
    let compiled = engine.compile("=ABS(0)").expect("compile formula");
    let ctx = EvalContext::default();

    let value = engine.evaluate(&compiled, &ctx).expect("evaluate");
    match value {
        Value::Float(f) if f.abs() < 1e-9 => {}
        other => panic!("expected 0, got {other:?}"),
    }
}
