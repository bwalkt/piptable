//! Abstract Syntax Tree (AST) definitions for piptable DSL.

use serde::{Deserialize, Serialize};

/// A complete piptable program.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Program {
    pub statements: Vec<Statement>,
}

/// A statement in the DSL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Statement {
    /// Variable declaration: `dim x = expr`
    Dim {
        name: String,
        type_hint: Option<TypeName>,
        value: Expr,
        line: usize,
    },

    /// Assignment: `x = expr`
    Assignment {
        target: LValue,
        value: Expr,
        line: usize,
    },

    /// If statement
    If {
        condition: Expr,
        then_body: Vec<Statement>,
        elseif_clauses: Vec<ElseIfClause>,
        else_body: Option<Vec<Statement>>,
        line: usize,
    },

    /// For each loop: `for each item in collection ... next`
    ForEach {
        variable: String,
        iterable: Expr,
        body: Vec<Statement>,
        line: usize,
    },

    /// For loop with counter: `for i = 1 to 10 step 1 ... next`
    For {
        variable: String,
        start: Expr,
        end: Expr,
        step: Option<Expr>,
        body: Vec<Statement>,
        line: usize,
    },

    /// While loop: `while condition ... wend`
    While {
        condition: Expr,
        body: Vec<Statement>,
        line: usize,
    },

    /// Function definition
    Function {
        name: String,
        params: Vec<String>,
        body: Vec<Statement>,
        is_async: bool,
        line: usize,
    },

    /// Sub (procedure) definition
    Sub {
        name: String,
        params: Vec<String>,
        body: Vec<Statement>,
        is_async: bool,
        line: usize,
    },

    /// Return statement
    Return { value: Option<Expr>, line: usize },

    /// Call statement: `call proc(args)` or just `proc(args)`
    Call {
        function: String,
        args: Vec<Expr>,
        line: usize,
    },

    /// Chart definition
    Chart {
        chart_type: ChartType,
        title: String,
        options: Vec<ChartOption>,
        line: usize,
    },

    /// Export statement: `export data to "file.csv"`
    Export {
        source: Expr,
        destination: Expr,
        options: Option<Expr>,
        line: usize,
    },

    /// Import statement: `import "file.csv" into data`
    Import {
        source: Expr,
        target: String,
        sheet_name: Option<Expr>,
        options: Option<Expr>,
        line: usize,
    },

    /// Expression statement (for side effects)
    Expr { expr: Expr, line: usize },
}

/// Elseif clause in an if statement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElseIfClause {
    pub condition: Expr,
    pub body: Vec<Statement>,
}

/// Left-hand side of an assignment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LValue {
    /// Simple variable
    Variable(String),
    /// Field access: `obj->field`
    Field { object: Box<LValue>, field: String },
    /// Array index: `arr[idx]`
    Index {
        array: Box<LValue>,
        index: Box<Expr>,
    },
}

/// Expression in the DSL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Expr {
    /// Literal value
    Literal(Literal),

    /// Variable reference
    Variable(String),

    /// Binary operation
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },

    /// Unary operation
    Unary { op: UnaryOp, operand: Box<Expr> },

    /// Field access: `expr->field`
    FieldAccess { object: Box<Expr>, field: String },

    /// Array index: `expr[index]`
    ArrayIndex { array: Box<Expr>, index: Box<Expr> },

    /// Type assertion: `expr::Type`
    TypeAssertion {
        expr: Box<Expr>,
        type_name: TypeName,
    },

    /// Function call: `func(args)`
    Call { function: String, args: Vec<Expr> },

    /// SQL query: `query(SELECT ...)`
    Query(Box<SqlQuery>),

    /// HTTP fetch: `fetch(url, options)`
    Fetch {
        url: Box<Expr>,
        options: Option<Box<Expr>>,
    },

    /// Async for each expression
    AsyncForEach {
        variable: String,
        iterable: Box<Expr>,
        body: Vec<Statement>,
    },

    /// Parallel execution block
    Parallel { expressions: Vec<Expr> },

    /// Await expression
    Await(Box<Expr>),

    /// Array literal: `[1, 2, 3]`
    Array(Vec<Expr>),

    /// Object literal: `{ key: value }`
    Object(Vec<(String, Expr)>),

    /// Ask (natural language query): `ask "query" from data`
    Ask {
        query: String,
        source: Box<Expr>,
        options: Option<Box<Expr>>,
    },
}

