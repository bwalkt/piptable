//! # piptable-core
//!
//! Core types and utilities for the piptable DSL.
//!
//! This crate provides:
//! - AST node definitions
//! - Value types for runtime
//! - Error types
//! - Common utilities

pub mod ast;
pub mod error;
pub mod value;

pub use ast::*;
pub use error::{PipError, PipResult};
pub use value::Value;
