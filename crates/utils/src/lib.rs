//! # Piptable Utils
//!
//! Utility functions for spreadsheet operations including number formatting,
//! date/time handling, and error utilities.

use piptable_formatting::ssf_format;
use piptable_primitives::Value;

pub mod cell_data;
pub mod datetime;
pub mod formatting;
pub mod math;

pub use cell_data::*;
pub use math::*;
pub use piptable_primitives::address::*;

/// Format a value for display, honoring an optional SSF-style format string.
pub fn format_value(value: &Value, format: Option<&str>) -> String {
    match value {
        Value::Empty => String::new(),
        Value::Bool(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
        Value::Int(n) => format_number_int(*n, format),
        Value::Float(f) => format_number_float(*f, format),
        Value::String(s) => s.clone(),
        Value::Error(e) => format!("#{:?}!", e),
        Value::Array(arr) => format!("[{} items]", arr.len()),
    }
}

/// Format an integer with optional format string
fn format_number_int(n: i64, format: Option<&str>) -> String {
    if let Some(fmt) = format {
        ssf_format(fmt, &Value::Int(n), None)
    } else {
        n.to_string()
    }
}

/// Format a float with optional format string
fn format_number_float(f: f64, format: Option<&str>) -> String {
    if let Some(fmt) = format {
        ssf_format(fmt, &Value::Float(f), None)
    } else {
        f.to_string()
    }
}

/// Parse a value from a string with basic type inference.
pub fn parse_value(s: &str) -> Value {
    // Empty string
    if s.is_empty() {
        return Value::Empty;
    }

    // Boolean
    if s.eq_ignore_ascii_case("true") {
        return Value::Bool(true);
    }
    if s.eq_ignore_ascii_case("false") {
        return Value::Bool(false);
    }

    // Try parsing as number
    if let Ok(n) = s.parse::<i64>() {
        return Value::Int(n);
    }
    if let Ok(f) = s.parse::<f64>() {
        return Value::Float(f);
    }

    // Formula (starts with =)
    if s.starts_with('=') {
        // This would be handled by formula parser
        // For now, treat as string
        return Value::String(s.to_string());
    }

    // Default to string
    Value::String(s.to_string())
}

/// Utility tests.
#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies column name/number conversions.
    #[test]
    fn test_column_conversions() {
        assert_eq!(column_index_to_letter(0), "A");
        assert_eq!(column_index_to_letter(1), "B");
        assert_eq!(column_index_to_letter(25), "Z");
        assert_eq!(column_index_to_letter(26), "AA");
        assert_eq!(column_index_to_letter(27), "AB");
        assert_eq!(column_index_to_letter(701), "ZZ");
        assert_eq!(column_index_to_letter(702), "AAA");

        assert_eq!(column_letter_to_index("A").unwrap(), 0);
        assert_eq!(column_letter_to_index("B").unwrap(), 1);
        assert_eq!(column_letter_to_index("Z").unwrap(), 25);
        assert_eq!(column_letter_to_index("AA").unwrap(), 26);
        assert_eq!(column_letter_to_index("AB").unwrap(), 27);
        assert_eq!(column_letter_to_index("ZZ").unwrap(), 701);
        assert_eq!(column_letter_to_index("AAA").unwrap(), 702);
    }

    /// Verifies value parsing helpers.
    #[test]
    fn test_parse_value() {
        assert_eq!(parse_value(""), Value::Empty);
        assert_eq!(parse_value("true"), Value::Bool(true));
        assert_eq!(parse_value("FALSE"), Value::Bool(false));
        assert_eq!(parse_value("123"), Value::Int(123));
        assert_eq!(parse_value("-456"), Value::Int(-456));
        assert_eq!(parse_value("3.14"), Value::Float(3.14));
        assert_eq!(parse_value("hello"), Value::String("hello".to_string()));
        assert_eq!(parse_value("=A1+B2"), Value::String("=A1+B2".to_string()));
    }
}
