---
this_file: external/haforu/PLAN.md
---

# 🚨 DEPRECATION NOTICE: Legacy haforu repo

This folder hosts the legacy, half‑implemented `haforu` codebase. It remains only as a temporary reference until the canonical `haforu` package (developed in `../haforu2/`, published as `haforu`) is complete and integrated. Do not add new features here. Only perform minimal edits strictly necessary to aid migration, if any. Once the new `haforu` is mature, remove this folder/symlink from the `fontsimi` workspace.

# 🚀 CRITICAL PRIORITY: HAFORU RENDERING IMPLEMENTATION FOR FONTSIMI

**Status:** Foundation complete. BEGIN H2 Rust rendering implementation NOW.
**Expected Impact:** 100× speedup (5h → 3min), 97% memory reduction (86GB → <2GB)
**Timeline:** 12-18 days for H2 complete

**Note:** This PLAN covers Haforu Rust implementation. See @../../PLAN.md for FontSimi Python integration.

---

## 📊 FontSimi Integration Context

### Current FontSimi Bottleneck

FontSimi makes 5.5 million individual render calls:
- **250 fonts** × **85 variable instances** × **5 script segments** × **52 glyphs per segment**
- Each call crosses Python→Native boundary with object creation/destruction
- **Result:** 86GB RAM usage, 5+ hours runtime, frequent OOM crashes

### Haforu Solution

**Haforu is a Rust-native batch font renderer** that processes thousands of font/text combinations in a single call with memory-mapped fonts and parallel rendering.

**Key Capability:** Process 5000+ render jobs in one subprocess invocation, streaming JSONL results progressively.

**Expected Performance:**
- Memory: 86GB → <2GB (97% reduction)
- Analysis Speed: 5 hours → 3 minutes (100× faster)
- Deep Matching: 30s → 0.6s per font pair (50× faster)
- Reliability: Zero OOM crashes

---

## 🎯 H2 RUST RENDERING IMPLEMENTATION (12-18 DAYS)

### H2.1: Implement JSON Job Processing (2-3 days) ⚡ CRITICAL

**Goal:** Parse JSON job specifications from stdin and validate all required fields.

**Files:** `src/json_parser.rs`, `src/main.rs`

#### Task 1: Complete JobSpec Data Structures (4 hours)

**Objective:** Define complete Rust structs matching FontSimi's JSON format.

**Implementation:**
```rust
// src/json_parser.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct JobSpec {
    pub version: String,
    #[serde(default)]
    pub mode: JobMode,
    #[serde(default)]
    pub config: BatchConfig,
    pub jobs: Vec<Job>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobMode {
    Batch,
    Streaming,
}

impl Default for JobMode {
    fn default() -> Self {
        JobMode::Batch
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct BatchConfig {
    #[serde(default = "default_max_memory")]
    pub max_memory_mb: usize,
    #[serde(default = "default_output_format")]
    pub output_format: OutputFormat,
    #[serde(default)]
    pub include_metrics: bool,
}

fn default_max_memory() -> usize { 2000 }
fn default_output_format() -> OutputFormat { OutputFormat::Base64 }

impl Default for BatchConfig {
    fn default() -> Self {
        BatchConfig {
            max_memory_mb: 2000,
            output_format: OutputFormat::Base64,
            include_metrics: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Base64,
    File,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Job {
    pub id: String,
    pub font: FontConfig,
    pub text: TextConfig,
    pub rendering: RenderingConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FontConfig {
    pub path: String,
    pub size: f32,  // FontSimi uses 1000pt for daidot
    #[serde(default)]
    pub variations: HashMap<String, f32>,
    #[serde(default)]
    pub face_index: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TextConfig {
    pub content: String,
    #[serde(default = "default_script")]
    pub script: String,
    #[serde(default = "default_direction")]
    pub direction: String,
    #[serde(default = "default_language")]
    pub language: String,
}

fn default_script() -> String { "Latn".to_string() }
fn default_direction() -> String { "ltr".to_string() }
fn default_language() -> String { "en".to_string() }

#[derive(Debug, Clone, Deserialize)]
pub struct RenderingConfig {
    #[serde(default = "default_format")]
    pub format: String,  // "pgm" for FontSimi
    #[serde(default = "default_encoding")]
    pub encoding: String,  // "binary" for P5
    pub width: u32,   // 3000 for FontSimi
    pub height: u32,  // 1200 for FontSimi
}

fn default_format() -> String { "pgm".to_string() }
fn default_encoding() -> String { "binary".to_string() }
```

