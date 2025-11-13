---
this_file: external/haforu2/TODO.md
---

# Haforu2 Implementation TODO

**Why This Matters:** FontSimi **cannot** analyze 1,000 fonts without Haforu - would require 344GB RAM (impossible)

**Current Status:** H2.1 Complete ✅ - All 6 modules working, 23/23 tests passing
**Critical Blocker REMOVED:** All 25 compilation errors fixed (completed in 4-6 hours)
**This Unlocked:** 15-28 days of H2.2-H5 work, making 1,000 font analysis feasible
**Timeline:** 7-10 days for complete H2 validation + integration

---

## H0 — Package naming and structure (required before release)

Goal: All published artifacts must be named `haforu` (no "2"). The on-disk folder here is `haforu2/` during development, but code, manifests, bindings, tests, CI and docs must reflect the canonical name.

- [ ] Cargo.toml renames
  - [ ] `[package].name = "haforu"`
  - [ ] `[lib].name = "haforu"`
  - [ ] `[[bin]].name = "haforu"`
- [ ] pyproject.toml renames
  - [ ] `[project].name = "haforu"`
  - [ ] `[tool.maturin].module-name = "haforu._haforu"`
- [ ] Python package path: `python/haforu/` (update .gitignore accordingly)
- [ ] README & CLI docs: replace `haforu2` → `haforu`
- [ ] Tests/scripts/CI: replace `haforu2` → `haforu`
- [ ] Verify `haforu` binary discovery in `fontsimi` (env var, repo-relative, PATH)

---

## H2 — Core Rendering Implementation ⚡ START HERE

**Goal:** Implement complete rendering pipeline with batch processing

**Location:** `external/haforu2/src/`

**Status:** Foundation complete ✅

### Foundation (Complete) ✅

- [x] Cargo.toml with all dependencies
- [x] Module structure (batch, fonts, shaping, render, output, error)
- [x] Error types with descriptive messages
- [x] JobSpec/Job/JobResult data structures with validation
- [x] FontLoader with memory-mapped loading and LRU caching
- [x] TextShaper with HarfBuzz integration
- [x] GlyphRasterizer with skrifa + zeno
- [x] ImageOutput with PGM/PNG and base64 encoding
- [x] CLI with batch and streaming modes
- [x] Unit tests for all modules
- [x] README.md with documentation

### H2.1: Fix API Errors & Validate Foundation ✅ COMPLETE

**BLOCKER REMOVED ✅** - All 25 compilation errors fixed in 4-6 hours as estimated

**File:** `external/haforu2/src/`

**Final State:**
- ✅ All 6 modules complete: error.rs, batch.rs, output.rs, fonts.rs, shaping.rs, render.rs
- ✅ 23/23 tests passing (22 lib + 0 main + 1 doc)
- ✅ Clean debug and release builds (0 errors, 1 benign warning)

**Completed Tasks:**

- [x] Fix fonts.rs API mismatches (1-2 hours)
  - [x] Fix skrifa font loading API calls (6 errors)
  - [x] Fix glyph_metrics API usage
  - [x] Fix outline extraction API
  - [x] Verify all imports resolve correctly
  - [x] Added font_data() method for raw bytes

- [x] Fix shaping.rs API mismatches (1-2 hours)
  - [x] Switch harfbuzz 0.6 → harfbuzz_rs 2.0 (critical fix)
  - [x] Fix Font creation from blob
  - [x] Fix shaping API calls
  - [x] Fix buffer ownership chaining
  - [x] Ensure all dependencies compile

- [x] Fix render.rs API mismatches (2-3 hours)
  - [x] Fix zeno path building API (11 errors)
  - [x] Fix rasterization API calls (Mask creation/rendering)
  - [x] Fix glyph composition (alpha blending)
  - [x] Fixed Point/Vector conversions
  - [x] Fixed Placement bounds handling

- [x] Run `cargo build` and verify clean compilation
  - [x] All 25 errors resolved
  - [x] No new warnings (1 benign unused import warning)

- [x] Run `cargo test` and verify all tests pass
  - [x] error.rs: 3 tests ✅
  - [x] batch.rs: 5 tests ✅
  - [x] output.rs: 7 tests ✅
  - [x] fonts.rs: 3 tests ✅ (now passing after fixes)
  - [x] shaping.rs: 2 tests ✅ (now passing after fixes)
  - [x] render.rs: 3 tests ✅ (now passing after fixes)
  - [x] Total: 23/23 tests passing (100%)

### H2.2: Integration Testing (2-3 days)

**File:** `tests/integration_tests.rs` (new file)

- [ ] Create integration test with real font file
  - [ ] Use `test-fonts/Arial-Black.ttf` from FontSimi
  - [ ] Create minimal job spec with single job
  - [ ] Invoke `process_job()` function
  - [ ] Verify result status is "success"

- [ ] Test complete pipeline: JSON → render → JSONL
  - [ ] Parse JobSpec from JSON string
  - [ ] Process all jobs
  - [ ] Verify JSONL output format
  - [ ] Decode base64 PGM images
  - [ ] Verify image dimensions match request

