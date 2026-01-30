//! # Piptable Formulas
//!
//! Formula parsing, compilation, and evaluation engine.
//! Includes formula registry for standard functions (SUM, VLOOKUP, etc.)

use piptable_primitives::{CellAddress, CellRange, Value};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
        // TODO: Implement formula parsing and compilation
        let _ = formula;
        todo!("Implement formula compilation")
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
        self.register("SUM", FunctionDefinition::variadic(1));
        self.register("AVERAGE", FunctionDefinition::variadic(1));
        self.register("COUNT", FunctionDefinition::variadic(1));
        self.register("MAX", FunctionDefinition::variadic(1));
        self.register("MIN", FunctionDefinition::variadic(1));

        // Logical functions
        self.register("IF", FunctionDefinition::fixed(3));
        self.register("AND", FunctionDefinition::variadic(1));
        self.register("OR", FunctionDefinition::variadic(1));
        self.register("NOT", FunctionDefinition::fixed(1));

        // Lookup functions
        self.register("VLOOKUP", FunctionDefinition::range(3, 4));
        self.register("HLOOKUP", FunctionDefinition::range(3, 4));
        self.register("INDEX", FunctionDefinition::range(2, 3));
        self.register("MATCH", FunctionDefinition::range(2, 3));
    }

    fn register(&mut self, name: &str, def: FunctionDefinition) {
        self.functions.insert(name.to_uppercase(), def);
    }

    /// Check if a function exists
    pub fn has_function(&self, name: &str) -> bool {
        self.functions.contains_key(&name.to_uppercase())
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
}

impl FunctionDefinition {
    /// Fixed number of arguments
    pub fn fixed(args: usize) -> Self {
        Self {
            min_args: args,
            max_args: Some(args),
        }
    }

    /// Variable number of arguments
    pub fn variadic(min: usize) -> Self {
        Self {
            min_args: min,
            max_args: None,
        }
    }

    /// Range of arguments
    pub fn range(min: usize, max: usize) -> Self {
        Self {
            min_args: min,
            max_args: Some(max),
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
}
