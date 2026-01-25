//! Python UDF runtime for piptable interpreter.
//!
//! Provides the ability to register and call Python functions from piptable scripts.
//!
//! # Example
//!
//! ```vba
//! ' Register a Python lambda
//! register_python("double", "lambda x: x * 2")
//! dim result = double(21)  ' 42
//!
//! ' Register from a file
//! register_python("clean", "transforms.py", "clean_data")
//! ```

use arrow::array::{Array, AsArray};
use arrow::datatypes::DataType;
use piptable_core::{PipError, PipResult, Value};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyModule, PyTuple};
use std::collections::HashMap;
use std::ffi::CString;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

/// A registered Python function.
pub struct PyFunctionDef {
    /// The Python callable object.
    callable: PyObject,
}

/// Python runtime for executing UDFs.
pub struct PythonRuntime {
    /// Registered Python functions by name.
    functions: Arc<RwLock<HashMap<String, PyFunctionDef>>>,
}

impl PythonRuntime {
    /// Create a new Python runtime.
    pub fn new() -> PipResult<Self> {
        // Initialize Python interpreter
        pyo3::prepare_freethreaded_python();

        Ok(Self {
            functions: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Register a Python function from inline code (callable expression, e.g. lambda).
    ///
    /// Note: Only Python expressions are supported, not statements like `def`.
    /// Use `register_from_file` for multi-line function definitions.
    ///
    /// # Arguments
    /// * `name` - The name to register the function under
    /// * `code` - Python callable expression (e.g., "lambda x: x * 2")
    pub async fn register_inline(&self, name: &str, code: &str) -> PipResult<()> {
        let name_owned = name.to_string();
        let code_owned = code.to_string();

        let callable = Python::with_gil(|py| -> PyResult<PyObject> {
            // Create CString for eval
            let code_cstr = CString::new(code_owned.as_str())
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

            // Use eval to evaluate the expression
            let result = py.eval(&code_cstr, None, None)?;

            // Check if it's callable
            if !result.is_callable() {
                return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
                    "'{}' is not callable",
                    code_owned
                )));
            }

            Ok(result.unbind())
        })
        .map_err(|e| PipError::Plugin {
            plugin: "python".to_string(),
            message: format!("Failed to register '{}': {}", name_owned, e),
        })?;

        let func_def = PyFunctionDef { callable };

        let mut funcs = self.functions.write().await;
        funcs.insert(name_owned, func_def);

        Ok(())
    }

    /// Register a Python function from a file.
    ///
    /// # Arguments
    /// * `name` - The name to register the function under
    /// * `file_path` - Path to the Python file
    /// * `function_name` - Name of the function in the file
    pub async fn register_from_file(
        &self,
        name: &str,
        file_path: &str,
        function_name: &str,
    ) -> PipResult<()> {
        let path = Path::new(file_path);
        if !path.exists() {
            return Err(PipError::Plugin {
                plugin: "python".to_string(),
                message: format!("Python file not found: {}", file_path),
            });
        }

        let code = tokio::fs::read_to_string(path)
            .await
            .map_err(PipError::Io)?;
        let name_owned = name.to_string();
        let file_path_owned = file_path.to_string();
        let function_name_owned = function_name.to_string();

        let callable = Python::with_gil(|py| -> PyResult<PyObject> {
            // Create CStrings for PyModule::from_code
            let code_cstr = CString::new(code.as_str())
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
            let file_cstr = CString::new(file_path_owned.as_str())
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
            let module_cstr = CString::new("piptable_udf")
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

            // Create a module from the file
            let module = PyModule::from_code(py, &code_cstr, &file_cstr, &module_cstr)?;

            // Get the function from the module
            let func = module.getattr(function_name_owned.as_str())?;

            if !func.is_callable() {
                return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
                    "'{}' is not callable in {}",
                    function_name_owned, file_path_owned
                )));
            }

