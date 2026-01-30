//! # Piptable Formulas
//!
//! Formula parsing, compilation, and evaluation engine.
//! Includes formula registry for standard functions (SUM, VLOOKUP, etc.)

use piptable_primitives::{CellAddress, CellRange, ErrorValue, Value};
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
    /// Range reference
    RangeRef(CellRange),
    /// Sheet-qualified cell reference (e.g., Sheet1!A1)
    SheetCellRef { sheet: String, addr: CellAddress },
    /// Sheet-qualified range reference (e.g., Sheet1!A1:B2)
    SheetRangeRef { sheet: String, range: CellRange },
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
            FormulaExpr::RangeRef(range) => Ok(Value::Array(context.get_range(range))),
            FormulaExpr::SheetCellRef { sheet, addr } => Ok(context.get_sheet_cell(sheet, addr)),
            FormulaExpr::SheetRangeRef { sheet, range } => {
                Ok(Value::Array(context.get_sheet_range(sheet, range)))
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

        // Lookup functions
        self.register(
            "VLOOKUP",
            FunctionDefinition::range(
                3,
                4,
                vec![ParamType::Any, ParamType::Range, ParamType::Number],
                ReturnType::Any,
                functions::not_implemented,
            ),
        );
        self.register(
            "HLOOKUP",
            FunctionDefinition::range(
                3,
                4,
                vec![ParamType::Any, ParamType::Range, ParamType::Number],
                ReturnType::Any,
                functions::not_implemented,
            ),
        );
        self.register(
            "INDEX",
            FunctionDefinition::range(
                2,
                3,
                vec![ParamType::Range, ParamType::Number],
                ReturnType::Any,
                functions::not_implemented,
            ),
        );
        self.register(
            "MATCH",
            FunctionDefinition::range(
                2,
                3,
                vec![ParamType::Any, ParamType::Range],
                ReturnType::Number,
                functions::not_implemented,
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
        self.ranges
            .get(range)
            .cloned()
            .unwrap_or_else(|| Vec::with_capacity(0))
    }
}

fn collect_dependencies(expr: &FormulaExpr, deps: &mut HashSet<CellAddress>) {
    match expr {
        FormulaExpr::Literal(_) => {}
        FormulaExpr::CellRef(addr) => {
            deps.insert(*addr);
        }
        FormulaExpr::RangeRef(range) => {
            for addr in range.iter() {
                deps.insert(addr);
            }
        }
        FormulaExpr::SheetCellRef { .. } => {}
        FormulaExpr::SheetRangeRef { .. } => {}
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
}
