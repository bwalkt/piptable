//! # piptable-core
//!
//! Core types and utilities for the piptable DSL.
//!
//! This crate provides:
//! - AST node definitions
//! - Value types for runtime
//! - Error types
//! - Common utilities

/// WASM boundary types and helpers.
pub mod boundary;
/// Error types and result aliases.
pub mod error;
/// Runtime value types.
pub mod value;

/// Re-export AST types from `piptable-types`.
pub use piptable_types::*;

/// Re-export core error types.
pub use error::{PipError, PipResult};
/// Re-export the runtime value type.
pub use value::Value;
