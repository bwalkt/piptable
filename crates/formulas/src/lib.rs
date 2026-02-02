//! # Piptable Formulas
//!
//! Formula parsing, compilation, and evaluation engine.
//! Includes formula registry for standard functions (SUM, VLOOKUP, etc.)

use piptable_dag::{CellCoordinate, CellCoordinateRange, Dag, NodeRef};
use piptable_primitives::{CellAddress, CellRange, ErrorValue, R1C1Ref, Value};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

pub mod functions;
pub mod parser;
pub mod refs;
pub mod utils;

pub use refs::{
    extract_references, formula_to_relative_reference, FormulaReference,
    FormulaToRelativeReferenceOptions, ReferenceKind, ReferenceMode,
};
pub use utils::{
    balance_formula, balance_parentheses, balance_quotes, is_a_formula, is_alternate_formula,
    is_balanced_parenthesis, is_valid_formula, validate_formula,
};

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
    /// Dependency graph for recalculation ordering
    dag: Dag,
    /// Function registry
    functions: FunctionRegistry,
}

impl FormulaEngine {
    /// Create a new formula engine
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            dag: Dag::new(),
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
        let cell_ref = NodeRef::Cell(cell_to_coordinate(cell));
        self.dag.delete_cell(cell_ref.clone());
        for dep in &compiled.dependencies {
            let dep_ref = NodeRef::Cell(cell_to_coordinate(*dep));
            self.dag
                .add_node_input(cell_ref.clone(), dep_ref, true)
                .map_err(|_| FormulaError::CircularReference)?;
        }
        let mut ranges = Vec::new();
        collect_range_dependencies(&compiled.ast, &mut ranges);
        for range in ranges {
            self.add_range_dependency(&cell, &range)?;
        }
        self.cache.insert(cell, compiled);
        Ok(())
    }

    /// Get compiled formula for a cell
    pub fn get_formula(&self, cell: &CellAddress) -> Option<&CompiledFormula> {
        self.cache.get(cell)
    }

    /// Clear cache for a cell
    pub fn invalidate(&mut self, cell: &CellAddress) {
        let cell_ref = NodeRef::Cell(cell_to_coordinate(*cell));
        self.dag.delete_cell(cell_ref);
        self.cache.remove(cell);
    }

    /// Remove a formula and its dependencies from the DAG.
    pub fn remove_formula(&mut self, cell: &CellAddress) {
        let cell_ref = NodeRef::Cell(cell_to_coordinate(*cell));
        self.dag.delete_node(cell_ref);
        self.cache.remove(cell);
    }

    /// Mark a cell as dirty to trigger dependent recalculation ordering.
    pub fn mark_dirty(&mut self, cell: &CellAddress) {
        let cell_ref = NodeRef::Cell(cell_to_coordinate(*cell));
        self.dag.mark_cell_as_dirty(cell_ref);
    }

    /// Get dirty nodes in recalculation order.
    pub fn get_dirty_nodes(&mut self) -> Result<Vec<CellAddress>, FormulaError> {
        let nodes = self
            .dag
            .get_dirty_nodes()
            .map_err(|_| FormulaError::CircularReference)?;
        Ok(nodes
            .into_iter()
            .filter_map(|node| node.position)
            .filter_map(|pos| match pos {
                piptable_dag::NodePosition::Cell(cell) => Some(coordinate_to_cell(cell)),
                _ => None,
            })
            .collect())
    }

    /// Add a range dependency to a formula cell.
    pub fn add_range_dependency(
        &mut self,
        formula_cell: &CellAddress,
        range: &CellRange,
    ) -> Result<(), FormulaError> {
        let cell_ref = NodeRef::Cell(cell_to_coordinate(*formula_cell));
        let range_ref = NodeRef::Range(CellCoordinateRange {
            sheet_id: 0,
            start_row_index: range.start.row,
            start_column_index: range.start.col,
            end_row_index: range.end.row,
            end_column_index: range.end.col,
        });
        self.dag
            .add_node_input(cell_ref, range_ref, true)
            .map_err(|_| FormulaError::CircularReference)?;
        Ok(())
    }

    /// Get dependents of a cell in recalculation order.
    pub fn dependents_in_order(
        &self,
        cell: &CellAddress,
    ) -> Result<Vec<CellAddress>, FormulaError> {
        let nodes = self
            .dag
            .get_dependents(NodeRef::Cell(cell_to_coordinate(*cell)))
            .map_err(|_| FormulaError::CircularReference)?;
        Ok(nodes
            .into_iter()
            .filter_map(|node| node.position)
            .filter_map(|pos| match pos {
                piptable_dag::NodePosition::Cell(cell) => Some(coordinate_to_cell(cell)),
                _ => None,
            })
            .collect())
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

        // Additional Math functions
        self.register(
            "ABS",
            FunctionDefinition::fixed(vec![ParamType::Number], ReturnType::Number, functions::abs),
        );
        self.register(
            "ROUND",
            FunctionDefinition::range(
                1,
                2,
                vec![ParamType::Number, ParamType::Number],
                ReturnType::Number,
                functions::round,
            ),
        );
        self.register(
            "ROUNDUP",
            FunctionDefinition::range(
                1,
                2,
                vec![ParamType::Number, ParamType::Number],
                ReturnType::Number,
                functions::roundup,
            ),
        );
        self.register(
            "ROUNDDOWN",
            FunctionDefinition::range(
                1,
                2,
                vec![ParamType::Number, ParamType::Number],
                ReturnType::Number,
                functions::rounddown,
            ),
        );
        self.register(
            "PRODUCT",
            FunctionDefinition::variadic(
                1,
                ParamType::Number,
                ReturnType::Number,
                functions::product,
            ),
        );
        self.register(
            "MOD",
            FunctionDefinition::fixed(
                vec![ParamType::Number, ParamType::Number],
                ReturnType::Number,
                functions::mod_fn,
            ),
        );
        self.register(
            "POWER",
            FunctionDefinition::fixed(
                vec![ParamType::Number, ParamType::Number],
                ReturnType::Number,
                functions::power,
            ),
        );
        self.register(
            "SQRT",
            FunctionDefinition::fixed(vec![ParamType::Number], ReturnType::Number, functions::sqrt),
        );
        self.register(
            "INT",
            FunctionDefinition::fixed(vec![ParamType::Number], ReturnType::Number, functions::int),
        );
        self.register(
            "TRUNC",
            FunctionDefinition::range(
                1,
                2,
                vec![ParamType::Number, ParamType::Number],
                ReturnType::Number,
                functions::trunc,
            ),
        );
        self.register(
            "SIGN",
            FunctionDefinition::fixed(vec![ParamType::Number], ReturnType::Number, functions::sign),
        );
        self.register(
            "EVEN",
            FunctionDefinition::fixed(vec![ParamType::Number], ReturnType::Number, functions::even),
        );
        self.register(
            "ODD",
            FunctionDefinition::fixed(vec![ParamType::Number], ReturnType::Number, functions::odd),
        );
        self.register(
            "RAND",
            FunctionDefinition::fixed(vec![], ReturnType::Number, functions::rand),
        );
        self.register(
            "RANDBETWEEN",
            FunctionDefinition::fixed(
                vec![ParamType::Number, ParamType::Number],
                ReturnType::Number,
                functions::randbetween,
            ),
        );
        self.register(
            "PI",
            FunctionDefinition::fixed(vec![], ReturnType::Number, functions::pi),
        );
        self.register(
            "EXP",
            FunctionDefinition::fixed(vec![ParamType::Number], ReturnType::Number, functions::exp),
        );
        self.register(
            "LN",
            FunctionDefinition::fixed(vec![ParamType::Number], ReturnType::Number, functions::ln),
        );
        self.register(
            "LOG",
            FunctionDefinition::range(
                1,
                2,
                vec![ParamType::Number, ParamType::Number],
                ReturnType::Number,
                functions::log,
            ),
        );
        self.register(
            "LOG10",
            FunctionDefinition::fixed(
                vec![ParamType::Number],
                ReturnType::Number,
                functions::log10,
            ),
        );
        self.register(
            "FACT",
            FunctionDefinition::fixed(vec![ParamType::Number], ReturnType::Number, functions::fact),
        );

        // Additional Text functions
        self.register(
            "TRIM",
            FunctionDefinition::fixed(vec![ParamType::Text], ReturnType::Text, functions::trim),
        );
        self.register(
            "UPPER",
            FunctionDefinition::fixed(vec![ParamType::Text], ReturnType::Text, functions::upper),
        );
        self.register(
            "LOWER",
            FunctionDefinition::fixed(vec![ParamType::Text], ReturnType::Text, functions::lower),
        );
        self.register(
            "PROPER",
            FunctionDefinition::fixed(vec![ParamType::Text], ReturnType::Text, functions::proper),
        );

        // Information functions
        self.register(
            "ISBLANK",
            FunctionDefinition::fixed(
                vec![ParamType::Any],
                ReturnType::Logical,
                functions::isblank,
            ),
        );
        self.register(
            "ISERROR",
            FunctionDefinition::fixed(
                vec![ParamType::Any],
                ReturnType::Logical,
                functions::iserror,
            ),
        );
        self.register(
            "ISNA",
            FunctionDefinition::fixed(vec![ParamType::Any], ReturnType::Logical, functions::isna),
        );
        self.register(
            "ISNUMBER",
            FunctionDefinition::fixed(
                vec![ParamType::Any],
                ReturnType::Logical,
                functions::isnumber,
            ),
        );
        self.register(
            "ISTEXT",
            FunctionDefinition::fixed(vec![ParamType::Any], ReturnType::Logical, functions::istext),
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

fn cell_to_coordinate(cell: CellAddress) -> CellCoordinate {
    CellCoordinate {
        row_index: cell.row,
        column_index: cell.col,
        sheet_id: 0,
        data_validation_id: None,
        conditional_format_id: None,
    }
}

fn coordinate_to_cell(cell: CellCoordinate) -> CellAddress {
    CellAddress::new(cell.row_index, cell.column_index)
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
            return Vec::new();
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

fn collect_range_dependencies(expr: &FormulaExpr, deps: &mut Vec<CellRange>) {
    match expr {
        FormulaExpr::RangeRef(range) => deps.push(*range),
        FormulaExpr::FunctionCall { args, .. } => {
            for arg in args {
                collect_range_dependencies(arg, deps);
            }
        }
        FormulaExpr::BinaryOp { left, right, .. } => {
            collect_range_dependencies(left, deps);
            collect_range_dependencies(right, deps);
        }
        FormulaExpr::UnaryOp { expr, .. } => {
            collect_range_dependencies(expr, deps);
        }
        _ => {}
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
        BinaryOperator::LessThan => compare_numbers(&left, &right, |l, r| l < r)
            .map(Value::Bool)
            .unwrap_or(Value::Error(ErrorValue::Value)),
        BinaryOperator::LessThanOrEqual => compare_numbers(&left, &right, |l, r| l <= r)
            .map(Value::Bool)
            .unwrap_or(Value::Error(ErrorValue::Value)),
        BinaryOperator::GreaterThan => compare_numbers(&left, &right, |l, r| l > r)
            .map(Value::Bool)
            .unwrap_or(Value::Error(ErrorValue::Value)),
        BinaryOperator::GreaterThanOrEqual => compare_numbers(&left, &right, |l, r| l >= r)
            .map(Value::Bool)
            .unwrap_or(Value::Error(ErrorValue::Value)),
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

fn compare_numbers(left: &Value, right: &Value, cmp: fn(f64, f64) -> bool) -> Option<bool> {
    match (left, right) {
        (Value::Int(l), Value::Int(r)) => Some(cmp(*l as f64, *r as f64)),
        (Value::Float(l), Value::Float(r)) => Some(cmp(*l, *r)),
        (Value::Int(l), Value::Float(r)) => Some(cmp(*l as f64, *r)),
        (Value::Float(l), Value::Int(r)) => Some(cmp(*l, *r as f64)),
        _ => None,
    }
}

fn logical_op(left: Value, right: Value, op: fn(bool, bool) -> bool) -> Value {
    match (
        functions::coerce_to_bool(&left),
        functions::coerce_to_bool(&right),
    ) {
        (Ok(l), Ok(r)) => Value::Bool(op(l, r)),
        (Err(err), _) | (_, Err(err)) => Value::Error(err),
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
    fn test_dependency_dirty_order() {
        let mut engine = FormulaEngine::new();
        let a1 = CellAddress::new(0, 0);
        let b1 = CellAddress::new(0, 1);
        engine.set_formula(b1, "=A1+1").expect("set ok");
        engine.mark_dirty(&a1);
        let dirty = engine.get_dirty_nodes().expect("dirty nodes");
        let a1_idx = dirty.iter().position(|cell| *cell == a1).unwrap();
        let b1_idx = dirty.iter().position(|cell| *cell == b1).unwrap();
        assert!(a1_idx < b1_idx);
    }

    #[test]
    fn test_dependency_range_support() {
        let mut engine = FormulaEngine::new();
        let a1 = CellAddress::new(0, 0);
        let c1 = CellAddress::new(0, 2);
        engine
            .set_formula(c1, "=SUM(A1:A2)")
            .expect("set ok");
        engine.mark_dirty(&a1);
        let dirty = engine.get_dirty_nodes().expect("dirty nodes");
        assert!(dirty.contains(&c1));
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
        assert_eq!(value, Value::Error(ErrorValue::Value));
    }
}
