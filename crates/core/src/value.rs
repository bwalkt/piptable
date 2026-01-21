//! Runtime value types for piptable.

use arrow::array::RecordBatch;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Runtime value in piptable.
#[derive(Debug, Clone)]
pub enum Value {
    /// Null value.
    Null,

    /// Boolean value.
    Bool(bool),

    /// Integer value (64-bit).
    Int(i64),

    /// Float value (64-bit).
    Float(f64),

    /// String value.
    String(String),

    /// Array of values.
    Array(Vec<Value>),

    /// Object (key-value map).
    Object(HashMap<String, Value>),

    /// Table data (Arrow RecordBatches).
    Table(Vec<Arc<RecordBatch>>),

    /// Function reference.
    Function {
        name: String,
        params: Vec<String>,
        is_async: bool,
    },
}

impl Value {
    /// Check if value is null.
    #[must_use]
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Check if value is truthy.
    #[must_use]
    pub fn is_truthy(&self) -> bool {
        match self {
            Self::Null => false,
            Self::Bool(b) => *b,
            Self::Int(n) => *n != 0,
            Self::Float(f) => *f != 0.0,
            Self::String(s) => !s.is_empty(),
            Self::Array(a) => !a.is_empty(),
            Self::Object(o) => !o.is_empty(),
            Self::Table(t) => !t.is_empty(),
            Self::Function { .. } => true,
        }
    }

    /// Get the type name of this value.
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Null => "Null",
            Self::Bool(_) => "Bool",
            Self::Int(_) => "Int",
            Self::Float(_) => "Float",
            Self::String(_) => "String",
            Self::Array(_) => "Array",
            Self::Object(_) => "Object",
            Self::Table(_) => "Table",
            Self::Function { .. } => "Function",
        }
    }

    /// Try to convert to bool.
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Try to convert to int.
    #[must_use]
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(n) => Some(*n),
            Self::Float(f) => Some(*f as i64),
            _ => None,
        }
    }

    /// Try to convert to float.
    #[must_use]
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Self::Float(f) => Some(*f),
            Self::Int(n) => Some(*n as f64),
            _ => None,
        }
    }

    /// Try to convert to string.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    /// Try to convert to array.
    #[must_use]
    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self {
            Self::Array(a) => Some(a),
            _ => None,
        }
    }

    /// Try to convert to object.
    #[must_use]
    pub fn as_object(&self) -> Option<&HashMap<String, Value>> {
        match self {
            Self::Object(o) => Some(o),
            _ => None,
        }
    }

    /// Try to convert to table.
    #[must_use]
    pub fn as_table(&self) -> Option<&Vec<Arc<RecordBatch>>> {
        match self {
            Self::Table(t) => Some(t),
            _ => None,
        }
    }
}

impl Default for Value {
    fn default() -> Self {
        Self::Null
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}

impl From<i64> for Value {
    fn from(n: i64) -> Self {
        Self::Int(n)
    }
}

impl From<i32> for Value {
    fn from(n: i32) -> Self {
        Self::Int(i64::from(n))
    }
}

impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Self::Float(f)
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(v: Vec<T>) -> Self {
        Self::Array(v.into_iter().map(Into::into).collect())
    }
}

impl From<HashMap<String, Value>> for Value {
    fn from(m: HashMap<String, Value>) -> Self {
        Self::Object(m)
    }
}

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Null => serializer.serialize_none(),
            Self::Bool(b) => serializer.serialize_bool(*b),
            Self::Int(n) => serializer.serialize_i64(*n),
            Self::Float(f) => serializer.serialize_f64(*f),
            Self::String(s) => serializer.serialize_str(s),
            Self::Array(a) => a.serialize(serializer),
            Self::Object(o) => o.serialize(serializer),
            Self::Table(_) => Err(serde::ser::Error::custom(
                "Table values are not JSON-serializable",
            )),
            Self::Function { .. } => Err(serde::ser::Error::custom(
                "Function values are not JSON-serializable",
            )),
        }
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let json: serde_json::Value = serde_json::Value::deserialize(deserializer)?;
        Ok(Self::from_json(json))
    }
}

impl Value {
    /// Convert from `serde_json::Value`.
    #[must_use]
    pub fn from_json(json: serde_json::Value) -> Self {
        match json {
            serde_json::Value::Null => Self::Null,
            serde_json::Value::Bool(b) => Self::Bool(b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Self::Int(i)
                } else if let Some(f) = n.as_f64() {
                    Self::Float(f)
                } else {
                    Self::Null
                }
            }
            serde_json::Value::String(s) => Self::String(s),
            serde_json::Value::Array(a) => {
                Self::Array(a.into_iter().map(Self::from_json).collect())
            }
            serde_json::Value::Object(o) => Self::Object(
                o.into_iter()
                    .map(|(k, v)| (k, Self::from_json(v)))
                    .collect(),
            ),
        }
    }

    /// Convert to `serde_json::Value`.
    ///
    /// # Errors
    ///
    /// Returns error if value contains Table or Function which cannot be serialized.
    pub fn to_json(&self) -> Result<serde_json::Value, &'static str> {
        match self {
            Self::Null => Ok(serde_json::Value::Null),
            Self::Bool(b) => Ok(serde_json::Value::Bool(*b)),
            Self::Int(n) => Ok(serde_json::Value::Number((*n).into())),
            Self::Float(f) => Ok(serde_json::Number::from_f64(*f)
                .map_or(serde_json::Value::Null, serde_json::Value::Number)),
            Self::String(s) => Ok(serde_json::Value::String(s.clone())),
            Self::Array(a) => {
                let items: Result<Vec<_>, _> = a.iter().map(Self::to_json).collect();
                Ok(serde_json::Value::Array(items?))
            }
            Self::Object(o) => {
                let items: Result<serde_json::Map<_, _>, _> = o
                    .iter()
                    .map(|(k, v)| v.to_json().map(|val| (k.clone(), val)))
                    .collect();
                Ok(serde_json::Value::Object(items?))
            }
            Self::Table(_) => Err("Table values are not JSON-serializable"),
            Self::Function { .. } => Err("Function values are not JSON-serializable"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_is_truthy() {
        assert!(!Value::Null.is_truthy());
        assert!(!Value::Bool(false).is_truthy());
        assert!(Value::Bool(true).is_truthy());
        assert!(!Value::Int(0).is_truthy());
        assert!(Value::Int(1).is_truthy());
        assert!(!Value::String(String::new()).is_truthy());
        assert!(Value::String("hello".to_string()).is_truthy());
    }

    #[test]
    fn test_value_type_name() {
        assert_eq!(Value::Null.type_name(), "Null");
        assert_eq!(Value::Bool(true).type_name(), "Bool");
        assert_eq!(Value::Int(42).type_name(), "Int");
        assert_eq!(Value::Float(3.14).type_name(), "Float");
        assert_eq!(Value::String("test".to_string()).type_name(), "String");
    }

    #[test]
    fn test_value_from_conversions() {
        let v: Value = true.into();
        assert!(matches!(v, Value::Bool(true)));

        let v: Value = 42i64.into();
        assert!(matches!(v, Value::Int(42)));

        let v: Value = "hello".into();
        assert!(matches!(v, Value::String(s) if s == "hello"));
    }
}
