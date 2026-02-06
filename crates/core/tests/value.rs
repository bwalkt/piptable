use arrow::array::{Int32Array, RecordBatch};
use arrow::datatypes::{DataType, Field, Schema};
use piptable_core::Value;
use piptable_sheet::{Book, Sheet};
use std::sync::Arc;

#[test]
fn test_type_name_and_truthy() {
    assert_eq!(Value::Null.type_name(), "Null");
    assert!(!Value::Null.is_truthy());
    assert!(Value::Bool(true).is_truthy());
    assert!(!Value::Bool(false).is_truthy());
    assert!(Value::Int(1).is_truthy());
    assert!(!Value::Int(0).is_truthy());
    assert!(Value::Float(1.5).is_truthy());
    assert!(!Value::Float(0.0).is_truthy());
    assert!(Value::String("x".to_string()).is_truthy());
    assert!(!Value::String(String::new()).is_truthy());
    assert!(Value::Array(vec![Value::Int(1)]).is_truthy());
    assert!(!Value::Array(Vec::new()).is_truthy());
}

#[test]
fn test_truthy_sheet_and_book() {
    let mut sheet = Sheet::new();
    assert!(!Value::Sheet(Box::new(sheet.clone())).is_truthy());
    sheet.row_append(vec![1]).expect("row");
    assert!(Value::Sheet(Box::new(sheet)).is_truthy());

    let mut book = Book::new();
    assert!(!Value::Book(Box::new(book.clone())).is_truthy());
    book.add_sheet("Sheet1", Sheet::new()).expect("add");
    assert!(Value::Book(Box::new(book)).is_truthy());
}

#[test]
fn test_as_accessors() {
    let value = Value::Int(42);
    assert_eq!(value.as_int(), Some(42));
    assert_eq!(value.as_float(), Some(42.0));
    assert_eq!(value.as_bool(), None);

    let value = Value::String("hi".to_string());
    assert_eq!(value.as_str(), Some("hi"));

    let array = Value::Array(vec![Value::Int(1)]);
    assert_eq!(array.as_array().unwrap().len(), 1);

    let object = Value::Object([("k".to_string(), Value::Int(1))].into_iter().collect());
    assert_eq!(object.as_object().unwrap().len(), 1);
}

#[test]
fn test_as_table_sheet_book() {
    let schema = Arc::new(Schema::new(vec![Field::new("x", DataType::Int32, false)]));
    let batch = RecordBatch::try_new(schema, vec![Arc::new(Int32Array::from(vec![1, 2, 3]))])
        .expect("batch");
    let table = Value::Table(vec![Arc::new(batch)]);
    assert!(table.as_table().is_some());

    let sheet = Value::Sheet(Box::new(Sheet::new()));
    assert!(sheet.as_sheet().is_some());

    let book = Value::Book(Box::new(Book::new()));
    assert!(book.as_book().is_some());
}
