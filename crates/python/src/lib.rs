//! Python bindings for piptable Sheet/Book API
//!
//! Provides a pyexcel-like interface for working with tabular data from Python.
//! Integrates seamlessly with pandas for data analysis and visualization.
//!
//! # Example
//!
//! ```python
//! from piptable import Sheet, Book
//!
//! # Load from CSV
//! sheet = Sheet.from_csv("data.csv", has_headers=True)
//!
//! # Convert to pandas for analysis
//! df = sheet.to_pandas()
//! df.describe()
//!
//! # Use with matplotlib/plotly
//! import matplotlib.pyplot as plt
//! df.plot.bar(x="region", y="sales")
//! plt.show()
//!
//! # Create from pandas
//! import pandas as pd
//! df = pd.DataFrame({"name": ["Alice", "Bob"], "age": [30, 25]})
//! sheet = Sheet.from_pandas(df)
//! sheet.save_as_xlsx("output.xlsx")
//! ```

use piptable_sheet::{
    Book as RustBook, CellValue as RustCellValue, CsvOptions as RustCsvOptions,
    Sheet as RustSheet, XlsxReadOptions as RustXlsxReadOptions,
};
use pyo3::exceptions::{PyImportError, PyIndexError, PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

/// Convert a Rust CellValue to a Python object
fn cell_value_to_py(py: Python<'_>, value: &RustCellValue) -> PyObject {
    match value {
        RustCellValue::Null => py.None(),
        RustCellValue::Bool(b) => b.into_pyobject(py).unwrap().to_owned().into_any().unbind(),
        RustCellValue::Int(i) => i.into_pyobject(py).unwrap().to_owned().into_any().unbind(),
        RustCellValue::Float(f) => f.into_pyobject(py).unwrap().to_owned().into_any().unbind(),
        RustCellValue::String(s) => s.into_pyobject(py).unwrap().to_owned().into_any().unbind(),
    }
}

/// Convert a Python object to a Rust CellValue
fn py_to_cell_value(obj: &Bound<'_, PyAny>) -> PyResult<RustCellValue> {
    if obj.is_none() {
        return Ok(RustCellValue::Null);
    }
    if let Ok(b) = obj.extract::<bool>() {
        return Ok(RustCellValue::Bool(b));
    }
    if let Ok(i) = obj.extract::<i64>() {
        return Ok(RustCellValue::Int(i));
    }
    if let Ok(f) = obj.extract::<f64>() {
        return Ok(RustCellValue::Float(f));
    }
    if let Ok(s) = obj.extract::<String>() {
        return Ok(RustCellValue::String(s));
    }
    Err(PyValueError::new_err(format!(
        "Cannot convert {} to CellValue",
        obj.get_type().name()?
    )))
}

/// Convert a Python list to a Vec<CellValue>
fn py_list_to_row(list: &Bound<'_, PyList>) -> PyResult<Vec<RustCellValue>> {
    list.iter().map(|item| py_to_cell_value(&item)).collect()
}

/// Convert a Python list of lists to Vec<Vec<CellValue>>
fn py_to_data(list: &Bound<'_, PyList>) -> PyResult<Vec<Vec<RustCellValue>>> {
    list.iter()
        .map(|row| {
            let row_list = row.downcast::<PyList>()?;
            py_list_to_row(row_list)
        })
        .collect()
}

/// A sheet representing a 2D grid of cells
///
/// Provides a pyexcel-like API for working with tabular data.
#[pyclass]
#[derive(Clone)]
pub struct Sheet {
    inner: RustSheet,
}

#[pymethods]
impl Sheet {
    /// Create a new empty sheet
    #[new]
    fn new() -> Self {
        Sheet {
            inner: RustSheet::new(),
        }
    }

    /// Create a sheet from a 2D list of values
    ///
    /// Args:
    ///     data: A list of lists containing the cell values
    ///
    /// Returns:
    ///     A new Sheet instance
    #[staticmethod]
    fn from_data(data: &Bound<'_, PyList>) -> PyResult<Self> {
        let converted = py_to_data(data)?;
        let mut sheet = RustSheet::new();
        *sheet.data_mut() = converted;
        Ok(Sheet { inner: sheet })
    }

    /// Load a sheet from a CSV file
    ///
    /// Args:
    ///     path: Path to the CSV file
    ///     has_headers: If True, first row is used as column names
    ///     delimiter: Column delimiter (default: ',')
    ///
    /// Returns:
    ///     A new Sheet instance
    #[staticmethod]
    #[pyo3(signature = (path, has_headers=false, delimiter=None))]
    fn from_csv(path: &str, has_headers: bool, delimiter: Option<char>) -> PyResult<Self> {
        let mut options = RustCsvOptions::default();
        if let Some(d) = delimiter {
            if !d.is_ascii() {
                return Err(PyValueError::new_err(
                    "delimiter must be a single-byte ASCII character",
                ));
            }
            options = options.with_delimiter(d as u8);
        }

        let mut sheet = RustSheet::from_csv_with_options(path, options)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        if has_headers && sheet.row_count() > 0 {
            sheet
                .name_columns_by_row(0)
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        }

        Ok(Sheet { inner: sheet })
    }

    /// Load a sheet from an Excel file (first sheet)
    ///
    /// Args:
    ///     path: Path to the Excel file
    ///     has_headers: If True, first row is used as column names
    ///
    /// Returns:
    ///     A new Sheet instance
    #[staticmethod]
    #[pyo3(signature = (path, has_headers=false))]
    fn from_xlsx(path: &str, has_headers: bool) -> PyResult<Self> {
        let options = RustXlsxReadOptions::default().with_headers(has_headers);
        let sheet = RustSheet::from_xlsx_with_options(path, options)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(Sheet { inner: sheet })
    }

    /// Load a specific sheet from an Excel file by name
    ///
    /// Args:
    ///     path: Path to the Excel file
    ///     sheet_name: Name of the sheet to load
    ///     has_headers: If True, first row is used as column names
    ///
    /// Returns:
    ///     A new Sheet instance
    #[staticmethod]
    #[pyo3(signature = (path, sheet_name, has_headers=false))]
    fn from_xlsx_sheet(path: &str, sheet_name: &str, has_headers: bool) -> PyResult<Self> {
        let options = RustXlsxReadOptions::default().with_headers(has_headers);
        let sheet = RustSheet::from_xlsx_sheet_with_options(path, sheet_name, options)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(Sheet { inner: sheet })
    }

    /// Get the sheet name
    fn name(&self) -> &str {
        self.inner.name()
    }

    /// Set the sheet name
    fn set_name(&mut self, name: &str) {
        self.inner.set_name(name);
    }

    /// Get the number of rows
    fn row_count(&self) -> usize {
        self.inner.row_count()
    }

    /// Get the number of columns
    fn col_count(&self) -> usize {
        self.inner.col_count()
    }

    /// Check if the sheet is empty
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Get a cell value by row and column index (0-based)
    fn get(&self, py: Python<'_>, row: usize, col: usize) -> PyResult<PyObject> {
        let value = self
            .inner
            .get(row, col)
            .map_err(|e| PyIndexError::new_err(e.to_string()))?;
        Ok(cell_value_to_py(py, value))
    }

    /// Set a cell value by row and column index (0-based)
    fn set(&mut self, row: usize, col: usize, value: &Bound<'_, PyAny>) -> PyResult<()> {
        let cell_value = py_to_cell_value(value)?;
        self.inner
            .set(row, col, cell_value)
            .map_err(|e| PyIndexError::new_err(e.to_string()))
    }

    /// Get a cell value by row index and column name
    fn get_by_name(&self, py: Python<'_>, row: usize, col_name: &str) -> PyResult<PyObject> {
        let value = self
            .inner
            .get_by_name(row, col_name)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(cell_value_to_py(py, value))
    }

    /// Get an entire row by index (0-based)
    fn row(&self, py: Python<'_>, index: usize) -> PyResult<PyObject> {
        let row = self
            .inner
            .row(index)
            .map_err(|e| PyIndexError::new_err(e.to_string()))?;
        let list = PyList::new(py, row.iter().map(|v| cell_value_to_py(py, v)))?;
        Ok(list.into())
    }

    /// Get an entire column by index (0-based)
    fn column(&self, py: Python<'_>, index: usize) -> PyResult<PyObject> {
        let col = self
            .inner
            .column(index)
            .map_err(|e| PyIndexError::new_err(e.to_string()))?;
        let list = PyList::new(py, col.iter().map(|v| cell_value_to_py(py, v)))?;
        Ok(list.into())
    }

    /// Get an entire column by name
    fn column_by_name(&self, py: Python<'_>, name: &str) -> PyResult<PyObject> {
        let col = self
            .inner
            .column_by_name(name)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        let list = PyList::new(py, col.iter().map(|v| cell_value_to_py(py, v)))?;
        Ok(list.into())
    }

    /// Use the specified row as column headers
    fn name_columns_by_row(&mut self, row_index: usize) -> PyResult<()> {
        self.inner
            .name_columns_by_row(row_index)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Get column names (if set)
    fn column_names(&self) -> Option<Vec<String>> {
        self.inner.column_names().cloned()
    }

    /// Append a row to the end of the sheet
    fn row_append(&mut self, data: &Bound<'_, PyList>) -> PyResult<()> {
        let row = py_list_to_row(data)?;
        self.inner
            .row_append(row)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Insert a row at a specific index
    fn row_insert(&mut self, index: usize, data: &Bound<'_, PyList>) -> PyResult<()> {
        let row = py_list_to_row(data)?;
        self.inner
            .row_insert(index, row)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Delete a row at a specific index
    fn row_delete(&mut self, index: usize) -> PyResult<()> {
        self.inner
            .row_delete(index)
            .map_err(|e| PyIndexError::new_err(e.to_string()))?;
        Ok(())
    }

    /// Append a column to the end of each row
    fn column_append(&mut self, data: &Bound<'_, PyList>) -> PyResult<()> {
        let col = py_list_to_row(data)?;
        self.inner
            .column_append(col)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Delete a column at a specific index
    fn column_delete(&mut self, index: usize) -> PyResult<()> {
        self.inner
            .column_delete(index)
            .map_err(|e| PyIndexError::new_err(e.to_string()))?;
        Ok(())
    }

    /// Delete a column by name
    fn column_delete_by_name(&mut self, name: &str) -> PyResult<()> {
        self.inner
            .column_delete_by_name(name)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(())
    }

    /// Convert to a 2D list
    fn to_list(&self, py: Python<'_>) -> PyResult<PyObject> {
        let data = self.inner.to_array();
        let mut rows: Vec<PyObject> = Vec::with_capacity(data.len());
        for row in data.iter() {
            let inner = PyList::new(py, row.iter().map(|v| cell_value_to_py(py, v)))?;
            rows.push(inner.into_any().unbind());
        }
        Ok(PyList::new(py, rows)?.into())
    }

    /// Convert to a dictionary (column name -> values)
    fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = self
            .inner
            .to_dict()
            .ok_or_else(|| PyRuntimeError::new_err("Columns not named"))?;

        let py_dict = PyDict::new(py);
        for (name, values) in dict {
            let list = PyList::new(py, values.iter().map(|v| cell_value_to_py(py, v)))?;
            py_dict.set_item(name, list)?;
        }
        Ok(py_dict.into())
    }

    /// Convert to a list of records (list of dictionaries)
    ///
    /// Each row becomes a dictionary with column names as keys.
    /// Requires columns to be named (call name_columns_by_row first or use has_headers=True).
    ///
    /// Returns:
    ///     A list of dictionaries, one per row
    ///
    /// Example:
    ///     >>> sheet = Sheet.from_csv("data.csv", has_headers=True)
    ///     >>> for record in sheet.to_records():
    ///     ...     print(f"{record['name']} is {record['age']} years old")
    fn to_records(&self, py: Python<'_>) -> PyResult<PyObject> {
        let records = self
            .inner
            .to_records()
            .ok_or_else(|| PyRuntimeError::new_err("Columns not named"))?;

        let py_list = PyList::empty(py);
        for record in records {
            let py_dict = PyDict::new(py);
            for (name, value) in record {
                py_dict.set_item(name, cell_value_to_py(py, &value))?;
            }
            py_list.append(py_dict)?;
        }
        Ok(py_list.into())
    }

    /// Create a sheet from a list of records (list of dictionaries)
    ///
    /// All records should have the same keys. Column order is determined
    /// by the first record.
    ///
    /// Args:
    ///     records: A list of dictionaries with the same keys
    ///
    /// Returns:
    ///     A new Sheet instance with columns named after the dictionary keys
    ///
    /// Example:
    ///     >>> records = [
    ///     ...     {"name": "Alice", "age": 30},
    ///     ...     {"name": "Bob", "age": 25}
    ///     ... ]
    ///     >>> sheet = Sheet.from_records(records)
    #[staticmethod]
    fn from_records(_py: Python<'_>, records: &Bound<'_, PyList>) -> PyResult<Self> {
        use indexmap::IndexMap;
        use std::collections::HashSet;

        if records.is_empty() {
            return Ok(Sheet {
                inner: RustSheet::new(),
            });
        }

        let mut rust_records: Vec<IndexMap<String, RustCellValue>> = Vec::new();
        let mut expected_keys: Option<HashSet<String>> = None;

        for (idx, item) in records.iter().enumerate() {
            let dict = item.downcast::<PyDict>()?;
            let mut record = IndexMap::new();
            let mut current_keys = HashSet::new();

            for (key, value) in dict.iter() {
                let key_str: String = key.extract()?;
                current_keys.insert(key_str.clone());
                let cell_value = py_to_cell_value(&value)?;
                record.insert(key_str, cell_value);
            }

            // Validate keys match the first record
            match &expected_keys {
                None => {
                    expected_keys = Some(current_keys);
                }
                Some(expected) => {
                    if &current_keys != expected {
                        let missing: Vec<_> = expected.difference(&current_keys).collect();
                        let extra: Vec<_> = current_keys.difference(expected).collect();
                        return Err(PyValueError::new_err(format!(
                            "Record at index {} has mismatched keys. Missing: {:?}, Extra: {:?}",
                            idx, missing, extra
                        )));
                    }
                }
            }

            rust_records.push(record);
        }

        let inner = RustSheet::from_records(rust_records)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        Ok(Sheet { inner })
    }

    /// Convert to a pandas DataFrame
    ///
    /// Requires pandas to be installed. If columns are named, they will be
    /// used as DataFrame column names.
    ///
    /// Returns:
    ///     A pandas DataFrame
    ///
    /// Raises:
    ///     ImportError: If pandas is not installed
    ///
    /// Example:
    ///     >>> sheet = Sheet.from_csv("data.csv", has_headers=True)
    ///     >>> df = sheet.to_pandas()
    ///     >>> df.plot.bar(x="region", y="sales")
    fn to_pandas(&self, py: Python<'_>) -> PyResult<PyObject> {
        // Import pandas
        let pd = py.import("pandas").map_err(|_| {
            PyImportError::new_err(
                "pandas is required for to_pandas(). Install with: pip install pandas",
            )
        })?;

        // If columns are named, use to_dict for proper column names
        if let Some(col_names) = self.inner.column_names() {
            let py_dict = PyDict::new(py);
            for (i, name) in col_names.iter().enumerate() {
                let col = self.inner.column(i)
                    .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
                let list = PyList::new(py, col.iter().map(|v| cell_value_to_py(py, v)))?;
                py_dict.set_item(name, list)?;
            }
            pd.call_method1("DataFrame", (py_dict,))
                .map(|obj| obj.unbind())
        } else {
            // No column names - use row data directly
            let data = self.inner.to_array();
            let mut rows: Vec<PyObject> = Vec::with_capacity(data.len());
            for row in data.iter() {
                let inner = PyList::new(py, row.iter().map(|v| cell_value_to_py(py, v)))?;
                rows.push(inner.into_any().unbind());
            }
            let py_list = PyList::new(py, rows)?;
            pd.call_method1("DataFrame", (py_list,))
                .map(|obj| obj.unbind())
        }
    }

    /// Create a sheet from a pandas DataFrame
    ///
    /// Args:
    ///     df: A pandas DataFrame
    ///
    /// Returns:
    ///     A new Sheet instance with DataFrame columns as named columns
    ///
    /// Note:
    ///     Only basic Python types (bool, int, float, str, None) are supported.
    ///     Datetime columns with missing values (pd.NaT) or other pandas-specific
    ///     types must be converted to supported types before calling this method
    ///     (e.g., `df['date_col'] = df['date_col'].astype(str)`).
    ///
    /// Example:
    ///     >>> import pandas as pd
    ///     >>> df = pd.DataFrame({"name": ["Alice", "Bob"], "age": [30, 25]})
    ///     >>> sheet = Sheet.from_pandas(df)
    #[staticmethod]
    fn from_pandas(_py: Python<'_>, df: &Bound<'_, PyAny>) -> PyResult<Self> {
        // Get column names (convert to strings - pandas allows non-string column names)
        let columns = df.getattr("columns")?;
        let col_list = columns.call_method0("tolist")?;
        let col_names: Vec<String> = col_list
            .try_iter()?
            .map(|item| {
                let item = item?;
                // Use Python's str() for robust string conversion
                item.str()?.extract::<String>()
            })
            .collect::<PyResult<Vec<String>>>()?;

        // Get values as list of lists
        let values = df.getattr("values")?;
        let rows_list = values.call_method0("tolist")?;

        // Create sheet with data
        let mut sheet = RustSheet::new();

        // Add header row
        let header_row: Vec<RustCellValue> = col_names
            .iter()
            .map(|s| RustCellValue::String(s.clone()))
            .collect();
        sheet.data_mut().push(header_row);

        // Add data rows
        for row in rows_list.try_iter()? {
            let row = row?;
            let row_list = row.downcast::<PyList>()?;
            let converted = py_list_to_row(row_list)?;
            sheet.data_mut().push(converted);
        }

        // Name columns by first row
        sheet
            .name_columns_by_row(0)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        Ok(Sheet { inner: sheet })
    }

    /// Save the sheet to a CSV file
    #[pyo3(signature = (path, delimiter=None))]
    fn save_as_csv(&self, path: &str, delimiter: Option<char>) -> PyResult<()> {
        let mut options = RustCsvOptions::default();
        if let Some(d) = delimiter {
            if !d.is_ascii() {
                return Err(PyValueError::new_err(
                    "delimiter must be a single-byte ASCII character",
                ));
            }
            options = options.with_delimiter(d as u8);
        }
        self.inner
            .save_as_csv_with_options(path, options)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Save the sheet to an Excel file
    fn save_as_xlsx(&self, path: &str) -> PyResult<()> {
        self.inner
            .save_as_xlsx(path)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Get CSV string representation
    fn to_csv_string(&self) -> PyResult<String> {
        self.inner
            .to_csv_string()
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "Sheet(name='{}', rows={}, cols={})",
            self.inner.name(),
            self.inner.row_count(),
            self.inner.col_count()
        )
    }

    fn __len__(&self) -> usize {
        self.inner.row_count()
    }
}

/// A book containing multiple sheets
///
/// Provides multi-sheet workbook functionality.
#[pyclass]
#[derive(Clone)]
pub struct Book {
    inner: RustBook,
}

#[pymethods]
impl Book {
    /// Create a new empty book
    #[new]
    fn new() -> Self {
        Book {
            inner: RustBook::new(),
        }
    }

    /// Load a book from an Excel file (all sheets)
    ///
    /// Args:
    ///     path: Path to the Excel file
    ///     has_headers: If True, first row of each sheet is used as column names
    ///
    /// Returns:
    ///     A new Book instance
    #[staticmethod]
    #[pyo3(signature = (path, has_headers=false))]
    fn from_xlsx(path: &str, has_headers: bool) -> PyResult<Self> {
        let options = RustXlsxReadOptions::default().with_headers(has_headers);
        let book = RustBook::from_xlsx_with_options(path, options)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(Book { inner: book })
    }

    /// Load a book from a directory of CSV files
    ///
    /// Args:
    ///     path: Path to the directory
    ///     has_headers: If True, first row of each file is used as column names
    ///
    /// Returns:
    ///     A new Book instance
    #[staticmethod]
    #[pyo3(signature = (path, has_headers=false))]
    fn from_csv_dir(path: &str, has_headers: bool) -> PyResult<Self> {
        let options = RustCsvOptions::default().with_headers(has_headers);
        let book = RustBook::from_csv_dir_with_options(path, options)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(Book { inner: book })
    }

    /// Get sheet names from an Excel file without loading data
    #[staticmethod]
    fn xlsx_sheet_names(path: &str) -> PyResult<Vec<String>> {
        RustBook::xlsx_sheet_names(path).map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Get the book name
    fn name(&self) -> &str {
        self.inner.name()
    }

    /// Get the number of sheets
    fn sheet_count(&self) -> usize {
        self.inner.sheet_count()
    }

    /// Check if the book is empty
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Get all sheet names
    fn sheet_names(&self) -> Vec<String> {
        self.inner.sheet_names().iter().map(|s| s.to_string()).collect()
    }

    /// Check if a sheet exists
    fn has_sheet(&self, name: &str) -> bool {
        self.inner.has_sheet(name)
    }

    /// Get a sheet by name (returns a copy)
    ///
    /// Note: This returns a copy of the sheet. Modifications to the returned
    /// sheet will not affect the book. Use `add_sheet` to replace a sheet
    /// after modifications.
    fn get_sheet(&self, name: &str) -> PyResult<Sheet> {
        let sheet = self
            .inner
            .get_sheet(name)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(Sheet {
            inner: sheet.clone(),
        })
    }

    /// Add a sheet to the book
    fn add_sheet(&mut self, name: &str, sheet: &Sheet) -> PyResult<()> {
        self.inner
            .add_sheet(name, sheet.inner.clone())
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Remove a sheet by name
    fn remove_sheet(&mut self, name: &str) -> PyResult<Sheet> {
        let sheet = self
            .inner
            .remove_sheet(name)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(Sheet { inner: sheet })
    }

    /// Rename a sheet
    fn rename_sheet(&mut self, old_name: &str, new_name: &str) -> PyResult<()> {
        self.inner
            .rename_sheet(old_name, new_name)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Save the book to an Excel file
    fn save_as_xlsx(&self, path: &str) -> PyResult<()> {
        self.inner
            .save_as_xlsx(path)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "Book(name='{}', sheets={})",
            self.inner.name(),
            self.inner.sheet_count()
        )
    }

    fn __len__(&self) -> usize {
        self.inner.sheet_count()
    }
}

/// Python module for piptable
#[pymodule]
fn piptable(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Sheet>()?;
    m.add_class::<Book>()?;
    Ok(())
}
