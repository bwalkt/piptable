//! Runtime value types for piptable.

use arrow::array::RecordBatch;
use piptable_sheet::{Book, Sheet};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use piptable_types::{Expr, Param};

/// Runtime value in piptable.
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
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

    /// Sheet data (piptable_sheet::Sheet).
    Sheet(Box<Sheet>),
    /// Book data (piptable_sheet::Book).
    Book(Box<Book>),

    /// Function reference.
    Function {
        name: String,
        params: Vec<Param>,
        is_async: bool,
    },

    /// Lambda expression (anonymous function).
    Lambda { params: Vec<String>, body: Expr },
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
            Self::Sheet(s) => s.row_count() > 0,
            Self::Book(b) => b.sheet_count() > 0,
            Self::Function { .. } => true,
            Self::Lambda { .. } => true,
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
            Self::Sheet(_) => "Sheet",
            Self::Book(_) => "Book",
            Self::Function { .. } => "Function",
            Self::Lambda { .. } => "Lambda",
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
    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Self::Array(a) => Some(a.as_slice()),
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
    pub fn as_table(&self) -> Option<&[Arc<RecordBatch>]> {
        match self {
            Self::Table(t) => Some(t.as_slice()),
            _ => None,
        }
    }

    /// Extract sheet reference if this value is a sheet.
    #[must_use]
    pub fn as_sheet(&self) -> Option<&Sheet> {
        match self {
            Self::Sheet(s) => Some(s.as_ref()),
            _ => None,
        }
    }

    /// Extract mutable sheet reference if this value is a sheet.
    #[must_use]
    pub fn as_sheet_mut(&mut self) -> Option<&mut Sheet> {
        match self {
            Self::Sheet(s) => Some(s.as_mut()),
            _ => None,
        }
    }

    /// Try to get Book reference.
    #[must_use]
    pub fn as_book(&self) -> Option<&Book> {
        match self {
            Self::Book(b) => Some(b.as_ref()),
            _ => None,
        }
    }

    /// Try to get Book mutable reference.
    pub fn as_book_mut(&mut self) -> Option<&mut Book> {
        match self {
            Self::Book(b) => Some(b.as_mut()),
            _ => None,
        }
    }
}

impl Default for Value {
    /// Returns `Value::Null`.
    fn default() -> Self {
        Self::Null
    }
}

impl From<bool> for Value {
    /// Converts a bool into a `Value`.
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}

impl From<i64> for Value {
    /// Converts an i64 into a `Value`.
    fn from(n: i64) -> Self {
        Self::Int(n)
    }
}

impl From<i32> for Value {
    /// Converts an i32 into a `Value`.
    fn from(n: i32) -> Self {
        Self::Int(i64::from(n))
    }
}

impl From<f64> for Value {
    /// Converts an f64 into a `Value`.
    fn from(f: f64) -> Self {
        Self::Float(f)
    }
}

impl From<String> for Value {
    /// Converts a String into a `Value`.
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<&str> for Value {
    /// Converts a &str into a `Value`.
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    /// Converts a Vec into a `Value::Array`.
    fn from(v: Vec<T>) -> Self {
        Self::Array(v.into_iter().map(Into::into).collect())
    }
}

impl From<HashMap<String, Value>> for Value {
    /// Converts a map into a `Value::Object`.
    fn from(m: HashMap<String, Value>) -> Self {
        Self::Object(m)
    }
}

impl Serialize for Value {
    /// Serializes the value to JSON-compatible output.
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
            Self::Sheet(_) => Err(serde::ser::Error::custom(
                "Sheet values are not JSON-serializable",
            )),
            Self::Book(_) => Err(serde::ser::Error::custom(
                "Book values are not JSON-serializable",
            )),
            Self::Function { name, .. } => Err(serde::ser::Error::custom(format!(
                "Function '{name}' is not JSON-serializable"
            ))),
            Self::Lambda { .. } => Err(serde::ser::Error::custom(
                "Lambda expressions are not JSON-serializable",
            )),
        }
    }
}

