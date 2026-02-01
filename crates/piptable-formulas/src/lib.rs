//! # Piptable Formulas
//!
//! Formula parsing, compilation, and evaluation engine.
//! Includes formula registry for standard functions (SUM, VLOOKUP, etc.)

use piptable_primitives::{CellAddress, CellRange, ErrorValue, R1C1Ref, Value};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

pub mod functions;
pub mod parser;

/// Compiled formula ready for evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledFormula {
    /// The original formula text
    pub source: String,
    /// Compiled AST or bytecode
    pub ast: FormulaExpr,
    /// Cell dependencies for recalculation
    pub dependencies: Vec<CellAddress>,
    /// Hash for cache invalidation
    pub hash: u64,
}

/// Formula expression AST
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FormulaExpr {
    /// Literal value
    Literal(Value),
    /// Cell reference
    CellRef(CellAddress),
    /// R1C1-style cell reference
    R1C1Ref(R1C1Ref),
    /// Range reference
    RangeRef(CellRange),
    /// R1C1-style range reference
    R1C1RangeRef { start: R1C1Ref, end: R1C1Ref },
    /// Sheet-qualified cell reference (e.g., Sheet1!A1)
    SheetCellRef { sheet: String, addr: CellAddress },
    /// Sheet-qualified R1C1-style cell reference
    SheetR1C1Ref { sheet: String, addr: R1C1Ref },
    /// Sheet-qualified range reference (e.g., Sheet1!A1:B2)
    SheetRangeRef { sheet: String, range: CellRange },
    /// Sheet-qualified R1C1-style range reference
    SheetR1C1RangeRef {
        sheet: String,
        start: R1C1Ref,
        end: R1C1Ref,
    },
    /// Function call
    FunctionCall {
        name: String,
        args: Vec<FormulaExpr>,
    },
    /// Binary operation
    BinaryOp {
        op: BinaryOperator,
        left: Box<FormulaExpr>,
        right: Box<FormulaExpr>,
    },
    /// Unary operation
    UnaryOp {
        op: UnaryOperator,
        expr: Box<FormulaExpr>,
    },
}

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Power,
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    And,
    Or,
    Concat,
}

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryOperator {
    Negate,
    Not,
    Percent,
}

/// Formula compilation and caching engine
pub struct FormulaEngine {
    /// Cache of compiled formulas
    cache: HashMap<CellAddress, CompiledFormula>,
    /// Function registry
    functions: FunctionRegistry,
}