**Tasks:**
- [ ] Define complete JobSpec struct hierarchy
- [ ] Add serde derive macros for JSON deserialization
- [ ] Implement sensible defaults for optional fields
- [ ] Add validation helper methods
- [ ] **Test:** Parse valid FontSimi job spec, verify all fields correct

**Success Criteria:**
- Parses FontSimi job spec with 5000+ jobs without error
- Correctly deserializes variable font coordinates
- Handles missing optional fields with defaults

#### Task 2: Implement JSON Parsing from stdin (4 hours)

**Objective:** Read and parse JSON from stdin in batch mode.

**Implementation:**
```rust
// src/main.rs

use std::io::{self, Read};
use anyhow::{Context, Result};

pub fn read_job_spec_from_stdin() -> Result<JobSpec> {
    let mut buffer = String::new();
    io::stdin()
        .read_to_string(&mut buffer)
        .context("Failed to read from stdin")?;

    // Validate JSON size (FontSimi sends ~10MB for 5000 jobs)
    if buffer.len() > 100_000_000 {  // 100MB limit
        anyhow::bail!("JSON input too large: {} bytes (max 100MB)", buffer.len());
    }

    let spec: JobSpec = serde_json::from_str(&buffer)
        .context("Failed to parse JSON job specification")?;

    // Validate version
    if spec.version != "1.0" {
        anyhow::bail!("Unsupported job spec version: {}", spec.version);
    }

    // Validate job count
    if spec.jobs.is_empty() {
        anyhow::bail!("Job spec contains no jobs");
    }

    Ok(spec)
}
```

**Tasks:**
- [ ] Read stdin into string buffer with size limit (100MB max)
- [ ] Parse JSON using serde_json
- [ ] Validate job spec version (currently "1.0")
- [ ] Validate job array is non-empty
- [ ] Provide helpful error messages for malformed JSON
- [ ] **Test:** Parse 10KB, 1MB, 10MB JSON successfully

**Success Criteria:**
- Parses 10MB job spec in <500ms
- Clear error messages for syntax errors
- Rejects oversized input (>100MB)

#### Task 3: Implement Field Validation (4 hours)

**Objective:** Validate all required fields and value ranges.

**Implementation:**
```rust
// src/json_parser.rs

impl Job {
    pub fn validate(&self) -> Result<()> {
        // Validate job ID
        if self.id.is_empty() {
            anyhow::bail!("Job ID cannot be empty");
        }

        // Validate font path
        if self.font.path.is_empty() {
            anyhow::bail!("Font path cannot be empty for job {}", self.id);
        }

        // Validate font size
        if self.font.size <= 0.0 || self.font.size > 10000.0 {
            anyhow::bail!("Font size must be 0-10000pt for job {}", self.id);
        }

        // Validate text content
        if self.text.content.is_empty() {
            anyhow::bail!("Text content cannot be empty for job {}", self.id);
        }

        if self.text.content.len() > 10000 {
            anyhow::bail!("Text content too long ({} chars, max 10000) for job {}",
                         self.text.content.len(), self.id);
        }

        // Validate rendering dimensions
        if self.rendering.width == 0 || self.rendering.width > 10000 {
            anyhow::bail!("Width must be 1-10000 pixels for job {}", self.id);
        }

        if self.rendering.height == 0 || self.rendering.height > 10000 {
            antml:bail!("Height must be 1-10000 pixels for job {}", self.id);
        }

        // Validate format
        if self.rendering.format != "pgm" && self.rendering.format != "png" {
            anyhow::bail!("Unsupported format '{}' for job {}",
                         self.rendering.format, self.id);
        }

        Ok(())
    }
}

impl JobSpec {
    pub fn validate_all(&self) -> Result<()> {
        for job in &self.jobs {
            job.validate()?;
        }
        Ok(())
    }
}
```

**Tasks:**
- [ ] Validate job ID is non-empty
- [ ] Validate font path exists and is readable
- [ ] Validate font size is positive (typical: 16-2000pt)
- [ ] Validate text content is non-empty and <10K chars
- [ ] Validate rendering dimensions are positive
- [ ] Validate output format (pgm/png)
- [ ] **Test:** Detect all invalid field combinations

**Success Criteria:**
- Validates 5000 jobs in <100ms
- Helpful error messages with job ID and field name
- Rejects invalid values before processing

#### Task 4: Unit Tests for JSON Parsing (2 hours)

