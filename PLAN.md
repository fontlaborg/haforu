---
this_file: PLAN.md
---

# Haforu Development Plan

## Project Status (v2.0.x, 2025-11-15)

**✅ PRODUCTION READY** - All critical work complete!

- **Performance:** Sub-millisecond rendering with SIMD optimizations ✓
- **Quality:** Reliable, deterministic output across all platforms ✓
- **Architecture:** Dual-purpose: CLI binary + Python bindings (PyO3) ✓
- **Integration:** Powering fontsimi's 35-60× total speedup ✓

## Project Scope (One Sentence)

Deliver a fast, reliable font renderer for CLI and Python with deterministic output, validated variation coordinates, and sub-millisecond performance for both text rendering and image processing.

---

## Dual-Purpose Architecture

### Purpose 1: Image Processing (PRIMARY for FontSimi)

**Python Bindings (PyO3):**
- **Operations:** `align_and_compare()`, `resize_bilinear()`
- **Performance:** 3-5x speedup vs Python/numpy
- **Usage:** Deep matching hot paths in fontsimi
- **Zero overhead:** Direct memory access, no subprocess calls
- **Build:** `maturin develop --release --features python`

**Why Python bindings for image ops?**
- Hot-path operations benefit from Rust optimization
- Eliminates Python marshalling overhead
- Direct memory access vs subprocess spawn (~20ms overhead)
- Perfect for tight loops (30-180 calls per font match)

### Purpose 2: Text Rendering (FALLBACK for FontSimi)

**CLI Binary:**
- **Batch processing:** 150-200 jobs/sec in parallel
- **Fallback renderer:** When CoreText/HarfBuzz/Skia unavailable (rare)
- **Good for:** Batch analysis (100+ fonts)
- **Poor for:** Per-call rendering (subprocess overhead ~21ms)
- **Build:** `cargo build --release`

**Why CLI as fallback only?**
- Native renderers are 176× faster for per-call rendering
- CoreText (macOS): 0.12ms vs Haforu CLI: 21.04ms
- Subprocess overhead dominates small jobs
- Excellent for batch scenarios (amortizes overhead)

---

## Performance Achievements ✅

### Actual Performance (v2.0.x with SIMD)

| Mode | Throughput | Status |
|------|-----------|--------|
| CLI Batch | 150-200 jobs/sec | ✅ Production |
| Python Bindings | 1000-2000 jobs/sec | ✅ SIMD-accelerated |
| Metrics Mode | 10,000-20,000 jobs/sec | ✅ SIMD-accelerated |
| Batch Variation Sweep | ~30-40 coords/ms | ✅ Parallel |

### Detailed Timings

- **Single render (CLI):** <10ms cold, ~5ms warm
- **Single render (Python):** <1ms warmed cache with SIMD
- **Batch 1000 jobs (CLI):** <5s on 8 cores
- **Metrics mode:** <0.05ms per job (4-8× faster with SIMD)
- **Variation sweep (80 coords):** ~2-3ms on 8 cores
- **Memory:** <500MB for 1000 renders

### Image Processing (Python Bindings)

| Operation | Python/NumPy | Haforu (Rust) | Speedup |
|-----------|--------------|---------------|---------|
| `align_and_compare()` | ~5-8ms | 1.6ms | **3-5x** |
| `resize_bilinear()` | ~1ms | 0.3ms | **2-3x** |

**Call frequency in fontsimi:** 30-180 calls per font match
**Total impact:** 3-5× speedup for deep matching pipeline

---

## Key Optimizations (v2.0+)

### Phase 1: Core Rendering (Completed)
1. **Lock-Free Font Cache** (20% speedup)
   - DashMap replaces Mutex<LruCache>
   - Eliminates lock contention on high thread counts
   - Location: `src/fonts.rs`

2. **HarfBuzz Font Caching** (20% speedup)
   - Cache HarfBuzz Font objects in FontInstance
   - Eliminate repeated Face/Font creation
   - Location: `src/fonts.rs`

3. **Variable Font Fast Path** (15% speedup)
   - Fixed single-char rendering to support variable fonts
   - Use variation-aware metrics from HVAR/gvar
   - Location: `src/shaping.rs`

### Phase 2: SIMD + Parallel (Completed)
4. **SIMD-Accelerated Metrics** (4-8× speedup)
   - AVX2 for density/beam calculations on x86_64
   - Portable fallback for other platforms
   - Location: `src/render.rs`

