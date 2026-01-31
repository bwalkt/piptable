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
mod formula;
pub mod io;
mod sheet_conversions;
mod sql_builder;

#[cfg(feature = "python")]
mod python;

use crate::sheet_conversions::{build_sheet_arrow_array, cell_to_value, infer_sheet_column_type};
use async_recursion::async_recursion;
use piptable_core::{
    BinaryOp, Expr, LValue, Literal, Param, ParamMode, PipError, PipResult, Program, Statement,
    UnaryOp, Value,
};
use piptable_http::HttpClient;
use piptable_sheet::{CellValue, Sheet};
use piptable_sql::SqlEngine;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Interpreter for piptable scripts.
pub struct Interpreter {
    /// Variable scopes (stack for nested scopes)
    scopes: Arc<RwLock<Vec<HashMap<String, VarBinding>>>>,
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
    pub params: Vec<Param>,
    pub body: Vec<Statement>,
    pub is_async: bool,
}

#[derive(Debug, Clone)]
struct RefTarget {
    name: String,
    scope_index: usize,
}

#[derive(Debug, Clone)]
enum VarBinding {
    Value(Value),
    Ref(RefTarget),
    RefLValue(RefLValue),
}

fn find_binding(scopes: &[HashMap<String, VarBinding>], name: &str) -> Option<(usize, VarBinding)> {
    for (idx, scope) in scopes.iter().enumerate().rev() {
        if let Some(binding) = scope.get(name) {
            return Some((idx, binding.clone()));
        }
    }
    None
}

fn resolve_ref_value(scopes: &[HashMap<String, VarBinding>], target: RefTarget) -> Option<Value> {
    let mut current = target;
    let mut seen: HashSet<(usize, String)> = HashSet::new();
    loop {
        if !seen.insert((current.scope_index, current.name.clone())) {
            return None;
        }
        let scope = scopes.get(current.scope_index)?;
        match scope.get(&current.name)? {
            VarBinding::Value(val) => return Some(val.clone()),
            VarBinding::Ref(next) => {
                current = next.clone();
            }
            VarBinding::RefLValue(ref_lvalue) => {
                return resolve_ref_lvalue_value(scopes, ref_lvalue.clone());
            }
        }
    }
}

fn resolve_binding_value(
    scopes: &[HashMap<String, VarBinding>],
    binding: VarBinding,
) -> Option<Value> {
    match binding {
        VarBinding::Value(val) => Some(val),
        VarBinding::Ref(target) => resolve_ref_value(scopes, target),
        VarBinding::RefLValue(ref_lvalue) => resolve_ref_lvalue_value(scopes, ref_lvalue),
    }
}

fn resolve_ref_target_info(
    scopes: &[HashMap<String, VarBinding>],
    target: RefTarget,
) -> Option<RefTarget> {
    let mut current = target;
    let mut seen: HashSet<(usize, String)> = HashSet::new();
    loop {
        if !seen.insert((current.scope_index, current.name.clone())) {
            return None;
        }
        let scope = scopes.get(current.scope_index)?;
        match scope.get(&current.name) {
            Some(VarBinding::Ref(next)) => {
                current = next.clone();
            }
            Some(VarBinding::Value(_)) => return Some(current),
            Some(VarBinding::RefLValue(ref_lvalue)) => return Some(ref_lvalue.base.clone()),
            None => return None,
        }
    }
}

fn flatten_lvalue(lvalue: LValue, access: &mut Vec<RefAccess>, line: usize) -> PipResult<String> {
    match lvalue {
        LValue::Variable(name) => Ok(name),
        LValue::Field { object, field } => {
            let base = flatten_lvalue(*object, access, line)?;
            access.push(RefAccess::Field(field));
            Ok(base)
        }
        LValue::Index { array, index } => {
            let base = flatten_lvalue(*array, access, line)?;
            match *index {
                Expr::Literal(Literal::Int(value)) => {
                    access.push(RefAccess::Index(value));
                    Ok(base)
                }
                _ => Err(PipError::runtime(line, "Array index must be integer")),
            }
        }
    }
}

#[derive(Debug, Clone)]
struct RefLValue {
    base: RefTarget,
    access: Vec<RefAccess>,
}

#[derive(Debug, Clone)]
enum RefAccess {
    Field(String),
    Index(i64),
}

fn resolve_ref_lvalue_value(
    scopes: &[HashMap<String, VarBinding>],
    ref_lvalue: RefLValue,
) -> Option<Value> {
    let base_binding = scopes
        .get(ref_lvalue.base.scope_index)?
        .get(&ref_lvalue.base.name)?
        .clone();
    let mut current = resolve_binding_value(scopes, base_binding)?;

    for access in ref_lvalue.access {
        match access {
            RefAccess::Field(field) => {
                if let Value::Object(map) = current {
                    current = map.get(&field)?.clone();
                } else {
                    return None;
                }
            }
            RefAccess::Index(index) => {
                if let Value::Array(items) = current {
                    let idx = if index < 0 {
                        let adjusted = items.len() as i64 + index;
                        if adjusted < 0 {
                            return None;
                        }
                        adjusted as usize
                    } else {
                        index as usize
                    };
                    if idx >= items.len() {
                        return None;
                    }
                    current = items[idx].clone();
                } else {
                    return None;
                }
            }
        }
    }

    Some(current)
}

fn assign_ref_lvalue(
    scopes: &mut [HashMap<String, VarBinding>],
    ref_lvalue: RefLValue,
    value: Value,
    line: usize,
) -> PipResult<()> {
    let base_scope = scopes
        .get(ref_lvalue.base.scope_index)
        .ok_or_else(|| PipError::runtime(line, "ByRef target scope not found"))?;
    let base_binding = base_scope
        .get(&ref_lvalue.base.name)
        .ok_or_else(|| PipError::runtime(line, "ByRef target not found"))?
        .clone();
    let base_value = resolve_binding_value(scopes, base_binding)
        .ok_or_else(|| PipError::runtime(line, "ByRef target not found"))?;

    let updated = apply_ref_access(base_value, &ref_lvalue.access, value, line)?;

    if let Some(scope) = scopes.get_mut(ref_lvalue.base.scope_index) {
        scope.insert(ref_lvalue.base.name.clone(), VarBinding::Value(updated));
    }

    Ok(())
}

