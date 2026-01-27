//! Conversions between Sheet, Arrow, and Value types.

use arrow::array::{ArrayRef, BooleanArray, Float64Array, Int64Array, StringArray};
use arrow::datatypes::DataType;
use arrow::record_batch::RecordBatch;
use piptable_core::Value;
use piptable_sheet::{CellValue, Sheet};
use std::collections::HashMap;
use std::sync::Arc;

/// Convert a Value to a Sheet.
pub fn value_to_sheet(value: &Value) -> Result<Sheet, String> {
    match value {
        Value::Sheet(sheet) => Ok(sheet.clone()),
        Value::Array(rows) => {
            let mut sheet = Sheet::new();

            // Collect all unique columns from all objects
            let mut columns_set = std::collections::HashSet::new();
            for row_value in rows {
                if let Value::Object(obj) = row_value {
                    for key in obj.keys() {
                        columns_set.insert(key.clone());
                    }
                }
            }

            // Convert to sorted Vec for consistent column ordering
            let mut columns: Vec<String> = columns_set.into_iter().collect();
            columns.sort();

            if !columns.is_empty() {
                // Add header row
                let header_cells: Vec<CellValue> = columns
                    .iter()
                    .map(|col| CellValue::String(col.clone()))
                    .collect();
                sheet.data_mut().push(header_cells);

                // Add data rows
                for row_value in rows {
                    if let Value::Object(row) = row_value {
                        let cells: Vec<CellValue> = columns
                            .iter()
                            .map(|col| row.get(col).map(value_to_cell).unwrap_or(CellValue::Null))
                            .collect();
                        sheet.data_mut().push(cells);
                    }
                }

                // Name columns by first row
                sheet
                    .name_columns_by_row(0)
                    .map_err(|e| format!("Failed to name columns: {}", e))?;
            }
            Ok(sheet)
        }
        Value::Table(batches) => {
            if batches.is_empty() {
                return Ok(Sheet::new());
            }
            arrow_batches_to_sheet(batches)
        }
        _ => Err(format!("Cannot convert {} to Sheet", value.type_name())),
    }
}

/// Convert a Value to a CellValue.
pub fn value_to_cell(value: &Value) -> CellValue {
    match value {
        Value::Null => CellValue::Null,
        Value::Bool(b) => CellValue::Bool(*b),
        Value::Int(n) => CellValue::Int(*n),
        Value::Float(f) => CellValue::Float(*f),
        Value::String(s) => CellValue::String(s.clone()),
        Value::Array(arr) => {
            // Convert array to JSON string representation
            let json = serde_json::to_string(arr).unwrap_or_else(|_| "[]".to_string());
            CellValue::String(json)
        }
        Value::Object(obj) => {
            let json = serde_json::to_string(obj).unwrap_or_else(|_| "{}".to_string());
            CellValue::String(json)
        }
        _ => CellValue::String(format!("{:?}", value)),
    }
}

/// Convert Arrow RecordBatches to a Sheet.
pub fn arrow_batches_to_sheet(batches: &[Arc<RecordBatch>]) -> Result<Sheet, String> {
    if batches.is_empty() {
        return Ok(Sheet::new());
    }

    let schema = batches[0].schema();
    let field_names: Vec<String> = schema.fields().iter().map(|f| f.name().clone()).collect();

    let mut sheet = Sheet::new();

    // Add column names as the first row
    let header_row: Vec<CellValue> = field_names
        .iter()
        .map(|name| CellValue::String(name.clone()))
        .collect();
    sheet.data_mut().push(header_row);

    // Add data rows
    for batch in batches {
        let num_rows = batch.num_rows();
        for row_idx in 0..num_rows {
            let mut row_cells = Vec::new();
            for col_idx in 0..batch.num_columns() {
                let array = batch.column(col_idx);
                let cell = arrow_value_to_cell(array, row_idx);
                row_cells.push(cell);
            }
            sheet.data_mut().push(row_cells);
        }
    }

    // Mark that the first row contains column names
    sheet
        .name_columns_by_row(0)
        .map_err(|e| format!("Failed to name columns: {}", e))?;

    Ok(sheet)
}

