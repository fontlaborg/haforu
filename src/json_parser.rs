// this_file: src/json_parser.rs
//! JSON job specification parser and validator

use crate::error::{Error, Result};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Font variation axis setting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariationSetting {
    /// Axis tag (e.g., "wght", "wdth", "slnt")
    pub tag: String,
    /// Axis value
    pub value: f32,
}

/// Font specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontSpec {
    /// Path to font file
    pub path: String,
    /// Optional variation settings for variable fonts
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variations: Option<Vec<VariationSetting>>,
    /// Optional named instance index
    #[serde(skip_serializing_if = "Option::is_none")]
    pub named_instance: Option<u16>,
}

/// Text shaping options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapingOptions {
    /// Text direction (ltr, rtl, ttb, btt)
    #[serde(default = "default_direction")]
    pub direction: String,
    /// BCP 47 language tag
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// ISO 15924 script tag
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
    /// Font features to enable/disable
    #[serde(default)]
    pub features: HashMap<String, bool>,
    /// Cluster level (0-3)
    #[serde(default)]
    pub cluster_level: u8,
}

/// Rendering options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderingOptions {
    /// Enable rendering
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Output format (png, svg, etc.)
    #[serde(default = "default_format")]
    pub format: String,
    /// Background color (RGBA)
    #[serde(default = "default_background")]
    pub background: [u8; 4],
    /// Foreground color (RGBA)
    #[serde(default = "default_foreground")]
    pub foreground: [u8; 4],
}

/// Storage options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageOptions {
    /// Storage backend (memory, filesystem, database)
    #[serde(default = "default_backend")]
    pub backend: String,
    /// Optional output path for filesystem backend
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_path: Option<String>,
    /// Enable compression
    #[serde(default = "default_true")]
    pub compress: bool,
    /// Compression level (1-9)
    #[serde(default = "default_compression_level")]
    pub compression_level: u8,
}

/// Individual job within a job specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    /// Unique job identifier
    pub id: String,
    /// Font specification
    pub font: FontSpec,
    /// Text to shape/render
    pub text: String,
    /// Font size in pixels
    pub size: f32,
    /// Optional shaping options
    #[serde(default)]
    pub shaping: ShapingOptions,
    /// Optional rendering options
    #[serde(default)]
    pub rendering: RenderingOptions,
}

/// Main job specification containing multiple jobs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSpec {
    /// Specification version
    #[serde(default = "default_version")]
    pub version: String,
    /// List of jobs to process
    pub jobs: Vec<Job>,
    /// Global storage options
    #[serde(default)]
    pub storage: StorageOptions,
    /// Include shaping output in results
    #[serde(default = "default_true")]
    pub include_shaping_output: bool,
}

/// Result for a single job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResult {
    /// Job identifier
    pub id: String,
    /// Original input parameters
    pub input: Job,
    /// Shaping output (if requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shaping_output: Option<ShapingOutput>,
    /// Rendering result
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rendering_result: Option<RenderingResult>,
    /// Processing status
    pub status: String,
    /// Error message (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
}

/// Shaping output similar to hb-shape
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapingOutput {
    /// Shaped glyphs
    pub glyphs: Vec<GlyphInfo>,
    /// Text direction
    pub direction: String,
    /// Script
    pub script: String,
    /// Language
    pub language: String,
}

/// Information about a shaped glyph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlyphInfo {
    /// Glyph ID
    pub glyph_id: u32,
    /// Cluster index
    pub cluster: u32,
    /// X advance
    pub x_advance: i32,
    /// Y advance
    pub y_advance: i32,
    /// X offset
    pub x_offset: i32,
    /// Y offset
    pub y_offset: i32,
}

/// Rendering result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderingResult {
    /// Storage identifier (file path or database key)
    pub identifier: String,
    /// Image width
    pub width: u32,
    /// Image height
    pub height: u32,
    /// File size in bytes
    pub size_bytes: u64,
    /// Format
    pub format: String,
}

