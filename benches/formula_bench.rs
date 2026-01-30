use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use piptable_formulas::{FormulaEngine, ValueResolver};
use piptable_primitives::{CellAddress, CellRange, Value};
use std::collections::HashMap;

struct BenchContext {
    cells: HashMap<CellAddress, Value>,
    ranges: HashMap<CellRange, Vec<Value>>,
}

impl BenchContext {
    fn new_with_range(size: usize) -> Self {
        let mut cells = HashMap::new();
        let mut values = Vec::new();

        for i in 0..size {
            let addr = CellAddress::new(i as u32, 0);
            let value = Value::Float((i as f64) * 1.5);
            cells.insert(addr, value.clone());
            values.push(value);
        }

        let mut ranges = HashMap::new();
        if size > 0 {
            let range = CellRange::new(
                CellAddress::new(0, 0),
                CellAddress::new((size - 1) as u32, 0),
            );
            ranges.insert(range, values);
        }

        Self { cells, ranges }
    }
}

impl ValueResolver for BenchContext {
    fn get_cell(&self, addr: &CellAddress) -> Value {
        self.cells.get(addr).cloned().unwrap_or(Value::Empty)
    }

    fn get_range(&self, range: &CellRange) -> Vec<Value> {
        self.ranges.get(range).cloned().unwrap_or_default()
    }

    fn get_sheet_cell(&self, _sheet: &str, addr: &CellAddress) -> Value {
        self.get_cell(addr)
    }

    fn get_sheet_range(&self, _sheet: &str, range: &CellRange) -> Vec<Value> {
        self.get_range(range)
    }
}

fn bench_parse_formulas(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse");
    let mut engine = FormulaEngine::new();

    group.bench_function("simple", |b| b.iter(|| engine.compile(black_box("=1+2"))));

    group.bench_function("cell_ref", |b| {
        b.iter(|| engine.compile(black_box("=A1+B2")))
    });

    group.bench_function("function_call", |b| {
        b.iter(|| engine.compile(black_box("=SUM(A1:A10)")))
    });

    group.bench_function("nested", |b| {
        b.iter(|| engine.compile(black_box("=IF(A1>10,SUM(B1:B10),AVERAGE(C1:C10))")))
    });

    group.bench_function("complex", |b| {
        b.iter(|| {
            engine.compile(black_box(
                "=IF(AND(A1>0,B1<100),SUM(C1:C10)*1.1,MAX(D1:D10)/MIN(E1:E10))",
            ))
        })
    });

    group.finish();
}

fn bench_evaluate_formulas(c: &mut Criterion) {
    let mut group = c.benchmark_group("evaluate");
    let mut engine = FormulaEngine::new();

    // Pre-compile formulas
    let simple = engine.compile("=1+2").unwrap();
    let cell_add = engine.compile("=A1+A2").unwrap();
    let sum_10 = engine.compile("=SUM(A1:A10)").unwrap();
    let sum_100 = engine.compile("=SUM(A1:A100)").unwrap();
    let sum_1000 = engine.compile("=SUM(A1:A1000)").unwrap();

    let ctx_10 = BenchContext::new_with_range(10);
    let ctx_100 = BenchContext::new_with_range(100);
    let ctx_1000 = BenchContext::new_with_range(1000);

    group.bench_function("literal", |b| {
        b.iter(|| engine.evaluate(black_box(&simple), black_box(&ctx_10)))
    });

    group.bench_function("cell_add", |b| {
        b.iter(|| engine.evaluate(black_box(&cell_add), black_box(&ctx_10)))
    });

    group.bench_function("sum_10", |b| {
        b.iter(|| engine.evaluate(black_box(&sum_10), black_box(&ctx_10)))
    });

    group.bench_function("sum_100", |b| {
        b.iter(|| engine.evaluate(black_box(&sum_100), black_box(&ctx_100)))
    });

    group.bench_function("sum_1000", |b| {
        b.iter(|| engine.evaluate(black_box(&sum_1000), black_box(&ctx_1000)))
    });

    group.finish();
}