/// Convert an Arrow array value at a specific row to CellValue.
pub fn arrow_value_to_cell(array: &Arc<dyn arrow::array::Array>, row: usize) -> CellValue {
    use arrow::array::{
        BooleanArray, Float32Array, Float64Array, Int16Array, Int32Array, Int64Array, Int8Array,
        LargeStringArray, StringArray, UInt16Array, UInt32Array, UInt64Array, UInt8Array,
    };
    use arrow::datatypes::DataType;

    if array.is_null(row) {
        return CellValue::Null;
    }

    match array.data_type() {
        DataType::Boolean => {
            let arr = array.as_any().downcast_ref::<BooleanArray>().unwrap();
            CellValue::Bool(arr.value(row))
        }
        DataType::Int8 => {
            let arr = array.as_any().downcast_ref::<Int8Array>().unwrap();
            CellValue::Int(i64::from(arr.value(row)))
        }
        DataType::Int16 => {
            let arr = array.as_any().downcast_ref::<Int16Array>().unwrap();
            CellValue::Int(i64::from(arr.value(row)))
        }
        DataType::Int32 => {
            let arr = array.as_any().downcast_ref::<Int32Array>().unwrap();
            CellValue::Int(i64::from(arr.value(row)))
        }
        DataType::Int64 => {
            let arr = array.as_any().downcast_ref::<Int64Array>().unwrap();
            CellValue::Int(arr.value(row))
        }
        DataType::UInt8 => {
            let arr = array.as_any().downcast_ref::<UInt8Array>().unwrap();
            CellValue::Int(i64::from(arr.value(row)))
        }
        DataType::UInt16 => {
            let arr = array.as_any().downcast_ref::<UInt16Array>().unwrap();
            CellValue::Int(i64::from(arr.value(row)))
        }
        DataType::UInt32 => {
            let arr = array.as_any().downcast_ref::<UInt32Array>().unwrap();
            CellValue::Int(i64::from(arr.value(row)))
        }
        DataType::UInt64 => {
            let arr = array.as_any().downcast_ref::<UInt64Array>().unwrap();
            let val = arr.value(row);
            if val > i64::MAX as u64 {
                CellValue::String(val.to_string())
            } else {
                CellValue::Int(val as i64)
            }
        }
        DataType::Float32 => {
            let arr = array.as_any().downcast_ref::<Float32Array>().unwrap();
            CellValue::Float(f64::from(arr.value(row)))
        }
        DataType::Float64 => {
            let arr = array.as_any().downcast_ref::<Float64Array>().unwrap();
            CellValue::Float(arr.value(row))
        }
        DataType::Utf8 => {
            let arr = array.as_any().downcast_ref::<StringArray>().unwrap();
            CellValue::String(arr.value(row).to_string())
        }
        DataType::LargeUtf8 => {
            let arr = array.as_any().downcast_ref::<LargeStringArray>().unwrap();
            CellValue::String(arr.value(row).to_string())
        }
        _ => {
            // For other types, convert to string representation
            CellValue::String(format!("{:?}", array))
        }
    }
}

/// Convert a Sheet to a Value (array of objects).
pub fn sheet_to_value(sheet: &Sheet) -> Value {
    if let Some(column_names) = sheet.column_names() {
        let mut rows = Vec::new();

        // Determine if we should skip the first row
        // Only skip if the first row matches the column names
        let should_skip_first = if sheet.data().is_empty() {
            0
        } else {
            let first_row = &sheet.data()[0];
            let names_match = column_names.iter().enumerate().all(|(idx, name)| {
                first_row
                    .get(idx)
                    .map(|cell| cell.as_str() == name.as_str())
                    .unwrap_or(false)
            });
            usize::from(names_match)
        };

        // Iterate from the appropriate starting point
        for row_idx in should_skip_first..sheet.row_count() {
            let mut row_obj = HashMap::new();

            if let Some(row_data) = sheet.data().get(row_idx) {
                for (col_idx, col_name) in column_names.iter().enumerate() {
                    let cell_value = row_data.get(col_idx).cloned().unwrap_or(CellValue::Null);
                    row_obj.insert(col_name.clone(), cell_to_value(cell_value));
                }
            }

            rows.push(Value::Object(row_obj));
        }

        Value::Array(rows)
    } else {
        // No column names, return as array of arrays
        let mut rows = Vec::new();
        for row_data in sheet.data() {
            let row: Vec<Value> = row_data
                .iter()
                .map(|cell| cell_to_value(cell.clone()))
                .collect();
            rows.push(Value::Array(row));
        }
        Value::Array(rows)
    }
}

