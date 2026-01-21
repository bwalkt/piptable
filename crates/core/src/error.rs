//! Error types for piptable.

use thiserror::Error;

/// Result type for piptable operations.
pub type PipResult<T> = Result<T, PipError>;

/// Errors that can occur in piptable.
#[derive(Debug, Error)]
pub enum PipError {
    /// Parse error with location information.
    #[error("Parse error at line {line}, column {column}: {message}")]
    Parse {
        line: usize,
        column: usize,
        message: String,
    },

    /// Runtime error during script execution.
    #[error("Runtime error at line {line}: {message}")]
    Runtime { line: usize, message: String },

    /// Type error when operations are applied to incompatible types.
    #[error("Type error: expected {expected}, got {got}")]
    Type { expected: String, got: String },

    /// Undefined variable error.
    #[error("Undefined variable: {0}")]
    UndefinedVariable(String),

    /// Undefined function error.
    #[error("Undefined function: {0}")]
    UndefinedFunction(String),

    /// SQL execution error.
    #[error("SQL error: {0}")]
    Sql(String),

    /// HTTP request error.
    #[error("HTTP error: {0}")]
    Http(String),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Export error.
    #[error("Export error: {0}")]
    Export(String),

    /// Plugin error.
    #[error("Plugin error in {plugin}: {message}")]
    Plugin { plugin: String, message: String },

    /// Internal error (should not happen).
    #[error("Internal error: {0}")]
    Internal(String),
}

impl PipError {
    /// Create a parse error.
    pub fn parse(line: usize, column: usize, message: impl Into<String>) -> Self {
        Self::Parse {
            line,
            column,
            message: message.into(),
        }
    }

    /// Create a runtime error.
    pub fn runtime(line: usize, message: impl Into<String>) -> Self {
        Self::Runtime {
            line,
            message: message.into(),
        }
    }

    /// Create a type error.
    pub fn type_error(expected: impl Into<String>, got: impl Into<String>) -> Self {
        Self::Type {
            expected: expected.into(),
            got: got.into(),
        }
    }
}