impl FormulaEngine {
    /// Create a new formula engine
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            functions: FunctionRegistry::default(),
        }
    }

    /// Compile a formula string
    pub fn compile(&mut self, formula: &str) -> Result<CompiledFormula, FormulaError> {
        let ast = parser::parse_formula(formula)?;
        let mut deps = HashSet::new();
        collect_dependencies(&ast, &mut deps);
        let dependencies = deps.into_iter().collect();
        let hash = hash_formula(formula);
        Ok(CompiledFormula {
            source: formula.to_string(),
            ast,
            dependencies,
            hash,
        })
    }

    /// Access registered functions
    pub fn functions(&self) -> &FunctionRegistry {
        &self.functions
    }

    /// Set a formula for a cell
    pub fn set_formula(&mut self, cell: CellAddress, formula: &str) -> Result<(), FormulaError> {
        let compiled = self.compile(formula)?;
        self.cache.insert(cell, compiled);
        Ok(())
    }

    /// Get compiled formula for a cell
    pub fn get_formula(&self, cell: &CellAddress) -> Option<&CompiledFormula> {
        self.cache.get(cell)
    }

    /// Clear cache for a cell
    pub fn invalidate(&mut self, cell: &CellAddress) {
        self.cache.remove(cell);
    }

    /// Evaluate a compiled formula against a context
    pub fn evaluate(
        &self,
        compiled: &CompiledFormula,
        context: &impl ValueResolver,
    ) -> Result<Value, FormulaError> {
        self.eval_expr(&compiled.ast, context)
    }

    fn eval_expr(
        &self,
        expr: &FormulaExpr,
        context: &impl ValueResolver,
    ) -> Result<Value, FormulaError> {
        match expr {
            FormulaExpr::Literal(value) => Ok(value.clone()),
            FormulaExpr::CellRef(addr) => Ok(context.get_cell(addr)),
            FormulaExpr::R1C1Ref(r1c1) => {
                let Some(base) = context.current_cell() else {
                    return Ok(Value::Error(ErrorValue::Ref));
                };
                match r1c1.resolve(Some(base)) {
                    Ok(addr) => Ok(context.get_cell(&addr)),
                    Err(_) => Ok(Value::Error(ErrorValue::Ref)),
                }
            }
            FormulaExpr::RangeRef(range) => Ok(Value::Array(context.get_range(range))),
            FormulaExpr::R1C1RangeRef { start, end } => {
                let Some(base) = context.current_cell() else {
                    return Ok(Value::Error(ErrorValue::Ref));
                };
                let start = start.resolve(Some(base));
                let end = end.resolve(Some(base));
                match (start, end) {
                    (Ok(start), Ok(end)) => {
                        let range = CellRange::new(start, end);
                        Ok(Value::Array(context.get_range(&range)))
                    }
                    _ => Ok(Value::Error(ErrorValue::Ref)),
                }
            }
            FormulaExpr::SheetCellRef { sheet, addr } => Ok(context.get_sheet_cell(sheet, addr)),
            FormulaExpr::SheetR1C1Ref { sheet, addr } => {
                let Some(base) = context.current_cell() else {
                    return Ok(Value::Error(ErrorValue::Ref));
                };
                match addr.resolve(Some(base)) {
                    Ok(resolved) => Ok(context.get_sheet_cell(sheet, &resolved)),
                    Err(_) => Ok(Value::Error(ErrorValue::Ref)),
                }
            }
            FormulaExpr::SheetRangeRef { sheet, range } => {
                Ok(Value::Array(context.get_sheet_range(sheet, range)))
            }
            FormulaExpr::SheetR1C1RangeRef { sheet, start, end } => {
                let Some(base) = context.current_cell() else {
                    return Ok(Value::Error(ErrorValue::Ref));
                };
                let start = start.resolve(Some(base));
                let end = end.resolve(Some(base));
                match (start, end) {
                    (Ok(start), Ok(end)) => {
                        let range = CellRange::new(start, end);
                        Ok(Value::Array(context.get_sheet_range(sheet, &range)))
                    }
                    _ => Ok(Value::Error(ErrorValue::Ref)),
                }
            }
            FormulaExpr::UnaryOp { op, expr } => {
                let value = self.eval_expr(expr, context)?;
                if let Value::Error(err) = value {
                    return Ok(Value::Error(err));
                }
                Ok(eval_unary(*op, value))
            }
            FormulaExpr::BinaryOp { op, left, right } => {
                let left_val = self.eval_expr(left, context)?;
                if let Value::Error(err) = left_val {
                    return Ok(Value::Error(err));
                }
                let right_val = self.eval_expr(right, context)?;
                if let Value::Error(err) = right_val {
                    return Ok(Value::Error(err));
                }
                Ok(eval_binary(*op, left_val, right_val))
            }
            FormulaExpr::FunctionCall { name, args } => {
                let def = self
                    .functions
                    .get(name)
                    .ok_or_else(|| FormulaError::UnknownFunction(name.clone()))?;

                def.validate_arg_count(args.len()).map_err(|expected| {
                    FormulaError::InvalidArgCount(name.clone(), expected, args.len())
                })?;

                let mut evaled_args = Vec::with_capacity(args.len());
                for arg in args {
                    let value = self.eval_expr(arg, context)?;
                    if let Value::Error(err) = &value {
                        return Ok(Value::Error(err.clone()));
                    }
                    evaled_args.push(value);
                }

                Ok((def.eval)(&evaled_args))
            }
        }
    }
}

/// Registry of available functions
pub struct FunctionRegistry {
    functions: HashMap<String, FunctionDefinition>,
}

impl Default for FunctionRegistry {
    fn default() -> Self {
        let mut registry = Self {
            functions: HashMap::new(),
        };

        // Register standard functions
        registry.register_standard_functions();
        registry
    }
}

