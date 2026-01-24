//! Parquet file support for Sheet
//!
//! Provides reading and writing sheets as Apache Parquet files,
//! a columnar storage format with efficient compression.

use crate::cell::CellValue;
use crate::error::{Result, SheetError};
use crate::sheet::Sheet;
use arrow::array::{
    Array, ArrayRef, BooleanArray, Float64Array, Int64Array, RecordBatch, StringArray,
};
use arrow::datatypes::{DataType, Field, Schema};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::arrow::ArrowWriter;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

impl Sheet {
    /// Load a sheet from a Parquet file
    ///
    /// # Example
    /// ```no_run
    /// use piptable_sheet::Sheet;
    ///
    /// let sheet = Sheet::from_parquet("data.parquet").unwrap();
    /// ```
    pub fn from_parquet<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path.as_ref())?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;

        let schema = builder.schema().clone();
        let reader = builder.build()?;

        // Collect all record batches
        let mut batches: Vec<RecordBatch> = Vec::new();
        for batch_result in reader {
            let batch = batch_result?;
            batches.push(batch);
        }

        if batches.is_empty() {
            return Ok(Sheet::new());
        }

        // Extract column names from schema
        let column_names: Vec<String> = schema.fields().iter().map(|f| f.name().clone()).collect();

        // Build data rows with header
        let mut data: Vec<Vec<CellValue>> = Vec::new();

        // Add header row
        let header: Vec<CellValue> = column_names
            .iter()
            .map(|n| CellValue::String(n.clone()))
            .collect();
        data.push(header);

        // Process each batch
        for batch in &batches {
            let num_rows = batch.num_rows();
            for row_idx in 0..num_rows {
                let mut row: Vec<CellValue> = Vec::with_capacity(batch.num_columns());
                for col_idx in 0..batch.num_columns() {
                    let array = batch.column(col_idx);
                    let cell = arrow_array_to_cell(array, row_idx);
                    row.push(cell);
                }
                data.push(row);
            }
        }

        let mut sheet = Sheet::with_name("Sheet1");
        *sheet.data_mut() = data;
        sheet.name_columns_by_row(0)?;

        Ok(sheet)
    }

    /// Save the sheet to a Parquet file
    ///
    /// Requires columns to be named.
    ///
    /// # Example
    /// ```no_run
    /// use piptable_sheet::Sheet;
    ///
    /// let mut sheet = Sheet::from_data(vec![
    ///     vec!["name", "age"],
    ///     vec!["Alice", "30"],
    /// ]);
    /// sheet.name_columns_by_row(0).unwrap();
    /// sheet.save_as_parquet("output.parquet").unwrap();
    /// ```
    pub fn save_as_parquet<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let column_names = self.column_names().ok_or_else(|| {
            SheetError::ColumnsNotNamed("Columns must be named to export as Parquet".to_string())
        })?;

        if self.row_count() <= 1 {
            // Only header row or empty - create empty parquet with schema
            let fields: Vec<Field> = column_names
                .iter()
                .map(|name| Field::new(name, DataType::Utf8, true))
                .collect();
            let schema = Arc::new(Schema::new(fields));
            let file = File::create(path)?;
            let writer = ArrowWriter::try_new(file, schema.clone(), None)?;
            writer.close()?;
            return Ok(());
        }

        // Analyze column types from data (skip header row)
        let data_rows: Vec<&Vec<CellValue>> = self.data().iter().skip(1).collect();
        let num_cols = column_names.len();

        // Infer types for each column
        let col_types: Vec<DataType> = (0..num_cols)
            .map(|col_idx| infer_column_type(&data_rows, col_idx))
            .collect();

        // Build schema
        let fields: Vec<Field> = column_names
            .iter()
            .zip(col_types.iter())
            .map(|(name, dtype)| Field::new(name, dtype.clone(), true))
            .collect();
        let schema = Arc::new(Schema::new(fields));

        // Build Arrow arrays for each column
        let arrays: Vec<ArrayRef> = (0..num_cols)
            .map(|col_idx| build_arrow_array(&data_rows, col_idx, &col_types[col_idx]))
            .collect();

        // Create RecordBatch
        let batch = RecordBatch::try_new(schema.clone(), arrays)?;

        // Write to file
        let file = File::create(path)?;
        let mut writer = ArrowWriter::try_new(file, schema, None)?;
        writer.write(&batch)?;
        writer.close()?;

        Ok(())
    }
}