impl<'de> Deserialize<'de> for Value {
    /// Deserializes a JSON-compatible value into `Value`.
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
    /// Returns error if value contains:
    /// - Table or Function (not representable in JSON)
    /// - Non-finite floats (NaN, Infinity)
    pub fn to_json(&self) -> Result<serde_json::Value, &'static str> {
        match self {
            Self::Null => Ok(serde_json::Value::Null),
            Self::Bool(b) => Ok(serde_json::Value::Bool(*b)),
            Self::Int(n) => Ok(serde_json::Value::Number((*n).into())),
            Self::Float(f) => serde_json::Number::from_f64(*f)
                .map(serde_json::Value::Number)
                .ok_or("Non-finite float values (NaN/Infinity) are not JSON-serializable"),
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
            Self::Sheet(_) => Err("Sheet values are not JSON-serializable"),
            Self::Book(_) => Err("Book values are not JSON-serializable"),
            Self::Function { .. } => Err("Function values are not JSON-serializable"),
            Self::Lambda { .. } => Err("Lambda expressions are not JSON-serializable"),
        }
    }
}

/// Tests for this module.
#[cfg(test)]
mod tests {
    #![allow(clippy::approx_constant)]
    use super::*;

    // ========================================================================
    // is_null tests
    // ========================================================================

    /// Verifies is null.
    #[test]
    fn test_is_null() {
        assert!(Value::Null.is_null());
        assert!(!Value::Bool(false).is_null());
        assert!(!Value::Int(0).is_null());
        assert!(!Value::String(String::new()).is_null());
    }

    // ========================================================================
    // is_truthy tests
    // ========================================================================

    /// Verifies value is truthy.
    #[test]
    fn test_value_is_truthy() {
        assert!(!Value::Null.is_truthy());
        assert!(!Value::Bool(false).is_truthy());
        assert!(Value::Bool(true).is_truthy());
        assert!(!Value::Int(0).is_truthy());
        assert!(Value::Int(1).is_truthy());
        assert!(Value::Int(-1).is_truthy());
        assert!(!Value::Float(0.0).is_truthy());
        assert!(Value::Float(0.1).is_truthy());
        assert!(!Value::String(String::new()).is_truthy());
        assert!(Value::String("hello".to_string()).is_truthy());
        assert!(!Value::Array(vec![]).is_truthy());
        assert!(Value::Array(vec![Value::Int(1)]).is_truthy());
        assert!(!Value::Object(HashMap::new()).is_truthy());
        let mut map = HashMap::new();
        map.insert("k".to_string(), Value::Int(1));
        assert!(Value::Object(map).is_truthy());
        assert!(!Value::Table(vec![]).is_truthy());
        assert!(!Value::Sheet(Box::new(Sheet::new())).is_truthy()); // Empty sheet
        assert!(!Value::Book(Box::new(Book::new())).is_truthy()); // Empty book
        let mut sheet_with_data = Sheet::new();
        sheet_with_data.row_append(vec!["test"]).unwrap();
        assert!(Value::Sheet(Box::new(sheet_with_data)).is_truthy()); // Non-empty sheet
        let mut sheet = Sheet::new();
        sheet.row_append(vec!["test"]).unwrap();
        let mut book = Book::new();
        book.add_sheet("Sheet1", sheet).unwrap();
        assert!(Value::Book(Box::new(book)).is_truthy());
        assert!(Value::Function {
            name: "f".to_string(),
            params: vec![],
            is_async: false
        }
        .is_truthy());
    }

    // ========================================================================
    // type_name tests
    // ========================================================================

    /// Verifies value type name.
    #[test]
    fn test_value_type_name() {
        assert_eq!(Value::Null.type_name(), "Null");
        assert_eq!(Value::Bool(true).type_name(), "Bool");
        assert_eq!(Value::Int(42).type_name(), "Int");
        assert_eq!(Value::Float(3.14).type_name(), "Float");
        assert_eq!(Value::String("test".to_string()).type_name(), "String");
        assert_eq!(Value::Array(vec![]).type_name(), "Array");
        assert_eq!(Value::Object(HashMap::new()).type_name(), "Object");
        assert_eq!(Value::Table(vec![]).type_name(), "Table");
        assert_eq!(Value::Sheet(Box::new(Sheet::new())).type_name(), "Sheet");
        assert_eq!(Value::Book(Box::new(Book::new())).type_name(), "Book");
        assert_eq!(
            Value::Function {
                name: "f".to_string(),
                params: vec![],
                is_async: false
            }
            .type_name(),
            "Function"
        );
    }

