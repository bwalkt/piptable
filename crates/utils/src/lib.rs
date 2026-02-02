//! # Piptable Utils
//!
//! Utility functions for spreadsheet operations including number formatting,
//! date/time handling, and error utilities.

use piptable_formatting::ssf_format;
use piptable_primitives::Value;

pub mod address;
pub mod cell_data;
pub mod datetime;
pub mod formatting;

pub use address::*;
pub use cell_data::*;

/// Format a value for display
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

/// Parse a value from a string
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

/// Convert column index to letter (0 -> A, 1 -> B, 25 -> Z, 26 -> AA, etc.)
pub fn column_index_to_letter(index: u32) -> String {
    let mut result = String::new();
    let mut n = index;

    loop {
        let remainder = n % 26;
        result.push((b'A' + remainder as u8) as char);
        n /= 26;

        if n == 0 {
            break;
        }
        n -= 1; // Adjust for 1-based indexing
    }

    result.chars().rev().collect()
}

/// Convert column letter to index (A -> 0, B -> 1, Z -> 25, AA -> 26, etc.)
pub fn column_letter_to_index(s: &str) -> Result<u32, String> {
    if s.is_empty() {
        return Err("Empty column".to_string());
    }

    let mut result = 0u32;
    for c in s.chars() {
        if !c.is_ascii_uppercase() {
            return Err(format!("Invalid column character: {}", c));
        }
        result = result * 26 + (c as u32 - 'A' as u32 + 1);
    }

    Ok(result - 1) // Convert to 0-based
}

#[cfg(test)]
mod tests {
    use super::*;

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
