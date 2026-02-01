//! Formula integration helpers for the DSL runtime.

use piptable_core::{PipError, PipResult, Value};
use piptable_formulas::{CompiledFormula, FormulaEngine, FunctionRegistry, ValueResolver};
use piptable_primitives::{CellAddress, CellRange, ErrorValue, Value as FormulaValue};
use piptable_sheet::{CellValue, Sheet};
use std::sync::OnceLock;
use std::{collections::HashMap, fmt::Display};

static FORMULA_REGISTRY: OnceLock<FunctionRegistry> = OnceLock::new();
const MAX_FORMULA_CACHE_ENTRIES: usize = 1024;

fn registry() -> &'static FunctionRegistry {
    FORMULA_REGISTRY.get_or_init(FunctionRegistry::default)
}

const DSL_FORMULA_FUNCTIONS: &[&str] = &[
    "SUM",
    "AVERAGE",
    "AVG",
    "COUNT",
    "MAX",
    "MIN",
    "IF",
    "AND",
    "OR",
    "NOT",
    "CONCAT",
    "CONCATENATE",
    "LEN",
    "LEFT",
    "RIGHT",
    "TODAY",
    "NOW",
    "DATE",
    "VLOOKUP",
    "HLOOKUP",
    "INDEX",
    "MATCH",
    "XLOOKUP",
    "OFFSET",
];

pub fn is_dsl_formula_function(name: &str) -> bool {
    let upper = name.to_uppercase();
    DSL_FORMULA_FUNCTIONS.contains(&upper.as_str())
}

pub fn call_formula_function(name: &str, args: &[Value], line: usize) -> PipResult<Value> {
    let def = registry()
        .get(name)
        .ok_or_else(|| PipError::runtime(line, format!("Unknown function: {name}")))?;

    let formula_args: Vec<FormulaValue> = args
        .iter()
        .map(|value| core_to_formula(value, line))
        .collect::<PipResult<Vec<_>>>()?;

    validate_arg_count(def.min_args, def.max_args, formula_args.len(), name, line)?;

    let result = (def.eval)(&formula_args);
    formula_to_core(result, line)
}

pub fn eval_sheet_formula(sheet: &Sheet, formula: &str, line: usize) -> PipResult<Value> {
    let mut engine = FormulaEngine::new();
    let compiled = engine.compile(formula).map_err(|e| {
        PipError::runtime(line, format_formula_error("sheet_eval_formula", formula, e))
    })?;
    let resolver = SheetResolver {
        sheet,
        base_cell: None,
    };
    let result = engine.evaluate(&compiled, &resolver).map_err(|e| {
        PipError::runtime(line, format_formula_error("sheet_eval_formula", formula, e))
    })?;
    formula_to_core_with_context(result, line, "sheet_eval_formula", formula)
}

pub struct CachedFormulaEngine {
    engine: FormulaEngine,
    cache: HashMap<String, CompiledFormula>,
}

impl CachedFormulaEngine {
    pub fn new() -> Self {
        Self {
            engine: FormulaEngine::new(),
            cache: HashMap::new(),
        }
    }

    pub fn compile_cached(
        &mut self,
        formula: &str,
        line: usize,
        context: &str,
    ) -> PipResult<CompiledFormula> {
        if self.cache.len() >= MAX_FORMULA_CACHE_ENTRIES {
            self.cache.clear();
        }
        if let Some(compiled) = self.cache.get(formula) {
            return Ok(compiled.clone());
        }
        let compiled = self
            .engine
            .compile(formula)
            .map_err(|e| PipError::runtime(line, format_formula_error(context, formula, e)))?;
        self.cache.insert(formula.to_string(), compiled.clone());
        Ok(compiled)
    }

    pub fn evaluate(
        &mut self,
        compiled: &CompiledFormula,
        sheet: &Sheet,
        base_cell: Option<CellAddress>,
        line: usize,
        context: &str,
        formula: &str,
    ) -> PipResult<Value> {
        let resolver = SheetResolver { sheet, base_cell };
        let result = self
            .engine
            .evaluate(compiled, &resolver)
            .map_err(|e| PipError::runtime(line, format_formula_error(context, formula, e)))?;
        formula_to_core_with_context(result, line, context, formula)
    }
}

pub fn eval_sheet_formula_cached(
    engine: &mut CachedFormulaEngine,
    sheet: &Sheet,
    formula: &str,
    line: usize,
    context: &str,
) -> PipResult<Value> {
    let compiled = engine.compile_cached(formula, line, context)?;
    engine.evaluate(&compiled, sheet, None, line, context, formula)
}