/// Convert an Arrow array element at a given index to a CellValue
fn arrow_array_to_cell(array: &ArrayRef, idx: usize) -> CellValue {
    if array.is_null(idx) {
        return CellValue::Null;
    }

    match array.data_type() {
        DataType::Boolean => {
            let arr = array.as_any().downcast_ref::<BooleanArray>().unwrap();
            CellValue::Bool(arr.value(idx))
        }
        DataType::Int8 | DataType::Int16 | DataType::Int32 | DataType::Int64 => {
            // Try to downcast to Int64Array first, then others
            if let Some(arr) = array.as_any().downcast_ref::<Int64Array>() {
                CellValue::Int(arr.value(idx))
            } else if let Some(arr) = array.as_any().downcast_ref::<arrow::array::Int32Array>() {
                CellValue::Int(i64::from(arr.value(idx)))
            } else if let Some(arr) = array.as_any().downcast_ref::<arrow::array::Int16Array>() {
                CellValue::Int(i64::from(arr.value(idx)))
            } else if let Some(arr) = array.as_any().downcast_ref::<arrow::array::Int8Array>() {
                CellValue::Int(i64::from(arr.value(idx)))
            } else {
                CellValue::String(format!("<int:{}>", array.data_type()))
            }
        }
        DataType::UInt8 | DataType::UInt16 | DataType::UInt32 | DataType::UInt64 => {
            if let Some(arr) = array.as_any().downcast_ref::<arrow::array::UInt64Array>() {
                CellValue::Int(arr.value(idx) as i64)
            } else if let Some(arr) = array.as_any().downcast_ref::<arrow::array::UInt32Array>() {
                CellValue::Int(i64::from(arr.value(idx)))
            } else if let Some(arr) = array.as_any().downcast_ref::<arrow::array::UInt16Array>() {
                CellValue::Int(i64::from(arr.value(idx)))
            } else if let Some(arr) = array.as_any().downcast_ref::<arrow::array::UInt8Array>() {
                CellValue::Int(i64::from(arr.value(idx)))
            } else {
                CellValue::String(format!("<uint:{}>", array.data_type()))
            }
        }
        DataType::Float16 | DataType::Float32 | DataType::Float64 => {
            if let Some(arr) = array.as_any().downcast_ref::<Float64Array>() {
                CellValue::Float(arr.value(idx))
            } else if let Some(arr) = array.as_any().downcast_ref::<arrow::array::Float32Array>() {
                CellValue::Float(f64::from(arr.value(idx)))
            } else {
                CellValue::String(format!("<float:{}>", array.data_type()))
            }
        }
        DataType::Utf8 | DataType::LargeUtf8 => {
            if let Some(arr) = array.as_any().downcast_ref::<StringArray>() {
                CellValue::String(arr.value(idx).to_string())
            } else if let Some(arr) = array
                .as_any()
                .downcast_ref::<arrow::array::LargeStringArray>()
            {
                CellValue::String(arr.value(idx).to_string())
            } else {
                CellValue::Null
            }
        }
        // For other types, convert to string representation
        _ => {
            let formatted = arrow::util::display::array_value_to_string(array, idx);
            match formatted {
                Ok(s) => CellValue::String(s),
                Err(_) => CellValue::String(format!("<{}>", array.data_type())),
            }
        }
    }
}

/// Infer the Arrow DataType for a column based on cell values
fn infer_column_type(rows: &[&Vec<CellValue>], col_idx: usize) -> DataType {
    let mut has_bool = false;
    let mut has_int = false;
    let mut has_float = false;
    let mut has_string = false;

    for row in rows {
        if col_idx >= row.len() {
            continue;
        }
        match &row[col_idx] {
            CellValue::Null => {}
            CellValue::Bool(_) => has_bool = true,
            CellValue::Int(_) => has_int = true,
            CellValue::Float(_) => has_float = true,
            CellValue::String(_) => has_string = true,
        }
    }

    // Priority: String > Float > Int > Bool (wider types win)
    if has_string {
        DataType::Utf8
    } else if has_float {
        DataType::Float64
    } else if has_int {
        DataType::Int64
    } else if has_bool {
        DataType::Boolean
    } else {
        DataType::Utf8 // Default to string for empty/null-only columns
    }
}

