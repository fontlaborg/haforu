// this_file: external/haforu/src/json_parser.rs
//! JSON job specification parser for FontSimi integration
//!
//! This module implements the JSON parsing for Haforu's batch rendering mode.
//! It matches the exact specification expected by FontSimi's HaforuRenderer.

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Main job specification containing multiple rendering jobs
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JobSpec {
    /// Specification version (expected: "1.0")
    pub version: String,
    /// List of jobs to process
    pub jobs: Vec<Job>,
}

/// Individual rendering job
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Job {
    /// Unique job identifier
    pub id: String,
    /// Font configuration
    pub font: FontConfig,
    /// Text configuration
    pub text: TextConfig,
    /// Rendering configuration
    pub rendering: RenderingConfig,
}

/// Font configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FontConfig {
    /// Path to font file (TTF/OTF)
    pub path: PathBuf,
    /// Font size in points (typically 1000 for FontSimi)
    pub size: u32,
    /// Variable font coordinates (e.g., {"wght": 600.0, "wdth": 100.0})
    #[serde(default)]
    pub variations: HashMap<String, f32>,
}

/// Text configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TextConfig {
    /// Text content to render (can be single character or string)
    pub content: String,
    /// Optional script hint (e.g., "Latn", "Cyrl")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
}

/// Rendering configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RenderingConfig {
    /// Output format (expected: "pgm")
    pub format: String,
    /// Encoding type (expected: "binary")
    pub encoding: String,
    /// Canvas width in pixels (typically 3000 for FontSimi)
    pub width: u32,
    /// Canvas height in pixels (typically 1200 for FontSimi)
    pub height: u32,
}

/// Result for a single job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResult {
    /// Job identifier (matches input job.id)
    pub id: String,
    /// Processing status ("success" or "error")
    pub status: String,
    /// Rendering output (if successful)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rendering: Option<RenderingOutput>,
    /// Error message (if failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Timing information
    pub timing: TimingInfo,
    /// Memory information (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<MemoryInfo>,
}

/// Rendering output data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderingOutput {
    /// Output format ("pgm")
    pub format: String,
    /// Encoding type ("base64")
    pub encoding: String,
    /// Base64-encoded PGM image data
    pub data: String,
    /// Image width in pixels
    pub width: u32,
    /// Image height in pixels
    pub height: u32,
    /// Actual bounding box of rendered content (x, y, w, h)
    pub actual_bbox: (u32, u32, u32, u32),
}

/// Timing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingInfo {
    /// Text shaping time in milliseconds
    pub shape_ms: f64,
    /// Rendering time in milliseconds
    pub render_ms: f64,
    /// Total processing time in milliseconds
    pub total_ms: f64,
}

/// Memory usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    /// Font cache size in megabytes
    pub font_cache_mb: f64,
    /// Total memory usage in megabytes
    pub total_mb: f64,
}

/// Parse job specification from JSON string
pub fn parse_job_spec(json: &str) -> Result<JobSpec> {
    let spec: JobSpec = serde_json::from_str(json).map_err(|e| {
        Error::InvalidInput(format!("JSON parse error: {}", e))
    })?;

    validate_job_spec(&spec)?;
    Ok(spec)
}

/// Parse job specification from stdin
pub fn read_job_spec_from_stdin() -> Result<JobSpec> {
    let stdin = std::io::stdin();
    let reader = stdin.lock();

    let spec: JobSpec = serde_json::from_reader(reader).map_err(|e| {
        Error::InvalidInput(format!("JSON parse error from stdin: {}", e))
    })?;

    validate_job_spec(&spec)?;
    Ok(spec)
}

/// Validate job specification
fn validate_job_spec(spec: &JobSpec) -> Result<()> {
    // Validate version
    if spec.version != "1.0" {
        return Err(Error::InvalidInput(format!(
            "Unsupported specification version: '{}' (expected '1.0')",
            spec.version
        )));
    }

    // Validate jobs array is non-empty
    if spec.jobs.is_empty() {
        return Err(Error::InvalidInput(
            "Job specification must contain at least one job".into(),
        ));
    }

    // Validate each job
    for job in &spec.jobs {
        validate_job(job)?;
    }

    Ok(())
}

