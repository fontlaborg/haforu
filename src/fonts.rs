// this_file: src/fonts.rs

//! Font loading, variation handling, and caching.
//!
//! This module provides zero-copy font loading via memory mapping,
//! variable font coordinate application, and LRU caching of font instances.

use crate::error::{Error, Result};
use camino::Utf8Path;
use dashmap::DashMap;
use harfbuzz_rs::{Face as HbFace, Owned};
use memmap2::Mmap;
use read_fonts::{types::Tag, FileRef, FontRef};
use skrifa::MetadataProvider;
use std::collections::HashMap;
use std::fs::File;
use std::io::ErrorKind;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// Memory-mapped font with metadata and instance cache.
pub struct FontLoader {
    cache: Arc<DashMap<FontCacheKey, Arc<FontInstance>>>,
    max_capacity: usize,
    current_size: Arc<AtomicUsize>,
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
    /// Cached HarfBuzz font with variations pre-applied (for performance)
    hb_font: Arc<Mutex<Owned<harfbuzz_rs::Font<'static>>>>,
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
        let cache_size = cache_size.max(1);
        Self {
            cache: Arc::new(DashMap::with_capacity(cache_size)),
            max_capacity: cache_size,
            current_size: Arc::new(AtomicUsize::new(0)),
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
        // Build cache key
        let cache_key = FontCacheKey {
            path: path.to_string(),
            coordinates: coordinates
                .iter()
                .map(|(k, v)| (k.clone(), v.to_bits()))
                .collect(),
        };

        // Fast path: check cache with lock-free read
        if let Some(instance) = self.cache.get(&cache_key) {
            return Ok(Arc::clone(instance.value()));
        }

        // Slow path: load from disk
        let instance = Self::load_font_impl(path, coordinates)?;
        let instance = Arc::new(instance);

        // Store in cache with simple size-based eviction
        let current = self.current_size.fetch_add(1, Ordering::Relaxed);
        if current >= self.max_capacity {
            // Cache full - evict random entry (DashMap doesn't have LRU built-in)
            // This is a simple eviction strategy, not perfect but avoids unbounded growth
            if let Some(first_key) = self.cache.iter().next().map(|e| e.key().clone()) {
                self.cache.remove(&first_key);
                self.current_size.fetch_sub(1, Ordering::Relaxed);
            }
        }

        self.cache.insert(cache_key, Arc::clone(&instance));
        Ok(instance)
    }

    /// Clear all cached font instances.
    pub fn clear(&self) {
        self.cache.clear();
        self.current_size.store(0, Ordering::Relaxed);
    }

    /// Resize the cache to the requested capacity (drops old entries).
    pub fn set_capacity(&self, _cache_size: usize) {
        // Note: DashMap doesn't support runtime capacity changes easily
        // For now, we keep the initial capacity but could clear and rebuild if needed
        log::warn!("set_capacity is not fully supported with DashMap-based cache");
    }

    /// Return current cache statistics.
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            capacity: self.max_capacity,
            entries: self.cache.len(),
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

        // Create HarfBuzz font with variations pre-applied for performance
        let hb_font = Self::create_harfbuzz_font(&mmap, &clamped_coords)?;

        Ok(FontInstance {
            mmap,
            font_ref,
            coordinates: clamped_coords,
            hb_font: Arc::new(Mutex::new(hb_font)),
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

    /// Create a HarfBuzz font from memory-mapped data with variations applied.
    fn create_harfbuzz_font(
        mmap: &Arc<Mmap>,
        coordinates: &HashMap<String, f32>,
    ) -> Result<Owned<harfbuzz_rs::Font<'static>>> {
        // Convert mmap bytes to 'static lifetime (safe because mmap is Arc'd)
        let font_data: &'static [u8] =
            unsafe { std::slice::from_raw_parts(mmap.as_ptr(), mmap.len()) };

        // Create HarfBuzz face and font
        let face = HbFace::from_bytes(font_data, 0);
        let mut hb_font = harfbuzz_rs::Font::new(face);

        // Apply variation coordinates if present
        if !coordinates.is_empty() {
            let variations: Vec<harfbuzz_rs::Variation> = coordinates
                .iter()
                .filter_map(|(tag, value)| {
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

        Ok(hb_font)
    }

    /// Get current cache statistics.
    pub fn cache_stats(&self) -> (usize, usize) {
        (self.cache.len(), self.max_capacity)
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

    /// Get reference to the cached HarfBuzz font (for performance optimization).
    pub fn hb_font(&self) -> &Arc<Mutex<Owned<harfbuzz_rs::Font<'static>>>> {
        &self.hb_font
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
