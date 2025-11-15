// this_file: src/lib.rs

//! Haforu: High-performance batch font renderer for FontSimi.
//!
//! This library provides zero-copy font loading, text shaping, and rasterization
//! with support for variable fonts and batch processing via JSONL.
//!
//! ## Architecture
//!
//! - **batch**: Job specification and JSONL I/O
//! - **fonts**: Memory-mapped font loading and caching
//! - **shaping**: Text shaping with HarfBuzz
//! - **render**: Glyph rasterization with zeno
//! - **output**: PGM/PNG image generation
//! - **error**: Error types and handling
//!
//! ## Example
//!
//! ```rust,no_run
//! use haforu::{FontLoader, TextShaper, GlyphRasterizer, Image, ImageOutput};
//! use std::collections::HashMap;
//! use camino::Utf8Path;
//!
//! // Load font with variations
//! let loader = FontLoader::new(512);
//! let mut coords = HashMap::new();
//! coords.insert("wght".to_string(), 600.0);
//! let font = loader.load_font(Utf8Path::new("font.ttf"), &coords)?;
//!
//! // Shape text
//! let shaper = TextShaper::new();
//! let shaped = shaper.shape(&font, "Hello", 100.0, Utf8Path::new("font.ttf").as_std_path())?;
//!
//! // Rasterize
//! let rasterizer = GlyphRasterizer::new();
//! let image: Image = rasterizer.render_text(
//!     &font,
//!     &shaped,
//!     3000,
//!     1200,
//!     0.0,
//!     Utf8Path::new("font.ttf").as_std_path(),
//! )?;
//!
//! // Generate PGM
//! let pgm = ImageOutput::write_pgm_binary(image.pixels(), 3000, 1200)?;
//! let base64 = ImageOutput::encode_base64(&pgm);
//! # Ok::<(), haforu::Error>(())
//! ```

pub mod batch;
mod bufpool;
mod cache;
pub mod error;
pub mod fonts;
pub mod image_ops;  // NEW: Image processing for font matching (v2.2)
pub mod output;
pub mod render;
pub mod security;
pub mod shaping;
pub mod varsweep;

// Python bindings (optional feature)
#[cfg(feature = "python")]
pub mod python;

use crate::cache::{GlyphCache, GlyphCacheKey};
use std::time::Duration;

// Re-export main types
pub use batch::{FontResult, Job, JobResult, JobSpec, MetricsOutput, RenderingOutput, TimingInfo};
pub use error::{Error, Result};
pub use fonts::{CacheStats, FontInstance, FontLoader};
pub use output::ImageOutput;
pub use render::{GlyphRasterizer, Image};
pub use shaping::{ShapeRequest, ShapedText, TextShaper};
pub use varsweep::{
    render_variation_sweep, render_variation_sweep_with_fallback, SweepConfig, SweepPoint,
    VariationCoords,
};

#[derive(Clone, Debug)]
pub(crate) enum JobPayload {
    Rendering(RenderingOutput),
    Metrics(MetricsOutput),
}

/// Execution options for processing jobs.
#[derive(Clone, Debug, Default)]
pub struct ExecutionOptions {
    /// Optional base directory to constrain font paths.
    pub base_dir: Option<camino::Utf8PathBuf>,
    /// Optional per-job timeout in milliseconds.
    pub timeout_ms: Option<u64>,
    /// Optional glyph cache shared across workers.
    pub(crate) glyph_cache: Option<GlyphCache>,
}

impl ExecutionOptions {
    /// Convenience constructor to avoid exposing glyph cache internals.
    pub fn new(base_dir: Option<camino::Utf8PathBuf>, timeout_ms: Option<u64>) -> Self {
        Self {
            base_dir,
            timeout_ms,
            glyph_cache: None,
        }
    }

    /// Attach or resize the glyph cache.
    pub fn set_glyph_cache_capacity(&mut self, capacity: usize) {
        if capacity == 0 {
            if let Some(cache) = self.glyph_cache.as_ref() {
                cache.clear();
            }
            self.glyph_cache = None;
            return;
        }
        match self.glyph_cache.as_ref() {
            Some(cache) => cache.set_capacity(capacity),
            None => {
                self.glyph_cache = GlyphCache::new(capacity);
            }
        }
    }

