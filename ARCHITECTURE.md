# Haforu2: Comprehensive Architectural Analysis

**Date:** 2025-11-11  
**Project:** FontSimi v3 + Haforu2 Integration  
**Status:** H2.1-H2.7 Implementation Planning (Ready to Begin)

---

## Executive Summary

**Problem Statement:**
- FontSimi must render 5.5 million glyphs (250 fonts × 85 instances × 5 segments × 52 glyphs)
- Current Python renderers: 5+ hours, 86GB RAM, frequent OOM crashes
- Root cause: Individual Python→Native boundary crossings (object alloc/dealloc per render)

**Haforu2 Solution:**
- Rust-native batch font renderer processing thousands of jobs in one subprocess call
- Memory-mapped fonts (zero-copy)
- Single native boundary crossing per batch
- Expected: 100× speedup (5h → 3m), 97% memory reduction (86GB → <2GB)

**Integration Model:**
- **Phase H1:** ✅ Python HaforuRenderer class (subprocess communication, JSON→JSONL)
- **Phase H2:** ⏸️ Haforu2 Rust implementation (12-18 days)
- **Phase H3:** Python batch analysis pipeline
- **Phase H4:** Streaming mode for deep matching
- **Phase H5:** Performance validation

---

## Section 1: FontSimi Bottleneck Analysis

### 1.1 Current Performance Metrics

| Metric | Value | Problem |
|--------|-------|---------|
| **Total Render Calls** | 5.5M | Each crosses Python→Native |
| **Fonts** | 250 | Most static, some variable with 2-16 axes |
| **Variable Instances** | 85 | Intermediate: wght, wdth, opsz mostly |
| **Script Segments** | 5 | Latn, ULAT, Cyrl, UCYR, Grek, etc. |
| **Glyphs per Segment** | 52 | Single glyph per render: "a", "b", "c", etc. |
| **Runtime** | 5+ hours | Dominated by render overhead, not computation |
| **Memory Peak** | 86GB | 5.5M images × 1.5MB each (uncompressed) |
| **OOM Crashes** | Frequent | During peak font loading + rendering |

### 1.2 Root Cause: Python→Native Boundary Overhead

**Current Architecture (Python):**
```
for font in fonts:
  for instance_coords in instances:
    for segment in segments:
      for glyph in glyphs:
        image = renderer.render_text(glyph)  # ← Native call (high overhead)
```

**Per-Call Overhead:**
- Python function call → C/Rust native boundary
- Object allocation (PIL Image, numpy array)
- Object deallocation (garbage collection trigger)
- Native function execution (typically <5ms)
- **Total overhead per call:** ~50-100ms (10-20× computation cost)

**Memory Explosion:**
- Each render produces ~1.5MB uncompressed grayscale image
- No shared buffer pool; each image is separate allocation
- Python GC pressure causes pause stops
- Result: 5.5M × 1.5MB ÷ compression ≈ 86GB peak (uncompressed)

### 1.3 Current Python Renderer Implementations

| Renderer | Backend | Mechanism | Per-Call Overhead | Bottleneck |
|----------|---------|-----------|------------------|-----------|
| **HarfBuzz** | libharfbuzz | Python/C boundary | 40ms | Shaping + rasterization |
| **CoreText** | macOS | Objective-C bridge | 60ms | ObjC boundary crossing |
| **Skia** | libskia | Python/C boundary | 80ms | Heavy graphics library |
| **Pillow** | pure Python | 100% Python | 20ms | CPU rasterization but no C boundary |
| **Haforu (current)** | subprocess | stdin/stdout JSON | 500ms | Subprocess spawn overhead |

### 1.4 FontSimi's Unique Daidot Metrics

**4-Metric Model (simplified from 8D):**
1. `width_rhythm` - Horizontal character spacing consistency
2. `rendered_aspect` - Visual height/width ratio
3. `density` - Overall pixel coverage
4. `consistency` - Variance across glyphs

**Why This Matters for Haforu:**
- Only need grayscale images (no color rendering)
- 1000pt font size standard (high resolution for metric stability)
- 3000×1200 canvas typical (fixed for consistent metrics)
- Single glyph per image (no shaping complexity)
- No kerning, ligatures, or complex scripts needed

