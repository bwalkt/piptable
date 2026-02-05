use std::collections::HashMap;
use std::sync::Arc;

use arrow::array::{Array, Float64Array, Int64Array, StringArray, UInt64Array};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use piptable_core::Value;
use piptable_interpreter::sheet_conversions;
use piptable_sheet::{CellValue, Sheet};

#[test]
fn test_value_to_sheet_from_objects() {
    let mut row1 = HashMap::new();
    row1.insert("b".to_string(), Value::Int(2));
    row1.insert("a".to_string(), Value::String("x".to_string()));

    let mut row2 = HashMap::new();
    row2.insert("b".to_string(), Value::Int(3));

    let value = Value::Array(vec![Value::Object(row1), Value::Object(row2)]);
    let sheet = sheet_conversions::value_to_sheet(&value).expect("convert sheet");

    let column_names = sheet.column_names().expect("column names");
    assert_eq!(column_names, &vec!["a".to_string(), "b".to_string()]);

    let data = sheet.data();
    assert_eq!(
        data[0],
        vec![
            CellValue::String("a".to_string()),
            CellValue::String("b".to_string())
        ]
    );
    assert_eq!(
        data[1],
        vec![CellValue::String("x".to_string()), CellValue::Int(2)]
    );
    assert_eq!(data[2], vec![CellValue::Null, CellValue::Int(3)]);
}

#[test]
fn test_value_to_sheet_from_empty_table() {
    let value = Value::Table(Vec::new());
    let sheet = sheet_conversions::value_to_sheet(&value).expect("convert sheet");
    assert_eq!(sheet.row_count(), 0);
}

#[test]
fn test_sheet_to_value_with_named_columns_skips_header() {
    let mut sheet = Sheet::from_data(vec![
        vec!["name", "age"],
        vec!["Alice", "30"],
        vec!["Bob", "40"],
    ]);
    sheet.name_columns_by_row(0).expect("name columns");

    let value = sheet_conversions::sheet_to_value(&sheet);
    let Value::Array(rows) = value else {
        panic!("expected array value");
    };
    assert_eq!(rows.len(), 2);

    let Value::Object(first) = &rows[0] else {
        panic!("expected object row");
    };
    assert!(matches!(
        first.get("name"),
        Some(Value::String(s)) if s == "Alice"
    ));
    assert!(matches!(
        first.get("age"),
        Some(Value::String(s)) if s == "30"
    ));
}

#[test]
fn test_sheet_to_value_without_column_names() {
    let sheet = Sheet::from_data(vec![vec![1, 2], vec![3, 4]]);
    let value = sheet_conversions::sheet_to_value(&sheet);
    let Value::Array(rows) = value else {
        panic!("expected array value");
    };
    assert_eq!(rows.len(), 2);
    assert!(matches!(
        rows[0],
        Value::Array(ref items)
            if matches!(items.as_slice(), [Value::Int(1), Value::Int(2)])
    ));
    assert!(matches!(
        rows[1],
        Value::Array(ref items)
            if matches!(items.as_slice(), [Value::Int(3), Value::Int(4)])
    ));
}

#[test]
fn test_cell_to_value_formula_cached_and_uncached() {
    let mut cached = CellValue::formula("=1+1");
    cached.set_cached(CellValue::Int(2));
    assert!(matches!(
        sheet_conversions::cell_to_value(cached),
        Value::Int(2)
    ));

    let uncached = CellValue::formula("=1+1");
    assert!(matches!(
        sheet_conversions::cell_to_value(uncached),
        Value::String(ref s) if s == "=1+1"
    ));
}

#[test]
fn test_infer_sheet_column_type_cases() {
    let row1 = vec![CellValue::Int(1), CellValue::Bool(true), CellValue::Null];
    let row2 = vec![CellValue::Int(2), CellValue::Bool(false), CellValue::Null];
    let rows = vec![&row1, &row2];

    assert_eq!(
        sheet_conversions::infer_sheet_column_type(&rows, 0),
        DataType::Int64
    );
    assert_eq!(
        sheet_conversions::infer_sheet_column_type(&rows, 1),
        DataType::Boolean
    );
    assert_eq!(
        sheet_conversions::infer_sheet_column_type(&rows, 2),
        DataType::Utf8
    );

    let row3 = vec![CellValue::Int(1), CellValue::Float(1.5)];
    let row4 = vec![CellValue::Int(2), CellValue::Float(2.5)];
    let rows = vec![&row3, &row4];
    assert_eq!(
        sheet_conversions::infer_sheet_column_type(&rows, 1),
        DataType::Float64
    );

    let row5 = vec![CellValue::Int(1), CellValue::Bool(true)];
    let row6 = vec![CellValue::Int(2), CellValue::Bool(false)];
    let rows = vec![&row5, &row6];
    assert_eq!(
        sheet_conversions::infer_sheet_column_type(&rows, 0),
        DataType::Int64
    );
    assert_eq!(
        sheet_conversions::infer_sheet_column_type(&rows, 1),
        DataType::Boolean
    );
}

#[test]
fn test_build_sheet_arrow_array_float_and_utf8() {
    let row1 = vec![CellValue::Int(1), CellValue::String("a".to_string())];
    let row2 = vec![CellValue::Float(2.5), CellValue::formula("=1+1")];
    let rows = vec![&row1, &row2];

    let float_array = sheet_conversions::build_sheet_arrow_array(&rows, 0, &DataType::Float64)
        .expect("float array");
    let float_array = float_array.as_any().downcast_ref::<Float64Array>().unwrap();
    assert_eq!(float_array.value(0), 1.0);
    assert_eq!(float_array.value(1), 2.5);

    let string_array = sheet_conversions::build_sheet_arrow_array(&rows, 1, &DataType::Utf8)
        .expect("string array");
    let string_array = string_array.as_any().downcast_ref::<StringArray>().unwrap();
    assert_eq!(string_array.value(0), "a");
    assert!(string_array.is_null(1));
}

#[test]
fn test_arrow_batches_to_sheet_and_value_conversion() {
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, true),
        Field::new("name", DataType::Utf8, true),
    ]);
    let batch = RecordBatch::try_new(
        Arc::new(schema),
        vec![
            Arc::new(Int64Array::from(vec![Some(1), Some(2)])) as Arc<dyn Array>,
            Arc::new(StringArray::from(vec![Some("a"), Some("b")])) as Arc<dyn Array>,
        ],
    )
    .expect("record batch");

    let sheet = sheet_conversions::arrow_batches_to_sheet(&[Arc::new(batch)]).expect("sheet");
    let data = sheet.data();
    assert_eq!(
        data[0],
        vec![
            CellValue::String("id".to_string()),
            CellValue::String("name".to_string())
        ]
    );
    assert_eq!(
        data[1],
        vec![CellValue::Int(1), CellValue::String("a".to_string())]
    );
}

#[test]
fn test_arrow_value_to_cell_uint64_overflow() {
    let array = Arc::new(UInt64Array::from(vec![u64::MAX])) as Arc<dyn Array>;
    let cell = sheet_conversions::arrow_value_to_cell(&array, 0);
    assert_eq!(cell, CellValue::String(u64::MAX.to_string()));
}