fn bench_range_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("range_ops");
    let mut engine = FormulaEngine::new();

    for size in [10, 100, 1000, 10000].iter() {
        let ctx = BenchContext::new_with_range(*size);
        let formula = format!("=SUM(A1:A{})", size);
        let compiled = engine.compile(&formula).unwrap();

        group.bench_with_input(BenchmarkId::new("sum", size), size, |b, _| {
            b.iter(|| engine.evaluate(black_box(&compiled), black_box(&ctx)))
        });
    }

    for size in [10, 100, 1000].iter() {
        let ctx = BenchContext::new_with_range(*size);
        let formula = format!("=AVERAGE(A1:A{})", size);
        let compiled = engine.compile(&formula).unwrap();

        group.bench_with_input(BenchmarkId::new("average", size), size, |b, _| {
            b.iter(|| engine.evaluate(black_box(&compiled), black_box(&ctx)))
        });
    }

    group.finish();
}

fn bench_text_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("text_ops");
    let mut engine = FormulaEngine::new();

    let concat_2 = engine.compile("=CONCAT(\"Hello\", \" World\")").unwrap();
    let concat_10 = engine
        .compile("=CONCAT(\"a\",\"b\",\"c\",\"d\",\"e\",\"f\",\"g\",\"h\",\"i\",\"j\")")
        .unwrap();
    let left = engine.compile("=LEFT(\"Hello World\", 5)").unwrap();
    let len = engine.compile("=LEN(\"Hello World\")").unwrap();

    let ctx = BenchContext::new_with_range(0);

    group.bench_function("concat_2", |b| {
        b.iter(|| engine.evaluate(black_box(&concat_2), black_box(&ctx)))
    });

    group.bench_function("concat_10", |b| {
        b.iter(|| engine.evaluate(black_box(&concat_10), black_box(&ctx)))
    });

    group.bench_function("left", |b| {
        b.iter(|| engine.evaluate(black_box(&left), black_box(&ctx)))
    });

    group.bench_function("len", |b| {
        b.iter(|| engine.evaluate(black_box(&len), black_box(&ctx)))
    });

    group.finish();
}

fn bench_logical_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("logical_ops");
    let mut engine = FormulaEngine::new();

    let if_simple = engine.compile("=IF(1>0, 10, 20)").unwrap();
    let if_nested = engine.compile("=IF(1>0, IF(2>1, 10, 20), 30)").unwrap();
    let and_op = engine.compile("=AND(1>0, 2>1, 3>2)").unwrap();
    let or_op = engine.compile("=OR(1>2, 2>3, 3>2)").unwrap();

    let ctx = BenchContext::new_with_range(0);

    group.bench_function("if_simple", |b| {
        b.iter(|| engine.evaluate(black_box(&if_simple), black_box(&ctx)))
    });

    group.bench_function("if_nested", |b| {
        b.iter(|| engine.evaluate(black_box(&if_nested), black_box(&ctx)))
    });

    group.bench_function("and", |b| {
        b.iter(|| engine.evaluate(black_box(&and_op), black_box(&ctx)))
    });

    group.bench_function("or", |b| {
        b.iter(|| engine.evaluate(black_box(&or_op), black_box(&ctx)))
    });

    group.finish();
}

fn bench_cache_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache");
    let mut engine = FormulaEngine::new();

    // Pre-populate cache
    for i in 0..100 {
        let addr = CellAddress::new(i, 0);
        let formula = format!("=A{}+1", i + 1);
        engine.set_formula(addr, &formula).unwrap();
    }

    let addr = CellAddress::new(50, 0);

    group.bench_function("lookup_hit", |b| {
        b.iter(|| engine.get_formula(black_box(&addr)))
    });

    let miss_addr = CellAddress::new(200, 0);
    group.bench_function("lookup_miss", |b| {
        b.iter(|| engine.get_formula(black_box(&miss_addr)))
    });

    group.bench_function("invalidate", |b| {
        b.iter(|| {
            engine.invalidate(black_box(&addr));
            // Re-add for next iteration
            engine.set_formula(addr, "=A51+1").unwrap();
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_parse_formulas,
    bench_evaluate_formulas,
    bench_range_operations,
    bench_text_operations,
    bench_logical_operations,
    bench_cache_operations
);
criterion_main!(benches);
