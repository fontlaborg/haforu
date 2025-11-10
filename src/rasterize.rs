// this_file: src/rasterize.rs
//! CPU-based font rasterization using skrifa and zeno

use crate::error::{Error, Result};
use crate::json_parser::GlyphInfo;
use log::{debug, info};
use read_fonts::types::GlyphId;
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::{FontRef, MetadataProvider};
use std::sync::Arc;
use zeno::{Command, Mask};

/// A single rendered glyph
#[derive(Debug, Clone)]
pub struct RenderedGlyph {
    /// Glyph ID
    pub glyph_id: u32,
    /// Bitmap width in pixels
    pub width: u32,
    /// Bitmap height in pixels
    pub height: u32,
    /// Left bearing (offset from origin to left edge)
    pub left: i32,
    /// Top bearing (offset from origin to top edge)
    pub top: i32,
    /// Horizontal advance
    pub advance_x: f32,
    /// Vertical advance
    pub advance_y: f32,
    /// Alpha mask data (8-bit grayscale)
    pub data: Vec<u8>,
}

/// CPU-based rasterizer using skrifa for outline extraction and zeno for rendering
pub struct CpuRasterizer {
    /// Target pixels per em
    #[allow(dead_code)]
    ppem: f32,
    /// Subpixel precision (4 or 8 for 1/4 or 1/8 pixel)
    subpixel_precision: u32,
}

impl CpuRasterizer {
    /// Create a new CPU rasterizer
    pub fn new(ppem: f32) -> Self {
        Self {
            ppem,
            subpixel_precision: 4, // 1/4 pixel precision by default
        }
    }

    /// Set subpixel precision (4 or 8 for 1/4 or 1/8 pixel)
    pub fn set_subpixel_precision(&mut self, precision: u32) {
        self.subpixel_precision = precision.clamp(1, 16);
    }

    /// Render a single glyph
    pub fn render_glyph(
        &self,
        font: &FontRef,
        glyph_id: GlyphId,
        size: f32,
    ) -> Result<RenderedGlyph> {
        debug!("Rendering glyph {} at size {}", glyph_id, size);

        // Create skrifa Size object
        let size_obj = Size::new(size);

        // Get the glyph outline
        let glyph = font
            .outline_glyphs()
            .get(glyph_id)
            .ok_or_else(|| Error::Rendering(format!("Glyph {} not found", glyph_id)))?;

        // Create a temporary pen to extract bounds
        let mut bounds_pen = BoundsPen::new();
        let settings = DrawSettings::unhinted(size_obj, LocationRef::default());
        let _ = glyph.draw(settings, &mut bounds_pen);

        let bounds = bounds_pen.bounds();

        if bounds.x_min >= bounds.x_max || bounds.y_min >= bounds.y_max {
            // Empty glyph (like space) - get advance from metrics if available
            // For now, use default advance
            return Ok(RenderedGlyph {
                glyph_id: glyph_id.to_u32(),
                width: 0,
                height: 0,
                left: 0,
                top: 0,
                advance_x: size * 0.5, // Default advance for space
                advance_y: 0.0,
                data: vec![],
            });
        }

        // Calculate bitmap dimensions in pixel units
        // Note: skrifa provides outline coordinates already scaled to the requested size.
        // They are floating-point pixel coordinates, so we round conservatively.
        let left = bounds.x_min.floor() as i32;
        let top = (-bounds.y_min).ceil() as i32; // Flip for bitmap coordinates
        let width = (bounds.x_max - bounds.x_min).ceil().max(0.0) as u32;
        let height = (bounds.y_max - bounds.y_min).ceil().max(0.0) as u32;

        // Create ZenoPen adapter to collect paths
        let mut pen = ZenoPen::new();

        // Extract outline using skrifa (we already got the glyph above)
        let settings = DrawSettings::unhinted(size_obj, LocationRef::default());

        glyph
            .draw(settings, &mut pen)
            .map_err(|e| Error::Rendering(format!("Failed to extract outline: {:?}", e)))?;

        // Build the path and render with zeno
        let path = pen.build();

        // Apply transform to position glyph correctly
        let transform = zeno::Transform::translation(-left as f32, -top as f32);

        // Create mask and render (returns tuple of (data, placement))
        let (mask_data, _placement) = Mask::new(&path)
            .transform(Some(transform))
            .size(width, height)
            .render();

        Ok(RenderedGlyph {
            glyph_id: glyph_id.to_u32(),
            width,
            height,
            left,
            top,
            advance_x: size * 0.6, // Default advance - would need metrics API for accurate value
            advance_y: 0.0,
            data: mask_data,
        })
    }