/// Validate individual job
fn validate_job(job: &Job) -> Result<()> {
    // Validate job ID is non-empty
    if job.id.is_empty() {
        return Err(Error::InvalidInput("Job ID cannot be empty".into()));
    }

    // Validate font path exists
    if !job.font.path.exists() {
        return Err(Error::InvalidInput(format!(
            "Job '{}': Font file not found: {}",
            job.id,
            job.font.path.display()
        )));
    }

    // Validate font path is readable
    if !job.font.path.is_file() {
        return Err(Error::InvalidInput(format!(
            "Job '{}': Font path is not a file: {}",
            job.id,
            job.font.path.display()
        )));
    }

    // Validate font size
    if job.font.size == 0 || job.font.size >= 10000 {
        return Err(Error::InvalidInput(format!(
            "Job '{}': Font size must be between 1 and 9999 (got {})",
            job.id, job.font.size
        )));
    }

    // Validate text content is non-empty
    if job.text.content.is_empty() {
        return Err(Error::InvalidInput(format!(
            "Job '{}': Text content cannot be empty",
            job.id
        )));
    }

    // Validate text content length
    if job.text.content.len() > 10000 {
        return Err(Error::InvalidInput(format!(
            "Job '{}': Text content too long ({} chars, max 10000)",
            job.id,
            job.text.content.len()
        )));
    }

    // Validate rendering format
    if job.rendering.format != "pgm" {
        return Err(Error::InvalidInput(format!(
            "Job '{}': Unsupported rendering format '{}' (expected 'pgm')",
            job.id, job.rendering.format
        )));
    }

    // Validate rendering dimensions
    if job.rendering.width == 0 || job.rendering.width > 10000 {
        return Err(Error::InvalidInput(format!(
            "Job '{}': Rendering width must be between 1 and 10000 (got {})",
            job.id, job.rendering.width
        )));
    }

    if job.rendering.height == 0 || job.rendering.height > 10000 {
        return Err(Error::InvalidInput(format!(
            "Job '{}': Rendering height must be between 1 and 10000 (got {})",
            job.id, job.rendering.height
        )));
    }

    Ok(())
}

/// Serialize job result to JSONL (single line)
pub fn serialize_job_result(result: &JobResult) -> Result<String> {
    serde_json::to_string(result).map_err(|e| {
        Error::Internal(format!("Failed to serialize job result: {}", e))
    })
}