            Ok(func.unbind())
        })
        .map_err(|e| PipError::Plugin {
            plugin: "python".to_string(),
            message: format!(
                "Failed to load '{}' from '{}': {}",
                function_name_owned, file_path_owned, e
            ),
        })?;

        let func_def = PyFunctionDef { callable };

        let mut funcs = self.functions.write().await;
        funcs.insert(name_owned, func_def);

        Ok(())
    }

    /// Check if a function is registered.
    pub async fn has_function(&self, name: &str) -> bool {
        let funcs = self.functions.read().await;
        funcs.contains_key(name)
    }

    /// Call a registered Python function.
    ///
    /// # Arguments
    /// * `name` - The registered function name
    /// * `args` - Arguments to pass to the function
    pub async fn call(&self, name: &str, args: Vec<Value>) -> PipResult<Value> {
        // Get the callable while holding the lock briefly
        let callable = {
            let funcs = self.functions.read().await;
            let func_def = funcs.get(name).ok_or_else(|| PipError::Plugin {
                plugin: "python".to_string(),
                message: format!("Python function '{}' not registered", name),
            })?;
            // Clone the PyObject reference (just increments refcount)
            Python::with_gil(|py| func_def.callable.clone_ref(py))
        };

        let name_owned = name.to_string();

        // Run Python code in a blocking task to avoid starving the async runtime.
        // The GIL blocks the thread, so we offload to tokio's blocking thread pool.
        tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| {
                // Convert arguments to Python objects
                let py_args: Vec<PyObject> = args
                    .iter()
                    .map(|v| value_to_py(py, v))
                    .collect::<PyResult<Vec<_>>>()?;

                // Create tuple of arguments
                let args_tuple = PyTuple::new(py, py_args)?;

                // Call the function
                let result = callable.call1(py, args_tuple)?;

                // Convert result back to Value
                py_to_value(py, result.bind(py))
            })
        })
        .await
        .map_err(|e| PipError::Plugin {
            plugin: "python".to_string(),
            message: format!("Python task panicked: {}", e),
        })?
        .map_err(|e: PyErr| PipError::Plugin {
            plugin: "python".to_string(),
            message: format!("Error calling '{}': {}", name_owned, e),
        })
    }

    /// List all registered function names.
    #[allow(dead_code)]
    pub async fn list_functions(&self) -> Vec<String> {
        let funcs = self.functions.read().await;
        funcs.keys().cloned().collect()
    }
}

/// Convert a piptable Value to a Python object.
fn value_to_py(py: Python<'_>, value: &Value) -> PyResult<PyObject> {
    match value {
        Value::Null => Ok(py.None()),
        Value::Bool(b) => Ok(b.into_pyobject(py)?.to_owned().into_any().unbind()),
        Value::Int(i) => Ok(i.into_pyobject(py)?.to_owned().into_any().unbind()),
        Value::Float(f) => Ok(f.into_pyobject(py)?.to_owned().into_any().unbind()),
        Value::String(s) => Ok(s.into_pyobject(py)?.to_owned().into_any().unbind()),
        Value::Array(arr) => {
            let items: Vec<PyObject> = arr
                .iter()
                .map(|v| value_to_py(py, v))
                .collect::<PyResult<Vec<_>>>()?;
            let list = PyList::new(py, items)?;
            Ok(list.into_any().unbind())
        }
        Value::Object(obj) => {
            let dict = PyDict::new(py);
            for (k, v) in obj {
                dict.set_item(k, value_to_py(py, v)?)?;
            }
            Ok(dict.into_any().unbind())
        }
        Value::Table(batches) => {
            // Convert Arrow RecordBatches to list of dicts
            let list = PyList::empty(py);
            for batch in batches {
                let schema = batch.schema();
                let num_rows = batch.num_rows();

                for row_idx in 0..num_rows {
                    let dict = PyDict::new(py);
                    for (col_idx, field) in schema.fields().iter().enumerate() {
                        let col = batch.column(col_idx);
                        let py_value = arrow_value_to_py(py, col.as_ref(), row_idx)?;
                        dict.set_item(field.name(), py_value)?;
                    }
                    list.append(dict)?;
                }
            }
            Ok(list.into_any().unbind())
        }
        Value::Function { name, .. } => {
            // Return function name as string
            Ok(name.into_pyobject(py)?.to_owned().into_any().unbind())
        }
    }
}

