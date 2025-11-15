// this_file: src/shaping.rs

//! Text shaping using HarfBuzz.
//!
//! This module shapes text into positioned glyphs, handling complex scripts,
//! ligatures, kerning, and other OpenType features.

use crate::batch::TextConfig;
use crate::error::{Error, Result};
use crate::fonts::FontInstance;
use harfbuzz_rs::{
    Direction, Face, Feature, Font as HbFont, GlyphBuffer, Language, Tag, UnicodeBuffer,
};
use read_fonts::TableProvider;
use std::path::Path;
use std::str::FromStr;

/// Shaped text with positioned glyphs.
#[derive(Debug, Clone)]
pub struct ShapedText {
    /// Positioned glyphs
    pub glyphs: Vec<ShapedGlyph>,
    /// Font size in points
    pub font_size: f32,
}

/// Single shaped glyph with position.
#[derive(Debug, Clone)]
pub struct ShapedGlyph {
    /// Glyph ID in the font
    pub glyph_id: u32,
    /// Horizontal advance (in font units)
    pub x_advance: i32,
    /// Vertical advance (in font units, typically 0)
    pub y_advance: i32,
    /// Horizontal offset from cursor (in font units)
    pub x_offset: i32,
    /// Vertical offset from baseline (in font units)
    pub y_offset: i32,
}

/// Input parameters for shaping text.
pub struct ShapeRequest<'a> {
    /// Literal text to shape.
    pub text: &'a str,
    /// Script hint (e.g., Latn, Arab).
    pub script: Option<&'a str>,
    /// Direction hint (ltr, rtl, ttb, btt).
    pub direction: Option<&'a str>,
    /// Language hint (BCP-47 tag).
    pub language: Option<&'a str>,
    /// OpenType features to force on/off.
    pub features: &'a [String],
}

impl<'a> From<&'a TextConfig> for ShapeRequest<'a> {
    fn from(config: &'a TextConfig) -> Self {
        Self {
            text: &config.content,
            script: config.script.as_deref(),
            direction: config.direction.as_deref(),
            language: config.language.as_deref(),
            features: &config.features,
        }
    }
}

/// Text shaper using HarfBuzz.
pub struct TextShaper;

impl TextShaper {
    /// Create a new text shaper.
    pub fn new() -> Self {
        Self
    }

    /// Shape text using the provided font instance.
    ///
    /// Returns positioned glyphs with advances and offsets.
    pub fn shape(
        &self,
        font_instance: &FontInstance,
        text: &str,
        font_size: f32,
        path: &Path,
    ) -> Result<ShapedText> {
        let empty: [String; 0] = [];
        let request = ShapeRequest {
            text,
            script: None,
            direction: None,
            language: None,
            features: &empty,
        };
        self.shape_with_request(font_instance, &request, font_size, path)
    }

    /// Shape text using the provided request (script/direction/language/features aware).
    pub fn shape_with_request(
        &self,
        font_instance: &FontInstance,
        request: &ShapeRequest<'_>,
        font_size: f32,
        path: &Path,
    ) -> Result<ShapedText> {
        if request.text.is_empty() {
            return Ok(ShapedText {
                glyphs: vec![],
                font_size,
            });
        }

        let advanced = request.script.is_some()
            || request.direction.is_some()
            || request.language.is_some()
            || request
                .features
                .iter()
                .any(|entry| !entry.trim().is_empty())
            || !font_instance.coordinates().is_empty();

        if !advanced && request.text.chars().count() == 1 {
            return self.shape_single_char(font_instance, request.text, font_size, path);
        }

        // Full shaping path
        self.shape_harfbuzz(font_instance, request, font_size, path)
    }

    /// Fast path: shape single character without HarfBuzz overhead.
    fn shape_single_char(
        &self,
        font_instance: &FontInstance,
        text: &str,
        font_size: f32,
        _path: &Path,
    ) -> Result<ShapedText> {
        let ch = text.chars().next().unwrap();
        let font = font_instance.font_ref();

        // Map character to glyph ID
        let cmap = font
            .cmap()
            .map_err(|e| Error::Internal(format!("Failed to read cmap table: {}", e)))?;
        let glyph_id = cmap
            .map_codepoint(ch as u32)
            .ok_or_else(|| Error::Internal(format!("Character '{}' not found in font", ch)))?
            .to_u32();

        // Get advance width from hmtx table
        // TODO: Use instance coordinates for variable fonts
        if !font_instance.coordinates().is_empty() {
            log::warn!(
                "Single-character fast path does not support variable font coordinates yet: {:?}. Using static metrics.",
                font_instance.coordinates()
            );
        }
        let hmtx = font
            .hmtx()
            .map_err(|e| Error::Internal(format!("Failed to read hmtx table: {}", e)))?;
        let advance = hmtx.advance(glyph_id.into()).unwrap_or(0) as i32;

        Ok(ShapedText {
            glyphs: vec![ShapedGlyph {
                glyph_id,
                x_advance: advance,
                y_advance: 0,
                x_offset: 0,
                y_offset: 0,
            }],
            font_size,
        })
    }