- [ ] Test variable font with coordinates
  - [ ] Use Playfair variable font
  - [ ] Apply wght=600, wdth=100 coordinates
  - [ ] Verify rendering succeeds
  - [ ] Compare against static font rendering

- [ ] Test error handling
  - [ ] Missing font file
  - [ ] Invalid coordinates (out of bounds)
  - [ ] Empty text content
  - [ ] Unsupported output format

- [ ] Test batch of 100 jobs
  - [ ] Generate 100 jobs programmatically
  - [ ] Measure processing time (<10s target)
  - [ ] Verify all results received
  - [ ] Check cache hit rate

### H2.3: FontSimi Compatibility Testing (1-2 days)

**Files:** `tests/fontsimi_compat_tests.rs` (new file), Python side validation

- [ ] Test exact FontSimi job format
  - [ ] Copy job JSON from HaforuRenderer
  - [ ] Process with haforu2
  - [ ] Verify JSONL output matches expected format

- [ ] Test base64 PGM decoding in Python
  - [ ] Create Python test script
  - [ ] Decode base64 from JSONL
  - [ ] Parse PGM P5 format
  - [ ] Convert to numpy array
  - [ ] Verify image dimensions and pixel values

- [ ] Compare rendering with CoreText/HarfBuzz
  - [ ] Render same glyph with haforu2 and CoreText
  - [ ] Compare pixel-by-pixel (tolerance <0.1%)
  - [ ] Verify Daidot metrics are identical
  - [ ] Document any differences

- [ ] Performance baseline
  - [ ] Single render: measure time (<100ms target)
  - [ ] Batch of 1000: measure time (<10s target)
  - [ ] Memory usage: measure peak RSS (<500MB target)

### H2.4: CLI Testing (1 day)

**Files:** Manual testing with command line

- [ ] Test batch mode from command line
  ```bash
  echo '{"version":"1.0","jobs":[...]}' | cargo run -- batch
  ```

- [ ] Test streaming mode from command line
  ```bash
  echo '{"id":"test1",...}' | cargo run -- stream
  ```

- [ ] Test with invalid input
  - [ ] Malformed JSON
  - [ ] Missing required fields
  - [ ] Invalid dimensions

- [ ] Test CLI flags
  - [ ] `--cache-size 1024`
  - [ ] `--workers 4`
  - [ ] `--verbose`

- [ ] Test output to file
  ```bash
  cargo run -- batch < jobs.json > results.jsonl 2> debug.log
  ```

  - [ ] Add script/direction hinting in shaping (uses HarfBuzz buffer properties).
  - [ ] Implement basic dedup/grouping in batch (e.g., group by font+coords to reuse shaped glyphs between lines).
  - [ ] Revisit storage shards only if your pipeline benefits from persistent image addressing.


### H2.5: Documentation (1 day)

**Files:** README.md, TODO.md, docs/

- [ ] Update README.md with actual usage examples
  - [ ] Add real command line examples
  - [ ] Add performance benchmarks
  - [ ] Document known limitations

- [ ] Create TESTING.md
  - [ ] Unit test strategy
  - [ ] Integration test guide
  - [ ] Performance testing procedures

- [ ] Create INTEGRATION.md
  - [ ] FontSimi integration guide
  - [ ] Job format specification
  - [ ] JSONL output format
  - [ ] Error handling guidelines

- [ ] Create TROUBLESHOOTING.md
  - [ ] Common compilation errors
  - [ ] Runtime error messages
  - [ ] Performance tuning tips

### H2.6: Production Readiness (1 day)

**Files:** All source files

- [ ] Code quality
  - [ ] Run `cargo clippy` and fix all warnings
  - [ ] Run `cargo fmt` to format code
  - [ ] Add missing `#[must_use]` annotations
  - [ ] Remove any unused code

- [ ] Error handling audit
  - [ ] Verify all `Result` types are handled
  - [ ] Ensure no `.unwrap()` in production code
  - [ ] Add context to all error messages
  - [ ] Test all error paths

- [ ] Performance optimization
  - [ ] Profile critical hot paths
  - [ ] Optimize memory allocations
  - [ ] Verify cache hit rates
  - [ ] Document any known bottlenecks

- [ ] Security audit
  - [ ] Check for unsafe code correctness
  - [ ] Verify input validation (sizes, paths)
  - [ ] Test with malicious inputs
  - [ ] Document security assumptions

---

## H3 — FontSimi Batch Pipeline Integration (AFTER H2)

**Goal:** Use haforu2 from FontSimi Python code

**Location:** `../../src/fontsimi/` (Python)

**Status:** BLOCKED until H2 complete

### H3.1: Batch Job Generation (1-2 days)

**File:** `src/fontsimi/daidot/daidot_analyzer.py`

- [ ] Modify DaidotAnalyzer to collect jobs
- [ ] Generate batch JSON specification
- [ ] Implement job collection workflow
- [ ] Test batch generation with 2 fonts

### H3.2: Batch Execution & Result Processing (2-3 days)

