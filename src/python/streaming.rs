// this_file: src/python/streaming.rs

//! Streaming session API for persistent rendering.
//!
//! This module provides the `StreamingSession` class for Python, which maintains
//! a persistent font cache and allows zero-overhead rendering across multiple calls.

use numpy::PyArray2;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyType};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::batch::{Job, JobResult};
use crate::cache::GlyphCache;
use crate::fonts::FontLoader;
use crate::{
    process_job_with_options, ExecutionOptions, GlyphRasterizer, ShapeRequest, TextShaper,
};
use camino::Utf8PathBuf;

/// Persistent rendering session with font cache.
///
/// Maintains loaded fonts across multiple renders for maximum performance.
/// Thread-safe: can be called from multiple threads concurrently.
///
/// # Example
///
/// ```python
/// import haforu
/// import json
///
/// with haforu.StreamingSession() as session:
///     job = {
///         "id": "test1",
///         "font": {"path": "/path/to/font.ttf", "size": 1000, "variations": {}},
///         "text": {"content": "a"},
///         "rendering": {"format": "pgm", "encoding": "base64", "width": 3000, "height": 1200}
///     }
///     result_json = session.render(json.dumps(job))
///     result = json.loads(result_json)
///     print(f"Status: {result['status']}")
/// ```
#[pyclass]
pub struct StreamingSession {
    font_loader: Arc<Mutex<FontLoader>>,
    glyph_cache: Option<GlyphCache>,
    closed: Arc<AtomicBool>,
}

#[pymethods]
impl StreamingSession {
    #[new]
    #[pyo3(signature = (cache_size=None, *, max_fonts=None, max_glyphs=2048))]
    pub fn new(
        cache_size: Option<usize>,
        max_fonts: Option<usize>,
        max_glyphs: usize,
    ) -> PyResult<Self> {
        let font_capacity = max_fonts.or(cache_size).unwrap_or(512);
        Self::build(font_capacity, max_glyphs)
    }

    #[classmethod]
    fn is_available(_cls: &Bound<'_, PyType>) -> bool {
        StreamingSession::build(1, 4).is_ok()
    }

    fn ensure_open(&self) -> PyResult<()> {
        if self.closed.load(Ordering::SeqCst) {
            Err(PyRuntimeError::new_err("StreamingSession is closed"))
        } else {
            Ok(())
        }
    }