    /// Full shaping using HarfBuzz.
    fn shape_harfbuzz(
        &self,
        font_instance: &FontInstance,
        request: &ShapeRequest<'_>,
        font_size: f32,
        path: &Path,
    ) -> Result<ShapedText> {
        // Get the raw font data from the FontInstance
        let font_data = font_instance.font_data();

        // Create HarfBuzz face from font data
        let face = Face::from_bytes(font_data, 0);
        let mut hb_font = HbFont::new(face);

        // Set font size (convert points to pixels, assuming 72 DPI)
        let ppem = font_size as u32;
        hb_font.set_ppem(ppem, ppem);

        // Apply variations if present
        if !font_instance.coordinates().is_empty() {
            let variations: Vec<harfbuzz_rs::Variation> = font_instance
                .coordinates()
                .iter()
                .filter_map(|(tag, value)| {
                    // Parse tag string (e.g. "wght") into 4 chars
                    let chars: Vec<char> = tag.chars().collect();
                    if chars.len() == 4 {
                        Some(harfbuzz_rs::Variation::new(
                            harfbuzz_rs::Tag::new(chars[0], chars[1], chars[2], chars[3]),
                            *value,
                        ))
                    } else {
                        None
                    }
                })
                .collect();

            hb_font.set_variations(&variations);
        }

        // Create buffer and add text (chain methods since they take ownership)
        let mut buffer = UnicodeBuffer::new().add_str(request.text);
        if let Some(dir) = request.direction.and_then(parse_direction) {
            buffer = buffer.set_direction(dir);
        }
        if let Some(script) = request.script.and_then(parse_script) {
            buffer = buffer.set_script(script);
        }
        if let Some(language) = request
            .language
            .and_then(|lang| Language::from_str(lang).ok())
        {
            buffer = buffer.set_language(language);
        }
        if request.script.is_none() || request.direction.is_none() || request.language.is_none() {
            buffer = buffer.guess_segment_properties();
        }

        let features = build_features(request.features);

        // Shape
        let glyph_buffer: GlyphBuffer = harfbuzz_rs::shape(&hb_font, buffer, &features);

        // Extract glyph positions
        let glyph_infos = glyph_buffer.get_glyph_infos();
        let glyph_positions = glyph_buffer.get_glyph_positions();

        if glyph_infos.is_empty() {
            return Err(Error::ShapingFailed {
                text: request.text.to_string(),
                path: path.to_path_buf(),
                reason: "HarfBuzz returned zero glyphs".to_string(),
            });
        }

        let glyphs = glyph_infos
            .iter()
            .zip(glyph_positions.iter())
            .map(|(info, pos)| ShapedGlyph {
                glyph_id: info.codepoint,
                x_advance: pos.x_advance,
                y_advance: pos.y_advance,
                x_offset: pos.x_offset,
                y_offset: pos.y_offset,
            })
            .collect();

        Ok(ShapedText { glyphs, font_size })
    }
}

impl ShapedText {
    /// Calculate total advance width in font units.
    pub fn total_advance_width(&self) -> i32 {
        self.glyphs.iter().map(|g| g.x_advance).sum()
    }

    /// Calculate bounding box of all glyphs (in font units).
    pub fn bounding_box(&self) -> (i32, i32, i32, i32) {
        if self.glyphs.is_empty() {
            return (0, 0, 0, 0);
        }

        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;

        let mut cursor_x = 0i32;
        for glyph in &self.glyphs {
            let glyph_x = cursor_x + glyph.x_offset;
            let glyph_y = glyph.y_offset;

            min_x = min_x.min(glyph_x);
            min_y = min_y.min(glyph_y);
            max_x = max_x.max(glyph_x + glyph.x_advance);
            max_y = max_y.max(glyph_y + glyph.y_advance);

            cursor_x += glyph.x_advance;
        }

        (min_x, min_y, max_x - min_x, max_y - min_y)
    }
}

fn parse_direction(label: &str) -> Option<Direction> {
    match label.to_ascii_lowercase().as_str() {
        "ltr" => Some(Direction::Ltr),
        "rtl" => Some(Direction::Rtl),
        "ttb" => Some(Direction::Ttb),
        "btt" => Some(Direction::Btt),
        _ => {
            log::warn!(
                "Ignoring unsupported direction '{}'; falling back to HarfBuzz heuristics",
                label
            );
            None
        }
    }
}

fn parse_script(label: &str) -> Option<Tag> {
    let trimmed = label.trim();
    if trimmed.is_empty() {
        return None;
    }
    match Tag::from_str(trimmed) {
        Ok(tag) => Some(tag),
        Err(_) => {
            log::warn!(
                "Ignoring invalid script tag '{}'; expected ISO15924 (four letters)",
                label
            );
            None
        }
    }
}

fn build_features(features: &[String]) -> Vec<Feature> {
    features
        .iter()
        .filter_map(|entry| {
            let trimmed = entry.trim();
            if trimmed.is_empty() {
                return None;
            }
            let (tag_part, value_part) = trimmed
                .split_once('=')
                .map(|(tag, value)| (tag.trim(), value.trim()))
                .unwrap_or((trimmed, "1"));
            let value = value_part.parse::<u32>().unwrap_or(1);
            let mut chars: Vec<char> = tag_part.chars().collect();
            if chars.len() > 4 {
                chars.truncate(4);
            }
            while chars.len() < 4 {
                chars.push(' ');
            }
            Some(Feature::new(
                Tag::new(chars[0], chars[1], chars[2], chars[3]),
                value,
                ..,
            ))
        })
        .collect()
}

impl Default for TextShaper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shaped_text_empty() {
        let shaped = ShapedText {
            glyphs: vec![],
            font_size: 100.0,
        };
        assert_eq!(shaped.total_advance_width(), 0);
        assert_eq!(shaped.bounding_box(), (0, 0, 0, 0));
    }

    #[test]
    fn test_shaped_text_single_glyph() {
        let shaped = ShapedText {
            glyphs: vec![ShapedGlyph {
                glyph_id: 1,
                x_advance: 500,
                y_advance: 0,
                x_offset: 0,
                y_offset: 0,
            }],
            font_size: 100.0,
        };
        assert_eq!(shaped.total_advance_width(), 500);
    }
}