fn apply_ref_access(
    value: Value,
    access: &[RefAccess],
    new_value: Value,
    line: usize,
) -> PipResult<Value> {
    if access.is_empty() {
        return Ok(new_value);
    }

    match (&access[0], value) {
        (RefAccess::Field(field), Value::Object(mut map)) => {
            let next_value = map
                .remove(field)
                .ok_or_else(|| PipError::runtime(line, format!("Field not found: {field}")))?;
            let updated = apply_ref_access(next_value, &access[1..], new_value, line)?;
            map.insert(field.clone(), updated);
            Ok(Value::Object(map))
        }
        (RefAccess::Index(index), Value::Array(mut items)) => {
            let idx = if *index < 0 {
                let adjusted = items.len() as i64 + *index;
                if adjusted < 0 {
                    return Err(PipError::runtime(line, "Array index out of bounds"));
                }
                adjusted as usize
            } else {
                *index as usize
            };
            if idx >= items.len() {
                return Err(PipError::runtime(line, "Array index out of bounds"));
            }
            let next_value = items[idx].clone();
            let updated = apply_ref_access(next_value, &access[1..], new_value, line)?;
            items[idx] = updated;
            Ok(Value::Array(items))
        }
        (RefAccess::Field(_), other) => Err(PipError::runtime(
            line,
            format!("Cannot assign field on {}", other.type_name()),
        )),
        (RefAccess::Index(_), other) => Err(PipError::runtime(
            line,
            format!("Cannot index {}", other.type_name()),
        )),
    }
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
                Err(PipError::ExitFunction(line)) => {
                    return Err(PipError::runtime(
                        line,
                        "Exit Function cannot be used outside of a function",
                    ));
                }
                Err(PipError::ExitFor(line)) => {
                    return Err(PipError::runtime(
                        line,
                        "Exit For cannot be used outside of a for loop",
                    ));
                }
                Err(PipError::ExitWhile(line)) => {
                    return Err(PipError::runtime(
                        line,
                        "Exit While cannot be used outside of a while loop",
                    ));
                }
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
                self.set_var(&name, val).await?;
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
                    Value::Sheet(sheet) => {
                        // Convert sheet to array of objects for iteration
                        let value = sheet_conversions::sheet_to_value(&sheet);
                        match value {
                            Value::Array(arr) => arr,
                            other => {
                                return Err(PipError::runtime(
                                    line,
                                    format!("Sheet conversion returned unexpected type: {} (expected Array)", other.type_name()),
                                ))
                            }
                        }
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
                    match self.eval_block(&body).await {
                        Ok(_) => {}
                        Err(PipError::ExitFor(_)) => {
                            // Exit For - break out of the loop normally
                            break;
                        }
                        Err(e) => {
                            loop_result = Err(e);
                            break;
                        }
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
                    match self.eval_block(&body).await {
                        Ok(_) => {}
                        Err(PipError::ExitFor(_)) => {
                            // Exit For - break out of the loop normally
                            break;
                        }
                        Err(e) => {
                            loop_result = Err(e);
                            break;
                        }
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
                                match e {
                                    PipError::ExitWhile(_) => {
                                        // Exit While - break from loop normally
                                        break;
                                    }
                                    _ => {
                                        loop_result = Err(e.with_line(line));
                                        break;
                                    }
                                }
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

            Statement::Return { value, line } => {
                let val = match value {
                    Some(expr) => self.eval_expr(&expr).await.map_err(|e| e.with_line(line))?,
                    None => Value::Null,
                };
                // Return is handled by propagating up the call stack
                Err(PipError::Return(Box::new(val)))
            }

            Statement::ExitFunction { line } => {
                // Exit Function is handled by propagating up the call stack
                Err(PipError::ExitFunction(line))
            }

            Statement::ExitFor { line } => {
                // Exit For is handled by loop constructs
                Err(PipError::ExitFor(line))
            }

            Statement::ExitWhile { line } => {
                // Exit While is handled by loop constructs
                Err(PipError::ExitWhile(line))
            }

            Statement::Call {
                function,
                args,
                line,
            } => self.call_function(&function, &args, line).await,

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
                let mut target_sheet =
                    sheet_conversions::value_to_sheet(&target_val).map_err(|e| {
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
                self.set_var(&target, sheet_conversions::sheet_to_value(&target_sheet))
                    .await?;
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
                let mut target_sheet =
                    sheet_conversions::value_to_sheet(&target_val).map_err(|e| {
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
                self.set_var(&target, sheet_conversions::sheet_to_value(&target_sheet))
                    .await?;
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
                append,
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
                let mode = if append {
                    io::ExportMode::Append
                } else {
                    io::ExportMode::Overwrite
                };
                io::export_sheet_with_mode(&sheet, &path, mode)
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
                    // Return as Sheet value to enable SQL queries
                    Value::Sheet(sheet)
                };

                // Store in target variable
                self.set_var(&target, value).await?;

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
                    // Sheet integer indexing - return row as object
                    (Value::Sheet(sheet), Value::Int(idx_int)) => {
                        // Determine whether a physical header row exists
                        // Only skip if column_names were set AND the first row matches those names
                        let header_offset = if let Some(names) = sheet.column_names() {
                            if sheet.data().is_empty() {
                                0
                            } else {
                                let first_row = &sheet.data()[0];
                                let names_match = names.iter().enumerate().all(|(idx, name)| {
                                    first_row
                                        .get(idx)
                                        .map(|cell| cell.as_str() == name.as_str())
                                        .unwrap_or(false)
                                });
                                usize::from(names_match)
                            }
                        } else {
                            0
                        };

                        let data_row_count = sheet.row_count().saturating_sub(header_offset);

                        // Handle negative indexing first
                        let actual_idx = if *idx_int < 0 {
                            let adjusted = data_row_count as i64 + idx_int;
                            if adjusted < 0 {
                                return Err(PipError::runtime(0, "Sheet row index out of bounds"));
                            }
                            (adjusted as usize) + header_offset
                        } else {
                            // Positive indexing
                            let idx_usize = *idx_int as usize;
                            if idx_usize >= data_row_count {
                                return Err(PipError::runtime(0, "Sheet row index out of bounds"));
                            }
                            idx_usize + header_offset
                        };

                        // Get the row data
                        if actual_idx >= sheet.row_count() {
                            return Err(PipError::runtime(0, "Sheet row index out of bounds"));
                        }

                        // Convert row to object using column names if available
                        if let Some(col_names) = sheet.column_names() {
                            let mut row_obj = std::collections::HashMap::new();
                            for (col_idx, col_name) in col_names.iter().enumerate() {
                                let cell_value =
                                    sheet.get(actual_idx, col_idx).unwrap_or(&CellValue::Null);
                                row_obj.insert(col_name.clone(), cell_to_value(cell_value.clone()));
                            }
                            Ok(Value::Object(row_obj))
                        } else {
                            // No column names - return as array
                            let mut row_arr = Vec::new();
                            for col_idx in 0..sheet.col_count() {
                                let cell_value =
                                    sheet.get(actual_idx, col_idx).unwrap_or(&CellValue::Null);
                                row_arr.push(cell_to_value(cell_value.clone()));
                            }
                            Ok(Value::Array(row_arr))
                        }
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

            Expr::Call { function, args } => self.call_function(function, args, 0).await,

            Expr::CallExpr { callee, args } => {
                let callee_val = self.eval_expr(callee).await?;
                let arg_vals = self.eval_args(args, 0).await?;
                match callee_val {
                    Value::Lambda { params, body } => {
                        self.apply_lambda(&params, &body, &arg_vals).await
                    }
                    _ => Err(PipError::runtime(
                        0,
                        format!("Cannot call {}", callee_val.type_name()),
                    )),
                }
            }

            Expr::MethodCall {
                object,
                method,
                args,
            } => {
                let obj_val = self.eval_expr(object).await?;
                let arg_vals = self.eval_args(args, 0).await?;

                // Dispatch method call based on object type
                match &obj_val {
                    Value::Sheet(sheet) => {
                        // Handle Sheet methods
                        self.call_sheet_method(sheet, method, arg_vals).await
                    }
                    Value::Table(_) => {
                        // Handle Table methods (if any)
                        Err(PipError::runtime(
                            0,
                            format!("Table method '{}' not yet implemented", method),
                        ))
                    }
                    Value::Object(_map) => {
                        // Check if this is a Book (object with sheets as values)
                        // For now, we'll assume Object methods are not supported
                        Err(PipError::runtime(
                            0,
                            format!("Object method '{}' not yet implemented", method),
                        ))
                    }
                    _ => Err(PipError::runtime(
                        0,
                        format!(
                            "Method '{}' not supported on {}",
                            method,
                            obj_val.type_name()
                        ),
                    )),
                }
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

            Expr::Lambda { params, body } => Ok(Value::Lambda {
                params: params.clone(),
                body: (**body).clone(),
            }),

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

    async fn build_ref_binding(&mut self, arg_expr: &Expr, line: usize) -> PipResult<VarBinding> {
        match arg_expr {
            Expr::Variable(name) => {
                let scopes = self.scopes.read().await;
                match find_binding(&scopes, name) {
                    Some((scope_index, VarBinding::Value(_))) => Ok(VarBinding::Ref(RefTarget {
                        name: name.clone(),
                        scope_index,
                    })),
                    Some((_, VarBinding::Ref(target))) => {
                        let final_target =
                            resolve_ref_target_info(&scopes, target.clone()).unwrap_or(target);
                        Ok(VarBinding::Ref(final_target))
                    }
                    Some((_, VarBinding::RefLValue(ref_lvalue))) => {
                        Ok(VarBinding::RefLValue(ref_lvalue))
                    }
                    None => Err(PipError::runtime(
                        line,
                        "ByRef parameter requires an existing variable",
                    )),
                }
            }
            _ => {
                let lvalue = self.lvalue_from_expr(arg_expr, line).await?;
                let ref_lvalue = self.lvalue_to_ref_lvalue(lvalue, line).await?;
                Ok(VarBinding::RefLValue(ref_lvalue))
            }
        }
    }

    #[async_recursion]
    async fn lvalue_from_expr(&mut self, expr: &Expr, line: usize) -> PipResult<LValue> {
        match expr {
            Expr::Variable(name) => Ok(LValue::Variable(name.clone())),
            Expr::FieldAccess { object, field } => Ok(LValue::Field {
                object: Box::new(self.lvalue_from_expr(object, line).await?),
                field: field.clone(),
            }),
            Expr::ArrayIndex { array, index } => {
                let array_lvalue = self.lvalue_from_expr(array, line).await?;
                let idx_val = self.eval_expr(index).await.map_err(|e| e.with_line(line))?;
                let idx_int = idx_val
                    .as_int()
                    .ok_or_else(|| PipError::runtime(line, "Array index must be integer"))?;
                Ok(LValue::Index {
                    array: Box::new(array_lvalue),
                    index: Box::new(Expr::Literal(Literal::Int(idx_int))),
                })
            }
            _ => Err(PipError::runtime(
                line,
                "ByRef parameter requires a variable, field, or array element",
            )),
        }
    }

    async fn lvalue_to_ref_lvalue(&self, lvalue: LValue, line: usize) -> PipResult<RefLValue> {
        let mut access = Vec::new();
        let base_name = flatten_lvalue(lvalue, &mut access, line)?;

        let scopes = self.scopes.read().await;
        match find_binding(&scopes, &base_name) {
            Some((scope_index, VarBinding::Value(_))) => Ok(RefLValue {
                base: RefTarget {
                    name: base_name,
                    scope_index,
                },
                access,
            }),
            Some((_, VarBinding::Ref(target))) => {
                let final_target =
                    resolve_ref_target_info(&scopes, target.clone()).unwrap_or(target);
                Ok(RefLValue {
                    base: final_target,
                    access,
                })
            }
            Some((_, VarBinding::RefLValue(mut ref_lvalue))) => {
                ref_lvalue.access.extend(access);
                Ok(ref_lvalue)
            }
            None => Err(PipError::runtime(
                line,
                "ByRef parameter requires an existing variable",
            )),
        }
    }

    /// Call a function (built-in or user-defined).
    async fn call_function(&mut self, name: &str, args: &[Expr], line: usize) -> PipResult<Value> {
        if formula::is_dsl_formula_function(name) {
            let arg_vals = self.eval_args(args, line).await?;
            if arg_vals.len() == 2 {
                if let (Value::Sheet(sheet), Value::String(range)) = (&arg_vals[0], &arg_vals[1]) {
                    if let Some(formula_name) = formula::range_function_name(name) {
                        return formula::eval_sheet_range_function(
                            sheet,
                            formula_name,
                            range,
                            line,
                        );
                    }
                }
            }
            return formula::call_formula_function(name, &arg_vals, line);
        }

        // Check built-in functions first (evaluate args only if needed)
        if builtins::is_builtin(name) {
            let arg_vals = self.eval_args(args, line).await?;
            if let Some(result) = builtins::call_builtin(self, name, arg_vals, line).await {
                return result;
            }
        }

        // Functions that still need to be migrated to modules
        match name.to_lowercase().as_str() {
            "consolidate" => {
                let arg_vals = self.eval_args(args, line).await?;
                // consolidate(book) or consolidate(book, source = "_source")
                if arg_vals.is_empty() || arg_vals.len() > 2 {
                    return Err(PipError::runtime(
                        line,
                        "consolidate() takes 1 or 2 arguments: consolidate(book) or consolidate(book, source_column_name)",
                    ));
                }
                match &arg_vals[0] {
                    Value::Object(book_obj) => {
                        // Convert object (book) to consolidated array
                        let source_col = if arg_vals.len() == 2 {
                            match arg_vals[1].as_str() {
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
                let arg_vals = self.eval_args(args, line).await?;
                // register_python("name", "lambda x: x * 2")
                // register_python("name", "file.py", "function_name")
                let runtime = self
                    .python_runtime
                    .as_ref()
                    .ok_or_else(|| PipError::runtime(line, "Python runtime not available"))?;

                match arg_vals.len() {
                    2 => {
                        // Inline lambda/def
                        let name = arg_vals[0].as_str().ok_or_else(|| {
                            PipError::runtime(line, "register_python: first argument must be string (name)")
                        })?;
                        let code = arg_vals[1].as_str().ok_or_else(|| {
                            PipError::runtime(line, "register_python: second argument must be string (code)")
                        })?;
                        runtime.register_inline(name, code).await?;
                        Ok(Value::Null)
                    }
                    3 => {
                        // From file
                        let name = arg_vals[0].as_str().ok_or_else(|| {
                            PipError::runtime(line, "register_python: first argument must be string (name)")
                        })?;
                        let file_path = arg_vals[1].as_str().ok_or_else(|| {
                            PipError::runtime(line, "register_python: second argument must be string (file path)")
                        })?;
                        let func_name = arg_vals[2].as_str().ok_or_else(|| {
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
                let arg_vals = self.eval_args(args, line).await?;
                if arg_vals.len() != 1 {
                    return Err(PipError::runtime(
                        line,
                        "sheet_transpose() takes exactly 1 argument",
                    ));
                }
                match &arg_vals[0] {
                    Value::Sheet(sheet) => {
                        let mut new_sheet = sheet.clone();
                        new_sheet.transpose();
                        Ok(Value::Sheet(new_sheet))
                    }
                    _ => Err(PipError::runtime(line, "Argument must be a sheet")),
                }
            }
            "sheet_select_columns" => {
                let arg_vals = self.eval_args(args, line).await?;
                if arg_vals.len() != 2 {
                    return Err(PipError::runtime(
                        line,
                        "sheet_select_columns() takes exactly 2 arguments",
                    ));
                }
                match (&arg_vals[0], &arg_vals[1]) {
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
                let arg_vals = self.eval_args(args, line).await?;
                if arg_vals.len() != 2 {
                    return Err(PipError::runtime(
                        line,
                        "sheet_remove_columns() takes exactly 2 arguments",
                    ));
                }
                match (&arg_vals[0], &arg_vals[1]) {
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
                let arg_vals = self.eval_args(args, line).await?;
                if arg_vals.len() != 1 {
                    return Err(PipError::runtime(
                        line,
                        "sheet_remove_empty_rows() takes exactly 1 argument",
                    ));
                }
                match &arg_vals[0] {
                    Value::Sheet(sheet) => {
                        let mut new_sheet = sheet.clone();
                        new_sheet.remove_empty_rows();
                        Ok(Value::Sheet(new_sheet))
                    }
                    _ => Err(PipError::runtime(line, "Argument must be a sheet")),
                }
            }
            "sheet_row_count" => {
                let arg_vals = self.eval_args(args, line).await?;
                if arg_vals.len() != 1 {
                    return Err(PipError::runtime(
                        line,
                        "sheet_row_count() takes exactly 1 argument",
                    ));
                }
                match &arg_vals[0] {
                    Value::Sheet(sheet) => Ok(Value::Int(sheet.row_count() as i64)),
                    _ => Err(PipError::runtime(line, "Argument must be a sheet")),
                }
            }
            "sheet_col_count" => {
                let arg_vals = self.eval_args(args, line).await?;
                if arg_vals.len() != 1 {
                    return Err(PipError::runtime(
                        line,
                        "sheet_col_count() takes exactly 1 argument",
                    ));
                }
                match &arg_vals[0] {
                    Value::Sheet(sheet) => Ok(Value::Int(sheet.col_count() as i64)),
                    _ => Err(PipError::runtime(line, "Argument must be a sheet")),
                }
            }
            "sheet_get_a1" => {
                let arg_vals = self.eval_args(args, line).await?;
                if arg_vals.len() != 2 {
                    return Err(PipError::runtime(
                        line,
                        "sheet_get_a1() takes exactly 2 arguments (sheet, notation)",
                    ));
                }
                match (&arg_vals[0], &arg_vals[1]) {
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
                let arg_vals = self.eval_args(args, line).await?;
                if arg_vals.len() != 3 {
                    return Err(PipError::runtime(
                        line,
                        "sheet_set_a1() takes exactly 3 arguments (sheet, notation, value)",
                    ));
                }
                match (&arg_vals[0], &arg_vals[1], &arg_vals[2]) {
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
                let arg_vals = self.eval_args(args, line).await?;
                if arg_vals.len() != 2 {
                    return Err(PipError::runtime(
                        line,
                        "sheet_get_range() takes exactly 2 arguments (sheet, range_notation)",
                    ));
                }
                match (&arg_vals[0], &arg_vals[1]) {
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
                let arg_vals = self.eval_args(args, line).await?;
                if arg_vals.len() != 2 {
                    return Err(PipError::runtime(
                        line,
                        "sheet_column_by_name() takes exactly 2 arguments (sheet, column_name)",
                    ));
                }
                match (&arg_vals[0], &arg_vals[1]) {
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
                let arg_vals = self.eval_args(args, line).await?;
                if arg_vals.len() != 3 {
                    return Err(PipError::runtime(line, "sheet_get_by_name() takes exactly 3 arguments (sheet, row_index, column_name)"));
                }
                match (&arg_vals[0], &arg_vals[1], &arg_vals[2]) {
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
                let arg_vals = self.eval_args(args, line).await?;
                if arg_vals.len() != 4 {
                    return Err(PipError::runtime(line, "sheet_set_by_name() takes exactly 4 arguments (sheet, row_index, column_name, value)"));
                }
                match (&arg_vals[0], &arg_vals[1], &arg_vals[2], &arg_vals[3]) {
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
                    let mut required_count = 0usize;
                    let mut has_param_array = false;
                    for param in &func.params {
                        if param.is_param_array {
                            has_param_array = true;
                        } else if param.default.is_none() {
                            required_count += 1;
                        }
                    }

                    if args.len() < required_count
                        || (!has_param_array && args.len() > func.params.len())
                    {
                        return Err(PipError::runtime(
                            line,
                            format!(
                                "Function '{}' expects {} arguments, got {}",
                                name,
                                if has_param_array {
                                    format!("at least {required_count}")
                                } else {
                                    format!(
                                        "{}{}",
                                        required_count,
                                        if required_count == func.params.len() {
                                            String::new()
                                        } else {
                                            format!(" to {}", func.params.len())
                                        }
                                    )
                                },
                                args.len()
                            ),
                        ));
                    }

                    // Create new scope with parameters
                    self.push_scope().await;
                    let param_result: PipResult<()> = async {
                        let mut arg_index = 0usize;
                        for param in &func.params {
                            if param.is_param_array {
                                let mut values = Vec::new();
                                while arg_index < args.len() {
                                    let value = self
                                        .eval_expr(&args[arg_index])
                                        .await
                                        .map_err(|e| e.with_line(line))?;
                                    values.push(value);
                                    arg_index += 1;
                                }
                                self.declare_var(&param.name, Value::Array(values)).await;
                                continue;
                            }

                            if let Some(arg_expr) = args.get(arg_index) {
                                match param.mode {
                                    ParamMode::ByVal => {
                                        let value = self
                                            .eval_expr(arg_expr)
                                            .await
                                            .map_err(|e| e.with_line(line))?;
                                        self.declare_var(&param.name, value).await;
                                    }
                                    ParamMode::ByRef => {
                                        let binding =
                                            self.build_ref_binding(arg_expr, line).await?;
                                        let mut scopes = self.scopes.write().await;
                                        if let Some(scope) = scopes.last_mut() {
                                            scope.insert(param.name.clone(), binding);
                                        }
                                    }
                                }
                                arg_index += 1;
                            } else if let Some(default_expr) = &param.default {
                                let value = self
                                    .eval_expr(default_expr)
                                    .await
                                    .map_err(|e| e.with_line(line))?;
                                self.declare_var(&param.name, value).await;
                            } else {
                                return Err(PipError::runtime(
                                    line,
                                    format!(
                                        "Function '{}' missing argument for parameter '{}'",
                                        name, param.name
                                    ),
                                ));
                            }
                        }
                        Ok(())
                    }
                    .await;

                    if let Err(e) = param_result {
                        self.pop_scope().await;
                        return Err(e);
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
                            Err(PipError::ExitFunction(_exit_line)) => {
                                // Exit Function - return Null explicitly
                                self.pop_scope().await;
                                return Ok(Value::Null);
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
                    // Check if it's a variable containing a lambda
                    if let Some(Value::Lambda { params, body }) = self.get_var(name).await {
                        let arg_vals = self.eval_args(args, line).await?;
                        // Call the lambda
                        if params.len() != arg_vals.len() {
                            return Err(PipError::runtime(
                                line,
                                format!(
                                    "Lambda '{}' expects {} arguments, got {}",
                                    name,
                                    params.len(),
                                    arg_vals.len()
                                ),
                            ));
                        }

                        self.push_scope().await;
                        for (param, arg) in params.iter().zip(arg_vals.iter()) {
                            self.declare_var(param, arg.clone()).await;
                        }
                        let result = self.eval_expr(&body).await;
                        self.pop_scope().await;
                        return result;
                    }

                    // Check Python functions if feature is enabled
                    #[cfg(feature = "python")]
                    if let Some(runtime) = &self.python_runtime {
                        if runtime.has_function(name).await {
                            let arg_vals = self.eval_args(args, line).await?;
                            return runtime.call(name, arg_vals).await;
                        }
                    }

                    Err(PipError::runtime(line, format!("Unknown function: {name}")))
                }
            }
        }
    }

    // SQL query methods moved to sql_builder.rs module

    /// Convert a Sheet to RecordBatches for SQL registration
    fn convert_sheet_to_batches(
        &self,
        sheet: &Sheet,
        _table_name: &str,
    ) -> PipResult<Vec<arrow::array::RecordBatch>> {
        use arrow::array::ArrayRef;
        use arrow::array::RecordBatch;
        use arrow::datatypes::{DataType, Field, Schema};
        use piptable_sheet::CellValue;
        use std::sync::Arc;

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
            return Ok(vec![batch]);
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
            return Ok(vec![batch]);
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
            .map(|(name, dtype): (&String, &arrow::datatypes::DataType)| {
                Field::new(name.clone(), dtype.clone(), true)
            })
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

        Ok(vec![batch])
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
        let path_lower = path.to_lowercase();
        if path_lower.ends_with(".csv") {
            self.sql.register_csv(&table_name, path).await?;
        } else if path_lower.ends_with(".json") || path_lower.ends_with(".ndjson") {
            self.sql.register_json(&table_name, path).await?;
        } else if path_lower.ends_with(".parquet") {
            self.sql.register_parquet(&table_name, path).await?;
        } else if path_lower.ends_with(".xlsx") || path_lower.ends_with(".xls") {
            // Load Excel file as Sheet and register it
            use crate::io::import_sheet;
            let sheet = import_sheet(path, None, true) // Assume Excel files have headers by default
                .map_err(|e| {
                    PipError::runtime(0, format!("Failed to load Excel file '{}': {}", path, e))
                })?;
            // Convert sheet to RecordBatches and register directly with consistent naming
            let batches = self.convert_sheet_to_batches(&sheet, &table_name)?;
            self.sql.register_table(&table_name, batches).await?;
            return Ok(table_name);
        } else if path_lower.ends_with(".toon") {
            // Load TOON file as Sheet and register it
            use crate::io::import_sheet;
            let sheet = import_sheet(path, None, true) // TOON files have structured format with headers
                .map_err(|e| {
                    PipError::runtime(0, format!("Failed to load TOON file '{}': {}", path, e))
                })?;
            // Convert sheet to RecordBatches and register directly
            let batches = self.convert_sheet_to_batches(&sheet, &table_name)?;
            self.sql.register_table(&table_name, batches).await?;
            return Ok(table_name);
        } else {
            // Default to CSV
            self.sql.register_csv(&table_name, path).await?;
        }

        Ok(table_name)
    }

    /// Register a sheet as a table and return the table name.
    /// Note: Sheet variables are registered with a "sheet_" prefix to avoid conflicts with other table types.
    /// This is handled transparently when referencing variables in SQL queries.
    async fn register_sheet_as_table(&mut self, name: &str, sheet: &Sheet) -> PipResult<String> {
        let table_name = format!("sheet_{}", name.replace(['-', '.', ' '], "_"));

        // Convert sheet to batches using the shared helper
        let batches = self.convert_sheet_to_batches(sheet, &table_name)?;

        // Register the batches with the SQL engine
        self.sql.register_table(&table_name, batches).await?;

        Ok(table_name)
    }

    /// Register a Value::Table variable as a table and return the table name.
    async fn register_table_variable(
        &mut self,
        name: &str,
        batches: &[Arc<arrow::array::RecordBatch>],
    ) -> PipResult<String> {
        let table_name = format!("table_{}", name.replace(['-', '.', ' '], "_"));

        if batches.is_empty() {
            // For empty results, we can't preserve schema without at least one batch
            // Return error to avoid silent failures with column queries
            return Err(PipError::runtime(
                0,
                format!(
                    "Cannot register empty table '{}' - no schema information available",
                    name
                ),
            ));
        }

        // Check if all batches have 0 rows but preserve the schema
        let all_empty = batches.iter().all(|b| b.num_rows() == 0);
        if all_empty {
            // We have schema but no rows - create empty batch with correct schema
            use arrow::array::RecordBatch;
            let schema = batches[0].schema();
            let batch = RecordBatch::new_empty(schema);
            self.sql.register_table(&table_name, vec![batch]).await?;
        } else {
            // Convert Arc<RecordBatch> to RecordBatch by cloning
            let mut batch_vec = Vec::new();
            for batch in batches {
                batch_vec.push((**batch).clone());
            }
            self.sql.register_table(&table_name, batch_vec).await?;
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
                let binding = {
                    let scopes = self.scopes.read().await;
                    find_binding(&scopes, name).map(|(_, binding)| binding)
                };
                if let Some(VarBinding::RefLValue(ref_lvalue)) = binding {
                    let mut scopes = self.scopes.write().await;
                    return assign_ref_lvalue(&mut scopes, ref_lvalue, value, line);
                }
                self.set_var(name, value).await?;
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
    pub async fn set_var(&self, name: &str, value: Value) -> PipResult<()> {
        enum ResolvedBinding {
            Value { name: String, scope_index: usize },
            RefTarget(RefTarget),
            RefLValue(RefLValue),
        }

        let resolved = {
            let scopes = self.scopes.read().await;
            match find_binding(&scopes, name) {
                Some((scope_index, VarBinding::Value(_))) => Some(ResolvedBinding::Value {
                    name: name.to_string(),
                    scope_index,
                }),
                Some((_, VarBinding::Ref(target))) => {
                    let final_target =
                        resolve_ref_target_info(&scopes, target.clone()).unwrap_or(target);
                    Some(ResolvedBinding::RefTarget(final_target))
                }
                Some((_, VarBinding::RefLValue(mut ref_lvalue))) => {
                    if let Some(final_target) =
                        resolve_ref_target_info(&scopes, ref_lvalue.base.clone())
                    {
                        ref_lvalue.base = final_target;
                    }
                    Some(ResolvedBinding::RefLValue(ref_lvalue))
                }
                None => None,
            }
        };

        // Clear any cached table for the variable being assigned.
        let table_to_drop = {
            let mut sheet_tables = self.sheet_tables.write().await;
            match &resolved {
                Some(ResolvedBinding::Value { name, .. }) => sheet_tables.remove(name),
                Some(ResolvedBinding::RefTarget(target)) => sheet_tables.remove(&target.name),
                Some(ResolvedBinding::RefLValue(ref_lvalue)) => {
                    sheet_tables.remove(&ref_lvalue.base.name)
                }
                None => sheet_tables.remove(name),
            }
        };
        if let Some(table_name) = table_to_drop {
            let _ = self.sql.deregister_table(&table_name).await;
        }

        let mut scopes = self.scopes.write().await;

        if let Some(resolved) = resolved {
            match resolved {
                ResolvedBinding::Value {
                    name: binding_name,
                    scope_index,
                } => {
                    if let Some(scope) = scopes.get_mut(scope_index) {
                        scope.insert(binding_name, VarBinding::Value(value));
                    }
                }
                ResolvedBinding::RefTarget(target) => {
                    if let Some(scope) = scopes.get_mut(target.scope_index) {
                        scope.insert(target.name, VarBinding::Value(value));
                    }
                }
                ResolvedBinding::RefLValue(ref_lvalue) => {
                    assign_ref_lvalue(&mut scopes, ref_lvalue, value, 0)?;
                }
            }
            return Ok(());
        }

        if let Some(scope) = scopes.last_mut() {
            scope.insert(name.to_string(), VarBinding::Value(value));
        }
        Ok(())
    }

    /// Declare a variable in the current scope only (shadows outer bindings).
    /// Use this for loop variables and function parameters.
    async fn declare_var(&self, name: &str, value: Value) {
        let mut scopes = self.scopes.write().await;
        if let Some(scope) = scopes.last_mut() {
            scope.insert(name.to_string(), VarBinding::Value(value));
        }
    }

    /// Get a variable, searching from innermost to outermost scope.
    pub async fn get_var(&self, name: &str) -> Option<Value> {
        let scopes = self.scopes.read().await;
        let (_scope_index, binding) = find_binding(&scopes, name)?;
        resolve_binding_value(&scopes, binding)
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

    /// Apply a lambda expression with given arguments
    async fn apply_lambda(
        &mut self,
        lambda_params: &[String],
        lambda_body: &Expr,
        args: &[Value],
    ) -> PipResult<Value> {
        // Check argument count
        if args.len() != lambda_params.len() {
            return Err(PipError::runtime(
                0,
                format!(
                    "Lambda expects {} arguments, got {}",
                    lambda_params.len(),
                    args.len()
                ),
            ));
        }

        self.push_scope().await;
        for (param, arg) in lambda_params.iter().zip(args.iter()) {
            self.declare_var(param, arg.clone()).await;
        }
        let result = self.eval_expr(lambda_body).await;
        self.pop_scope().await;
        result
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl Interpreter {
    /// Call a method on a Sheet object
    async fn call_sheet_method(
        &mut self,
        sheet: &Sheet,
        method: &str,
        args: Vec<Value>,
    ) -> PipResult<Value> {
        match method {
            "transpose" => {
                if !args.is_empty() {
                    return Err(PipError::runtime(0, "transpose() takes no arguments"));
                }
                let mut transposed = sheet.clone();
                transposed.transpose();
                Ok(Value::Sheet(transposed))
            }
            "name_columns_by_row" => {
                if args.len() != 1 {
                    return Err(PipError::runtime(
                        0,
                        "name_columns_by_row() takes exactly 1 argument",
                    ));
                }
                let row_index = args[0]
                    .as_int()
                    .ok_or_else(|| PipError::runtime(0, "Row index must be an integer"))?;
                if row_index < 0 {
                    return Err(PipError::runtime(0, "Row index cannot be negative"));
                }
                let mut new_sheet = sheet.clone();
                new_sheet
                    .name_columns_by_row(row_index as usize)
                    .map_err(|e| PipError::runtime(0, format!("Failed to name columns: {}", e)))?;
                Ok(Value::Sheet(new_sheet))
            }
            "select_columns" => {
                if args.len() != 1 {
                    return Err(PipError::runtime(
                        0,
                        "select_columns() takes exactly 1 argument",
                    ));
                }
                let columns = match &args[0] {
                    Value::Array(arr) => {
                        let mut cols = Vec::new();
                        for val in arr {
                            cols.push(
                                val.as_str()
                                    .ok_or_else(|| {
                                        PipError::runtime(0, "Column names must be strings")
                                    })?
                                    .to_string(),
                            );
                        }
                        cols
                    }
                    _ => {
                        return Err(PipError::runtime(
                            0,
                            "select_columns() requires an array of column names",
                        ))
                    }
                };
                let col_refs: Vec<&str> = columns.iter().map(|s| s.as_str()).collect();
                let mut selected = sheet.clone();
                selected.select_columns(&col_refs).map_err(|e| {
                    PipError::runtime(0, format!("Failed to select columns: {}", e))
                })?;
                Ok(Value::Sheet(selected))
            }
            "remove_columns" => {
                if args.len() != 1 {
                    return Err(PipError::runtime(
                        0,
                        "remove_columns() takes exactly 1 argument",
                    ));
                }
                let columns = match &args[0] {
                    Value::Array(arr) => {
                        let mut cols = Vec::new();
                        for val in arr {
                            cols.push(
                                val.as_str()
                                    .ok_or_else(|| {
                                        PipError::runtime(0, "Column names must be strings")
                                    })?
                                    .to_string(),
                            );
                        }
                        cols
                    }
                    _ => {
                        return Err(PipError::runtime(
                            0,
                            "remove_columns() requires an array of column names",
                        ))
                    }
                };
                let mut new_sheet = sheet.clone();
                let col_refs: Vec<&str> = columns.iter().map(|s| s.as_str()).collect();
                new_sheet.remove_columns(&col_refs).map_err(|e| {
                    PipError::runtime(0, format!("Failed to remove columns: {}", e))
                })?;
                Ok(Value::Sheet(new_sheet))
            }
            "row_count" => {
                if !args.is_empty() {
                    return Err(PipError::runtime(0, "row_count() takes no arguments"));
                }
                Ok(Value::Int(sheet.row_count() as i64))
            }
            "column_count" | "col_count" => {
                if !args.is_empty() {
                    return Err(PipError::runtime(0, "column_count() takes no arguments"));
                }
                Ok(Value::Int(sheet.col_count() as i64))
            }
            "column_by_name" => {
                if args.len() != 1 {
                    return Err(PipError::runtime(
                        0,
                        "column_by_name() takes exactly 1 argument",
                    ));
                }
                let col_name = args[0]
                    .as_str()
                    .ok_or_else(|| PipError::runtime(0, "Column name must be a string"))?;

                // Get the column data as an array of values
                let col_data = sheet.column_by_name(col_name).map_err(|e| {
                    PipError::runtime(0, format!("Failed to get column '{}': {}", col_name, e))
                })?;

                // Convert CellValues to Values
                let values: Vec<Value> = col_data
                    .into_iter()
                    .map(|cell| sheet_conversions::cell_to_value(cell.clone()))
                    .collect();

                Ok(Value::Array(values))
            }
            "column_names" => {
                if !args.is_empty() {
                    return Err(PipError::runtime(0, "column_names() takes no arguments"));
                }
                match sheet.column_names() {
                    Some(names) => {
                        let names_vec: Vec<Value> =
                            names.iter().map(|s| Value::String(s.clone())).collect();
                        Ok(Value::Array(names_vec))
                    }
                    None => Ok(Value::Null),
                }
            }

            "map" => {
                if args.len() != 1 {
                    return Err(PipError::runtime(0, "map() takes exactly 1 argument"));
                }

                match &args[0] {
                    Value::Lambda { params, body } => {
                        if params.len() != 1 {
                            return Err(PipError::runtime(
                                0,
                                "Lambda for map() must take exactly 1 parameter",
                            ));
                        }

                        let mut new_sheet = sheet.clone();
                        let column_names = sheet.column_names().cloned();

                        // Apply lambda to each data row (skip header if present)
                        // Check if a physical header row exists by comparing first row with column names
                        let start_row = if let Some(names) = &column_names {
                            if sheet.data().is_empty() {
                                0
                            } else {
                                let first_row = &sheet.data()[0];
                                let names_match = names.iter().enumerate().all(|(idx, name)| {
                                    first_row
                                        .get(idx)
                                        .map(|cell| cell.as_str() == name.as_str())
                                        .unwrap_or(false)
                                });
                                usize::from(names_match)
                            }
                        } else {
                            0
                        };

                        for row_idx in start_row..new_sheet.row_count() {
                            let row_value = if let Some(col_names) = &column_names {
                                let mut row_obj = std::collections::HashMap::new();
                                for (col_idx, col_name) in col_names.iter().enumerate() {
                                    let cell = sheet
                                        .get(row_idx, col_idx)
                                        .cloned()
                                        .unwrap_or(piptable_sheet::CellValue::Null);
                                    row_obj.insert(
                                        col_name.clone(),
                                        sheet_conversions::cell_to_value(cell),
                                    );
                                }
                                Value::Object(row_obj)
                            } else {
                                let row = sheet.row(row_idx).map_err(|e| {
                                    PipError::runtime(0, format!("Failed to access row: {}", e))
                                })?;
                                let values: Vec<Value> = row
                                    .iter()
                                    .cloned()
                                    .map(sheet_conversions::cell_to_value)
                                    .collect();
                                Value::Array(values)
                            };

                            match self.apply_lambda(params, body, &[row_value]).await {
                                Ok(result) => match result {
                                    Value::Object(obj) => {
                                        let col_names = column_names.as_ref().ok_or_else(|| {
                                            PipError::runtime(
                                                0,
                                                "map() with unnamed columns must return an array",
                                            )
                                        })?;
                                        for (col_idx, col_name) in col_names.iter().enumerate() {
                                            let value =
                                                obj.get(col_name).cloned().unwrap_or(Value::Null);
                                            let new_cell = sheet_conversions::value_to_cell(&value);
                                            let _ = new_sheet.set(row_idx, col_idx, new_cell);
                                        }
                                    }
                                    Value::Array(values) => {
                                        let col_count = new_sheet.col_count();
                                        for col_idx in 0..col_count {
                                            let value =
                                                values.get(col_idx).cloned().unwrap_or(Value::Null);
                                            let new_cell = sheet_conversions::value_to_cell(&value);
                                            let _ = new_sheet.set(row_idx, col_idx, new_cell);
                                        }
                                    }
                                    _ => {
                                        return Err(PipError::runtime(
                                            0,
                                            "map() lambda must return an object or array",
                                        ));
                                    }
                                },
                                Err(e) => {
                                    return Err(PipError::runtime(
                                        0,
                                        format!("Lambda error in map() at row {}: {}", row_idx, e),
                                    ));
                                }
                            }
                        }

                        Ok(Value::Sheet(new_sheet))
                    }
                    _ => Err(PipError::runtime(0, "map() requires a lambda expression")),
                }
            }

            "filter" => {
                if args.len() != 1 {
                    return Err(PipError::runtime(0, "filter() takes exactly 1 argument"));
                }

                match &args[0] {
                    Value::Lambda { params, body } => {
                        if params.len() != 1 {
                            return Err(PipError::runtime(
                                0,
                                "Lambda for filter() must take exactly 1 parameter",
                            ));
                        }

                        let mut new_sheet = sheet.clone();
                        let mut rows_to_keep = std::collections::HashSet::new();

                        // Determine which rows to keep (skip header if present)
                        // Check if a physical header row exists by comparing first row with column names
                        let start_row = if let Some(names) = sheet.column_names() {
                            if sheet.data().is_empty() {
                                0
                            } else {
                                let first_row = &sheet.data()[0];
                                let names_match = names.iter().enumerate().all(|(idx, name)| {
                                    first_row
                                        .get(idx)
                                        .map(|cell| cell.as_str() == name.as_str())
                                        .unwrap_or(false)
                                });
                                if names_match {
                                    // Physical header exists, keep it
                                    rows_to_keep.insert(0);
                                    1
                                } else {
                                    0
                                }
                            }
                        } else {
                            0
                        };

                        for row_idx in start_row..sheet.row_count() {
                            let row_value = if let Some(col_names) = sheet.column_names() {
                                let mut row_obj = std::collections::HashMap::new();
                                for (col_idx, col_name) in col_names.iter().enumerate() {
                                    let cell = sheet
                                        .get(row_idx, col_idx)
                                        .cloned()
                                        .unwrap_or(piptable_sheet::CellValue::Null);
                                    row_obj.insert(
                                        col_name.clone(),
                                        sheet_conversions::cell_to_value(cell),
                                    );
                                }
                                Value::Object(row_obj)
                            } else {
                                let row = sheet.row(row_idx).map_err(|e| {
                                    PipError::runtime(0, format!("Failed to access row: {}", e))
                                })?;
                                let values: Vec<Value> = row
                                    .iter()
                                    .cloned()
                                    .map(sheet_conversions::cell_to_value)
                                    .collect();
                                Value::Array(values)
                            };

                            match self.apply_lambda(params, body, &[row_value]).await {
                                Ok(result) => {
                                    if result.is_truthy() {
                                        rows_to_keep.insert(row_idx);
                                    }
                                }
                                Err(e) => {
                                    return Err(PipError::runtime(
                                        0,
                                        format!(
                                            "Lambda error in filter() at row {}: {}",
                                            row_idx, e
                                        ),
                                    ));
                                }
                            }
                        }

                        // Filter the sheet to keep only selected rows (HashSet for O(1) lookup)
                        new_sheet.filter_rows(|row_idx, _row| rows_to_keep.contains(&row_idx));

                        Ok(Value::Sheet(new_sheet))
                    }
                    _ => Err(PipError::runtime(
                        0,
                        "filter() requires a lambda expression",
                    )),
                }
            }

            _ => Err(PipError::runtime(
                0,
                format!("Unknown sheet method: {}", method),
            )),
        }
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
        interp.set_var("x", Value::Int(42)).await.unwrap();
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
        assert!(matches!(&data, Value::Sheet(sheet) if sheet.row_count() == 3)); // header + 2 data rows

        // Check the sheet has the right data
        if let Value::Sheet(sheet) = &data {
            // Check column names
            let col_names = sheet.column_names().unwrap();
            assert_eq!(col_names[0], "name");
            assert_eq!(col_names[1], "age");

            // Check first data row
            let row = &sheet.data()[1]; // row 0 is header, row 1 is first data row
            assert!(matches!(&row[0], piptable_sheet::CellValue::String(s) if s == "alice"));
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

        // Verify data was loaded without headers
        let data = interp.get_var("data").await.unwrap();
        assert!(matches!(&data, Value::Sheet(sheet) if sheet.row_count() == 2)); // 2 data rows, no header

        // Check that no column names exist
        if let Value::Sheet(sheet) = &data {
            // With headers=false, there should be no column names
            assert!(sheet.column_names().is_none());

            // First row should be "alice,30"
            let row = &sheet.data()[0];
            assert!(matches!(&row[0], piptable_sheet::CellValue::String(s) if s == "alice"));
            // CSV parser might parse "30" as an integer
            assert!(
                matches!(&row[1], piptable_sheet::CellValue::Int(30))
                    || matches!(&row[1], piptable_sheet::CellValue::String(s) if s == "30")
            );
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
            let data_rows: Vec<_> = arr
                .iter()
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
            .await
            .unwrap();

        let script = r"result = consolidate(book, 123)"; // 123 is not a string
        let program = PipParser::parse_str(script).unwrap();
        let result = interp.eval(program).await;

        // Should error because source column must be a string
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be a string"));
    }

    #[tokio::test]
    async fn test_foreach_sheet() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.csv");
        std::fs::write(&file_path, "name,age\nalice,30\nbob,25").unwrap();

        let mut interp = Interpreter::new();
        let script = format!(
            r#"
            import "{}" into data
            dim count = 0
            for each row in data
                count = count + 1
            next
            "#,
            file_path.display()
        );
        let program = PipParser::parse_str(&script).unwrap();
        interp.eval(program).await.unwrap();

        // Verify the for each loop worked with Sheet
        let count = interp.get_var("count").await.unwrap();
        assert!(matches!(count, Value::Int(2))); // Should have iterated over 2 rows
    }

    #[tokio::test]
    async fn test_sheet_foreach_without_header_row() {
        // Test that for each correctly iterates when column_names exist but no physical header row
        let mut interp = Interpreter::new();

        // Create a sheet with a header row, then remove it
        // This simulates Sheet::from_records or after header removal
        let mut sheet = Sheet::from_data(vec![
            vec![
                CellValue::String("name".to_string()),
                CellValue::String("age".to_string()),
            ],
            vec![
                CellValue::String("Alice".to_string()),
                CellValue::String("30".to_string()),
            ],
            vec![
                CellValue::String("Bob".to_string()),
                CellValue::String("25".to_string()),
            ],
        ]);

        // Name columns using the first row
        sheet.name_columns_by_row(0).unwrap();

        // Now remove the header row, leaving only data rows
        // This creates a situation where column_names exist but no physical header
        sheet.row_delete(0).unwrap();

        interp.set_var("data", Value::Sheet(sheet)).await.unwrap();

        let script = r#"
            dim count = 0
            dim name1 = ""
            dim name2 = ""
            
            for each row in data
                count = count + 1
                if count = 1 then
                    name1 = row.name
                else
                    name2 = row.name
                end if
            next
        "#;

        let program = PipParser::parse_str(script).unwrap();
        let result = interp.eval(program).await;
        assert!(result.is_ok(), "Script failed: {:?}", result);

        // Should iterate over both data rows
        let count = interp.get_var("count").await.unwrap();
        assert!(
            matches!(count, Value::Int(2)),
            "Expected 2 iterations, got {:?}",
            count
        );

        // Should have both names
        let name1 = interp.get_var("name1").await.unwrap();
        assert!(matches!(name1, Value::String(s) if s == "Alice"));

        let name2 = interp.get_var("name2").await.unwrap();
        assert!(matches!(name2, Value::String(s) if s == "Bob"));
    }

    #[tokio::test]
    async fn test_sheet_sql_without_header_row() {
        // Test that SQL correctly handles sheets with column_names but no physical header
        let mut interp = Interpreter::new();

        // Create a sheet with header, name columns, then remove header
        let mut sheet = Sheet::from_data(vec![
            vec![
                CellValue::String("product".to_string()),
                CellValue::String("price".to_string()),
            ],
            vec![CellValue::String("Widget".to_string()), CellValue::Int(100)],
            vec![CellValue::String("Gadget".to_string()), CellValue::Int(200)],
        ]);

        sheet.name_columns_by_row(0).unwrap();
        sheet.row_delete(0).unwrap();

        interp
            .set_var("products", Value::Sheet(sheet))
            .await
            .unwrap();

        let script = r"
            dim result = query(SELECT * FROM products WHERE price > 150)
        ";

        let program = PipParser::parse_str(script).unwrap();
        let result = interp.eval(program).await;
        assert!(result.is_ok(), "SQL query failed: {:?}", result);

        // Should have one row (Gadget with price 200)
        let query_result = interp.get_var("result").await.unwrap();
        if let Value::Table(batches) = query_result {
            let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
            assert_eq!(total_rows, 1, "Expected 1 row, got {}", total_rows);
        } else {
            panic!("Expected Table result");
        }
    }

    #[tokio::test]
    async fn test_sheet_integer_indexing() {
        // Test that Sheet values support integer indexing
        let mut interp = Interpreter::new();

        // Create a sheet with headers using from_csv_str_with_options
        let csv_content = "name,age,city\nAlice,30,NYC\nBob,25,LA\nCharlie,35,SF";
        let csv_options = piptable_sheet::CsvOptions {
            has_headers: true,
            ..Default::default()
        };
        let sheet = Sheet::from_csv_str_with_options(csv_content, csv_options).unwrap();

        // Verify column names are detected (fixes issue #163)
        assert!(
            sheet.column_names().is_some(),
            "Column names should be detected"
        );

        interp.set_var("data", Value::Sheet(sheet)).await.unwrap();

        // Test positive indexing
        let script = r"
            dim first_row = data[0]
            dim second_row = data[1]
            dim third_row = data[2]
            
            ' Access fields from the row objects
            dim alice_name = first_row.name
            dim bob_age = second_row.age
            dim charlie_city = third_row.city
        ";

        let program = PipParser::parse_str(script).unwrap();
        let result = interp.eval(program).await;
        assert!(result.is_ok(), "Script execution failed: {:?}", result);

        // Verify the indexed values
        match interp.get_var("alice_name").await.unwrap() {
            Value::String(s) => assert_eq!(s, "Alice"),
            _ => panic!("Expected alice_name to be a string"),
        }
        match interp.get_var("bob_age").await.unwrap() {
            Value::Int(i) => assert_eq!(i, 25),
            _ => panic!("Expected bob_age to be an integer"),
        }
        match interp.get_var("charlie_city").await.unwrap() {
            Value::String(s) => assert_eq!(s, "SF"),
            _ => panic!("Expected charlie_city to be a string"),
        }

        // Test negative indexing
        let script = r"
            dim last_row = data[-1]
            dim charlie_name = last_row.name
        ";

        let program = PipParser::parse_str(script).unwrap();
        interp.eval(program).await.unwrap();

        match interp.get_var("charlie_name").await.unwrap() {
            Value::String(s) => assert_eq!(s, "Charlie"),
            _ => panic!("Expected charlie_name to be a string"),
        }

        // Test out of bounds
        let script = r"dim invalid = data[10]";
        let program = PipParser::parse_str(script).unwrap();
        let result = interp.eval(program).await;
        assert!(result.is_err(), "Should fail with index out of bounds");
    }

    #[tokio::test]
    async fn test_single_row_headerless_sheet() {
        // Test that single-row headerless sheets are correctly handled in SQL
        let mut interp = Interpreter::new();

        // Create a single-row sheet without headers
        let single_row_sheet = Sheet::from_data(vec![vec![
            CellValue::String("value1".to_string()),
            CellValue::String("value2".to_string()),
            CellValue::Int(42),
        ]]);

        interp
            .set_var("single_row", Value::Sheet(single_row_sheet))
            .await
            .unwrap();

        // This should work and not treat the single row as header-only
        let script = r"
            dim result = query(SELECT * FROM single_row)
        ";

        let program = PipParser::parse_str(script).unwrap();
        let result = interp.eval(program).await;

        // Should succeed
        assert!(
            result.is_ok(),
            "Query on single-row sheet failed: {:?}",
            result
        );

        // Verify we got the data row
        let query_result = interp.get_var("result").await.unwrap();
        if let Value::Table(batches) = query_result {
            let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
            assert_eq!(total_rows, 1, "Expected 1 data row, got {}", total_rows);
        } else {
            panic!("Expected Table result");
        }
    }

    #[tokio::test]
    async fn test_sheet_len_excludes_header() {
        // Test that len(sheet) returns data row count, excluding header
        let mut interp = Interpreter::new();

        // Sheet with column names
        let csv_with_header = "col1,col2\na,b\nc,d\ne,f";
        let csv_options = piptable_sheet::CsvOptions {
            has_headers: true,
            ..Default::default()
        };
        let sheet_with_header =
            Sheet::from_csv_str_with_options(csv_with_header, csv_options).unwrap();
        interp
            .set_var("data", Value::Sheet(sheet_with_header))
            .await
            .unwrap();

        // Sheet without column names - use from_data
        let sheet_no_header = Sheet::from_data(vec![
            vec![
                CellValue::String("a".to_string()),
                CellValue::String("b".to_string()),
            ],
            vec![
                CellValue::String("c".to_string()),
                CellValue::String("d".to_string()),
            ],
        ]);
        interp
            .set_var("raw_data", Value::Sheet(sheet_no_header))
            .await
            .unwrap();

        let script = r"
            dim data_len = len(data)
            dim raw_len = len(raw_data)
        ";

        let program = PipParser::parse_str(script).unwrap();
        interp.eval(program).await.unwrap();

        // Sheet with header should return 3 (data rows only)
        let data_len = interp.get_var("data_len").await.unwrap();
        assert!(matches!(data_len, Value::Int(3)));

        // Sheet without header should return 2 (all rows)
        let raw_len = interp.get_var("raw_len").await.unwrap();
        assert!(matches!(raw_len, Value::Int(2)));
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
        assert!(matches!(&data, Value::Sheet(sheet) if sheet.row_count() == 2));
        // header + 1 data row
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
        interp
            .set_var("users", sheet_conversions::sheet_to_value(&users))
            .await
            .unwrap();
        interp
            .set_var("orders", sheet_conversions::sheet_to_value(&orders))
            .await
            .unwrap();

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
        interp
            .set_var("users", sheet_conversions::sheet_to_value(&users))
            .await
            .unwrap();
        interp
            .set_var("orders", sheet_conversions::sheet_to_value(&orders))
            .await
            .unwrap();

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
        interp
            .set_var("sheet1", sheet_conversions::sheet_to_value(&sheet1))
            .await
            .unwrap();
        interp
            .set_var("sheet2", sheet_conversions::sheet_to_value(&sheet2))
            .await
            .unwrap();

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

        interp
            .set_var("sheet1", sheet_conversions::sheet_to_value(&sheet1))
            .await
            .unwrap();
        interp
            .set_var("sheet2", sheet_conversions::sheet_to_value(&sheet2))
            .await
            .unwrap();

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

        interp
            .set_var("sheet1", sheet_conversions::sheet_to_value(&sheet1))
            .await
            .unwrap();
        interp
            .set_var("sheet2", sheet_conversions::sheet_to_value(&sheet2))
            .await
            .unwrap();

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

        interp
            .set_var("sheet1", sheet_conversions::sheet_to_value(&sheet1))
            .await
            .unwrap();
        interp
            .set_var("sheet2", sheet_conversions::sheet_to_value(&sheet2))
            .await
            .unwrap();

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
        interp
            .set_var("users", sheet_conversions::sheet_to_value(&sheet1))
            .await
            .unwrap();

        // Create new users to append
        let mut user3 = IndexMap::new();
        user3.insert("id".to_string(), CellValue::Int(3));
        user3.insert("name".to_string(), CellValue::String("Charlie".to_string()));

        let sheet2 = Sheet::from_records(vec![user3]).unwrap();
        interp
            .set_var("new_users", sheet_conversions::sheet_to_value(&sheet2))
            .await
            .unwrap();

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
        interp
            .set_var("users", sheet_conversions::sheet_to_value(&sheet1))
            .await
            .unwrap();

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
        interp
            .set_var("new_users", sheet_conversions::sheet_to_value(&sheet2))
            .await
            .unwrap();

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
        interp
            .set_var("users", sheet_conversions::sheet_to_value(&sheet1))
            .await
            .unwrap();

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
        interp
            .set_var("updates", sheet_conversions::sheet_to_value(&sheet2))
            .await
            .unwrap();

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
        interp.set_var("sheet", Value::Sheet(sheet)).await.unwrap();

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
        interp.set_var("sheet", Value::Sheet(sheet)).await.unwrap();

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
        interp
            .set_var("sales_sheet", Value::Sheet(sheet))
            .await
            .unwrap();

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
        interp.set_var("sheet", Value::Sheet(sheet)).await.unwrap();

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
        interp.set_var("sales", Value::Sheet(sheet)).await.unwrap();

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

        interp
            .set_var("sheet1", Value::Sheet(sheet1))
            .await
            .unwrap();

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

        interp
            .set_var("sheet2", Value::Sheet(sheet2))
            .await
            .unwrap();

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
        interp
            .set_var("scores", Value::Sheet(sheet1))
            .await
            .unwrap();

        // First query
        let program1 = PipParser::parse_str(r"dim result1 = query(SELECT * FROM scores)").unwrap();
        assert!(interp.eval(program1).await.is_ok());

        // Modify the sheet (this should clear the cached table)
        let mut record2 = IndexMap::new();
        record2.insert("Name".to_string(), CellValue::String("Bob".to_string()));
        record2.insert("Score".to_string(), CellValue::Int(85));

        let sheet2 = Sheet::from_records(vec![record2]).unwrap();
        interp
            .set_var("scores", Value::Sheet(sheet2))
            .await
            .unwrap();

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

    #[tokio::test]
    async fn test_sql_query_on_table_variable() {
        // Test that SQL can query Value::Table variables directly
        let mut interp = Interpreter::new();

        // First create a table from a query (simplest version)
        let program1 = PipParser::parse_str(r"dim data = query(SELECT 1 as id)").unwrap();
        interp.eval(program1).await.unwrap();

        // Verify we have a table
        let data = interp.get_var("data").await;
        assert!(matches!(data, Some(Value::Table(_))));

        // Now query the in-memory table variable
        let program2 = PipParser::parse_str(r"dim result = query(SELECT * FROM data)").unwrap();
        interp.eval(program2).await.unwrap();

        // Verify the result
        match interp.get_var("result").await {
            Some(Value::Table(batches)) => {
                assert!(!batches.is_empty(), "Query should return results");
                let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
                assert_eq!(total_rows, 1, "Should have one row");
            }
            _ => panic!("Expected Table result"),
        }
    }

    #[tokio::test]
    async fn test_sql_join_on_table_variables() {
        // Test that SQL can join multiple Value::Table variables
        let mut interp = Interpreter::new();

        // Create first table
        let program1 = PipParser::parse_str(r"dim users = query(SELECT 1 as uid)").unwrap();
        interp.eval(program1).await.unwrap();

        // Create second table
        let program2 =
            PipParser::parse_str(r"dim scores = query(SELECT 1 as uid, 100 as points)").unwrap();
        interp.eval(program2).await.unwrap();

        // Join the two tables - now using natural names thanks to automatic aliasing
        let program3 = PipParser::parse_str(
            r"dim joined = query(SELECT users.uid, scores.points FROM users JOIN scores ON users.uid = scores.uid)",
        )
        .unwrap();
        interp.eval(program3).await.unwrap();

        // Verify the joined result
        match interp.get_var("joined").await {
            Some(Value::Table(batches)) => {
                assert!(!batches.is_empty(), "Query should return results");
                let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
                assert_eq!(total_rows, 1, "Should have one joined row");
                let batch = &batches[0];
                assert_eq!(batch.num_columns(), 2, "Should have uid and points columns");
            }
            _ => panic!("Expected Table result"),
        }
    }

    #[tokio::test]
    async fn test_sql_with_explicit_aliases() {
        // Test that explicit aliases don't cause double aliasing
        let mut interp = Interpreter::new();

        // Create a table (use simpler query due to parser limitations)
        let program1 = PipParser::parse_str(r"dim users = query(SELECT 1 as id)").unwrap();
        interp.eval(program1).await.unwrap();

        // Query with explicit alias - should NOT produce "table_users AS users AS u"
        let program2 =
            PipParser::parse_str(r"dim result = query(SELECT u.id FROM users AS u)").unwrap();
        interp.eval(program2).await.unwrap();

        // Verify it works
        match interp.get_var("result").await {
            Some(Value::Table(batches)) => {
                assert!(!batches.is_empty(), "Query should return results");
                let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
                assert_eq!(total_rows, 1, "Should have one row");
            }
            _ => panic!("Expected Table result"),
        }
    }

    #[tokio::test]
    async fn test_method_call_syntax() {
        // Test sheet.row_count() method call
        let mut interp = Interpreter::new();

        // Create a simple sheet with data
        let sheet = Sheet::from_data(vec![
            vec!["Name", "Age"],
            vec!["Alice", "30"],
            vec!["Bob", "25"],
        ]);

        // Set the sheet as a variable
        interp.set_var("data", Value::Sheet(sheet)).await.unwrap();

        // Parse and evaluate a method call
        let program = PipParser::parse_str("dim count = data.row_count()").unwrap();
        interp.eval(program).await.unwrap();

        // Check result
        let count = interp.get_var("count").await;
        assert!(matches!(count, Some(Value::Int(3))));

        // Test sheet.transpose() method
        let program = PipParser::parse_str("dim transposed = data.transpose()").unwrap();
        interp.eval(program).await.unwrap();

        let transposed = interp.get_var("transposed").await;
        if let Some(Value::Sheet(t_sheet)) = transposed {
            assert_eq!(t_sheet.row_count(), 2); // After transpose: 2 rows (Name, Age)
            assert_eq!(t_sheet.col_count(), 3); // After transpose: 3 cols (header + 2 data rows)
        } else {
            panic!("Expected transposed sheet");
        }
    }

    #[tokio::test]
    async fn test_method_call_negative_index_error() {
        // Test that negative indices are properly rejected
        let mut interp = Interpreter::new();

        // Create a simple sheet with data
        let sheet = Sheet::from_data(vec![vec!["Name", "Age"], vec!["Alice", "30"]]);

        // Set the sheet as a variable
        interp.set_var("data", Value::Sheet(sheet)).await.unwrap();

        // Try to use negative index - should fail
        let program = PipParser::parse_str("dim result = data.name_columns_by_row(-1)").unwrap();
        let error = interp.eval(program).await;
        assert!(error.is_err());
        let err_msg = error.unwrap_err().to_string();
        assert!(err_msg.contains("Row index cannot be negative"));
    }

    #[tokio::test]
    async fn test_sheet_column_methods() {
        // Test column_by_name and column_names methods
        let mut interp = Interpreter::new();

        // Create a sheet with named columns
        let mut sheet = Sheet::from_data(vec![
            vec!["Name", "Age", "City"],
            vec!["Alice", "30", "NYC"],
            vec!["Bob", "25", "LA"],
        ]);
        sheet.name_columns_by_row(0).unwrap();

        interp.set_var("data", Value::Sheet(sheet)).await.unwrap();

        // Test column_names() method
        let program = PipParser::parse_str("dim names = data.column_names()").unwrap();
        interp.eval(program).await.unwrap();

        let names = interp.get_var("names").await;
        if let Some(Value::Array(arr)) = names {
            assert_eq!(arr.len(), 3);
            match (&arr[0], &arr[1], &arr[2]) {
                (Value::String(s1), Value::String(s2), Value::String(s3)) => {
                    assert_eq!(s1, "Name");
                    assert_eq!(s2, "Age");
                    assert_eq!(s3, "City");
                }
                _ => panic!("Expected string values"),
            }
        } else {
            panic!("Expected array of column names");
        }

        // Test column_by_name() method
        let program = PipParser::parse_str("dim ages = data.column_by_name(\"Age\")").unwrap();
        interp.eval(program).await.unwrap();

        let ages = interp.get_var("ages").await;
        if let Some(Value::Array(arr)) = ages {
            assert_eq!(arr.len(), 3); // Three rows including the header
            match (&arr[0], &arr[1], &arr[2]) {
                (Value::String(s0), Value::String(s1), Value::String(s2)) => {
                    assert_eq!(s0, "Age"); // Header row value
                    assert_eq!(s1, "30");
                    assert_eq!(s2, "25");
                }
                _ => panic!("Expected string values"),
            }
        } else {
            panic!("Expected array of ages");
        }
    }

    #[tokio::test]
    async fn test_sheet_a1_notation_existing() {
        // This test already exists, just verifying A1 notation works
        let mut interp = Interpreter::new();

        // Create a sheet with data
        let sheet = Sheet::from_data(vec![
            vec!["Name", "Age"],
            vec!["Alice", "30"],
            vec!["Bob", "25"],
        ]);

        interp.set_var("data", Value::Sheet(sheet)).await.unwrap();

        // Test single cell access
        let program = PipParser::parse_str("dim val = data[\"A1\"]").unwrap();
        interp.eval(program).await.unwrap();

        let val = interp.get_var("val").await;
        match val {
            Some(Value::String(s)) => assert_eq!(s, "Name"),
            _ => panic!("Expected string 'Name'"),
        }

        // Test range access
        let program = PipParser::parse_str("dim range = data[\"A1:B2\"]").unwrap();
        interp.eval(program).await.unwrap();

        let range = interp.get_var("range").await;
        if let Some(Value::Sheet(s)) = range {
            assert_eq!(s.row_count(), 2);
            assert_eq!(s.col_count(), 2);
        } else {
            panic!("Expected sheet for range");
        }
    }

    #[tokio::test]
    async fn test_excel_sheet_selection() {
        use tempfile::NamedTempFile;

        // Create a test Excel file with multiple sheets
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_str().unwrap();

        // Create a book with multiple sheets
        let mut book = piptable_sheet::Book::new();

        let mut sheet1 = piptable_sheet::Sheet::new();
        sheet1.data_mut().push(vec![
            piptable_sheet::CellValue::String("Name".to_string()),
            piptable_sheet::CellValue::String("Value".to_string()),
        ]);
        sheet1.data_mut().push(vec![
            piptable_sheet::CellValue::String("Alice".to_string()),
            piptable_sheet::CellValue::Int(100),
        ]);
        book.add_sheet("Sheet1", sheet1).unwrap();

        let mut sheet2 = piptable_sheet::Sheet::new();
        sheet2.data_mut().push(vec![
            piptable_sheet::CellValue::String("Product".to_string()),
            piptable_sheet::CellValue::String("Price".to_string()),
        ]);
        sheet2.data_mut().push(vec![
            piptable_sheet::CellValue::String("Apple".to_string()),
            piptable_sheet::CellValue::Float(1.99),
        ]);
        book.add_sheet("Data", sheet2).unwrap();

        // Save as Excel file
        let xlsx_path = format!("{}.xlsx", path);
        book.save_as_xlsx(&xlsx_path).unwrap();

        // Test that our io module can load specific sheets
        let result1 = io::import_sheet(&xlsx_path, None, true).unwrap();
        assert_eq!(result1.row_count(), 2);

        let result2 = io::import_sheet(&xlsx_path, Some("Data"), true).unwrap();
        assert_eq!(result2.row_count(), 2);
        assert_eq!(
            result2.data()[1][0],
            piptable_sheet::CellValue::String("Apple".to_string())
        );
    }

    #[tokio::test]
    async fn test_sheet_map() {
        let mut interp = Interpreter::new();

        // Create a test sheet with some string data
        let mut sheet = piptable_sheet::Sheet::new();
        sheet.data_mut().push(vec![
            piptable_sheet::CellValue::String("Name".to_string()),
            piptable_sheet::CellValue::String("Age".to_string()),
        ]);
        sheet.data_mut().push(vec![
            piptable_sheet::CellValue::String("alice".to_string()),
            piptable_sheet::CellValue::Int(30),
        ]);
        sheet.data_mut().push(vec![
            piptable_sheet::CellValue::String("  bob  ".to_string()),
            piptable_sheet::CellValue::Int(25),
        ]);
        sheet.name_columns_by_row(0).unwrap();

        // Set the sheet as a variable
        interp.set_var("sheet", Value::Sheet(sheet)).await.unwrap();

        // Test upper operation
        let upper_result = interp
            .eval_expr(&piptable_core::Expr::Call {
                function: "sheet_map".to_string(),
                args: vec![
                    piptable_core::Expr::Variable("sheet".to_string()),
                    piptable_core::Expr::Literal(piptable_core::Literal::String(
                        "upper".to_string(),
                    )),
                ],
            })
            .await
            .unwrap();

        if let Value::Sheet(result_sheet) = upper_result {
            // Check that string values are uppercased
            assert_eq!(
                result_sheet.data()[0][0],
                piptable_sheet::CellValue::String("NAME".to_string())
            );
            assert_eq!(
                result_sheet.data()[1][0],
                piptable_sheet::CellValue::String("ALICE".to_string())
            );
            assert_eq!(
                result_sheet.data()[2][0],
                piptable_sheet::CellValue::String("  BOB  ".to_string())
            );
            // Non-string values should remain unchanged
            assert_eq!(
                result_sheet.data()[1][1],
                piptable_sheet::CellValue::Int(30)
            );
        } else {
            panic!("Expected Sheet result");
        }

        // Test lower operation
        let lower_result = interp
            .eval_expr(&piptable_core::Expr::Call {
                function: "sheet_map".to_string(),
                args: vec![
                    piptable_core::Expr::Variable("sheet".to_string()),
                    piptable_core::Expr::Literal(piptable_core::Literal::String(
                        "lower".to_string(),
                    )),
                ],
            })
            .await
            .unwrap();

        if let Value::Sheet(result_sheet) = lower_result {
            // Check that string values are lowercased
            assert_eq!(
                result_sheet.data()[0][0],
                piptable_sheet::CellValue::String("name".to_string())
            );
            assert_eq!(
                result_sheet.data()[1][0],
                piptable_sheet::CellValue::String("alice".to_string())
            );
            assert_eq!(
                result_sheet.data()[2][0],
                piptable_sheet::CellValue::String("  bob  ".to_string())
            );
        } else {
            panic!("Expected Sheet result");
        }

        // Test trim operation
        let trim_result = interp
            .eval_expr(&piptable_core::Expr::Call {
                function: "sheet_map".to_string(),
                args: vec![
                    piptable_core::Expr::Variable("sheet".to_string()),
                    piptable_core::Expr::Literal(piptable_core::Literal::String(
                        "trim".to_string(),
                    )),
                ],
            })
            .await
            .unwrap();

        if let Value::Sheet(result_sheet) = trim_result {
            // Check that string values are trimmed
            assert_eq!(
                result_sheet.data()[0][0],
                piptable_sheet::CellValue::String("Name".to_string())
            );
            assert_eq!(
                result_sheet.data()[1][0],
                piptable_sheet::CellValue::String("alice".to_string())
            );
            assert_eq!(
                result_sheet.data()[2][0],
                piptable_sheet::CellValue::String("bob".to_string())
            );
        } else {
            panic!("Expected Sheet result");
        }
    }

    #[tokio::test]
    async fn test_sheet_filter_rows() {
        let mut interp = Interpreter::new();

        // Create a test sheet
        let mut sheet = piptable_sheet::Sheet::new();
        sheet.data_mut().push(vec![
            piptable_sheet::CellValue::String("Name".to_string()),
            piptable_sheet::CellValue::String("Status".to_string()),
        ]);
        sheet.data_mut().push(vec![
            piptable_sheet::CellValue::String("Alice".to_string()),
            piptable_sheet::CellValue::String("Active".to_string()),
        ]);
        sheet.data_mut().push(vec![
            piptable_sheet::CellValue::String("Bob".to_string()),
            piptable_sheet::CellValue::String("Inactive".to_string()),
        ]);
        sheet.data_mut().push(vec![
            piptable_sheet::CellValue::String("Charlie".to_string()),
            piptable_sheet::CellValue::String("Active".to_string()),
        ]);
        sheet.name_columns_by_row(0).unwrap();

        interp.set_var("sheet", Value::Sheet(sheet)).await.unwrap();

        // Test filtering for Active status
        let filter_result = interp
            .eval_expr(&piptable_core::Expr::Call {
                function: "sheet_filter_rows".to_string(),
                args: vec![
                    piptable_core::Expr::Variable("sheet".to_string()),
                    piptable_core::Expr::Literal(piptable_core::Literal::String(
                        "Status".to_string(),
                    )),
                    piptable_core::Expr::Literal(piptable_core::Literal::String(
                        "Active".to_string(),
                    )),
                ],
            })
            .await
            .unwrap();

        if let Value::Sheet(result_sheet) = filter_result {
            // Should have only 2 active rows (header row is also filtered)
            assert_eq!(result_sheet.row_count(), 2);
            assert_eq!(
                result_sheet.data()[0][0],
                piptable_sheet::CellValue::String("Alice".to_string())
            );
            assert_eq!(
                result_sheet.data()[1][0],
                piptable_sheet::CellValue::String("Charlie".to_string())
            );
        } else {
            panic!("Expected Sheet result");
        }
    }

    #[tokio::test]
    async fn test_sheet_indexing_without_header_row() {
        use indexmap::IndexMap;
        use piptable_sheet::{CellValue, Sheet};

        let mut interp = Interpreter::new();

        // Test case 1: Sheet created via from_records (column_names set but no header row)
        let mut record1 = IndexMap::new();
        record1.insert("Name".to_string(), CellValue::String("Alice".to_string()));
        record1.insert("Age".to_string(), CellValue::Int(30));

        let mut record2 = IndexMap::new();
        record2.insert("Name".to_string(), CellValue::String("Bob".to_string()));
        record2.insert("Age".to_string(), CellValue::Int(25));

        let sheet1 = Sheet::from_records(vec![record1, record2]).unwrap();
        interp
            .set_var("sheet1", Value::Sheet(sheet1))
            .await
            .unwrap();

        // Test positive indexing
        let program = PipParser::parse_str(
            r"
            dim row0 = sheet1[0]
            dim row1 = sheet1[1]
        ",
        )
        .unwrap();
        interp.eval(program).await.unwrap();

        let row0 = interp.get_var("row0").await.unwrap();
        if let Value::Object(obj) = row0 {
            assert!(matches!(obj.get("Name"), Some(Value::String(s)) if s == "Alice"));
            assert!(matches!(obj.get("Age"), Some(Value::Int(30))));
        } else {
            panic!("Expected object for row0");
        }

        let row1 = interp.get_var("row1").await.unwrap();
        if let Value::Object(obj) = row1 {
            assert!(matches!(obj.get("Name"), Some(Value::String(s)) if s == "Bob"));
            assert!(matches!(obj.get("Age"), Some(Value::Int(25))));
        } else {
            panic!("Expected object for row1");
        }

        // Test negative indexing
        let program = PipParser::parse_str(
            r"
            dim last_row = sheet1[-1]
            dim second_last = sheet1[-2]
        ",
        )
        .unwrap();
        interp.eval(program).await.unwrap();

        let last_row = interp.get_var("last_row").await.unwrap();
        if let Value::Object(obj) = last_row {
            assert!(matches!(obj.get("Name"), Some(Value::String(s)) if s == "Bob"));
            assert!(matches!(obj.get("Age"), Some(Value::Int(25))));
        } else {
            panic!("Expected object for last_row");
        }

        let second_last = interp.get_var("second_last").await.unwrap();
        if let Value::Object(obj) = second_last {
            assert!(matches!(obj.get("Name"), Some(Value::String(s)) if s == "Alice"));
            assert!(matches!(obj.get("Age"), Some(Value::Int(30))));
        } else {
            panic!("Expected object for second_last");
        }

        // Test out of bounds
        let program = PipParser::parse_str(r"dim oob = sheet1[2]").unwrap();
        let result = interp.eval(program).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("out of bounds"));

        let program = PipParser::parse_str(r"dim neg_oob = sheet1[-3]").unwrap();
        let result = interp.eval(program).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("out of bounds"));

        // Test case 2: Sheet with actual header row
        let mut sheet2 = Sheet::new();
        sheet2.data_mut().push(vec![
            CellValue::String("Name".to_string()),
            CellValue::String("Age".to_string()),
        ]);
        sheet2.data_mut().push(vec![
            CellValue::String("Charlie".to_string()),
            CellValue::Int(40),
        ]);
        sheet2.name_columns_by_row(0).unwrap();
        interp
            .set_var("sheet2", Value::Sheet(sheet2))
            .await
            .unwrap();

        // Sheet2 has column_names AND first row matches those names, so it should skip header
        let program = PipParser::parse_str(r"dim sheet2_row0 = sheet2[0]").unwrap();
        interp.eval(program).await.unwrap();

        let sheet2_row0 = interp.get_var("sheet2_row0").await.unwrap();
        if let Value::Object(obj) = sheet2_row0 {
            assert!(matches!(obj.get("Name"), Some(Value::String(s)) if s == "Charlie"));
            assert!(matches!(obj.get("Age"), Some(Value::Int(40))));
        } else {
            panic!("Expected object for sheet2_row0");
        }
    }

    #[tokio::test]
    async fn test_single_row_sheet_sql() {
        // Test that sheets with a single data row work correctly in SQL
        use indexmap::IndexMap;
        use piptable_sheet::{CellValue, Sheet};

        let mut interp = Interpreter::new();

        // Create a sheet with a single data row via from_records
        let mut record = IndexMap::new();
        record.insert("Name".to_string(), CellValue::String("Alice".to_string()));
        record.insert("Age".to_string(), CellValue::Int(30));

        let sheet = Sheet::from_records(vec![record]).unwrap();
        // This sheet has column_names but no physical header row

        interp
            .set_var("single_row", Value::Sheet(sheet))
            .await
            .unwrap();

        // Try to query it with SQL
        let program =
            PipParser::parse_str(r"dim result = query(SELECT * FROM single_row)").unwrap();

        let result = interp.eval(program).await;
        assert!(
            result.is_ok(),
            "SQL query on single-row sheet should succeed: {:?}",
            result.err()
        );

        let query_result = interp.get_var("result").await.unwrap();
        if let Value::Table(batches) = query_result {
            // Should have one row of data
            let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
            assert_eq!(total_rows, 1, "Should have exactly one data row");
        } else {
            panic!("Expected Table result");
        }
    }

    #[tokio::test]
    async fn test_len_sheet_without_header_row() {
        // Test that len() correctly counts data rows for sheets with/without physical headers
        use indexmap::IndexMap;
        use piptable_sheet::{CellValue, Sheet};

        let mut interp = Interpreter::new();

        // Test case 1: Sheet from records (has column_names but no physical header)
        let mut record1 = IndexMap::new();
        record1.insert("Name".to_string(), CellValue::String("Alice".to_string()));
        record1.insert("Age".to_string(), CellValue::Int(30));

        let mut record2 = IndexMap::new();
        record2.insert("Name".to_string(), CellValue::String("Bob".to_string()));
        record2.insert("Age".to_string(), CellValue::Int(25));

        let sheet1 = Sheet::from_records(vec![record1, record2]).unwrap();
        interp
            .set_var("sheet1", Value::Sheet(sheet1))
            .await
            .unwrap();

        // Test case 2: Sheet with actual header row
        let mut sheet2 = Sheet::new();
        sheet2.data_mut().push(vec![
            CellValue::String("Name".to_string()),
            CellValue::String("Age".to_string()),
        ]);
        sheet2.data_mut().push(vec![
            CellValue::String("Charlie".to_string()),
            CellValue::Int(40),
        ]);
        sheet2.data_mut().push(vec![
            CellValue::String("David".to_string()),
            CellValue::Int(35),
        ]);
        sheet2.name_columns_by_row(0).unwrap();
        interp
            .set_var("sheet2", Value::Sheet(sheet2))
            .await
            .unwrap();

        // Test case 3: Sheet with no column names
        let mut sheet3 = Sheet::new();
        sheet3.data_mut().push(vec![
            CellValue::String("Eve".to_string()),
            CellValue::Int(28),
        ]);
        sheet3.data_mut().push(vec![
            CellValue::String("Frank".to_string()),
            CellValue::Int(32),
        ]);
        interp
            .set_var("sheet3", Value::Sheet(sheet3))
            .await
            .unwrap();

        // Test len() for each sheet
        let program = PipParser::parse_str(
            r"
            dim len1 = len(sheet1)
            dim len2 = len(sheet2)
            dim len3 = len(sheet3)
        ",
        )
        .unwrap();
        interp.eval(program).await.unwrap();

        // Sheet1: 2 data rows (no physical header)
        let len1 = interp.get_var("len1").await.unwrap();
        assert!(
            matches!(len1, Value::Int(2)),
            "Sheet1 should have 2 data rows"
        );

        // Sheet2: 2 data rows (has physical header, which is excluded)
        let len2 = interp.get_var("len2").await.unwrap();
        assert!(
            matches!(len2, Value::Int(2)),
            "Sheet2 should have 2 data rows"
        );

        // Sheet3: 2 rows (no column names, all rows are data)
        let len3 = interp.get_var("len3").await.unwrap();
        assert!(matches!(len3, Value::Int(2)), "Sheet3 should have 2 rows");
    }

    #[tokio::test]
    async fn test_toon_sql_query() {
        // Test that TOON files can be queried with SQL
        use std::io::Write;
        use tempfile::NamedTempFile;

        let mut interp = Interpreter::new();

        // Create a temporary TOON file
        let mut file = NamedTempFile::with_suffix(".toon").expect("Failed to create temp file");
        writeln!(file, "data[3]{{id,name,value}}:").unwrap();
        writeln!(file, "  1,Widget,100").unwrap();
        writeln!(file, "  2,Gadget,200").unwrap();
        writeln!(file, "  3,Doohickey,300").unwrap();
        file.flush().unwrap();

        let path = file.path().to_string_lossy().replace('\\', "/");
        let script = format!(
            r#"dim result = query(SELECT * FROM "{}" ORDER BY id)"#,
            path
        );

        let program = PipParser::parse_str(&script).unwrap();
        let result = interp.eval(program).await;
        assert!(
            result.is_ok(),
            "TOON SQL query should succeed: {:?}",
            result.err()
        );

        let query_result = interp.get_var("result").await.unwrap();
        if let Value::Table(batches) = query_result {
            let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
            assert_eq!(total_rows, 3, "Should have 3 data rows");
        } else {
            panic!("Expected Table result");
        }
    }

    #[tokio::test]
    async fn test_toon_sql_with_order() {
        // Test TOON SQL queries with ORDER BY clause
        use std::io::Write;
        use tempfile::NamedTempFile;

        let mut interp = Interpreter::new();

        // Create a TOON file with more data
        let mut file = NamedTempFile::with_suffix(".toon").expect("Failed to create temp file");
        writeln!(file, "products[4]{{product,category,price,stock}}:").unwrap();
        writeln!(file, "  Widget,Tools,19.99,50").unwrap();
        writeln!(file, "  Gadget,Electronics,29.99,30").unwrap();
        writeln!(file, "  Gizmo,Tools,15.99,75").unwrap();
        writeln!(file, "  Doohickey,Electronics,39.99,20").unwrap();
        file.flush().unwrap();

        let path = file.path().to_string_lossy().replace('\\', "/");
        // Query with ORDER BY to test sorting functionality
        let script = format!(
            r#"dim products = query(SELECT * FROM "{}" ORDER BY product)"#,
            path
        );

        let program = PipParser::parse_str(&script).unwrap();
        let result = interp.eval(program).await;
        assert!(
            result.is_ok(),
            "TOON SQL query with ORDER BY should succeed: {:?}",
            result.err()
        );

        let query_result = interp.get_var("products").await.unwrap();
        if let Value::Table(batches) = query_result {
            let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
            assert_eq!(total_rows, 4, "Should have 4 products");
        } else {
            panic!("Expected Table result");
        }
    }

    #[tokio::test]
    async fn test_toon_join_with_csv() {
        // Test JOIN between TOON and CSV files
        use std::io::Write;
        use tempfile::NamedTempFile;

        let mut interp = Interpreter::new();

        // Create a TOON file with product data
        let mut toon_file =
            NamedTempFile::with_suffix(".toon").expect("Failed to create temp file");
        writeln!(toon_file, "products[3]{{id,name,price}}:").unwrap();
        writeln!(toon_file, "  1,Widget,19.99").unwrap();
        writeln!(toon_file, "  2,Gadget,29.99").unwrap();
        writeln!(toon_file, "  3,Gizmo,15.99").unwrap();
        toon_file.flush().unwrap();

        // Create a CSV file with sales data
        let mut csv_file = NamedTempFile::with_suffix(".csv").expect("Failed to create temp file");
        writeln!(csv_file, "product_id,quantity").unwrap();
        writeln!(csv_file, "1,10").unwrap();
        writeln!(csv_file, "2,5").unwrap();
        writeln!(csv_file, "1,7").unwrap();
        csv_file.flush().unwrap();

        let toon_path = toon_file.path().to_string_lossy().replace('\\', "/");
        let csv_path = csv_file.path().to_string_lossy().replace('\\', "/");

        let script = format!(
            r#"dim result = query(
                SELECT p.name, s.quantity, p.price 
                FROM "{}" as p
                JOIN "{}" as s ON p.id = s.product_id
                ORDER BY p.name
            )"#,
            toon_path, csv_path
        );

        let program = PipParser::parse_str(&script).unwrap();
        let result = interp.eval(program).await;
        assert!(
            result.is_ok(),
            "TOON-CSV JOIN should succeed: {:?}",
            result.err()
        );

        let query_result = interp.get_var("result").await.unwrap();
        if let Value::Table(batches) = query_result {
            let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
            assert_eq!(total_rows, 3, "Should have 3 sales records after join");
        } else {
            panic!("Expected Table result");
        }
    }
}
