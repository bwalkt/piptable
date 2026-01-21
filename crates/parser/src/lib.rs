//! # piptable-parser
//!
//! Parser for the piptable DSL combining VBA-like syntax with SQL.
//!
//! This crate uses [pest](https://pest.rs) for parsing.

use pest_derive::Parser;
use piptable_core::{PipResult, Program};

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct PiptableParser;

/// Main parser interface.
pub struct PipParser;

impl PipParser {
    /// Parse a piptable script string into an AST.
    ///
    /// # Errors
    ///
    /// Returns a `PipError::Parse` if the input is invalid.
    pub fn parse_str(_input: &str) -> PipResult<Program> {
        // TODO: Implement parsing logic
        Ok(Program::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty() {
        let result = PipParser::parse_str("");
        assert!(result.is_ok());
    }
}
