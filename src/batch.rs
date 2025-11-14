// this_file: src/batch.rs

//! Batch job processing and JSONL I/O.
//!
//! This module defines the job specification format that FontSimi sends,
//! and the result format that haforu returns via JSONL.

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complete batch job specification (top-level JSON input).
#[derive(Debug, Clone, Deserialize)]
pub struct JobSpec {
    /// API version (must be "1.0")
    pub version: String,
    /// List of rendering jobs to process
    pub jobs: Vec<Job>,
}

/// Single rendering job.
#[derive(Debug, Clone, Deserialize)]
pub struct Job {
    /// Unique job identifier for correlation with results
    pub id: String,
    /// Font configuration
    pub font: FontConfig,
    /// Text to render
    pub text: TextConfig,
    /// Rendering parameters
    pub rendering: RenderingConfig,
}

/// Font configuration for a job.
#[derive(Debug, Clone, Deserialize)]
pub struct FontConfig {
    /// Absolute path to font file
    pub path: Utf8PathBuf,
    /// Font size in points (typically 1000 for FontSimi)
    pub size: u32,
    /// Variable font coordinates (axis tag → value)
    #[serde(default)]
    pub variations: HashMap<String, f32>,
}

/// Text configuration for a job.
#[derive(Debug, Clone, Deserialize)]
pub struct TextConfig {
    /// Text content to render (single glyph or string)
    pub content: String,
    /// Optional script hint (e.g., "Latn", "Cyrl")
    #[serde(default)]
    pub script: Option<String>,
    /// Requested text direction (ltr, rtl, ttb, btt)
    #[serde(default)]
    pub direction: Option<String>,
    /// Requested language hint (e.g., "en", "ar")
    #[serde(default)]
    pub language: Option<String>,
    /// OpenType feature toggles (e.g., ["liga=0", "kern"])
    #[serde(default)]
    pub features: Vec<String>,
}

/// Rendering parameters for a job.
#[derive(Debug, Clone, Deserialize)]
pub struct RenderingConfig {
    /// Output format ("pgm" or "png")
    pub format: String,
    /// Encoding ("binary" for PGM, "base64" for JSONL)
    pub encoding: String,
    /// Canvas width in pixels
    pub width: u32,
    /// Canvas height in pixels
    pub height: u32,
}

/// Job result (JSONL output line).
#[derive(Debug, Clone, Serialize)]
pub struct JobResult {
    /// Job ID (matches input)
    pub id: String,
    /// Status: "success" or "error"
    pub status: String,
    /// Rendering output (only present on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rendering: Option<RenderingOutput>,
    /// Metrics output (present when format == "metrics")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<MetricsOutput>,
    /// Error message (only present on error)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Sanitized font metadata (path + applied variations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font: Option<FontResult>,
    /// Timing information
    pub timing: TimingInfo,
    /// Memory statistics (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<MemoryInfo>,
}

/// Rendering output data.
#[derive(Debug, Clone, Serialize)]
pub struct RenderingOutput {
    /// Output format ("pgm" or "png")
    pub format: String,
    /// Encoding ("base64")
    pub encoding: String,
    /// Base64-encoded image data
    pub data: String,
    /// Image width in pixels
    pub width: u32,
    /// Image height in pixels
    pub height: u32,
    /// Actual bounding box of rendered content (x, y, w, h)
    pub actual_bbox: (u32, u32, u32, u32),
}

/// Metrics output data for metrics-only jobs.
#[derive(Debug, Clone, Serialize)]
pub struct MetricsOutput {
    /// Normalized pixel density [0.0, 1.0]
    pub density: f64,
    /// Longest contiguous non-zero run relative to canvas size [0.0, 1.0]
    pub beam: f64,
}

/// Timing statistics for a job.
#[derive(Debug, Clone, Serialize)]
pub struct TimingInfo {
    /// Time spent shaping text (milliseconds)
    pub shape_ms: f64,
    /// Time spent rasterizing glyphs (milliseconds)
    pub render_ms: f64,
    /// Total time for job (milliseconds)
    pub total_ms: f64,
}

/// Memory usage statistics (optional).
#[derive(Debug, Clone, Serialize)]
pub struct MemoryInfo {
    /// Font cache size (megabytes)
    pub font_cache_mb: f64,
    /// Total memory usage (megabytes)
    pub total_mb: f64,
}

