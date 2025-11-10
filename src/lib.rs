// this_file: src/lib.rs
//! Haforu - A Rust library for font shaping and rendering
//!
//! This library provides functionality for:
//! - Font loading and parsing
//! - Text shaping using HarfRust
//! - GPU-accelerated rendering with Vello
//! - Batch processing via JSON job specifications
//! - High-performance storage of pre-rendered results

pub mod error;
pub mod font_loader;
pub mod json_parser;
pub mod logging;
pub mod mmap_font;
pub mod orchestrator;
pub mod rasterize;
pub mod security;
pub mod shaping;
pub mod storage;

// Re-export commonly used types
pub use error::{Error, Result};
pub use font_loader::FontLoader;
pub use json_parser::{Job, JobResult, JobSpec};
pub use orchestrator::{JobOrchestrator, JobStats, ParallelizationStrategy};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