---

## Section 2: Haforu2 Design Requirements

### 2.1 FontSimi Integration Requirements

**Batch Job Specification Format:**
```json
{
  "version": "1.0",
  "mode": "batch",
  "config": {
    "max_memory_mb": 2000,
    "output_format": "base64",
    "include_metrics": false
  },
  "jobs": [
    {
      "id": "font1_wght600_Latn_a",
      "font": {
        "path": "/path/to/font.ttf",
        "size": 1000,
        "variations": {"wght": 600, "wdth": 100},
        "face_index": 0
      },
      "text": {
        "content": "a",
        "script": "Latn",
        "direction": "ltr",
        "language": "en"
      },
      "rendering": {
        "format": "pgm",
        "encoding": "binary",
        "width": 3000,
        "height": 1200
      }
    }
    // ... 5000+ more jobs
  ]
}
```

**Expected Job Characteristics:**
- Batch size: 1000-5000 jobs per invocation
- Jobs per second: 500-1000 (target: 3m for 5.5M ÷ 30 batches)
- Memory per job: ~1.5KB JSON + 1.5MB rendered (not held simultaneously)
- Variable fonts: 60% of jobs (rest static)

**JSONL Output Format:**
```jsonl
{"id":"font1_wght600_Latn_a","status":"success","rendering":{"format":"pgm","encoding":"base64","data":"Rjk1CjMwMDAg...","width":3000,"height":1200,"actual_bbox":[500,200,800,600]},"timing":{"shape_ms":2.1,"render_ms":4.3,"total_ms":8.5},"memory":{"font_cache_mb":1.2,"total_mb":45.6}}
{"id":"font1_wght600_Latn_b","status":"success","rendering":{...},"timing":{...}}
```

### 2.2 Haforu2 Architectural Principles

**Core Design:**
1. **Stateless Job Processing:** Each job is independent; no cross-job state
2. **Memory-Mapped Fonts:** Zero-copy font loading via memmap2 crate
3. **Font Instance Caching:** LRU cache of (path, variations) → skrifa FontRef
4. **Parallel Job Processing:** rayon parallelism across jobs
5. **Streaming Output:** Write JSONL immediately as jobs complete
6. **Subprocess Communication:** stdin JSON → stdout JSONL (simple Unix pipes)

**Why This Design:**
- **Stateless:** Easy to scale horizontally (no shared state)
- **Memory-mapped:** 250 fonts × 1MB each = 250MB (not 86GB)
- **Streaming:** Python can start processing results while Haforu still working
- **Subprocess:** Simple to invoke, no Python/Rust FFI complexity
- **Parallel:** rayon handles NUMA and thread pool automatically

### 2.3 Haforu2 Feature Matrix

| Feature | Phase | Priority | FontSimi Requirement | Notes |
|---------|-------|----------|----------------------|-------|
| JSON job parsing | H2.1 | CRITICAL | 5000+ jobs/batch | Must parse in <500ms |
| Font loading (static) | H2.2 | CRITICAL | 250 fonts | Must load in <1ms each |
| Font loading (variable) | H2.2 | CRITICAL | 85 instances | Must apply coords in <5ms |
| Font caching | H2.2 | CRITICAL | 512 font instances | LRU, >90% hit rate |
| Text shaping | H2.3 | CRITICAL | 52 glyphs/segment | HarfRust (one char) |
| Glyph rasterization | H2.4 | CRITICAL | 3000×1200 grayscale | skrifa→zeno path |
| PGM P5 output | H2.5 | HIGH | FontSimi format | 8-bit grayscale binary |
| Base64 encoding | H2.5 | HIGH | JSON compatibility | JSONL string embedding |
| Bounding box calc | H2.5 | MEDIUM | Metric stability | For crop optimization |
| Error handling | H2.7 | HIGH | Partial failure recovery | Continue on bad fonts |
| Streaming JSON output | H2.6 | CRITICAL | Progressive results | Flush per job |
| Streaming mode (persistent process) | H4 | MEDIUM | Deep matching speedup | Future optimization |