/// Convert a Python object to a piptable Value.
#[allow(clippy::only_used_in_recursion)]
fn py_to_value(py: Python<'_>, obj: &Bound<'_, PyAny>) -> PyResult<Value> {
    if obj.is_none() {
        return Ok(Value::Null);
    }

    // Check for bool before int (bool is subclass of int in Python)
    if let Ok(b) = obj.extract::<bool>() {
        return Ok(Value::Bool(b));
    }

    if let Ok(i) = obj.extract::<i64>() {
        return Ok(Value::Int(i));
    }

    if let Ok(f) = obj.extract::<f64>() {
        return Ok(Value::Float(f));
    }

    if let Ok(s) = obj.extract::<String>() {
        return Ok(Value::String(s));
    }

    // Check for list
    if let Ok(list) = obj.downcast::<PyList>() {
        let mut arr = Vec::with_capacity(list.len());
        for item in list.iter() {
            arr.push(py_to_value(py, &item)?);
        }
        return Ok(Value::Array(arr));
    }

    // Check for dict
    if let Ok(dict) = obj.downcast::<PyDict>() {
        let mut map = HashMap::new();
        for (k, v) in dict.iter() {
            let key = k.extract::<String>()?;
            let val = py_to_value(py, &v)?;
            map.insert(key, val);
        }
        return Ok(Value::Object(map));
    }

    // Fallback: convert to string representation
    let repr = obj.repr()?.extract::<String>()?;
    Ok(Value::String(repr))
}

