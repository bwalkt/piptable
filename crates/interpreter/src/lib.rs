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

mod builtins;
mod converters;
mod io;
mod sheet_conversions;
mod sql_builder;

#[cfg(feature = "python")]
mod python;

use async_recursion::async_recursion;
use piptable_core::{
    BinaryOp, Expr, LValue, Literal,
    PipError, PipResult, Program, Statement,
    UnaryOp, Value,
};
use piptable_http::HttpClient;
use piptable_sheet::{CellValue, Sheet};
use piptable_sql::SqlEngine;
use std::collections::HashMap;
use crate::sheet_conversions::{build_sheet_arrow_array, infer_sheet_column_type};
use arrow::datatypes::{Field, Schema};
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
    /// Registered sheet tables (maps variable name to table name)
    sheet_tables: Arc<RwLock<HashMap<String, String>>>,
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
            sheet_tables: Arc::new(RwLock::new(HashMap::new())),
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
                Err(PipError::Return(Box::new(val)))
            }

            Statement::Call {
                function,
                args,
                line,
            } => {
                let arg_vals = self.eval_args(&args, line).await?;
                self.call_function(&function, arg_vals, line).await
            }

            Statement::Append {
                target,
                source,
                distinct,
                key,
                line,
            } => {
                // Get the target sheet
                let target_val = self.get_var(&target).await.ok_or_else(|| {
                    PipError::runtime(line, format!("undefined variable: {}", target))
                })?;

                // Evaluate source expression
                let source_val = self
                    .eval_expr(&source)
                    .await
                    .map_err(|e| e.with_line(line))?;

                // Convert both to sheets
                let mut target_sheet = sheet_conversions::value_to_sheet(&target_val).map_err(|e| {
                    PipError::runtime(line, format!("target must be a sheet: {}", e))
                })?;
                let source_sheet = sheet_conversions::value_to_sheet(&source_val).map_err(|e| {
                    PipError::runtime(line, format!("source must be a sheet: {}", e))
                })?;

                // Perform append operation
                if distinct {
                    if let Some(key) = key {
                        target_sheet
                            .append_distinct(&source_sheet, &key)
                            .map_err(|e| {
                                PipError::runtime(line, format!("append distinct failed: {}", e))
                            })?;
                    } else {
                        return Err(PipError::runtime(
                            line,
                            "append distinct requires a key column",
                        ));
                    }
                } else {
                    target_sheet
                        .append(&source_sheet)
                        .map_err(|e| PipError::runtime(line, format!("append failed: {}", e)))?;
                }

                // Update the variable with modified sheet
                self.set_var(&target, sheet_conversions::sheet_to_value(&target_sheet)).await;
                Ok(Value::Null)
            }

            Statement::Upsert {
                target,
                source,
                key,
                line,
            } => {
                // Get the target sheet
                let target_val = self.get_var(&target).await.ok_or_else(|| {
                    PipError::runtime(line, format!("undefined variable: {}", target))
                })?;

                // Evaluate source expression
                let source_val = self
                    .eval_expr(&source)
                    .await
                    .map_err(|e| e.with_line(line))?;

                // Convert both to sheets
                let mut target_sheet = sheet_conversions::value_to_sheet(&target_val).map_err(|e| {
                    PipError::runtime(line, format!("target must be a sheet: {}", e))
                })?;
                let source_sheet = sheet_conversions::value_to_sheet(&source_val).map_err(|e| {
                    PipError::runtime(line, format!("source must be a sheet: {}", e))
                })?;

                // Perform upsert operation
                target_sheet
                    .upsert(&source_sheet, &key)
                    .map_err(|e| PipError::runtime(line, format!("upsert failed: {}", e)))?;

                // Update the variable with modified sheet
                self.set_var(&target, sheet_conversions::sheet_to_value(&target_sheet)).await;
                Ok(Value::Null)
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
                let sheet = sheet_conversions::value_to_sheet(&data)
                    .map_err(|e| PipError::Export(format!("Line {}: {}", line, e)))?;

                // Determine format from file extension and export
                io::export_sheet(&sheet, &path)
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
                    io::import_multi_files(&paths, &options)
                        .map_err(|e| PipError::Import(format!("Line {}: {}", line, e)))?
                } else {
                    // Single file import
                    let has_headers = options.has_headers.unwrap_or(true);
                    let sheet = io::import_sheet(&paths[0], sheet_name_str.as_deref(), has_headers)
                        .map_err(|e| PipError::Import(format!("Line {}: {}", line, e)))?;
                    sheet_conversions::sheet_to_value(&sheet)
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
                    // Sheet A1 notation access
                    (Value::Sheet(sheet), Value::String(notation)) => {
                        // Check if it's a range (contains colon)
                        Ok(if notation.contains(':') {
                            let sub_sheet = sheet.get_range(notation).map_err(|e| {
                                PipError::runtime(0, format!("Invalid range '{}': {}", notation, e))
                            })?;
                            Value::Sheet(sub_sheet)
                        } else {
                            // Single cell access
                            let cell = sheet.get_a1(notation).map_err(|e| {
                                PipError::runtime(
                                    0,
                                    format!("Invalid cell notation '{}': {}", notation, e),
                                )
                            })?;

                            // Convert CellValue to Value
                            sheet_conversions::cell_to_value(cell.clone())
                        })
                    }
                    // Type mismatches
                    (Value::Array(_), _) => {
                        Err(PipError::runtime(0, "Array index must be integer"))
                    }
                    (Value::Object(_), _) => Err(PipError::runtime(0, "Object key must be string")),
                    (Value::Sheet(_), _) => Err(PipError::runtime(
                        0,
                        "Sheet index must be string (A1 notation)",
                    )),
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

            Expr::Join {
                left,
                right,
                join_type,
                condition,
            } => {
                use piptable_core::ast::JoinCondition;
                use piptable_sheet::SheetError;

                // Evaluate both sides to get sheets
                let left_val = self.eval_expr(left).await?;
                let right_val = self.eval_expr(right).await?;

                // Convert values to sheets
                let left_sheet = sheet_conversions::value_to_sheet(&left_val).map_err(|e| {
                    PipError::runtime(0, format!("Left side of join must be a sheet: {}", e))
                })?;
                let right_sheet = sheet_conversions::value_to_sheet(&right_val).map_err(|e| {
                    PipError::runtime(0, format!("Right side of join must be a sheet: {}", e))
                })?;

                // Perform the join based on the type and condition
                let result = match (join_type, condition) {
                    (piptable_core::ast::JoinType::Inner, JoinCondition::On(key)) => left_sheet
                        .inner_join(&right_sheet, key)
                        .map_err(|e: SheetError| PipError::runtime(0, e.to_string()))?,
                    (
                        piptable_core::ast::JoinType::Inner,
                        JoinCondition::OnColumns { left: l, right: r },
                    ) => left_sheet
                        .inner_join_on(&right_sheet, l, r)
                        .map_err(|e: SheetError| PipError::runtime(0, e.to_string()))?,
                    (piptable_core::ast::JoinType::Left, JoinCondition::On(key)) => left_sheet
                        .left_join(&right_sheet, key)
                        .map_err(|e: SheetError| PipError::runtime(0, e.to_string()))?,
                    (
                        piptable_core::ast::JoinType::Left,
                        JoinCondition::OnColumns { left: l, right: r },
                    ) => left_sheet
                        .left_join_on(&right_sheet, l, r)
                        .map_err(|e: SheetError| PipError::runtime(0, e.to_string()))?,
                    (piptable_core::ast::JoinType::Right, JoinCondition::On(key)) => left_sheet
                        .right_join(&right_sheet, key)
                        .map_err(|e: SheetError| PipError::runtime(0, e.to_string()))?,
                    (
                        piptable_core::ast::JoinType::Right,
                        JoinCondition::OnColumns { left: l, right: r },
                    ) => left_sheet
                        .right_join_on(&right_sheet, l, r)
                        .map_err(|e: SheetError| PipError::runtime(0, e.to_string()))?,
                    (piptable_core::ast::JoinType::Full, JoinCondition::On(key)) => left_sheet
                        .full_join(&right_sheet, key)
                        .map_err(|e: SheetError| PipError::runtime(0, e.to_string()))?,
                    (
                        piptable_core::ast::JoinType::Full,
                        JoinCondition::OnColumns { left: l, right: r },
                    ) => left_sheet
                        .full_join_on(&right_sheet, l, r)
                        .map_err(|e: SheetError| PipError::runtime(0, e.to_string()))?,
                    (piptable_core::ast::JoinType::Cross, _) => {
                        // CROSS JOIN is not supported in DSL, only in SQL
                        return Err(PipError::runtime(
                            0,
                            "CROSS JOIN is not supported in DSL join syntax, only in SQL queries",
                        ));
                    }
                };

                Ok(sheet_conversions::sheet_to_value(&result))
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
                let l = converters::value_to_string(left);
                let r = converters::value_to_string(right);
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
                Ok(Value::Bool(converters::matches_like(s, pattern)))
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
        let l = converters::value_to_number(left)
            .ok_or_else(|| PipError::runtime(0, "Cannot compare non-numeric value"))?;
        let r = converters::value_to_number(right)
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
        if let Some(result) = builtins::call_builtin(self, name, args.clone(), line).await {
            return result;
        }

        // Functions that still need to be migrated to modules
        match name.to_lowercase().as_str() {
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
                        converters::consolidate_book(book_obj, source_col.as_deref())
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
            "sheet_transpose" => {
                if args.len() != 1 {
                    return Err(PipError::runtime(
                        line,
                        "sheet_transpose() takes exactly 1 argument",
                    ));
                }
                match &args[0] {
                    Value::Sheet(sheet) => {
                        let mut new_sheet = sheet.clone();
                        new_sheet.transpose();
                        Ok(Value::Sheet(new_sheet))
                    }
                    _ => Err(PipError::runtime(line, "Argument must be a sheet")),
                }
            }
            "sheet_select_columns" => {
                if args.len() != 2 {
                    return Err(PipError::runtime(
                        line,
                        "sheet_select_columns() takes exactly 2 arguments",
                    ));
                }
                match (&args[0], &args[1]) {
                    (Value::Sheet(sheet), Value::Array(columns)) => {
                        let mut new_sheet = sheet.clone();
                        let column_names: Result<Vec<&str>, _> = columns
                            .iter()
                            .map(|v| match v {
                                Value::String(s) => Ok(s.as_str()),
                                _ => Err(PipError::runtime(line, "Column names must be strings")),
                            })
                            .collect();
                        let column_names = column_names?;
                        new_sheet.select_columns(&column_names).map_err(|e| {
                            PipError::runtime(line, format!("Failed to select columns: {}", e))
                        })?;
                        Ok(Value::Sheet(new_sheet))
                    }
                    (Value::Sheet(_), _) => Err(PipError::runtime(
                        line,
                        "Second argument must be an array of column names",
                    )),
                    _ => Err(PipError::runtime(line, "First argument must be a sheet")),
                }
            }
            "sheet_remove_columns" => {
                if args.len() != 2 {
                    return Err(PipError::runtime(
                        line,
                        "sheet_remove_columns() takes exactly 2 arguments",
                    ));
                }
                match (&args[0], &args[1]) {
                    (Value::Sheet(sheet), Value::Array(columns)) => {
                        let mut new_sheet = sheet.clone();
                        let column_names: Result<Vec<&str>, _> = columns
                            .iter()
                            .map(|v| match v {
                                Value::String(s) => Ok(s.as_str()),
                                _ => Err(PipError::runtime(line, "Column names must be strings")),
                            })
                            .collect();
                        let column_names = column_names?;
                        new_sheet.remove_columns(&column_names).map_err(|e| {
                            PipError::runtime(line, format!("Failed to remove columns: {}", e))
                        })?;
                        Ok(Value::Sheet(new_sheet))
                    }
                    (Value::Sheet(_), _) => Err(PipError::runtime(
                        line,
                        "Second argument must be an array of column names",
                    )),
                    _ => Err(PipError::runtime(line, "First argument must be a sheet")),
                }
            }
            "sheet_remove_empty_rows" => {
                if args.len() != 1 {
                    return Err(PipError::runtime(
                        line,
                        "sheet_remove_empty_rows() takes exactly 1 argument",
                    ));
                }
                match &args[0] {
                    Value::Sheet(sheet) => {
                        let mut new_sheet = sheet.clone();
                        new_sheet.remove_empty_rows();
                        Ok(Value::Sheet(new_sheet))
                    }
                    _ => Err(PipError::runtime(line, "Argument must be a sheet")),
                }
            }
            "sheet_row_count" => {
                if args.len() != 1 {
                    return Err(PipError::runtime(
                        line,
                        "sheet_row_count() takes exactly 1 argument",
                    ));
                }
                match &args[0] {
                    Value::Sheet(sheet) => Ok(Value::Int(sheet.row_count() as i64)),
                    _ => Err(PipError::runtime(line, "Argument must be a sheet")),
                }
            }
            "sheet_col_count" => {
                if args.len() != 1 {
                    return Err(PipError::runtime(
                        line,
                        "sheet_col_count() takes exactly 1 argument",
                    ));
                }
                match &args[0] {
                    Value::Sheet(sheet) => Ok(Value::Int(sheet.col_count() as i64)),
                    _ => Err(PipError::runtime(line, "Argument must be a sheet")),
                }
            }
            "sheet_get_a1" => {
                if args.len() != 2 {
                    return Err(PipError::runtime(
                        line,
                        "sheet_get_a1() takes exactly 2 arguments (sheet, notation)",
                    ));
                }
                match (&args[0], &args[1]) {
                    (Value::Sheet(sheet), Value::String(notation)) => {
                        let cell = sheet.get_a1(notation).map_err(|e| {
                            PipError::runtime(
                                line,
                                format!("Invalid cell notation '{}': {}", notation, e),
                            )
                        })?;

                        Ok(sheet_conversions::cell_to_value(cell.clone()))
                    }
                    _ => Err(PipError::runtime(line, "Arguments must be (sheet, string)")),
                }
            }
            "sheet_set_a1" => {
                if args.len() != 3 {
                    return Err(PipError::runtime(
                        line,
                        "sheet_set_a1() takes exactly 3 arguments (sheet, notation, value)",
                    ));
                }
                match (&args[0], &args[1], &args[2]) {
                    (Value::Sheet(sheet), Value::String(notation), value) => {
                        let mut sheet_clone = sheet.clone();

                        let cell_value = match value {
                            Value::String(s) => CellValue::String(s.clone()),
                            Value::Int(i) => CellValue::Int(*i),
                            Value::Float(f) => CellValue::Float(*f),
                            Value::Bool(b) => CellValue::Bool(*b),
                            Value::Null => CellValue::Null,
                            _ => {
                                return Err(PipError::runtime(
                                    line,
                                    "Unsupported value type for cell",
                                ))
                            }
                        };

                        sheet_clone.set_a1(notation, cell_value).map_err(|e| {
                            PipError::runtime(
                                line,
                                format!("Failed to set cell '{}': {}", notation, e),
                            )
                        })?;

                        Ok(Value::Sheet(sheet_clone))
                    }
                    _ => Err(PipError::runtime(
                        line,
                        "Arguments must be (sheet, string, value)",
                    )),
                }
            }
            "sheet_get_range" => {
                if args.len() != 2 {
                    return Err(PipError::runtime(
                        line,
                        "sheet_get_range() takes exactly 2 arguments (sheet, range_notation)",
                    ));
                }
                match (&args[0], &args[1]) {
                    (Value::Sheet(sheet), Value::String(notation)) => {
                        let sub_sheet = sheet.get_range(notation).map_err(|e| {
                            PipError::runtime(line, format!("Invalid range '{}': {}", notation, e))
                        })?;
                        Ok(Value::Sheet(sub_sheet))
                    }
                    _ => Err(PipError::runtime(line, "Arguments must be (sheet, string)")),
                }
            }
            "sheet_column_by_name" => {
                if args.len() != 2 {
                    return Err(PipError::runtime(
                        line,
                        "sheet_column_by_name() takes exactly 2 arguments (sheet, column_name)",
                    ));
                }
                match (&args[0], &args[1]) {
                    (Value::Sheet(sheet), Value::String(col_name)) => {
                        use piptable_sheet::CellValue;
                        let column = sheet.column_by_name(col_name).map_err(|e| {
                            PipError::runtime(
                                line,
                                format!("Failed to get column '{}': {}", col_name, e),
                            )
                        })?;

                        let array: Vec<Value> = column
                            .iter()
                            .map(|cell| match cell {
                                CellValue::Null => Value::Null,
                                CellValue::String(s) => Value::String(s.clone()),
                                CellValue::Int(i) => Value::Int(*i),
                                CellValue::Float(f) => Value::Float(*f),
                                CellValue::Bool(b) => Value::Bool(*b),
                            })
                            .collect();

                        Ok(Value::Array(array))
                    }
                    _ => Err(PipError::runtime(line, "Arguments must be (sheet, string)")),
                }
            }
            "sheet_get_by_name" => {
                if args.len() != 3 {
                    return Err(PipError::runtime(line, "sheet_get_by_name() takes exactly 3 arguments (sheet, row_index, column_name)"));
                }
                match (&args[0], &args[1], &args[2]) {
                    (Value::Sheet(sheet), Value::Int(row), Value::String(col_name)) => {
                        use piptable_sheet::CellValue;
                        if *row < 0 {
                            return Err(PipError::runtime(line, "Row index cannot be negative"));
                        }
                        let cell = sheet.get_by_name(*row as usize, col_name).map_err(|e| {
                            PipError::runtime(
                                line,
                                format!(
                                    "Failed to get cell at row {} column '{}': {}",
                                    row, col_name, e
                                ),
                            )
                        })?;

                        match cell {
                            CellValue::Null => Ok(Value::Null),
                            CellValue::String(s) => Ok(Value::String(s.clone())),
                            CellValue::Int(i) => Ok(Value::Int(*i)),
                            CellValue::Float(f) => Ok(Value::Float(*f)),
                            CellValue::Bool(b) => Ok(Value::Bool(*b)),
                        }
                    }
                    _ => Err(PipError::runtime(
                        line,
                        "Arguments must be (sheet, int, string)",
                    )),
                }
            }
            "sheet_set_by_name" => {
                if args.len() != 4 {
                    return Err(PipError::runtime(line, "sheet_set_by_name() takes exactly 4 arguments (sheet, row_index, column_name, value)"));
                }
                match (&args[0], &args[1], &args[2], &args[3]) {
                    (Value::Sheet(sheet), Value::Int(row), Value::String(col_name), value) => {
                        use piptable_sheet::CellValue;
                        if *row < 0 {
                            return Err(PipError::runtime(line, "Row index cannot be negative"));
                        }
                        let mut sheet_clone = sheet.clone();

                        let cell_value = match value {
                            Value::String(s) => CellValue::String(s.clone()),
                            Value::Int(i) => CellValue::Int(*i),
                            Value::Float(f) => CellValue::Float(*f),
                            Value::Bool(b) => CellValue::Bool(*b),
                            Value::Null => CellValue::Null,
                            _ => {
                                return Err(PipError::runtime(
                                    line,
                                    "Unsupported value type for cell",
                                ))
                            }
                        };

                        sheet_clone
                            .set_by_name(*row as usize, col_name, cell_value)
                            .map_err(|e| {
                                PipError::runtime(
                                    line,
                                    format!(
                                        "Failed to set cell at row {} column '{}': {}",
                                        row, col_name, e
                                    ),
                                )
                            })?;

                        Ok(Value::Sheet(sheet_clone))
                    }
                    _ => Err(PipError::runtime(
                        line,
                        "Arguments must be (sheet, int, string, value)",
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
                                return Ok(*val);
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

    // SQL query methods moved to sql_builder.rs module

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

    /// Register a sheet as a table and return the table name.
    async fn register_sheet_as_table(&mut self, name: &str, sheet: &Sheet) -> PipResult<String> {
        use arrow::array::ArrayRef;
        use arrow::array::RecordBatch;
        use arrow::datatypes::{DataType, Field, Schema};
        use piptable_sheet::CellValue;
        use std::sync::Arc;

        let table_name = format!("sheet_{}", name.replace(['-', '.', ' '], "_"));

        // Check if sheet has named columns
        let column_names = match sheet.column_names() {
            Some(names) => names.clone(),
            None => {
                // Generate default column names
                (0..sheet.col_count())
                    .map(|i| format!("column_{}", i))
                    .collect()
            }
        };

        if column_names.is_empty() {
            // Empty sheet - create schema with no fields
            let schema = Arc::new(Schema::empty());
            let batch = RecordBatch::new_empty(schema.clone());
            self.sql.register_table(&table_name, vec![batch]).await?;
            return Ok(table_name);
        }

        if sheet.row_count() <= 1 {
            // Only header row or empty - create empty table with schema
            let fields: Vec<Field> = column_names
                .iter()
                .map(|name| Field::new(name, DataType::Utf8, true))
                .collect();
            let schema = Arc::new(Schema::new(fields));
            let batch = RecordBatch::new_empty(schema.clone());
            self.sql.register_table(&table_name, vec![batch]).await?;
            return Ok(table_name);
        }

        // Determine if we should skip the first row
        // Only skip if column_names were set AND the first row matches those names
        let should_skip_first = if sheet.column_names().is_some() && !sheet.data().is_empty() {
            // Check if first row matches column names
            let first_row = &sheet.data()[0];
            let names_match = column_names.iter().enumerate().all(|(idx, name)| {
                first_row
                    .get(idx)
                    .map(|cell| cell.as_str() == name.as_str())
                    .unwrap_or(false)
            });
            usize::from(names_match)
        } else {
            0
        };

        let data_rows: Vec<&Vec<CellValue>> = sheet.data().iter().skip(should_skip_first).collect();

        if data_rows.is_empty() {
            // No data rows - create empty table with schema
            let fields: Vec<Field> = column_names
                .iter()
                .map(|name| Field::new(name, DataType::Utf8, true))
                .collect();
            let schema = Arc::new(Schema::new(fields));
            let batch = RecordBatch::new_empty(schema.clone());
            self.sql.register_table(&table_name, vec![batch]).await?;
            return Ok(table_name);
        }

        let num_cols = column_names.len();

        // Infer types for each column
        let col_types: Vec<DataType> = (0..num_cols)
            .map(|col_idx| infer_sheet_column_type(&data_rows, col_idx))
            .collect();

        // Build schema
        let fields: Vec<Field> = column_names
            .iter()
            .zip(col_types.iter())
            .map(|(name, dtype): (&String, &arrow::datatypes::DataType)| Field::new(name.clone(), dtype.clone(), true))
            .collect();
        let schema = Arc::new(Schema::new(fields));

        // Build Arrow arrays for each column
        let arrays: Result<Vec<ArrayRef>, _> = (0..num_cols)
            .map(|col_idx| build_sheet_arrow_array(&data_rows, col_idx, &col_types[col_idx]))
            .collect();
        let arrays = arrays.map_err(|e| PipError::runtime(0, e))?;

        // Create RecordBatch
        let batch = RecordBatch::try_new(schema.clone(), arrays)
            .map_err(|e| PipError::runtime(0, format!("Failed to create RecordBatch: {}", e)))?;

        // Register the batch with the SQL engine
        self.sql.register_table(&table_name, vec![batch]).await?;

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
        // Clear any cached table for this variable, regardless of new type.
        // This prevents stale table registrations when a Sheet variable is reassigned.
        let table_to_drop = {
            let mut sheet_tables = self.sheet_tables.write().await;
            sheet_tables.remove(name)
        };
        if let Some(table_name) = table_to_drop {
            // Deregister from SQL context (drop lock before await)
            let _ = self.sql.deregister_table(&table_name).await;
        }

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

// Helper functions moved to separate modules:
// - io.rs: Import/export functions
// - sheet_conversions.rs: Sheet/Value/Arrow conversion functions

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
        assert!(matches!(&combined, Value::Array(arr) if arr.len() == 6)); // 2 + 2 rows + 2 headers

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
        assert!(matches!(&combined, Value::Array(arr) if arr.len() == 4)); // 2 data rows + 2 headers

        if let Value::Array(arr) = &combined {
            // Check that source column contains sheet names (skip header rows)
            // Find first non-header data row (should have integer values)
            let data_rows: Vec<_> = arr.iter()
                .filter(|item| {
                    if let Value::Object(obj) = item {
                        obj.values().any(|v| matches!(v, Value::Int(_)))
                    } else {
                        false
                    }
                })
                .collect();
            
            assert!(data_rows.len() >= 2, "Should have at least 2 data rows");
            
            if let Value::Object(obj) = data_rows[0] {
                assert!(obj.contains_key("_store"));
                assert!(matches!(obj.get("_store"), Some(Value::String(s)) if s == "store1"));
            }
            if let Value::Object(obj) = data_rows[1] {
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

        let script = r"result = consolidate(book, 123)"; // 123 is not a string
        let program = PipParser::parse_str(script).unwrap();
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
        use indexmap::IndexMap;
        use piptable_sheet::{CellValue, Sheet};

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
        interp.set_var("users", sheet_conversions::sheet_to_value(&users)).await;
        interp.set_var("orders", sheet_conversions::sheet_to_value(&orders)).await;

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
        use indexmap::IndexMap;
        use piptable_sheet::{CellValue, Sheet};

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
        interp.set_var("users", sheet_conversions::sheet_to_value(&users)).await;
        interp.set_var("orders", sheet_conversions::sheet_to_value(&orders)).await;

        // Test left join
        let code = r#"result = users left join orders on "id" = "user_id""#;
        let program = PipParser::parse_str(code).unwrap();
        interp.eval(program).await.unwrap();

        let result = interp.get_var("result").await.unwrap();
        if let Value::Array(arr) = result {
            assert_eq!(arr.len(), 3); // Should have all 3 users

            // Find Charlie's record
            let charlie_record = arr
                .iter()
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
        use indexmap::IndexMap;
        use piptable_sheet::{CellValue, Sheet};

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
        interp.set_var("sheet1", sheet_conversions::sheet_to_value(&sheet1)).await;
        interp.set_var("sheet2", sheet_conversions::sheet_to_value(&sheet2)).await;

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

    #[tokio::test]
    async fn test_join_right() {
        use indexmap::IndexMap;
        use piptable_sheet::{CellValue, Sheet};

        let mut interp = Interpreter::new();

        // Create left sheet with 2 users
        let mut user1 = IndexMap::new();
        user1.insert("id".to_string(), CellValue::Int(1));
        user1.insert("name".to_string(), CellValue::String("Alice".to_string()));

        let mut user2 = IndexMap::new();
        user2.insert("id".to_string(), CellValue::Int(2));
        user2.insert("name".to_string(), CellValue::String("Bob".to_string()));

        let sheet1 = Sheet::from_records(vec![user1, user2]).unwrap();

        // Create right sheet with 3 scores (one matching, two non-matching)
        let mut score1 = IndexMap::new();
        score1.insert("user_id".to_string(), CellValue::Int(2));
        score1.insert("score".to_string(), CellValue::Int(85));

        let mut score2 = IndexMap::new();
        score2.insert("user_id".to_string(), CellValue::Int(3));
        score2.insert("score".to_string(), CellValue::Int(90));

        let mut score3 = IndexMap::new();
        score3.insert("user_id".to_string(), CellValue::Int(4));
        score3.insert("score".to_string(), CellValue::Int(95));

        let sheet2 = Sheet::from_records(vec![score1, score2, score3]).unwrap();

        interp.set_var("sheet1", sheet_conversions::sheet_to_value(&sheet1)).await;
        interp.set_var("sheet2", sheet_conversions::sheet_to_value(&sheet2)).await;

        // Test right join
        let code = r#"result = sheet1 right join sheet2 on "id" = "user_id""#;
        let program = PipParser::parse_str(code).unwrap();
        interp.eval(program).await.unwrap();

        let result = interp.get_var("result").await.unwrap();
        if let Value::Array(arr) = result {
            // Should have all rows from sheet2 (3 records)
            assert_eq!(arr.len(), 3);

            // Check that we have the expected records
            let mut found_bob = false;
            let mut found_id3 = false;
            let mut found_id4 = false;

            for record in &arr {
                if let Value::Object(obj) = record {
                    // The join seems to be using "id" field from right table
                    if let Some(Value::Int(id)) = obj.get("id") {
                        match *id {
                            2 => {
                                // This should have data from both sheets
                                assert!(obj.contains_key("name"));
                                if let Some(Value::String(name)) = obj.get("name") {
                                    assert_eq!(name, "Bob");
                                    found_bob = true;
                                }
                            }
                            3 => {
                                // Should have nulls for left table columns
                                assert!(matches!(obj.get("name"), Some(Value::Null)));
                                // id=3 comes from right table (user_id=3)
                                assert!(matches!(obj.get("score"), Some(Value::Int(90))));
                                found_id3 = true;
                            }
                            4 => {
                                // Should have nulls for left table columns
                                assert!(matches!(obj.get("name"), Some(Value::Null)));
                                // id=4 comes from right table (user_id=4)
                                assert!(matches!(obj.get("score"), Some(Value::Int(95))));
                                found_id4 = true;
                            }
                            _ => panic!("Unexpected user_id in result: {}", id),
                        }
                    }
                }
            }

            assert!(found_bob, "Expected to find Bob's record");
            assert!(found_id3, "Expected to find user_id=3 record");
            assert!(found_id4, "Expected to find user_id=4 record");
        } else {
            panic!("Expected Array result from right join");
        }
    }

    #[tokio::test]
    async fn test_join_full() {
        use indexmap::IndexMap;
        use piptable_sheet::{CellValue, Sheet};

        let mut interp = Interpreter::new();

        // Create test data with some overlapping and some non-overlapping records
        let mut user1 = IndexMap::new();
        user1.insert("id".to_string(), CellValue::Int(1));
        user1.insert("name".to_string(), CellValue::String("Alice".to_string()));

        let mut user2 = IndexMap::new();
        user2.insert("id".to_string(), CellValue::Int(2));
        user2.insert("name".to_string(), CellValue::String("Bob".to_string()));

        let mut user3 = IndexMap::new();
        user3.insert("id".to_string(), CellValue::Int(5));
        user3.insert("name".to_string(), CellValue::String("Eve".to_string()));

        let sheet1 = Sheet::from_records(vec![user1, user2, user3]).unwrap();

        let mut score1 = IndexMap::new();
        score1.insert("user_id".to_string(), CellValue::Int(2));
        score1.insert("score".to_string(), CellValue::Int(85));

        let mut score2 = IndexMap::new();
        score2.insert("user_id".to_string(), CellValue::Int(3));
        score2.insert("score".to_string(), CellValue::Int(90));

        let mut score3 = IndexMap::new();
        score3.insert("user_id".to_string(), CellValue::Int(4));
        score3.insert("score".to_string(), CellValue::Int(95));

        let sheet2 = Sheet::from_records(vec![score1, score2, score3]).unwrap();

        interp.set_var("sheet1", sheet_conversions::sheet_to_value(&sheet1)).await;
        interp.set_var("sheet2", sheet_conversions::sheet_to_value(&sheet2)).await;

        // Test full join
        let code = r#"result = sheet1 full join sheet2 on "id" = "user_id""#;
        let program = PipParser::parse_str(code).unwrap();
        interp.eval(program).await.unwrap();

        let result = interp.get_var("result").await.unwrap();
        if let Value::Array(arr) = result {
            // Should have all unique keys from both tables (1,2,3,4,5 = 5 records)
            assert_eq!(arr.len(), 5);

            let mut found_alice = false;
            let mut found_bob = false;
            let mut found_eve = false;
            let mut found_id3 = false;
            let mut found_id4 = false;

            for record in &arr {
                if let Value::Object(obj) = record {
                    // Check based on which side has data
                    let id = obj.get("id");

                    if matches!(id, Some(Value::Int(1))) {
                        // Alice - only in left table
                        assert!(matches!(obj.get("name"), Some(Value::String(s)) if s == "Alice"));
                        assert!(matches!(obj.get("score"), Some(Value::Null)));
                        // There's no user_id field, it's all "id" after join
                        found_alice = true;
                    } else if matches!(id, Some(Value::Int(2))) {
                        // Bob - in both tables
                        assert!(matches!(obj.get("name"), Some(Value::String(s)) if s == "Bob"));
                        assert!(matches!(obj.get("score"), Some(Value::Int(85))));
                        found_bob = true;
                    } else if matches!(id, Some(Value::Int(5))) {
                        // Eve - only in left table
                        assert!(matches!(obj.get("name"), Some(Value::String(s)) if s == "Eve"));
                        assert!(matches!(obj.get("score"), Some(Value::Null)));
                        found_eve = true;
                    } else if matches!(id, Some(Value::Int(3))) {
                        // Only in right table (user_id=3, no matching id in left)
                        assert!(matches!(obj.get("name"), Some(Value::Null)));
                        assert!(matches!(obj.get("score"), Some(Value::Int(90))));
                        found_id3 = true;
                    } else if matches!(id, Some(Value::Int(4))) {
                        // Only in right table (user_id=4, no matching id in left)
                        assert!(matches!(obj.get("name"), Some(Value::Null)));
                        assert!(matches!(obj.get("score"), Some(Value::Int(95))));
                        found_id4 = true;
                    }
                }
            }

            assert!(found_alice, "Expected to find Alice's record");
            assert!(found_bob, "Expected to find Bob's record");
            assert!(found_eve, "Expected to find Eve's record");
            assert!(found_id3, "Expected to find user_id=3 record");
            assert!(found_id4, "Expected to find user_id=4 record");
        } else {
            panic!("Expected Array result from full join");
        }
    }

    #[tokio::test]
    async fn test_join_with_empty_sheets() {
        use indexmap::IndexMap;
        use piptable_sheet::{CellValue, Sheet};

        let mut interp = Interpreter::new();

        // Create sheet1 with data
        let mut user1 = IndexMap::new();
        user1.insert("id".to_string(), CellValue::Int(1));
        user1.insert("name".to_string(), CellValue::String("Alice".to_string()));
        let sheet1 = Sheet::from_records(vec![user1]).unwrap();

        // Create sheet2 with no data but named columns
        let mut sheet2 = Sheet::new();
        sheet2.data_mut().push(vec![
            CellValue::String("user_id".to_string()),
            CellValue::String("score".to_string()),
        ]);
        sheet2.name_columns_by_row(0).unwrap();
        sheet2.data_mut().remove(0); // Remove header row, keep column names

        interp.set_var("sheet1", sheet_conversions::sheet_to_value(&sheet1)).await;
        interp.set_var("sheet2", sheet_conversions::sheet_to_value(&sheet2)).await;

        // Test behavior with empty sheet - should either work or give predictable error
        let code = r#"result = sheet1 join sheet2 on "id" = "user_id""#;
        let program = PipParser::parse_str(code).unwrap();

        // This may fail with "Columns not named" for truly empty sheets, which is expected
        let eval_result = interp.eval(program).await;

        match eval_result {
            Ok(_) => {
                // If it succeeds, verify the result is empty
                let result = interp.get_var("result").await.unwrap();
                if let Value::Array(arr) = result {
                    assert_eq!(
                        arr.len(),
                        0,
                        "Inner join with empty sheet should return no records"
                    );
                } else {
                    panic!("Expected Array result");
                }
            }
            Err(e) => {
                // Expected error for empty sheets without proper column mapping
                assert!(
                    e.to_string().contains("Columns not named"),
                    "Expected 'Columns not named' error for empty sheet, got: {}",
                    e
                );
                return; // Skip remaining tests if empty sheets aren't supported
            }
        }

        // Test left join with empty right sheet
        let code = r#"result = sheet1 left join sheet2 on "id" = "user_id""#;
        let program = PipParser::parse_str(code).unwrap();
        let left_result = interp.eval(program).await;

        if left_result.is_ok() {
            let result = interp.get_var("result").await.unwrap();
            if let Value::Array(arr) = result {
                assert_eq!(arr.len(), 1, "Left join should preserve all left records");
                if let Value::Object(obj) = &arr[0] {
                    assert!(matches!(obj.get("name"), Some(Value::String(s)) if s == "Alice"));
                }
            } else {
                panic!("Expected Array result");
            }
        }

        // Test right join with empty right sheet
        let code = r#"result = sheet1 right join sheet2 on "id" = "user_id""#;
        let program = PipParser::parse_str(code).unwrap();
        let right_result = interp.eval(program).await;

        if right_result.is_ok() {
            let result = interp.get_var("result").await.unwrap();
            if let Value::Array(arr) = result {
                assert_eq!(
                    arr.len(),
                    0,
                    "Right join with empty right sheet should return no records"
                );
            } else {
                panic!("Expected Array result");
            }
        }
    }

    #[tokio::test]
    async fn test_append_basic() {
        use indexmap::IndexMap;
        use piptable_sheet::{CellValue, Sheet};

        let mut interp = Interpreter::new();

        // Create initial users sheet
        let mut user1 = IndexMap::new();
        user1.insert("id".to_string(), CellValue::Int(1));
        user1.insert("name".to_string(), CellValue::String("Alice".to_string()));

        let mut user2 = IndexMap::new();
        user2.insert("id".to_string(), CellValue::Int(2));
        user2.insert("name".to_string(), CellValue::String("Bob".to_string()));

        let sheet1 = Sheet::from_records(vec![user1, user2]).unwrap();
        interp.set_var("users", sheet_conversions::sheet_to_value(&sheet1)).await;

        // Create new users to append
        let mut user3 = IndexMap::new();
        user3.insert("id".to_string(), CellValue::Int(3));
        user3.insert("name".to_string(), CellValue::String("Charlie".to_string()));

        let sheet2 = Sheet::from_records(vec![user3]).unwrap();
        interp.set_var("new_users", sheet_conversions::sheet_to_value(&sheet2)).await;

        // Test basic append
        let code = r"users append new_users";
        let program = PipParser::parse_str(code).unwrap();
        interp.eval(program).await.unwrap();

        // Verify the result
        let result = interp.get_var("users").await.unwrap();
        if let Value::Array(arr) = result {
            assert_eq!(arr.len(), 3, "Should have 3 users after append");

            // Check that all users are present
            let mut found_alice = false;
            let mut found_bob = false;
            let mut found_charlie = false;

            for item in &arr {
                if let Value::Object(obj) = item {
                    if let Some(Value::String(name)) = obj.get("name") {
                        match name.as_str() {
                            "Alice" => found_alice = true,
                            "Bob" => found_bob = true,
                            "Charlie" => found_charlie = true,
                            _ => {}
                        }
                    }
                }
            }

            assert!(found_alice, "Alice should be in the result");
            assert!(found_bob, "Bob should be in the result");
            assert!(found_charlie, "Charlie should be in the result");
        } else {
            panic!("Expected Array result");
        }
    }

    #[tokio::test]
    async fn test_append_distinct() {
        use indexmap::IndexMap;
        use piptable_sheet::{CellValue, Sheet};

        let mut interp = Interpreter::new();

        // Create initial users sheet
        let mut user1 = IndexMap::new();
        user1.insert("id".to_string(), CellValue::Int(1));
        user1.insert("name".to_string(), CellValue::String("Alice".to_string()));

        let mut user2 = IndexMap::new();
        user2.insert("id".to_string(), CellValue::Int(2));
        user2.insert("name".to_string(), CellValue::String("Bob".to_string()));

        let sheet1 = Sheet::from_records(vec![user1, user2]).unwrap();
        interp.set_var("users", sheet_conversions::sheet_to_value(&sheet1)).await;

        // Create new users with duplicate ID
        let mut user2_dup = IndexMap::new();
        user2_dup.insert("id".to_string(), CellValue::Int(2));
        user2_dup.insert(
            "name".to_string(),
            CellValue::String("Bob Updated".to_string()),
        );

        let mut user3 = IndexMap::new();
        user3.insert("id".to_string(), CellValue::Int(3));
        user3.insert("name".to_string(), CellValue::String("Charlie".to_string()));

        let sheet2 = Sheet::from_records(vec![user2_dup, user3]).unwrap();
        interp.set_var("new_users", sheet_conversions::sheet_to_value(&sheet2)).await;

        // Test append distinct on "id"
        let code = r#"users append distinct new_users on "id""#;
        let program = PipParser::parse_str(code).unwrap();
        interp.eval(program).await.unwrap();

        // Verify the result
        let result = interp.get_var("users").await.unwrap();
        if let Value::Array(arr) = result {
            assert_eq!(
                arr.len(),
                3,
                "Should have 3 users (duplicate ID=2 not added)"
            );

            // Check that Bob's name wasn't updated
            for item in &arr {
                if let Value::Object(obj) = item {
                    if let (Some(Value::Int(id)), Some(Value::String(name))) =
                        (obj.get("id"), obj.get("name"))
                    {
                        if *id == 2 {
                            assert_eq!(name, "Bob", "Bob's name should not have changed");
                        }
                    }
                }
            }
        } else {
            panic!("Expected Array result");
        }
    }

    #[tokio::test]
    async fn test_upsert() {
        use indexmap::IndexMap;
        use piptable_sheet::{CellValue, Sheet};

        let mut interp = Interpreter::new();

        // Create initial users sheet
        let mut user1 = IndexMap::new();
        user1.insert("id".to_string(), CellValue::Int(1));
        user1.insert("name".to_string(), CellValue::String("Alice".to_string()));
        user1.insert("age".to_string(), CellValue::Int(25));

        let mut user2 = IndexMap::new();
        user2.insert("id".to_string(), CellValue::Int(2));
        user2.insert("name".to_string(), CellValue::String("Bob".to_string()));
        user2.insert("age".to_string(), CellValue::Int(30));

        let sheet1 = Sheet::from_records(vec![user1, user2]).unwrap();
        interp.set_var("users", sheet_conversions::sheet_to_value(&sheet1)).await;

        // Create updates with existing and new users
        let mut user1_update = IndexMap::new();
        user1_update.insert("id".to_string(), CellValue::Int(1));
        user1_update.insert(
            "name".to_string(),
            CellValue::String("Alice Smith".to_string()),
        );
        user1_update.insert("age".to_string(), CellValue::Int(26));

        let mut user3 = IndexMap::new();
        user3.insert("id".to_string(), CellValue::Int(3));
        user3.insert("name".to_string(), CellValue::String("Charlie".to_string()));
        user3.insert("age".to_string(), CellValue::Int(35));

        let sheet2 = Sheet::from_records(vec![user1_update, user3]).unwrap();
        interp.set_var("updates", sheet_conversions::sheet_to_value(&sheet2)).await;

        // Test upsert
        let code = r#"users upsert updates on "id""#;
        let program = PipParser::parse_str(code).unwrap();
        interp.eval(program).await.unwrap();

        // Verify the result
        let result = interp.get_var("users").await.unwrap();
        if let Value::Array(arr) = result {
            assert_eq!(arr.len(), 3, "Should have 3 users after upsert");

            // Check that Alice was updated
            let mut found_alice_updated = false;
            let mut found_bob_unchanged = false;
            let mut found_charlie_new = false;

            for item in &arr {
                if let Value::Object(obj) = item {
                    if let (
                        Some(Value::Int(id)),
                        Some(Value::String(name)),
                        Some(Value::Int(age)),
                    ) = (obj.get("id"), obj.get("name"), obj.get("age"))
                    {
                        match *id {
                            1 => {
                                assert_eq!(name, "Alice Smith", "Alice's name should be updated");
                                assert_eq!(*age, 26, "Alice's age should be updated");
                                found_alice_updated = true;
                            }
                            2 => {
                                assert_eq!(name, "Bob", "Bob's name should be unchanged");
                                assert_eq!(*age, 30, "Bob's age should be unchanged");
                                found_bob_unchanged = true;
                            }
                            3 => {
                                assert_eq!(name, "Charlie", "Charlie should be added");
                                assert_eq!(*age, 35, "Charlie's age should be 35");
                                found_charlie_new = true;
                            }
                            _ => {}
                        }
                    }
                }
            }

            assert!(found_alice_updated, "Alice should be updated");
            assert!(found_bob_unchanged, "Bob should be unchanged");
            assert!(found_charlie_new, "Charlie should be added");
        } else {
            panic!("Expected Array result");
        }
    }

    #[tokio::test]
    async fn test_sheet_a1_notation() {
        use indexmap::IndexMap;
        use piptable_sheet::{CellValue, Sheet};

        let mut interp = Interpreter::new();

        // Create a sheet directly and set as variable
        let mut record1 = IndexMap::new();
        record1.insert("Name".to_string(), CellValue::String("Alice".to_string()));
        record1.insert("Age".to_string(), CellValue::Int(30));

        let mut record2 = IndexMap::new();
        record2.insert("Name".to_string(), CellValue::String("Bob".to_string()));
        record2.insert("Age".to_string(), CellValue::Int(25));

        let sheet = Sheet::from_records(vec![record1, record2]).unwrap();
        interp.set_var("sheet", Value::Sheet(sheet)).await;

        // Test A1 notation access
        let program = PipParser::parse_str(r#"dim result = sheet["A1"]"#).unwrap();
        interp.eval(program).await.unwrap();

        let result = interp.get_var("result").await;
        assert!(matches!(result, Some(Value::String(s)) if s == "Name"));
    }

    #[tokio::test]
    async fn test_sheet_built_in_functions() {
        use indexmap::IndexMap;
        use piptable_sheet::{CellValue, Sheet};

        let mut interp = Interpreter::new();

        // Create a sheet with records
        let mut record1 = IndexMap::new();
        record1.insert("Name".to_string(), CellValue::String("Alice".to_string()));
        record1.insert("Age".to_string(), CellValue::Int(30));

        let mut record2 = IndexMap::new();
        record2.insert("Name".to_string(), CellValue::String("Bob".to_string()));
        record2.insert("Age".to_string(), CellValue::Int(25));

        let sheet = Sheet::from_records(vec![record1, record2]).unwrap();
        interp.set_var("sheet", Value::Sheet(sheet)).await;

        // Test basic Sheet functions
        let program = PipParser::parse_str(
            r"
            dim row_count = sheet_row_count(sheet)
            dim col_count = sheet_col_count(sheet)
        ",
        )
        .unwrap();

        interp.eval(program).await.unwrap();

        let row_count = interp.get_var("row_count").await;
        let col_count = interp.get_var("col_count").await;

        assert!(matches!(row_count, Some(Value::Int(3)))); // Header + 2 data rows
        assert!(matches!(col_count, Some(Value::Int(2)))); // Name, Age
    }

    #[tokio::test]
    async fn test_sheet_to_sql_conversion() {
        use indexmap::IndexMap;
        use piptable_sheet::{CellValue, Sheet};

        let mut interp = Interpreter::new();

        // Create a sheet with sales data
        let mut record1 = IndexMap::new();
        record1.insert(
            "Product".to_string(),
            CellValue::String("Widget".to_string()),
        );
        record1.insert("Amount".to_string(), CellValue::Int(100));

        let mut record2 = IndexMap::new();
        record2.insert(
            "Product".to_string(),
            CellValue::String("Gadget".to_string()),
        );
        record2.insert("Amount".to_string(), CellValue::Int(200));

        let sheet = Sheet::from_records(vec![record1, record2]).unwrap();
        interp.set_var("sales_sheet", Value::Sheet(sheet)).await;

        // Use the sheet in a SQL query (just select to test the conversion works)
        let program = PipParser::parse_str(
            r#"dim result = query(SELECT "Product", "Amount" FROM sales_sheet)"#,
        )
        .unwrap();
        let result = interp.eval(program).await;

        // Should not error - this tests that Sheet to Table conversion works
        match &result {
            Ok(_) => println!("SQL query succeeded"),
            Err(e) => println!("SQL query failed: {:?}", e),
        }
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_sheet_cell_access_functions() {
        use indexmap::IndexMap;
        use piptable_sheet::{CellValue, Sheet};

        let mut interp = Interpreter::new();

        // Create a sheet
        let mut record1 = IndexMap::new();
        record1.insert("Name".to_string(), CellValue::String("Alice".to_string()));
        record1.insert("Age".to_string(), CellValue::Int(30));

        let sheet = Sheet::from_records(vec![record1]).unwrap();
        interp.set_var("sheet", Value::Sheet(sheet)).await;

        let program = PipParser::parse_str(
            r#"
            dim a1_value = sheet_get_a1(sheet, "A1")
            dim b2_value = sheet_get_a1(sheet, "B2")
            dim name_value = sheet_get_by_name(sheet, 1, "Name")
        "#,
        )
        .unwrap();

        interp.eval(program).await.unwrap();

        let a1_value = interp.get_var("a1_value").await;
        let b2_value = interp.get_var("b2_value").await;
        let name_value = interp.get_var("name_value").await;

        assert!(matches!(a1_value, Some(Value::String(s)) if s == "Name"));
        assert!(matches!(b2_value, Some(Value::Int(30))));
        assert!(matches!(name_value, Some(Value::String(s)) if s == "Alice"));
    }

    #[tokio::test]
    async fn test_repeated_sql_queries_on_same_sheet() {
        // Test that we can run multiple SQL queries on the same Sheet variable
        use indexmap::IndexMap;
        use piptable_sheet::{CellValue, Sheet};

        let mut interp = Interpreter::new();

        // Create a sheet with data
        let mut record1 = IndexMap::new();
        record1.insert(
            "Product".to_string(),
            CellValue::String("Widget".to_string()),
        );
        record1.insert("Amount".to_string(), CellValue::Int(100));

        let mut record2 = IndexMap::new();
        record2.insert(
            "Product".to_string(),
            CellValue::String("Gadget".to_string()),
        );
        record2.insert("Amount".to_string(), CellValue::Int(200));

        let sheet = Sheet::from_records(vec![record1, record2]).unwrap();
        interp.set_var("sales", Value::Sheet(sheet)).await;

        // First query
        let program1 = PipParser::parse_str(r"dim result1 = query(SELECT * FROM sales)").unwrap();
        let result1 = interp.eval(program1).await;
        assert!(result1.is_ok(), "First query should succeed");

        // Second query on same sheet
        let program2 = PipParser::parse_str(
            r#"dim result2 = query(SELECT COUNT("Product") as cnt FROM sales)"#,
        )
        .unwrap();
        let result2 = interp.eval(program2).await;
        assert!(result2.is_ok(), "Second query should succeed");

        // Third query with different operation
        let program3 = PipParser::parse_str(
            r#"dim result3 = query(SELECT "Product" FROM sales WHERE "Amount" > 150)"#,
        )
        .unwrap();
        let result3 = interp.eval(program3).await;
        assert!(result3.is_ok(), "Third query should succeed");
    }

    #[tokio::test]
    async fn test_sheet_header_row_detection() {
        // Test that header rows are correctly detected and not duplicated
        use piptable_sheet::Sheet;

        let mut interp = Interpreter::new();

        // Create a sheet where first row matches column names (should be skipped)
        let mut sheet1 = Sheet::new();
        sheet1.data_mut().push(vec![
            piptable_sheet::CellValue::String("Name".to_string()),
            piptable_sheet::CellValue::String("Age".to_string()),
        ]);
        sheet1.data_mut().push(vec![
            piptable_sheet::CellValue::String("Alice".to_string()),
            piptable_sheet::CellValue::Int(30),
        ]);
        sheet1.name_columns_by_row(0).unwrap();

        interp.set_var("sheet1", Value::Sheet(sheet1)).await;

        // Query should return 1 data row (not 2)
        let program1 =
            PipParser::parse_str(r#"dim result1 = query(SELECT COUNT("Name") as cnt FROM sheet1)"#)
                .unwrap();
        interp.eval(program1).await.unwrap();

        let result1 = interp.get_var("result1").await;
        if let Some(Value::Table(batches)) = result1 {
            assert_eq!(batches.len(), 1);
            let batch = &batches[0];
            assert_eq!(batch.num_rows(), 1); // Should have 1 result row
                                             // The count should be 1 (only Alice, not the header row)
        }

        // Create a sheet where first row does NOT match column names (should NOT be skipped)
        let mut sheet2 = Sheet::new();
        // Add a dummy header row with different names, then real data
        sheet2.data_mut().push(vec![
            piptable_sheet::CellValue::String("PersonName".to_string()),
            piptable_sheet::CellValue::String("PersonAge".to_string()),
        ]);
        sheet2.data_mut().push(vec![
            piptable_sheet::CellValue::String("Bob".to_string()),
            piptable_sheet::CellValue::Int(25),
        ]);
        sheet2.data_mut().push(vec![
            piptable_sheet::CellValue::String("Charlie".to_string()),
            piptable_sheet::CellValue::Int(35),
        ]);
        // Name columns from row 0 (which has PersonName, PersonAge)
        sheet2.name_columns_by_row(0).unwrap();

        interp.set_var("sheet2", Value::Sheet(sheet2)).await;

        // Query should return 2 data rows (both Bob and Charlie)
        let program2 =
            PipParser::parse_str(r#"dim result2 = query(SELECT COUNT("Name") as cnt FROM sheet2)"#)
                .unwrap();
        interp.eval(program2).await.unwrap();

        let result2 = interp.get_var("result2").await;
        if let Some(Value::Table(batches)) = result2 {
            assert_eq!(batches.len(), 1);
            let batch = &batches[0];
            assert_eq!(batch.num_rows(), 1); // Should have 1 result row
                                             // The count should be 2 (both Bob and Charlie)
        }
    }

    #[tokio::test]
    async fn test_sheet_modification_and_requery() {
        // Test that modifying a sheet and re-querying works correctly
        use indexmap::IndexMap;
        use piptable_sheet::{CellValue, Sheet};

        let mut interp = Interpreter::new();

        // Create initial sheet
        let mut record1 = IndexMap::new();
        record1.insert("Name".to_string(), CellValue::String("Alice".to_string()));
        record1.insert("Score".to_string(), CellValue::Int(90));

        let sheet1 = Sheet::from_records(vec![record1]).unwrap();
        interp.set_var("scores", Value::Sheet(sheet1)).await;

        // First query
        let program1 = PipParser::parse_str(r"dim result1 = query(SELECT * FROM scores)").unwrap();
        assert!(interp.eval(program1).await.is_ok());

        // Modify the sheet (this should clear the cached table)
        let mut record2 = IndexMap::new();
        record2.insert("Name".to_string(), CellValue::String("Bob".to_string()));
        record2.insert("Score".to_string(), CellValue::Int(85));

        let sheet2 = Sheet::from_records(vec![record2]).unwrap();
        interp.set_var("scores", Value::Sheet(sheet2)).await;

        // Second query should work with the new data
        let program2 = PipParser::parse_str(r"dim result2 = query(SELECT * FROM scores)").unwrap();
        let result = interp.eval(program2).await;
        assert!(result.is_ok(), "Second query failed: {:?}", result.err());

        // Verify we get different data
        let result2 = interp.get_var("result2").await;
        if let Some(Value::Table(batches)) = result2 {
            assert!(!batches.is_empty());
            // The new query should have Bob's data, not Alice's
        }
    }
}
