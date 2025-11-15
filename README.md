---
this_file: README.md
---

# Haforu: Fast Font Renderer

**Status:** Production Ready (v2.0.x)

Fast, deterministic font rendering with dual-purpose architecture: Python bindings for image processing (PRIMARY) and CLI binary for batch/fallback text rendering.

---

## What It Does

Haforu serves **two complementary purposes**:

### 1. Image Processing (PRIMARY for FontSimi)

**Python bindings (PyO3)** provide Rust-optimized image operations:
- `align_and_compare()` - Align images and compute pixel delta (3-5× faster than Python/numpy)
- `resize_bilinear()` - Bilinear image scaling (2-3× faster than OpenCV)
- **Zero overhead:** Direct memory access, no subprocess calls
- **Perfect for:** Hot-path operations in tight loops (30-180 calls per font match)

### 2. Text Rendering (FALLBACK for FontSimi)

**CLI binary** provides batch processing and fallback rendering:
- Render glyphs to PGM/PNG/metrics with sub-millisecond performance
- **Good for:** Batch processing (100+ jobs), amortizes subprocess overhead
- **Poor for:** Per-call rendering (21ms overhead vs 0.12ms for CoreText)
- **Use when:** Native renderers (CoreText/HarfBuzz/Skia) unavailable (rare)

---

## Why Both Backends?

**They serve complementary purposes - NOT competing!**

| Use Case | Backend | Performance | When to Use |
|----------|---------|-------------|-------------|
| **Image processing** | Python bindings | 1.6ms | ✅ PRIMARY for fontsimi (hot paths) |
| **Text rendering** | Native (CoreText) | 0.12ms | ✅ PRIMARY for fontsimi (text) |
| **Text rendering** | Haforu CLI | 21ms | ⚠️ FALLBACK only (native unavailable) |
| **Batch processing** | Haforu CLI | 150-200 jobs/sec | ✅ Good for batch (100+ fonts) |

**Key insight:** Subprocess overhead (~21ms) makes CLI poor for per-call rendering, but excellent for batch. Python bindings have zero subprocess overhead, perfect for hot paths.

---

## Install

### Python Bindings (Recommended for FontSimi)

```bash
# Using pip (if published)
pip install haforu

# Or build from source
cd haforu-src
source /path/to/venv/bin/activate
uvx maturin develop --release --features python

# Verify installation
python -c "import haforu; print('Version:', haforu.__version__)"
```

### CLI Binary

```bash
# Using cargo (if published)
cargo install haforu

# Or build from source
cd haforu-src
cargo build --release
./target/release/haforu --version
```

### Requirements

- **For Python bindings:** Python 3.8+, Rust 1.70+ (build-time only)
- **For CLI binary:** Rust 1.70+
- **Runtime:** No compiler needed for pip-installed wheels

---

## Quick Start

### Python Bindings (Image Processing)

```python
import haforu
import numpy as np

# Image alignment and comparison (3-5× faster than Python)
img1 = np.random.randint(0, 256, (200, 1200), dtype=np.uint8)
img2 = np.random.randint(0, 256, (200, 1200), dtype=np.uint8)

result = haforu.align_and_compare(img1, img2, method="center")
print(f"Pixel delta: {result.pixel_delta:.4f}")
print(f"Center-weighted delta: {result.center_weighted_delta:.4f}")

# Image scaling (2-3× faster than OpenCV)
scaled = haforu.resize_bilinear(img1, multiplier=0.5)
print(f"Scaled to: {scaled.width}×{scaled.height}")

# Text rendering with streaming session (<1ms per render)
import json

with haforu.StreamingSession(max_fonts=512, max_glyphs=2048) as session:
    # Render to JSON
    job = {
        "id": "test",
        "font": {"path": "/path/to/font.ttf", "size": 1000},
        "text": {"content": "A"},
        "rendering": {"format": "pgm", "encoding": "base64", "width": 3000, "height": 1200}
    }
    result = session.render(json.dumps(job))

    # Or render directly to numpy array (zero-copy)
    image = session.render_to_numpy(
        font_path="/path/to/font.ttf",
        text="A",
        size=1000.0,
        width=3000,
        height=1200,
        variations={"wght": 600.0}
    )
    # Returns numpy.ndarray, shape (height, width), dtype uint8
```