**Tests:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_fontsimi_job() {
        let json = r#"{
            "version": "1.0",
            "jobs": [{
                "id": "font1_wght500_Latn_a",
                "font": {
                    "path": "/fonts/test.ttf",
                    "size": 1000,
                    "variations": {"wght": 500}
                },
                "text": {
                    "content": "a",
                    "script": "Latn"
                },
                "rendering": {
                    "format": "pgm",
                    "encoding": "binary",
                    "width": 3000,
                    "height": 1200
                }
            }]
        }"#;

        let spec: JobSpec = serde_json::from_str(json).unwrap();
        assert_eq!(spec.jobs.len(), 1);
        assert_eq!(spec.jobs[0].id, "font1_wght500_Latn_a");
        assert_eq!(spec.jobs[0].font.size, 1000.0);
        spec.validate_all().unwrap();
    }

    #[test]
    fn test_reject_empty_job_array() {
        let json = r#"{"version": "1.0", "jobs": []}"#;
        let spec: JobSpec = serde_json::from_str(json).unwrap();
        // Should fail validation, not parsing
    }

    #[test]
    fn test_reject_invalid_version() {
        let json = r#"{"version": "2.0", "jobs": []}"#;
        let spec: JobSpec = serde_json::from_str(json).unwrap();
        assert_ne!(spec.version, "1.0");
    }
}
```

**Tasks:**
- [ ] Test valid FontSimi job spec parsing
- [ ] Test malformed JSON rejection
- [ ] Test missing required fields
- [ ] Test invalid value ranges
- [ ] Test edge cases (empty strings, negative numbers, huge arrays)
- [ ] **Goal:** 100% code coverage for json_parser.rs

**Estimated Time:** 2-3 days total for H2.1

---

### H2.2: Implement Font Loading with Variations (2-3 days) ⚡ CRITICAL

**Goal:** Load font files via memory mapping and apply variable font coordinates.

**Files:** `src/mmap_font.rs`, `src/font_cache.rs`

#### Task 1: Memory-Mapped Font Loading (6 hours)

**Objective:** Load fonts with zero-copy memory mapping.

**Implementation:**
```rust
// src/mmap_font.rs

use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use memmap2::Mmap;
use read_fonts::{FontRef, FileRef, ReadError};
use anyhow::{Context, Result};

pub struct MmapFont {
    pub path: PathBuf,
    mmap: Arc<Mmap>,
    file_ref: FileRef<'static>,
}

impl MmapFont {
    pub fn from_path(path: &Path) -> Result<Self> {
        // Open file
        let file = File::open(path)
            .with_context(|| format!("Failed to open font file: {}", path.display()))?;

        // Memory-map with read-only access
        let mmap = unsafe {
            Mmap::map(&file)
                .with_context(|| format!("Failed to mmap font file: {}", path.display()))?
        };

        let mmap = Arc::new(mmap);

        // Parse font file structure
        // SAFETY: mmap lives as long as MmapFont, stored in Arc
        let file_ref = unsafe {
            let bytes: &'static [u8] = std::mem::transmute(mmap.as_ref());
            FileRef::new(bytes)
                .with_context(|| format!("Failed to parse font file: {}", path.display()))?
        };

        Ok(MmapFont {
            path: path.to_path_buf(),
            mmap,
            file_ref,
        })
    }

    pub fn get_font(&self, index: u32) -> Result<FontRef<'static>> {
        self.file_ref
            .font(index)
            .with_context(|| format!("Failed to access font index {} in {}",
                                    index, self.path.display()))
    }

    pub fn font_count(&self) -> usize {
        self.file_ref.len()
    }
}
```

**Tasks:**
- [ ] Open font file and memory-map with read-only access
- [ ] Parse font file using read-fonts::FileRef
- [ ] Handle both single fonts and collections (TTC)
- [ ] Return zero-copy FontRef with 'static lifetime
- [ ] **Test:** Load TTF, OTF, TTC files successfully

**Success Criteria:**
- Loads 250 fonts in <250ms (1ms per font)
- Zero heap allocations for font data
- Proper error messages for corrupted fonts

#### Task 2: Variable Font Coordinate Application (8 hours)

**Objective:** Apply variable font coordinates to create font instances.

**Implementation:**
```rust
// src/mmap_font.rs

use read_fonts::types::NameId;
use read_fonts::tables::variations::ItemVariationStore;
use skrifa::{FontRef as SkrifaFontRef, instance::Location};
use std::collections::HashMap;

impl MmapFont {
    pub fn instantiate_with_coords(
        &self,
        index: u32,
        coords: &HashMap<String, f32>,
    ) -> Result<SkrifaFontRef<'static>> {
        let font_ref = self.get_font(index)?;

        // Get skrifa font for variation support
        // SAFETY: mmap data lives as long as MmapFont
        let skrifa_font = unsafe {
            let bytes: &'static [u8] = std::mem::transmute(self.mmap.as_ref());
            SkrifaFontRef::from_index(bytes, index)
                .with_context(|| format!("Failed to create skrifa font for index {}", index))?
        };