    /// Render multiple glyphs from shaped output
    pub fn render_shaped_glyphs(
        &self,
        font: &FontRef,
        glyphs: &[GlyphInfo],
        size: f32,
    ) -> Result<Vec<RenderedGlyph>> {
        let mut rendered = Vec::with_capacity(glyphs.len());

        for glyph_info in glyphs {
            let glyph_id = GlyphId::new(glyph_info.glyph_id);
            match self.render_glyph(font, glyph_id, size) {
                Ok(mut rg) => {
                    // Apply shaped advances (already in pixel units)
                    rg.advance_x = glyph_info.x_advance as f32;
                    rg.advance_y = glyph_info.y_advance as f32;
                    rendered.push(rg);
                }
                Err(e) => {
                    log::warn!("Failed to render glyph {}: {}", glyph_info.glyph_id, e);
                    // Create placeholder for missing glyph
                    rendered.push(RenderedGlyph {
                        glyph_id: glyph_info.glyph_id,
                        width: 0,
                        height: 0,
                        left: 0,
                        top: 0,
                        advance_x: glyph_info.x_advance as f32 / 64.0,
                        advance_y: glyph_info.y_advance as f32 / 64.0,
                        data: vec![],
                    });
                }
            }
        }

        Ok(rendered)
    }

    /// Render text to a single bitmap
    pub fn render_text_line(
        &self,
        _font: &FontRef,
        glyphs: &[RenderedGlyph],
    ) -> Result<(Vec<u8>, u32, u32)> {
        if glyphs.is_empty() {
            return Ok((vec![], 0, 0));
        }

        // Calculate total dimensions
        let mut total_width = 0.0;
        let mut min_top = i32::MAX;
        let mut max_bottom = i32::MIN;

        for glyph in glyphs {
            total_width += glyph.advance_x;
            if glyph.height > 0 {
                min_top = min_top.min(glyph.top);
                max_bottom = max_bottom.max(glyph.top - glyph.height as i32);
            }
        }

        let width = total_width.ceil() as u32;
        let height = if min_top == i32::MAX {
            32 // Default height for empty text
        } else {
            (min_top - max_bottom) as u32
        };

        // Create bitmap
        let mut bitmap = vec![0u8; (width * height) as usize];

        // Composite glyphs
        let mut x_offset = 0.0;
        for glyph in glyphs {
            if glyph.width > 0 && glyph.height > 0 {
                let x = (x_offset + glyph.left as f32) as i32;
                let y = min_top - glyph.top;

                // Blit glyph to bitmap
                for gy in 0..glyph.height {
                    let dst_y = (y + gy as i32) as usize;
                    if dst_y >= height as usize {
                        continue;
                    }

                    for gx in 0..glyph.width {
                        let dst_x = (x + gx as i32) as usize;
                        if dst_x >= width as usize {
                            continue;
                        }

                        let src_idx = (gy * glyph.width + gx) as usize;
                        let dst_idx = dst_y * width as usize + dst_x;

                        if src_idx < glyph.data.len() && dst_idx < bitmap.len() {
                            // Alpha compositing
                            let bg = bitmap[dst_idx];
                            let fg = glyph.data[src_idx];
                            bitmap[dst_idx] = bg.saturating_add(fg.saturating_mul(255 - bg) / 255);
                        }
                    }
                }
            }
            x_offset += glyph.advance_x;
        }

        info!("Rendered text line: {}x{} pixels", width, height);
        Ok((bitmap, width, height))
    }
}

/// Simple bounding box struct
#[derive(Debug, Clone, Copy)]
struct BoundingBox {
    x_min: f32,
    y_min: f32,
    x_max: f32,
    y_max: f32,
}

/// Pen for calculating bounding box
struct BoundsPen {
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
    has_points: bool,
}

impl BoundsPen {
    fn new() -> Self {
        Self {
            min_x: f32::INFINITY,
            max_x: f32::NEG_INFINITY,
            min_y: f32::INFINITY,
            max_y: f32::NEG_INFINITY,
            has_points: false,
        }
    }

    fn bounds(&self) -> BoundingBox {
        if !self.has_points {
            BoundingBox {
                x_min: 0.0,
                y_min: 0.0,
                x_max: 0.0,
                y_max: 0.0,
            }
        } else {
            BoundingBox {
                x_min: self.min_x,
                y_min: self.min_y,
                x_max: self.max_x,
                y_max: self.max_y,
            }
        }
    }

    fn update_bounds(&mut self, x: f32, y: f32) {
        self.min_x = self.min_x.min(x);
        self.max_x = self.max_x.max(x);
        self.min_y = self.min_y.min(y);
        self.max_y = self.max_y.max(y);
        self.has_points = true;
    }
}

impl OutlinePen for BoundsPen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.update_bounds(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.update_bounds(x, y);
    }

    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.update_bounds(cx, cy);
        self.update_bounds(x, y);
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.update_bounds(cx0, cy0);
        self.update_bounds(cx1, cy1);
        self.update_bounds(x, y);
    }

    fn close(&mut self) {}
}