impl FunctionRegistry {
    /// Register standard Excel-compatible functions
    fn register_standard_functions(&mut self) {
        // Math functions
        self.register(
            "SUM",
            FunctionDefinition::variadic(1, ParamType::Number, ReturnType::Number, functions::sum),
        );
        self.register(
            "AVERAGE",
            FunctionDefinition::variadic(
                1,
                ParamType::Number,
                ReturnType::Number,
                functions::average,
            ),
        );
        self.register(
            "AVG",
            FunctionDefinition::variadic(
                1,
                ParamType::Number,
                ReturnType::Number,
                functions::average,
            ),
        );
        self.register(
            "COUNT",
            FunctionDefinition::variadic(
                1,
                ParamType::Number,
                ReturnType::Number,
                functions::count,
            ),
        );
        self.register(
            "COUNTA",
            FunctionDefinition::variadic(1, ParamType::Any, ReturnType::Number, functions::counta),
        );
        self.register(
            "MAX",
            FunctionDefinition::variadic(1, ParamType::Number, ReturnType::Number, functions::max),
        );
        self.register(
            "MIN",
            FunctionDefinition::variadic(1, ParamType::Number, ReturnType::Number, functions::min),
        );

        // Logical functions
        self.register(
            "IF",
            FunctionDefinition::range(
                2,
                3,
                vec![ParamType::Logical, ParamType::Any, ParamType::Any],
                ReturnType::Any,
                functions::if_fn,
            ),
        );
        self.register(
            "AND",
            FunctionDefinition::variadic(
                1,
                ParamType::Logical,
                ReturnType::Logical,
                functions::and_fn,
            ),
        );
        self.register(
            "OR",
            FunctionDefinition::variadic(
                1,
                ParamType::Logical,
                ReturnType::Logical,
                functions::or_fn,
            ),
        );
        self.register(
            "NOT",
            FunctionDefinition::fixed(
                vec![ParamType::Logical],
                ReturnType::Logical,
                functions::not_fn,
            ),
        );

        // Text functions
        self.register(
            "CONCAT",
            FunctionDefinition::variadic(1, ParamType::Any, ReturnType::Text, functions::concat),
        );
        self.register(
            "CONCATENATE",
            FunctionDefinition::variadic(1, ParamType::Any, ReturnType::Text, functions::concat),
        );
        self.register(
            "LEN",
            FunctionDefinition::fixed(vec![ParamType::Text], ReturnType::Number, functions::len),
        );
        self.register(
            "LEFT",
            FunctionDefinition::range(
                1,
                2,
                vec![ParamType::Text, ParamType::Number],
                ReturnType::Text,
                functions::left,
            ),
        );
        self.register(
            "RIGHT",
            FunctionDefinition::range(
                1,
                2,
                vec![ParamType::Text, ParamType::Number],
                ReturnType::Text,
                functions::right,
            ),
        );

        // Date functions
        self.register(
            "TODAY",
            FunctionDefinition::fixed(vec![], ReturnType::Number, functions::today),
        );
        self.register(
            "NOW",
            FunctionDefinition::fixed(vec![], ReturnType::Number, functions::now),
        );
        self.register(
            "DATE",
            FunctionDefinition::fixed(
                vec![ParamType::Number, ParamType::Number, ParamType::Number],
                ReturnType::Number,
                functions::date,
            ),
        );

        // Lookup functions
        self.register(
            "VLOOKUP",
            FunctionDefinition::range(
                3,
                4,
                vec![ParamType::Any, ParamType::Range, ParamType::Number],
                ReturnType::Any,
                functions::vlookup,
            ),
        );
        self.register(
            "HLOOKUP",
            FunctionDefinition::range(
                3,
                4,
                vec![ParamType::Any, ParamType::Range, ParamType::Number],
                ReturnType::Any,
                functions::hlookup,
            ),
        );
        self.register(
            "INDEX",
            FunctionDefinition::range(
                2,
                3,
                vec![ParamType::Range, ParamType::Number],
                ReturnType::Any,
                functions::index,
            ),
        );
        self.register(
            "MATCH",
            FunctionDefinition::range(
                2,
                3,
                vec![ParamType::Any, ParamType::Range],
                ReturnType::Number,
                functions::match_fn,
            ),
        );
        self.register(
            "XLOOKUP",
            FunctionDefinition::range(
                3,
                7,
                vec![ParamType::Any, ParamType::Range, ParamType::Range],
                ReturnType::Any,
                functions::xlookup,
            ),
        );
        self.register(
            "OFFSET",
            FunctionDefinition::range(
                3,
                5,
                vec![ParamType::Range, ParamType::Number, ParamType::Number],
                ReturnType::Range,
                functions::offset,
            ),
        );
    }

    fn register(&mut self, name: &str, def: FunctionDefinition) {
        self.functions.insert(name.to_uppercase(), def);
    }

    /// Check if a function exists
    pub fn has_function(&self, name: &str) -> bool {
        self.functions.contains_key(&name.to_uppercase())
    }

    /// Get a function definition by name
    pub fn get(&self, name: &str) -> Option<&FunctionDefinition> {
        self.functions.get(&name.to_uppercase())
    }
}

