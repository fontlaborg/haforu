this_file: README.md

# Haforu: High-Performance Batch Font Renderer

**Status:** Production-ready foundation for FontSimi H2-H5 integration

Haforu is a Rust-native batch font renderer designed to accelerate FontSimi's font matching pipeline by 100× (5 hours → 3 minutes) while reducing memory usage by 97% (86GB → <2GB).

## Architecture

### Core Principles

1. **Zero-copy font loading** via memory mapping (memmap2)
2. **LRU caching** of font instances (512 entries by default)
3. **Parallel batch processing** using Rayon
4. **Streaming JSONL I/O** for progressive results
5. **Production-grade error handling** with descriptive messages

### Module Structure

```
src/
├── batch.rs      # JobSpec, Job, JobResult data structures
├── fonts.rs      # FontLoader with memory-mapped fonts and caching
├── shaping.rs    # TextShaper using HarfBuzz
├── render.rs     # GlyphRasterizer using zeno
├── output.rs     # PGM/PNG generation with base64 encoding
├── error.rs      # Error types with context
├── lib.rs        # Public API and process_job()
└── main.rs       # CLI with batch and streaming modes
```

## Install

### Python bindings (recommended for deep matching)

```bash
uv pip install haforu
```

This installs the PyO3 module (`haforu.StreamingSession`, `haforu.process_jobs`, `haforu.is_available`) with universal2/manylinux wheels so no compiler is required.

### CLI binary

```bash
cargo install haforu
# or build from source inside this repo:
cargo build --release
export HAFORU_BIN="$PWD/target/release/haforu"
```

`fontsimi` looks for `HAFORU_BIN` first; falling back to `target/release/haforu` works for local development.

## Features

### Batch Mode

Read entire job specification from stdin, process in parallel, stream results as JSONL:

```bash
echo '{
  "version": "1.0",
  "jobs": [{
    "id": "test1",
    "font": {
      "path": "/path/to/font.ttf",
      "size": 1000,
      "variations": {"wght": 600.0}
    },
    "text": {"content": "A"},
    "rendering": {
      "format": "pgm",
      "encoding": "base64",
      "width": 3000,
      "height": 1200
    }
  }]
}' | haforu batch
```

### Streaming Mode (H4)

Keep process alive for continuous job processing:

```bash
haforu stream < jobs.jsonl > results.jsonl
```

Each input line is a single Job JSON, each output line is a JobResult.

## Job Specification Format

### Input: JobSpec (Batch) or Job (Streaming)

```json
{
  "version": "1.0",
  "jobs": [{
    "id": "unique_job_id",
    "font": {
      "path": "/absolute/path/to/font.ttf",
      "size": 1000,
      "variations": {"wght": 600.0, "wdth": 100.0}
    },
    "text": {
      "content": "A",
      "script": "Latn"
    },
    "rendering": {
      "format": "pgm",
      "encoding": "base64",
      "width": 3000,
      "height": 1200
    }
  }]
}
```

### Output: JobResult (JSONL)

**Success:**
```json
{
  "id": "unique_job_id",
  "status": "success",
  "rendering": {
    "format": "pgm",
    "encoding": "base64",
    "data": "UDUKMzAwMCAxMjAwCjI1NQo...",
    "width": 3000,
    "height": 1200,
    "actual_bbox": [450, 200, 1200, 800]
  },
  "timing": {
    "shape_ms": 1.2,
    "render_ms": 3.4,
    "total_ms": 5.0
  }
}
```

**Error:**
```json
{
  "id": "unique_job_id",
  "status": "error",
  "error": "Font file not found: /path/to/missing.ttf",
  "timing": {"shape_ms": 0.0, "render_ms": 0.0, "total_ms": 0.1}
}
```

## CLI Usage

### Batch Mode

```bash
# Basic usage
haforu batch < jobs.json > results.jsonl

# Custom cache size and explicit parallelism (alias: --workers)
haforu batch --cache-size 1024 --jobs 8 < jobs.json > results.jsonl

# JSONL input (one job per line, perfect for streaming chunks)
haforu batch --jobs 6 < jobs.jsonl > results.jsonl

# Verbose logging
haforu batch --verbose < jobs.json > results.jsonl 2> debug.log
```

### Streaming Mode

```bash
# Process jobs line-by-line
haforu stream < jobs.jsonl > results.jsonl

# With verbose logging
haforu stream --verbose < jobs.jsonl > results.jsonl 2> debug.log
```

## Building

### Development Build

```bash
cargo build
cargo test
```

### Release Build

```bash
cargo build --release
```

Binary: `target/release/haforu`

### Python Bindings

Haforu provides zero-overhead Python bindings for maximum performance in Python-based font analysis pipelines.

#### Installation

```bash
# Development installation (from source)
cd haforu
maturin develop --features python

# Verify installation
python -c "import haforu; print(haforu.__version__)"
```

#### Quick Start: Batch Mode

```python
import haforu
import json

# Create job specification
spec = {
    "version": "1.0",
    "jobs": [{
        "id": "test1",
        "font": {"path": "/path/to/font.ttf", "size": 1000, "variations": {}},
        "text": {"content": "A", "script": "Latn"},
        "rendering": {"format": "pgm", "encoding": "base64", "width": 3000, "height": 1200}
    }]
}

# Process jobs in parallel
for result_json in haforu.process_jobs(json.dumps(spec)):
    result = json.loads(result_json)
    print(f"Job {result['id']}: {result['status']}")
```

#### Quick Start: Streaming Mode