/// Literal values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Literal {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Interval { value: i64, unit: IntervalUnit },
}

/// Interval units for time-based operations.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum IntervalUnit {
    Millisecond,
    Second,
    Minute,
    Hour,
    Day,
    Week,
    Month,
    Year,
}

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryOp {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,

    // Comparison
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,

    // Logical
    And,
    Or,

    // String
    Concat,

    // SQL-specific
    Like,
    In,
}

/// Unary operators.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum UnaryOp {
    Neg,
    Not,
}

/// Type names for type hints and assertions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeName {
    Int,
    Float,
    String,
    Bool,
    Timestamp,
    Duration,
    Array,
    Object,
    Table,
}

/// Chart types.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ChartType {
    Bar,
    Line,
    Pie,
    Scatter,
    Area,
}

/// Chart option (key-value pair).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartOption {
    pub key: String,
    pub value: Expr,
}

/// SQL query AST.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlQuery {
    pub with_clause: Option<WithClause>,
    pub select: SelectClause,
    pub from: Option<FromClause>,
    pub joins: Vec<JoinClause>,
    pub where_clause: Option<Box<Expr>>,
    pub group_by: Option<Vec<Expr>>,
    pub having: Option<Box<Expr>>,
    pub order_by: Option<Vec<OrderByItem>>,
    pub limit: Option<Box<Expr>>,
    pub offset: Option<Box<Expr>>,
    pub trigger: Option<Trigger>,
}

/// WITH clause (CTEs).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithClause {
    pub recursive: bool,
    pub ctes: Vec<Cte>,
}

/// Common Table Expression.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cte {
    pub name: String,
    pub columns: Option<Vec<String>>,
    pub query: Box<SqlQuery>,
}

/// SELECT clause.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectClause {
    pub distinct: bool,
    pub items: Vec<SelectItem>,
}

/// Item in SELECT list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectItem {
    pub expr: Expr,
    pub alias: Option<String>,
}

/// FROM clause.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FromClause {
    pub source: TableRef,
    pub alias: Option<String>,
}

/// Table reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TableRef {
    /// Simple table name
    Table(String),
    /// Qualified name: `db.schema.table`
    Qualified {
        database: String,
        schema: Option<String>,
        table: String,
    },
    /// File path: `'data.csv'`
    File(String),
    /// Table function: `csv('file.csv', delimiter => ';')`
    Function {
        name: String,
        args: Vec<FunctionArg>,
    },
    /// Stdin
    Stdin,
    /// Subquery
    Subquery(Box<SqlQuery>),
}

/// Function argument (positional or named).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FunctionArg {
    Positional(Expr),
    Named { name: String, value: Expr },
}

/// JOIN clause.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinClause {
    pub join_type: JoinType,
    pub table: TableRef,
    pub alias: Option<String>,
    pub on_clause: Option<Box<Expr>>,
}

/// JOIN types.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Cross,
}

/// ORDER BY item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderByItem {
    pub expr: Expr,
    pub direction: SortDirection,
}

/// Sort direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SortDirection {
    #[default]
    Asc,
    Desc,
}

/// Trigger for streaming queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Trigger {
    Counting(u64),
    OnWatermark,
    OnEndOfStream,
}

impl Program {
    /// Create a new empty program.
    #[must_use]
    pub fn new() -> Self {
        Self {
            statements: Vec::new(),
        }
    }

    /// Create a program from statements.
    #[must_use]
    pub fn from_statements(statements: Vec<Statement>) -> Self {
        Self { statements }
    }
}

impl Default for Program {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_program_new() {
        let program = Program::new();
        assert!(program.statements.is_empty());
    }

    #[test]
    fn test_literal_serialization() {
        let lit = Literal::Int(42);
        let json = serde_json::to_string(&lit).unwrap();
        assert!(json.contains("42"));
    }
}
