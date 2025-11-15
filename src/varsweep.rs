// this_file: src/varsweep.rs

//! Batch variable font coordinate sweep API for font matching optimization.
//!
//! This module provides specialized APIs for rendering the same glyph at multiple
//! variation coordinates in parallel, which is critical for font matching tools
//! that need to explore the variable font design space efficiently.

use crate::batch::{Job, JobResult, MetricsOutput};
use crate::error::{Error, Result};
use crate::fonts::FontLoader;
use crate::{process_job_with_options, ExecutionOptions};
use rayon::prelude::*;
use std::collections::HashMap;

/// Coordinate set for a variable font (axis tag → value).
pub type VariationCoords = HashMap<String, f32>;

/// Result from a single coordinate sweep point.
#[derive(Debug, Clone)]
pub struct SweepPoint {
    /// The variation coordinates used
    pub coords: VariationCoords,
    /// Calculated metrics (density, beam)
    pub metrics: MetricsOutput,
    /// Render time in milliseconds
    pub render_ms: f64,
}

/// Sweep configuration for batch rendering.
pub struct SweepConfig {
    /// Font path
    pub font_path: String,
    /// Font size in points
    pub font_size: u32,
    /// Text to render (usually a single glyph)
    pub text: String,
    /// Canvas width
    pub width: u32,
    /// Canvas height
    pub height: u32,
    /// List of variation coordinate sets to render
    pub coord_sets: Vec<VariationCoords>,
}

/// Render the same glyph at multiple variation coordinates in parallel.
///
/// This is optimized for font matching use cases where you need to explore
/// the variable font design space by rendering many coordinate combinations
/// and comparing their metrics.
///
/// # Arguments
///
/// * `config` - Sweep configuration with font, text, and coordinate sets
/// * `options` - Execution options (cache sizes, parallelism)
///
/// # Returns
///
/// Vector of sweep points with metrics for each coordinate set, in the same
/// order as `config.coord_sets`.
///
/// # Errors
///
/// Returns first error encountered. For partial results, use `sweep_with_fallback`.
///
/// # Example
///
/// ```rust,no_run
/// use haforu::varsweep::{SweepConfig, VariationCoords, render_variation_sweep};
/// use haforu::ExecutionOptions;
/// use std::collections::HashMap;
///
/// // Generate coordinate sets for weight sweep
/// let mut coord_sets = Vec::new();
/// for wght in (100..=900).step_by(50) {
///     let mut coords = HashMap::new();
///     coords.insert("wght".to_string(), wght as f32);
///     coord_sets.push(coords);
/// }
///
/// let config = SweepConfig {
///     font_path: "/path/to/font.ttf".to_string(),
///     font_size: 1000,
///     text: "A".to_string(),
///     width: 3000,
///     height: 1200,
///     coord_sets,
/// };
///
/// let font_loader = FontLoader::new(512);
/// let mut options = ExecutionOptions::new(None, None);
/// options.set_glyph_cache_capacity(2048);
/// let results = render_variation_sweep(&config, &font_loader, &options)?;
///
/// for (i, point) in results.iter().enumerate() {
///     println!("Coords {:?}: density={:.4}, beam={:.4}",
///              point.coords, point.metrics.density, point.metrics.beam);
/// }
/// # Ok::<(), haforu::Error>(())
/// ```
pub fn render_variation_sweep(
    config: &SweepConfig,
    font_loader: &FontLoader,
    options: &ExecutionOptions,
) -> Result<Vec<SweepPoint>> {
    // Process all coordinate sets in parallel using Rayon
    let results: Result<Vec<SweepPoint>> = config
        .coord_sets
        .par_iter()
        .map(|coords| render_single_point(config, coords, font_loader, options))
        .collect();

    results
}

/// Render the same glyph at multiple variation coordinates with fallback.
///
/// Unlike `render_variation_sweep`, this returns partial results even if some
/// coordinate sets fail. Failed renders are replaced with None.
///
/// # Arguments
///
/// * `config` - Sweep configuration
/// * `options` - Execution options
///
/// # Returns
///
/// Vector of Option<SweepPoint> where None indicates a failed render.
pub fn render_variation_sweep_with_fallback(
    config: &SweepConfig,
    font_loader: &FontLoader,
    options: &ExecutionOptions,
) -> Vec<Option<SweepPoint>> {
    config
        .coord_sets
        .par_iter()
        .map(|coords| render_single_point(config, coords, font_loader, options).ok())
        .collect()
}