    // ========================================================================
    // as_* accessor tests
    // ========================================================================

    /// Verifies as bool.
    #[test]
    fn test_as_bool() {
        assert_eq!(Value::Bool(true).as_bool(), Some(true));
        assert_eq!(Value::Bool(false).as_bool(), Some(false));
        assert_eq!(Value::Int(1).as_bool(), None);
        assert_eq!(Value::Null.as_bool(), None);
    }

    /// Verifies as int.
    #[test]
    fn test_as_int() {
        assert_eq!(Value::Int(42).as_int(), Some(42));
        assert_eq!(Value::Float(3.7).as_int(), Some(3)); // truncates
        assert_eq!(Value::String("42".to_string()).as_int(), None);
        assert_eq!(Value::Null.as_int(), None);
    }

    /// Verifies as float.
    #[test]
    fn test_as_float() {
        assert_eq!(Value::Float(3.14).as_float(), Some(3.14));
        assert_eq!(Value::Int(42).as_float(), Some(42.0));
        assert_eq!(Value::String("3.14".to_string()).as_float(), None);
        assert_eq!(Value::Null.as_float(), None);
    }

    /// Verifies as str.
    #[test]
    fn test_as_str() {
        assert_eq!(Value::String("hello".to_string()).as_str(), Some("hello"));
        assert_eq!(Value::Int(42).as_str(), None);
        assert_eq!(Value::Null.as_str(), None);
    }

    /// Verifies as array.
    #[test]
    fn test_as_array() {
        let arr = vec![Value::Int(1), Value::Int(2)];
        let v = Value::Array(arr.clone());
        assert!(v.as_array().is_some());
        assert_eq!(v.as_array().unwrap().len(), 2);
        assert!(Value::Int(42).as_array().is_none());
    }

    /// Verifies as object.
    #[test]
    fn test_as_object() {
        let mut map = HashMap::new();
        map.insert("key".to_string(), Value::Int(42));
        let v = Value::Object(map);
        assert!(v.as_object().is_some());
        assert_eq!(
            v.as_object().unwrap().get("key").unwrap().as_int(),
            Some(42)
        );
        assert!(Value::Int(42).as_object().is_none());
    }

    /// Verifies as table.
    #[test]
    fn test_as_table() {
        let v = Value::Table(vec![]);
        assert!(v.as_table().is_some());
        assert!(Value::Int(42).as_table().is_none());
    }

    /// Verifies as sheet.
    #[test]
    fn test_as_sheet() {
        let sheet = Sheet::new();
        let v = Value::Sheet(Box::new(sheet));
        assert!(v.as_sheet().is_some());
        assert_eq!(v.as_sheet().unwrap().row_count(), 0);
        assert!(Value::Int(42).as_sheet().is_none());
    }

    // ========================================================================
    // Default impl test
    // ========================================================================

    /// Verifies default.
    #[test]
    fn test_default() {
        let v = Value::default();
        assert!(v.is_null());
    }

    // ========================================================================
    // From conversions tests
    // ========================================================================

    /// Verifies value from conversions.
    #[test]
    fn test_value_from_conversions() {
        let v: Value = true.into();
        assert!(matches!(v, Value::Bool(true)));

        let v: Value = 42i64.into();
        assert!(matches!(v, Value::Int(42)));

        let v: Value = 42i32.into();
        assert!(matches!(v, Value::Int(42)));

        let v: Value = 3.14f64.into();
        assert!(matches!(v, Value::Float(f) if (f - 3.14).abs() < f64::EPSILON));

        let v: Value = "hello".into();
        assert!(matches!(v, Value::String(s) if s == "hello"));

        let v: Value = String::from("world").into();
        assert!(matches!(v, Value::String(s) if s == "world"));

        let v: Value = vec![1i64, 2i64, 3i64].into();
        assert!(matches!(v, Value::Array(arr) if arr.len() == 3));

        let mut map = HashMap::new();
        map.insert("key".to_string(), Value::Int(42));
        let v: Value = map.into();
        assert!(matches!(v, Value::Object(_)));
    }

    // ========================================================================
    // JSON conversion tests
    // ========================================================================

