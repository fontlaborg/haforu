// this_file: src/python/mod.rs

//! Python bindings for haforu using PyO3.
//!
//! This module provides Python access to haforu's batch rendering capabilities
//! via zero-overhead native bindings.

use pyo3::prelude::*;

pub mod batch;
pub mod errors;
pub mod image_ops;  // NEW: Image processing bindings (v2.2)
pub mod streaming;
pub mod types;

#[pyfunction]
fn is_available() -> bool {
    streaming::StreamingSession::new(None, Some(1), 4).is_ok()
}

/// Python module definition.
///
/// This creates the `haforu._haforu` extension module that Python code can import.
#[pymodule]
fn _haforu(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Add version information
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("__doc__", "Haforu: High-performance batch font renderer")?;

    // Add batch mode function
    m.add_function(wrap_pyfunction!(batch::process_jobs, m)?)?;
    m.add_function(wrap_pyfunction!(is_available, m)?)?;

    // Add streaming session class
    m.add_class::<streaming::StreamingSession>()?;

    // Add image processing functions (v2.2)
    m.add_function(wrap_pyfunction!(image_ops::align_and_compare, m)?)?;
    m.add_class::<image_ops::AlignCompareResult>()?;

    // Add image scaling function (v2.3)
    m.add_function(wrap_pyfunction!(image_ops::resize_bilinear, m)?)?;
    m.add_class::<image_ops::ResizeResult>()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_creation() {
        // Basic smoke test that the module compiles
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let module = PyModule::new_bound(py, "_haforu").unwrap();
            module.add("test", "value").unwrap();
            assert!(module.getattr("test").is_ok());
        });
    }
}