/// Render a single point in the variation sweep.
fn render_single_point(
    config: &SweepConfig,
    coords: &VariationCoords,
    font_loader: &FontLoader,
    options: &ExecutionOptions,
) -> Result<SweepPoint> {
    use camino::Utf8PathBuf;
    use std::time::Instant;

    // Build job for this coordinate set
    let job = Job {
        id: format!("sweep_{}", coords.len()), // Temporary ID
        font: crate::batch::FontConfig {
            path: Utf8PathBuf::from(&config.font_path),
            size: config.font_size,
            variations: coords.clone(),
        },
        text: crate::batch::TextConfig {
            content: config.text.clone(),
            script: None,
            direction: None,
            language: None,
            features: vec![],
        },
        rendering: crate::batch::RenderingConfig {
            format: "metrics".to_string(),
            encoding: "json".to_string(),
            width: config.width,
            height: config.height,
        },
    };

    // Render with timing
    let start = Instant::now();
    let result = process_job_with_options(&job, font_loader, options);
    let render_ms = start.elapsed().as_secs_f64() * 1000.0;

    // Extract metrics from result
    match result {
        JobResult {
            status,
            metrics: Some(metrics),
            ..
        } if status == "success" => Ok(SweepPoint {
            coords: coords.clone(),
            metrics,
            render_ms,
        }),
        JobResult {
            error: Some(err), ..
        } => Err(Error::Internal(format!("Sweep render failed: {}", err))),
        _ => Err(Error::Internal(
            "Unexpected result format from sweep render".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sweep_single_coordinate_set() {
        let mut coords = HashMap::new();
        coords.insert("wght".to_string(), 700.0);

        let config = SweepConfig {
            font_path: "testdata/fonts/Arial-Black.ttf".to_string(),
            font_size: 256,
            text: "A".to_string(),
            width: 64,
            height: 64,
            coord_sets: vec![coords.clone()],
        };

        let font_loader = FontLoader::new(512);
        let mut options = ExecutionOptions::new(None, None);
        options.set_glyph_cache_capacity(2048);
        let results = render_variation_sweep(&config, &font_loader, &options).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].coords, coords);
        assert!(results[0].metrics.density > 0.0);
        assert!(results[0].metrics.beam > 0.0);
        assert!(results[0].render_ms > 0.0);
    }

    #[test]
    fn sweep_multiple_coordinates_parallel() {
        // Create 10 different weight values
        let mut coord_sets = Vec::new();
        for wght in (100..=900).step_by(100) {
            let mut coords = HashMap::new();
            coords.insert("wght".to_string(), wght as f32);
            coord_sets.push(coords);
        }

        let config = SweepConfig {
            font_path: "testdata/fonts/Arial-Black.ttf".to_string(),
            font_size: 256,
            text: "A".to_string(),
            width: 64,
            height: 64,
            coord_sets: coord_sets.clone(),
        };

        let font_loader = FontLoader::new(512);
        let mut options = ExecutionOptions::new(None, None);
        options.set_glyph_cache_capacity(2048);
        let results = render_variation_sweep(&config, &font_loader, &options).unwrap();

        assert_eq!(results.len(), 9);
        // All renders should succeed
        for (i, result) in results.iter().enumerate() {
            assert_eq!(result.coords, coord_sets[i]);
            assert!(result.metrics.density > 0.0);
        }
    }

    #[test]
    fn sweep_with_fallback_handles_errors() {
        let mut coord_sets = vec![];

        // Valid coord set
        let mut valid_coords = HashMap::new();
        valid_coords.insert("wght".to_string(), 700.0);
        coord_sets.push(valid_coords);

        let config = SweepConfig {
            font_path: "testdata/fonts/Arial-Black.ttf".to_string(),
            font_size: 256,
            text: "A".to_string(),
            width: 64,
            height: 64,
            coord_sets,
        };

        let font_loader = FontLoader::new(512);
        let mut options = ExecutionOptions::new(None, None);
        options.set_glyph_cache_capacity(2048);
        let results = render_variation_sweep_with_fallback(&config, &font_loader, &options);

        assert_eq!(results.len(), 1);
        assert!(results[0].is_some());
    }
}