```python
import haforu

# Create persistent session with font cache
with haforu.StreamingSession(cache_size=512) as session:
    # Render single job
    job = {"id": "test", "font": {...}, "text": {...}, "rendering": {...}}
    result_json = session.render(json.dumps(job))

    # Or get numpy array directly (zero-copy)
    image = session.render_to_numpy(
        font_path="/path/to/font.ttf",
        text="A",
        size=1000.0,
        width=3000,
        height=1200,
        variations={"wght": 600.0}
    )
    # image is numpy.ndarray of shape (height, width), dtype uint8
```

#### API Reference

**`haforu.process_jobs(spec_json: str) -> Iterator[str]`**

Process a batch of rendering jobs in parallel. Returns iterator yielding JSONL result strings.

- **Args**: `spec_json` - JSON string containing JobSpec with jobs array
- **Returns**: Iterator of JSONL result strings (one per completed job)
- **Raises**: `ValueError` (invalid JSON/spec), `RuntimeError` (font/rendering errors)
- **Performance**: 100-150 jobs/sec on 8 cores

**`haforu.StreamingSession(cache_size: int = 512)`**

Persistent rendering session with font cache for zero-overhead repeated rendering.

- **`render(job_json: str) -> str`**: Render single job, return JSONL result
- **`render_to_numpy(...) -> np.ndarray`**: Render directly to numpy array (zero-copy)
  - Args: `font_path, text, size, width, height, variations, script, direction, language`
  - Returns: 2D array of shape (height, width), dtype uint8, grayscale 0-255
  - Performance: 1-2ms per render (30-50× faster than CLI subprocess)
- **`warm_up(font_path: str | None = None, *, text=\"Haforu\", size=600, width=128, height=128) -> bool`**:
  Ping the cache or proactively render a glyph so later renders stay within the 1-2 ms budget.
- **`cache_stats() -> dict[str, int]`** and **`set_cache_size(cache_size: int) -> None`**:
  Inspect or tune the LRU capacity at runtime (setting a new size resets the cache safely).
- **`close()`**: Release font cache and resources
- **Context manager**: Supports `with` statement for automatic cleanup
- **`is_available()` (classmethod)**: Cheap probe fontsimi can call without importing heavy deps.

**`haforu.is_available() -> bool`**

Module-level probe that returns True when the native extension is importable and ready.

#### Examples

See `examples/python/` for complete examples:

- **`batch_demo.py`**: Parallel batch processing
- **`streaming_demo.py`**: Persistent session with font caching
- **`numpy_demo.py`**: Zero-copy numpy arrays for image analysis
- **`error_handling_demo.py`**: Robust error handling patterns

#### Performance Comparison

| Mode | Overhead | Render Time | Total | Use Case |
|------|----------|-------------|-------|----------|
| CLI Batch | 500ms | 50-75s (5000 jobs) | ~50s | Initial analysis |
| CLI Streaming | 10-20ms | 30-50ms | 40-70ms | Subprocess overhead |
| **Python Bindings** | **0ms** | **1-2ms** | **1-2ms** | Deep matching, ML pipelines |

**Speedup**: Python bindings are 30-50× faster than CLI streaming for repeated renders.

## Testing

### Unit Tests

```bash
cargo test
```

### Integration Tests

```bash
# Test with real font
echo '{"version":"1.0","jobs":[{
  "id":"test1",
  "font":{"path":"../../test-fonts/Arial-Black.ttf","size":1000},
  "text":{"content":"A"},
  "rendering":{"format":"pgm","encoding":"base64","width":3000,"height":1200}
}]}' | ./target/release/haforu batch | jq .

# 2-second JSONL smoke (uses testdata/jobs_smoke.jsonl)
./scripts/batch_smoke.sh
```

## Performance Characteristics

| Metric | Target | Status |
|--------|--------|--------|
| Single render | <100ms | ✅ |
| Batch (1000 jobs) | <10s | ✅ |
| Memory usage | <500MB (1000 renders) | ✅ |
| Cache hit rate | >80% (typical workload) | ✅ |

## Error Handling

All errors include descriptive context:

- **FontNotFound**: Includes path
- **UnknownAxis**: Lists available axes
- **CoordinateOutOfBounds**: Shows bounds and provided value
- **ShapingFailed**: Includes text and font path
- **RasterizationFailed**: Includes glyph ID and reason

Errors never crash the process - failed jobs return `status="error"` and processing continues.

## Dependencies

### Core Font Stack

- **read-fonts 0.22**: Font file parsing
- **skrifa 0.22**: Glyph outlines and metadata
- **harfbuzz 0.4**: Text shaping (bundled)
- **zeno 0.3**: Rasterization

### Infrastructure

- **memmap2 0.9**: Zero-copy font loading
- **lru 0.12**: Font instance caching
- **rayon 1.10**: Parallel processing
- **serde/serde_json**: JSON I/O
- **base64 0.22**: JSONL encoding
- **image 0.25**: PNG output
- **clap 4.5**: CLI
- **thiserror/anyhow**: Error handling

## Integration with FontSimi

Haforu2 integrates into FontSimi via `src/fontsimi/renderers/haforu.py`:

```python
from fontsimi.renderers.base import BaseRenderer

class HaforuRenderer(BaseRenderer):
    def render_text(self, font_path, text, size, variations=None):
        # Generate job JSON
        # Invoke haforu subprocess
        # Parse JSONL output
        # Decode base64 PGM
        # Return numpy array
        ...
```

### H2-H5 Roadmap

- **H2 (this)**: Core rendering implementation ✅
- **H3**: FontSimi batch analysis pipeline (Python)
- **H4**: Streaming mode for deep matching (Rust + Python)
- **H5**: Performance validation and optimization

## License

MIT OR Apache-2.0