// Default functions for serde
fn default_direction() -> String {
    "ltr".to_string()
}
fn default_format() -> String {
    "png".to_string()
}
fn default_backend() -> String {
    "filesystem".to_string()
}
fn default_version() -> String {
    "1.0.0".to_string()
}
fn default_true() -> bool {
    true
}
fn default_compression_level() -> u8 {
    3
}
fn default_background() -> [u8; 4] {
    [255, 255, 255, 255]
}
fn default_foreground() -> [u8; 4] {
    [0, 0, 0, 255]
}

impl Default for ShapingOptions {
    fn default() -> Self {
        Self {
            direction: default_direction(),
            language: None,
            script: None,
            features: HashMap::new(),
            cluster_level: 0,
        }
    }
}

impl Default for RenderingOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            format: default_format(),
            background: default_background(),
            foreground: default_foreground(),
        }
    }
}

impl Default for StorageOptions {
    fn default() -> Self {
        Self {
            backend: default_backend(),
            output_path: None,
            compress: true,
            compression_level: default_compression_level(),
        }
    }
}

/// Parse job specification from JSON string
pub fn parse_job_spec(json: &str) -> Result<JobSpec> {
    // Validate JSON size before parsing
    crate::security::validate_json_size(json)?;

    let spec: JobSpec = serde_json::from_str(json)?;
    validate_job_spec(&spec)?;
    Ok(spec)
}

/// Validate job specification
fn validate_job_spec(spec: &JobSpec) -> Result<()> {
    // Validate version
    if !spec.version.starts_with("1.") {
        warn!("Unsupported spec version: {}", spec.version);
    }

    // Validate jobs
    if spec.jobs.is_empty() {
        return Err(Error::InvalidParameter(
            "Job specification must contain at least one job".into(),
        ));
    }

    // Check maximum number of jobs
    if spec.jobs.len() > crate::security::MAX_JOBS_PER_SPEC {
        return Err(Error::InvalidParameter(format!(
            "Too many jobs: {} (max: {})",
            spec.jobs.len(),
            crate::security::MAX_JOBS_PER_SPEC
        )));
    }

    for job in &spec.jobs {
        validate_job(job)?;
    }

    // Validate storage options
    validate_storage_options(&spec.storage)?;

    info!("Validated job specification with {} jobs", spec.jobs.len());
    Ok(())
}

/// Validate individual job
fn validate_job(job: &Job) -> Result<()> {
    // Validate ID
    if job.id.is_empty() {
        return Err(Error::InvalidParameter("Job ID cannot be empty".into()));
    }

    // Validate font path
    if job.font.path.is_empty() {
        return Err(Error::InvalidParameter(format!(
            "Job {}: font path cannot be empty",
            job.id
        )));
    }

    // Validate text
    if job.text.is_empty() {
        return Err(Error::InvalidParameter(format!(
            "Job {}: text cannot be empty",
            job.id
        )));
    }

    // Validate text input for security
    crate::security::validate_text_input(&job.text)?;

    // Validate size
    if job.size <= 0.0 || job.size > 10000.0 {
        return Err(Error::InvalidParameter(format!(
            "Job {}: font size must be between 0 and 10000",
            job.id
        )));
    }

    // Validate direction
    if !["ltr", "rtl", "ttb", "btt"].contains(&job.shaping.direction.as_str()) {
        return Err(Error::InvalidParameter(format!(
            "Job {}: invalid text direction '{}'",
            job.id, job.shaping.direction
        )));
    }

    // Validate cluster level
    if job.shaping.cluster_level > 3 {
        return Err(Error::InvalidParameter(format!(
            "Job {}: cluster level must be 0-3",
            job.id
        )));
    }

    // Validate rendering format
    if !["png", "svg", "pdf"].contains(&job.rendering.format.as_str()) {
        return Err(Error::InvalidParameter(format!(
            "Job {}: unsupported rendering format '{}'",
            job.id, job.rendering.format
        )));
    }

    debug!("Validated job {}", job.id);
    Ok(())
}

