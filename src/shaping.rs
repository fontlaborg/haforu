// this_file: src/shaping.rs
//! Text shaping module using HarfRust

use crate::error::{Error, Result};
use crate::json_parser::{GlyphInfo, ShapingOptions, ShapingOutput};
use harfrust::{
    BufferClusterLevel, Direction, Feature, FontRef as HarfRustFontRef, Script,
    ShaperData, ShaperInstance, Tag, UnicodeBuffer, Variation,
};
use log::{debug, info};
use read_fonts::{FontRef, TableProvider};

/// Text shaper using HarfRust
pub struct TextShaper {
    /// Cached font data for reuse
    cached_font_data: Option<(Vec<u8>, ShaperData)>,
}

impl TextShaper {
    /// Create a new text shaper
    pub fn new() -> Self {
        Self {
            cached_font_data: None,
        }
    }

    /// Shape text using the provided font and options
    pub fn shape(
        &mut self,
        font_data: &[u8],
        text: &str,
        size: f32,
        options: &ShapingOptions,
    ) -> Result<ShapingOutput> {
        debug!("Shaping text: '{}' with size {}", text, size);

        // Create HarfRust FontRef from font data
        let font = HarfRustFontRef::from_index(font_data, 0)
            .map_err(|e| Error::Font(format!("Failed to create HarfRust font: {:?}", e)))?;

        // Create or reuse ShaperData
        let shaper_data = if let Some((ref cached_data, ref cached_shaper)) = self.cached_font_data {
            if cached_data == font_data {
                cached_shaper
            } else {
                self.cached_font_data = Some((font_data.to_vec(), ShaperData::new(&font)));
                &self.cached_font_data.as_ref().unwrap().1
            }
        } else {
            self.cached_font_data = Some((font_data.to_vec(), ShaperData::new(&font)));
            &self.cached_font_data.as_ref().unwrap().1
        };

        // Create shaper instance with variations if needed
        // Convert HashMap to Vec<String> for feature parsing
        let features_vec: Vec<String> = options.features.iter()
            .filter_map(|(k, v)| if *v { Some(k.clone()) } else { None })
            .collect();
        let variations = parse_variations(&features_vec)?;
        let instance = if !variations.is_empty() {
            Some(ShaperInstance::from_variations(&font, &variations))
        } else {
            None
        };

        // Build the shaper
        let mut builder = shaper_data.shaper(&font);
        if let Some(ref inst) = instance {
            builder = builder.instance(Some(inst));
        }

        // Set point size if needed (for tracking table)
        if size > 0.0 {
            builder = builder.point_size(Some(size));
        }

        let shaper = builder.build();

        // Create and configure buffer
        let mut buffer = UnicodeBuffer::new();
        buffer.push_str(text);

        // Set buffer properties
        let direction = parse_harfrust_direction(&options.direction)?;
        buffer.set_direction(direction);

        if let Some(ref script_str) = options.script {
            let script = parse_harfrust_script(script_str)?;
            buffer.set_script(script);
        } else {
            // Let HarfRust guess the script
            buffer.guess_segment_properties();
        }

        if let Some(ref _lang) = options.language {
            // Language uses from_iso639_str or similar
            // For now, let's use the default language
            // TODO: Fix Language construction when API is clear
            // buffer.set_language(Language::from_iso639_str(lang));
        }

        // Set cluster level
        match options.cluster_level {
            1 => buffer.set_cluster_level(BufferClusterLevel::Characters),
            2 => buffer.set_cluster_level(BufferClusterLevel::MonotoneGraphemes),
            _ => buffer.set_cluster_level(BufferClusterLevel::MonotoneCharacters),
        }

        // Parse features for shaping
        let features = parse_features(&features_vec)?;

        // Shape the buffer
        let glyph_buffer = shaper.shape(buffer, &features);

        // Extract shaped glyphs
        let glyph_infos = glyph_buffer.glyph_infos();
        let glyph_positions = glyph_buffer.glyph_positions();

        // Note: HarfRust shapes at UnitsPerEm scale, so we need to scale the results
        let upem = font.head().map(|h| h.units_per_em()).unwrap_or(1000) as f32;
        let scale = size / upem;

        let glyphs = glyph_infos
            .iter()
            .zip(glyph_positions.iter())
            .map(|(info, pos)| {
                GlyphInfo {
                    glyph_id: info.glyph_id,
                    cluster: info.cluster,
                    // Scale from UPEM to requested size
                    x_advance: (pos.x_advance as f32 * scale) as i32,
                    y_advance: (pos.y_advance as f32 * scale) as i32,
                    x_offset: (pos.x_offset as f32 * scale) as i32,
                    y_offset: (pos.y_offset as f32 * scale) as i32,
                }
            })
            .collect();

        let output = ShapingOutput {
            glyphs,
            direction: options.direction.clone(),
            script: options.script.clone().unwrap_or_else(|| "auto".to_string()),
            language: options.language.clone().unwrap_or_else(|| "en".to_string()),
        };

        info!("Shaped {} glyphs using HarfRust", output.glyphs.len());
        Ok(output)
    }