    /// Warm up the streaming session (optionally rendering a font).
    ///
    /// Args:
    ///     font_path: Optional font path to pre-load via a quick render.
    ///     text: Optional short string to render during warm-up.
    ///     size: Font size in points (default 600).
    ///     width: Canvas width (default 128).
    ///     height: Canvas height (default 128).
    ///
    /// Returns:
    ///     bool: True when warm-up completed.
    #[pyo3(signature = (font_path=None, *, text="Haforu", size=600.0, width=128, height=128))]
    fn warm_up<'py>(
        &self,
        py: Python<'py>,
        font_path: Option<&str>,
        text: &str,
        size: f64,
        width: u32,
        height: u32,
    ) -> PyResult<bool> {
        self.ensure_open()?;
        if let Some(path) = font_path {
            // Render via numpy path; ignore pixels but surface errors.
            let _ =
                self.render_to_numpy(py, path, text, size, width, height, None, None, None, None)?;
        } else {
            // Touch the cache to ensure structures are allocated.
            let _ = self.font_loader.lock().unwrap();
        }
        Ok(true)
    }

    /// Cheap liveness probe so callers can avoid exception handling.
    fn ping(&self) -> PyResult<bool> {
        self.ensure_open()?;
        Ok(true)
    }

    /// Return cache statistics for observability.
    fn cache_stats(&self) -> PyResult<HashMap<&'static str, usize>> {
        let loader = self.font_loader.lock().unwrap();
        let stats = loader.stats();
        let glyph_stats = self
            .glyph_cache
            .as_ref()
            .map(|cache| cache.stats())
            .unwrap_or_default();
        Ok(HashMap::from([
            ("capacity", stats.capacity),
            ("entries", stats.entries),
            ("font_capacity", stats.capacity),
            ("font_entries", stats.entries),
            ("glyph_capacity", glyph_stats.capacity),
            ("glyph_entries", glyph_stats.entries),
            ("glyph_hits", glyph_stats.hits as usize),
        ]))
    }

    /// Resize the cache capacity (drops stored entries).
    fn set_cache_size(&self, cache_size: usize) -> PyResult<()> {
        if cache_size == 0 {
            return Err(PyValueError::new_err("cache_size must be >= 1"));
        }
        self.ensure_open()?;
        let loader = self.font_loader.lock().unwrap();
        loader.set_capacity(cache_size);
        Ok(())
    }

    /// Resize glyph-result cache (drops cached renders).
    fn set_glyph_cache_size(&mut self, max_glyphs: usize) -> PyResult<()> {
        self.ensure_open()?;
        if max_glyphs == 0 {
            self.glyph_cache = None;
            return Ok(());
        }
        match self.glyph_cache.as_ref() {
            Some(cache) => cache.set_capacity(max_glyphs),
            None => {
                self.glyph_cache = GlyphCache::new(max_glyphs);
            }
        }
        Ok(())
    }

    /// Render a single job and return JSONL result.
    ///
    /// # Arguments
    ///
    /// * `job_json` - JSON string containing single Job specification
    ///
    /// # Returns
    ///
    /// JSONL result string with base64-encoded image
    ///
    /// # Raises
    ///
    /// * `ValueError` - Invalid JSON or job specification
    /// * `RuntimeError` - Font loading or rendering errors
    ///
    /// # Example
    ///
    /// ```python
    /// session = haforu.StreamingSession()
    /// job_json = json.dumps({
    ///     "id": "test1",
    ///     "font": {"path": "/path/to/font.ttf", "size": 1000, "variations": {}},
    ///     "text": {"content": "a"},
    ///     "rendering": {"format": "pgm", "encoding": "base64", "width": 3000, "height": 1200}
    /// })
    /// result_json = session.render(job_json)
    /// ```
    fn render(&self, job_json: &str) -> PyResult<String> {
        self.ensure_open()?;
        let job = match parse_stream_job(job_json) {
            Ok(job) => job,
            Err(err_result) => return serialize_job_result(err_result),
        };

        let result = {
            let loader = self.font_loader.lock().unwrap();
            let mut opts = ExecutionOptions::default();
            opts.glyph_cache = self.glyph_cache.clone();
            process_job_with_options(&job, &loader, &opts)
        };

        serialize_job_result(result)
    }

    /// Render text directly to numpy array (zero-copy).
    ///
    /// # Arguments
    ///
    /// * `font_path` - Absolute path to font file
    /// * `text` - Text to render (typically single glyph)
    /// * `size` - Font size in points (typically 1000)
    /// * `width` - Canvas width in pixels
    /// * `height` - Canvas height in pixels
    /// * `variations` - Optional variable font coordinates (e.g. {"wght": 600})
    /// * `script` - Script tag (default: "Latn")
    /// * `direction` - Text direction (default: "ltr")
    /// * `language` - Language tag (default: "en")
    ///
    /// # Returns
    ///
    /// 2D numpy array of shape (height, width), dtype uint8
    /// Grayscale values 0-255
    ///
    /// # Raises
    ///
    /// * `ValueError` - Invalid parameters
    /// * `RuntimeError` - Font loading or rendering errors
    ///
    /// # Example
    ///
    /// ```python
    /// session = haforu.StreamingSession()
    /// image = session.render_to_numpy(
    ///     font_path="/path/to/font.ttf",
    ///     text="a",
    ///     size=1000.0,
    ///     width=3000,
    ///     height=1200,
    ///     variations={"wght": 600.0}
    /// )
    /// assert image.shape == (1200, 3000)
    /// assert image.dtype == numpy.uint8
    /// ```
    #[pyo3(signature = (font_path, text, size, width, height, variations=None, script=None, direction=None, language=None))]
    fn render_to_numpy<'py>(
        &self,
        py: Python<'py>,
        font_path: &str,
        text: &str,
        size: f64,
        width: u32,
        height: u32,
        variations: Option<HashMap<String, f64>>,
        script: Option<&str>,
        direction: Option<&str>,
        language: Option<&str>,
    ) -> PyResult<Bound<'py, PyArray2<u8>>> {
        self.ensure_open()?;
        // Convert font path to Utf8PathBuf
        let font_path_buf = Utf8PathBuf::from(font_path);

        // Convert variation coordinates from f64 to f32
        let variations_f32: HashMap<String, f32> = variations
            .unwrap_or_default()
            .into_iter()
            .map(|(k, v)| (k, v as f32))
            .collect();

        // Load font with variations
        let font_instance = {
            let loader = self.font_loader.lock().unwrap();
            loader
                .load_font(&font_path_buf, &variations_f32)
                .map_err(|e| PyRuntimeError::new_err(format!("Font loading failed: {}", e)))?
        };

        // Shape text
        let shaper = TextShaper::new();
        let tmp_features: [String; 0] = [];
        let request = ShapeRequest {
            text,
            script,
            direction,
            language,
            features: &tmp_features,
        };
        let shaped = shaper
            .shape_with_request(&font_instance, &request, size as f32, font_path_buf.as_std_path())
            .map_err(|e| PyRuntimeError::new_err(format!("Text shaping failed: {}", e)))?;

        // Rasterize
        let rasterizer = GlyphRasterizer::new();
        let image = rasterizer
            .render_text(
                &font_instance,
                &shaped,
                width,
                height,
                0.0, // No tracking
                font_path_buf.as_std_path(),
            )
            .map_err(|e| PyRuntimeError::new_err(format!("Rendering failed: {}", e)))?;

        // Convert to 2D array: pixels is Vec<u8> of length width*height
        // numpy expects shape (height, width) in row-major order
        let array_2d: Vec<Vec<u8>> = image
            .pixels()
            .chunks(image.width() as usize)
            .map(|row| row.to_vec())
            .collect();

        // Convert to numpy array using from_vec2_bound (returns Bound)
        PyArray2::from_vec2_bound(py, &array_2d)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to create numpy array: {}", e)))
    }

    /// Close session and release resources immediately.
    fn close(&self) {
        if self.closed.swap(true, Ordering::SeqCst) {
            return;
        }
        if let Ok(loader) = self.font_loader.lock() {
            loader.clear();
        }
        if let Some(cache) = self.glyph_cache.as_ref() {
            cache.clear();
        }
    }

    fn __enter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    #[pyo3(signature = (_exc_type=None, _exc_val=None, _exc_tb=None))]
    fn __exit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_val: Option<&Bound<'_, PyAny>>,
        _exc_tb: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<bool> {
        self.close();
        Ok(false) // Don't suppress exceptions
    }
}