#[allow(dead_code)]
pub fn eval_sheet_range_function(
    sheet: &Sheet,
    function: &str,
    range: &str,
    line: usize,
) -> PipResult<Value> {
    let formula = format!("{}({})", function, range);
    eval_sheet_formula(sheet, &formula, line)
}

pub fn eval_sheet_range_function_cached(
    engine: &mut CachedFormulaEngine,
    sheet: &Sheet,
    function: &str,
    range: &str,
    line: usize,
) -> PipResult<Value> {
    let formula = format!("{}({})", function, range);
    eval_sheet_formula_cached(engine, sheet, &formula, line, "sheet_eval_formula")
}

pub fn range_function_name(name: &str) -> Option<&'static str> {
    match name.to_uppercase().as_str() {
        "SUM" => Some("SUM"),
        "AVG" | "AVERAGE" => Some("AVERAGE"),
        "MIN" => Some("MIN"),
        "MAX" => Some("MAX"),
        "COUNT" => Some("COUNT"),
        _ => None,
    }
}

#[allow(dead_code)]
pub fn eval_sheet_cell(sheet: &Sheet, notation: &str, line: usize) -> PipResult<Value> {
    let cell = sheet.get_a1(notation).map_err(|e| {
        PipError::runtime(line, format!("Invalid cell notation '{}': {}", notation, e))
    })?;
    match cell {
        CellValue::String(s) if s.trim_start().starts_with('=') => {
            let base_cell = sheet.get_a1_addr(notation).map_err(|e| {
                PipError::runtime(line, format!("Invalid cell notation '{}': {}", notation, e))
            })?;
            let mut engine = FormulaEngine::new();
            let compiled = engine.compile(s).map_err(|e| {
                PipError::runtime(line, format_formula_error("sheet_eval_formula", s, e))
            })?;
            let resolver = SheetResolver {
                sheet,
                base_cell: Some(base_cell),
            };
            let result = engine.evaluate(&compiled, &resolver).map_err(|e| {
                PipError::runtime(line, format_formula_error("sheet_eval_formula", s, e))
            })?;
            formula_to_core_with_context(result, line, "sheet_eval_formula", s)
        }
        _ => Ok(cell_to_core(cell)),
    }
}

pub fn eval_sheet_cell_cached(
    engine: &mut CachedFormulaEngine,
    sheet: &Sheet,
    notation: &str,
    line: usize,
) -> PipResult<Value> {
    let cell = sheet.get_a1(notation).map_err(|e| {
        PipError::runtime(line, format!("Invalid cell notation '{}': {}", notation, e))
    })?;
    match cell {
        CellValue::String(s) if s.trim_start().starts_with('=') => {
            let base_cell = sheet.get_a1_addr(notation).map_err(|e| {
                PipError::runtime(line, format!("Invalid cell notation '{}': {}", notation, e))
            })?;
            let context = format!("cell {}", notation);
            let compiled = engine.compile_cached(s, line, &context)?;
            engine.evaluate(&compiled, sheet, Some(base_cell), line, &context, s)
        }
        _ => Ok(cell_to_core(cell)),
    }
}

struct SheetResolver<'a> {
    sheet: &'a Sheet,
    base_cell: Option<CellAddress>,
}

impl ValueResolver for SheetResolver<'_> {
    fn get_cell(&self, addr: &CellAddress) -> FormulaValue {
        let row = addr.row as usize;
        let col = addr.col as usize;
        match self.sheet.get(row, col) {
            Ok(cell) => cell_to_formula(cell),
            Err(_) => FormulaValue::Error(ErrorValue::Ref),
        }
    }

    fn get_range(&self, range: &CellRange) -> Vec<FormulaValue> {
        let normalized = range.normalized();
        let rows = normalized.rows() as usize;
        let cols = normalized.cols() as usize;
        let mut values = Vec::with_capacity(rows);
        for r in 0..rows {
            let mut row = Vec::with_capacity(cols);
            for c in 0..cols {
                let addr = CellAddress::new(
                    normalized.start.row + r as u32,
                    normalized.start.col + c as u32,
                );
                row.push(self.get_cell(&addr));
            }
            values.push(FormulaValue::Array(row));
        }
        values
    }

    fn current_cell(&self) -> Option<CellAddress> {
        self.base_cell
    }
}

fn validate_arg_count(
    min_args: usize,
    max_args: Option<usize>,
    provided: usize,
    name: &str,
    line: usize,
) -> PipResult<()> {
    if provided < min_args {
        return Err(PipError::runtime(
            line,
            format!(
                "Function '{}' expects {}, got {}",
                name,
                format_expected_args(min_args, max_args),
                provided
            ),
        ));
    }
    if let Some(max) = max_args {
        if provided > max {
            return Err(PipError::runtime(
                line,
                format!(
                    "Function '{}' expects {}, got {}",
                    name,
                    format_expected_args(min_args, max_args),
                    provided
                ),
            ));
        }
    }
    Ok(())
}

