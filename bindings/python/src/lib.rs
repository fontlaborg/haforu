// this_file: bindings/python/src/lib.rs
//! PyO3 bindings for the `haforu` Rust crate.
//! Minimal, stable API intended for scripting and batch automation.

use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;

/// Return the package version (matches Rust crate version)
#[pyfunction]
fn version() -> PyResult<String> {
    Ok(env!("CARGO_PKG_VERSION").to_string())
}

/// Validate a JSON job specification according to haforu schema.
/// Returns `True` on success; raises ValueError with message on error.
#[pyfunction]
fn validate_spec(spec_json: &str) -> PyResult<bool> {
    match haforu::json_parser::parse_job_spec(spec_json) {
        Ok(_) => Ok(true),
        Err(e) => Err(PyValueError::new_err(format!("Invalid spec: {}", e))),
    }
}

/// Process a JSON job specification and return JSONL lines as a Python list of strings.
/// Note: This uses the current Rust library behavior, which may output placeholder results
/// until rendering and shaping are fully implemented.
#[pyfunction]
fn process(spec_json: &str) -> PyResult<Vec<String>> {
    let spec = haforu::json_parser::parse_job_spec(spec_json)
        .map_err(|e| PyValueError::new_err(format!("Invalid spec: {}", e)))?;

    let mut lines = Vec::with_capacity(spec.jobs.len());
    for job in &spec.jobs {
        let result = haforu::json_parser::JobResult {
            id: job.id.clone(),
            input: job.clone(),
            shaping_output: None,
            rendering_result: None,
            status: "pending".to_string(),
            error: Some("Not fully implemented yet".to_string()),
            processing_time_ms: 0,
        };
        let json_line = haforu::json_parser::serialize_job_result(&result)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        lines.push(json_line);
    }
    Ok(lines)
}

/// Python module definition
#[pymodule]
fn haforu(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(version, m)?)?;
    m.add_function(wrap_pyfunction!(validate_spec, m)?)?;
    m.add_function(wrap_pyfunction!(process, m)?)?;
    Ok(())
}

