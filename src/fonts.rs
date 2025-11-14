// this_file: src/fonts.rs

//! Font loading, variation handling, and caching.
//!
//! This module provides zero-copy font loading via memory mapping,
//! variable font coordinate application, and LRU caching of font instances.

use crate::error::{Error, Result};
use camino::Utf8Path;
use lru::LruCache;
use memmap2::Mmap;
use read_fonts::{types::Tag, FileRef, FontRef};
use skrifa::MetadataProvider;
use std::collections::HashMap;
use std::fs::File;
use std::io::ErrorKind;
use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Memory-mapped font with metadata and instance cache.
pub struct FontLoader {
    cache: Arc<Mutex<LruCache<FontCacheKey, Arc<FontInstance>>>>,
}

/// Font cache statistics for observability.
#[derive(Debug, Clone, Copy)]
pub struct CacheStats {
    /// Maximum number of cached font instances.
    pub capacity: usize,
    /// Currently cached font instances.
    pub entries: usize,
}

/// Font instance with applied variations.
pub struct FontInstance {
    /// Memory-mapped font data
    #[allow(dead_code)]
    mmap: Arc<Mmap>,
    /// Font reference (zero-copy view into mmap)
    font_ref: FontRef<'static>,
    /// Applied variation coordinates
    coordinates: HashMap<String, f32>,
}

/// Cache key for font instances.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct FontCacheKey {
    path: String,
    coordinates: Vec<(String, u32)>, // (axis, f32 as bits)
}

impl FontLoader {
    /// Create a new font loader with specified cache size.
    pub fn new(cache_size: usize) -> Self {
        let cache_size = NonZeroUsize::new(cache_size).unwrap_or(NonZeroUsize::new(512).unwrap());
        Self {
            cache: Arc::new(Mutex::new(LruCache::new(cache_size))),
        }
    }

    /// Load a font and apply variable font coordinates.
    ///
    /// Returns a cached instance if available, otherwise loads from disk.
    pub fn load_font(
        &self,
        path: &Utf8Path,
        coordinates: &HashMap<String, f32>,
    ) -> Result<Arc<FontInstance>> {
        // Check cache first
        let cache_key = FontCacheKey {
            path: path.to_string(),
            coordinates: coordinates
                .iter()
                .map(|(k, v)| (k.clone(), v.to_bits()))
                .collect(),
        };

        {
            let mut cache = self.cache.lock().unwrap();
            if let Some(instance) = cache.get(&cache_key) {
                return Ok(Arc::clone(instance));
            }
        }

        // Not in cache - load from disk
        let instance = Self::load_font_impl(path, coordinates)?;
        let instance = Arc::new(instance);

        // Store in cache
        {
            let mut cache = self.cache.lock().unwrap();
            cache.put(cache_key, Arc::clone(&instance));
        }

        Ok(instance)
    }

    /// Clear all cached font instances.
    pub fn clear(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear();
    }

    /// Resize the cache to the requested capacity (drops old entries).
    pub fn set_capacity(&self, cache_size: usize) {
        let cap = NonZeroUsize::new(cache_size.max(1)).unwrap();
        let mut cache = self.cache.lock().unwrap();
        if cache.cap() == cap {
            return;
        }
        *cache = LruCache::new(cap);
    }

    /// Return current cache statistics.
    pub fn stats(&self) -> CacheStats {
        let cache = self.cache.lock().unwrap();
        CacheStats {
            capacity: cache.cap().get(),
            entries: cache.len(),
        }
    }

