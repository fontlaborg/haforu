// this_file: src/shaping.rs
//! Text shaping module using HarfRust

use crate::error::{Error, Result};
use crate::json_parser::{GlyphInfo, ShapingOptions, ShapingOutput};
use log::{debug, info};
use read_fonts::FontRef;

/// Text shaper using HarfRust
/// Note: This is a simplified implementation. Full HarfRust integration
/// requires more complex API usage that varies by version.
pub struct TextShaper {
    // We'll expand this as we integrate HarfRust properly
}

impl TextShaper {
    /// Create a new text shaper
    pub fn new() -> Self {
        Self {}
    }

    /// Shape text using the provided font and options
    /// This is a placeholder implementation that will be expanded
    pub fn shape(
        &mut self,
        _font_ref: &FontRef,
        text: &str,
        size: f32,
        options: &ShapingOptions,
    ) -> Result<ShapingOutput> {
        debug!("Shaping text: '{}' with size {}", text, size);

        // For now, create a simple placeholder output
        // In a full implementation, this would use HarfRust's shaping engine
        let glyphs = text
            .chars()
            .enumerate()
            .map(|(i, _ch)| {
                GlyphInfo {
                    glyph_id: i as u32, // Placeholder glyph ID
                    cluster: i as u32,
                    x_advance: (size * 0.6) as i32, // Approximate advance
                    y_advance: 0,
                    x_offset: 0,
                    y_offset: 0,
                }
            })
            .collect();

        let output = ShapingOutput {
            glyphs,
            direction: options.direction.clone(),
            script: options.script.clone().unwrap_or_else(|| "auto".to_string()),
            language: options.language.clone().unwrap_or_else(|| "en".to_string()),
        };

        info!(
            "Created placeholder shaping output with {} glyphs",
            output.glyphs.len()
        );
        Ok(output)
    }
}

impl Default for TextShaper {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse direction string (placeholder for proper HarfRust Direction)
pub fn parse_direction(dir: &str) -> Result<String> {
    match dir.to_lowercase().as_str() {
        "ltr" | "rtl" | "ttb" | "btt" => Ok(dir.to_string()),
        _ => Err(Error::InvalidParameter(format!(
            "Invalid direction: {}",
            dir
        ))),
    }
}

/// Parse script string (placeholder for proper HarfRust Script)
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