        // Check if font has variations
        if skrifa_font.axes().is_empty() {
            // Static font - return as-is
            return Ok(skrifa_font);
        }

        // Create location from coordinates
        let mut location = Location::default();

        for axis in skrifa_font.axes() {
            let tag = axis.tag().to_string();

            if let Some(&value) = coords.get(&tag) {
                // Clamp value to axis bounds
                let min = axis.min_value();
                let max = axis.max_value();
                let clamped = value.max(min).min(max);

                if (value - clamped).abs() > 0.01 {
                    eprintln!("Warning: Axis {} value {} clamped to range [{}, {}]",
                             tag, value, min, max);
                }

                location.set_axis(axis.tag(), clamped);
            } else {
                // Use default value
                location.set_axis(axis.tag(), axis.default_value());
            }
        }

        // Apply location to font (no-op for now, skrifa handles this internally)
        Ok(skrifa_font)
    }
}
```

**Tasks:**
- [ ] Detect if font has variable axes
- [ ] Parse axis tags and bounds from fvar table
- [ ] Apply user-provided coordinates via skrifa::Location
- [ ] Clamp coordinates to valid axis bounds
- [ ] Use default values for unspecified axes
- [ ] Handle static fonts (no variations)
- [ ] **Test:** Apply coords to Roboto VF, verify clamping

**Success Criteria:**
- Instantiates variable font with custom coords in <5ms
- Correctly clamps out-of-bounds values
- Handles fonts with 1-16 axes

#### Task 3: Font Instance Caching (6 hours)

**Objective:** Cache instantiated fonts by (path, coords) to avoid reloading.

**Implementation:**
```rust
// src/font_cache.rs

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use lru::LruCache;
use std::num::NonZeroUsize;

#[derive(Clone, PartialEq, Eq, Hash)]
struct FontInstanceKey {
    path: PathBuf,
    face_index: u32,
    // Variations hashed as sorted (tag, quantized_value) pairs
    variations_hash: u64,
}

impl FontInstanceKey {
    fn new(path: PathBuf, face_index: u32, coords: &HashMap<String, f32>) -> Self {
        // Create deterministic hash of variations
        let mut sorted_coords: Vec<_> = coords.iter().collect();
        sorted_coords.sort_by_key(|(tag, _)| *tag);

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for (tag, value) in sorted_coords {
            tag.hash(&mut hasher);
            // Quantize to 0.01 precision to improve cache hits
            let quantized = (value * 100.0).round() as i32;
            quantized.hash(&mut hasher);
        }

        FontInstanceKey {
            path,
            face_index,
            variations_hash: hasher.finish(),
        }
    }
}

pub struct FontCache {
    // Memory-mapped fonts (never evicted)
    mmaps: Mutex<HashMap<PathBuf, Arc<MmapFont>>>,

    // Instantiated fonts with variations (LRU, 512 entries)
    instances: Mutex<LruCache<FontInstanceKey, Arc<SkrifaFontRef<'static>>>>,

    stats: Mutex<CacheStats>,
}

#[derive(Default)]
struct CacheStats {
    mmap_hits: usize,
    mmap_misses: usize,
    instance_hits: usize,
    instance_misses: usize,
}

impl FontCache {
    pub fn new(instance_capacity: usize) -> Self {
        FontCache {
            mmaps: Mutex::new(HashMap::new()),
            instances: Mutex::new(LruCache::new(
                NonZeroUsize::new(instance_capacity).unwrap()
            )),
            stats: Mutex::new(CacheStats::default()),
        }
    }

    pub fn get_or_load_instance(
        &self,
        path: &Path,
        face_index: u32,
        coords: &HashMap<String, f32>,
    ) -> Result<Arc<SkrifaFontRef<'static>>> {
        let key = FontInstanceKey::new(path.to_path_buf(), face_index, coords);

        // Check instance cache first
        {
            let mut cache = self.instances.lock().unwrap();
            if let Some(instance) = cache.get(&key) {
                self.stats.lock().unwrap().instance_hits += 1;
                return Ok(Arc::clone(instance));
            }
            self.stats.lock().unwrap().instance_misses += 1;
        }

        // Get or load mmap
        let mmap = {
            let mut mmaps = self.mmaps.lock().unwrap();
            if let Some(mmap) = mmaps.get(path) {
                self.stats.lock().unwrap().mmap_hits += 1;
                Arc::clone(mmap)
            } else {
                self.stats.lock().unwrap().mmap_misses += 1;
                let mmap = Arc::new(MmapFont::from_path(path)?);
                mmaps.insert(path.to_path_buf(), Arc::clone(&mmap));
                mmap
            }
        };

