// this_file: src/error.rs
//! Error types for the haforu library

use thiserror::Error;

/// Main error type for haforu operations
#[derive(Debug, Error)]
pub enum Error {
    /// Font file loading or parsing error
    #[error("Font error: {0}")]
    Font(String),

    /// JSON parsing or validation error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// IO operation error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Storage backend error
    #[error("Storage error: {0}")]
    Storage(String),

    /// Shaping operation error
    #[error("Shaping error: {0}")]
    Shaping(String),

    /// Rendering error
    #[error("Rendering error: {0}")]
    Rendering(String),

    /// Invalid input parameter
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    /// Resource not found
    #[error("Resource not found: {0}")]
    NotFound(String),
}

/// Result type alias for haforu operations
pub type Result<T> = std::result::Result<T, Error>;
