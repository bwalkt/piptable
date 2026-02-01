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
        params: Vec<Param>,
        body: Vec<Statement>,
        is_async: bool,
        line: usize,
    },

    /// Return statement
    Return { value: Option<Expr>, line: usize },

    /// Exit Function statement
    ExitFunction { line: usize },

    /// Exit For statement
    ExitFor { line: usize },

    /// Exit While statement
    ExitWhile { line: usize },

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

    /// Export statement: `export data to "file.csv"` or `export data to "file.csv" append`
    Export {
        source: Expr,
        destination: Expr,
        append: bool,
        options: Option<Expr>,
        line: usize,
    },

    /// Import statement: `import "file.csv" into data` or `import "a.csv", "b.csv" into book`
    Import {
        sources: Vec<Expr>,
        target: String,
        sheet_name: Option<Expr>,
        options: ImportOptions,
        line: usize,
    },

    /// Append statement: `users append new_users` or `users append distinct new_users on "id"`
    Append {
        target: String,
        source: Expr,
        distinct: bool,
        key: Option<String>,
        line: usize,
    },

    /// Upsert statement: `users upsert updates on "id"`
    Upsert {
        target: String,
        source: Expr,
        key: String,
        line: usize,
    },

    /// Expression statement (for side effects)
    Expr { expr: Expr, line: usize },
}

/// Parameter passing mode for function parameters.
///
/// This enum determines how arguments are passed to functions:
/// - `ByVal`: Parameters are passed by value (copied), modifications don't affect the original
/// - `ByRef`: Parameters are passed by reference, modifications affect the original variable
///
/// # Examples
///
/// ```text
/// // ByVal example - original variable unchanged
/// function double_byval(ByVal x)
///     x = x * 2
///     return x
/// end function
/// dim original = 5
/// dim result = double_byval(original)  // result = 10, original = 5
///
/// // ByRef example - original variable modified
/// function double_byref(ByRef x)
///     x = x * 2
/// end function
/// dim value = 5
/// call double_byref(value)  // value = 10
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParamMode {
    /// Pass by value - argument is copied, original unchanged
    ByVal,
    /// Pass by reference - argument references original, modifications affect original
    ByRef,
}

/// Function parameter definition.
///
/// Represents a single parameter in a function definition, including
/// its name, passing mode, default value, and special parameter attributes.
/// This structure is used during parsing and execution to manage parameter
/// binding and reference semantics.
///
/// # Fields
///
/// * `name` - The parameter name as it appears in the function signature
/// * `mode` - How the parameter should be passed (ByVal or ByRef)
/// * `default` - Optional default value for optional parameters
/// * `is_param_array` - Whether this parameter accepts variable arguments (ParamArray)
///
/// # Examples
///
/// ```rust,ignore
/// // Basic parameter: function example(ByVal a, ByRef b)
/// let param_a = Param {
///     name: "a".to_string(),
///     mode: ParamMode::ByVal,
///     default: None,
///     is_param_array: false
/// };
///
/// // Optional parameter: function example(Optional x = 10)
/// let param_optional = Param {
///     name: "x".to_string(),
///     mode: ParamMode::ByVal,
///     default: Some(Expr::Literal(Literal::Int(10))),
///     is_param_array: false
/// };
///
/// // ParamArray parameter: function example(ParamArray values)
/// let param_array = Param {
///     name: "values".to_string(),
///     mode: ParamMode::ByVal,
///     default: None,
///     is_param_array: true
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Param {
    /// The parameter name as declared in the function signature
    pub name: String,
    /// The parameter passing mode (ByVal or ByRef)
    pub mode: ParamMode,
    /// Default value for optional parameters (None if parameter is required)
    pub default: Option<Expr>,
    /// Whether this parameter accepts variable arguments (ParamArray)
    pub is_param_array: bool,
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

    /// Call expression: `expr(args)` (e.g., immediate lambda invocation)
    CallExpr { callee: Box<Expr>, args: Vec<Expr> },

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

    /// Join expression: `sheet1 join sheet2 on "id"` or `sheet1 left join sheet2 on "id" = "user_id"`
    Join {
        left: Box<Expr>,
        right: Box<Expr>,
        join_type: JoinType,
        condition: JoinCondition,
    },

    /// Method call: `object.method(args)`
    MethodCall {
        object: Box<Expr>,
        method: String,
        args: Vec<Expr>,
    },

    /// Lambda expression: `x => x + 1` or `(a, b) => a > b`
    Lambda {
        params: Vec<String>,
        body: Box<Expr>,
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

/// Import options for the import statement.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImportOptions {
    /// Whether files have headers (default: true)
    pub has_headers: Option<bool>,
    /// Optional page range (PDF only). Example: "1-5"
    pub page_range: Option<String>,
    /// Minimum table rows (PDF/Markdown)
    pub min_table_rows: Option<usize>,
    /// Minimum table columns (PDF/Markdown)
    pub min_table_cols: Option<usize>,
    /// Alias for headers detection (PDF/Markdown)
    pub detect_headers: Option<bool>,
}

impl ImportOptions {
    /// Create new import options with headers enabled.
    ///
    /// This creates the default configuration where files are assumed to have headers.
    /// Equivalent to calling `ImportOptions::default()`.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let options = ImportOptions::new();
    /// assert_eq!(options.has_headers, None); // Uses default (true)
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Create options without headers.
    ///
    /// Use this when importing files that don't have a header row,
    /// such as raw data files or CSV files without column names.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let options = ImportOptions::without_headers();
    /// assert_eq!(options.has_headers, Some(false));
    /// ```
    pub fn without_headers() -> Self {
        Self {
            has_headers: Some(false),
        }
    }
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
    Full,
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
    ///
    /// Returns a program with no statements. This is typically used as a starting
    /// point for building programs programmatically or as a default value.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let program = Program::new();
    /// assert!(program.statements.is_empty());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            statements: Vec::new(),
        }
    }

    /// Create a program from statements.
    ///
    /// Constructs a program containing the provided statements. This is useful
    /// when you already have a collection of parsed or constructed statements.
    ///
    /// # Arguments
    ///
    /// * `statements` - A vector of statements that make up the program
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let statements = vec![
    ///     Statement::Expr { expr: Expr::Literal(Literal::Int(42)), line: 1 }
    /// ];
    /// let program = Program::from_statements(statements);
    /// assert_eq!(program.statements.len(), 1);
    /// ```
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

/// Join condition specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JoinCondition {
    /// Join on same column name: `on "id"`
    On(String),
    /// Join on different columns: `on "left_col" = "right_col"`
    OnColumns { left: String, right: String },
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
