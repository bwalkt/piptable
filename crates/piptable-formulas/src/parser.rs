//! Formula parser module

use crate::{FormulaError, FormulaExpr};

/// Parse a formula string into an AST
pub fn parse_formula(formula: &str) -> Result<FormulaExpr, FormulaError> {
    // Remove leading '=' if present
    let formula = formula.strip_prefix('=').unwrap_or(formula);

    // TODO: Implement full formula parsing
    // For now, just a placeholder
    if formula.is_empty() {
        return Err(FormulaError::ParseError("Empty formula".to_string()));
    }

    // Placeholder: return a literal for now
    Ok(FormulaExpr::Literal(piptable_primitives::Value::String(
        formula.to_string(),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty() {
        assert!(parse_formula("").is_err());
        assert!(parse_formula("=").is_err());
    }
}