/// Font metadata emitted with each job result so callers can inspect sanitization.
#[derive(Debug, Clone, Serialize)]
pub struct FontResult {
    /// Absolute path used after sanitization
    pub path: String,
    /// Applied variation coordinates (after clamping/dropping)
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub variations: HashMap<String, f32>,
}

impl Default for TimingInfo {
    fn default() -> Self {
        Self {
            shape_ms: 0.0,
            render_ms: 0.0,
            total_ms: 0.0,
        }
    }
}

impl JobResult {
    /// Convenience constructor for emitting error results in streaming/CLI paths.
    pub fn error(id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            status: "error".to_string(),
            rendering: None,
            metrics: None,
            error: Some(message.into()),
            font: None,
            timing: TimingInfo::default(),
            memory: None,
        }
    }
}

impl JobSpec {
    /// Validate job specification structure and parameters.
    pub fn validate(&self) -> Result<(), crate::error::Error> {
        self.validate_header()?;

        for job in &self.jobs {
            job.validate()?;
        }

        Ok(())
    }

    /// Validate header-level constraints (version + job counts) without per-job validation.
    pub fn validate_header(&self) -> Result<(), crate::error::Error> {
        use crate::error::Error;
        use crate::security::MAX_JOBS_PER_SPEC;

        // Check version
        if self.version != "1.0" {
            return Err(Error::InvalidJobSpec {
                reason: format!("Unsupported API version '{}', expected '1.0'", self.version),
            });
        }

        // Check jobs array is non-empty
        if self.jobs.is_empty() {
            return Err(Error::InvalidJobSpec {
                reason: "Jobs array is empty".to_string(),
            });
        }

        // Check limit on number of jobs
        if self.jobs.len() > MAX_JOBS_PER_SPEC {
            return Err(Error::InvalidJobSpec {
                reason: format!(
                    "Too many jobs in spec: {} (max: {})",
                    self.jobs.len(),
                    MAX_JOBS_PER_SPEC
                ),
            });
        }

        Ok(())
    }
}