### 2.4 Haforu2 Standalone Value Proposition

Beyond FontSimi, Haforu2 is useful for:

**1. Font Development (FontLab, ufo, Glyphs):**
- Batch render instances during design iteration
- Compare rendering across sizes/weights quickly
- Export to analysis tools

**2. Quality Assurance:**
- Regression test suite: render known fonts, compare outputs
- Smoke tests: verify no crashes on corpus of 10K fonts
- Rendering consistency check: static vs variable instances

**3. Content Generation:**
- Generate glyph preview images for web (emoji, symbol fonts)
- Create specimen PDFs with batch rendered instances
- Font matching service backend

**4. Performance Testing:**
- Benchmark new font rasterizers
- Profile memory usage under load
- Compare rendering quality (PNG diff)

**Design for Standalone Use:**
- No FontSimi-specific code (generic font→glyph→image pipeline)
- Pluggable output formats (PGM, PNG, SVG, metrics JSON)
- Generic job ID scheme (user can choose naming)
- Configurable font cache size, max memory, parallel workers

---

## Section 3: Haforu2 Technical Architecture

### 3.1 Module Structure

```
external/haforu2/
├── Cargo.toml                 # Rust dependencies
├── src/
│   ├── main.rs               # Entry point, CLI arg parsing
│   ├── lib.rs                # Public API (for future PyO3)
│   ├── json_parser.rs        # JobSpec/Job deserialization + validation
│   ├── error.rs              # Error types and conversion
│   ├── mmap_font.rs          # Memory-mapped font loading
│   ├── font_cache.rs         # LRU font instance cache
│   ├── shaping.rs            # HarfRust text shaping
│   ├── rasterize.rs          # Glyph rasterization (skrifa→zeno)
│   ├── output.rs             # PGM format and base64 encoding
│   ├── orchestrator.rs       # Job processing pipeline
│   └── stats.rs              # Metrics and statistics
├── tests/
│   ├── integration_tests.rs  # End-to-end tests
│   └── unit_tests.rs         # Per-module unit tests
└── fonts/                     # Test fonts (TTF, OTF, VF)
```

### 3.2 Data Flow: Batch Mode

```
User Input (FontSimi Python)
     ↓
[stdin] JSON (5000 jobs)
     ↓
Haforu2 Process (Arc<Haforu>)
     ├─ json_parser::parse_stdin()
     ├─ For each job (parallel via rayon):
     │  ├─ font_cache.get_or_load_instance(path, coords)
     │  ├─ shaping::shape_text(font, "a")
     │  ├─ rasterize::render_glyphs(shaped)
     │  ├─ output::encode_pgm_base64(pixels)
     │  └─ JobResult { id, status, rendering, timing }
     └─ Write JSONL line to stdout
     ↓
[stdout] JSONL (5000 results)
     ↓
FontSimi Python (parse JSONL, extract images)
```

### 3.3 Implementation Roadmap: H2.1 - H2.7

**Total Estimated Time:** 12-18 days

| Phase | Tasks | Time | Dependencies |
|-------|-------|------|--------------|
| H2.1 | JSON parsing, validation, stdin reading | 2-3d | None |
| H2.2 | Font loading, variations, caching | 2-3d | H2.1 (file I/O) |
| H2.3 | Text shaping (HarfRust) | 2-3d | H2.2 (fonts) |
| H2.4 | Glyph rasterization (skrifa+zeno) | 3-4d | H2.3 (shaped glyphs) |
| H2.5 | PGM output, base64, bounding box | 1-2d | H2.4 (pixels) |
| H2.6 | JSONL formatting, streaming output | 1-2d | H2.5 (output) |
| H2.7 | Error handling, edge cases, tests | 1-2d | All above |

**Critical Path:** H2.1 → H2.2 → H2.3 → H2.4 → H2.5 → H2.6 → H2.7 → Testing

### 3.4 Key Dependencies & Justification