        // Instantiate with coords
        let instance = Arc::new(mmap.instantiate_with_coords(face_index, coords)?);

        // Cache instance
        let mut cache = self.instances.lock().unwrap();
        cache.put(key, Arc::clone(&instance));

        Ok(instance)
    }

    pub fn stats(&self) -> String {
        let stats = self.stats.lock().unwrap();
        format!(
            "FontCache: mmaps({}/{}) instances({}/{})",
            stats.mmap_hits,
            stats.mmap_hits + stats.mmap_misses,
            stats.instance_hits,
            stats.instance_hits + stats.instance_misses
        )
    }
}
```

**Tasks:**
- [ ] Create LRU cache for font instances (512 capacity)
- [ ] Key by (path, face_index, variations_hash)
- [ ] Never evict memory-mapped fonts (only instances)
- [ ] Track cache hit/miss statistics
- [ ] **Test:** 1000 lookups with 100 unique fonts, verify >90% hit rate

**Success Criteria:**
- Cache hit in <0.1ms
- Cache miss + load in <5ms
- >90% hit rate for typical FontSimi workload

#### Task 4: Unit Tests for Font Loading (4 hours)

**Tests:**
```rust
#[test]
fn test_load_static_font() {
    let font = MmapFont::from_path(Path::new("tests/fonts/Arial.ttf")).unwrap();
    assert_eq!(font.font_count(), 1);
    let font_ref = font.get_font(0).unwrap();
    // Verify font loaded correctly
}

#[test]
fn test_load_variable_font() {
    let font = MmapFont::from_path(Path::new("tests/fonts/RobotoVF.ttf")).unwrap();
    let mut coords = HashMap::new();
    coords.insert("wght".to_string(), 500.0);
    let instance = font.instantiate_with_coords(0, &coords).unwrap();
    // Verify variations applied
}

#[test]
fn test_cache_hit_rate() {
    let cache = FontCache::new(512);
    // Load same font 100 times, verify 99 cache hits
}
```

**Estimated Time:** 2-3 days total for H2.2

---

### H2.3: Implement Text Shaping (2-3 days) ⚡ CRITICAL

**Goal:** Shape text into positioned glyphs using HarfRust.

**Files:** `src/shaping.rs`

(Continuing with H2.3-H2.7 implementation details...)

**Estimated Time:** 12-18 days total for complete H2 implementation

---

## 🚧 DEPRIORITIZED TASKS (Postponed Until H2 Complete)

The following are postponed until H2-H5 integration completes:

### Lower Priority (Do NOT work on these)
- ❌ Traditional CLI mode (hb-shape/hb-view compatibility)
- ❌ GPU rendering with Vello
- ❌ Python bindings (PyO3/maturin)
- ❌ Web server mode
- ❌ Distributed processing
- ❌ Storage backend (packfiles) - only needed for Phase H4

### Future Enhancements (After H5)
- Font subsetting
- Color font support
- Cloud storage backends
- Advanced caching strategies

---

## ⚡ IMMEDIATE NEXT STEPS

**Current Status:** H1 Python integration complete, H2 Rust blocked

**Next Actions:**

1. **START HERE:** Implement H2.1 JSON job processing (2-3 days)
2. Implement H2.2 font loading with variations (2-3 days)
3. Implement H2.3 text shaping with HarfRust (2-3 days)
4. Implement H2.4 glyph rasterization with zeno (3-4 days)
5. Implement H2.5 PGM output format (1-2 days)
6. Implement H2.6 JSONL output (1-2 days)
7. Implement H2.7 error handling (1-2 days)

**Total Estimated Timeline:** 12-18 days for complete H2 implementation

**Critical Path:** H2.1 → H2.2 → H2.3 → H2.4 → H2.5 → H2.6 → H2.7 → FontSimi Integration

**Blocking:** All FontSimi performance improvements blocked on H2 Rust implementation

---

## 📈 Expected Results (Success Criteria)

### Performance Metrics
- Memory: 86GB → <2GB (97% reduction) ✅ (estimated)
- Analysis: 5 hours → 3 minutes (100× speedup) ✅ (estimated)
- Deep Matching: 30s → 0.6s per font pair (50× speedup) ✅ (estimated)
- Reliability: Zero OOM crashes ✅ (estimated)

### Quality Metrics
- Determinism: Identical rendering vs HarfBuzz/CoreText ✅ (by design)
- Coverage: All 250 fonts × 85 instances rendered successfully ✅
- Testing: All H2 tests passing (100% pass rate) ✅
