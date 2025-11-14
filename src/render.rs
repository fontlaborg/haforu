// this_file: src/render.rs

//! Glyph rasterization and compositing using zeno.
//!
//! This module extracts glyph outlines from fonts and rasterizes them
//! into grayscale images with proper antialiasing.

use crate::error::{Error, Result};
use crate::fonts::FontInstance;
use crate::shaping::ShapedText;
use read_fonts::TableProvider;
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::MetadataProvider;
use std::path::Path;
use zeno::{Command, Mask, Transform};

/// Fallback delta value reported when renders cannot be compared safely.
pub const PIXEL_DELTA_FALLBACK: f64 = 999_999.0;

/// Grayscale image wrapper used for validation and metrics.
#[derive(Clone, Debug)]
pub struct Image {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl Image {
    /// Create a new image, validating dimensions and buffer size.
    pub fn new(width: u32, height: u32, pixels: Vec<u8>) -> Result<Self> {
        if width == 0 || height == 0 {
            return Err(Error::InvalidRenderParams {
                reason: "Image dimensions must be non-zero".to_string(),
            });
        }
        let expected = (width as usize) * (height as usize);
        if pixels.len() != expected {
            return Err(Error::Internal(format!(
                "Pixel data size mismatch: expected {} bytes, got {}",
                expected,
                pixels.len()
            )));
        }
        Ok(Self {
            width,
            height,
            pixels,
        })
    }

    /// Access raw grayscale pixels.
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Consume the image and return the owned pixel buffer.
    pub fn into_pixels(self) -> Vec<u8> {
        self.pixels
    }

    /// Width in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Height in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Total number of pixels.
    fn len(&self) -> usize {
        self.pixels.len()
    }

    /// Return true when every pixel is zero (blank render).
    pub fn is_empty(&self) -> bool {
        self.pixels.iter().all(|&px| px == 0)
    }

    /// Calculate tight bounding box of non-zero pixels.
    pub fn calculate_bbox(&self) -> (u32, u32, u32, u32) {
        let mut min_x = self.width;
        let mut min_y = self.height;
        let mut max_x = 0u32;
        let mut max_y = 0u32;

        for y in 0..self.height {
            for x in 0..self.width {
                let idx = (y * self.width + x) as usize;
                if self.pixels[idx] > 0 {
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                }
            }
        }

        if min_x > max_x {
            return (0, 0, 0, 0);
        }

        (min_x, min_y, max_x - min_x + 1, max_y - min_y + 1)
    }

    /// Compute normalized pixel delta with hard clamps to avoid infinities.
    pub fn pixel_delta(&self, other: &Image) -> f64 {
        if self.width != other.width || self.height != other.height {
            return PIXEL_DELTA_FALLBACK;
        }
        if self.is_empty() || other.is_empty() {
            return PIXEL_DELTA_FALLBACK;
        }

        let mut diff: u64 = 0;
        for (lhs, rhs) in self.pixels.iter().zip(other.pixels.iter()) {
            diff += lhs.abs_diff(*rhs) as u64;
        }

        let denom = (self.len() as u64 * 255).max(1);
        let delta = diff as f64 / denom as f64;
        if !delta.is_finite() {
            return PIXEL_DELTA_FALLBACK;
        }
        delta.clamp(0.0, PIXEL_DELTA_FALLBACK)
    }
}

/// Glyph rasterizer using zeno.
pub struct GlyphRasterizer;

impl GlyphRasterizer {
    /// Create a new glyph rasterizer.
    pub fn new() -> Self {
        Self
    }

    /// Render shaped text to a grayscale image.
    ///
    /// Returns a grayscale image wrapper (0-255) in row-major order.
    pub fn render_text(
        &self,
        font_instance: &FontInstance,
        shaped: &ShapedText,
        width: u32,
        height: u32,
        tracking: f32,
        path: &Path,
    ) -> Result<Image> {
        // Create blank canvas
        let mut canvas = vec![0u8; (width * height) as usize];

        if shaped.glyphs.is_empty() {
            return Image::new(width, height, canvas);
        }

        let font = font_instance.font_ref();

        // TODO: Properly convert variation coordinates to normalized F2Dot14 values
        // For now, use default location (static font rendering only)
        if !font_instance.coordinates().is_empty() {
            log::warn!(
                "Variable font coordinates requested but not yet supported in rendering: {:?}. Using default coordinates.",
                font_instance.coordinates()
            );
        }
        let location_ref = LocationRef::default();

        // Calculate scale factor (font size to pixels)
        let head = font
            .head()
            .map_err(|e| Error::Internal(format!("Failed to read head table: {}", e)))?;
        let upem = head.units_per_em();
        let scale = shaped.font_size / upem as f32;

        // Position baseline at 75% height
        let baseline_y = height as f32 * 0.75;
        let mut cursor_x = 0.0f32;

        // Render each glyph
        for glyph in &shaped.glyphs {
            let glyph_id = glyph.glyph_id.into();

            // Extract outline
            let outline = font.outline_glyphs();
            let Some(glyph_outline) = outline.get(glyph_id) else {
                log::warn!("Glyph ID {} not found in font", glyph.glyph_id);
                cursor_x += (glyph.x_advance as f32 + tracking) * scale;
                continue;
            };

            // Build path
            let mut path_commands = Vec::new();
            let mut pen = ZenoPen::new(&mut path_commands);

            let draw_settings = DrawSettings::unhinted(Size::unscaled(), location_ref);
            if let Err(e) = glyph_outline.draw(draw_settings, &mut pen) {
                return Err(Error::RasterizationFailed {
                    glyph_id: glyph.glyph_id,
                    path: path.to_path_buf(),
                    reason: format!("Failed to draw outline: {}", e),
                });
            }

            // Calculate glyph position
            let glyph_x = cursor_x + (glyph.x_offset as f32 * scale);
            let glyph_y = baseline_y - (glyph.y_offset as f32 * scale);

            // Rasterize and composite
            self.composite_glyph(
                &mut canvas,
                &path_commands,
                glyph_x,
                glyph_y,
                scale,
                width,
                height,
            )?;

            // Advance cursor
            cursor_x += (glyph.x_advance as f32 + tracking) * scale;
        }

        Image::new(width, height, canvas)
    }