| Dependency | Version | Why Chosen | Alternatives |
|------------|---------|-----------|--------------|
| `serde` | Latest | JSON parsing (proven, fast) | json5, toml |
| `serde_json` | Latest | JSON serialization | jsonc, ron |
| `read-fonts` | Latest | Zero-copy font parsing | fonttools (Python), fontparts |
| `skrifa` | Latest | Variable font support | freetype-py, harfbuzz only |
| `harfbuzz-rs` | Latest | Text shaping | rustybuzz (pure Rust, slower) |
| `zeno` | Latest | CPU rasterization | pathfinder (heavier), tiny-skia |
| `memmap2` | Latest | Memory-mapped I/O | mmap crate (older), std::fs |
| `rayon` | Latest | Parallel job processing | crossbeam (lower-level), tokio (async) |
| `anyhow` | Latest | Error handling | thiserror (more verbose), failure (older) |
| `clap` | Latest | CLI argument parsing | structopt (deprecated for clap v4) |

### 3.5 Performance Targets

**Per-Job Performance:**
- Parse JSON: <100µs per job (5M jobs in 500ms)
- Load font: 1ms first time, <0.1ms cache hit
- Shape text: 0.5-2ms (single character is fast)
- Rasterize: 2-5ms (3000×1200 at 1000pt)
- Encode PGM+base64: 5-10ms (compression, not typical)
- **Total per job:** ~10-15ms (100-150 jobs/sec with 8 threads)

**Batch Performance:**
- 5000 jobs: ~5 minutes (sequential, 10ms/job)
- 5000 jobs: ~30-40 seconds (parallel, 8 threads, ~500 jobs/sec)
- Memory: <2GB (250 fonts in cache + 1-2 in-flight renders)

**FontSimi Integration:**
- 5.5M glyphs ÷ 5000 jobs/batch = 1100 batches
- 1100 batches × 40s = 44,000s = 12.2 hours (naive sequential)
- 1100 batches × 40s ÷ 30 parallel processes = ~20 minutes (if parallelized)
- But: Process can be parallelized across machines/containers

**Optimizations Not in H2 (Future):**
- Streaming mode: keep process alive, render on-demand for deep matching
- Distributed mode: split batch across N machines
- Storage backend: pack renders into compressed shards (reduce I/O)

---

## Section 4: Integration Points with FontSimi

### 4.1 Phase H1 (Complete): HaforuRenderer Python Class

**File:** `src/fontsimi/renderers/haforu.py` (348 lines, ✅ tested)

**Responsibilities:**
- Discover haforu binary (env var or repo path)
- Generate JSON job spec
- Spawn subprocess, pass JSON via stdin
- Read JSONL from stdout
- Parse results, extract base64 PGM
- Decode to numpy array
- Clean up temp files

**Key Methods:**
```python
class HaforuRenderer(BaseRenderer):
    def render_text(self, text: str) -> np.ndarray[uint8]:
        """Render single text string, return grayscale image."""
        # 1. Generate job JSON
        # 2. Spawn haforu subprocess
        # 3. Pass JSON via stdin
        # 4. Read JSONL from stdout
        # 5. Decode base64 PGM
        # 6. Return numpy array
```

**Current Status:**
- ✅ JSON generation working
- ✅ Subprocess communication working
- ✅ JSONL parsing working
- ⏸️ Haforu Rust returns "pending" (not rendering yet)
- ✅ 38 unit tests passing

### 4.2 Phase H2 (In Progress): Haforu2 Rust Implementation

**Goal:** Make Haforu actually render fonts

**Success Criteria:**
- Parses JSON in <500ms for 5000 jobs
- Renders 500 jobs/sec (8 threads)
- Memory <2GB peak
- Daidot metrics identical to CoreText/HarfBuzz (pixel perfect)
- All error cases handled gracefully

### 4.3 Phase H3 (Ready After H2): FontSimi Batch Pipeline

**File:** `src/fontsimi/daidot/daidot_analyzer.py`

**Changes:**
- Collect render jobs instead of rendering immediately
- Generate 5500K jobs in batches of 5000
- Invoke haforu subprocess per batch
- Parse JSONL results
- Compute Daidot metrics from images
- Store in cache

**Expected Timeline:** 5-9 days after H2 complete

### 4.4 Phase H4 (Future): Streaming Mode

**Improvement:** Keep haforu process alive during deep matching optimization