    /// Internal implementation: load font from disk and apply variations.
    fn load_font_impl(path: &Utf8Path, coordinates: &HashMap<String, f32>) -> Result<FontInstance> {
        // Memory-map the font file
        let std_path = path.as_std_path();
        let file = File::open(std_path).map_err(|e| match e.kind() {
            ErrorKind::NotFound => Error::FontNotFound {
                path: std_path.to_path_buf(),
            },
            _ => Error::Mmap {
                path: std_path.to_path_buf(),
                source: e,
            },
        })?;

        // Pre-check file size against limit
        let meta = file.metadata().map_err(|e| Error::Mmap {
            path: std_path.to_path_buf(),
            source: e,
        })?;
        crate::security::validate_font_size(meta.len())?;

        let mmap = unsafe {
            Mmap::map(&file).map_err(|e| Error::Mmap {
                path: std_path.to_path_buf(),
                source: e,
            })?
        };

        let mmap = Arc::new(mmap);

        // Parse font
        let font_data: &'static [u8] =
            unsafe { std::slice::from_raw_parts(mmap.as_ptr(), mmap.len()) };

        let file_ref = FileRef::new(font_data).map_err(|e| Error::InvalidFont {
            path: path.as_std_path().to_path_buf(),
            reason: format!("Failed to parse font file: {}", e),
        })?;

        let font_ref = match file_ref {
            FileRef::Font(f) => f,
            FileRef::Collection(c) => c.get(0).map_err(|e| Error::InvalidFont {
                path: path.as_std_path().to_path_buf(),
                reason: format!("Failed to get font from collection: {}", e),
            })?,
        };

        // Validate and clamp variation coordinates
        let clamped_coords = if !coordinates.is_empty() {
            Self::validate_and_clamp_coordinates(&font_ref, path.as_std_path(), coordinates)?
        } else {
            coordinates.clone()
        };

        Ok(FontInstance {
            mmap,
            font_ref,
            coordinates: clamped_coords,
        })
    }

    /// Validate variation axes and clamp coordinates to bounds.
    fn validate_and_clamp_coordinates(
        font: &FontRef,
        path: &Path,
        coordinates: &HashMap<String, f32>,
    ) -> Result<HashMap<String, f32>> {
        // Extract available axes from font
        let axes: HashMap<String, (f32, f32, f32)> = font
            .axes()
            .iter()
            .map(|axis| {
                let tag = axis.tag().to_string();
                (
                    tag,
                    (axis.min_value(), axis.default_value(), axis.max_value()),
                )
            })
            .collect();

        if axes.is_empty() {
            // Static font - ignore all coordinates
            if !coordinates.is_empty() {
                log::warn!(
                    "Font {} is static but coordinates provided - ignoring",
                    path.display()
                );
            }
            return Ok(HashMap::new());
        }

        // Validate and clamp each coordinate
        let mut clamped = HashMap::new();
        for (axis, value) in coordinates {
            if let Some((min, _default, max)) = axes.get(axis) {
                // Prefer font-provided bounds, but apply well-known sane clamps
                // for common axes as an additional safeguard.
                let (hard_min, hard_max) = match axis.as_str() {
                    "wght" => (100.0_f32, 900.0_f32),
                    "wdth" => (50.0_f32, 200.0_f32),
                    _ => (*min, *max),
                };

                // Combine clamps conservatively within the intersection of bounds
                let eff_min = hard_min.max(*min);
                let eff_max = hard_max.min(*max);
                let clamped_value = value.clamp(eff_min, eff_max);
                if (clamped_value - value).abs() > 0.001 {
                    log::warn!(
                        "Coordinate for axis '{}' clamped from {} to {} (bounds: [{}, {}], hard: [{}, {}])",
                        axis,
                        value,
                        clamped_value,
                        min,
                        max,
                        hard_min,
                        hard_max
                    );
                }
                clamped.insert(axis.clone(), clamped_value);
            } else {
                // Axis not present in this font: warn-and-drop per integration contract.
                log::warn!(
                    "Unknown variation axis '{}' for font {} — dropping coordinate",
                    axis,
                    path.display()
                );
                // Intentionally do not include this axis in the resulting map.
            }
        }

        Ok(clamped)
    }

    /// Get current cache statistics.
    pub fn cache_stats(&self) -> (usize, usize) {
        let cache = self.cache.lock().unwrap();
        (cache.len(), cache.cap().get())
    }
}