    /// Composite a single glyph onto the canvas.
    fn composite_glyph(
        &self,
        canvas: &mut [u8],
        path: &[Command],
        x: f32,
        y: f32,
        scale: f32,
        width: u32,
        height: u32,
    ) -> Result<()> {
        // Create transform (scale + translate)
        let transform = Transform::scale(scale, scale).then_translate(x, y);

        // Rasterize to temporary mask
        let mut mask = Mask::new(path);
        mask.size(width, height).transform(Some(transform));

        let (alpha_data, placement) = mask.render();

        // Alpha blend onto canvas
        let top = placement.top.max(0) as u32;
        let left = placement.left.max(0) as u32;
        let bottom = (placement.top + placement.height as i32).min(height as i32) as u32;
        let right = (placement.left + placement.width as i32).min(width as i32) as u32;

        for py in top..bottom {
            for px in left..right {
                let canvas_idx = (py * width + px) as usize;
                let mask_y = (py as i32 - placement.top) as u32;
                let mask_x = (px as i32 - placement.left) as u32;
                let mask_idx = (mask_y * placement.width + mask_x) as usize;

                if mask_idx < alpha_data.len() {
                    let alpha = alpha_data[mask_idx];
                    let src = canvas[canvas_idx];

                    // Blend: dst + src * (1 - dst_alpha/255)
                    let blended =
                        src.saturating_add(((alpha as u16 * (255 - src) as u16) / 255) as u8);
                    canvas[canvas_idx] = blended;
                }
            }
        }

        Ok(())
    }
}

impl Default for GlyphRasterizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Adapter to convert skrifa OutlinePen to zeno command vector.
struct ZenoPen<'a> {
    commands: &'a mut Vec<Command>,
}

impl<'a> ZenoPen<'a> {
    fn new(commands: &'a mut Vec<Command>) -> Self {
        Self { commands }
    }
}

impl<'a> OutlinePen for ZenoPen<'a> {
    fn move_to(&mut self, x: f32, y: f32) {
        self.commands.push(Command::MoveTo([x, -y].into())); // Flip Y for graphics coordinates
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.commands.push(Command::LineTo([x, -y].into()));
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.commands
            .push(Command::QuadTo([cx0, -cy0].into(), [x, -y].into()));
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.commands.push(Command::CurveTo(
            [cx0, -cy0].into(),
            [cx1, -cy1].into(),
            [x, -y].into(),
        ));
    }

    fn close(&mut self) {
        self.commands.push(Command::Close);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_rejects_invalid_dimensions() {
        let result = Image::new(0, 10, vec![]);
        assert!(result.is_err());

        let result = Image::new(10, 10, vec![0u8; 5]);
        assert!(result.is_err());
    }

    #[test]
    fn image_is_empty_detects_blank_canvas() {
        let img = Image::new(4, 4, vec![0u8; 16]).unwrap();
        assert!(img.is_empty());

        let mut pixels = vec![0u8; 16];
        pixels[3] = 1;
        let img = Image::new(4, 4, pixels).unwrap();
        assert!(!img.is_empty());
    }

    #[test]
    fn calculate_bbox_handles_basic_shapes() {
        let mut pixels = vec![0u8; 100 * 50];
        assert_eq!(
            Image::new(100, 50, pixels.clone())
                .unwrap()
                .calculate_bbox(),
            (0, 0, 0, 0)
        );

        pixels[25 * 100 + 50] = 255;
        assert_eq!(
            Image::new(100, 50, pixels.clone())
                .unwrap()
                .calculate_bbox(),
            (50, 25, 1, 1)
        );

        pixels.fill(0);
        for y in 10..15 {
            for x in 20..30 {
                pixels[y * 100 + x] = 255;
            }
        }
        assert_eq!(
            Image::new(100, 50, pixels).unwrap().calculate_bbox(),
            (20, 10, 10, 5)
        );
    }

    #[test]
    fn pixel_delta_clamps_on_invalid_inputs() {
        let img = Image::new(4, 4, vec![0u8; 16]).unwrap();
        assert_eq!(img.pixel_delta(&img), PIXEL_DELTA_FALLBACK);

        let other = Image::new(2, 8, vec![0u8; 16]).unwrap();
        assert_eq!(img.pixel_delta(&other), PIXEL_DELTA_FALLBACK);
    }

    #[test]
    fn pixel_delta_returns_normalized_difference() {
        let left = Image::new(2, 2, vec![0, 0, 0, 255]).unwrap();
        let right = Image::new(2, 2, vec![255, 0, 0, 255]).unwrap();
        let delta = left.pixel_delta(&right);
        assert!(delta > 0.0);
        assert!(delta < 1.0);
    }
}