    /// Shape text using FontRef (compatibility method)
    pub fn shape_with_fontref(
        &mut self,
        _font_ref: &FontRef,
        font_data: &[u8],
        text: &str,
        size: f32,
        options: &ShapingOptions,
    ) -> Result<ShapingOutput> {
        // For now, we need the raw font data for HarfRust
        // In the future, we might integrate more directly
        self.shape(font_data, text, size, options)
    }
}

impl Default for TextShaper {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse direction string to HarfRust Direction
pub fn parse_harfrust_direction(dir: &str) -> Result<Direction> {
    match dir.to_lowercase().as_str() {
        "ltr" => Ok(Direction::LeftToRight),
        "rtl" => Ok(Direction::RightToLeft),
        "ttb" => Ok(Direction::TopToBottom),
        "btt" => Ok(Direction::BottomToTop),
        _ => Err(Error::InvalidParameter(format!(
            "Invalid direction: {}",
            dir
        ))),
    }
}

/// Parse script string to HarfRust Script
pub fn parse_harfrust_script(script_str: &str) -> Result<Script> {
    // Convert script string to 4-byte tag
    let bytes = script_str.as_bytes();
    if bytes.len() != 4 {
        return Err(Error::InvalidParameter(format!(
            "Script tag must be 4 characters: {}",
            script_str
        )));
    }

    // Create Script from tag - convert to fixed array
    let mut tag_bytes = [0u8; 4];
    tag_bytes.copy_from_slice(bytes);
    let tag = Tag::from_be_bytes(tag_bytes);
    // Script has a from_iso15924_tag constructor
    Script::from_iso15924_tag(tag)
        .ok_or_else(|| Error::InvalidParameter(format!("Invalid script tag: {}", script_str)))
}

/// Parse variation strings to HarfRust variations
pub fn parse_variations(features: &[String]) -> Result<Vec<Variation>> {
    let mut variations = Vec::new();

    for feature_str in features {
        // Look for variation syntax like "wght=500" or "wdth=75"
        if let Some(pos) = feature_str.find('=') {
            let tag_str = &feature_str[..pos];
            if tag_str.len() == 4 {
                // This might be a variation axis
                let value_str = &feature_str[pos + 1..];
                if let Ok(value) = value_str.parse::<f32>() {
                    let mut tag_bytes = [0u8; 4];
                    tag_bytes.copy_from_slice(tag_str.as_bytes());
                    let tag = Tag::from_be_bytes(tag_bytes);
                    variations.push(Variation { tag, value });
                }
            }
        }
    }

    Ok(variations)
}

/// Parse feature strings to HarfRust features
pub fn parse_features(features: &[String]) -> Result<Vec<Feature>> {
    let mut parsed_features = Vec::new();

    for feature_str in features {
        // Skip variation axes (handled separately)
        if feature_str.len() == 4 && feature_str.contains('=') {
            if let Some(pos) = feature_str.find('=') {
                let value_str = &feature_str[pos + 1..];
                if value_str.parse::<f32>().is_ok() {
                    continue; // This is a variation, not a feature
                }
            }
        }

        // Parse feature string format: "kern", "+kern", "-kern", "kern=1", etc.
        let (tag_str, value) = if let Some(pos) = feature_str.find('=') {
            let (tag, val) = feature_str.split_at(pos);
            let val = val[1..].parse::<u32>().unwrap_or(1);
            (tag.trim_start_matches('+').trim_start_matches('-'), val)
        } else if feature_str.starts_with('-') {
            (&feature_str[1..], 0)
        } else if feature_str.starts_with('+') {
            (&feature_str[1..], 1)
        } else {
            (feature_str.as_str(), 1)
        };

        // Ensure tag is 4 characters (pad with spaces if needed)
        let mut tag_bytes = [b' '; 4];
        let bytes = tag_str.as_bytes();
        let len = bytes.len().min(4);
        tag_bytes[..len].copy_from_slice(&bytes[..len]);

        let tag = Tag::from_be_bytes(tag_bytes);
        let feature = Feature {
            tag,
            value,
            start: 0,
            end: u32::MAX, // Global feature
        };

        parsed_features.push(feature);
    }

    Ok(parsed_features)
}

/// Parse direction string (compatibility function)
pub fn parse_direction(dir: &str) -> Result<String> {
    match dir.to_lowercase().as_str() {
        "ltr" | "rtl" | "ttb" | "btt" => Ok(dir.to_string()),
        _ => Err(Error::InvalidParameter(format!(
            "Invalid direction: {}",
            dir
        ))),
    }
}

/// Parse script string (compatibility function)
pub fn parse_script(script_str: &str) -> Result<String> {
    // Convert script string to 4-byte tag
    let bytes = script_str.as_bytes();
    if bytes.len() != 4 {
        return Err(Error::InvalidParameter(format!(
            "Script tag must be 4 characters: {}",
            script_str
        )));
    }
    Ok(script_str.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json_parser::ShapingOptions;

    #[test]
    fn test_text_shaper_creation() {
        let _shaper = TextShaper::new();
        // Just ensure it creates without panic
        assert!(true);
    }

    #[test]
    fn test_parse_direction() {
        assert_eq!(parse_direction("ltr").unwrap(), "ltr");
        assert_eq!(parse_direction("rtl").unwrap(), "rtl");
        assert_eq!(parse_direction("ttb").unwrap(), "ttb");
        assert_eq!(parse_direction("btt").unwrap(), "btt");
        assert!(parse_direction("invalid").is_err());
    }

    #[test]
    fn test_parse_script() {
        // Valid 4-character script tags
        assert!(parse_script("Arab").is_ok());
        assert!(parse_script("Latn").is_ok());
        assert!(parse_script("Deva").is_ok());

        // Invalid lengths
        assert!(parse_script("Ar").is_err());
        assert!(parse_script("Arabic").is_err());
        assert!(parse_script("").is_err());
    }

    #[test]
    fn test_shaping_options_default() {
        let options = ShapingOptions::default();
        assert_eq!(options.direction, "ltr");
        assert!(options.language.is_none());
        assert!(options.script.is_none());
        assert_eq!(options.cluster_level, 0);
        assert!(options.features.is_empty());
    }

    #[test]
    fn test_placeholder_shaping() {
        let _shaper = TextShaper::new();
        let _options = ShapingOptions::default();

        // Note: We can't test with real FontRef without a real font file
        // This would be expanded in integration tests

        // For now, just verify the shaper doesn't panic
        assert!(true);
    }
}
