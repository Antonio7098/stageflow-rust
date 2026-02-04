//! Python bindings for the stageflow Rust library.
//!
//! This module provides PyO3 bindings to expose the Rust stageflow
//! implementation to Python, enabling drop-in replacement of the
//! Python stageflow module.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::collections::HashMap;

/// Python wrapper for StageOutput.
#[pyclass(name = "StageOutput")]
#[derive(Clone)]
pub struct PyStageOutput {
    status: String,
    data: Option<HashMap<String, serde_json::Value>>,
    error: Option<String>,
    retryable: bool,
    metadata: HashMap<String, serde_json::Value>,
}

#[pymethods]
impl PyStageOutput {
    /// Creates a successful output with no data.
    #[staticmethod]
    fn ok_empty() -> Self {
        Self {
            status: "ok".to_string(),
            data: None,
            error: None,
            retryable: false,
            metadata: HashMap::new(),
        }
    }

    /// Creates a successful output with data.
    #[staticmethod]
    fn ok(data: &Bound<'_, PyDict>) -> PyResult<Self> {
        let data_map = dict_to_hashmap(data)?;
        Ok(Self {
            status: "ok".to_string(),
            data: Some(data_map),
            error: None,
            retryable: false,
            metadata: HashMap::new(),
        })
    }

    /// Creates a failure output.
    #[staticmethod]
    fn fail(error: String) -> Self {
        Self {
            status: "fail".to_string(),
            data: None,
            error: Some(error),
            retryable: false,
            metadata: HashMap::new(),
        }
    }

    /// Creates a retryable failure output.
    #[staticmethod]
    fn fail_retryable(error: String) -> Self {
        Self {
            status: "fail".to_string(),
            data: None,
            error: Some(error),
            retryable: true,
            metadata: HashMap::new(),
        }
    }

    /// Creates a skip output.
    #[staticmethod]
    fn skip(reason: String) -> Self {
        Self {
            status: "skip".to_string(),
            data: None,
            error: None,
            retryable: false,
            metadata: HashMap::new(),
        }
    }

    /// Creates a cancel output.
    #[staticmethod]
    fn cancel(reason: String) -> Self {
        Self {
            status: "cancel".to_string(),
            data: None,
            error: None,
            retryable: false,
            metadata: HashMap::new(),
        }
    }

    /// Returns the status.
    #[getter]
    fn status(&self) -> &str {
        &self.status
    }

    /// Returns true if successful.
    fn is_success(&self) -> bool {
        self.status == "ok"
    }

    /// Returns true if failed.
    fn is_failure(&self) -> bool {
        self.status == "fail"
    }

    /// Returns true if retryable.
    fn is_retryable(&self) -> bool {
        self.retryable
    }

    /// Gets a value from data.
    fn get(&self, key: &str) -> Option<PyObject> {
        Python::with_gil(|py| {
            self.data.as_ref().and_then(|d| {
                d.get(key).map(|v| json_to_py(py, v))
            })
        })
    }

    /// Converts to a dictionary.
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new_bound(py);
        dict.set_item("status", &self.status)?;
        
        if let Some(ref data) = self.data {
            let data_dict = PyDict::new_bound(py);
            for (k, v) in data {
                data_dict.set_item(k, json_to_py(py, v))?;
            }
            dict.set_item("data", data_dict)?;
        }
        
        if let Some(ref error) = self.error {
            dict.set_item("error", error)?;
        }
        
        dict.set_item("retryable", self.retryable)?;
        
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!("StageOutput(status='{}')", self.status)
    }
}

/// Python wrapper for StageStatus.
#[pyclass(name = "StageStatus")]
#[derive(Clone)]
pub struct PyStageStatus {
    value: String,
}

#[pymethods]
impl PyStageStatus {
    #[staticmethod]
    fn ok() -> Self {
        Self { value: "ok".to_string() }
    }

    #[staticmethod]
    fn fail() -> Self {
        Self { value: "fail".to_string() }
    }

    #[staticmethod]
    fn skip() -> Self {
        Self { value: "skip".to_string() }
    }

    #[staticmethod]
    fn cancel() -> Self {
        Self { value: "cancel".to_string() }
    }

    #[staticmethod]
    fn retry() -> Self {
        Self { value: "retry".to_string() }
    }

    fn __repr__(&self) -> String {
        format!("StageStatus.{}", self.value)
    }

    fn __str__(&self) -> &str {
        &self.value
    }
}

/// Python wrapper for RunIdentity.
#[pyclass(name = "RunIdentity")]
#[derive(Clone)]
pub struct PyRunIdentity {
    pipeline_run_id: Option<String>,
    request_id: Option<String>,
    session_id: Option<String>,
    user_id: Option<String>,
    org_id: Option<String>,
}

#[pymethods]
impl PyRunIdentity {
    #[new]
    fn new() -> Self {
        Self {
            pipeline_run_id: Some(uuid::Uuid::new_v4().to_string()),
            request_id: None,
            session_id: None,
            user_id: None,
            org_id: None,
        }
    }

    #[getter]
    fn pipeline_run_id(&self) -> Option<&str> {
        self.pipeline_run_id.as_deref()
    }

    #[getter]
    fn request_id(&self) -> Option<&str> {
        self.request_id.as_deref()
    }

    #[getter]
    fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    #[getter]
    fn user_id(&self) -> Option<&str> {
        self.user_id.as_deref()
    }

    #[getter]
    fn org_id(&self) -> Option<&str> {
        self.org_id.as_deref()
    }

    fn with_request_id(&self, request_id: String) -> Self {
        let mut new = self.clone();
        new.request_id = Some(request_id);
        new
    }

