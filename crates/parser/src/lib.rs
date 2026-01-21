//! # piptable-parser
//!
//! Parser for the piptable DSL combining VBA-like syntax with SQL.
//!
//! This crate uses [pest](https://pest.rs) for parsing.

use pest::Parser;
use pest_derive::Parser as PestParser;
use piptable_core::{PipError, PipResult, Program};

#[derive(PestParser)]
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
    pub fn parse_str(input: &str) -> PipResult<Program> {
        // Validate syntax against grammar
        PiptableParser::parse(Rule::program, input)
            .map_err(|e| PipError::parse(0, 0, e.to_string()))?;

        // TODO: Build AST from pest pairs
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

    #[test]
    fn test_parse_invalid_syntax() {
        let result = PipParser::parse_str("!@#$%^ invalid");
        assert!(result.is_err());
    }
}