/// Adapter implementing skrifa's OutlinePen to build zeno paths
struct ZenoPen {
    commands: Vec<Command>,
    current_x: f32,
    current_y: f32,
}

impl ZenoPen {
    fn new() -> Self {
        Self {
            commands: Vec::new(),
            current_x: 0.0,
            current_y: 0.0,
        }
    }

    fn build(self) -> Vec<Command> {
        self.commands
    }
}

impl OutlinePen for ZenoPen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.commands.push(Command::MoveTo((x, y).into()));
        self.current_x = x;
        self.current_y = y;
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.commands.push(Command::LineTo((x, y).into()));
        self.current_x = x;
        self.current_y = y;
    }

    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.commands
            .push(Command::QuadTo((cx, cy).into(), (x, y).into()));
        self.current_x = x;
        self.current_y = y;
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.commands.push(Command::CurveTo(
            (cx0, cy0).into(),
            (cx1, cy1).into(),
            (x, y).into(),
        ));
        self.current_x = x;
        self.current_y = y;
    }

    fn close(&mut self) {
        self.commands.push(Command::Close);
    }
}

/// Parallel rasterizer using Rayon for batch processing
pub struct ParallelRasterizer {
    ppem: f32,
    num_threads: usize,
}

impl ParallelRasterizer {
    /// Create a new parallel rasterizer
    pub fn new(ppem: f32) -> Self {
        Self {
            ppem,
            num_threads: num_cpus::get(),
        }
    }

    /// Render glyphs in parallel
    pub fn render_glyphs_parallel(
        &self,
        font: Arc<Vec<u8>>,
        glyphs: &[GlyphInfo],
        size: f32,
    ) -> Result<Vec<RenderedGlyph>> {
        use rayon::prelude::*;

        let font_ref = FontRef::new(&font)
            .map_err(|e| Error::Font(format!("Failed to create font ref: {}", e)))?;

        let rasterizer = CpuRasterizer::new(self.ppem);

        let rendered: Vec<_> = glyphs
            .par_iter()
            .map(|glyph_info| {
                let glyph_id = GlyphId::new(glyph_info.glyph_id);
                match rasterizer.render_glyph(&font_ref, glyph_id, size) {
                    Ok(mut rg) => {
                        rg.advance_x = glyph_info.x_advance as f32 / 64.0;
                        rg.advance_y = glyph_info.y_advance as f32 / 64.0;
                        rg
                    }
                    Err(e) => {
                        log::warn!("Failed to render glyph {}: {}", glyph_info.glyph_id, e);
                        RenderedGlyph {
                            glyph_id: glyph_info.glyph_id,
                            width: 0,
                            height: 0,
                            left: 0,
                            top: 0,
                            advance_x: glyph_info.x_advance as f32,
                            advance_y: glyph_info.y_advance as f32,
                            data: vec![],
                        }
                    }
                }
            })
            .collect();

        info!(
            "Rendered {} glyphs in parallel using {} threads",
            rendered.len(),
            self.num_threads
        );
        Ok(rendered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_rasterizer_creation() {
        let rasterizer = CpuRasterizer::new(16.0);
        assert_eq!(rasterizer.ppem, 16.0);
        assert_eq!(rasterizer.subpixel_precision, 4);
    }

    #[test]
    fn test_subpixel_precision() {
        let mut rasterizer = CpuRasterizer::new(16.0);
        rasterizer.set_subpixel_precision(8);
        assert_eq!(rasterizer.subpixel_precision, 8);

        rasterizer.set_subpixel_precision(20);
        assert_eq!(rasterizer.subpixel_precision, 16); // Clamped to max

        rasterizer.set_subpixel_precision(0);
        assert_eq!(rasterizer.subpixel_precision, 1); // Clamped to min
    }

    #[test]
    fn test_parallel_rasterizer() {
        let rasterizer = ParallelRasterizer::new(24.0);
        assert_eq!(rasterizer.ppem, 24.0);
        assert!(rasterizer.num_threads > 0);
    }

    #[test]
    fn test_empty_render() {
        let rasterizer = CpuRasterizer::new(16.0);
        let glyphs = vec![];

        // Create dummy font ref for testing
        let dummy_font_data = vec![0u8; 100];
        let font_ref = FontRef::new(&dummy_font_data);

        if let Ok(font) = font_ref {
            let result = rasterizer.render_text_line(&font, &glyphs);
            assert!(result.is_ok());
            let (bitmap, width, height) = result.unwrap();
            assert_eq!(width, 0);
            assert_eq!(height, 0);
            assert!(bitmap.is_empty());
        }
    }
}