impl Drop for StreamingSession {
    fn drop(&mut self) {
        self.close();
    }
}

impl StreamingSession {
    fn build(max_fonts: usize, max_glyphs: usize) -> PyResult<Self> {
        if max_fonts == 0 {
            return Err(PyValueError::new_err(
                "max_fonts/cache_size must be at least 1",
            ));
        }
        let glyph_cache = GlyphCache::new(max_glyphs);
        Ok(Self {
            font_loader: Arc::new(Mutex::new(FontLoader::new(max_fonts))),
            glyph_cache,
            closed: Arc::new(AtomicBool::new(false)),
        })
    }
}

fn parse_stream_job(job_json: &str) -> Result<Job, JobResult> {
    let trimmed = job_json.trim();
    if trimmed.is_empty() {
        return Err(JobResult::error("stream-empty", "Job payload is empty"));
    }

    let fallback_id = "stream-invalid".to_string();
    let parsed_value: Value = serde_json::from_str(trimmed)
        .map_err(|err| JobResult::error(fallback_id.clone(), format!("Invalid JSON: {err}")))?;

    let job_id = parsed_value
        .get("id")
        .and_then(|value| value.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| fallback_id.clone());

    let job: Job = serde_json::from_value(parsed_value)
        .map_err(|err| JobResult::error(job_id.clone(), format!("Invalid job JSON: {err}")))?;

    if let Err(err) = job.validate() {
        return Err(JobResult::error(job.id.clone(), err.to_string()));
    }

    Ok(job)
}

fn serialize_job_result(result: JobResult) -> PyResult<String> {
    serde_json::to_string(&result)
        .map_err(|e| PyValueError::new_err(format!("Failed to serialize result: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::Instant;

    #[test]
    fn test_streaming_session_creation() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let session = StreamingSession::build(512, 8).unwrap();
            assert!(Arc::strong_count(&session.font_loader) >= 1);
        });
    }

    #[test]
    fn test_invalid_json() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|_py| {
            let session = StreamingSession::build(512, 8).unwrap();
            let result = session.render("not valid json").unwrap();
            let payload: serde_json::Value = serde_json::from_str(&result).unwrap();
            assert_eq!(
                payload.get("status").and_then(|v| v.as_str()),
                Some("error")
            );
        });
    }

    #[test]
    fn cached_metrics_renders_stay_under_one_millisecond() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|_py| {
            let session = StreamingSession::build(128, 256).unwrap();
            let mut job = json!({
                "id": "perf-0",
                "font": {
                    "path": "testdata/fonts/Arial-Black.ttf",
                    "size": 256,
                    "variations": {}
                },
                "text": {"content": "H"},
                "rendering": {
                    "format": "metrics",
                    "encoding": "json",
                    "width": 64,
                    "height": 64
                }
            });

            let iterations = 1200;
            let start = Instant::now();
            for idx in 0..iterations {
                job["id"] = format!("perf-{idx}").into();
                let job_json = serde_json::to_string(&job).unwrap();
                let payload = session.render(&job_json).unwrap();
                let parsed: serde_json::Value = serde_json::from_str(&payload).unwrap();
                assert_eq!(
                    parsed.get("status").and_then(|v| v.as_str()),
                    Some("success")
                );
            }
            let avg_ms = start.elapsed().as_secs_f64() * 1000.0 / iterations as f64;
            assert!(avg_ms < 1.0, "expected <1ms average, got {avg_ms:.4}ms");
        });
    }
}