impl Default for FormulaEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Function definition
pub struct FunctionDefinition {
    pub min_args: usize,
    pub max_args: Option<usize>,
    pub metadata: FunctionMetadata,
    pub eval: FunctionImpl,
}

impl FunctionDefinition {
    /// Fixed number of arguments
    pub fn fixed(params: Vec<ParamType>, return_type: ReturnType, eval: FunctionImpl) -> Self {
        let args = params.len();
        Self {
            min_args: args,
            max_args: Some(args),
            metadata: FunctionMetadata {
                params,
                variadic: None,
                return_type,
            },
            eval,
        }
    }

    /// Variable number of arguments
    pub fn variadic(
        min: usize,
        variadic: ParamType,
        return_type: ReturnType,
        eval: FunctionImpl,
    ) -> Self {
        Self {
            min_args: min,
            max_args: None,
            metadata: FunctionMetadata {
                params: Vec::new(),
                variadic: Some(variadic),
                return_type,
            },
            eval,
        }
    }

    /// Range of arguments
    pub fn range(
        min: usize,
        max: usize,
        params: Vec<ParamType>,
        return_type: ReturnType,
        eval: FunctionImpl,
    ) -> Self {
        Self {
            min_args: min,
            max_args: Some(max),
            metadata: FunctionMetadata {
                params,
                variadic: None,
                return_type,
            },
            eval,
        }
    }

    fn validate_arg_count(&self, provided: usize) -> Result<(), String> {
        if provided < self.min_args {
            return Err(self.expected_args_label());
        }
        if let Some(max) = self.max_args {
            if provided > max {
                return Err(self.expected_args_label());
            }
        }
        Ok(())
    }

    fn expected_args_label(&self) -> String {
        match self.max_args {
            Some(max) if max == self.min_args => format!("{}", self.min_args),
            Some(max) => format!("{}..{}", self.min_args, max),
            None => format!("{}+", self.min_args),
        }
    }
}