**File:** `src/fontsimi/daidot/daidot_analyzer.py`

- [ ] Implement batch execution method
- [ ] Stream JSONL results line-by-line
- [ ] Decode base64 PGM images
- [ ] Compute Daidot metrics from rendered images

### H3.3: Cache Integration (1-2 days)

**File:** `src/fontsimi/cache.py`

- [ ] Store batch results in cache
- [ ] Maintain backward compatibility
- [ ] Test cache round-trip

### H3.4: Error Recovery & Progress (1-2 days)

**File:** `src/fontsimi/daidot/daidot_analyzer.py`

- [ ] Implement per-job error handling
- [ ] Retry failed jobs individually
- [ ] Report progress (X/Y jobs complete)

---

## H4 — Streaming Mode (AFTER H3)

**Goal:** Keep haforu2 process alive for deep matching

**Location:** Both repos

**Status:** BLOCKED until H3 complete

### H4.1: Streaming Mode Implementation (Haforu2 Rust)

**File:** `src/main.rs`

- [ ] Already implemented! ✅
- [ ] Test streaming mode thoroughly
- [ ] Verify font cache persists across jobs
- [ ] Handle EOF gracefully

### H4.2: Streaming Renderer Client (FontSimi Python)

**File:** `src/fontsimi/renderers/haforu_streaming.py` (new)

- [ ] Create HaforuStreamingRenderer class
- [ ] Launch persistent haforu2 subprocess
- [ ] Implement LRU cache for rendered images
- [ ] Handle process crashes/restarts

### H4.3: Deep Matcher Integration (FontSimi Python)

**File:** `src/fontsimi/matcher/deep_optimization.py`

- [ ] Replace per-call rendering with streaming
- [ ] Maintain single haforu2 process per match
- [ ] Test deep match performance

---

## H5 — Performance Validation (AFTER H4)

**Goal:** Verify 100× speedup and <2GB memory

**Location:** `benchmarks/` (new directory)

**Status:** BLOCKED until H4 complete

### H5.1: Benchmarking

**File:** `benchmarks/benchmark_haforu2.py` (new)

- [ ] Benchmark analysis phase (target: <3 min)
- [ ] Benchmark deep matching (target: <1s per pair)
- [ ] Measure memory usage (target: <2GB)
- [ ] Compare metrics vs baseline

### H5.2: Documentation & Migration

**Files:** README.md, docs/

- [ ] Update README with performance claims
- [ ] Create migration guide v2 → v3+haforu2
- [ ] Add troubleshooting guide

### H5.3: Fallback & Compatibility

**File:** `src/fontsimi/renderers/__init__.py`

- [ ] Ensure `--renderer=auto` falls back gracefully
- [ ] Maintain all existing renderers
- [ ] Document fallback behavior

---

## Success Criteria (H2 Complete)

- [ ] All Rust tests passing (30+ tests)
- [ ] Batch of 1000 jobs completes <10 seconds
- [ ] Memory usage <500MB for 1000 renders
- [ ] Integration tests with real fonts pass
- [ ] JSONL output format matches FontSimi expectations
- [ ] All error cases handled gracefully
- [ ] Documentation complete with examples

## Success Criteria (H3-H5 Complete)

- [ ] Analysis: 5 hours → 3 minutes (100× speedup) ✅
- [ ] Memory: 86GB → <2GB (97% reduction) ✅
- [ ] Deep Matching: 30s → 0.6s per pair (50× speedup) ✅
- [ ] Daidot metrics identical to baseline (<0.1% tolerance) ✅
- [ ] All 250 fonts × 85 instances process successfully ✅
- [ ] Zero OOM crashes ✅

---

## IMMEDIATE NEXT STEPS ⚡

**Current Status:** Foundation 25% complete - API fixes needed before any testing

**CRITICAL:** 4-6 hours of API fixes unlocks 7-10 days of H2 work, then 15-28 days of H3-H5

### This Week (4-6 hours) - THE BLOCKER
1. **Fix API errors in fonts.rs, shaping.rs, render.rs** (see H2.1 above)
   - skrifa API: font loading, metrics, outline extraction (1-2h)
   - harfbuzz API: Blob/Font creation, shaping calls (1-2h)
   - zeno API: path building, rasterization (2-3h)
2. **Verify with `cargo build`** - all 25 errors resolved
3. **Verify with `cargo test`** - all 23 tests passing

### After API Fixes (7-10 days)
4. H2.2: Integration testing with real fonts (2-3 days)
5. H2.3: FontSimi compatibility validation (1-2 days)
6. H2.4: CLI testing and smoke tests (1 day)
7. H2.5: Documentation updates (1 day)
8. H2.6: Production readiness audit (1 day)

**Timeline:** 7-10 days for complete H2 implementation after API fixes

**Critical Path:**
```
H2.1 API fixes (4-6h) → H2.2-H2.6 (7-10d) → H3-H5 (15-28d)
         ↑
  BLOCKING POINT
```

**Blocking:** All H2.2-H5 work blocked on H2.1 API fixes