    fn with_session_id(&self, session_id: String) -> Self {
        let mut new = self.clone();
        new.session_id = Some(session_id);
        new
    }

    fn with_user_id(&self, user_id: String) -> Self {
        let mut new = self.clone();
        new.user_id = Some(user_id);
        new
    }

    fn with_org_id(&self, org_id: String) -> Self {
        let mut new = self.clone();
        new.org_id = Some(org_id);
        new
    }

    fn __repr__(&self) -> String {
        format!(
            "RunIdentity(pipeline_run_id='{}')",
            self.pipeline_run_id.as_deref().unwrap_or("None")
        )
    }
}

/// Configuration for retry behavior.
#[pyclass(name = "RetryConfig")]
#[derive(Clone)]
pub struct PyRetryConfig {
    max_attempts: usize,
    base_delay_ms: u64,
    max_delay_ms: u64,
    backoff_strategy: String,
    jitter_strategy: String,
}

#[pymethods]
impl PyRetryConfig {
    #[new]
    #[pyo3(signature = (max_attempts=3, base_delay_ms=1000, max_delay_ms=30000))]
    fn new(max_attempts: usize, base_delay_ms: u64, max_delay_ms: u64) -> Self {
        Self {
            max_attempts,
            base_delay_ms,
            max_delay_ms,
            backoff_strategy: "exponential".to_string(),
            jitter_strategy: "full".to_string(),
        }
    }

    #[getter]
    fn max_attempts(&self) -> usize {
        self.max_attempts
    }

    #[getter]
    fn base_delay_ms(&self) -> u64 {
        self.base_delay_ms
    }

    #[getter]
    fn max_delay_ms(&self) -> u64 {
        self.max_delay_ms
    }

    fn with_backoff(&self, strategy: String) -> Self {
        let mut new = self.clone();
        new.backoff_strategy = strategy;
        new
    }

    fn with_jitter(&self, strategy: String) -> Self {
        let mut new = self.clone();
        new.jitter_strategy = strategy;
        new
    }
}

/// Failure mode for pipeline execution.
#[pyclass(name = "FailureMode")]
#[derive(Clone)]
pub struct PyFailureMode {
    value: String,
}

#[pymethods]
impl PyFailureMode {
    #[staticmethod]
    fn fail_fast() -> Self {
        Self { value: "fail_fast".to_string() }
    }

    #[staticmethod]
    fn continue_on_failure() -> Self {
        Self { value: "continue_on_failure".to_string() }
    }

    #[staticmethod]
    fn best_effort() -> Self {
        Self { value: "best_effort".to_string() }
    }

    fn __repr__(&self) -> String {
        format!("FailureMode.{}", self.value)
    }
}

// Helper functions

fn dict_to_hashmap(dict: &Bound<'_, PyDict>) -> PyResult<HashMap<String, serde_json::Value>> {
    let mut map = HashMap::new();
    for (key, value) in dict.iter() {
        let key_str: String = key.extract()?;
        let json_value = py_to_json(&value)?;
        map.insert(key_str, json_value);
    }
    Ok(map)
}

fn py_to_json(obj: &Bound<'_, PyAny>) -> PyResult<serde_json::Value> {
    if obj.is_none() {
        return Ok(serde_json::Value::Null);
    }
    
    if let Ok(b) = obj.extract::<bool>() {
        return Ok(serde_json::Value::Bool(b));
    }
    
    if let Ok(i) = obj.extract::<i64>() {
        return Ok(serde_json::Value::Number(i.into()));
    }
    
    if let Ok(f) = obj.extract::<f64>() {
        if let Some(n) = serde_json::Number::from_f64(f) {
            return Ok(serde_json::Value::Number(n));
        }
    }
    
    if let Ok(s) = obj.extract::<String>() {
        return Ok(serde_json::Value::String(s));
    }
    
    if let Ok(list) = obj.downcast::<PyList>() {
        let mut arr = Vec::new();
        for item in list.iter() {
            arr.push(py_to_json(&item)?);
        }
        return Ok(serde_json::Value::Array(arr));
    }
    
    if let Ok(dict) = obj.downcast::<PyDict>() {
        let mut map = serde_json::Map::new();
        for (key, value) in dict.iter() {
            let key_str: String = key.extract()?;
            map.insert(key_str, py_to_json(&value)?);
        }
        return Ok(serde_json::Value::Object(map));
    }
    
    // Fallback: convert to string representation
    Ok(serde_json::Value::String(obj.str()?.to_string()))
}

fn json_to_py(py: Python<'_>, value: &serde_json::Value) -> PyObject {
    match value {
        serde_json::Value::Null => py.None(),
        serde_json::Value::Bool(b) => b.into_py(py),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.into_py(py)
            } else if let Some(f) = n.as_f64() {
                f.into_py(py)
            } else {
                py.None()
            }
        }
        serde_json::Value::String(s) => s.into_py(py),
        serde_json::Value::Array(arr) => {
            let list = PyList::new_bound(py, arr.iter().map(|v| json_to_py(py, v)));
            list.into_py(py)
        }
        serde_json::Value::Object(map) => {
            let dict = PyDict::new_bound(py);
            for (k, v) in map {
                dict.set_item(k, json_to_py(py, v)).unwrap();
            }
            dict.into_py(py)
        }
    }
}

/// The stageflow Python module.
#[pymodule]
fn stageflow_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyStageOutput>()?;
    m.add_class::<PyStageStatus>()?;
    m.add_class::<PyRunIdentity>()?;
    m.add_class::<PyRetryConfig>()?;
    m.add_class::<PyFailureMode>()?;
    
    // Add version info
    m.add("__version__", "0.1.0")?;
    m.add("__rust_version__", env!("CARGO_PKG_VERSION"))?;
    
    Ok(())
}