/// Build an Arrow array from column data
fn build_arrow_array(rows: &[&Vec<CellValue>], col_idx: usize, dtype: &DataType) -> ArrayRef {
    match dtype {
        DataType::Boolean => {
            let values: Vec<Option<bool>> = rows
                .iter()
                .map(|row| row.get(col_idx).and_then(CellValue::as_bool))
                .collect();
            Arc::new(BooleanArray::from(values))
        }
        DataType::Int64 => {
            let values: Vec<Option<i64>> = rows
                .iter()
                .map(|row| row.get(col_idx).and_then(CellValue::as_int))
                .collect();
            Arc::new(Int64Array::from(values))
        }
        DataType::Float64 => {
            let values: Vec<Option<f64>> = rows
                .iter()
                .map(|row| row.get(col_idx).and_then(CellValue::as_float))
                .collect();
            Arc::new(Float64Array::from(values))
        }
        _ => {
            // Default to string for Utf8 and any other types
            let values: Vec<Option<String>> = rows
                .iter()
                .map(|row| {
                    row.get(col_idx).and_then(|cell| {
                        if cell.is_null() {
                            None
                        } else {
                            Some(cell.as_str())
                        }
                    })
                })
                .collect();
            Arc::new(StringArray::from(values))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_parquet_roundtrip() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.parquet");

        // Create test data
        let mut sheet = Sheet::from_data(vec![
            vec!["name", "age", "active"],
            vec!["Alice", "30", "true"],
            vec!["Bob", "25", "false"],
        ]);
        sheet.name_columns_by_row(0).unwrap();

        // Save to parquet
        sheet.save_as_parquet(&file_path).unwrap();

        // Load back
        let loaded = Sheet::from_parquet(&file_path).unwrap();

        assert_eq!(loaded.row_count(), sheet.row_count());
        assert_eq!(loaded.col_count(), sheet.col_count());
        assert!(loaded.column_names().is_some());
    }

    #[test]
    fn test_parquet_with_types() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("typed.parquet");

        // Create sheet with typed data
        let mut sheet = Sheet::new();
        sheet.data_mut().push(vec![
            CellValue::String("id".to_string()),
            CellValue::String("score".to_string()),
            CellValue::String("active".to_string()),
        ]);
        sheet.data_mut().push(vec![
            CellValue::Int(1),
            CellValue::Float(95.5),
            CellValue::Bool(true),
        ]);
        sheet.data_mut().push(vec![
            CellValue::Int(2),
            CellValue::Float(87.3),
            CellValue::Bool(false),
        ]);
        sheet.name_columns_by_row(0).unwrap();

        // Save
        sheet.save_as_parquet(&file_path).unwrap();

        // Load and verify types are preserved
        let loaded = Sheet::from_parquet(&file_path).unwrap();

        // Check values (skip header row)
        assert!(matches!(loaded.get(1, 0).unwrap(), CellValue::Int(1)));
        assert!(
            matches!(loaded.get(1, 1).unwrap(), CellValue::Float(f) if (f - 95.5).abs() < 0.001)
        );
        assert!(matches!(loaded.get(1, 2).unwrap(), CellValue::Bool(true)));
    }

    #[test]
    fn test_parquet_with_nulls() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("nulls.parquet");

        let mut sheet = Sheet::new();
        sheet.data_mut().push(vec![
            CellValue::String("a".to_string()),
            CellValue::String("b".to_string()),
        ]);
        sheet
            .data_mut()
            .push(vec![CellValue::Int(1), CellValue::Null]);
        sheet.data_mut().push(vec![
            CellValue::Null,
            CellValue::String("hello".to_string()),
        ]);
        sheet.name_columns_by_row(0).unwrap();

        sheet.save_as_parquet(&file_path).unwrap();
        let loaded = Sheet::from_parquet(&file_path).unwrap();

        // Verify nulls are preserved
        assert!(loaded.get(1, 1).unwrap().is_null());
        assert!(loaded.get(2, 0).unwrap().is_null());
    }

    #[test]
    fn test_parquet_empty_sheet() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("empty.parquet");

        let mut sheet = Sheet::from_data(vec![vec!["col1", "col2"]]);
        sheet.name_columns_by_row(0).unwrap();

        sheet.save_as_parquet(&file_path).unwrap();
        let loaded = Sheet::from_parquet(&file_path).unwrap();

        // Empty sheet should still have schema
        assert_eq!(loaded.row_count(), 0);
    }
}
