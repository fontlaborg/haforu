// this_file: src/security.rs
//! Security and validation utilities

use crate::error::{Error, Result};
use log::{debug, warn};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Maximum allowed JSON input size (10MB)
pub const MAX_JSON_SIZE: usize = 10 * 1024 * 1024;

/// Maximum allowed number of jobs in a single spec
pub const MAX_JOBS_PER_SPEC: usize = 1000;

/// Maximum allowed text length for shaping
pub const MAX_TEXT_LENGTH: usize = 10_000;

/// Maximum allowed font file size (50MB)
pub const MAX_FONT_SIZE: usize = 50 * 1024 * 1024;

/// Default operation timeout
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Maximum memory usage allowed (500MB)
pub const MAX_MEMORY_USAGE: usize = 500 * 1024 * 1024;

/// Validate and sanitize a file path
pub fn sanitize_path<P: AsRef<Path>>(path: P, base_dir: Option<&Path>) -> Result<PathBuf> {
    let path = path.as_ref();

    // Check for directory traversal attempts
    let path_str = path.to_string_lossy();
    if path_str.contains("..") || path_str.contains("~") {
        warn!("Potential directory traversal attempt: {}", path_str);
        return Err(Error::InvalidParameter(
            "Path contains invalid characters (.. or ~)".into(),
        ));
    }

    // Resolve to absolute path
    let absolute_path = if path.is_absolute() {
        path.to_path_buf()
    } else if let Some(base) = base_dir {
        base.join(path)
    } else {
        std::env::current_dir()
            .map_err(|e| Error::Io(e.to_string()))?
            .join(path)
    };

    // Canonicalize to resolve symlinks
    let canonical = absolute_path.canonicalize().map_err(|e| {
        Error::InvalidParameter(format!(
            "Cannot resolve path {}: {}",
            absolute_path.display(),
            e
        ))
    })?;

    // If base_dir is provided, ensure the path is within it
    if let Some(base) = base_dir {
        let base_canonical = base.canonicalize().map_err(|e| {
            Error::InvalidParameter(format!(
                "Cannot resolve base path {}: {}",
                base.display(),
                e
            ))
        })?;

        if !canonical.starts_with(&base_canonical) {
            warn!(
                "Path {} is outside base directory {}",
                canonical.display(),
                base_canonical.display()
            );
            return Err(Error::InvalidParameter(
                "Path is outside allowed directory".into(),
            ));
        }
    }

    debug!(
        "Sanitized path: {} -> {}",
        path.display(),
        canonical.display()
    );
    Ok(canonical)
}

/// Validate JSON input size
pub fn validate_json_size(json: &str) -> Result<()> {
    if json.len() > MAX_JSON_SIZE {
        return Err(Error::InvalidParameter(format!(
            "JSON input too large: {} bytes (max: {} bytes)",
            json.len(),
            MAX_JSON_SIZE
        )));
    }
    Ok(())
}

/// Validate text input for shaping
pub fn validate_text_input(text: &str) -> Result<()> {
    if text.len() > MAX_TEXT_LENGTH {
        return Err(Error::InvalidParameter(format!(
            "Text too long: {} characters (max: {} characters)",
            text.len(),
            MAX_TEXT_LENGTH
        )));
    }

    // Check for control characters that might cause issues
    if text.chars().any(|c| c.is_control() && !c.is_whitespace()) {
        return Err(Error::InvalidParameter(
            "Text contains invalid control characters".into(),
        ));
    }

    Ok(())
}

/// Validate font file size
pub fn validate_font_size(size: usize) -> Result<()> {
    if size > MAX_FONT_SIZE {
        return Err(Error::InvalidParameter(format!(
            "Font file too large: {} bytes (max: {} bytes)",
            size, MAX_FONT_SIZE
        )));
    }
    Ok(())
}

/// Memory usage tracker
pub struct MemoryMonitor {
    initial_usage: usize,
    max_allowed: usize,
}

impl MemoryMonitor {
    /// Create a new memory monitor
    pub fn new(max_allowed: usize) -> Self {
        Self {
            initial_usage: Self::current_usage(),
            max_allowed,
        }
    }

    /// Get current memory usage (simplified - in production use proper memory tracking)
    fn current_usage() -> usize {
        // This is a simplified placeholder
        // In production, use system-specific memory tracking
        0
    }

    /// Check if memory usage is within limits
    pub fn check_usage(&self) -> Result<()> {
        let current = Self::current_usage();
        let delta = current.saturating_sub(self.initial_usage);

        if delta > self.max_allowed {
            return Err(Error::InvalidParameter(format!(
                "Memory usage exceeded: {} bytes (max: {} bytes)",
                delta, self.max_allowed
            )));
        }
        Ok(())
    }
}

/// Timeout wrapper for operations
pub struct TimeoutGuard {
    start: std::time::Instant,
    timeout: Duration,
    operation: String,
}

impl TimeoutGuard {
    /// Create a new timeout guard
    pub fn new(operation: impl Into<String>, timeout: Duration) -> Self {
        Self {
            start: std::time::Instant::now(),
            timeout,
            operation: operation.into(),
        }
    }

    /// Check if the operation has timed out
    pub fn check(&self) -> Result<()> {
        if self.start.elapsed() > self.timeout {
            return Err(Error::InvalidParameter(format!(
                "Operation '{}' timed out after {:?}",
                self.operation, self.timeout
            )));
        }
        Ok(())
    }

    /// Get elapsed time
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_path_sanitization_rejects_traversal() {
        let result = sanitize_path("../etc/passwd", None);
        assert!(result.is_err());

        let result = sanitize_path("~/sensitive", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_path_sanitization_with_base_dir() {
        let dir = tempdir().unwrap();
        let base = dir.path();

        // Create a subdirectory
        let subdir = base.join("subdir");
        std::fs::create_dir(&subdir).unwrap();

        // Valid path within base
        let valid_path = sanitize_path("subdir", Some(base));
        assert!(valid_path.is_ok());

        // Path outside base should fail
        let parent = base.parent().unwrap();
        let result = sanitize_path(parent, Some(base));
        assert!(result.is_err());
    }

    #[test]
    fn test_json_size_validation() {
        let small_json = r#"{"test": "data"}"#;
        assert!(validate_json_size(small_json).is_ok());

        let large_json = "x".repeat(MAX_JSON_SIZE + 1);
        assert!(validate_json_size(&large_json).is_err());
    }

    #[test]
    fn test_text_validation() {
        let valid_text = "Hello, world!";
        assert!(validate_text_input(valid_text).is_ok());

        let long_text = "x".repeat(MAX_TEXT_LENGTH + 1);
        assert!(validate_text_input(&long_text).is_err());

        let text_with_control = "Hello\x00World";
        assert!(validate_text_input(text_with_control).is_err());
    }

    #[test]
    fn test_font_size_validation() {
        assert!(validate_font_size(1024).is_ok());
        assert!(validate_font_size(MAX_FONT_SIZE + 1).is_err());
    }

    #[test]
    fn test_timeout_guard() {
        let guard = TimeoutGuard::new("test op", Duration::from_secs(1));
        assert!(guard.check().is_ok());

        std::thread::sleep(Duration::from_millis(10));
        assert!(guard.check().is_ok());
        assert!(guard.elapsed() >= Duration::from_millis(10));
    }

    #[test]
    fn test_memory_monitor() {
        let monitor = MemoryMonitor::new(MAX_MEMORY_USAGE);
        // In this simplified version, check always succeeds
        assert!(monitor.check_usage().is_ok());
    }
}