impl FontInstance {
    /// Get the font reference.
    pub fn font_ref(&self) -> &FontRef<'static> {
        &self.font_ref
    }

    /// Get the applied variation coordinates.
    pub fn coordinates(&self) -> &HashMap<String, f32> {
        &self.coordinates
    }

    /// Get the raw font data bytes.
    pub fn font_data(&self) -> &[u8] {
        self.mmap.as_ref()
    }

    /// Create a skrifa Location for rendering.
    pub fn location(&self) -> Vec<(Tag, f32)> {
        self.coordinates
            .iter()
            .filter_map(|(tag_str, value)| {
                Tag::new_checked(tag_str.as_bytes())
                    .ok()
                    .map(|tag| (tag, *value))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Tests require actual font files to be present
    // In production, use test fixtures from test-fonts/

    #[test]
    fn test_font_loader_creation() {
        let loader = FontLoader::new(256);
        let (used, cap) = loader.cache_stats();
        assert_eq!(used, 0);
        assert_eq!(cap, 256);
    }

    #[test]
    fn test_cache_key_equality() {
        let key1 = FontCacheKey {
            path: "font.ttf".to_string(),
            coordinates: vec![("wght".to_string(), 600.0f32.to_bits())],
        };
        let key2 = FontCacheKey {
            path: "font.ttf".to_string(),
            coordinates: vec![("wght".to_string(), 600.0f32.to_bits())],
        };
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_cache_key_inequality_different_coords() {
        let key1 = FontCacheKey {
            path: "font.ttf".to_string(),
            coordinates: vec![("wght".to_string(), 600.0f32.to_bits())],
        };
        let key2 = FontCacheKey {
            path: "font.ttf".to_string(),
            coordinates: vec![("wght".to_string(), 700.0f32.to_bits())],
        };
        assert_ne!(key1, key2);
    }

    #[test]
    fn load_static_font_drops_unknown_axes() {
        // Arial-Black.ttf in testdata is a static font; any coordinates should be ignored.
        let loader = FontLoader::new(8);
        let font_path = camino::Utf8PathBuf::from("testdata/fonts/Arial-Black.ttf");
        // Provide a couple of coordinates including a common axis and a bogus one.
        let mut coords = HashMap::new();
        coords.insert("wght".to_string(), 700.0_f32);
        coords.insert("ZZZZ".to_string(), 12.34_f32);

        // Load should succeed and coordinates should be empty for static fonts
        let inst = loader
            .load_font(&font_path, &coords)
            .expect("static font should load successfully");
        assert!(
            inst.coordinates().is_empty(),
            "Static font must not retain variation coordinates"
        );
    }

    #[test]
    fn variable_font_clamps_and_drops_coordinates() {
        let loader = FontLoader::new(16);
        let font_path = camino::Utf8PathBuf::from("testdata/fonts/IBMPlexSans-VF.ttf");
        let mut coords = HashMap::new();
        coords.insert("wght".to_string(), 2500.0);
        coords.insert("wdth".to_string(), 5.0);
        coords.insert("ZZZZ".to_string(), 12.34);

        let inst = loader
            .load_font(&font_path, &coords)
            .expect("variable font should load successfully");
        let applied = inst.coordinates();
        assert!(applied.contains_key("wght"));
        assert!(applied.contains_key("wdth"));
        assert!(
            !applied.contains_key("ZZZZ"),
            "Unknown coordinates must be dropped"
        );

        // Compare against actual axis bounds reported by skrifa to ensure clamps applied.
        let mut axis_bounds = HashMap::new();
        for axis in inst.font_ref().axes().iter() {
            axis_bounds.insert(axis.tag().to_string(), (axis.min_value(), axis.max_value()));
        }

        let (wght_min, wght_max) = axis_bounds.get("wght").copied().expect("wght axis present");
        let clamped_wght = *applied.get("wght").expect("wght applied");
        assert!(
            clamped_wght <= wght_max.min(900.0) + f32::EPSILON,
            "wght must respect hard + font max"
        );
        assert!(
            clamped_wght >= wght_min.max(100.0) - f32::EPSILON,
            "wght must respect hard + font min"
        );

        let (wdth_min, wdth_max) = axis_bounds.get("wdth").copied().expect("wdth axis present");
        let clamped_wdth = *applied.get("wdth").expect("wdth applied");
        assert!(
            clamped_wdth >= wdth_min.max(50.0) - f32::EPSILON,
            "wdth must respect hard minimum"
        );
        assert!(
            clamped_wdth <= wdth_max.min(200.0) + f32::EPSILON,
            "wdth must respect hard maximum"
        );
    }

    #[test]
    fn location_reports_sanitized_coordinates() {
        let loader = FontLoader::new(16);
        let font_path = camino::Utf8PathBuf::from("testdata/fonts/IBMPlexSans-VF.ttf");
        let mut coords = HashMap::new();
        coords.insert("wdth".to_string(), 10.0);

        let inst = loader
            .load_font(&font_path, &coords)
            .expect("variable font should load");
        let expected = inst.coordinates().clone();
        let mut location_map = HashMap::new();
        for (tag, value) in inst.location() {
            location_map.insert(tag.to_string(), value);
        }

        assert_eq!(
            location_map.get("wdth"),
            expected.get("wdth"),
            "Streaming to skrifa must use sanitized values"
        );
    }
}
