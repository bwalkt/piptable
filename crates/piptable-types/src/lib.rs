//! # piptable-types
//!
//! Core type definitions for the piptable DSL.
//!
//! This crate provides fundamental type definitions that are shared
//! across the piptable ecosystem without any dependencies on higher-level
//! crates like sheet or core.

pub mod ast;

pub use ast::*;