    /// Current glyph-cache capacity (0 when disabled).
    pub fn glyph_cache_capacity(&self) -> usize {
        self.glyph_cache
            .as_ref()
            .map(|cache| cache.stats().capacity)
            .unwrap_or(0)
    }

    /// Snapshot glyph-cache stats when enabled.
    pub fn glyph_cache_stats(&self) -> Option<(usize, usize, u64)> {
        self.glyph_cache.as_ref().map(|cache| {
            let stats = cache.stats();
            (stats.entries, stats.capacity, stats.hits)
        })
    }
}

/// Process a single job and return the result.
///
/// This is the main entry point for batch processing.
pub fn process_job(job: &Job, font_loader: &FontLoader) -> JobResult {
    process_job_with_options(job, font_loader, &ExecutionOptions::default())
}

/// Process a single job with execution options and return the result.
pub fn process_job_with_options(
    job: &Job,
    font_loader: &FontLoader,
    opts: &ExecutionOptions,
) -> JobResult {
    use std::time::Instant;

    if let Err(err) = job.validate() {
        return JobResult::error(job.id.clone(), err.to_string());
    }

    let start = Instant::now();
    let timeout_guard = opts
        .timeout_ms
        .map(|ms| crate::security::TimeoutGuard::new(std::time::Duration::from_millis(ms)));

    let mut font_metadata: Option<FontResult> = None;

    let outcome = (|| -> Result<(JobPayload, Option<GlyphCacheKey>, bool)> {
        if let Some(ref guard) = timeout_guard {
            guard.check("start")?;
        }
        let font_path = if let Some(base) = opts.base_dir.as_ref() {
            crate::security::sanitize_path(&job.font.path, Some(base.as_path()))?
        } else {
            job.font.path.clone()
        };
        let font_instance = font_loader.load_font(&font_path, &job.font.variations)?;
        let applied_variations = font_instance.coordinates().clone();
        font_metadata = Some(FontResult {
            path: font_path.to_string(),
            variations: applied_variations.clone(),
        });
        log::debug!(
            "Job {} sanitized coordinates: {:?}",
            job.id,
            applied_variations
        );

        let cache_key = opts
            .glyph_cache
            .as_ref()
            .map(|_| build_cache_key(job, &font_path, &applied_variations));
        if let (Some(cache), Some(ref key)) = (opts.glyph_cache.as_ref(), cache_key.as_ref()) {
            if let Some(payload) = cache.get(key) {
                return Ok((payload, cache_key, true));
            }
        }

        let shaper = TextShaper::new();
        let shape_request = ShapeRequest::from(&job.text);
        let shaped = shaper.shape_with_request(
            &font_instance,
            &shape_request,
            job.font.size as f32,
            font_path.as_std_path(),
        )?;

        if let Some(ref guard) = timeout_guard {
            guard.check("shape")?;
        }

        let rasterizer = GlyphRasterizer::new();
        let image = rasterizer.render_text(
            &font_instance,
            &shaped,
            job.rendering.width,
            job.rendering.height,
            0.0,
            font_path.as_std_path(),
        )?;

        if job.rendering.format == "metrics" {
            if let Some(ref guard) = timeout_guard {
                guard.check("render")?;
            }
            return Ok((
                JobPayload::Metrics(MetricsOutput {
                    density: image.density(),
                    beam: image.beam(),
                }),
                cache_key,
                false,
            ));
        }

        let bbox = image.calculate_bbox();

        if let Some(ref guard) = timeout_guard {
            guard.check("render")?;
        }

        let image_data = match job.rendering.format.as_str() {
            "pgm" => ImageOutput::write_pgm_binary(
                image.pixels(),
                job.rendering.width,
                job.rendering.height,
            )?,
            "png" => {
                ImageOutput::write_png(image.pixels(), job.rendering.width, job.rendering.height)?
            }
            _ => {
                return Err(Error::InvalidRenderParams {
                    reason: format!("Unsupported output format: {}", job.rendering.format),
                })
            }
        };

        let base64_data = ImageOutput::encode_base64(&image_data);

        Ok((
            JobPayload::Rendering(RenderingOutput {
                format: job.rendering.format.clone(),
                encoding: "base64".to_string(),
                data: base64_data,
                width: job.rendering.width,
                height: job.rendering.height,
                actual_bbox: bbox,
            }),
            cache_key,
            false,
        ))
    })();

    let elapsed = start.elapsed();

    match outcome {
        Ok((payload, cache_key, from_cache)) => {
            if !from_cache {
                if let (Some(cache), Some(key)) = (opts.glyph_cache.as_ref(), cache_key) {
                    cache.insert(key, payload.clone());
                }
            }
            build_job_result(job, font_metadata.clone(), payload, elapsed)
        }
        Err(e) => JobResult {
            id: job.id.clone(),
            status: "error".to_string(),
            rendering: None,
            metrics: None,
            error: Some(e.to_string()),
            font: font_metadata.clone(),
            timing: TimingInfo {
                shape_ms: 0.0,
                render_ms: 0.0,
                total_ms: elapsed.as_secs_f64() * 1000.0,
            },
            memory: None,
        },
    }
}