### CLI Binary (Text Rendering)

```bash
# Single render
haforu render -f font.ttf -t "Hello" -s 72 -o output.pgm

# Batch processing (best use case for CLI)
echo '{
  "version": "1.0",
  "jobs": [{
    "id": "test1",
    "font": {"path": "/path/to/font.ttf", "size": 1000},
    "text": {"content": "A"},
    "rendering": {"format": "pgm", "encoding": "base64", "width": 3000, "height": 1200}
  }]
}' | haforu batch > results.jsonl

# Streaming mode (line-by-line JSONL)
haforu stream < jobs.jsonl > results.jsonl

# Metrics only (10,000-20,000 jobs/sec)
haforu render -f font.ttf -t "A" -s 256 --format metrics
```

---

## Architecture

### Rendering Pipeline

```
Input: Font + Text + Parameters
  ↓
FontLoader (memory-mapped, LRU cached)
  ↓
HarfBuzz Shaping
  ↓
Zeno Rasterization
  ↓
Output: PGM/PNG/Metrics JSON
```

**Key Features:**
- **Memory-mapped fonts:** Zero-copy loading via memmap2
- **LRU font cache:** 512 entries default (DashMap - lock-free)
- **Parallel batch processing:** Rayon for multi-core
- **Deterministic JSONL I/O:** Every error returns JSON, no crashes
- **SIMD optimizations:** AVX2 on x86_64, portable fallback

### Python Bindings (PyO3)

**Image Processing Operations:**

```python
# align_and_compare(img_a, img_b, method="center")
# → AlignCompareResult {
#     aligned_a, aligned_b,  # Aligned images
#     pixel_delta,           # Mean absolute difference
#     center_weighted_delta, # Gaussian-weighted delta
#     density_a, density_b,  # Ink density
#     aspect_a, aspect_b     # Width/height ratio
#   }

# resize_bilinear(image, multiplier)
# → ResizeResult { image, width, height }
```

**Text Rendering:**
- `StreamingSession` - Persistent session with font/glyph caching
- `render()` - Render job, return JSONL result
- `render_to_numpy()` - Render directly to numpy array (zero-copy)
- `warm_up()`, `ping()`, `cache_stats()` - Session management

---

## Performance

### Actual Performance (v2.0.x with SIMD)

| Mode | Throughput | Use Case |
|------|-----------|----------|
| **CLI Batch** | 150-200 jobs/sec | ✅ Batch analysis (100+ fonts) |
| **Python Bindings** | 1000-2000 jobs/sec | ✅ Hot paths (SIMD-accelerated) |
| **Metrics Mode** | 10,000-20,000 jobs/sec | ✅ Feature extraction (SIMD) |
| **Variation Sweep** | ~30-40 coords/ms | ✅ Font matching optimization |

### Detailed Timings

**CLI:**
- Single render: <10ms cold, ~5ms warm
- Batch (1000 jobs): <5s on 8 cores (40-50% faster than baseline)
- Memory: <500MB for 1000 renders

**Python Bindings:**
- Single render: <1ms warmed cache with SIMD
- Metrics mode: <0.05ms per job (4-8× faster with SIMD)
- Variation sweep (80 coords): ~2-3ms on 8 cores
- Image alignment: 1.6ms (was ~5-8ms in Python/numpy)
- Image scaling: 0.3ms (was ~1ms in OpenCV)

### Key Optimizations

1. **SIMD-Accelerated Metrics** (4-8× speedup)
   - AVX2 for density/beam calculations on x86_64
   - Portable fallback for other platforms

2. **Lock-Free Font Cache** (20% speedup)
   - DashMap eliminates lock contention
   - Scales linearly on high thread counts

3. **Thread-Local Buffer Pools** (10-15% speedup)
   - Eliminates allocation overhead in tight loops

