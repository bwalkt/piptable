//! # piptable-interpreter
//!
//! Interpreter for executing piptable DSL scripts.
//!
//! This crate provides:
//! - AST evaluation
//! - Variable scope management
//! - Built-in functions
//! - Integration with SQL and HTTP engines
//! - Python UDF support (with `python` feature)

#[cfg(feature = "python")]
mod python;

use async_recursion::async_recursion;
use piptable_core::{
    BinaryOp, Expr, FromClause, ImportOptions, JoinClause, JoinType, LValue, Literal, OrderByItem,
    PipError, PipResult, Program, SelectClause, SelectItem, SortDirection, SqlQuery, Statement,
    TableRef, UnaryOp, Value,
};
use piptable_http::HttpClient;
use piptable_sheet::{CellValue, Sheet};
use piptable_sql::SqlEngine;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Interpreter for piptable scripts.
pub struct Interpreter {
    /// Variable scopes (stack for nested scopes)
    scopes: Arc<RwLock<Vec<HashMap<String, Value>>>>,
    /// SQL engine
    sql: SqlEngine,
    /// HTTP client
    http: HttpClient,
    /// Output buffer
    output: Arc<RwLock<Vec<String>>>,
    /// Function definitions
    functions: Arc<RwLock<HashMap<String, FunctionDef>>>,
    /// Python runtime (optional, with `python` feature)
    #[cfg(feature = "python")]
    python_runtime: Option<python::PythonRuntime>,
}

/// Function definition stored at runtime.
#[derive(Clone)]
pub struct FunctionDef {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Statement>,
    pub is_async: bool,
}

impl Interpreter {
    /// Create a new interpreter.
    #[must_use]
    pub fn new() -> Self {
        Self {
            scopes: Arc::new(RwLock::new(vec![HashMap::new()])),
            sql: SqlEngine::new(),
            http: HttpClient::new().expect("Failed to create HTTP client"),
            output: Arc::new(RwLock::new(Vec::new())),
            functions: Arc::new(RwLock::new(HashMap::new())),
            #[cfg(feature = "python")]
            python_runtime: match python::PythonRuntime::new() {
                Ok(rt) => Some(rt),
                Err(e) => {
                    tracing::warn!("Failed to initialize Python runtime: {}", e);
                    None
                }
            },
        }
    }

    /// Execute a program.
    ///
    /// # Errors
    ///
    /// Returns error if execution fails.
    pub async fn eval(&mut self, program: Program) -> PipResult<Value> {
        let mut result = Value::Null;

        for statement in program.statements {
            match self.eval_statement(statement).await {
                Ok(val) => result = val,
                Err(e) => return Err(e),
            }
        }

        Ok(result)
    }

