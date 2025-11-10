// this_file: src/mmap_font.rs
//! Memory-mapped font loading module using fontgrep patterns for zero-copy efficiency

use crate::error::{Error, Result};
use log::{debug, info};
use memmap2::Mmap;
use read_fonts::{FontRef, TableProvider};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// File information for memory-mapped font data
#[derive(Debug)]
pub struct FileInfo {
    /// Path to the font file
    pub path: PathBuf,
    /// Memory-mapped data
    pub mmap: Arc<Mmap>,
    /// File size in bytes
    pub size: usize,
    /// Font type detected from signature
    pub font_type: FontType,
    /// Number of fonts in the file (for collections)
    pub font_count: u32,
}

/// Font file type based on signature
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FontType {
    TrueType,
    OpenType,
    TrueTypeCollection,
    OpenTypeCollection,
    WOFF,
    WOFF2,
}

impl FontType {
    /// Detect font type from file data
    pub fn from_data(data: &[u8]) -> Option<Self> {
        if data.len() < 4 {
            return None;
        }

        match &data[0..4] {
            b"\x00\x01\x00\x00" => Some(FontType::TrueType),
            b"OTTO" => Some(FontType::OpenType),
            b"ttcf" => Some(FontType::TrueTypeCollection),
            b"wOFF" => Some(FontType::WOFF),
            b"wOF2" => Some(FontType::WOFF2),
            _ => None,
        }
    }

    /// Check if this is a collection format
    pub fn is_collection(&self) -> bool {
        matches!(self, FontType::TrueTypeCollection | FontType::OpenTypeCollection)
    }
}

impl FileInfo {
    /// Create FileInfo from a path with memory mapping
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let canonical_path = path.canonicalize()
            .map_err(|e| Error::Font(format!("Failed to resolve path {}: {}", path.display(), e)))?;

        debug!("Memory-mapping font file: {}", canonical_path.display());

        // Open file for memory mapping
        let file = File::open(&canonical_path)
            .map_err(|e| Error::Font(format!("Failed to open font file {}: {}", canonical_path.display(), e)))?;

        let metadata = file.metadata()
            .map_err(|e| Error::Font(format!("Failed to get file metadata: {}", e)))?;

        let size = metadata.len() as usize;

        if size == 0 {
            return Err(Error::Font("Font file is empty".into()));
        }

        // Create memory map - safe because we're only reading
        let mmap = unsafe { Mmap::map(&file) }
            .map_err(|e| Error::Font(format!("Failed to memory-map file: {}", e)))?;

        // Detect font type
        let font_type = FontType::from_data(&mmap)
            .ok_or_else(|| Error::Font("Unknown or invalid font file format".into()))?;

        // Count fonts in the file
        let font_count = if font_type.is_collection() {
            Self::count_collection_fonts(&mmap)?
        } else {
            1
        };

        info!("Mapped {} font file: {} ({} font(s), {} bytes)",
            match font_type {
                FontType::TrueType => "TrueType",
                FontType::OpenType => "OpenType",
                FontType::TrueTypeCollection => "TTC",
                FontType::OpenTypeCollection => "OTC",
                FontType::WOFF => "WOFF",
                FontType::WOFF2 => "WOFF2",
            },
            canonical_path.display(), font_count, size);