4. **Batch Variation Sweep** (5-8× speedup)
   - Parallel rendering across cores
   - Optimized for font matching (30-180 calls per font)

5. **HarfBuzz Font Caching** (20% speedup)
   - Caches HarfBuzz Font objects
   - Eliminates repeated Face/Font creation

---

## CLI Commands

### batch

Read full JSON, process in parallel:

```bash
haforu batch [--max-fonts N] [--max-glyphs N] [--jobs N] [--stats]
```

### stream

Line-by-line JSONL processing:

```bash
haforu stream [--max-fonts N] [--max-glyphs N]
```

### render

Single text render (HarfBuzz-compatible):

```bash
haforu render -f FONT -t TEXT [-s SIZE] [-o OUTPUT] [--format pgm|png|metrics]
```

### diagnostics

Show system info and defaults:

```bash
haforu diagnostics [--format text|json]
```

### validate

Check job specification:

```bash
haforu validate < jobs.json
```

**Cache Tuning:**
- `--max-fonts N` - Font cache size (default: 512)
- `--max-glyphs N` - Glyph cache size (default: 2048, 0 to disable)
- `--jobs N` - Parallel workers (default: CPU count)

---

## Python API

### `haforu.align_and_compare(img_a, img_b, method="center")`

Align images and compute pixel delta (3-5× faster than Python/numpy).

**Parameters:**
- `img_a, img_b` - numpy arrays (uint8, 2D)
- `method` - Alignment method: "center" (default)

**Returns:** `AlignCompareResult` with aligned images, pixel delta, density, aspect

### `haforu.resize_bilinear(image, multiplier)`

Bilinear image scaling (2-3× faster than OpenCV).

**Parameters:**
- `image` - numpy array (uint8, 2D)
- `multiplier` - Scale factor (float)

**Returns:** `ResizeResult` with scaled image, width, height

### `haforu.StreamingSession(max_fonts=512, max_glyphs=2048)`

Persistent rendering session with font/glyph caching.

**Methods:**
- `render(job_json: str) -> str` - Render job, return JSONL result
- `render_to_numpy(...) -> np.ndarray` - Render directly to numpy array (zero-copy)
- `warm_up(font_path: str) -> bool` - Warm up cache
- `ping() -> bool` - Liveness check
- `cache_stats() -> dict` - Inspect cache state
- `set_cache_size(n: int)` - Resize font cache
- `set_glyph_cache_size(n: int)` - Resize glyph cache
- `close()` - Release resources

### `haforu.is_available() -> bool`

Check if native extension is available.

### Batch Variation Sweep API

Optimized for font matching - render same glyph at multiple variation coordinates in parallel.

```python
from haforu.varsweep import SweepConfig, render_variation_sweep
from haforu import FontLoader, ExecutionOptions

# Generate variation coordinates
coord_sets = []
for wght in range(100, 950, 50):
    coord_sets.append({"wght": float(wght)})

config = SweepConfig(
    font_path="/path/to/font.ttf",
    font_size=1000,
    text="A",
    width=3000,
    height=1200,
    coord_sets=coord_sets,  # Render at all coordinates in parallel
)

font_loader = FontLoader(512)
options = ExecutionOptions(None, None)
options.set_glyph_cache_capacity(2048)

# Parallel rendering across cores (5-8× speedup)
results = render_variation_sweep(config, font_loader, options)

for point in results:
    print(f"wght={point.coords['wght']}: density={point.metrics.density:.4f}")
```

**Performance:** Renders 80 coordinates in ~2-3ms on 8 cores (vs ~16ms sequential).

See `examples/python/varsweep_demo.py` for complete examples.

---

## Job Format

### Input (Batch)

```json
{
  "version": "1.0",
  "jobs": [{
    "id": "unique_id",
    "font": {
      "path": "/path/to/font.ttf",
      "size": 1000,
      "variations": {"wght": 600.0}
    },
    "text": {"content": "A", "script": "Latn"},
    "rendering": {
      "format": "pgm",
      "encoding": "base64",
      "width": 3000,
      "height": 1200
    }
  }]
}
```