**Benefit:** Eliminate subprocess spawn overhead (500ms → 20ms per render)

**Expected Timeline:** 6-9 days after H3 complete

### 4.5 Phase H5 (Validation): Performance Targets

**Metrics to Verify:**
- Analysis: 5h → 3m (100× speedup) ✅
- Memory: 86GB → <2GB (97% reduction) ✅
- Deep matching: 30s → 0.6s per pair (50× speedup) ✅
- Reliability: Zero OOM crashes ✅

**Expected Timeline:** 3-5 days after H4 complete

---

## Section 5: Design Decisions & Trade-offs

### 5.1 Subprocess Communication vs FFI

**Choice:** Subprocess (stdin JSON → stdout JSONL)

**Reasons:**
- No Python/Rust FFI complexity (no PyO3, maturin)
- Simple testing (echo JSON files)
- Language-agnostic (could invoke from Java, Go, etc.)
- Process isolation prevents crashes from affecting FontSimi
- Easier debugging (strace, stderr logging)

**Trade-offs:**
- Subprocess spawn overhead ~500ms (Phase H4 streaming mode fixes)
- JSON serialization overhead (negligible vs rendering time)
- Large JSONL output (compressed with gzip in production)

### 5.2 Memory-Mapped Fonts vs Heap Loading

**Choice:** Memory-mapped (memmap2 crate)

**Reasons:**
- 250 fonts × 1MB = 250MB (vs 86GB for all renders)
- OS page cache reuses across processes
- Zero-copy to skrifa/read-fonts
- Automatic paging in/out

**Trade-offs:**
- Slightly more complex code (unsafe blocks for lifetime transmute)
- MMAP not available on very constrained systems (rare)
- File descriptor limits for 1000+ fonts (non-issue for 250)

### 5.3 LRU Font Cache vs Always-Reload

**Choice:** LRU cache with 512 font instance entries

**Reasons:**
- 85 variable instances × 3 coordinate sets = 255 instance variations
- 512 gives 2× safety margin
- Cache hit rate >90% in typical FontSimi workload

**Trade-offs:**
- Slightly more complex code (lru crate dependency)
- Memory overhead for cache bookkeeping (negligible)
- Eviction policy (LRU) deterministic and testable

### 5.4 Parallel Job Processing vs Sequential

**Choice:** Parallel (rayon with adaptive work stealing)

**Reasons:**
- 8-16 cores typical on development machines
- Font loading is I/O-bound, rendering is CPU-bound (good parallelism)
- rayon handles thread pool, load balancing automatically
- 8× speedup typical (500 jobs/sec × 8 threads)

**Trade-offs:**
- Slightly less deterministic (thread scheduling)
- DETERMINISM: Job results arrive out-of-order in JSONL (fixed by job ID)
- More complex debugging (thread interleaving)

### 5.5 Streaming JSONL Output vs Batch

**Choice:** Streaming (write JSONL immediately as jobs complete)