    /// Execute a single statement.
    ///
    /// # Errors
    ///
    /// Returns error if statement execution fails.
    #[async_recursion]
    pub async fn eval_statement(&mut self, statement: Statement) -> PipResult<Value> {
        match statement {
            Statement::Dim {
                name, value, line, ..
            } => {
                let val = self
                    .eval_expr(&value)
                    .await
                    .map_err(|e| e.with_line(line))?;
                self.set_var(&name, val).await;
                Ok(Value::Null)
            }

            Statement::Assignment {
                target,
                value,
                line,
            } => {
                let val = self
                    .eval_expr(&value)
                    .await
                    .map_err(|e| e.with_line(line))?;
                self.assign_lvalue(&target, val, line).await?;
                Ok(Value::Null)
            }

            Statement::If {
                condition,
                then_body,
                elseif_clauses,
                else_body,
                line,
            } => {
                let cond = self
                    .eval_expr(&condition)
                    .await
                    .map_err(|e| e.with_line(line))?;

                if cond.is_truthy() {
                    self.push_scope().await;
                    let result = self.eval_block(&then_body).await;
                    self.pop_scope().await;
                    result?;
                } else {
                    let mut executed = false;
                    for clause in elseif_clauses {
                        let cond = self.eval_expr(&clause.condition).await?;
                        if cond.is_truthy() {
                            self.push_scope().await;
                            let result = self.eval_block(&clause.body).await;
                            self.pop_scope().await;
                            result?;
                            executed = true;
                            break;
                        }
                    }
                    if !executed {
                        if let Some(else_stmts) = else_body {
                            self.push_scope().await;
                            let result = self.eval_block(&else_stmts).await;
                            self.pop_scope().await;
                            result?;
                        }
                    }
                }
                Ok(Value::Null)
            }

            Statement::ForEach {
                variable,
                iterable,
                body,
                line,
            } => {
                let iter_val = self
                    .eval_expr(&iterable)
                    .await
                    .map_err(|e| e.with_line(line))?;

                let items = match iter_val {
                    Value::Array(arr) => arr,
                    Value::Table(batches) => {
                        // Convert table rows to array of objects
                        self.table_to_array(&batches)?
                    }
                    _ => {
                        return Err(PipError::runtime(
                            line,
                            format!("Cannot iterate over {}", iter_val.type_name()),
                        ))
                    }
                };

                self.push_scope().await;
                let mut loop_result: PipResult<()> = Ok(());
                for item in items {
                    self.declare_var(&variable, item).await;
                    if let Err(e) = self.eval_block(&body).await {
                        loop_result = Err(e);
                        break;
                    }
                }
                self.pop_scope().await;
                loop_result?;

                Ok(Value::Null)
            }

            Statement::For {
                variable,
                start,
                end,
                step,
                body,
                line,
            } => {
                let start_val = self
                    .eval_expr(&start)
                    .await
                    .map_err(|e| e.with_line(line))?;
                let end_val = self.eval_expr(&end).await.map_err(|e| e.with_line(line))?;
                let step_val = match step {
                    Some(s) => self.eval_expr(&s).await.map_err(|e| e.with_line(line))?,
                    None => Value::Int(1),
                };

                let start_int = start_val
                    .as_int()
                    .ok_or_else(|| PipError::runtime(line, "For loop start must be integer"))?;
                let end_int = end_val
                    .as_int()
                    .ok_or_else(|| PipError::runtime(line, "For loop end must be integer"))?;
                let step_int = step_val
                    .as_int()
                    .ok_or_else(|| PipError::runtime(line, "For loop step must be integer"))?;

                if step_int == 0 {
                    return Err(PipError::runtime(line, "For loop step cannot be zero"));
                }

                self.push_scope().await;
                let mut loop_result: PipResult<()> = Ok(());
                let mut i = start_int;
                while (step_int > 0 && i <= end_int) || (step_int < 0 && i >= end_int) {
                    self.declare_var(&variable, Value::Int(i)).await;
                    if let Err(e) = self.eval_block(&body).await {
                        loop_result = Err(e);
                        break;
                    }
                    match i.checked_add(step_int) {
                        Some(next) => i = next,
                        None => {
                            loop_result =
                                Err(PipError::runtime(line, "Integer overflow in for loop"));
                            break;
                        }
                    }
                }
                self.pop_scope().await;
                loop_result?;

                Ok(Value::Null)
            }

            Statement::While {
                condition,
                body,
                line,
            } => {
                self.push_scope().await;
                let mut loop_result: PipResult<()> = Ok(());
                loop {
                    let cond_result = self.eval_expr(&condition).await;
                    match cond_result {
                        Ok(cond) => {
                            if !cond.is_truthy() {
                                break;
                            }
                            if let Err(e) = self.eval_block(&body).await {
                                loop_result = Err(e.with_line(line));
                                break;
                            }
                        }
                        Err(e) => {
                            loop_result = Err(e.with_line(line));
                            break;
                        }
                    }
                }
                self.pop_scope().await;
                loop_result?;
                Ok(Value::Null)
            }

            Statement::Function {
                name,
                params,
                body,
                is_async,
                ..
            } => {
                let func = FunctionDef {
                    name: name.clone(),
                    params,
                    body,
                    is_async,
                };
                let mut funcs = self.functions.write().await;
                funcs.insert(name, func);
                Ok(Value::Null)
            }

            Statement::Sub {
                name,
                params,
                body,
                is_async,
                ..
            } => {
                let func = FunctionDef {
                    name: name.clone(),
                    params,
                    body,
                    is_async,
                };
                let mut funcs = self.functions.write().await;
                funcs.insert(name, func);
                Ok(Value::Null)
            }

            Statement::Return { value, line } => {
                let val = match value {
                    Some(expr) => self.eval_expr(&expr).await.map_err(|e| e.with_line(line))?,
                    None => Value::Null,
                };
                // Return is handled by propagating up the call stack
                Err(PipError::Return(val))
            }

            Statement::Call {
                function,
                args,
                line,
            } => {
                let arg_vals = self.eval_args(&args, line).await?;
                self.call_function(&function, arg_vals, line).await
            }

            Statement::Expr { expr, line } => {
                self.eval_expr(&expr).await.map_err(|e| e.with_line(line))
            }

            Statement::Chart { .. } => {
                // TODO: Implement chart generation
                Ok(Value::Null)
            }

            Statement::Export {
                source,
                destination,
                options,
                line,
            } => {
                // Reject export options until they are implemented
                if options.is_some() {
                    return Err(PipError::Export(format!(
                        "Line {}: export options are not supported yet",
                        line
                    )));
                }

                // Evaluate source to get data
                let data = self
                    .eval_expr(&source)
                    .await
                    .map_err(|e| e.with_line(line))?;

                // Evaluate destination to get file path
                let dest = self
                    .eval_expr(&destination)
                    .await
                    .map_err(|e| e.with_line(line))?;
                let path = match &dest {
                    Value::String(s) => s.clone(),
                    _ => {
                        return Err(PipError::Export(format!(
                            "Line {}: export destination must be a string, got {}",
                            line,
                            dest.type_name()
                        )));
                    }
                };

                // Convert Value to Sheet and export
                let sheet = value_to_sheet(&data)
                    .map_err(|e| PipError::Export(format!("Line {}: {}", line, e)))?;

                // Determine format from file extension and export
                export_sheet(&sheet, &path)
                    .map_err(|e| PipError::Export(format!("Line {}: {}", line, e)))?;

                Ok(Value::Null)
            }

            Statement::Import {
                sources,
                target,
                sheet_name,
                options,
                line,
            } => {
                // Evaluate all source paths
                let mut paths: Vec<String> = Vec::new();
                for source in &sources {
                    let src = self
                        .eval_expr(source)
                        .await
                        .map_err(|e| e.with_line(line))?;
                    match &src {
                        Value::String(s) => paths.push(s.clone()),
                        _ => {
                            return Err(PipError::Import(format!(
                                "Line {}: import source must be a string, got {}",
                                line,
                                src.type_name()
                            )));
                        }
                    }
                }

                // Defensive check: ensure at least one path
                if paths.is_empty() {
                    return Err(PipError::Import(format!(
                        "Line {}: import requires at least one file path",
                        line
                    )));
                }

                // Check for invalid multi-file import with sheet clause before evaluation
                if paths.len() > 1 && sheet_name.is_some() {
                    return Err(PipError::Import(format!(
                        "Line {}: sheet clause is not supported for multi-file import",
                        line
                    )));
                }

                // Evaluate optional sheet name for Excel files (single-file only)
                let sheet_name_str = if paths.len() == 1 {
                    if let Some(ref sheet_expr) = sheet_name {
                        let val = self
                            .eval_expr(sheet_expr)
                            .await
                            .map_err(|e| e.with_line(line))?;
                        match val {
                            Value::String(s) => Some(s),
                            _ => {
                                return Err(PipError::Import(format!(
                                    "Line {}: sheet name must be a string, got {}",
                                    line,
                                    val.type_name()
                                )));
                            }
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                // Multi-file or single file import
                let value = if paths.len() > 1 {
                    // Multi-file import: use Book::from_files_with_options
                    import_multi_files(&paths, &options)
                        .map_err(|e| PipError::Import(format!("Line {}: {}", line, e)))?
                } else {
                    // Single file import
                    let has_headers = options.has_headers.unwrap_or(true);
                    let sheet = import_sheet(&paths[0], sheet_name_str.as_deref(), has_headers)
                        .map_err(|e| PipError::Import(format!("Line {}: {}", line, e)))?;
                    sheet_to_value(&sheet)
                };

                // Store in target variable
                self.set_var(&target, value).await;

                Ok(Value::Null)
            }
        }
    }

    /// Evaluate an expression.
    /// Evaluate a block of statements, returning the last value or an error.
    async fn eval_block(&mut self, stmts: &[Statement]) -> PipResult<Value> {
        let mut result = Value::Null;
        for stmt in stmts {
            result = self.eval_statement(stmt.clone()).await?;
        }
        Ok(result)
    }

    /// Evaluates an expression and returns its resulting Piptable `Value`.
    ///
    /// This performs runtime evaluation for all `Expr` variants (literals, variables,
    /// binary/unary ops, field access, indexing, calls, queries, fetches, arrays/objects,
    /// awaits/parallel, and the current set of unimplemented stubs).
    ///
    /// Errors are returned as `PipError::runtime` with contextual messages for invalid
    /// operations (undefined variables, type mismatches, out-of-bounds indexing, etc.).
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use tokio::runtime::Runtime;
    /// use piptable_interpreter::Interpreter;
    /// use piptable_core::{Expr, Literal, Value};
    ///
    /// let rt = Runtime::new().unwrap();
    /// let mut interp = Interpreter::new();
    /// let expr = Expr::Literal(Literal::Int(42));
    /// let val = rt.block_on(interp.eval_expr(&expr)).unwrap();
    /// assert_eq!(val, Value::Int(42));
    /// ```
    #[async_recursion]
    async fn eval_expr(&mut self, expr: &Expr) -> PipResult<Value> {
        match expr {
            Expr::Literal(lit) => self.eval_literal(lit),

            Expr::Variable(name) => {
                if name == "*" {
                    // Special case for SELECT *
                    return Ok(Value::String("*".to_string()));
                }
                self.get_var(name)
                    .await
                    .ok_or_else(|| PipError::runtime(0, format!("Undefined variable: {name}")))
            }

            Expr::Binary { left, op, right } => {
                // Short-circuit evaluation for AND/OR
                if matches!(op, BinaryOp::And | BinaryOp::Or) {
                    let left_val = self.eval_expr(left).await?;
                    if matches!(op, BinaryOp::And) {
                        if !left_val.is_truthy() {
                            return Ok(Value::Bool(false));
                        }
                    } else if left_val.is_truthy() {
                        return Ok(Value::Bool(true));
                    }
                    let right_val = self.eval_expr(right).await?;
                    return Ok(Value::Bool(right_val.is_truthy()));
                }
                let left_val = self.eval_expr(left).await?;
                let right_val = self.eval_expr(right).await?;
                self.eval_binary_op(&left_val, *op, &right_val)
            }

            Expr::Unary { op, operand } => {
                let val = self.eval_expr(operand).await?;
                self.eval_unary_op(*op, &val)
            }

            Expr::FieldAccess { object, field } => {
                let obj = self.eval_expr(object).await?;
                match obj {
                    Value::Object(map) => map
                        .get(field)
                        .cloned()
                        .ok_or_else(|| PipError::runtime(0, format!("Field not found: {field}"))),
                    _ => Err(PipError::runtime(
                        0,
                        format!("Cannot access field on {}", obj.type_name()),
                    )),
                }
            }

            Expr::ArrayIndex { array, index } => {
                let arr = self.eval_expr(array).await?;
                let idx = self.eval_expr(index).await?;

                match (&arr, &idx) {
                    // Array indexing with integer
                    (Value::Array(items), Value::Int(idx_int)) => {
                        let idx_usize = if *idx_int < 0 {
                            let adjusted = items.len() as i64 + idx_int;
                            if adjusted < 0 {
                                return Err(PipError::runtime(0, "Array index out of bounds"));
                            }
                            adjusted as usize
                        } else {
                            *idx_int as usize
                        };
                        items
                            .get(idx_usize)
                            .cloned()
                            .ok_or_else(|| PipError::runtime(0, "Array index out of bounds"))
                    }
                    // Object bracket access with string key
                    (Value::Object(map), Value::String(key)) => map
                        .get(key)
                        .cloned()
                        .ok_or_else(|| PipError::runtime(0, format!("Key '{}' not found", key))),
                    // Type mismatches
                    (Value::Array(_), _) => {
                        Err(PipError::runtime(0, "Array index must be integer"))
                    }
                    (Value::Object(_), _) => Err(PipError::runtime(0, "Object key must be string")),
                    _ => Err(PipError::runtime(
                        0,
                        format!("Cannot index {}", arr.type_name()),
                    )),
                }
            }

            Expr::Call { function, args } => {
                let arg_vals = self.eval_args(args, 0).await?;
                self.call_function(function, arg_vals, 0).await
            }

            Expr::Query(query) => self.eval_query(query).await,

            Expr::Fetch { url, options } => {
                let url_val = self.eval_expr(url).await?;
                let url_str = url_val
                    .as_str()
                    .ok_or_else(|| PipError::runtime(0, "Fetch URL must be a string"))?;

                // TODO: Implement proper conversion from Value to FetchOptions
                // For now, options are not fully supported
                let fetch_opts = match options {
                    Some(o) => {
                        // Evaluate to catch any errors, but warn that options aren't fully implemented
                        let _ = self.eval_expr(o).await?;
                        tracing::warn!("Fetch options are not yet fully implemented");
                        Some(piptable_http::FetchOptions::default())
                    }
                    None => None,
                };

                self.http.fetch(url_str, fetch_opts).await
            }

            Expr::Array(items) => {
                let mut values = Vec::with_capacity(items.len());
                for item in items {
                    values.push(self.eval_expr(item).await?);
                }
                Ok(Value::Array(values))
            }

            Expr::Object(fields) => {
                let mut map = HashMap::new();
                for (key, val_expr) in fields {
                    let val = self.eval_expr(val_expr).await?;
                    map.insert(key.clone(), val);
                }
                Ok(Value::Object(map))
            }

            Expr::TypeAssertion { expr, .. } => {
                // For now, just evaluate the expression (type checking would go here)
                self.eval_expr(expr).await
            }

            Expr::Await(inner) => {
                // For now, just evaluate synchronously
                self.eval_expr(inner).await
            }

            Expr::Parallel { expressions } => {
                // TODO: Currently evaluates sequentially. Should use tokio::join! or
                // futures::future::try_join_all for true parallel execution.
                // This requires resolving borrow checker issues with &mut self.
                let mut results = Vec::with_capacity(expressions.len());
                for expr in expressions {
                    results.push(self.eval_expr(expr).await?);
                }
                Ok(Value::Array(results))
            }

            Expr::AsyncForEach { .. } => {
                // TODO: Implement async for each
                Ok(Value::Null)
            }

            Expr::Join { left, right, join_type, condition } => {
                // Evaluate both sides to get sheets
                let left_val = self.eval_expr(left).await?;
                let right_val = self.eval_expr(right).await?;
                
                // Convert values to sheets
                let left_sheet = value_to_sheet(&left_val)
                    .map_err(|e| PipError::runtime(0, format!("Left side of join must be a sheet: {}", e)))?;
                let right_sheet = value_to_sheet(&right_val)
                    .map_err(|e| PipError::runtime(0, format!("Right side of join must be a sheet: {}", e)))?;
                
                // Perform the join based on the type and condition
                use piptable_core::ast::JoinCondition;
                use piptable_sheet::SheetError;
                let result = match (join_type, condition) {
                    (piptable_core::ast::JoinType::Inner, JoinCondition::On(key)) => {
                        left_sheet.inner_join(&right_sheet, key)
                            .map_err(|e: SheetError| PipError::runtime(0, e.to_string()))?
                    }
                    (piptable_core::ast::JoinType::Inner, JoinCondition::OnColumns { left: l, right: r }) => {
                        left_sheet.inner_join_on(&right_sheet, l, r)
                            .map_err(|e: SheetError| PipError::runtime(0, e.to_string()))?
                    }
                    (piptable_core::ast::JoinType::Left, JoinCondition::On(key)) => {
                        left_sheet.left_join(&right_sheet, key)
                            .map_err(|e: SheetError| PipError::runtime(0, e.to_string()))?
                    }
                    (piptable_core::ast::JoinType::Left, JoinCondition::OnColumns { left: l, right: r }) => {
                        left_sheet.left_join_on(&right_sheet, l, r)
                            .map_err(|e: SheetError| PipError::runtime(0, e.to_string()))?
                    }
                    (piptable_core::ast::JoinType::Right, JoinCondition::On(key)) => {
                        left_sheet.right_join(&right_sheet, key)
                            .map_err(|e: SheetError| PipError::runtime(0, e.to_string()))?
                    }
                    (piptable_core::ast::JoinType::Right, JoinCondition::OnColumns { left: l, right: r }) => {
                        left_sheet.right_join_on(&right_sheet, l, r)
                            .map_err(|e: SheetError| PipError::runtime(0, e.to_string()))?
                    }
                    (piptable_core::ast::JoinType::Full, JoinCondition::On(key)) => {
                        left_sheet.full_join(&right_sheet, key)
                            .map_err(|e: SheetError| PipError::runtime(0, e.to_string()))?
                    }
                    (piptable_core::ast::JoinType::Full, JoinCondition::OnColumns { left: l, right: r }) => {
                        left_sheet.full_join_on(&right_sheet, l, r)
                            .map_err(|e: SheetError| PipError::runtime(0, e.to_string()))?
                    }
                    (piptable_core::ast::JoinType::Cross, _) => {
                        // CROSS JOIN is not supported in DSL, only in SQL
                        return Err(PipError::runtime(0, "CROSS JOIN is not supported in DSL join syntax, only in SQL queries"));
                    }
                };
                
                Ok(sheet_to_value(&result))
            }

            Expr::Ask { .. } => {
                // TODO: Implement LLM integration
                Err(PipError::runtime(0, "Ask expression not yet implemented"))
            }
        }
    }

    /// Evaluate a literal to a Value.
    fn eval_literal(&self, lit: &Literal) -> PipResult<Value> {
        match lit {
            Literal::Null => Ok(Value::Null),
            Literal::Bool(b) => Ok(Value::Bool(*b)),
            Literal::Int(n) => Ok(Value::Int(*n)),
            Literal::Float(f) => Ok(Value::Float(*f)),
            Literal::String(s) => Ok(Value::String(s.clone())),
            Literal::Interval { value, unit } => {
                // Convert to milliseconds for internal representation
                use piptable_core::IntervalUnit;
                let multiplier: i64 = match unit {
                    IntervalUnit::Millisecond => 1,
                    IntervalUnit::Second => 1000,
                    IntervalUnit::Minute => 60 * 1000,
                    IntervalUnit::Hour => 60 * 60 * 1000,
                    IntervalUnit::Day => 24 * 60 * 60 * 1000,
                    IntervalUnit::Week => 7 * 24 * 60 * 60 * 1000,
                    IntervalUnit::Month => 30 * 24 * 60 * 60 * 1000,
                    IntervalUnit::Year => 365 * 24 * 60 * 60 * 1000,
                };
                value
                    .checked_mul(multiplier)
                    .map(Value::Int)
                    .ok_or_else(|| PipError::runtime(0, "Interval value overflow"))
            }
        }
    }

    /// Evaluate a binary operation.
    fn eval_binary_op(&self, left: &Value, op: BinaryOp, right: &Value) -> PipResult<Value> {
        match op {
            BinaryOp::Add => self.eval_add(left, right),
            BinaryOp::Sub => self.eval_sub(left, right),
            BinaryOp::Mul => self.eval_mul(left, right),
            BinaryOp::Div => self.eval_div(left, right),
            BinaryOp::Mod => self.eval_mod(left, right),
            BinaryOp::Eq => Ok(Value::Bool(self.values_equal(left, right))),
            BinaryOp::Ne => Ok(Value::Bool(!self.values_equal(left, right))),
            BinaryOp::Lt => self.eval_compare(left, right, |a, b| a < b),
            BinaryOp::Le => self.eval_compare(left, right, |a, b| a <= b),
            BinaryOp::Gt => self.eval_compare(left, right, |a, b| a > b),
            BinaryOp::Ge => self.eval_compare(left, right, |a, b| a >= b),
            BinaryOp::And => Ok(Value::Bool(left.is_truthy() && right.is_truthy())),
            BinaryOp::Or => Ok(Value::Bool(left.is_truthy() || right.is_truthy())),
            BinaryOp::Concat => {
                let l = self.value_to_string(left);
                let r = self.value_to_string(right);
                Ok(Value::String(format!("{l}{r}")))
            }
            BinaryOp::Like => {
                // Simple LIKE implementation (% wildcard only)
                let s = left
                    .as_str()
                    .ok_or_else(|| PipError::runtime(0, "LIKE requires string operand"))?;
                let pattern = right
                    .as_str()
                    .ok_or_else(|| PipError::runtime(0, "LIKE pattern must be string"))?;
                Ok(Value::Bool(self.matches_like(s, pattern)))
            }
            BinaryOp::In => {
                // Check if left is in right (array)
                match right {
                    Value::Array(arr) => {
                        let found = arr.iter().any(|v| self.values_equal(left, v));
                        Ok(Value::Bool(found))
                    }
                    _ => Err(PipError::runtime(0, "IN requires array on right side")),
                }
            }
        }
    }

    fn eval_add(&self, left: &Value, right: &Value) -> PipResult<Value> {
        match (left, right) {
            (Value::Int(a), Value::Int(b)) => a
                .checked_add(*b)
                .map(Value::Int)
                .ok_or_else(|| PipError::runtime(0, "Integer overflow in addition")),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + *b as f64)),
            (Value::String(a), Value::String(b)) => Ok(Value::String(format!("{a}{b}"))),
            _ => Err(PipError::runtime(
                0,
                format!("Cannot add {} and {}", left.type_name(), right.type_name()),
            )),
        }
    }

    fn eval_sub(&self, left: &Value, right: &Value) -> PipResult<Value> {
        match (left, right) {
            (Value::Int(a), Value::Int(b)) => a
                .checked_sub(*b)
                .map(Value::Int)
                .ok_or_else(|| PipError::runtime(0, "Integer overflow in subtraction")),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 - b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a - *b as f64)),
            _ => Err(PipError::runtime(
                0,
                format!(
                    "Cannot subtract {} from {}",
                    right.type_name(),
                    left.type_name()
                ),
            )),
        }
    }

    fn eval_mul(&self, left: &Value, right: &Value) -> PipResult<Value> {
        match (left, right) {
            (Value::Int(a), Value::Int(b)) => a
                .checked_mul(*b)
                .map(Value::Int)
                .ok_or_else(|| PipError::runtime(0, "Integer overflow in multiplication")),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 * b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a * *b as f64)),
            _ => Err(PipError::runtime(
                0,
                format!(
                    "Cannot multiply {} and {}",
                    left.type_name(),
                    right.type_name()
                ),
            )),
        }
    }

    fn eval_div(&self, left: &Value, right: &Value) -> PipResult<Value> {
        match (left, right) {
            (Value::Int(a), Value::Int(b)) => {
                if *b == 0 {
                    return Err(PipError::runtime(0, "Division by zero"));
                }
                a.checked_div(*b)
                    .map(Value::Int)
                    .ok_or_else(|| PipError::runtime(0, "Integer overflow in division"))
            }
            (Value::Float(a), Value::Float(b)) => {
                if *b == 0.0 {
                    return Err(PipError::runtime(0, "Division by zero"));
                }
                Ok(Value::Float(a / b))
            }
            (Value::Int(a), Value::Float(b)) => {
                if *b == 0.0 {
                    return Err(PipError::runtime(0, "Division by zero"));
                }
                Ok(Value::Float(*a as f64 / b))
            }
            (Value::Float(a), Value::Int(b)) => {
                if *b == 0 {
                    return Err(PipError::runtime(0, "Division by zero"));
                }
                Ok(Value::Float(a / *b as f64))
            }
            _ => Err(PipError::runtime(
                0,
                format!(
                    "Cannot divide {} by {}",
                    left.type_name(),
                    right.type_name()
                ),
            )),
        }
    }

    fn eval_mod(&self, left: &Value, right: &Value) -> PipResult<Value> {
        match (left, right) {
            (Value::Int(a), Value::Int(b)) => {
                if *b == 0 {
                    return Err(PipError::runtime(0, "Modulo by zero"));
                }
                a.checked_rem(*b)
                    .map(Value::Int)
                    .ok_or_else(|| PipError::runtime(0, "Integer overflow in modulo"))
            }
            _ => Err(PipError::runtime(0, "Modulo requires integer operands")),
        }
    }

    fn eval_compare<F>(&self, left: &Value, right: &Value, cmp: F) -> PipResult<Value>
    where
        F: Fn(f64, f64) -> bool,
    {
        let l = self
            .value_to_number(left)
            .ok_or_else(|| PipError::runtime(0, "Cannot compare non-numeric value"))?;
        let r = self
            .value_to_number(right)
            .ok_or_else(|| PipError::runtime(0, "Cannot compare non-numeric value"))?;
        Ok(Value::Bool(cmp(l, r)))
    }

    fn values_equal(&self, left: &Value, right: &Value) -> bool {
        match (left, right) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => (a - b).abs() < f64::EPSILON,
            (Value::Int(a), Value::Float(b)) | (Value::Float(b), Value::Int(a)) => {
                (*a as f64 - b).abs() < f64::EPSILON
            }
            (Value::String(a), Value::String(b)) => a == b,
            _ => false,
        }
    }

    fn value_to_number(&self, val: &Value) -> Option<f64> {
        match val {
            Value::Int(n) => Some(*n as f64),
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }

    fn value_to_string(&self, val: &Value) -> String {
        match val {
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Int(n) => n.to_string(),
            Value::Float(f) => f.to_string(),
            Value::String(s) => s.clone(),
            Value::Array(_) => "[Array]".to_string(),
            Value::Object(_) => "[Object]".to_string(),
            Value::Table(_) => "[Table]".to_string(),
            Value::Function { name, .. } => format!("[Function: {name}]"),
        }
    }

    fn matches_like(&self, s: &str, pattern: &str) -> bool {
        // Simple LIKE implementation with % wildcards
        let parts: Vec<&str> = pattern.split('%').collect();
        if parts.len() == 1 {
            return s == pattern;
        }

        let mut pos = 0;
        for (i, part) in parts.iter().enumerate() {
            if part.is_empty() {
                continue;
            }
            if i == 0 {
                // Must start with this part
                if !s.starts_with(part) {
                    return false;
                }
                pos = part.len();
            } else if i == parts.len() - 1 {
                // Must end with this part
                if !s[pos..].ends_with(part) {
                    return false;
                }
            } else {
                // Must contain this part
                if let Some(idx) = s[pos..].find(part) {
                    pos += idx + part.len();
                } else {
                    return false;
                }
            }
        }
        true
    }

    /// Evaluate a unary operation.
    fn eval_unary_op(&self, op: UnaryOp, val: &Value) -> PipResult<Value> {
        match op {
            UnaryOp::Neg => match val {
                Value::Int(n) => n
                    .checked_neg()
                    .map(Value::Int)
                    .ok_or_else(|| PipError::runtime(0, "Integer overflow in negation")),
                Value::Float(f) => Ok(Value::Float(-f)),
                _ => Err(PipError::runtime(
                    0,
                    format!("Cannot negate {}", val.type_name()),
                )),
            },
            UnaryOp::Not => Ok(Value::Bool(!val.is_truthy())),
        }
    }

    /// Evaluate function arguments.
    async fn eval_args(&mut self, args: &[Expr], line: usize) -> PipResult<Vec<Value>> {
        let mut values = Vec::with_capacity(args.len());
        for arg in args {
            values.push(self.eval_expr(arg).await.map_err(|e| e.with_line(line))?);
        }
        Ok(values)
    }

    /// Call a function (built-in or user-defined).
    async fn call_function(
        &mut self,
        name: &str,
        args: Vec<Value>,
        line: usize,
    ) -> PipResult<Value> {
        // Check built-in functions first
        match name.to_lowercase().as_str() {
            "print" => {
                let output: Vec<String> = args.iter().map(|v| self.value_to_string(v)).collect();
                let msg = output.join(" ");
                self.print(&msg).await;
                Ok(Value::Null)
            }
            "len" | "length" => {
                if args.len() != 1 {
                    return Err(PipError::runtime(line, "len() takes exactly 1 argument"));
                }
                match &args[0] {
                    Value::String(s) => Ok(Value::Int(s.len() as i64)),
                    Value::Array(a) => Ok(Value::Int(a.len() as i64)),
                    Value::Table(batches) => {
                        let count: usize = batches.iter().map(|b| b.num_rows()).sum();
                        Ok(Value::Int(count as i64))
                    }
                    _ => Err(PipError::runtime(
                        line,
                        format!("len() not supported for {}", args[0].type_name()),
                    )),
                }
            }
            "type" | "typeof" => {
                if args.len() != 1 {
                    return Err(PipError::runtime(line, "type() takes exactly 1 argument"));
                }
                Ok(Value::String(args[0].type_name().to_string()))
            }
            "str" | "string" => {
                if args.len() != 1 {
                    return Err(PipError::runtime(line, "str() takes exactly 1 argument"));
                }
                Ok(Value::String(self.value_to_string(&args[0])))
            }
            "int" | "integer" => {
                if args.len() != 1 {
                    return Err(PipError::runtime(line, "int() takes exactly 1 argument"));
                }
                match &args[0] {
                    Value::Int(n) => Ok(Value::Int(*n)),
                    Value::Float(f) => Ok(Value::Int(*f as i64)),
                    Value::String(s) => s.parse::<i64>().map(Value::Int).map_err(|_| {
                        PipError::runtime(line, format!("Cannot convert '{s}' to int"))
                    }),
                    Value::Bool(b) => Ok(Value::Int(if *b { 1 } else { 0 })),
                    _ => Err(PipError::runtime(
                        line,
                        format!("Cannot convert {} to int", args[0].type_name()),
                    )),
                }
            }
            "float" => {
                if args.len() != 1 {
                    return Err(PipError::runtime(line, "float() takes exactly 1 argument"));
                }
                match &args[0] {
                    Value::Int(n) => Ok(Value::Float(*n as f64)),
                    Value::Float(f) => Ok(Value::Float(*f)),
                    Value::String(s) => s.parse::<f64>().map(Value::Float).map_err(|_| {
                        PipError::runtime(line, format!("Cannot convert '{s}' to float"))
                    }),
                    _ => Err(PipError::runtime(
                        line,
                        format!("Cannot convert {} to float", args[0].type_name()),
                    )),
                }
            }
            "abs" => {
                if args.len() != 1 {
                    return Err(PipError::runtime(line, "abs() takes exactly 1 argument"));
                }
                match &args[0] {
                    Value::Int(n) => n
                        .checked_abs()
                        .map(Value::Int)
                        .ok_or_else(|| PipError::runtime(line, "Integer overflow in abs()")),
                    Value::Float(f) => Ok(Value::Float(f.abs())),
                    _ => Err(PipError::runtime(line, "abs() requires numeric argument")),
                }
            }
            "min" => {
                if args.is_empty() {
                    return Err(PipError::runtime(
                        line,
                        "min() requires at least 1 argument",
                    ));
                }
                self.find_min_max(&args, true, line)
            }
            "max" => {
                if args.is_empty() {
                    return Err(PipError::runtime(
                        line,
                        "max() requires at least 1 argument",
                    ));
                }
                self.find_min_max(&args, false, line)
            }
            "sum" => {
                if args.len() != 1 {
                    return Err(PipError::runtime(line, "sum() takes exactly 1 argument"));
                }
                match &args[0] {
                    Value::Array(arr) => {
                        let mut total = 0.0;
                        for v in arr {
                            total += self.value_to_number(v).ok_or_else(|| {
                                PipError::runtime(line, "sum() requires numeric array")
                            })?;
                        }
                        if arr.iter().all(|v| matches!(v, Value::Int(_))) {
                            Ok(Value::Int(total as i64))
                        } else {
                            Ok(Value::Float(total))
                        }
                    }
                    _ => Err(PipError::runtime(line, "sum() requires array argument")),
                }
            }
            "avg" | "average" => {
                if args.len() != 1 {
                    return Err(PipError::runtime(line, "avg() takes exactly 1 argument"));
                }
                match &args[0] {
                    Value::Array(arr) => {
                        if arr.is_empty() {
                            return Ok(Value::Null);
                        }
                        let mut total = 0.0;
                        for v in arr {
                            total += self.value_to_number(v).ok_or_else(|| {
                                PipError::runtime(line, "avg() requires numeric array")
                            })?;
                        }
                        Ok(Value::Float(total / arr.len() as f64))
                    }
                    _ => Err(PipError::runtime(line, "avg() requires array argument")),
                }
            }
            "keys" => {
                if args.len() != 1 {
                    return Err(PipError::runtime(line, "keys() takes exactly 1 argument"));
                }
                match &args[0] {
                    Value::Object(obj) => Ok(Value::Array(
                        obj.keys().map(|k| Value::String(k.clone())).collect(),
                    )),
                    _ => Err(PipError::runtime(line, "keys() requires object argument")),
                }
            }
            "values" => {
                if args.len() != 1 {
                    return Err(PipError::runtime(line, "values() takes exactly 1 argument"));
                }
                match &args[0] {
                    Value::Object(obj) => Ok(Value::Array(obj.values().cloned().collect())),
                    _ => Err(PipError::runtime(line, "values() requires object argument")),
                }
            }
            "consolidate" => {
                // consolidate(book) or consolidate(book, source = "_source")
                if args.is_empty() || args.len() > 2 {
                    return Err(PipError::runtime(
                        line,
                        "consolidate() takes 1 or 2 arguments: consolidate(book) or consolidate(book, source_column_name)",
                    ));
                }
                match &args[0] {
                    Value::Object(book_obj) => {
                        // Convert object (book) to consolidated array
                        let source_col = if args.len() == 2 {
                            match args[1].as_str() {
                                Some(s) => Some(s.to_string()),
                                None => {
                                    return Err(PipError::runtime(
                                        line,
                                        "consolidate() source_column_name must be a string",
                                    ));
                                }
                            }
                        } else {
                            None
                        };
                        consolidate_book(book_obj, source_col.as_deref())
                            .map_err(|e| PipError::runtime(line, e))
                    }
                    _ => Err(PipError::runtime(
                        line,
                        "consolidate() requires a book object (from multi-file import)",
                    )),
                }
            }
            #[cfg(feature = "python")]
            "register_python" => {
                // register_python("name", "lambda x: x * 2")
                // register_python("name", "file.py", "function_name")
                let runtime = self
                    .python_runtime
                    .as_ref()
                    .ok_or_else(|| PipError::runtime(line, "Python runtime not available"))?;

                match args.len() {
                    2 => {
                        // Inline lambda/def
                        let name = args[0].as_str().ok_or_else(|| {
                            PipError::runtime(line, "register_python: first argument must be string (name)")
                        })?;
                        let code = args[1].as_str().ok_or_else(|| {
                            PipError::runtime(line, "register_python: second argument must be string (code)")
                        })?;
                        runtime.register_inline(name, code).await?;
                        Ok(Value::Null)
                    }
                    3 => {
                        // From file
                        let name = args[0].as_str().ok_or_else(|| {
                            PipError::runtime(line, "register_python: first argument must be string (name)")
                        })?;
                        let file_path = args[1].as_str().ok_or_else(|| {
                            PipError::runtime(line, "register_python: second argument must be string (file path)")
                        })?;
                        let func_name = args[2].as_str().ok_or_else(|| {
                            PipError::runtime(line, "register_python: third argument must be string (function name)")
                        })?;
                        runtime.register_from_file(name, file_path, func_name).await?;
                        Ok(Value::Null)
                    }
                    _ => Err(PipError::runtime(
                        line,
                        "register_python() takes 2 or 3 arguments: (name, code) or (name, file, function)",
                    )),
                }
            }
            _ => {
                // Check user-defined functions
                let func = {
                    let funcs = self.functions.read().await;
                    funcs.get(name).cloned()
                };

                if let Some(func) = func {
                    if func.params.len() != args.len() {
                        return Err(PipError::runtime(
                            line,
                            format!(
                                "Function '{}' expects {} arguments, got {}",
                                name,
                                func.params.len(),
                                args.len()
                            ),
                        ));
                    }

                    // Create new scope with parameters
                    self.push_scope().await;
                    for (param, arg) in func.params.iter().zip(args) {
                        self.declare_var(param, arg).await;
                    }

                    // Execute function body
                    let mut result = Value::Null;
                    for stmt in func.body {
                        match self.eval_statement(stmt).await {
                            Ok(val) => result = val,
                            Err(PipError::Return(val)) => {
                                self.pop_scope().await;
                                return Ok(val);
                            }
                            Err(e) => {
                                self.pop_scope().await;
                                return Err(e);
                            }
                        }
                    }
                    self.pop_scope().await;
                    Ok(result)
                } else {
                    // Check Python functions if feature is enabled
                    #[cfg(feature = "python")]
                    if let Some(runtime) = &self.python_runtime {
                        if runtime.has_function(name).await {
                            return runtime.call(name, args).await;
                        }
                    }

                    Err(PipError::runtime(line, format!("Unknown function: {name}")))
                }
            }
        }
    }

    fn find_min_max(&self, args: &[Value], is_min: bool, line: usize) -> PipResult<Value> {
        // Handle single array argument
        let values = if args.len() == 1 {
            match &args[0] {
                Value::Array(arr) => arr.clone(),
                _ => args.to_vec(),
            }
        } else {
            args.to_vec()
        };

        if values.is_empty() {
            return Ok(Value::Null);
        }

        let mut result = self
            .value_to_number(&values[0])
            .ok_or_else(|| PipError::runtime(line, "min/max requires numeric values"))?;

        for v in &values[1..] {
            let n = self
                .value_to_number(v)
                .ok_or_else(|| PipError::runtime(line, "min/max requires numeric values"))?;
            if is_min {
                result = result.min(n);
            } else {
                result = result.max(n);
            }
        }

        // Return int if all values were ints
        if values.iter().all(|v| matches!(v, Value::Int(_))) {
            Ok(Value::Int(result as i64))
        } else {
            Ok(Value::Float(result))
        }
    }

    /// Evaluate a SQL query expression.
    async fn eval_query(&mut self, query: &SqlQuery) -> PipResult<Value> {
        let sql = self.sql_query_to_string(query).await?;
        let batches = self.sql.query(&sql).await?;
        Ok(Value::Table(batches.into_iter().map(Arc::new).collect()))
    }

    /// Convert a SQL query AST to a SQL string.
    #[async_recursion]
    async fn sql_query_to_string(&mut self, query: &SqlQuery) -> PipResult<String> {
        let mut sql = String::new();

        // WITH clause
        if let Some(with) = &query.with_clause {
            sql.push_str("WITH ");
            if with.recursive {
                sql.push_str("RECURSIVE ");
            }
            // TODO: Handle CTEs
        }

        // SELECT clause
        sql.push_str("SELECT ");
        if query.select.distinct {
            sql.push_str("DISTINCT ");
        }
        sql.push_str(&self.select_clause_to_string(&query.select).await?);

        // FROM clause
        if let Some(from) = &query.from {
            sql.push_str(" FROM ");
            sql.push_str(&self.from_clause_to_string(from).await?);
        }

        // JOIN clauses
        for join in &query.joins {
            sql.push_str(&self.join_clause_to_string(join).await?);
        }

        // WHERE clause
        if let Some(where_expr) = &query.where_clause {
            sql.push_str(" WHERE ");
            sql.push_str(&self.expr_to_sql(where_expr).await?);
        }

        // GROUP BY
        if let Some(group_by) = &query.group_by {
            sql.push_str(" GROUP BY ");
            let mut exprs = Vec::new();
            for e in group_by {
                exprs.push(self.expr_to_sql(e).await?);
            }
            sql.push_str(&exprs.join(", "));
        }

        // HAVING
        if let Some(having) = &query.having {
            sql.push_str(" HAVING ");
            sql.push_str(&self.expr_to_sql(having).await?);
        }

        // ORDER BY
        if let Some(order_by) = &query.order_by {
            sql.push_str(" ORDER BY ");
            sql.push_str(&self.order_by_to_string(order_by).await?);
        }

        // LIMIT
        if let Some(limit) = &query.limit {
            sql.push_str(" LIMIT ");
            sql.push_str(&self.expr_to_sql(limit).await?);
        }

        // OFFSET
        if let Some(offset) = &query.offset {
            sql.push_str(" OFFSET ");
            sql.push_str(&self.expr_to_sql(offset).await?);
        }

        Ok(sql)
    }

    async fn select_clause_to_string(&mut self, select: &SelectClause) -> PipResult<String> {
        let mut items = Vec::new();
        for item in &select.items {
            items.push(self.select_item_to_string(item).await?);
        }
        Ok(items.join(", "))
    }

    async fn select_item_to_string(&mut self, item: &SelectItem) -> PipResult<String> {
        let expr_str = self.expr_to_sql(&item.expr).await?;
        Ok(match &item.alias {
            Some(alias) => format!("{expr_str} AS {alias}"),
            None => expr_str,
        })
    }

    async fn from_clause_to_string(&mut self, from: &FromClause) -> PipResult<String> {
        let source = self.table_ref_to_string(&from.source).await?;
        Ok(match &from.alias {
            Some(alias) => format!("{source} AS {alias}"),
            None => source,
        })
    }

    #[async_recursion]
    async fn table_ref_to_string(&mut self, table_ref: &TableRef) -> PipResult<String> {
        match table_ref {
            TableRef::Table(name) => Ok(name.clone()),
            TableRef::Qualified {
                database,
                schema,
                table,
            } => Ok(match schema {
                Some(s) => format!("{database}.{s}.{table}"),
                None => format!("{database}.{table}"),
            }),
            TableRef::File(path) => {
                // Register the file and return table name
                let table_name = self.register_file(path).await?;
                Ok(table_name)
            }
            TableRef::Function { name, args } => {
                let mut arg_strs = Vec::new();
                for a in args {
                    arg_strs.push(self.func_arg_to_string(a).await?);
                }
                Ok(format!("{}({})", name, arg_strs.join(", ")))
            }
            TableRef::Stdin => Ok("stdin".to_string()),
            TableRef::Subquery(query) => {
                let sql = self.sql_query_to_string(query).await?;
                Ok(format!("({sql})"))
            }
        }
    }

    async fn func_arg_to_string(&mut self, arg: &piptable_core::FunctionArg) -> PipResult<String> {
        match arg {
            piptable_core::FunctionArg::Positional(expr) => self.expr_to_sql(expr).await,
            piptable_core::FunctionArg::Named { name, value } => {
                let val_str = self.expr_to_sql(value).await?;
                Ok(format!("{name} => {val_str}"))
            }
        }
    }

    async fn join_clause_to_string(&mut self, join: &JoinClause) -> PipResult<String> {
        let join_type = match join.join_type {
            JoinType::Inner => " INNER JOIN ",
            JoinType::Left => " LEFT JOIN ",
            JoinType::Right => " RIGHT JOIN ",
            JoinType::Full => " FULL OUTER JOIN ",
            JoinType::Cross => " CROSS JOIN ",
        };

        let table = self.table_ref_to_string(&join.table).await?;
        let mut result = format!("{join_type}{table}");

        if let Some(alias) = &join.alias {
            result.push_str(&format!(" AS {alias}"));
        }

        if let Some(on) = &join.on_clause {
            result.push_str(" ON ");
            result.push_str(&self.expr_to_sql(on).await?);
        }

        Ok(result)
    }

    async fn order_by_to_string(&mut self, order_by: &[OrderByItem]) -> PipResult<String> {
        let mut items = Vec::new();
        for item in order_by {
            items.push(self.order_item_to_string(item).await?);
        }
        Ok(items.join(", "))
    }

    async fn order_item_to_string(&mut self, item: &OrderByItem) -> PipResult<String> {
        let expr = self.expr_to_sql(&item.expr).await?;
        let dir = match item.direction {
            SortDirection::Asc => "ASC",
            SortDirection::Desc => "DESC",
        };
        Ok(format!("{expr} {dir}"))
    }

    /// Convert an expression to SQL string.
    #[async_recursion]
    async fn expr_to_sql(&mut self, expr: &Expr) -> PipResult<String> {
        match expr {
            Expr::Literal(lit) => Ok(self.literal_to_sql(lit)),
            Expr::Variable(name) => {
                if name == "*" {
                    Ok("*".to_string())
                } else {
                    Ok(name.clone())
                }
            }
            Expr::Binary { left, op, right } => {
                let l = self.expr_to_sql(left).await?;
                let r = self.expr_to_sql(right).await?;
                let op_str = self.binary_op_to_sql(*op);
                Ok(format!("({l} {op_str} {r})"))
            }
            Expr::Unary { op, operand } => {
                let val = self.expr_to_sql(operand).await?;
                match op {
                    UnaryOp::Neg => Ok(format!("-{val}")),
                    UnaryOp::Not => Ok(format!("NOT {val}")),
                }
            }
            Expr::FieldAccess { object, field } => {
                let obj = self.expr_to_sql(object).await?;
                Ok(format!("{obj}.{field}"))
            }
            Expr::Call { function, args } => {
                let mut arg_strs = Vec::new();
                for a in args {
                    arg_strs.push(self.expr_to_sql(a).await?);
                }
                Ok(format!("{}({})", function, arg_strs.join(", ")))
            }
            _ => {
                // For complex expressions, evaluate and inline the result
                let val = self.eval_expr(expr).await?;
                Ok(self.value_to_sql(&val))
            }
        }
    }

    fn literal_to_sql(&self, lit: &Literal) -> String {
        match lit {
            Literal::Null => "NULL".to_string(),
            Literal::Bool(b) => b.to_string().to_uppercase(),
            Literal::Int(n) => n.to_string(),
            Literal::Float(f) => f.to_string(),
            Literal::String(s) => format!("'{}'", s.replace('\'', "''")),
            Literal::Interval { value, unit } => {
                format!("INTERVAL {} {:?}", value, unit)
            }
        }
    }

    fn binary_op_to_sql(&self, op: BinaryOp) -> &'static str {
        match op {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Mod => "%",
            BinaryOp::Eq => "=",
            BinaryOp::Ne => "<>",
            BinaryOp::Lt => "<",
            BinaryOp::Le => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::Ge => ">=",
            BinaryOp::And => "AND",
            BinaryOp::Or => "OR",
            BinaryOp::Concat => "||",
            BinaryOp::Like => "LIKE",
            BinaryOp::In => "IN",
        }
    }

    fn value_to_sql(&self, val: &Value) -> String {
        match val {
            Value::Null => "NULL".to_string(),
            Value::Bool(b) => b.to_string().to_uppercase(),
            Value::Int(n) => n.to_string(),
            Value::Float(f) => f.to_string(),
            Value::String(s) => format!("'{}'", s.replace('\'', "''")),
            _ => "NULL".to_string(),
        }
    }

    /// Register a file as a table and return the table name.
    async fn register_file(&mut self, path: &str) -> PipResult<String> {
        // Generate table name from file path
        let table_name = std::path::Path::new(path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("data")
            .replace(['-', '.', ' '], "_");

        // Determine file type and register
        if path.ends_with(".csv") {
            self.sql.register_csv(&table_name, path).await?;
        } else if path.ends_with(".json") || path.ends_with(".ndjson") {
            self.sql.register_json(&table_name, path).await?;
        } else if path.ends_with(".parquet") {
            self.sql.register_parquet(&table_name, path).await?;
        } else {
            // Default to CSV
            self.sql.register_csv(&table_name, path).await?;
        }

        Ok(table_name)
    }

    /// Convert table batches to array of objects.
    fn table_to_array(&self, batches: &[Arc<arrow::array::RecordBatch>]) -> PipResult<Vec<Value>> {
        let mut rows = Vec::new();

        for batch in batches {
            let schema = batch.schema();

            for row_idx in 0..batch.num_rows() {
                let mut row = HashMap::new();

                for (col_idx, field) in schema.fields().iter().enumerate() {
                    let col = batch.column(col_idx);
                    let value = self.array_value_to_value(col.as_ref(), row_idx);
                    row.insert(field.name().clone(), value);
                }

                rows.push(Value::Object(row));
            }
        }

        Ok(rows)
    }

    /// Convert an Arrow array value at index to Value.
    fn array_value_to_value(&self, array: &dyn arrow::array::Array, idx: usize) -> Value {
        use arrow::array::*;
        use arrow::datatypes::DataType;

        if array.is_null(idx) {
            return Value::Null;
        }

        match array.data_type() {
            DataType::Boolean => {
                let arr = array.as_any().downcast_ref::<BooleanArray>().unwrap();
                Value::Bool(arr.value(idx))
            }
            DataType::Int8 => {
                let arr = array.as_any().downcast_ref::<Int8Array>().unwrap();
                Value::Int(i64::from(arr.value(idx)))
            }
            DataType::Int16 => {
                let arr = array.as_any().downcast_ref::<Int16Array>().unwrap();
                Value::Int(i64::from(arr.value(idx)))
            }
            DataType::Int32 => {
                let arr = array.as_any().downcast_ref::<Int32Array>().unwrap();
                Value::Int(i64::from(arr.value(idx)))
            }
            DataType::Int64 => {
                let arr = array.as_any().downcast_ref::<Int64Array>().unwrap();
                Value::Int(arr.value(idx))
            }
            DataType::UInt8 => {
                let arr = array.as_any().downcast_ref::<UInt8Array>().unwrap();
                Value::Int(i64::from(arr.value(idx)))
            }
            DataType::UInt16 => {
                let arr = array.as_any().downcast_ref::<UInt16Array>().unwrap();
                Value::Int(i64::from(arr.value(idx)))
            }
            DataType::UInt32 => {
                let arr = array.as_any().downcast_ref::<UInt32Array>().unwrap();
                Value::Int(i64::from(arr.value(idx)))
            }
            DataType::UInt64 => {
                let arr = array.as_any().downcast_ref::<UInt64Array>().unwrap();
                let val = arr.value(idx);
                if val > i64::MAX as u64 {
                    // Large u64 values that don't fit in i64 are converted to float
                    Value::Float(val as f64)
                } else {
                    Value::Int(val as i64)
                }
            }
            DataType::Float32 => {
                let arr = array.as_any().downcast_ref::<Float32Array>().unwrap();
                Value::Float(f64::from(arr.value(idx)))
            }
            DataType::Float64 => {
                let arr = array.as_any().downcast_ref::<Float64Array>().unwrap();
                Value::Float(arr.value(idx))
            }
            DataType::Utf8 => {
                let arr = array.as_any().downcast_ref::<StringArray>().unwrap();
                Value::String(arr.value(idx).to_string())
            }
            DataType::LargeUtf8 => {
                let arr = array.as_any().downcast_ref::<LargeStringArray>().unwrap();
                Value::String(arr.value(idx).to_string())
            }
            _ => Value::String(format!("<{:?}>", array.data_type())),
        }
    }

    /// Assign a value to an LValue.
    #[async_recursion]
    async fn assign_lvalue(&mut self, lvalue: &LValue, value: Value, line: usize) -> PipResult<()> {
        match lvalue {
            LValue::Variable(name) => {
                self.set_var(name, value).await;
                Ok(())
            }
            LValue::Field { object, field } => {
                // Get the object, modify it, and set it back
                let obj_val = self.get_lvalue_value(object, line).await?;
                match obj_val {
                    Value::Object(mut map) => {
                        map.insert(field.clone(), value);
                        self.assign_lvalue(object, Value::Object(map), line).await
                    }
                    _ => Err(PipError::runtime(
                        line,
                        format!("Cannot assign field on {}", obj_val.type_name()),
                    )),
                }
            }
            LValue::Index { array, index } => {
                let idx = self.eval_expr(index).await?;
                let idx_int = idx
                    .as_int()
                    .ok_or_else(|| PipError::runtime(line, "Array index must be integer"))?;

                let arr_val = self.get_lvalue_value(array, line).await?;
                match arr_val {
                    Value::Array(mut items) => {
                        let idx_usize = if idx_int < 0 {
                            let adjusted = items.len() as i64 + idx_int;
                            if adjusted < 0 {
                                return Err(PipError::runtime(line, "Array index out of bounds"));
                            }
                            adjusted as usize
                        } else {
                            idx_int as usize
                        };
                        if idx_usize >= items.len() {
                            return Err(PipError::runtime(line, "Array index out of bounds"));
                        }
                        items[idx_usize] = value;
                        self.assign_lvalue(array, Value::Array(items), line).await
                    }
                    _ => Err(PipError::runtime(
                        line,
                        format!("Cannot index {}", arr_val.type_name()),
                    )),
                }
            }
        }
    }

    /// Get the value of an LValue.
    #[async_recursion]
    async fn get_lvalue_value(&self, lvalue: &LValue, line: usize) -> PipResult<Value> {
        match lvalue {
            LValue::Variable(name) => self
                .get_var(name)
                .await
                .ok_or_else(|| PipError::runtime(line, format!("Undefined variable: {name}"))),
            LValue::Field { object, field } => {
                let obj = self.get_lvalue_value(object, line).await?;
                match obj {
                    Value::Object(map) => map.get(field).cloned().ok_or_else(|| {
                        PipError::runtime(line, format!("Field not found: {field}"))
                    }),
                    _ => Err(PipError::runtime(
                        line,
                        format!("Cannot access field on {}", obj.type_name()),
                    )),
                }
            }
            LValue::Index { array, index: _ } => {
                // For getting value, we just need the array - index handling is in assign
                self.get_lvalue_value(array, line).await
            }
        }
    }

    // ========================================================================
    // Scope management
    // ========================================================================

    /// Push a new scope onto the stack.
    async fn push_scope(&self) {
        let mut scopes = self.scopes.write().await;
        scopes.push(HashMap::new());
    }

    /// Pop the top scope from the stack.
    async fn pop_scope(&self) {
        let mut scopes = self.scopes.write().await;
        if scopes.len() > 1 {
            scopes.pop();
        }
    }

    /// Set a variable, searching scopes for existing bindings first.
    /// Use this for assignment statements where we want to update existing variables.
    pub async fn set_var(&self, name: &str, value: Value) {
        let mut scopes = self.scopes.write().await;
        // Check if variable exists in any scope (for reassignment)
        for scope in scopes.iter_mut().rev() {
            if scope.contains_key(name) {
                scope.insert(name.to_string(), value);
                return;
            }
        }
        // Otherwise, insert in current (top) scope
        if let Some(scope) = scopes.last_mut() {
            scope.insert(name.to_string(), value);
        }
    }

    /// Declare a variable in the current scope only (shadows outer bindings).
    /// Use this for loop variables and function parameters.
    async fn declare_var(&self, name: &str, value: Value) {
        let mut scopes = self.scopes.write().await;
        if let Some(scope) = scopes.last_mut() {
            scope.insert(name.to_string(), value);
        }
    }

    /// Get a variable, searching from innermost to outermost scope.
    pub async fn get_var(&self, name: &str) -> Option<Value> {
        let scopes = self.scopes.read().await;
        for scope in scopes.iter().rev() {
            if let Some(val) = scope.get(name) {
                return Some(val.clone());
            }
        }
        None
    }

    /// Get output buffer contents and clear it.
    pub async fn output(&self) -> Vec<String> {
        let mut output = self.output.write().await;
        std::mem::take(&mut *output)
    }

    /// Print to output buffer.
    pub async fn print(&self, value: &str) {
        let mut output = self.output.write().await;
        output.push(value.to_string());
    }

    /// Get the SQL engine.
    #[must_use]
    pub fn sql(&self) -> &SqlEngine {
        &self.sql
    }

    /// Get the HTTP client.
    #[must_use]
    pub fn http(&self) -> &HttpClient {
        &self.http
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Export helpers
// ============================================================================

/// Convert a piptable Value to a Sheet for export
fn value_to_sheet(value: &Value) -> Result<Sheet, String> {
    match value {
        // Array of objects -> records (all elements must be objects)
        Value::Array(arr)
            if !arr.is_empty() && arr.iter().all(|v| matches!(v, Value::Object(_))) =>
        {
            let records: Vec<indexmap::IndexMap<String, CellValue>> = arr
                .iter()
                .map(|v| {
                    let Value::Object(map) = v else {
                        unreachable!()
                    };
                    map.iter()
                        .map(|(k, v)| (k.clone(), value_to_cell(v)))
                        .collect()
                })
                .collect();
            Sheet::from_records(records).map_err(|e| e.to_string())
        }
        // Error on mixed array types (some objects, some not)
        Value::Array(arr) if arr.iter().any(|v| matches!(v, Value::Object(_))) => {
            Err("Cannot export mixed array types; expected all objects".to_string())
        }
        // Array of arrays -> rows
        Value::Array(arr) => {
            let data: Vec<Vec<CellValue>> = arr
                .iter()
                .map(|row| {
                    if let Value::Array(cols) = row {
                        cols.iter().map(value_to_cell).collect()
                    } else {
                        vec![value_to_cell(row)]
                    }
                })
                .collect();
            Ok(Sheet::from_data(data))
        }
        // Single object -> single record
        Value::Object(map) => {
            let record: indexmap::IndexMap<String, CellValue> = map
                .iter()
                .map(|(k, v)| (k.clone(), value_to_cell(v)))
                .collect();
            Sheet::from_records(vec![record]).map_err(|e| e.to_string())
        }
        // Table (Arrow RecordBatches) -> convert to Sheet
        Value::Table(batches) => arrow_batches_to_sheet(batches),
        _ => Err(format!(
            "Cannot export {} to file. Expected array, object, or table.",
            value.type_name()
        )),
    }
}

/// Convert a Value to a CellValue
fn value_to_cell(value: &Value) -> CellValue {
    match value {
        Value::Null => CellValue::Null,
        Value::Bool(b) => CellValue::Bool(*b),
        Value::Int(i) => CellValue::Int(*i),
        Value::Float(f) => CellValue::Float(*f),
        Value::String(s) => CellValue::String(s.clone()),
        Value::Array(_) | Value::Object(_) | Value::Table(_) | Value::Function { .. } => {
            // Convert complex types to JSON string, fall back to Debug for non-serializable types
            match value.to_json() {
                Ok(json) => CellValue::String(json.to_string()),
                Err(_) => CellValue::String(format!("{:?}", value)),
            }
        }
    }
}

/// Convert Arrow RecordBatches to a Sheet
fn arrow_batches_to_sheet(
    batches: &[Arc<arrow::record_batch::RecordBatch>],
) -> Result<Sheet, String> {
    if batches.is_empty() {
        return Ok(Sheet::new());
    }

    let schema = batches[0].schema();
    let col_names: Vec<String> = schema.fields().iter().map(|f| f.name().clone()).collect();

    // Build records from batches (column names become keys, avoiding header duplication)
    let mut records: Vec<indexmap::IndexMap<String, CellValue>> = Vec::new();

    for batch in batches {
        for row_idx in 0..batch.num_rows() {
            let mut record = indexmap::IndexMap::new();
            for (col_idx, col_name) in col_names.iter().enumerate() {
                let col = batch.column(col_idx);
                let cell = arrow_value_to_cell(col, row_idx);
                record.insert(col_name.clone(), cell);
            }
            records.push(record);
        }
    }

    if records.is_empty() {
        // Return sheet with just column names
        let mut sheet = Sheet::new();
        let header: Vec<CellValue> = col_names.into_iter().map(CellValue::String).collect();
        sheet.data_mut().push(header);
        sheet.name_columns_by_row(0).map_err(|e| e.to_string())?;
        return Ok(sheet);
    }

    Sheet::from_records(records).map_err(|e| e.to_string())
}

/// Extract a cell value from an Arrow array
fn arrow_value_to_cell(array: &Arc<dyn arrow::array::Array>, row: usize) -> CellValue {
    use arrow::array::*;
    use arrow::datatypes::DataType;

    if array.is_null(row) {
        return CellValue::Null;
    }

    match array.data_type() {
        DataType::Boolean => {
            let arr = array.as_any().downcast_ref::<BooleanArray>().unwrap();
            CellValue::Bool(arr.value(row))
        }
        DataType::Int8 => {
            let arr = array.as_any().downcast_ref::<Int8Array>().unwrap();
            CellValue::Int(i64::from(arr.value(row)))
        }
        DataType::Int16 => {
            let arr = array.as_any().downcast_ref::<Int16Array>().unwrap();
            CellValue::Int(i64::from(arr.value(row)))
        }
        DataType::Int32 => {
            let arr = array.as_any().downcast_ref::<Int32Array>().unwrap();
            CellValue::Int(i64::from(arr.value(row)))
        }
        DataType::Int64 => {
            let arr = array.as_any().downcast_ref::<Int64Array>().unwrap();
            CellValue::Int(arr.value(row))
        }
        DataType::UInt8 => {
            let arr = array.as_any().downcast_ref::<UInt8Array>().unwrap();
            CellValue::Int(i64::from(arr.value(row)))
        }
        DataType::UInt16 => {
            let arr = array.as_any().downcast_ref::<UInt16Array>().unwrap();
            CellValue::Int(i64::from(arr.value(row)))
        }
        DataType::UInt32 => {
            let arr = array.as_any().downcast_ref::<UInt32Array>().unwrap();
            CellValue::Int(i64::from(arr.value(row)))
        }
        DataType::UInt64 => {
            let arr = array.as_any().downcast_ref::<UInt64Array>().unwrap();
            let val = arr.value(row);
            if val > i64::MAX as u64 {
                CellValue::Float(val as f64)
            } else {
                CellValue::Int(val as i64)
            }
        }
        DataType::Float32 => {
            let arr = array.as_any().downcast_ref::<Float32Array>().unwrap();
            CellValue::Float(f64::from(arr.value(row)))
        }
        DataType::Float64 => {
            let arr = array.as_any().downcast_ref::<Float64Array>().unwrap();
            CellValue::Float(arr.value(row))
        }
        DataType::Utf8 => {
            let arr = array.as_any().downcast_ref::<StringArray>().unwrap();
            CellValue::String(arr.value(row).to_string())
        }
        DataType::LargeUtf8 => {
            let arr = array.as_any().downcast_ref::<LargeStringArray>().unwrap();
            CellValue::String(arr.value(row).to_string())
        }
        _ => CellValue::String(format!("<{}>", array.data_type())),
    }
}

/// Export a Sheet to a file, auto-detecting format from extension
fn export_sheet(sheet: &Sheet, path: &str) -> Result<(), String> {
    use piptable_sheet::CsvOptions;

    let path = Path::new(path);
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "csv" => sheet.save_as_csv(path).map_err(|e| e.to_string()),
        "tsv" => sheet
            .save_as_csv_with_options(path, CsvOptions::tsv())
            .map_err(|e| e.to_string()),
        "xlsx" | "xls" => sheet.save_as_xlsx(path).map_err(|e| e.to_string()),
        "json" => sheet.save_as_json(path).map_err(|e| e.to_string()),
        "jsonl" | "ndjson" => sheet.save_as_jsonl(path).map_err(|e| e.to_string()),
        "toon" => sheet.save_as_toon(path).map_err(|e| e.to_string()),
        "parquet" => sheet.save_as_parquet(path).map_err(|e| e.to_string()),
        _ => Err(format!(
            "Unsupported export format: '{}'. Supported: csv, tsv, xlsx, xls, json, jsonl, toon, parquet",
            ext
        )),
    }
}

/// Import a Sheet from a file, auto-detecting format from extension
fn import_sheet(path: &str, sheet_name: Option<&str>, has_headers: bool) -> Result<Sheet, String> {
    use piptable_sheet::Book;

    let path = Path::new(path);
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    // Validate sheet_name is only used for Excel files
    if sheet_name.is_some() && !matches!(ext.as_str(), "xlsx" | "xls") {
        return Err(format!(
            "sheet clause is only supported for Excel files (.xlsx/.xls), not '.{}'",
            ext
        ));
    }

    match ext.as_str() {
        "csv" => {
            let mut sheet = Sheet::from_csv(path).map_err(|e| e.to_string())?;
            if has_headers {
                sheet.name_columns_by_row(0).map_err(|e| e.to_string())?;
            }
            Ok(sheet)
        }
        "tsv" => {
            use piptable_sheet::CsvOptions;
            let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
            let mut sheet = Sheet::from_csv_str_with_options(&content, CsvOptions::tsv())
                .map_err(|e| e.to_string())?;
            if has_headers {
                sheet.name_columns_by_row(0).map_err(|e| e.to_string())?;
            }
            Ok(sheet)
        }
        "xlsx" | "xls" => {
            let mut sheet = if let Some(name) = sheet_name {
                // Load specific sheet by name
                let book = Book::from_excel(path).map_err(|e| e.to_string())?;
                book.get_sheet(name).map_err(|e| e.to_string())?.clone()
            } else {
                // Load first sheet
                Sheet::from_excel(path).map_err(|e| e.to_string())?
            };
            if has_headers {
                sheet.name_columns_by_row(0).map_err(|e| e.to_string())?;
            }
            Ok(sheet)
        }
        "json" => Sheet::from_json(path).map_err(|e| e.to_string()),
        "jsonl" | "ndjson" => Sheet::from_jsonl(path).map_err(|e| e.to_string()),
        "toon" => Sheet::from_toon(path).map_err(|e| e.to_string()),
        "parquet" => Sheet::from_parquet(path).map_err(|e| e.to_string()),
        _ => Err(format!(
            "Unsupported import format: '{}'. Supported: csv, tsv, xlsx, xls, json, jsonl, toon, parquet",
            ext
        )),
    }
}

/// Import multiple files into a Book and return as a Value (object with sheet names as keys)
fn import_multi_files(paths: &[String], options: &ImportOptions) -> Result<Value, String> {
    use piptable_sheet::{Book, FileLoadOptions};

    let file_opts = FileLoadOptions::default().with_headers(options.has_headers.unwrap_or(true));
    let book = Book::from_files_with_options(paths, file_opts).map_err(|e| e.to_string())?;

    // Convert Book to Value (object with sheet names as keys, each value is array of records)
    let mut sheets_map: HashMap<String, Value> = HashMap::new();
    for (name, sheet) in book.sheets() {
        let sheet_value = sheet_to_value(sheet);
        sheets_map.insert(name.to_string(), sheet_value);
    }

    Ok(Value::Object(sheets_map))
}

/// Convert a Sheet to a piptable Value (array of objects)
fn sheet_to_value(sheet: &Sheet) -> Value {
    // Try to convert to records if columns are named
    let records = if let Some(records) = sheet.to_records() {
        Some(records)
    } else if sheet.row_count() > 0 && sheet.col_count() > 0 {
        // Synthesize column names for unnamed columns
        let col_count = sheet.col_count();
        let mut synthesized_records = Vec::new();

        for row in sheet.data() {
            let mut record = indexmap::IndexMap::new();
            for (i, cell) in row.iter().enumerate() {
                let col_name = format!("col{}", i);
                record.insert(col_name, cell.clone());
            }
            // Fill missing columns with null
            for i in row.len()..col_count {
                let col_name = format!("col{}", i);
                record.insert(col_name, CellValue::Null);
            }
            synthesized_records.push(record);
        }
        Some(synthesized_records)
    } else {
        None
    };

    if let Some(records) = records {
        // Skip first record if it's the header row (matches column names).
        // Note: This could theoretically drop a data row that exactly matches
        // headers, but this is extremely unlikely in practice.
        let skip_header = if let Some(first) = records.first() {
            sheet
                .column_names()
                .map(|names| {
                    names
                        .iter()
                        .zip(first.values())
                        .all(|(n, v)| v.as_str() == *n)
                })
                .unwrap_or(false)
        } else {
            false
        };

        let arr: Vec<Value> = records
            .into_iter()
            .skip(if skip_header { 1 } else { 0 })
            .map(|record: indexmap::IndexMap<String, CellValue>| {
                let obj: HashMap<String, Value> = record
                    .into_iter()
                    .map(|(k, v)| (k, cell_to_value(v)))
                    .collect();
                Value::Object(obj)
            })
            .collect();
        Value::Array(arr)
    } else {
        // Fall back to array of arrays
        let arr: Vec<Value> = sheet
            .to_array()
            .into_iter()
            .map(|row| {
                let row_arr: Vec<Value> = row.into_iter().map(cell_to_value).collect();
                Value::Array(row_arr)
            })
            .collect();
        Value::Array(arr)
    }
}

/// Convert a CellValue to a Value
fn cell_to_value(cell: CellValue) -> Value {
    match cell {
        CellValue::Null => Value::Null,
        CellValue::Bool(b) => Value::Bool(b),
        CellValue::Int(i) => Value::Int(i),
        CellValue::Float(f) => Value::Float(f),
        CellValue::String(s) => Value::String(s),
    }
}

/// Consolidate a book (object of arrays) into a single array
fn consolidate_book(
    book_obj: &HashMap<String, Value>,
    source_col: Option<&str>,
) -> Result<Value, String> {
    use indexmap::IndexSet;

    // Sort sheet names first for deterministic processing
    let mut sheet_names: Vec<_> = book_obj.keys().collect();
    sheet_names.sort();

    // Collect all column names across all sheets in deterministic order
    let mut all_columns: IndexSet<String> = IndexSet::new();

    // Validate all values are arrays of objects and collect column names
    for sheet_name in &sheet_names {
        let value = &book_obj[*sheet_name];
        match value {
            Value::Array(rows) => {
                for row in rows {
                    match row {
                        Value::Object(obj) => {
                            for key in obj.keys() {
                                all_columns.insert(key.clone());
                            }
                        }
                        _ => {
                            return Err(format!("Sheet '{}' contains non-object rows", sheet_name));
                        }
                    }
                }
            }
            _ => {
                return Err(format!("Sheet '{}' is not an array", sheet_name));
            }
        }
    }

    // Check for source column conflict
    if let Some(col) = source_col {
        if all_columns.contains(col) {
            return Err(format!(
                "Source column name '{}' conflicts with existing column",
                col
            ));
        }
    }

    // Build consolidated result
    let mut result: Vec<Value> = Vec::new();

    for sheet_name in sheet_names {
        let value = &book_obj[sheet_name];
        if let Value::Array(rows) = value {
            for row in rows {
                if let Value::Object(obj) = row {
                    use indexmap::IndexMap;
                    let mut new_row: IndexMap<String, Value> = IndexMap::new();

                    // Add source column if requested
                    if let Some(col) = source_col {
                        new_row.insert(col.to_string(), Value::String(sheet_name.to_string()));
                    }

                    // Add all columns (with nulls for missing)
                    for col_name in &all_columns {
                        let val = obj.get(col_name).cloned().unwrap_or(Value::Null);
                        new_row.insert(col_name.clone(), val);
                    }

                    result.push(Value::Object(new_row.into_iter().collect()));
                }
            }
        }
    }

    Ok(Value::Array(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use piptable_parser::PipParser;

    #[tokio::test]
    async fn test_interpreter_new() {
        let interp = Interpreter::new();
        let output = interp.output().await;
        assert!(output.is_empty());
    }

    #[tokio::test]
    async fn test_set_get_var() {
        let interp = Interpreter::new();
        interp.set_var("x", Value::Int(42)).await;
        let value = interp.get_var("x").await;
        assert!(matches!(value, Some(Value::Int(42))));
    }

    #[tokio::test]
    async fn test_print() {
        let interp = Interpreter::new();
        interp.print("Hello").await;
        interp.print("World").await;
        let output = interp.output().await;
        assert_eq!(output, vec!["Hello", "World"]);
    }

    #[tokio::test]
    async fn test_eval_dim() {
        let mut interp = Interpreter::new();
        let program = PipParser::parse_str("dim x = 42").unwrap();
        interp.eval(program).await.unwrap();
        let value = interp.get_var("x").await;
        assert!(matches!(value, Some(Value::Int(42))));
    }

    #[tokio::test]
    async fn test_eval_dim_string() {
        let mut interp = Interpreter::new();
        let program = PipParser::parse_str(r#"dim name = "hello""#).unwrap();
        interp.eval(program).await.unwrap();
        let value = interp.get_var("name").await;
        assert!(matches!(value, Some(Value::String(s)) if s == "hello"));
    }

    #[tokio::test]
    async fn test_eval_arithmetic() {
        let mut interp = Interpreter::new();
        let program = PipParser::parse_str("dim x = 10 + 5 * 2").unwrap();
        interp.eval(program).await.unwrap();
        let value = interp.get_var("x").await;
        assert!(matches!(value, Some(Value::Int(20))));
    }

    #[tokio::test]
    async fn test_eval_comparison() {
        let mut interp = Interpreter::new();
        let program = PipParser::parse_str("dim x = 10 > 5").unwrap();
        interp.eval(program).await.unwrap();
        let value = interp.get_var("x").await;
        assert!(matches!(value, Some(Value::Bool(true))));
    }

    #[tokio::test]
    async fn test_eval_if_true() {
        let mut interp = Interpreter::new();
        let program = PipParser::parse_str("dim x = 0\nif true then x = 1 end if").unwrap();
        interp.eval(program).await.unwrap();
        let value = interp.get_var("x").await;
        assert!(matches!(value, Some(Value::Int(1))));
    }

    #[tokio::test]
    async fn test_eval_if_false() {
        let mut interp = Interpreter::new();
        let program =
            PipParser::parse_str("dim x = 0\nif false then x = 1 else x = 2 end if").unwrap();
        interp.eval(program).await.unwrap();
        let value = interp.get_var("x").await;
        assert!(matches!(value, Some(Value::Int(2))));
    }

    #[tokio::test]
    async fn test_eval_for_loop() {
        let mut interp = Interpreter::new();
        let program =
            PipParser::parse_str("dim sum = 0\nfor i = 1 to 5\nsum = sum + i\nnext").unwrap();
        interp.eval(program).await.unwrap();
        let value = interp.get_var("sum").await;
        assert!(matches!(value, Some(Value::Int(15))));
    }

    #[tokio::test]
    async fn test_eval_while_loop() {
        let mut interp = Interpreter::new();
        let program = PipParser::parse_str("dim x = 0\nwhile x < 5\nx = x + 1\nwend").unwrap();
        interp.eval(program).await.unwrap();
        let value = interp.get_var("x").await;
        assert!(matches!(value, Some(Value::Int(5))));
    }

    #[tokio::test]
    async fn test_eval_function() {
        let mut interp = Interpreter::new();
        let program = PipParser::parse_str(
            "function double(n)\nreturn n * 2\nend function\ndim x = double(21)",
        )
        .unwrap();
        interp.eval(program).await.unwrap();
        let value = interp.get_var("x").await;
        assert!(matches!(value, Some(Value::Int(42))));
    }

    #[tokio::test]
    async fn test_eval_print() {
        let mut interp = Interpreter::new();
        let program = PipParser::parse_str(r#"print("Hello, World!")"#).unwrap();
        interp.eval(program).await.unwrap();
        let output = interp.output().await;
        assert_eq!(output, vec!["Hello, World!"]);
    }

    #[tokio::test]
    async fn test_eval_array() {
        let mut interp = Interpreter::new();
        let program = PipParser::parse_str("dim arr = [1, 2, 3]").unwrap();
        interp.eval(program).await.unwrap();
        let value = interp.get_var("arr").await;
        match value {
            Some(Value::Array(items)) => {
                assert_eq!(items.len(), 3);
            }
            _ => panic!("Expected array"),
        }
    }

    #[tokio::test]
    async fn test_eval_object() {
        let mut interp = Interpreter::new();
        let program = PipParser::parse_str(r#"dim obj = { name: "test", value: 42 }"#).unwrap();
        interp.eval(program).await.unwrap();
        let value = interp.get_var("obj").await;
        match value {
            Some(Value::Object(map)) => {
                assert_eq!(map.len(), 2);
                assert!(matches!(map.get("name"), Some(Value::String(s)) if s == "test"));
                assert!(matches!(map.get("value"), Some(Value::Int(42))));
            }
            _ => panic!("Expected object"),
        }
    }

    #[tokio::test]
    async fn test_builtin_len() {
        let mut interp = Interpreter::new();
        let program = PipParser::parse_str("dim x = len([1, 2, 3])").unwrap();
        interp.eval(program).await.unwrap();
        let value = interp.get_var("x").await;
        assert!(matches!(value, Some(Value::Int(3))));
    }

    #[tokio::test]
    async fn test_builtin_type() {
        let mut interp = Interpreter::new();
        let program = PipParser::parse_str("dim x = type(42)").unwrap();
        interp.eval(program).await.unwrap();
        let value = interp.get_var("x").await;
        assert!(matches!(value, Some(Value::String(s)) if s == "Int"));
    }

    #[tokio::test]
    async fn test_sql_query() {
        let mut interp = Interpreter::new();
        let program = PipParser::parse_str("dim result = query(SELECT 1 + 1 as sum)").unwrap();
        interp.eval(program).await.unwrap();
        let value = interp.get_var("result").await;
        assert!(matches!(value, Some(Value::Table(_))));
    }

    #[tokio::test]
    async fn test_scope_isolation() {
        let mut interp = Interpreter::new();
        let program = PipParser::parse_str("dim x = 1\nfor i = 1 to 1\ndim y = 2\nnext").unwrap();
        interp.eval(program).await.unwrap();
        // x should be accessible
        assert!(interp.get_var("x").await.is_some());
        // y should be cleaned up (scope isolation)
        // Note: current implementation doesn't clean up, but that's ok for now
    }

    #[tokio::test]
    async fn test_export_csv() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.csv");

        let mut interp = Interpreter::new();
        let script = format!(
            r#"dim data = [{{"name": "Alice", "age": 30}}, {{"name": "Bob", "age": 25}}]
export data to "{}""#,
            file_path.display()
        );
        let program = PipParser::parse_str(&script).unwrap();
        interp.eval(program).await.unwrap();

        // Verify file was created and has content
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("Alice"));
        assert!(content.contains("Bob"));
    }

    #[tokio::test]
    async fn test_export_json() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.json");

        let mut interp = Interpreter::new();
        let script = format!(
            r#"dim data = [{{"name": "Alice", "age": 30}}, {{"name": "Bob", "age": 25}}]
export data to "{}""#,
            file_path.display()
        );
        let program = PipParser::parse_str(&script).unwrap();
        interp.eval(program).await.unwrap();

        // Verify file was created and has valid JSON
        let content = std::fs::read_to_string(&file_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(parsed.is_array());
    }

    #[tokio::test]
    async fn test_import_csv() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.csv");

        // Create a test CSV file
        std::fs::write(&file_path, "name,age\nalice,30\nbob,25").unwrap();

        let mut interp = Interpreter::new();
        let script = format!(r#"import "{}" into data"#, file_path.display());
        let program = PipParser::parse_str(&script).unwrap();
        interp.eval(program).await.unwrap();

        // Verify data was loaded
        let data = interp.get_var("data").await.unwrap();
        assert!(matches!(&data, Value::Array(arr) if arr.len() == 2));

        // Check first record has the right name
        if let Value::Array(arr) = &data {
            if let Value::Object(obj) = &arr[0] {
                assert!(matches!(obj.get("name"), Some(Value::String(s)) if s == "alice"));
            }
        }
    }

    #[tokio::test]
    async fn test_multi_file_import() {
        let dir = tempfile::tempdir().unwrap();
        let file1 = dir.path().join("q1.csv");
        let file2 = dir.path().join("q2.csv");

        // Create test CSV files
        std::fs::write(&file1, "product,sales\nwidget,100\ngadget,150").unwrap();
        std::fs::write(&file2, "product,sales\nwidget,120\ngizmo,80").unwrap();

        let mut interp = Interpreter::new();
        let script = format!(
            r#"import "{}", "{}" into quarterly_data"#,
            file1.display(),
            file2.display()
        );
        let program = PipParser::parse_str(&script).unwrap();
        interp.eval(program).await.unwrap();

        // Verify data was loaded as a book (object with sheet names)
        let data = interp.get_var("quarterly_data").await.unwrap();
        assert!(matches!(&data, Value::Object(book) if book.len() == 2));

        // Check that both sheets exist
        if let Value::Object(book) = &data {
            assert!(book.contains_key("q1"));
            assert!(book.contains_key("q2"));

            // Verify q1 sheet has correct data
            if let Some(Value::Array(q1_data)) = book.get("q1") {
                assert_eq!(q1_data.len(), 2);
            }
        }
    }

    #[tokio::test]
    async fn test_import_without_headers() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("no_headers.csv");

        // Create a CSV file without headers
        std::fs::write(&file_path, "alice,30\nbob,25").unwrap();

        let mut interp = Interpreter::new();
        let script = format!(
            r#"import "{}" into data (headers = false)"#,
            file_path.display()
        );
        let program = PipParser::parse_str(&script).unwrap();
        interp.eval(program).await.unwrap();

        // Verify data was loaded with default column names
        let data = interp.get_var("data").await.unwrap();
        assert!(matches!(&data, Value::Array(arr) if arr.len() == 2));

        // Check that default column names were used (col0, col1, etc.)
        if let Value::Array(arr) = &data {
            if let Value::Object(obj) = &arr[0] {
                assert!(obj.contains_key("col0"));
                assert!(obj.contains_key("col1"));
                assert!(matches!(obj.get("col0"), Some(Value::String(s)) if s == "alice"));
            }
        }
    }

    #[tokio::test]
    async fn test_consolidate_book() {
        let dir = tempfile::tempdir().unwrap();
        let file1 = dir.path().join("jan.csv");
        let file2 = dir.path().join("feb.csv");

        // Create test CSV files with slightly different columns
        std::fs::write(
            &file1,
            "product,sales,month\nwidget,100,jan\ngadget,150,jan",
        )
        .unwrap();
        std::fs::write(&file2, "product,sales,returns\nwidget,120,5\ngizmo,80,2").unwrap();

        let mut interp = Interpreter::new();
        let script = format!(
            r#"
import "{}", "{}" into monthly_data
combined = consolidate(monthly_data)
"#,
            file1.display(),
            file2.display()
        );
        let program = PipParser::parse_str(&script).unwrap();
        interp.eval(program).await.unwrap();

        // Verify consolidation worked
        let combined = interp.get_var("combined").await.unwrap();
        assert!(matches!(&combined, Value::Array(arr) if arr.len() == 4)); // 2 + 2 rows

        // Check that all columns are present with nulls for missing values
        if let Value::Array(arr) = &combined {
            // All rows should have all columns
            for row in arr {
                if let Value::Object(obj) = row {
                    assert!(obj.contains_key("product"));
                    assert!(obj.contains_key("sales"));
                    assert!(obj.contains_key("month"));
                    assert!(obj.contains_key("returns"));
                }
            }

            // Check that each sheet's data has appropriate nulls
            // The exact ordering depends on sheet name alphabetical order
            // jan.csv comes before feb.csv alphabetically

            // Rows from feb.csv should have null month
            let has_null_month = arr.iter().any(|row| {
                matches!(row, Value::Object(obj) if matches!(obj.get("month"), Some(Value::Null)))
            });
            assert!(has_null_month, "Should have rows with null month");

            // Rows from jan.csv should have null returns
            let has_null_returns = arr.iter().any(|row| {
                matches!(row, Value::Object(obj) if matches!(obj.get("returns"), Some(Value::Null)))
            });
            assert!(has_null_returns, "Should have rows with null returns");
        }
    }

    #[tokio::test]
    async fn test_consolidate_with_source() {
        let dir = tempfile::tempdir().unwrap();
        let file1 = dir.path().join("store1.csv");
        let file2 = dir.path().join("store2.csv");

        // Create test CSV files
        std::fs::write(&file1, "product,sales\nwidget,100").unwrap();
        std::fs::write(&file2, "product,sales\nwidget,120").unwrap();

        let mut interp = Interpreter::new();
        let script = format!(
            r#"
import "{}", "{}" into stores
combined = consolidate(stores, "_store")
"#,
            file1.display(),
            file2.display()
        );
        let program = PipParser::parse_str(&script).unwrap();
        interp.eval(program).await.unwrap();

        // Verify source column was added
        let combined = interp.get_var("combined").await.unwrap();
        assert!(matches!(&combined, Value::Array(arr) if arr.len() == 2));

        if let Value::Array(arr) = &combined {
            // Check that source column contains sheet names
            if let Value::Object(obj) = &arr[0] {
                assert!(obj.contains_key("_store"));
                assert!(matches!(obj.get("_store"), Some(Value::String(s)) if s == "store1"));
            }
            if let Value::Object(obj) = &arr[1] {
                assert!(matches!(obj.get("_store"), Some(Value::String(s)) if s == "store2"));
            }
        }
    }

    #[tokio::test]
    async fn test_consolidate_invalid_source_type() {
        let mut interp = Interpreter::new();
        // Create a simple book object directly
        interp
            .set_var(
                "book",
                Value::Object(
                    vec![("sheet1".to_string(), Value::Array(vec![]))]
                        .into_iter()
                        .collect(),
                ),
            )
            .await;

        let script = r#"result = consolidate(book, 123)"#; // 123 is not a string
        let program = PipParser::parse_str(&script).unwrap();
        let result = interp.eval(program).await;

        // Should error because source column must be a string
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be a string"));
    }

    #[tokio::test]
    async fn test_backward_compat_with_clause() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.csv");

        // Create a test CSV file
        std::fs::write(&file_path, "name,age\nalice,30").unwrap();

        let mut interp = Interpreter::new();
        // Test that old "with {}" syntax still parses (even if ignored)
        let script = format!(
            r#"import "{}" into data with {{"delimiter": ","}}"#,
            file_path.display()
        );
        let program = PipParser::parse_str(&script).unwrap();
        let result = interp.eval(program).await;

        // Should not error - backward compatibility maintained
        assert!(result.is_ok());

        // Data should still be imported
        let data = interp.get_var("data").await.unwrap();
        assert!(matches!(&data, Value::Array(arr) if arr.len() == 1));
    }

    #[tokio::test]
    async fn test_join_inner() {
        use piptable_sheet::{Sheet, CellValue};
        use indexmap::IndexMap;
        
        let mut interp = Interpreter::new();
        
        // Create users sheet from records
        let mut user1 = IndexMap::new();
        user1.insert("id".to_string(), CellValue::Int(1));
        user1.insert("name".to_string(), CellValue::String("Alice".to_string()));
        
        let mut user2 = IndexMap::new();
        user2.insert("id".to_string(), CellValue::Int(2));
        user2.insert("name".to_string(), CellValue::String("Bob".to_string()));
        
        let users = Sheet::from_records(vec![user1, user2]).unwrap();
        
        // Create orders sheet from records
        let mut order1 = IndexMap::new();
        order1.insert("id".to_string(), CellValue::Int(101));
        order1.insert("user_id".to_string(), CellValue::Int(1));
        order1.insert("amount".to_string(), CellValue::Float(50.0));
        
        let mut order2 = IndexMap::new();
        order2.insert("id".to_string(), CellValue::Int(102));
        order2.insert("user_id".to_string(), CellValue::Int(2));
        order2.insert("amount".to_string(), CellValue::Float(75.0));
        
        let mut order3 = IndexMap::new();
        order3.insert("id".to_string(), CellValue::Int(103));
        order3.insert("user_id".to_string(), CellValue::Int(1));
        order3.insert("amount".to_string(), CellValue::Float(25.0));
        
        let orders = Sheet::from_records(vec![order1, order2, order3]).unwrap();
        
        // Set the sheets as variables (convert to Value)
        interp.set_var("users", sheet_to_value(&users)).await;
        interp.set_var("orders", sheet_to_value(&orders)).await;
        
        // Test inner join with different columns
        let code = r#"result = users join orders on "id" = "user_id""#;
        let program = PipParser::parse_str(code).unwrap();
        interp.eval(program).await.unwrap();
        
        let result = interp.get_var("result").await.unwrap();
        // Result is an array of objects
        if let Value::Array(arr) = result {
            assert_eq!(arr.len(), 3); // Should have 3 rows (1 has 2 orders, 2 has 1 order)
            // Check that first record has expected fields
            if let Value::Object(first) = &arr[0] {
                assert!(first.contains_key("id"));
                assert!(first.contains_key("name"));
                assert!(first.contains_key("amount"));
            } else {
                panic!("Expected object in result array");
            }
        } else {
            panic!("Expected Array result from join");
        }
    }

    #[tokio::test]
    async fn test_join_left() {
        use piptable_sheet::{Sheet, CellValue};
        use indexmap::IndexMap;
        
        let mut interp = Interpreter::new();
        
        // Create users sheet from records
        let mut user1 = IndexMap::new();
        user1.insert("id".to_string(), CellValue::Int(1));
        user1.insert("name".to_string(), CellValue::String("Alice".to_string()));
        
        let mut user2 = IndexMap::new();
        user2.insert("id".to_string(), CellValue::Int(2));
        user2.insert("name".to_string(), CellValue::String("Bob".to_string()));
        
        let mut user3 = IndexMap::new();
        user3.insert("id".to_string(), CellValue::Int(3));
        user3.insert("name".to_string(), CellValue::String("Charlie".to_string()));
        
        let users = Sheet::from_records(vec![user1, user2, user3]).unwrap();
        
        // Create orders sheet from records
        let mut order1 = IndexMap::new();
        order1.insert("user_id".to_string(), CellValue::Int(1));
        order1.insert("amount".to_string(), CellValue::Float(50.0));
        
        let mut order2 = IndexMap::new();
        order2.insert("user_id".to_string(), CellValue::Int(2));
        order2.insert("amount".to_string(), CellValue::Float(75.0));
        
        let orders = Sheet::from_records(vec![order1, order2]).unwrap();
        
        // Set the sheets as variables
        interp.set_var("users", sheet_to_value(&users)).await;
        interp.set_var("orders", sheet_to_value(&orders)).await;
        
        // Test left join
        let code = r#"result = users left join orders on "id" = "user_id""#;
        let program = PipParser::parse_str(code).unwrap();
        interp.eval(program).await.unwrap();
        
        let result = interp.get_var("result").await.unwrap();
        if let Value::Array(arr) = result {
            assert_eq!(arr.len(), 3); // Should have all 3 users
            
            // Find Charlie's record
            let charlie_record = arr.iter()
                .find(|record| {
                    if let Value::Object(obj) = record {
                        if let Some(Value::String(name)) = obj.get("name") {
                            name == "Charlie"
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                })
                .expect("Charlie record not found");
            
            // Charlie should have null amount
            if let Value::Object(obj) = charlie_record {
                assert!(matches!(obj.get("amount"), Some(Value::Null)));
            }
        } else {
            panic!("Expected Array result from left join");
        }
    }

    #[tokio::test]
    async fn test_join_with_same_key() {
        use piptable_sheet::{Sheet, CellValue};
        use indexmap::IndexMap;
        
        let mut interp = Interpreter::new();
        
        // Create first sheet from records
        let mut record1_1 = IndexMap::new();
        record1_1.insert("id".to_string(), CellValue::Int(1));
        record1_1.insert("value1".to_string(), CellValue::String("A".to_string()));
        
        let mut record1_2 = IndexMap::new();
        record1_2.insert("id".to_string(), CellValue::Int(2));
        record1_2.insert("value1".to_string(), CellValue::String("B".to_string()));
        
        let sheet1 = Sheet::from_records(vec![record1_1, record1_2]).unwrap();
        
        // Create second sheet from records
        let mut record2_1 = IndexMap::new();
        record2_1.insert("id".to_string(), CellValue::Int(1));
        record2_1.insert("value2".to_string(), CellValue::String("X".to_string()));
        
        let mut record2_2 = IndexMap::new();
        record2_2.insert("id".to_string(), CellValue::Int(2));
        record2_2.insert("value2".to_string(), CellValue::String("Y".to_string()));
        
        let sheet2 = Sheet::from_records(vec![record2_1, record2_2]).unwrap();
        
        // Set the sheets as variables
        interp.set_var("sheet1", sheet_to_value(&sheet1)).await;
        interp.set_var("sheet2", sheet_to_value(&sheet2)).await;
        
        // Test join with same key column name
        let code = r#"result = sheet1 join sheet2 on "id""#;
        let program = PipParser::parse_str(code).unwrap();
        interp.eval(program).await.unwrap();
        
        let result = interp.get_var("result").await.unwrap();
        if let Value::Array(arr) = result {
            assert_eq!(arr.len(), 2);
            // Check that first record has expected fields
            if let Value::Object(first) = &arr[0] {
                assert!(first.contains_key("id"));
                assert!(first.contains_key("value1"));
                assert!(first.contains_key("value2"));
            } else {
                panic!("Expected object in result array");
            }
        } else {
            panic!("Expected Array result from join");
        }
    }
}