### Output (JSONL)

```json
{"id": "unique_id", "status": "success", "rendering": {...}, "timing": {...}}
{"id": "unique_id", "status": "error", "error": "Font not found", "timing": {...}}
```

### Metrics Output

```json
{
  "id": "unique_id",
  "status": "success",
  "metrics": {"density": 0.48, "beam": 0.012},
  "timing": {"shape_ms": 1.1, "render_ms": 0.7, "total_ms": 2.0}
}
```

---

## Building from Source

### CLI Binary

```bash
cargo build --release
export HAFORU_BIN="$PWD/target/release/haforu"
```

### Python Bindings

```bash
# Dev build (fast iteration)
pip install maturin
maturin develop

# Release build
maturin develop --release --features python

# Wheels for distribution
maturin build --release
```

---

## Testing

```bash
# Rust tests (41 tests)
cargo test

# Python tests
pip install -e .
pytest python/tests

# Smoke test (validates CLI contract in ~2s)
./scripts/batch_smoke.sh
```

---

## Error Handling

All errors return JSON with `status: "error"` and descriptive messages. No crashes, no silent failures.

**Common Errors:**
- Font not found → `"Font file not found: /path/to/font.ttf"`
- Invalid variation → `"Unknown variation axis: slnt"`
- Shaping failed → `"Text shaping failed for 'text'"`

---

## Integration with FontSimi

Haforu powers FontSimi's font matching pipeline with **10-20× speedup** through optimization-specific APIs:

### Primary Use: Image Processing (Python Bindings)

```python
import haforu

# Align and compare images (3-5× faster than Python/numpy)
result = haforu.align_and_compare(original_img, candidate_img, method="center")
pixel_delta = result.pixel_delta

# Scale images (2-3× faster than OpenCV)
scaled = haforu.resize_bilinear(image, multiplier=0.9)
```

**Call frequency:** 30-180 calls per font match
**Total impact:** 3-5× speedup for deep matching pipeline

### Secondary Use: Text Rendering Fallback (CLI)

Only when CoreText/HarfBuzz/Skia unavailable (rare). Use case: batch processing 100+ fonts.

Set `HAFORU_BIN` environment variable to use CLI mode, or import `haforu` for Python bindings.

---

## Use Cases

### 1. Font Matching Optimization (PRIMARY)

**Problem:** FontSimi needs to render same glyph at 80+ variation coordinates during optimization.

**Solution:** Batch variation sweep API + Python bindings

**Performance:** 80 coords in ~2-3ms (parallel) vs ~16ms (sequential) = **5-8× speedup**

```python
from haforu.varsweep import render_variation_sweep

# Generate coordinates
coord_sets = [{"wght": w} for w in range(100, 950, 50)]

# Parallel rendering
results = render_variation_sweep(config, font_loader, options)
```

### 2. Image Processing Hot Paths

**Problem:** Python/numpy image operations (align, scale, compare) in tight loops.

**Solution:** Rust-optimized Python bindings

**Performance:** 3-5× speedup for alignment, 2-3× for scaling

```python
# 30-180 calls per font match
for iteration in optimization_loop:
    aligned = haforu.align_and_compare(original, candidate, "center")
    scaled = haforu.resize_bilinear(image, multiplier)
```

### 3. Batch Font Analysis (FALLBACK)

**Problem:** Analyze 100+ fonts when native renderers unavailable.

**Solution:** Haforu CLI batch mode

**Performance:** 150-200 jobs/sec on 8 cores

```bash
# Generate jobs for all fonts
generate_jobs > jobs.json

# Batch processing (amortizes subprocess overhead)
haforu batch < jobs.json > results.jsonl
```

---

## Dependencies

**Core:** read-fonts 0.22, skrifa 0.22, harfbuzz_rs 2.0, zeno 0.3, memmap2 0.9, dashmap 6.1, rayon 1.10

**Python:** pyo3 0.22, numpy 0.22

---

## License

MIT OR Apache-2.0

---

**Status:** Production Ready
**Version:** v2.0.x
**Last Updated:** 2025-11-15