fn format_expected_args(min_args: usize, max_args: Option<usize>) -> String {
    match max_args {
        Some(max) if max == min_args => {
            if min_args == 1 {
                "1 argument".to_string()
            } else {
                format!("{min_args} arguments")
            }
        }
        Some(max) => format!("{min_args}..{max} arguments"),
        None => format!("{min_args}+ arguments"),
    }
}

fn cell_to_formula(cell: &CellValue) -> FormulaValue {
    match cell {
        CellValue::Null => FormulaValue::Empty,
        CellValue::Bool(b) => FormulaValue::Bool(*b),
        CellValue::Int(i) => FormulaValue::Int(*i),
        CellValue::Float(f) => FormulaValue::Float(*f),
        CellValue::String(s) => FormulaValue::String(s.clone()),
    }
}

fn cell_to_core(cell: &CellValue) -> Value {
    match cell {
        CellValue::Null => Value::Null,
        CellValue::Bool(b) => Value::Bool(*b),
        CellValue::Int(i) => Value::Int(*i),
        CellValue::Float(f) => Value::Float(*f),
        CellValue::String(s) => Value::String(s.clone()),
    }
}

fn core_to_formula(value: &Value, line: usize) -> PipResult<FormulaValue> {
    match value {
        Value::Null => Ok(FormulaValue::Empty),
        Value::Bool(b) => Ok(FormulaValue::Bool(*b)),
        Value::Int(i) => Ok(FormulaValue::Int(*i)),
        Value::Float(f) => Ok(FormulaValue::Float(*f)),
        Value::String(s) => Ok(FormulaValue::String(s.clone())),
        Value::Array(items) => {
            let converted = items
                .iter()
                .map(|item| core_to_formula(item, line))
                .collect::<PipResult<Vec<_>>>()?;
            Ok(FormulaValue::Array(converted))
        }
        Value::Sheet(sheet) => {
            let header_offset = match sheet.column_names() {
                Some(names) => {
                    if sheet
                        .data()
                        .first()
                        .map(|row| {
                            names.iter().enumerate().all(|(idx, name)| {
                                row.get(idx)
                                    .map(|cell| cell.as_str() == name.as_str())
                                    .unwrap_or(false)
                            })
                        })
                        .unwrap_or(false)
                    {
                        1
                    } else {
                        0
                    }
                }
                None => 0,
            };

            let mut rows = Vec::new();
            for row in sheet.data().iter().skip(header_offset) {
                let mut values = Vec::new();
                for cell in row {
                    values.push(cell_to_formula(cell));
                }
                rows.push(FormulaValue::Array(values));
            }
            Ok(FormulaValue::Array(rows))
        }
        Value::Object(_) => Err(PipError::runtime(
            line,
            "Formula arguments cannot be objects",
        )),
        Value::Table(_) => Err(PipError::runtime(
            line,
            "Formula arguments cannot be tables",
        )),
        Value::Function { .. } => Err(PipError::runtime(
            line,
            "Formula arguments cannot be functions",
        )),
        Value::Lambda { .. } => Err(PipError::runtime(
            line,
            "Formula arguments cannot be lambdas",
        )),
    }
}

fn formula_to_core(value: FormulaValue, line: usize) -> PipResult<Value> {
    match value {
        FormulaValue::Empty => Ok(Value::Null),
        FormulaValue::Bool(b) => Ok(Value::Bool(b)),
        FormulaValue::Int(i) => Ok(Value::Int(i)),
        FormulaValue::Float(f) => Ok(Value::Float(f)),
        FormulaValue::String(s) => Ok(Value::String(s)),
        FormulaValue::Array(items) => {
            let converted = items
                .into_iter()
                .map(|item| formula_to_core(item, line))
                .collect::<PipResult<Vec<_>>>()?;
            Ok(Value::Array(converted))
        }
        FormulaValue::Error(err) => Err(PipError::runtime(
            line,
            format!("Formula error: {}", err.label()),
        )),
    }
}

fn formula_to_core_with_context(
    value: FormulaValue,
    line: usize,
    context: &str,
    formula: &str,
) -> PipResult<Value> {
    match value {
        FormulaValue::Error(err) => Err(PipError::runtime(
            line,
            format_formula_error(context, formula, err.label()),
        )),
        other => formula_to_core(other, line),
    }
}

fn format_formula_error(context: &str, formula: &str, err: impl Display) -> String {
    format!(
        "Formula error in {}: {} (formula: \"{}\")",
        context, err, formula
    )
}
