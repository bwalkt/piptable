use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents a cell value in a sheet
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CellValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

impl CellValue {
    /// Check if the value is null
    #[must_use]
    pub fn is_null(&self) -> bool {
        matches!(self, CellValue::Null)
    }

    /// Try to get the value as a boolean
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            CellValue::Bool(b) => Some(*b),
            CellValue::Int(i) => Some(*i != 0),
            CellValue::Float(f) => Some(*f != 0.0),
            CellValue::String(s) => s.parse().ok(),
            CellValue::Null => None,
        }
    }

    /// Try to get the value as an integer
    #[must_use]
    pub fn as_int(&self) -> Option<i64> {
        match self {
            CellValue::Int(i) => Some(*i),
            CellValue::Float(f) => Some(*f as i64),
            CellValue::Bool(b) => Some(i64::from(*b)),
            CellValue::String(s) => s.parse().ok(),
            CellValue::Null => None,
        }
    }

    /// Try to get the value as a float
    #[must_use]
    pub fn as_float(&self) -> Option<f64> {
        match self {
            CellValue::Float(f) => Some(*f),
            CellValue::Int(i) => Some(*i as f64),
            CellValue::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            CellValue::String(s) => s.parse().ok(),
            CellValue::Null => None,
        }
    }

    /// Get the value as a string
    #[must_use]
    pub fn as_str(&self) -> String {
        match self {
            CellValue::Null => String::new(),
            CellValue::Bool(b) => b.to_string(),
            CellValue::Int(i) => i.to_string(),
            CellValue::Float(f) => f.to_string(),
            CellValue::String(s) => s.clone(),
        }
    }

    /// Parse a string into a `CellValue` with type inference
    /// Tries: null -> bool -> int -> float -> string
    #[must_use]
    pub fn parse(s: &str) -> CellValue {
        let trimmed = s.trim();

        // Check for null/empty
        if trimmed.is_empty() {
            return CellValue::Null;
        }

        // Check for boolean (note: "1"/"0" are parsed as Int, not Bool)
        match trimmed.to_lowercase().as_str() {
            "true" | "yes" => return CellValue::Bool(true),
            "false" | "no" => return CellValue::Bool(false),
            _ => {}
        }

        // Check for integer
        if let Ok(i) = trimmed.parse::<i64>() {
            return CellValue::Int(i);
        }

        // Check for float
        if let Ok(f) = trimmed.parse::<f64>() {
            return CellValue::Float(f);
        }

        // Default to string
        CellValue::String(s.to_string())
    }
}

impl Default for CellValue {
    fn default() -> Self {
        CellValue::Null
    }
}

impl fmt::Display for CellValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CellValue::Null => write!(f, ""),
            CellValue::Bool(b) => write!(f, "{b}"),
            CellValue::Int(i) => write!(f, "{i}"),
            CellValue::Float(fl) => write!(f, "{fl}"),
            CellValue::String(s) => write!(f, "{s}"),
        }
    }
}

impl From<bool> for CellValue {
    fn from(b: bool) -> Self {
        CellValue::Bool(b)
    }
}

impl From<i64> for CellValue {
    fn from(i: i64) -> Self {
        CellValue::Int(i)
    }
}

impl From<i32> for CellValue {
    fn from(i: i32) -> Self {
        CellValue::Int(i64::from(i))
    }
}

impl From<f64> for CellValue {
    fn from(f: f64) -> Self {
        CellValue::Float(f)
    }
}

impl From<f32> for CellValue {
    fn from(f: f32) -> Self {
        CellValue::Float(f64::from(f))
    }
}

impl From<String> for CellValue {
    fn from(s: String) -> Self {
        CellValue::String(s)
    }
}

impl From<&str> for CellValue {
    fn from(s: &str) -> Self {
        CellValue::String(s.to_string())
    }
}

impl<T: Into<CellValue>> From<Option<T>> for CellValue {
    fn from(opt: Option<T>) -> Self {
        match opt {
            Some(v) => v.into(),
            None => CellValue::Null,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::approx_constant)]
    use super::*;

    #[test]
    fn test_parse_null() {
        assert_eq!(CellValue::parse(""), CellValue::Null);
        assert_eq!(CellValue::parse("  "), CellValue::Null);
    }

    #[test]
    fn test_parse_bool() {
        assert_eq!(CellValue::parse("true"), CellValue::Bool(true));
        assert_eq!(CellValue::parse("false"), CellValue::Bool(false));
        assert_eq!(CellValue::parse("TRUE"), CellValue::Bool(true));
        assert_eq!(CellValue::parse("yes"), CellValue::Bool(true));
        assert_eq!(CellValue::parse("no"), CellValue::Bool(false));
    }

    #[test]
    fn test_parse_int() {
        assert_eq!(CellValue::parse("42"), CellValue::Int(42));
        assert_eq!(CellValue::parse("-123"), CellValue::Int(-123));
    }

    #[test]
    fn test_parse_float() {
        assert_eq!(CellValue::parse("3.14"), CellValue::Float(3.14));
        assert_eq!(CellValue::parse("-2.5"), CellValue::Float(-2.5));
    }

    #[test]
    fn test_parse_string() {
        assert_eq!(
            CellValue::parse("hello"),
            CellValue::String("hello".to_string())
        );
    }

    #[test]
    fn test_conversions() {
        assert_eq!(CellValue::Int(42).as_float(), Some(42.0));
        assert_eq!(CellValue::Float(3.14).as_int(), Some(3));
        assert_eq!(CellValue::Bool(true).as_int(), Some(1));
        assert_eq!(CellValue::String("42".to_string()).as_int(), Some(42));
    }
}
