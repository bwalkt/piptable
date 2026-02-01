//! # piptable-core
//!
//! Core types and utilities for the piptable DSL.
//!
//! This crate provides:
//! - AST node definitions
//! - Value types for runtime
//! - Error types
//! - Common utilities

pub mod error;
pub mod value;

// Re-export AST types from piptable-types
pub use piptable_types::*;

pub use error::{PipError, PipResult};
pub use value::Value;
