//! # piptable-interpreter
//!
//! Interpreter for executing piptable DSL scripts.
//!
//! This crate provides:
//! - AST evaluation
//! - Variable scope management
//! - Built-in functions
//! - Integration with SQL and HTTP engines

use piptable_core::{PipError, PipResult, Program, Statement, Value};
use piptable_http::HttpClient;
use piptable_sql::SqlEngine;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Interpreter for piptable scripts.
pub struct Interpreter {
    /// Variable storage
    variables: Arc<RwLock<HashMap<String, Value>>>,
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
            variables: Arc::new(RwLock::new(HashMap::new())),
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
            result = self.eval_statement(statement).await?;
        }

        Ok(result)
    }

    /// Execute a single statement.
    ///
    /// # Errors
    ///
    /// Returns error if statement execution fails.
    pub async fn eval_statement(&mut self, _statement: Statement) -> PipResult<Value> {
        // TODO: Implement statement evaluation
        Ok(Value::Null)
    }

    /// Set a variable.
    pub async fn set_var(&self, name: &str, value: Value) {
        let mut vars = self.variables.write().await;
        vars.insert(name.to_string(), value);
    }

    /// Get a variable.
    pub async fn get_var(&self, name: &str) -> Option<Value> {
        let vars = self.variables.read().await;
        vars.get(name).cloned()
    }

    /// Get output buffer contents.
    pub async fn output(&self) -> Vec<String> {
        let output = self.output.read().await;
        output.clone()
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
}
