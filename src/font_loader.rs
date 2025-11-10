// this_file: src/font_loader.rs
//! Font loading and management module

use crate::error::{Error, Result};
use log::{debug, info, warn};
use read_fonts::FontRef;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Font loader with caching support
pub struct FontLoader {
    /// Cache of loaded fonts
    cache: HashMap<PathBuf, Arc<Vec<u8>>>,
    /// Maximum cache size in bytes
    max_cache_size: usize,
    /// Current cache size in bytes
    current_cache_size: usize,
}

impl FontLoader {
    /// Create a new font loader with default cache size (100MB)
    pub fn new() -> Self {
        Self::with_cache_size(100 * 1024 * 1024)
    }

    /// Create a new font loader with specified cache size
    pub fn with_cache_size(max_cache_size: usize) -> Self {
        Self {
            cache: HashMap::new(),
            max_cache_size,
            current_cache_size: 0,
        }
    }

    /// Load font data from file path
    pub fn load_font_data<P: AsRef<Path>>(&mut self, path: P) -> Result<Arc<Vec<u8>>> {
        let path = path.as_ref();
        let canonical_path = path.canonicalize().map_err(|e| {
            Error::Font(format!("Failed to resolve path {}: {}", path.display(), e))
        })?;

        // Check if font is already cached
        if let Some(data) = self.cache.get(&canonical_path) {
            debug!("Using cached font: {}", canonical_path.display());
            return Ok(data.clone());
        }

        // Load font file
        info!("Loading font: {}", canonical_path.display());
        let data = fs::read(&canonical_path).map_err(|e| {
            Error::Font(format!(
                "Failed to read font file {}: {}",
                canonical_path.display(),
                e
            ))
        })?;

        // Validate file size
        if data.is_empty() {
            return Err(Error::Font("Font file is empty".into()));
        }

        // Basic validation: check for TTF/OTF signatures
        if !Self::is_valid_font_signature(&data) {
            return Err(Error::Font(
                "Invalid font file format (expected TTF/OTF/TTC)".into(),
            ));
        }

        let data = Arc::new(data);

        // Validate that it can be parsed
        self.validate_font(&data)?;

        // Add to cache if there's room
        if self.should_cache(data.len()) {
            self.add_to_cache(canonical_path.clone(), data.clone());
        }

        Ok(data)
    }

    /// Parse and validate font data
    fn validate_font(&self, data: &[u8]) -> Result<()> {
        FontRef::new(data).map_err(|e| Error::Font(format!("Failed to parse font: {}", e)))?;
        Ok(())
    }

    /// Create a FontRef from cached data with proper lifetime
    pub fn create_font_ref<'a>(&self, data: &'a [u8]) -> Result<FontRef<'a>> {
        FontRef::new(data)
            .map_err(|e| Error::Font(format!("Failed to create font reference: {}", e)))
    }

    /// Check if data has valid font signature
    fn is_valid_font_signature(data: &[u8]) -> bool {
        if data.len() < 4 {
            return false;
        }

        let signature = &data[0..4];
        matches!(
            signature,
            // TrueType
            b"\x00\x01\x00\x00" |
            // OpenType
            b"OTTO" |
            // TrueType Collection
            b"ttcf" |
            // Web Open Font Format
            b"wOFF" |
            b"wOF2"
        )
    }

    /// Check if we should cache this font
    fn should_cache(&self, size: usize) -> bool {
        // Don't cache if it would exceed our limit
        if self.current_cache_size + size > self.max_cache_size {
            // Could implement LRU eviction here
            warn!("Font cache full, not caching");
            return false;
        }
        true
    }

    /// Add font to cache
    fn add_to_cache(&mut self, path: PathBuf, data: Arc<Vec<u8>>) {
        let size = data.len();
        self.cache.insert(path, data);
        self.current_cache_size += size;
        debug!(
            "Cached font, total cache size: {} bytes",
            self.current_cache_size
        );
    }

    /// Clear the font cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
        self.current_cache_size = 0;
        info!("Font cache cleared");
    }

    /// Get current cache size
    pub fn cache_size(&self) -> usize {
        self.current_cache_size
    }

    /// Get number of cached fonts
    pub fn cache_count(&self) -> usize {
        self.cache.len()
    }
}

impl Default for FontLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_font_loader_creation() {
        let loader = FontLoader::new();
        assert_eq!(loader.cache_count(), 0);
        assert_eq!(loader.cache_size(), 0);
    }

    #[test]
    fn test_invalid_font_signatures() {
        assert!(!FontLoader::is_valid_font_signature(b""));
        assert!(!FontLoader::is_valid_font_signature(b"ABC"));
        assert!(!FontLoader::is_valid_font_signature(b"INVALID"));
    }

    #[test]
    fn test_valid_font_signatures() {
        assert!(FontLoader::is_valid_font_signature(
            b"\x00\x01\x00\x00extra"
        ));
        assert!(FontLoader::is_valid_font_signature(b"OTTOextra"));
        assert!(FontLoader::is_valid_font_signature(b"ttcfextra"));
    }

    #[test]
    fn test_load_nonexistent_file() {
        let mut loader = FontLoader::new();
        let result = loader.load_font_data("nonexistent.ttf");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::Font(_)));
    }

    #[test]
    fn test_load_empty_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("empty.ttf");
        File::create(&file_path).unwrap();

        let mut loader = FontLoader::new();
        let result = loader.load_font_data(file_path);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::Font(_)));
    }

    #[test]
    fn test_load_invalid_font_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("invalid.ttf");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"This is not a font file").unwrap();

        let mut loader = FontLoader::new();
        let result = loader.load_font_data(file_path);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::Font(_)));
    }

    #[test]
    fn test_cache_operations() {
        let mut loader = FontLoader::with_cache_size(1024);
        assert_eq!(loader.cache_count(), 0);

        loader.clear_cache();
        assert_eq!(loader.cache_count(), 0);
        assert_eq!(loader.cache_size(), 0);
    }
}
