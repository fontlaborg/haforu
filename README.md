---
this_file: README.md
---

# Haforu: Fast Font Renderer

**Status:** Production-ready

Fast, deterministic font rendering for CLI and Python. Renders glyphs to PGM/PNG/metrics with sub-millisecond performance.

## What It Does

Haforu renders text using TrueType/OpenType fonts with:
- **Parallel batch processing** - Process thousands of glyphs via CLI
- **Streaming mode** - Keep process alive for continuous rendering
- **Python bindings** - Sub-2ms renders with persistent sessions
- **Three output modes:**
  - `pgm` - Grayscale Netpbm format
  - `png` - PNG image
  - `metrics` - Density + beam measurements (10× faster)

## Install

```bash
# Python (includes CLI binary)
pip install haforu

# Rust binary only
cargo install haforu
```

**Prerequisites:** Python 3.8+ or Rust 1.70+. No compiler needed for pip install (prebuilt wheels).

## Quick Start

### CLI

```bash
# Single render
haforu render -f font.ttf -t "Hello" -s 72 -o output.pgm

# Batch processing
echo '{
  "version": "1.0",
  "jobs": [{
    "id": "test1",
    "font": {"path": "/path/to/font.ttf", "size": 1000},
    "text": {"content": "A"},
    "rendering": {"format": "pgm", "encoding": "base64", "width": 3000, "height": 1200}
  }]
}' | haforu batch > results.jsonl

# Streaming mode
haforu stream < jobs.jsonl > results.jsonl

# Metrics only (fast)
haforu render -f font.ttf -t "A" -s 256 --format metrics
```

### Python

```python
import haforu
import json

# Batch processing
spec = {
    "version": "1.0",
    "jobs": [{
        "id": "test1",
        "font": {"path": "/path/to/font.ttf", "size": 1000},
        "text": {"content": "A"},
        "rendering": {"format": "pgm", "encoding": "base64", "width": 3000, "height": 1200}
    }]
}

for result_json in haforu.process_jobs(json.dumps(spec)):
    result = json.loads(result_json)
    print(f"Job {result['id']}: {result['status']}")

# Streaming session (persistent, fast)
with haforu.StreamingSession(max_fonts=512, max_glyphs=2048) as session:
    # Render to JSON
    job = {"id": "test", "font": {...}, "text": {...}, "rendering": {...}}
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

## Architecture

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
- Memory-mapped fonts (zero-copy loading via memmap2)
- LRU font cache (512 entries default)
- Parallel batch processing (Rayon)
- Deterministic JSONL I/O
- Every error returns JSON (no crashes)

## Performance

| Mode | Throughput | Use Case |
|------|-----------|----------|
| CLI Batch | 100-150 jobs/sec | Initial analysis |
| CLI Streaming | ~100 jobs/sec | Continuous processing |
| Python Bindings | 500-1000 jobs/sec | High-frequency rendering |
| Metrics Mode | 2000-5000 jobs/sec | Feature extraction |

**Targets:**
- Single render: <10ms cold, <2ms warm (Python)
- Batch (1000 jobs): <10s on 8 cores
- Memory: <500MB for 1000 renders

## CLI Commands

```bash
# Batch: Read full JSON, process in parallel
haforu batch [--max-fonts N] [--max-glyphs N] [--jobs N] [--stats]

# Stream: Line-by-line JSONL processing
haforu stream [--max-fonts N] [--max-glyphs N]

# Render: Single text render (HarfBuzz-compatible)
haforu render -f FONT -t TEXT [-s SIZE] [-o OUTPUT] [--format pgm|png|metrics]

# Diagnostics: Show system info and defaults
haforu diagnostics [--format text|json]

# Validate: Check job specification
haforu validate < jobs.json
```

**Cache Tuning:**
- `--max-fonts N` - Font cache size (default: 512)
- `--max-glyphs N` - Glyph cache size (default: 2048, 0 to disable)
- `--jobs N` - Parallel workers (default: CPU count)

## Python API

### `haforu.process_jobs(spec_json: str) -> Iterator[str]`

Process batch of jobs in parallel. Returns iterator of JSONL results.

### `haforu.StreamingSession(max_fonts=512, max_glyphs=2048)`

Persistent rendering session with font/glyph caching.

**Methods:**
- `render(job_json: str) -> str` - Render job, return JSONL result
- `render_to_numpy(...) -> np.ndarray` - Render directly to numpy array
- `warm_up(font_path: str) -> bool` - Warm up cache
- `ping() -> bool` - Liveness check
- `cache_stats() -> dict` - Inspect cache state
- `set_cache_size(n: int)` - Resize font cache
- `set_glyph_cache_size(n: int)` - Resize glyph cache
- `close()` - Release resources

### `haforu.is_available() -> bool`

Check if native extension is available.

## Job Format

**Input (batch):**
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

**Output (JSONL):**
```json
{"id": "unique_id", "status": "success", "rendering": {...}, "timing": {...}}
{"id": "unique_id", "status": "error", "error": "Font not found", "timing": {...}}
```

**Metrics Output:**
```json
{
  "id": "unique_id",
  "status": "success",
  "metrics": {"density": 0.48, "beam": 0.012},
  "timing": {"shape_ms": 1.1, "render_ms": 0.7, "total_ms": 2.0}
}
```

## Building from Source

```bash
# Rust CLI
cargo build --release
export HAFORU_BIN="$PWD/target/release/haforu"

# Python bindings (dev)
pip install maturin
maturin develop

# Python wheels
maturin build --release
```

## Testing

```bash
# Rust tests
cargo test

# Python tests
pip install -e .
pytest python/tests

# Smoke test (validates CLI contract in ~2s)
./scripts/batch_smoke.sh
```

## Error Handling

All errors return JSON with `status: "error"` and descriptive messages. No crashes, no silent failures.

**Common Errors:**
- Font not found → `"Font file not found: /path/to/font.ttf"`
- Invalid variation → `"Unknown variation axis: slnt"`
- Shaping failed → `"Text shaping failed for 'text'"`

## Integration with FontSimi

Haforu powers FontSimi's font matching pipeline:

1. **Batch Analysis** - Process thousands of glyphs via CLI streaming
2. **Deep Matching** - Use Python StreamingSession for rapid comparison
3. **Metrics Mode** - Extract features without image encoding overhead

Set `HAFORU_BIN` environment variable to use CLI mode, or import `haforu` for Python bindings.

## Dependencies

**Core:** read-fonts 0.22, skrifa 0.22, harfbuzz_rs 2.0, zeno 0.3, memmap2 0.9, lru 0.12, rayon 1.10

**Python:** pyo3 0.22, numpy 0.22

## License

MIT OR Apache-2.0