/// Formula errors
#[derive(Debug, thiserror::Error)]
pub enum FormulaError {
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Unknown function: {0}")]
    UnknownFunction(String),
    #[error("Invalid argument count for {0}: expected {1}, got {2}")]
    InvalidArgCount(String, String, usize),
    #[error("Circular reference detected")]
    CircularReference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParamType {
    Any,
    Number,
    Logical,
    Text,
    Range,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReturnType {
    Any,
    Number,
    Logical,
    Text,
    Range,
}

pub type FunctionImpl = fn(&[Value]) -> Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionMetadata {
    pub params: Vec<ParamType>,
    pub variadic: Option<ParamType>,
    pub return_type: ReturnType,
}

pub trait ValueResolver {
    fn get_cell(&self, addr: &CellAddress) -> Value;
    fn get_range(&self, range: &CellRange) -> Vec<Value>;
    fn current_cell(&self) -> Option<CellAddress> {
        None
    }
    fn get_sheet_cell(&self, _sheet: &str, addr: &CellAddress) -> Value {
        self.get_cell(addr)
    }
    fn get_sheet_range(&self, _sheet: &str, range: &CellRange) -> Vec<Value> {
        self.get_range(range)
    }
}

#[derive(Default)]
pub struct EvalContext {
    cells: HashMap<CellAddress, Value>,
    ranges: HashMap<CellRange, Vec<Value>>,
}

impl EvalContext {
    pub fn with_cells(cells: HashMap<CellAddress, Value>) -> Self {
        Self {
            cells,
            ranges: HashMap::new(),
        }
    }

    pub fn with_ranges(ranges: HashMap<CellRange, Vec<Value>>) -> Self {
        Self {
            cells: HashMap::new(),
            ranges,
        }
    }
}

impl ValueResolver for EvalContext {
    fn get_cell(&self, addr: &CellAddress) -> Value {
        self.cells.get(addr).cloned().unwrap_or(Value::Empty)
    }

    fn get_range(&self, range: &CellRange) -> Vec<Value> {
        let Some(values) = self.ranges.get(range) else {
            return Vec::with_capacity(0);
        };
        if values.iter().all(|v| matches!(v, Value::Array(_))) {
            return values.clone();
        }

        let rows = range.rows() as usize;
        let cols = range.cols() as usize;
        if rows.saturating_mul(cols) == values.len() && rows > 0 && cols > 0 {
            let mut out = Vec::with_capacity(rows);
            for r in 0..rows {
                let mut row = Vec::with_capacity(cols);
                for c in 0..cols {
                    let idx = r * cols + c;
                    row.push(values[idx].clone());
                }
                out.push(Value::Array(row));
            }
            return out;
        }

        values.clone()
    }
}

fn collect_dependencies(expr: &FormulaExpr, deps: &mut HashSet<CellAddress>) {
    match expr {
        FormulaExpr::Literal(_) => {}
        FormulaExpr::CellRef(addr) => {
            deps.insert(*addr);
        }
        FormulaExpr::R1C1Ref(_) => {}
        FormulaExpr::RangeRef(range) => {
            for addr in range.iter() {
                deps.insert(addr);
            }
        }
        FormulaExpr::R1C1RangeRef { .. } => {}
        FormulaExpr::SheetCellRef { .. } => {}
        FormulaExpr::SheetR1C1Ref { .. } => {}
        FormulaExpr::SheetRangeRef { .. } => {}
        FormulaExpr::SheetR1C1RangeRef { .. } => {}
        FormulaExpr::FunctionCall { args, .. } => {
            for arg in args {
                collect_dependencies(arg, deps);
            }
        }
        FormulaExpr::BinaryOp { left, right, .. } => {
            collect_dependencies(left, deps);
            collect_dependencies(right, deps);
        }
        FormulaExpr::UnaryOp { expr, .. } => {
            collect_dependencies(expr, deps);
        }
    }
}

fn hash_formula(formula: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    formula.hash(&mut hasher);
    hasher.finish()
}

fn eval_unary(op: UnaryOperator, value: Value) -> Value {
    match op {
        UnaryOperator::Negate => match value {
            Value::Int(n) => Value::Int(-n),
            Value::Float(f) => Value::Float(-f),
            _ => Value::Error(ErrorValue::Value),
        },
        UnaryOperator::Not => match value {
            Value::Bool(b) => Value::Bool(!b),
            Value::Int(n) => Value::Bool(n == 0),
            Value::Float(f) => Value::Bool(f == 0.0),
            Value::Empty => Value::Bool(true),
            _ => Value::Error(ErrorValue::Value),
        },
        UnaryOperator::Percent => match value {
            Value::Int(n) => Value::Float(n as f64 / 100.0),
            Value::Float(f) => Value::Float(f / 100.0),
            _ => Value::Error(ErrorValue::Value),
        },
    }
}

fn eval_binary(op: BinaryOperator, left: Value, right: Value) -> Value {
    match op {
        BinaryOperator::Add => numeric_op(left, right, |l, r| l + r),
        BinaryOperator::Subtract => numeric_op(left, right, |l, r| l - r),
        BinaryOperator::Multiply => numeric_op(left, right, |l, r| l * r),
        BinaryOperator::Divide => {
            if is_zero(&right) {
                Value::Error(ErrorValue::Div0)
            } else {
                numeric_op(left, right, |l, r| l / r)
            }
        }
        BinaryOperator::Power => numeric_op(left, right, |l, r| l.powf(r)),
        BinaryOperator::Equal => Value::Bool(left == right),
        BinaryOperator::NotEqual => Value::Bool(left != right),
        BinaryOperator::LessThan => Value::Bool(compare_numbers(&left, &right, |l, r| l < r)),
        BinaryOperator::LessThanOrEqual => {
            Value::Bool(compare_numbers(&left, &right, |l, r| l <= r))
        }
        BinaryOperator::GreaterThan => Value::Bool(compare_numbers(&left, &right, |l, r| l > r)),
        BinaryOperator::GreaterThanOrEqual => {
            Value::Bool(compare_numbers(&left, &right, |l, r| l >= r))
        }
        BinaryOperator::And => logical_op(left, right, |l, r| l && r),
        BinaryOperator::Or => logical_op(left, right, |l, r| l || r),
        BinaryOperator::Concat => Value::String(format!(
            "{}{}",
            value_to_string(left),
            value_to_string(right)
        )),
    }
}

fn numeric_op(left: Value, right: Value, op: fn(f64, f64) -> f64) -> Value {
    match (left, right) {
        (Value::Int(l), Value::Int(r)) => Value::Float(op(l as f64, r as f64)),
        (Value::Float(l), Value::Float(r)) => Value::Float(op(l, r)),
        (Value::Int(l), Value::Float(r)) => Value::Float(op(l as f64, r)),
        (Value::Float(l), Value::Int(r)) => Value::Float(op(l, r as f64)),
        _ => Value::Error(ErrorValue::Value),
    }
}

fn is_zero(value: &Value) -> bool {
    match value {
        Value::Int(n) => *n == 0,
        Value::Float(f) => *f == 0.0,
        _ => false,
    }
}

fn compare_numbers(left: &Value, right: &Value, cmp: fn(f64, f64) -> bool) -> bool {
    match (left, right) {
        (Value::Int(l), Value::Int(r)) => cmp(*l as f64, *r as f64),
        (Value::Float(l), Value::Float(r)) => cmp(*l, *r),
        (Value::Int(l), Value::Float(r)) => cmp(*l as f64, *r),
        (Value::Float(l), Value::Int(r)) => cmp(*l, *r as f64),
        _ => false,
    }
}

fn logical_op(left: Value, right: Value, op: fn(bool, bool) -> bool) -> Value {
    let to_bool = |value: &Value| -> Option<bool> {
        match value {
            Value::Bool(b) => Some(*b),
            Value::Int(n) => Some(*n != 0),
            Value::Float(f) => {
                if f.is_nan() {
                    None
                } else {
                    Some(*f != 0.0)
                }
            }
            _ => None,
        }
    };

    match (to_bool(&left), to_bool(&right)) {
        (Some(l), Some(r)) => Value::Bool(op(l, r)),
        _ => Value::Error(ErrorValue::Value),
    }
}

fn value_to_string(value: Value) -> String {
    match value {
        Value::Empty => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Int(n) => n.to_string(),
        Value::Float(f) => f.to_string(),
        Value::String(s) => s,
        Value::Error(err) => err.label().to_string(),
        Value::Array(_) => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_registry() {
        let registry = FunctionRegistry::default();
        assert!(registry.has_function("SUM"));
        assert!(registry.has_function("sum")); // Case insensitive
        assert!(registry.has_function("VLOOKUP"));
        assert!(!registry.has_function("UNKNOWN"));
    }

    #[test]
    fn test_registry_metadata() {
        let registry = FunctionRegistry::default();
        let sum = registry.get("SUM").expect("SUM registered");
        assert_eq!(sum.min_args, 1);
        assert_eq!(sum.max_args, None);
        assert_eq!(sum.metadata.variadic, Some(ParamType::Number));
        assert_eq!(sum.metadata.return_type, ReturnType::Number);
    }

    #[test]
    fn test_dispatch_unknown_function() {
        let engine = FormulaEngine::new();
        let expr = FormulaExpr::FunctionCall {
            name: "NOPE".to_string(),
            args: vec![],
        };
        let ctx = EvalContext::default();
        let err = engine.eval_expr(&expr, &ctx).unwrap_err();
        assert!(matches!(err, FormulaError::UnknownFunction(_)));
    }

    #[test]
    fn test_dispatch_arg_count_error() {
        let engine = FormulaEngine::new();
        let expr = FormulaExpr::FunctionCall {
            name: "SUM".to_string(),
            args: vec![],
        };
        let ctx = EvalContext::default();
        let err = engine.eval_expr(&expr, &ctx).unwrap_err();
        assert!(matches!(err, FormulaError::InvalidArgCount(_, _, _)));
    }

    #[test]
    fn test_error_propagation() {
        let engine = FormulaEngine::new();
        let expr = FormulaExpr::FunctionCall {
            name: "SUM".to_string(),
            args: vec![
                FormulaExpr::Literal(Value::Error(ErrorValue::Div0)),
                FormulaExpr::Literal(Value::Int(3)),
            ],
        };
        let ctx = EvalContext::default();
        let value = engine.eval_expr(&expr, &ctx).expect("eval ok");
        assert_eq!(value, Value::Error(ErrorValue::Div0));
    }

    #[test]
    fn test_evaluate_arithmetic_and_percent() {
        let mut engine = FormulaEngine::new();
        let compiled = engine.compile("=5%+1").unwrap();
        let ctx = EvalContext::default();
        let value = engine.evaluate(&compiled, &ctx).expect("eval ok");
        assert!(matches!(value, Value::Float(f) if (f - 1.05).abs() < 1e-9));
    }

    #[test]
    fn test_evaluate_concat_and_compare() {
        let mut engine = FormulaEngine::new();
        let compiled = engine.compile("=\"a\"&\"b\"").unwrap();
        let ctx = EvalContext::default();
        let value = engine.evaluate(&compiled, &ctx).expect("eval ok");
        assert_eq!(value, Value::String("ab".to_string()));

        let compiled = engine.compile("=1<2").unwrap();
        let value = engine.evaluate(&compiled, &ctx).expect("eval ok");
        assert_eq!(value, Value::Bool(true));
    }

    #[test]
    fn test_evaluate_cells_and_ranges() {
        let mut engine = FormulaEngine::new();
        let compiled = engine.compile("=A1+1").unwrap();
        let mut cells = HashMap::new();
        cells.insert(CellAddress::new(0, 0), Value::Int(2));
        let ctx = EvalContext::with_cells(cells);
        let value = engine.evaluate(&compiled, &ctx).expect("eval ok");
        assert!(matches!(value, Value::Float(f) if (f - 3.0).abs() < 1e-9));

        let compiled = engine.compile("=SUM(A1:A2)").unwrap();
        let mut ranges = HashMap::new();
        ranges.insert(
            CellRange::new(CellAddress::new(0, 0), CellAddress::new(1, 0)),
            vec![Value::Int(2), Value::Int(3)],
        );
        let ctx = EvalContext::with_ranges(ranges);
        let value = engine.evaluate(&compiled, &ctx).expect("eval ok");
        assert!(matches!(value, Value::Float(f) if (f - 5.0).abs() < 1e-9));
    }

    #[test]
    fn test_engine_cache_helpers() {
        let mut engine = FormulaEngine::new();
        let cell = CellAddress::new(0, 0);
        engine.set_formula(cell, "=1+2").expect("set ok");
        let compiled = engine.get_formula(&cell).expect("cached");
        assert_eq!(compiled.source, "=1+2");
        engine.invalidate(&cell);
        assert!(engine.get_formula(&cell).is_none());
    }

    #[test]
    fn test_sheet_qualified_refs_and_defaults() {
        let mut engine = FormulaEngine::new();
        let compiled = engine.compile("=Sheet1!A1+SUM(Sheet1!A1:A2)").unwrap();
        let mut ranges = HashMap::new();
        ranges.insert(
            CellRange::new(CellAddress::new(0, 0), CellAddress::new(1, 0)),
            vec![Value::Int(2), Value::Int(3)],
        );
        let mut cells = HashMap::new();
        cells.insert(CellAddress::new(0, 0), Value::Int(2));
        let ctx = EvalContext { cells, ranges };
        let value = engine.evaluate(&compiled, &ctx).expect("eval ok");
        assert!(matches!(value, Value::Float(f) if (f - 7.0).abs() < 1e-9));
    }

    #[test]
    fn test_function_definition_arg_count_labels() {
        let fixed =
            FunctionDefinition::fixed(vec![ParamType::Number], ReturnType::Number, functions::sum);
        assert_eq!(fixed.expected_args_label(), "1");
        assert!(fixed.validate_arg_count(1).is_ok());
        assert!(fixed.validate_arg_count(2).is_err());

        let ranged = FunctionDefinition::range(
            2,
            3,
            vec![ParamType::Number, ParamType::Number, ParamType::Number],
            ReturnType::Number,
            functions::sum,
        );
        assert_eq!(ranged.expected_args_label(), "2..3");
        assert!(ranged.validate_arg_count(2).is_ok());
        assert!(ranged.validate_arg_count(4).is_err());

        let variadic =
            FunctionDefinition::variadic(1, ParamType::Number, ReturnType::Number, functions::sum);
        assert_eq!(variadic.expected_args_label(), "1+");
        assert!(variadic.validate_arg_count(10).is_ok());
    }

    #[test]
    fn test_unary_and_binary_error_paths() {
        let mut engine = FormulaEngine::new();
        let ctx = EvalContext::default();

        let compiled = engine.compile("=-\"a\"").unwrap();
        let value = engine.evaluate(&compiled, &ctx).expect("eval ok");
        assert_eq!(value, Value::Error(ErrorValue::Value));

        let compiled = engine.compile("=\"a\"^2").unwrap();
        let value = engine.evaluate(&compiled, &ctx).expect("eval ok");
        assert_eq!(value, Value::Error(ErrorValue::Value));

        let compiled = engine.compile("=1/0").unwrap();
        let value = engine.evaluate(&compiled, &ctx).expect("eval ok");
        assert_eq!(value, Value::Error(ErrorValue::Div0));

        let expr = FormulaExpr::BinaryOp {
            op: BinaryOperator::And,
            left: Box::new(FormulaExpr::Literal(Value::String("a".to_string()))),
            right: Box::new(FormulaExpr::Literal(Value::Bool(true))),
        };
        let value = engine.eval_expr(&expr, &ctx).expect("eval ok");
        assert_eq!(value, Value::Error(ErrorValue::Value));
    }

    #[test]
    fn test_collect_dependencies_and_hash() {
        let expr = FormulaExpr::BinaryOp {
            op: BinaryOperator::Add,
            left: Box::new(FormulaExpr::CellRef(CellAddress::new(0, 0))),
            right: Box::new(FormulaExpr::RangeRef(CellRange::new(
                CellAddress::new(0, 1),
                CellAddress::new(1, 1),
            ))),
        };
        let mut deps = HashSet::new();
        collect_dependencies(&expr, &mut deps);
        assert!(deps.contains(&CellAddress::new(0, 0)));
        assert!(deps.contains(&CellAddress::new(0, 1)));
        assert!(deps.contains(&CellAddress::new(1, 1)));
        assert_eq!(hash_formula("=A1"), hash_formula("=A1"));
    }

    #[test]
    fn test_value_to_string_error_and_array() {
        let err = Value::Error(ErrorValue::Value);
        assert_eq!(value_to_string(err), "#VALUE!");
        let arr = Value::Array(vec![Value::Int(1)]);
        assert_eq!(value_to_string(arr), "");
    }

    #[test]
    fn test_eval_context_defaults() {
        let ctx = EvalContext::default();
        let range = CellRange::new(CellAddress::new(0, 0), CellAddress::new(0, 0));
        assert!(ctx.get_range(&range).is_empty());
    }

    #[test]
    fn test_eval_context_cells_and_ranges_fallbacks() {
        let mut cells = HashMap::new();
        cells.insert(CellAddress::new(1, 2), Value::Int(7));
        let ctx = EvalContext::with_cells(cells);
        assert_eq!(ctx.get_cell(&CellAddress::new(1, 2)), Value::Int(7));
        assert_eq!(ctx.get_cell(&CellAddress::new(9, 9)), Value::Empty);

        let mut ranges = HashMap::new();
        let range = CellRange::new(CellAddress::new(0, 0), CellAddress::new(0, 1));
        ranges.insert(range, vec![Value::Int(1), Value::Int(2)]);
        let ctx = EvalContext::with_ranges(ranges);
        assert_eq!(
            ctx.get_range(&range),
            vec![Value::Array(vec![Value::Int(1), Value::Int(2)])]
        );
        let missing = CellRange::new(CellAddress::new(9, 9), CellAddress::new(9, 9));
        assert!(ctx.get_range(&missing).is_empty());
    }

    #[test]
    fn test_collect_dependencies_sheet_refs_noop() {
        let expr = FormulaExpr::SheetCellRef {
            sheet: "S".to_string(),
            addr: CellAddress::new(0, 0),
        };
        let mut deps = HashSet::new();
        collect_dependencies(&expr, &mut deps);
        assert!(deps.is_empty());

        let expr = FormulaExpr::SheetRangeRef {
            sheet: "S".to_string(),
            range: CellRange::new(CellAddress::new(0, 0), CellAddress::new(0, 1)),
        };
        collect_dependencies(&expr, &mut deps);
        assert!(deps.is_empty());
    }

    #[test]
    fn test_logical_op_error_on_non_coercible() {
        let engine = FormulaEngine::new();
        let ctx = EvalContext::default();
        let expr = FormulaExpr::BinaryOp {
            op: BinaryOperator::Or,
            left: Box::new(FormulaExpr::Literal(Value::Array(vec![Value::Int(1)]))),
            right: Box::new(FormulaExpr::Literal(Value::Bool(false))),
        };
        let value = engine.eval_expr(&expr, &ctx).expect("eval ok");
        assert_eq!(value, Value::Error(ErrorValue::Value));
    }

    #[test]
    fn test_value_to_string_basic_types() {
        assert_eq!(value_to_string(Value::Bool(true)), "true");
        assert_eq!(value_to_string(Value::Int(3)), "3");
        assert_eq!(value_to_string(Value::Float(1.25)), "1.25");
    }

    #[test]
    fn test_compare_non_numeric_returns_false() {
        let mut engine = FormulaEngine::new();
        let ctx = EvalContext::default();
        let compiled = engine.compile("=\"a\"<1").unwrap();
        let value = engine.evaluate(&compiled, &ctx).expect("eval ok");
        assert_eq!(value, Value::Bool(false));
    }
}