/// Validate storage options
fn validate_storage_options(options: &StorageOptions) -> Result<()> {
    // Validate backend
    if !["memory", "filesystem", "database"].contains(&options.backend.as_str()) {
        return Err(Error::InvalidParameter(format!(
            "Invalid storage backend '{}'",
            options.backend
        )));
    }

    // Validate compression level
    if options.compression_level == 0 || options.compression_level > 9 {
        return Err(Error::InvalidParameter(
            "Compression level must be between 1 and 9".into(),
        ));
    }

    // Validate output path for filesystem backend
    if options.backend == "filesystem" && options.output_path.is_none() {
        return Err(Error::InvalidParameter(
            "Filesystem backend requires output_path".into(),
        ));
    }

    Ok(())
}

/// Serialize job result to JSON line
pub fn serialize_job_result(result: &JobResult) -> Result<String> {
    serde_json::to_string(result).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_job_spec() {
        let json = r#"{
            "version": "1.0.0",
            "jobs": [{
                "id": "job1",
                "font": {"path": "/path/to/font.ttf"},
                "text": "Hello, world!",
                "size": 24.0
            }],
            "storage": {"backend": "filesystem", "output_path": "/tmp"}
        }"#;

        let spec = parse_job_spec(json).unwrap();
        assert_eq!(spec.version, "1.0.0");
        assert_eq!(spec.jobs.len(), 1);
        assert_eq!(spec.jobs[0].id, "job1");
    }

    #[test]
    fn test_parse_job_with_variations() {
        let json = r#"{
            "version": "1.0.0",
            "jobs": [{
                "id": "job1",
                "font": {
                    "path": "/path/to/font.ttf",
                    "variations": [
                        {"tag": "wght", "value": 500.0},
                        {"tag": "wdth", "value": 75.0}
                    ]
                },
                "text": "Test",
                "size": 16.0
            }],
            "storage": {"backend": "memory"}
        }"#;

        let spec = parse_job_spec(json).unwrap();
        let variations = spec.jobs[0].font.variations.as_ref().unwrap();
        assert_eq!(variations.len(), 2);
        assert_eq!(variations[0].tag, "wght");
        assert_eq!(variations[0].value, 500.0);
    }

    #[test]
    fn test_validate_empty_jobs() {
        let json = r#"{
            "version": "1.0.0",
            "jobs": []
        }"#;

        let result = parse_job_spec(json);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::InvalidParameter(_)));
    }

    #[test]
    fn test_validate_invalid_size() {
        let json = r#"{
            "version": "1.0.0",
            "jobs": [{
                "id": "job1",
                "font": {"path": "/path/to/font.ttf"},
                "text": "Test",
                "size": -1.0
            }]
        }"#;

        let result = parse_job_spec(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_invalid_direction() {
        let json = r#"{
            "version": "1.0.0",
            "jobs": [{
                "id": "job1",
                "font": {"path": "/path/to/font.ttf"},
                "text": "Test",
                "size": 16.0,
                "shaping": {"direction": "invalid"}
            }]
        }"#;

        let result = parse_job_spec(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_serialize_job_result() {
        let result = JobResult {
            id: "job1".to_string(),
            input: Job {
                id: "job1".to_string(),
                font: FontSpec {
                    path: "/path/to/font.ttf".to_string(),
                    variations: None,
                    named_instance: None,
                },
                text: "Test".to_string(),
                size: 16.0,
                shaping: ShapingOptions::default(),
                rendering: RenderingOptions::default(),
            },
            shaping_output: None,
            rendering_result: None,
            status: "success".to_string(),
            error: None,
            processing_time_ms: 42,
        };

        let json = serialize_job_result(&result).unwrap();
        assert!(json.contains("\"id\":\"job1\""));
        assert!(json.contains("\"status\":\"success\""));
        assert!(json.contains("\"processing_time_ms\":42"));
    }
}