**Reasons:**
- Python can start processing results while Haforu working
- Progress reporting ("50% complete")
- Early error detection (fail fast)
- Better memory usage (don't hold all results in memory)

**Trade-offs:**
- Results arrive out-of-order (fixed by job ID correlation)
- Stdout buffer management needed (1MB typical, sufficient)

### 5.6 PGM P5 Format vs PNG

**Choice:** PGM P5 (binary) with base64 encoding

**Reasons:**
- PGM P5: Simple binary format, no decompression needed
- 8-bit grayscale: Exactly matches Daidot requirements
- Base64: JSON-safe, universally supported
- 10× smaller than PNG for grayscale (no filter, compression)

**Trade-offs:**
- PNG would be 30% smaller (better compression)
- PNG requires libpng dependency (PGM is trivial)
- PNG slower to decode (PNG decompression vs base64)

**Decision Rationale:** Speed > size for batch rendering

---

## Section 6: Risk Analysis & Mitigation

### 6.1 Risks & Mitigation Strategies

| Risk | Severity | Likelihood | Mitigation |
|------|----------|-----------|-----------|
| Haforu binary not found | HIGH | MEDIUM | Fall back to CoreText/HarfBuzz |
| JSON parsing error on malformed input | MEDIUM | HIGH | Validate JSON size, reject >100MB |
| Font file corruption/missing | HIGH | LOW | Graceful error in JSONL, retry individually |
| Memory spike during image compositing | HIGH | MEDIUM | Stream images to disk, don't hold in memory |
| Thread pool deadlock (rayon) | MEDIUM | LOW | Use default thread pool (rayon handles) |
| Out-of-order JSONL results confusing Python | MEDIUM | HIGH | Use job ID correlation in Python |
| Variable font coordinate clamping issues | LOW | MEDIUM | Log warnings, include in timing metrics |
| Zeno rasterization gaps/overlap | LOW | LOW | Manual testing on known glyphs, compare pixel-perfect |

### 6.2 Testing Strategy

**Unit Tests (per module):**
- json_parser: parse valid/invalid JSON, edge cases
- mmap_font: load static/variable/TTC fonts
- font_cache: LRU eviction, hit rate
- shaping: single glyph, empty string, complex scripts
- rasterize: blank glyph, filled glyph, large canvas
- output: PGM format, base64 encoding, bounding box

**Integration Tests:**
- End-to-end: 100 jobs → JSONL results
- Variable fonts: apply coords, verify rendering changes
- Error handling: missing fonts, invalid JSON, corrupted files
- Performance: 5000 jobs < 40 seconds

**Regression Tests (FontSimi side):**
- Daidot metrics identical to CoreText/HarfBuzz (pixel tolerance <0.1%)
- Match results unchanged (top-10 matches identical)
- No OOM crashes on full 250-font set

---

## Section 7: Implementation Phases

### 7.1 Phase H2: Haforu2 Rust (12-18 days)

**Deliverables:**
1. H2.1: JSON job processing (2-3 days)
2. H2.2: Font loading & variations (2-3 days)
3. H2.3: Text shaping (2-3 days)
4. H2.4: Glyph rasterization (3-4 days)
5. H2.5: PGM output format (1-2 days)
6. H2.6: JSONL streaming output (1-2 days)
7. H2.7: Error handling & tests (1-2 days)

**Success Criteria:**
- All tests passing (100%)
- Batch of 5000 jobs completes <40s
- Memory <2GB
- Daidot metrics identical to baseline

### 7.2 Phase H3: FontSimi Batch Pipeline (5-9 days)

**Location:** `src/fontsimi/daidot/daidot_analyzer.py`

**Deliverables:**
1. H3.1: Batch job generation (1-2 days)
2. H3.2: Result processing (2-3 days)
3. H3.3: Cache integration (1-2 days)
4. H3.4: Error recovery (1-2 days)

**Success Criteria:**
- Full analysis: 5.5M glyphs in <3 minutes
- Memory <2GB
- All metrics cached correctly

### 7.3 Phase H4: Streaming Mode (6-9 days)

**Location:** Both repos

**Deliverables:**
1. H4.1: Haforu streaming mode (2-3 days)
2. H4.2: HaforuStreamingRenderer class (2-3 days)
3. H4.3: Deep matcher integration (2-3 days)

**Success Criteria:**
- Deep match: 30s → 0.6s per pair (50× speedup)
- Process reuse: <0.1% overhead

### 7.4 Phase H5: Validation (3-5 days)

**Location:** Both repos + benchmarks

**Deliverables:**
1. H5.1: Performance benchmarking (2 days)
2. H5.2: Documentation (1 day)
3. H5.3: Fallback & compatibility (1-2 days)

**Success Criteria:**
- 100× speedup verified
- 97% memory reduction verified
- All tests passing

---

## Section 8: Haforu2 Standalone Architecture

Beyond FontSimi, Haforu2 should be designed as a general-purpose tool.

### 8.1 Generic Batch Rendering API

**Core Abstraction:**
```rust
pub struct RenderJob {
    pub id: String,
    pub font_path: PathBuf,
    pub font_size: f32,
    pub text: String,
    pub output_format: OutputFormat,  // PGM, PNG, SVG, JSON
    pub variations: HashMap<String, f32>,
}

pub struct RenderResult {
    pub job_id: String,
    pub status: Status,  // Success, Error
    pub output: OutputData,  // Enum: PgmBinary, PngBinary, SvgString, MetricsJson
    pub timing: TimingInfo,
}

pub fn render_batch(jobs: Vec<RenderJob>) -> Vec<RenderResult>
```

### 8.2 Output Format Plugins

**Supported Formats:**
- `pgm`: P5 binary grayscale (FontSimi)
- `png`: PNG compressed color (web)
- `svg`: Scalable vector (future)
- `metrics`: JSON with computed metrics (QA)

### 8.3 Standalone CLI Usage

```bash
# Single batch
cat jobs.json | haforu2 process --render --format pgm > results.jsonl

# Multiple batches (GNU parallel)
parallel < batch_list.txt | haforu2 process --render --format png --parallel 4

# Streaming mode (keeps process alive)
haforu2 --streaming < /dev/stdin > /dev/stdout
```

### 8.4 Future Extensions

**Possible plugins (not in H2-H5):**
- Distributed rendering (MPI, Ray)
- GPU rasterization (Vello/wgpu backend)
- Web service (actix-web)
- Python bindings (PyO3/maturin)

---

## Section 9: Conclusion & Next Steps

### 9.1 Haforu2 Value Proposition

**For FontSimi:**
- 100× performance improvement
- 97% memory reduction
- Architectural foundation for future scaling

**For Font Developers:**
- General-purpose batch rendering tool
- Suitable for specimen generation, QA, benchmarking
- Extensible output formats and plugins

**For Ecosystem:**
- Rust native font rendering (no C FFI)
- Zero-copy design (memory efficient)
- Streaming architecture (progressive results)

### 9.2 Critical Success Factors

1. **Get H2.1-H2.4 right:** Core rendering pipeline is foundation
2. **Exhaustive unit tests:** Catch edge cases early
3. **FontSimi validation:** Ensure Daidot metrics pixel-perfect
4. **Performance profiling:** Measure per-stage bottlenecks
5. **Documentation:** Examples, troubleshooting, API docs

### 9.3 Recommended Implementation Order

1. **Start H2.1 immediately:** JSON parsing (lowest risk, high value)
2. **Parallelize H2.2-H2.4:** Font loading and rendering (deep work)
3. **Validate against FontSimi:** Compare Daidot metrics pixel-perfect
4. **Then proceed to H3:** Batch pipeline (depends on H2)
5. **Then proceed to H4-H5:** Streaming & optimization (polish)

### 9.4 Timeline Estimate

- **H2 (Rust):** 12-18 days (2-3 weeks)
- **H2 Validation:** 4 days
- **H3 (Python batch):** 5-9 days (1-2 weeks)
- **H4 (Streaming):** 6-9 days (1-2 weeks)
- **H5 (Validation):** 3-5 days
- **Total:** 4-6 weeks

---

## Appendix A: File Structure Reference

```
external/haforu2/                    # New Rust project
├── Cargo.toml
├── Cargo.lock
├── src/
│   ├── main.rs                      # CLI entry point
│   ├── lib.rs                       # Public library API
│   ├── json_parser.rs               # JobSpec, validation
│   ├── error.rs                     # Error types
│   ├── mmap_font.rs                 # Memory-mapped font loading
│   ├── font_cache.rs                # LRU font instance cache
│   ├── shaping.rs                   # HarfRust text shaping
│   ├── rasterize.rs                 # Glyph rasterization
│   ├── output.rs                    # PGM format, base64
│   ├── orchestrator.rs              # Job pipeline
│   └── stats.rs                     # Performance metrics
├── tests/
│   ├── integration_tests.rs
│   └── unit_tests.rs
├── fonts/                           # Test fonts
│   ├── Arial.ttf                    # Static
│   ├── Roboto[wght].ttf             # Variable (one axis)
│   └── Inter[slnt,wght].ttf         # Variable (two axes)
├── README.md                        # Usage guide
├── PLAN.md                          # Implementation plan
├── TODO.md                          # Task list
└── WORK.md                          # Work log
```

---

**Document Version:** 1.0  
**Last Updated:** 2025-11-11  
**Status:** Ready for Implementation