impl Job {
    /// Validate individual job parameters.
    pub fn validate(&self) -> Result<(), crate::error::Error> {
        use crate::error::Error;
        use crate::security::validate_text_input;

        // Check ID is non-empty
        if self.id.is_empty() {
            return Err(Error::InvalidJobSpec {
                reason: "Job ID is empty".to_string(),
            });
        }

        // Validate font config
        if self.font.size == 0 || self.font.size > 10000 {
            return Err(Error::InvalidRenderParams {
                reason: format!("Font size {} out of bounds (1-10000)", self.font.size),
            });
        }

        // Validate text config
        if self.text.content.is_empty() {
            return Err(Error::InvalidJobSpec {
                reason: "Text content is empty".to_string(),
            });
        }

        if self.text.content.len() > 10000 {
            return Err(Error::InvalidJobSpec {
                reason: format!(
                    "Text content too long ({} chars, max 10000)",
                    self.text.content.len()
                ),
            });
        }

        // Additional text validation to prevent control chars, etc.
        validate_text_input(&self.text.content)?;

        if let Some(direction) = self.text.direction.as_deref() {
            let dir = direction.to_lowercase();
            let valid = matches!(dir.as_str(), "ltr" | "rtl" | "ttb" | "btt");
            if !valid {
                return Err(Error::InvalidJobSpec {
                    reason: format!(
                        "Unsupported text direction '{}', expected ltr/rtl/ttb/btt",
                        direction
                    ),
                });
            }
        }

        if let Some(language) = self.text.language.as_deref() {
            if language.len() > 32 {
                return Err(Error::InvalidJobSpec {
                    reason: format!("Language tag '{}' is too long", language),
                });
            }
            if !language
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            {
                return Err(Error::InvalidJobSpec {
                    reason: format!("Language tag '{}' contains invalid characters", language),
                });
            }
        }

        if self.text.features.len() > 64 {
            return Err(Error::InvalidJobSpec {
                reason: format!(
                    "Too many OpenType features supplied ({} > 64)",
                    self.text.features.len()
                ),
            });
        }

        for feature in &self.text.features {
            if feature.trim().is_empty() {
                return Err(Error::InvalidJobSpec {
                    reason: "OpenType feature entries must be non-empty".to_string(),
                });
            }
        }

        // Validate rendering config
        if self.rendering.format != "pgm"
            && self.rendering.format != "png"
            && self.rendering.format != "metrics"
        {
            return Err(Error::InvalidRenderParams {
                reason: format!(
                    "Invalid output format '{}', expected 'pgm', 'png', or 'metrics'",
                    self.rendering.format
                ),
            });
        }

        if self.rendering.width == 0
            || self.rendering.height == 0
            || self.rendering.width > 10000
            || self.rendering.height > 10000
        {
            return Err(Error::InvalidRenderParams {
                reason: format!(
                    "Canvas dimensions {}×{} out of bounds (1-10000)",
                    self.rendering.width, self.rendering.height
                ),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn sample_job_json() -> &'static str {
        r#"{
            "version": "1.0",
            "jobs": [{
                "id": "test1",
                "font": {
                    "path": "/path/to/font.ttf",
                    "size": 1000,
                    "variations": {"wght": 600.0}
                },
                "text": {
                    "content": "A"
                },
                "rendering": {
                    "format": "pgm",
                    "encoding": "base64",
                    "width": 3000,
                    "height": 1200
                }
            }]
        }"#
    }

    #[test]
    fn test_parse_valid_job_spec() {
        let spec: JobSpec = serde_json::from_str(sample_job_json()).unwrap();
        assert_eq!(spec.version, "1.0");
        assert_eq!(spec.jobs.len(), 1);
        assert_eq!(spec.jobs[0].id, "test1");
        assert_eq!(spec.jobs[0].font.size, 1000);
        assert_eq!(spec.jobs[0].text.content, "A");
    }

    #[test]
    fn test_validate_valid_spec() {
        let spec: JobSpec = serde_json::from_str(sample_job_json()).unwrap();
        assert!(spec.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_version() {
        let json = r#"{"version": "2.0", "jobs": []}"#;
        let spec: JobSpec = serde_json::from_str(json).unwrap();
        let err = spec.validate().unwrap_err();
        assert!(err.to_string().contains("Unsupported API version"));
    }

    #[test]
    fn test_validate_empty_jobs() {
        let json = r#"{"version": "1.0", "jobs": []}"#;
        let spec: JobSpec = serde_json::from_str(json).unwrap();
        let err = spec.validate().unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn test_serialize_job_result_success() {
        let result = JobResult {
            id: "test1".to_string(),
            status: "success".to_string(),
            rendering: Some(RenderingOutput {
                format: "pgm".to_string(),
                encoding: "base64".to_string(),
                data: "AQIDBA==".to_string(),
                width: 100,
                height: 50,
                actual_bbox: (10, 20, 80, 30),
            }),
            metrics: None,
            error: None,
            font: Some(FontResult {
                path: "/fonts/test.ttf".to_string(),
                variations: HashMap::from([(String::from("wght"), 650.0)]),
            }),
            timing: TimingInfo {
                shape_ms: 1.2,
                render_ms: 3.4,
                total_ms: 5.0,
            },
            memory: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"status\":\"success\""));
        assert!(json.contains("\"rendering\""));
        assert!(!json.contains("\"error\""));
    }

    #[test]
    fn test_serialize_job_result_error() {
        let result = JobResult {
            id: "test1".to_string(),
            status: "error".to_string(),
            rendering: None,
            metrics: None,
            error: Some("Font not found".to_string()),
            font: None,
            timing: TimingInfo {
                shape_ms: 0.0,
                render_ms: 0.0,
                total_ms: 0.1,
            },
            memory: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"status\":\"error\""));
        assert!(json.contains("\"error\""));
        assert!(!json.contains("\"rendering\""));
    }

    #[test]
    fn test_serialize_job_result_metrics_only() {
        let result = JobResult {
            id: "metrics-1".to_string(),
            status: "success".to_string(),
            rendering: None,
            metrics: Some(MetricsOutput {
                density: 0.42,
                beam: 0.15,
            }),
            error: None,
            font: None,
            timing: TimingInfo::default(),
            memory: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"metrics\""), "JSON: {json}");
        assert!(
            !json.contains("\"rendering\""),
            "JSON should omit rendering"
        );
    }
}