    /// Verifies from json.
    #[test]
    fn test_from_json() {
        assert!(Value::from_json(serde_json::Value::Null).is_null());
        assert!(matches!(
            Value::from_json(serde_json::Value::Bool(true)),
            Value::Bool(true)
        ));
        assert!(matches!(
            Value::from_json(serde_json::json!(42)),
            Value::Int(42)
        ));
        assert!(matches!(
            Value::from_json(serde_json::json!(3.14)),
            Value::Float(f) if (f - 3.14).abs() < f64::EPSILON
        ));
        assert!(matches!(
            Value::from_json(serde_json::json!("hello")),
            Value::String(s) if s == "hello"
        ));
        assert!(matches!(
            Value::from_json(serde_json::json!([1, 2, 3])),
            Value::Array(arr) if arr.len() == 3
        ));
        assert!(matches!(
            Value::from_json(serde_json::json!({"key": "value"})),
            Value::Object(_)
        ));
    }

    /// Verifies to json.
    #[test]
    fn test_to_json() {
        assert_eq!(Value::Null.to_json().unwrap(), serde_json::Value::Null);
        assert_eq!(
            Value::Bool(true).to_json().unwrap(),
            serde_json::Value::Bool(true)
        );
        assert_eq!(Value::Int(42).to_json().unwrap(), serde_json::json!(42));
        assert_eq!(
            Value::Float(3.14).to_json().unwrap(),
            serde_json::json!(3.14)
        );
        assert_eq!(
            Value::String("hello".to_string()).to_json().unwrap(),
            serde_json::json!("hello")
        );

        // Array
        let arr = Value::Array(vec![Value::Int(1), Value::Int(2)]);
        assert_eq!(arr.to_json().unwrap(), serde_json::json!([1, 2]));

        // Object
        let mut map = HashMap::new();
        map.insert("key".to_string(), Value::Int(42));
        let obj = Value::Object(map);
        assert_eq!(obj.to_json().unwrap(), serde_json::json!({"key": 42}));
    }

    /// Verifies to json errors.
    #[test]
    fn test_to_json_errors() {
        // Table is not JSON-serializable
        let table = Value::Table(vec![]);
        assert!(table.to_json().is_err());

        // Function is not JSON-serializable
        let func = Value::Function {
            name: "f".to_string(),
            params: vec![],
            is_async: false,
        };
        assert!(func.to_json().is_err());

        // Book is not JSON-serializable
        let book = Value::Book(Box::new(Book::new()));
        assert!(book.to_json().is_err());

        // NaN is not JSON-serializable
        let nan = Value::Float(f64::NAN);
        assert!(nan.to_json().is_err());

        // Infinity is not JSON-serializable
        let inf = Value::Float(f64::INFINITY);
        assert!(inf.to_json().is_err());
    }

    // ========================================================================
    // Serialize/Deserialize tests
    // ========================================================================

    /// Verifies serialize.
    #[test]
    fn test_serialize() {
        let v = Value::Int(42);
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, "42");

        let v = Value::String("hello".to_string());
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, "\"hello\"");

        let v = Value::Array(vec![Value::Int(1), Value::Int(2)]);
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, "[1,2]");

        let v = Value::Null;
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, "null");
    }

    /// Verifies serialize errors.
    #[test]
    fn test_serialize_errors() {
        // Table cannot be serialized
        let v = Value::Table(vec![]);
        assert!(serde_json::to_string(&v).is_err());

        // Book cannot be serialized
        let v = Value::Book(Box::new(Book::new()));
        assert!(serde_json::to_string(&v).is_err());

        // Function cannot be serialized
        let v = Value::Function {
            name: "f".to_string(),
            params: vec![],
            is_async: false,
        };
        assert!(serde_json::to_string(&v).is_err());
    }

    /// Verifies deserialize.
    #[test]
    fn test_deserialize() {
        let v: Value = serde_json::from_str("42").unwrap();
        assert!(matches!(v, Value::Int(42)));

        let v: Value = serde_json::from_str("\"hello\"").unwrap();
        assert!(matches!(v, Value::String(s) if s == "hello"));

        let v: Value = serde_json::from_str("[1, 2, 3]").unwrap();
        assert!(matches!(v, Value::Array(arr) if arr.len() == 3));

        let v: Value = serde_json::from_str("null").unwrap();
        assert!(v.is_null());
    }
}