/// Write job result to stdout as JSONL
pub fn write_job_result_to_stdout(result: &JobResult) -> Result<()> {
    use std::io::Write;

    let json_line = serialize_job_result(result)?;
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();

    writeln!(handle, "{}", json_line).map_err(|e| {
        Error::Io(format!("Failed to write to stdout: {}", e))
    })?;

    handle.flush().map_err(|e| {
        Error::Io(format!("Failed to flush stdout: {}", e))
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    fn create_test_font() -> NamedTempFile {
        let file = NamedTempFile::new().unwrap();
        // Write minimal TTF magic bytes
        fs::write(file.path(), b"\x00\x01\x00\x00").unwrap();
        file
    }

    #[test]
    fn test_parse_valid_job_spec() {
        let font_file = create_test_font();
        let json = format!(
            r#"{{
                "version": "1.0",
                "jobs": [{{
                    "id": "test1",
                    "font": {{
                        "path": "{}",
                        "size": 1000
                    }},
                    "text": {{
                        "content": "A"
                    }},
                    "rendering": {{
                        "format": "pgm",
                        "encoding": "binary",
                        "width": 3000,
                        "height": 1200
                    }}
                }}]
            }}"#,
            font_file.path().display()
        );

        let spec = parse_job_spec(&json).unwrap();
        assert_eq!(spec.version, "1.0");
        assert_eq!(spec.jobs.len(), 1);
        assert_eq!(spec.jobs[0].id, "test1");
        assert_eq!(spec.jobs[0].font.size, 1000);
        assert_eq!(spec.jobs[0].text.content, "A");
    }

    #[test]
    fn test_parse_job_with_variations() {
        let font_file = create_test_font();
        let json = format!(
            r#"{{
                "version": "1.0",
                "jobs": [{{
                    "id": "test1",
                    "font": {{
                        "path": "{}",
                        "size": 1000,
                        "variations": {{
                            "wght": 600.0,
                            "wdth": 100.0
                        }}
                    }},
                    "text": {{
                        "content": "A"
                    }},
                    "rendering": {{
                        "format": "pgm",
                        "encoding": "binary",
                        "width": 3000,
                        "height": 1200
                    }}
                }}]
            }}"#,
            font_file.path().display()
        );

        let spec = parse_job_spec(&json).unwrap();
        let variations = &spec.jobs[0].font.variations;
        assert_eq!(variations.len(), 2);
        assert_eq!(variations.get("wght"), Some(&600.0));
        assert_eq!(variations.get("wdth"), Some(&100.0));
    }

    #[test]
    fn test_parse_multiple_jobs() {
        let font_file = create_test_font();
        let json = format!(
            r#"{{
                "version": "1.0",
                "jobs": [
                    {{
                        "id": "test1",
                        "font": {{"path": "{}", "size": 1000}},
                        "text": {{"content": "A"}},
                        "rendering": {{"format": "pgm", "encoding": "binary", "width": 3000, "height": 1200}}
                    }},
                    {{
                        "id": "test2",
                        "font": {{"path": "{}", "size": 500}},
                        "text": {{"content": "B"}},
                        "rendering": {{"format": "pgm", "encoding": "binary", "width": 3000, "height": 1200}}
                    }}
                ]
            }}"#,
            font_file.path().display(),
            font_file.path().display()
        );

        let spec = parse_job_spec(&json).unwrap();
        assert_eq!(spec.jobs.len(), 2);
        assert_eq!(spec.jobs[0].id, "test1");
        assert_eq!(spec.jobs[1].id, "test2");
    }

    #[test]
    fn test_validate_empty_jobs() {
        let json = r#"{
            "version": "1.0",
            "jobs": []
        }"#;

        let result = parse_job_spec(json);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::InvalidInput(_)));
    }

    #[test]
    fn test_validate_invalid_version() {
        let font_file = create_test_font();
        let json = format!(
            r#"{{
                "version": "2.0",
                "jobs": [{{
                    "id": "test1",
                    "font": {{"path": "{}", "size": 1000}},
                    "text": {{"content": "A"}},
                    "rendering": {{"format": "pgm", "encoding": "binary", "width": 3000, "height": 1200}}
                }}]
            }}"#,
            font_file.path().display()
        );

        let result = parse_job_spec(&json);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_missing_font_file() {
        let json = r#"{
            "version": "1.0",
            "jobs": [{
                "id": "test1",
                "font": {"path": "/nonexistent/font.ttf", "size": 1000},
                "text": {"content": "A"},
                "rendering": {"format": "pgm", "encoding": "binary", "width": 3000, "height": 1200}
            }]
        }"#;

        let result = parse_job_spec(json);
        assert!(result.is_err());
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(err_msg.contains("not found"));
    }

    #[test]
    fn test_validate_invalid_font_size() {
        let font_file = create_test_font();
        let json = format!(
            r#"{{
                "version": "1.0",
                "jobs": [{{
                    "id": "test1",
                    "font": {{"path": "{}", "size": 0}},
                    "text": {{"content": "A"}},
                    "rendering": {{"format": "pgm", "encoding": "binary", "width": 3000, "height": 1200}}
                }}]
            }}"#,
            font_file.path().display()
        );

        let result = parse_job_spec(&json);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_empty_text() {
        let font_file = create_test_font();
        let json = format!(
            r#"{{
                "version": "1.0",
                "jobs": [{{
                    "id": "test1",
                    "font": {{"path": "{}", "size": 1000}},
                    "text": {{"content": ""}},
                    "rendering": {{"format": "pgm", "encoding": "binary", "width": 3000, "height": 1200}}
                }}]
            }}"#,
            font_file.path().display()
        );

        let result = parse_job_spec(&json);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_invalid_format() {
        let font_file = create_test_font();
        let json = format!(
            r#"{{
                "version": "1.0",
                "jobs": [{{
                    "id": "test1",
                    "font": {{"path": "{}", "size": 1000}},
                    "text": {{"content": "A"}},
                    "rendering": {{"format": "png", "encoding": "binary", "width": 3000, "height": 1200}}
                }}]
            }}"#,
            font_file.path().display()
        );

        let result = parse_job_spec(&json);
        assert!(result.is_err());
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(err_msg.contains("pgm"));
    }

    #[test]
    fn test_serialize_job_result() {
        let result = JobResult {
            id: "test1".to_string(),
            status: "success".to_string(),
            rendering: Some(RenderingOutput {
                format: "pgm".to_string(),
                encoding: "base64".to_string(),
                data: "UDUKMTAgMTAKMjU1CgAAAAAAAAAAAAAAAAAAAAAAAA==".to_string(),
                width: 10,
                height: 10,
                actual_bbox: (0, 0, 10, 10),
            }),
            error: None,
            timing: TimingInfo {
                shape_ms: 1.5,
                render_ms: 2.3,
                total_ms: 3.8,
            },
            memory: None,
        };

        let json = serialize_job_result(&result).unwrap();
        assert!(json.contains("\"id\":\"test1\""));
        assert!(json.contains("\"status\":\"success\""));
        assert!(json.contains("\"shape_ms\":1.5"));
    }

    #[test]
    fn test_very_long_text() {
        let font_file = create_test_font();
        let long_text = "A".repeat(9999);
        let json = format!(
            r#"{{
                "version": "1.0",
                "jobs": [{{
                    "id": "test1",
                    "font": {{"path": "{}", "size": 1000}},
                    "text": {{"content": "{}"}},
                    "rendering": {{"format": "pgm", "encoding": "binary", "width": 3000, "height": 1200}}
                }}]
            }}"#,
            font_file.path().display(),
            long_text
        );

        let spec = parse_job_spec(&json).unwrap();
        assert_eq!(spec.jobs[0].text.content.len(), 9999);
    }

    #[test]
    fn test_text_too_long() {
        let font_file = create_test_font();
        let too_long_text = "A".repeat(10001);
        let json = format!(
            r#"{{
                "version": "1.0",
                "jobs": [{{
                    "id": "test1",
                    "font": {{"path": "{}", "size": 1000}},
                    "text": {{"content": "{}"}},
                    "rendering": {{"format": "pgm", "encoding": "binary", "width": 3000, "height": 1200}}
                }}]
            }}"#,
            font_file.path().display(),
            too_long_text
        );

        let result = parse_job_spec(&json);
        assert!(result.is_err());
    }

    #[test]
    fn test_many_variation_axes() {
        let font_file = create_test_font();
        let mut variations = HashMap::new();
        for i in 0..20 {
            variations.insert(format!("ax{:02}", i), i as f32 * 10.0);
        }

        let variations_json: String = variations
            .iter()
            .map(|(k, v)| format!("\"{}\": {}", k, v))
            .collect::<Vec<_>>()
            .join(", ");

        let json = format!(
            r#"{{
                "version": "1.0",
                "jobs": [{{
                    "id": "test1",
                    "font": {{
                        "path": "{}",
                        "size": 1000,
                        "variations": {{{}}}
                    }},
                    "text": {{"content": "A"}},
                    "rendering": {{"format": "pgm", "encoding": "binary", "width": 3000, "height": 1200}}
                }}]
            }}"#,
            font_file.path().display(),
            variations_json
        );

        let spec = parse_job_spec(&json).unwrap();
        assert_eq!(spec.jobs[0].font.variations.len(), 20);
    }
}