        Ok(FileInfo {
            path: canonical_path,
            mmap: Arc::new(mmap),
            size,
            font_type,
            font_count,
        })
    }

    /// Count the number of fonts in a TTC/OTC collection
    fn count_collection_fonts(data: &[u8]) -> Result<u32> {
        if data.len() < 12 {
            return Err(Error::Font("TTC file too small".into()));
        }

        // TTC header: 4-byte signature, 4-byte version, 4-byte numFonts
        let num_fonts = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);

        if num_fonts == 0 || num_fonts > 1024 {
            return Err(Error::Font(format!("Invalid font count in collection: {}", num_fonts)));
        }

        Ok(num_fonts)
    }

    /// Get FontRef for a specific font index (0 for single fonts, 0..n for collections)
    pub fn get_font(&self, index: u32) -> Result<FontRef<'_>> {
        if self.font_type.is_collection() {
            if index >= self.font_count {
                return Err(Error::Font(format!(
                    "Font index {} out of range (collection has {} fonts)",
                    index, self.font_count
                )));
            }

            // For TTC, we need to get the offset table for the specific font
            self.get_collection_font(index)
        } else {
            if index != 0 {
                return Err(Error::Font(format!(
                    "Font index {} invalid for non-collection font",
                    index
                )));
            }

            FontRef::new(&self.mmap)
                .map_err(|e| Error::Font(format!("Failed to parse font: {}", e)))
        }
    }

    /// Get a specific font from a TTC/OTC collection
    fn get_collection_font(&self, index: u32) -> Result<FontRef<'_>> {
        // TTC format:
        // - 4 bytes: 'ttcf' signature
        // - 4 bytes: version
        // - 4 bytes: numFonts
        // - numFonts * 4 bytes: offset table offsets

        let data = &*self.mmap;

        if data.len() < 16 {
            return Err(Error::Font("TTC file too small".into()));
        }

        let offset_start = 12 + (index as usize) * 4;
        if data.len() < offset_start + 4 {
            return Err(Error::Font("TTC offset table truncated".into()));
        }

        let offset = u32::from_be_bytes([
            data[offset_start],
            data[offset_start + 1],
            data[offset_start + 2],
            data[offset_start + 3],
        ]) as usize;

        if offset >= data.len() {
            return Err(Error::Font(format!("Invalid TTC font offset: {}", offset)));
        }

        // Create FontRef starting at the offset
        FontRef::new(&data[offset..])
            .map_err(|e| Error::Font(format!("Failed to parse font at index {}: {}", index, e)))
    }

    /// Get all fonts from the file
    pub fn get_all_fonts(&self) -> Result<Vec<FontRef<'_>>> {
        let mut fonts = Vec::with_capacity(self.font_count as usize);

        for i in 0..self.font_count {
            fonts.push(self.get_font(i)?);
        }

        Ok(fonts)
    }

    /// Get font metadata without fully parsing
    pub fn get_metadata(&self, index: u32) -> Result<FontMetadata> {
        let font = self.get_font(index)?;

        // Extract basic metadata using TableProvider
        let _name_table = font.name()
            .map_err(|e| Error::Font(format!("Failed to read name table: {}", e)))?;

        // Get family name from name table - use simple approach for now
        let family_name = "Unknown".to_string();

        Ok(FontMetadata {
            index,
            family_name,
            is_variable: font.fvar().is_ok(),
        })
    }
}

/// Basic font metadata
#[derive(Debug, Clone)]
pub struct FontMetadata {
    pub index: u32,
    pub family_name: String,
    pub is_variable: bool,
}

/// Memory-mapped font cache
pub struct MmapFontCache {
    /// Cached memory-mapped fonts
    files: std::collections::HashMap<PathBuf, Arc<FileInfo>>,
    /// Maximum number of cached files
    max_files: usize,
}

impl MmapFontCache {
    /// Create a new cache with specified maximum files
    pub fn new(max_files: usize) -> Self {
        Self {
            files: std::collections::HashMap::new(),
            max_files,
        }
    }

    /// Get or load a font file
    pub fn get_or_load<P: AsRef<Path>>(&mut self, path: P) -> Result<Arc<FileInfo>> {
        let path = path.as_ref();
        let canonical = path.canonicalize()
            .map_err(|e| Error::Font(format!("Failed to resolve path: {}", e)))?;

        // Check cache
        if let Some(info) = self.files.get(&canonical) {
            debug!("Using cached mmap for: {}", canonical.display());
            return Ok(info.clone());
        }

        // Load new file
        let info = Arc::new(FileInfo::from_path(&canonical)?);

        // Add to cache if there's room
        if self.files.len() < self.max_files {
            self.files.insert(canonical, info.clone());
        } else {
            debug!("MmapFontCache full, not caching");
        }

        Ok(info)
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.files.clear();
        info!("Cleared memory-mapped font cache");
    }

    /// Get number of cached files
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

impl Default for MmapFontCache {
    fn default() -> Self {
        Self::new(256)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_font_type_detection() {
        assert_eq!(FontType::from_data(b"\x00\x01\x00\x00"), Some(FontType::TrueType));
        assert_eq!(FontType::from_data(b"OTTO"), Some(FontType::OpenType));
        assert_eq!(FontType::from_data(b"ttcf"), Some(FontType::TrueTypeCollection));
        assert_eq!(FontType::from_data(b"wOFF"), Some(FontType::WOFF));
        assert_eq!(FontType::from_data(b"wOF2"), Some(FontType::WOFF2));
        assert_eq!(FontType::from_data(b"INVALID"), None);
        assert_eq!(FontType::from_data(b""), None);
    }

    #[test]
    fn test_font_type_is_collection() {
        assert!(FontType::TrueTypeCollection.is_collection());
        assert!(FontType::OpenTypeCollection.is_collection());
        assert!(!FontType::TrueType.is_collection());
        assert!(!FontType::OpenType.is_collection());
        assert!(!FontType::WOFF.is_collection());
    }

    #[test]
    fn test_mmap_cache() {
        let mut cache = MmapFontCache::new(10);
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());

        cache.clear();
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_empty_file_error() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("empty.ttf");
        fs::File::create(&file_path).unwrap();

        let result = FileInfo::from_path(&file_path);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::Font(_)));
    }

    #[test]
    fn test_invalid_font_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("invalid.ttf");
        let mut file = fs::File::create(&file_path).unwrap();
        file.write_all(b"This is not a font file").unwrap();

        let result = FileInfo::from_path(&file_path);
        assert!(result.is_err());
    }
}