/// Convert a CellValue to a Value.
pub fn cell_to_value(cell: CellValue) -> Value {
    match cell {
        CellValue::Null => Value::Null,
        CellValue::Bool(b) => Value::Bool(b),
        CellValue::Int(i) => Value::Int(i),
        CellValue::Float(f) => Value::Float(f),
        CellValue::String(s) => Value::String(s),
    }
}

/// Infer the appropriate Arrow DataType for a column from row data.
pub fn infer_sheet_column_type(
    rows: &[&Vec<piptable_sheet::CellValue>],
    col_idx: usize,
) -> DataType {
    let mut has_int = false;
    let mut has_float = false;
    let mut has_bool = false;
    let mut has_string = false;
    let mut all_null = true;

    for row in rows {
        if col_idx >= row.len() {
            continue;
        }
        match &row[col_idx] {
            CellValue::Int(_) => {
                has_int = true;
                all_null = false;
            }
            CellValue::Float(_) => {
                has_float = true;
                all_null = false;
            }
            CellValue::Bool(_) => {
                has_bool = true;
                all_null = false;
            }
            CellValue::String(_) => {
                has_string = true;
                all_null = false;
            }
            CellValue::Null => {}
        }
    }

    if all_null || has_string || (has_int && has_bool) || (has_float && has_bool) {
        DataType::Utf8
    } else if has_float {
        DataType::Float64
    } else if has_int {
        DataType::Int64
    } else if has_bool {
        DataType::Boolean
    } else {
        DataType::Utf8
    }
}

/// Build an Arrow array from column data.
pub fn build_sheet_arrow_array(
    rows: &[&Vec<piptable_sheet::CellValue>],
    col_idx: usize,
    dtype: &DataType,
) -> Result<ArrayRef, String> {
    match dtype {
        DataType::Boolean => {
            let values: Vec<Option<bool>> = rows
                .iter()
                .map(|row| {
                    row.get(col_idx).and_then(|cell| match cell {
                        CellValue::Bool(b) => Some(*b),
                        _ => None,
                    })
                })
                .collect();
            Ok(Arc::new(BooleanArray::from(values)))
        }
        DataType::Int64 => {
            let values: Vec<Option<i64>> = rows
                .iter()
                .map(|row| {
                    row.get(col_idx).and_then(|cell| match cell {
                        CellValue::Int(i) => Some(*i),
                        _ => None,
                    })
                })
                .collect();
            Ok(Arc::new(Int64Array::from(values)))
        }
        DataType::Float64 => {
            let values: Vec<Option<f64>> = rows
                .iter()
                .map(|row| {
                    row.get(col_idx).and_then(|cell| match cell {
                        CellValue::Float(f) => Some(*f),
                        CellValue::Int(i) => Some(*i as f64),
                        _ => None,
                    })
                })
                .collect();
            Ok(Arc::new(Float64Array::from(values)))
        }
        DataType::Utf8 => {
            let values: Vec<Option<String>> = rows
                .iter()
                .map(|row| {
                    row.get(col_idx).and_then(|cell| match cell {
                        CellValue::String(s) => Some(s.clone()),
                        CellValue::Int(i) => Some(i.to_string()),
                        CellValue::Float(f) => Some(f.to_string()),
                        CellValue::Bool(b) => Some(b.to_string()),
                        CellValue::Null => None,
                    })
                })
                .collect();
            Ok(Arc::new(StringArray::from(values)))
        }
        _ => Err(format!("Unsupported data type: {:?}", dtype)),
    }
}
