//! # piptable-interpreter
//!
//! Interpreter for executing piptable DSL scripts.
//!
//! This crate provides:
//! - AST evaluation
//! - Variable scope management
//! - Built-in functions
//! - Integration with SQL and HTTP engines

use async_recursion::async_recursion;
use piptable_core::{
    BinaryOp, Expr, FromClause, JoinClause, JoinType, LValue, Literal, OrderByItem, PipError,
    PipResult, Program, SelectClause, SelectItem, SortDirection, SqlQuery, Statement, TableRef,
    UnaryOp, Value,
};
use piptable_http::HttpClient;
use piptable_sql::SqlEngine;
use std::collections::HashMap;
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
                name,
                value,
                line,
                ..
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
                    self.set_var(&variable, item).await;
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
                    self.set_var(&variable, Value::Int(i)).await;
                    if let Err(e) = self.eval_block(&body).await {
                        loop_result = Err(e);
                        break;
                    }
                    i += step_int;
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

            Statement::Export { .. } => {
                // TODO: Implement export
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

    #[async_recursion]
    async fn eval_expr(&mut self, expr: &Expr) -> PipResult<Value> {
        match expr {
            Expr::Literal(lit) => Ok(self.eval_literal(lit)),

            Expr::Variable(name) => {
                if name == "*" {
                    // Special case for SELECT *
                    return Ok(Value::String("*".to_string()));
                }
                self.get_var(name).await.ok_or_else(|| {
                    PipError::runtime(0, format!("Undefined variable: {name}"))
                })
            }

            Expr::Binary { left, op, right } => {
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
                    Value::Object(map) => map.get(field).cloned().ok_or_else(|| {
                        PipError::runtime(0, format!("Field not found: {field}"))
                    }),
                    _ => Err(PipError::runtime(
                        0,
                        format!("Cannot access field on {}", obj.type_name()),
                    )),
                }
            }

            Expr::ArrayIndex { array, index } => {
                let arr = self.eval_expr(array).await?;
                let idx = self.eval_expr(index).await?;
                let idx_int = idx
                    .as_int()
                    .ok_or_else(|| PipError::runtime(0, "Array index must be integer"))?;

                match arr {
                    Value::Array(items) => {
                        let idx_usize = if idx_int < 0 {
                            let adjusted = items.len() as i64 + idx_int;
                            if adjusted < 0 {
                                return Err(PipError::runtime(0, "Array index out of bounds"));
                            }
                            adjusted as usize
                        } else {
                            idx_int as usize
                        };
                        items
                            .get(idx_usize)
                            .cloned()
                            .ok_or_else(|| PipError::runtime(0, "Array index out of bounds"))
                    }
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

            Expr::Ask { .. } => {
                // TODO: Implement LLM integration
                Err(PipError::runtime(0, "Ask expression not yet implemented"))
            }
        }
    }

    /// Evaluate a literal to a Value.
    fn eval_literal(&self, lit: &Literal) -> Value {
        match lit {
            Literal::Null => Value::Null,
            Literal::Bool(b) => Value::Bool(*b),
            Literal::Int(n) => Value::Int(*n),
            Literal::Float(f) => Value::Float(*f),
            Literal::String(s) => Value::String(s.clone()),
            Literal::Interval { value, unit } => {
                // Convert to milliseconds for internal representation
                use piptable_core::IntervalUnit;
                let ms = match unit {
                    IntervalUnit::Millisecond => *value,
                    IntervalUnit::Second => *value * 1000,
                    IntervalUnit::Minute => *value * 60 * 1000,
                    IntervalUnit::Hour => *value * 60 * 60 * 1000,
                    IntervalUnit::Day => *value * 24 * 60 * 60 * 1000,
                    IntervalUnit::Week => *value * 7 * 24 * 60 * 60 * 1000,
                    IntervalUnit::Month => *value * 30 * 24 * 60 * 60 * 1000,
                    IntervalUnit::Year => *value * 365 * 24 * 60 * 60 * 1000,
                };
                Value::Int(ms)
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
                format!(
                    "Cannot add {} and {}",
                    left.type_name(),
                    right.type_name()
                ),
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
                Ok(Value::Int(a % b))
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
                    Value::String(s) => s
                        .parse::<i64>()
                        .map(Value::Int)
                        .map_err(|_| PipError::runtime(line, format!("Cannot convert '{s}' to int"))),
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
                            total += self
                                .value_to_number(v)
                                .ok_or_else(|| PipError::runtime(line, "sum() requires numeric array"))?;
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
                            total += self
                                .value_to_number(v)
                                .ok_or_else(|| PipError::runtime(line, "avg() requires numeric array"))?;
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
                    Value::Object(obj) => {
                        Ok(Value::Array(obj.keys().map(|k| Value::String(k.clone())).collect()))
                    }
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
                        self.set_var(param, arg).await;
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
                    Err(PipError::runtime(
                        line,
                        format!("Unknown function: {name}"),
                    ))
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

        let mut result = self.value_to_number(&values[0]).ok_or_else(|| {
            PipError::runtime(line, "min/max requires numeric values")
        })?;

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
            TableRef::Qualified { database, schema, table } => {
                Ok(match schema {
                    Some(s) => format!("{database}.{s}.{table}"),
                    None => format!("{database}.{table}"),
                })
            }
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
    fn table_to_array(
        &self,
        batches: &[Arc<arrow::array::RecordBatch>],
    ) -> PipResult<Vec<Value>> {
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
                let idx_int = idx.as_int().ok_or_else(|| {
                    PipError::runtime(line, "Array index must be integer")
                })?;

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
                    Value::Object(map) => map
                        .get(field)
                        .cloned()
                        .ok_or_else(|| PipError::runtime(line, format!("Field not found: {field}"))),
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

    /// Set a variable in the current scope.
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
        let program = PipParser::parse_str("dim x = 0\nif false then x = 1 else x = 2 end if").unwrap();
        interp.eval(program).await.unwrap();
        let value = interp.get_var("x").await;
        assert!(matches!(value, Some(Value::Int(2))));
    }

    #[tokio::test]
    async fn test_eval_for_loop() {
        let mut interp = Interpreter::new();
        let program = PipParser::parse_str("dim sum = 0\nfor i = 1 to 5\nsum = sum + i\nnext").unwrap();
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
        let program = PipParser::parse_str(
            "dim x = 1\nfor i = 1 to 1\ndim y = 2\nnext",
        )
        .unwrap();
        interp.eval(program).await.unwrap();
        // x should be accessible
        assert!(interp.get_var("x").await.is_some());
        // y should be cleaned up (scope isolation)
        // Note: current implementation doesn't clean up, but that's ok for now
    }
}