fn build_job_result(
    job: &Job,
    font_metadata: Option<FontResult>,
    payload: JobPayload,
    elapsed: Duration,
) -> JobResult {
    match payload {
        JobPayload::Rendering(output) => JobResult {
            id: job.id.clone(),
            status: "success".to_string(),
            rendering: Some(output),
            metrics: None,
            error: None,
            font: font_metadata,
            timing: TimingInfo {
                shape_ms: 0.0,
                render_ms: 0.0,
                total_ms: elapsed.as_secs_f64() * 1000.0,
            },
            memory: None,
        },
        JobPayload::Metrics(metrics) => JobResult {
            id: job.id.clone(),
            status: "success".to_string(),
            rendering: None,
            metrics: Some(metrics),
            error: None,
            font: font_metadata,
            timing: TimingInfo {
                shape_ms: 0.0,
                render_ms: 0.0,
                total_ms: elapsed.as_secs_f64() * 1000.0,
            },
            memory: None,
        },
    }
}

fn build_cache_key(
    job: &Job,
    font_path: &camino::Utf8PathBuf,
    applied_variations: &std::collections::HashMap<String, f32>,
) -> GlyphCacheKey {
    use smallvec::SmallVec;
    let mut variations: SmallVec<[(String, u32); 4]> = applied_variations
        .iter()
        .map(|(axis, value)| (axis.clone(), value.to_bits()))
        .collect();
    variations.sort_by(|a, b| a.0.cmp(&b.0));
    GlyphCacheKey {
        font_path: font_path.to_string(),
        font_size: job.font.size,
        width: job.rendering.width,
        height: job.rendering.height,
        format: job.rendering.format.clone(),
        encoding: job.rendering.encoding.clone(),
        text: job.text.content.clone(),
        script: job.text.script.clone(),
        variations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::batch::{FontConfig, RenderingConfig, TextConfig};
    use camino::Utf8PathBuf;
    use std::collections::HashMap;

    #[test]
    fn test_module_structure() {
        // Verify key modules stay linkable for downstream crates.
        let _ = batch::JobSpec {
            version: "1.0".to_string(),
            jobs: vec![],
        };
        let _ = FontLoader::new(512);
        let _ = TextShaper::new();
        let _ = GlyphRasterizer::new();
    }

    #[test]
    fn process_job_returns_error_for_invalid_dimensions() {
        let job = Job {
            id: "invalid-canvas".to_string(),
            font: FontConfig {
                path: Utf8PathBuf::from("/tmp/nonexistent.ttf"),
                size: 1000,
                variations: HashMap::new(),
            },
            text: TextConfig {
                content: "A".to_string(),
                script: Some("Latn".to_string()),
                direction: None,
                language: None,
                features: Vec::new(),
            },
            rendering: RenderingConfig {
                format: "pgm".to_string(),
                encoding: "base64".to_string(),
                width: 0,
                height: 64,
            },
        };
        let loader = FontLoader::new(8);
        let result = process_job(&job, &loader);
        assert_eq!(result.status, "error");
        let message = result.error.unwrap_or_default();
        assert!(message.contains("Canvas"));
        assert!(
            result.font.is_none(),
            "font metadata should be absent when validation fails"
        );
    }

    #[test]
    fn process_job_attaches_sanitized_variations() {
        let mut variations = HashMap::new();
        variations.insert("wght".to_string(), 2500.0);
        variations.insert("ZZZZ".to_string(), 42.0);
        let job = Job {
            id: "mutator".to_string(),
            font: FontConfig {
                path: Utf8PathBuf::from("testdata/fonts/IBMPlexSans-VF.ttf"),
                size: 640,
                variations,
            },
            text: TextConfig {
                content: "A".to_string(),
                script: Some("Latn".to_string()),
                direction: None,
                language: None,
                features: Vec::new(),
            },
            rendering: RenderingConfig {
                format: "pgm".to_string(),
                encoding: "base64".to_string(),
                width: 128,
                height: 128,
            },
        };
        let loader = FontLoader::new(32);
        let result = process_job(&job, &loader);
        assert_eq!(
            result.status, "success",
            "render should succeed for bundled font"
        );
        let font_meta = result.font.expect("font metadata should be attached");
        assert!(font_meta.path.ends_with("IBMPlexSans-VF.ttf"));
        assert!(font_meta.variations.contains_key("wght"));
        assert!(
            !font_meta.variations.contains_key("ZZZZ"),
            "unknown axes should be dropped"
        );
        let wght = *font_meta
            .variations
            .get("wght")
            .expect("wght should be clamped into result");
        assert!(
            wght <= 900.0 && wght >= 100.0,
            "wght value should be clamped into sane bounds"
        );
    }

    #[test]
    fn process_job_metrics_format_returns_metrics_payload() {
        let job = Job {
            id: "metrics-job".to_string(),
            font: FontConfig {
                path: Utf8PathBuf::from("testdata/fonts/Arial-Black.ttf"),
                size: 256,
                variations: HashMap::new(),
            },
            text: TextConfig {
                content: "A".to_string(),
                script: Some("Latn".to_string()),
                direction: None,
                language: None,
                features: Vec::new(),
            },
            rendering: RenderingConfig {
                format: "metrics".to_string(),
                encoding: "json".to_string(),
                width: 64,
                height: 64,
            },
        };
        let loader = FontLoader::new(32);
        let result = process_job(&job, &loader);
        assert_eq!(result.status, "success", "metrics job should succeed");
        assert!(
            result.rendering.is_none(),
            "metrics format should not include rendering payload"
        );
        let metrics = result.metrics.expect("metrics payload required");
        assert!(
            (0.0..=1.0).contains(&metrics.density),
            "density should be normalized: {}",
            metrics.density
        );
        assert!(
            (0.0..=1.0).contains(&metrics.beam),
            "beam should be normalized: {}",
            metrics.beam
        );
    }

    #[test]
    fn process_job_uses_glyph_cache_for_identical_jobs() {
        let job = Job {
            id: "cache-me".to_string(),
            font: FontConfig {
                path: Utf8PathBuf::from("testdata/fonts/Arial-Black.ttf"),
                size: 256,
                variations: HashMap::new(),
            },
            text: TextConfig {
                content: "C".to_string(),
                script: Some("Latn".to_string()),
                direction: None,
                language: None,
                features: Vec::new(),
            },
            rendering: RenderingConfig {
                format: "pgm".to_string(),
                encoding: "base64".to_string(),
                width: 64,
                height: 64,
            },
        };
        let loader = FontLoader::new(32);
        let cache = GlyphCache::new(4).expect("cache enabled");
        let mut opts = ExecutionOptions::default();
        opts.glyph_cache = Some(cache.clone());

        let first = process_job_with_options(&job, &loader, &opts);
        assert_eq!(first.status, "success");

        let second = process_job_with_options(&job, &loader, &opts);
        assert_eq!(second.status, "success");
        assert_eq!(
            first.rendering.as_ref().map(|r| &r.data),
            second.rendering.as_ref().map(|r| &r.data)
        );
        assert!(
            cache.stats().hits >= 1,
            "glyph cache should record a hit after re-render"
        );
    }
}