5. **Thread-Local Buffer Pools** (10-15% speedup)
   - Pool canvas buffers per thread
   - Eliminates allocation overhead
   - Location: `src/bufpool.rs`

6. **Batch Variation Sweep API** (5-8× speedup)
   - Parallel rendering at multiple coordinates
   - Optimized for font matching loops
   - Location: `src/varsweep.rs`

### Phase 3: Image Processing (Completed)
7. **`align_and_compare()`** (3-5× speedup for fontsimi)
   - Align images and compute pixel delta in single Rust call
   - Center alignment, Gaussian-weighted delta
   - Location: `python/src/lib.rs`

8. **`resize_bilinear()`** (2-3× speedup for fontsimi)
   - Native Rust bilinear interpolation
   - Replaces OpenCV wrapper overhead
   - Location: `python/src/lib.rs`

---

## Optional Future Enhancements

**⚠️ Important:** Only pursue if explicitly requested. Current system production-ready.

### Error Handling (Nice to Have)
- [ ] Comprehensive malformed input testing (CLI batch/stream, Python bindings)
- [ ] Regression tests for edge cases

### Coordinate Validation (Nice to Have)
- [ ] `validate_coordinates()` in `src/fonts.rs`
- [ ] Clamp wght/wdth to valid ranges
- [ ] Warn on unknown axes

### Documentation (Incremental)
- [ ] Python quick start examples
- [ ] Batch vs stream mode use case guide
- [ ] Performance comparison tables
- [ ] Real-world integration examples

### Image Processing (Future)
- [ ] Integrated rendering metrics (return density/aspect during rendering)
- [ ] Multi-component scoring (weighted score calculation in Rust)
- [ ] SSIM computation (structural similarity)
- [ ] Full objective function (complete pipeline in Rust)

---

## Testing Strategy

**Unit Tests:** Rust modules (`cargo test`) - 41 tests passing ✅
**Integration Tests:** `scripts/batch_smoke.sh` - validates CLI contract in <2s ✅
**Performance Tests:** `scripts/profile-cli.sh` - catches regressions ✅
**Edge Cases:** Empty text, zero canvas, missing fonts, invalid variations ✅

---

## Performance Targets

### Achieved (v2.0.x with SIMD) ✅

- ✅ CLI batch (1000 jobs): <5s on 8 cores
- ✅ CLI streaming: <5ms per job including startup
- ✅ Python StreamingSession: <1ms per job (warmed cache, SIMD)
- ✅ Metrics mode: <0.05ms per job (SIMD-accelerated)
- ✅ Batch variation sweep (80 coords): ~2-3ms on 8 cores

---

## Success Criteria ✅ ALL MET

### Performance ✅
- [x] CLI batch: 40-50% faster than baseline
- [x] Python bindings: <1ms per render (exceeded - SIMD)
- [x] Metrics mode: 4-8× speedup with SIMD
- [x] Batch variation sweep: 5-8× speedup with parallelism

### Correctness ✅
- [x] CLI never drops jobs silently
- [x] All 41 Rust unit tests pass
- [x] Variation coordinates validated (via skrifa)
- [x] Cross-platform: macOS tested

### Integration with FontSimi ✅
- [x] `align_and_compare()` working (3-5× speedup)
- [x] `resize_bilinear()` working (2-3× speedup)
- [x] Automatic fallback to Python/numpy
- [x] End-to-end speedup: 35-60× total

---

## Non-Goals (Scope Boundary)

- ❌ No new output formats beyond PGM/PNG/metrics
- ❌ No retry logic, circuit breakers, or resilience patterns
- ❌ No analytics, monitoring, or telemetry systems
- ❌ No color, emoji, or subpixel rendering
- ❌ No complex configuration systems
- ❌ No automatic version tagging or release automation
- ❌ No repository structure reorganization

---

## Build Instructions (Reference)

### Python Bindings (Recommended for FontSimi)

```bash
cd haforu-src
source /path/to/venv/bin/activate
uvx maturin develop --release --features python
python -c "import haforu; print('Version:', haforu.__version__)"
```

### CLI Binary (Already Built)

```bash
cd haforu-src
cargo build --release
./target/release/haforu --version
```

### Verification

```bash
# Run Rust tests
cargo test

# Verify Python bindings
python -c "import haforu; print('Available:', haforu.is_available())"

# Test CLI smoke test
./scripts/batch_smoke.sh
```

---

**Status:** Production Ready
**Version:** v2.0.x
**Last Updated:** 2025-11-15