/// Convert an Arrow array value at a given row index to a Python object.
fn arrow_value_to_py(py: Python<'_>, array: &dyn Array, row: usize) -> PyResult<PyObject> {
    if array.is_null(row) {
        return Ok(py.None());
    }

    match array.data_type() {
        DataType::Boolean => {
            let arr = array.as_boolean();
            Ok(arr
                .value(row)
                .into_pyobject(py)?
                .to_owned()
                .into_any()
                .unbind())
        }
        DataType::Int8 => {
            let arr = array.as_primitive::<arrow::datatypes::Int8Type>();
            Ok(arr
                .value(row)
                .into_pyobject(py)?
                .to_owned()
                .into_any()
                .unbind())
        }
        DataType::Int16 => {
            let arr = array.as_primitive::<arrow::datatypes::Int16Type>();
            Ok(arr
                .value(row)
                .into_pyobject(py)?
                .to_owned()
                .into_any()
                .unbind())
        }
        DataType::Int32 => {
            let arr = array.as_primitive::<arrow::datatypes::Int32Type>();
            Ok(arr
                .value(row)
                .into_pyobject(py)?
                .to_owned()
                .into_any()
                .unbind())
        }
        DataType::Int64 => {
            let arr = array.as_primitive::<arrow::datatypes::Int64Type>();
            Ok(arr
                .value(row)
                .into_pyobject(py)?
                .to_owned()
                .into_any()
                .unbind())
        }
        DataType::UInt8 => {
            let arr = array.as_primitive::<arrow::datatypes::UInt8Type>();
            Ok(arr
                .value(row)
                .into_pyobject(py)?
                .to_owned()
                .into_any()
                .unbind())
        }
        DataType::UInt16 => {
            let arr = array.as_primitive::<arrow::datatypes::UInt16Type>();
            Ok(arr
                .value(row)
                .into_pyobject(py)?
                .to_owned()
                .into_any()
                .unbind())
        }
        DataType::UInt32 => {
            let arr = array.as_primitive::<arrow::datatypes::UInt32Type>();
            Ok(arr
                .value(row)
                .into_pyobject(py)?
                .to_owned()
                .into_any()
                .unbind())
        }
        DataType::UInt64 => {
            let arr = array.as_primitive::<arrow::datatypes::UInt64Type>();
            Ok(arr
                .value(row)
                .into_pyobject(py)?
                .to_owned()
                .into_any()
                .unbind())
        }
        DataType::Float32 => {
            let arr = array.as_primitive::<arrow::datatypes::Float32Type>();
            Ok(arr
                .value(row)
                .into_pyobject(py)?
                .to_owned()
                .into_any()
                .unbind())
        }
        DataType::Float64 => {
            let arr = array.as_primitive::<arrow::datatypes::Float64Type>();
            Ok(arr
                .value(row)
                .into_pyobject(py)?
                .to_owned()
                .into_any()
                .unbind())
        }
        DataType::Utf8 => {
            let arr = array.as_string::<i32>();
            Ok(arr
                .value(row)
                .into_pyobject(py)?
                .to_owned()
                .into_any()
                .unbind())
        }
        DataType::LargeUtf8 => {
            let arr = array.as_string::<i64>();
            Ok(arr
                .value(row)
                .into_pyobject(py)?
                .to_owned()
                .into_any()
                .unbind())
        }
        _ => {
            // Fallback: convert to string representation
            tracing::warn!(
                "Unsupported Arrow type {:?}, converting to string",
                array.data_type()
            );
            Ok(format!("{:?}", array.data_type())
                .into_pyobject(py)?
                .to_owned()
                .into_any()
                .unbind())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_lambda() {
        let runtime = PythonRuntime::new().unwrap();
        runtime
            .register_inline("double", "lambda x: x * 2")
            .await
            .unwrap();

        assert!(runtime.has_function("double").await);
    }

    #[tokio::test]
    async fn test_call_lambda() {
        let runtime = PythonRuntime::new().unwrap();
        runtime
            .register_inline("double", "lambda x: x * 2")
            .await
            .unwrap();

        let result = runtime.call("double", vec![Value::Int(21)]).await.unwrap();
        match result {
            Value::Int(n) => assert_eq!(n, 42),
            _ => panic!("Expected Int(42), got {:?}", result),
        }
    }

    #[tokio::test]
    async fn test_call_with_array() {
        let runtime = PythonRuntime::new().unwrap();
        runtime
            .register_inline("sum_list", "lambda arr: sum(arr)")
            .await
            .unwrap();

        let arr = Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
        let result = runtime.call("sum_list", vec![arr]).await.unwrap();
        match result {
            Value::Int(n) => assert_eq!(n, 6),
            _ => panic!("Expected Int(6), got {:?}", result),
        }
    }

    #[tokio::test]
    async fn test_call_returns_dict() {
        let runtime = PythonRuntime::new().unwrap();
        runtime
            .register_inline("make_dict", "lambda x: {'value': x, 'doubled': x * 2}")
            .await
            .unwrap();

        let result = runtime
            .call("make_dict", vec![Value::Int(5)])
            .await
            .unwrap();

        if let Value::Object(map) = result {
            match map.get("value") {
                Some(Value::Int(5)) => {}
                other => panic!("Expected value=5, got {:?}", other),
            }
            match map.get("doubled") {
                Some(Value::Int(10)) => {}
                other => panic!("Expected doubled=10, got {:?}", other),
            }
        } else {
            panic!("Expected Object, got {:?}", result);
        }
    }

    #[tokio::test]
    async fn test_error_handling() {
        let runtime = PythonRuntime::new().unwrap();
        runtime
            .register_inline("bad_func", "lambda x: 1 / 0")
            .await
            .unwrap();

        let result = runtime.call("bad_func", vec![Value::Int(1)]).await;
        assert!(result.is_err());
    }
}
