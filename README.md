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

### Quick Install

```bash
# Python package (recommended)
pip install haforu

# Rust binary via Cargo
cargo install haforu
```

This installs both the native binary and Python bindings with universal2/manylinux wheels (no compiler required).

**Platform-specific installations** and troubleshooting: See [INSTALL.md](INSTALL.md)

### Environment Setup

```bash
# Point to the binary location
export HAFORU_BIN="$(which haforu)"

# Or for local development
export HAFORU_BIN="$PWD/target/release/haforu"
```

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

Tune caches per workload:

- `--max-fonts` (alias `--cache-size`) controls the FontLoader LRU entries.
- `--max-glyphs` sizes the render-result cache (set `0` to disable reuse).
- `--jobs` adjusts Rayon workers; `0` keeps the auto-detected default.
- `--stats` emits a JSON summary to `stderr` so you can track throughput/regressions.

### Streaming Mode (H4)

Keep process alive for continuous job processing:

```bash
haforu stream < jobs.jsonl > results.jsonl
```

Each input line is a single Job JSON, each output line is a JobResult.
`haforu stream --max-fonts 256 --max-glyphs 2048` keeps both caches hot;
pass `--max-glyphs 0` when you need deterministic uncached renders.

### Diagnostics

Inspect the CLI environment and defaults:

```bash
haforu diagnostics
haforu diagnostics --format json
```

The Python CLI mirrors these commands via `python -m haforu diagnostics`.

### HarfBuzz-Compatible Render Mode

Quick single-text rendering using HarfBuzz-style syntax:

```bash
# Basic rendering
haforu render -f font.ttf -t "Hello World" -o output.pgm

# With variations
haforu render -f font.ttf -t "Text" --variations "wght=700,wdth=100" -s 48

# Metrics only
haforu render -f font.ttf -t "A" --format metrics

# Get HarfBuzz help
haforu render --help-harfbuzz
```

### Metrics Mode

Skip base64 blobs entirely by setting `rendering.format` to `"metrics"`. Haforu still rasterizes
the glyph but reuses the in-memory buffer to emit normalized `density` (pixel coverage) and
`beam` (longest contiguous non-zero run) metrics under a new `metrics` field. The `rendering`
object is omitted for these jobs and the `encoding` value is ignored. See
`examples/python/metrics_demo.py` for an end-to-end Python demo.

### Python CLI

Use haforu from Python with Fire-based commands:

```bash
# Via module
python -m haforu batch -input jobs.json -output results.jsonl

# Via installed script
haforu-py render_single --text "Hello" --font font.ttf --size 72

# Multiple formats
haforu-py metrics -input jobs.json -format csv
haforu-py render --font font.ttf --text "Hello" --format metrics --direction rtl
```

Run `python -m haforu diagnostics --format json` to inspect the Python CLI defaults and environment (mirrors the Rust `haforu diagnostics` command).

### Smoke Test

`scripts/batch_smoke.sh` validates the CLI contract in ~2 s once the release
binary exists (the very first run includes a `cargo build --release`, so expect
~80 s of one-time compilation). Override knobs via `CACHE_SIZE`, `GLYPH_CACHE_SIZE`,
and `JOB_THREADS`; point `HAFORU_BIN` at a prebuilt binary when running on CI:

```bash
export HAFORU_BIN="$PWD/target/release/haforu"
./scripts/batch_smoke.sh
```

The script asserts both success/error payloads, enforces the metrics schema, and
fails fast if a JSONL line is malformed.

## Build & Release Pipeline

- `scripts/build.sh` drives reproducible builds (Rust CLI, wheels, tests, smoke runs) and snapshots artifacts under `target/artifacts/<timestamp>/`.
- `scripts/run.sh` replays the bundled fixtures (batch/stream/metrics) in under a minute so you can verify JSON contracts locally.
- GitHub Actions mirrors the same steps via `ci.yml` (per-OS smoke + tests) and `release.yml` (tagged releases publish binaries + wheels).
- Version numbers come from git tags via `hatch-vcs` + `scripts/sync-version.sh`, and `release.yml` only triggers on `vX.Y.Z` tags.

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
      "format": "pgm",            // "pgm", "png", or "metrics"
      "encoding": "base64",       // set to "json" for metrics jobs (ignored)
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
  "font": {
    "path": "/path/to/font.ttf",
    "variations": {"wght": 650.0}
  },
  "timing": {
    "shape_ms": 1.2,
    "render_ms": 3.4,
    "total_ms": 5.0
  }
}
```

**Success (metrics mode):**
```json
{
  "id": "unique_job_id",
  "status": "success",
  "metrics": {
    "density": 0.48,
    "beam": 0.012
  },
  "font": {
    "path": "/path/to/font.ttf",
    "variations": {"wght": 650.0}
  },
  "timing": {"shape_ms": 1.1, "render_ms": 0.7, "total_ms": 2.0}
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

Every result optionally includes a `font` object with the sanitized path and variation coordinates actually used. The field is omitted when a job fails validation before loading a font. Jobs rendered with `format: "metrics"` also emit a `metrics` object (density + beam) and suppress the `rendering` blob entirely.

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

### Canonical Build & Demo Scripts

The `scripts/` directory now hosts the end-to-end workflow expected by FontSimi:

- `./scripts/build.sh` builds the Rust CLI (`--bin haforu`), produces wheels for the
  current platform (universal2 on macOS, manylinux on Linux, Windows wheels when running
  on Windows), runs `cargo test`, `uvx hatch test`, and executes the JSONL smoke suite.
  Artifacts live under `target/artifacts/<timestamp>/` with a `latest` symlink plus
  `summary.txt`/`timings.txt` for reproducibility. Example invocations:

  ```bash
  ./scripts/build.sh               # release build + wheels + tests + smoke
  ./scripts/build.sh --skip-wheels # re-use cached wheels when iterating on Rust only
  ./scripts/build.sh --profile dev --skip-tests --skip-smoke
  ```

- `./scripts/run.sh smoke` streams the bundled fixtures through the batch, metrics, and
  streaming commands (re-using `scripts/jobs_smoke.jsonl`). It prints condensed summaries
  and logs the raw output under `target/run-log/`. Add `python` to the mode to demo the
  PyO3 `StreamingSession` once a wheel is installed.

These scripts are the fastest way to validate a tree before publishing artifacts.

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

#### Wheel Builds

Ship reproducible wheels with `maturin` (macOS universal2 + manylinux):

```bash
uv tool install maturin
uv run maturin build --release --target universal2-apple-darwin --out wheels
uv run maturin build --release --compatibility manylinux_2_28 --out wheels
```

Publish the resulting `.whl` files or install locally via
`uv pip install wheels/haforu-<version>-<tag>.whl`. Remember to export
`HAFORU_BIN` so fontsimi and the smoke script pick up the freshly built binary.

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

# Create persistent session with tunable caches
with haforu.StreamingSession(max_fonts=512, max_glyphs=2048) as session:
    session.ping()  # cheap liveness probe
    stats = session.cache_stats()  # font_capacity, glyph_entries, glyph_hits, ...

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
    session.set_glyph_cache_size(256)  # resize/disable reuse on the fly
```

#### API Reference

**`haforu.process_jobs(spec_json: str) -> Iterator[str]`**

Process a batch of rendering jobs in parallel. Returns iterator yielding JSONL result strings.

- **Args**: `spec_json` - JSON string containing JobSpec with jobs array
- **Returns**: Iterator of JSONL result strings (one per completed job)
- **Raises**: `ValueError` (invalid JSON/spec), `RuntimeError` (font/rendering errors)
- **Performance**: 100-150 jobs/sec on 8 cores

**`haforu.StreamingSession(cache_size: int | None = None, *, max_fonts: int | None = None, max_glyphs: int = 2048)`**

Persistent rendering session with independent font + glyph caches for zero-overhead repeated rendering.

- **`render(job_json: str) -> str`**: Render single job, always returning a JSONL result even for invalid specs (schema matches CLI/JSONL stream)
- **`render_to_numpy(...) -> np.ndarray`**: Render directly to numpy array (zero-copy)
  - Args: `font_path, text, size, width, height, variations, script, direction, language`
  - Returns: 2D array of shape (height, width), dtype uint8, grayscale 0-255
  - Performance: 1-2ms per render (30-50× faster than CLI subprocess)
- **`warm_up(font_path: str | None = None, *, text=\"Haforu\", size=600, width=128, height=128) -> bool`**:
  Ping the cache or proactively render a glyph so later renders stay within the 1-2 ms budget.
- **`ping() -> bool`**: Microsecond liveness probe so fontsimi can avoid exception-driven health checks.
- **`cache_stats() -> dict[str, int]`**, **`set_cache_size(cache_size: int)`**, **`set_glyph_cache_size(max_glyphs: int)`**:
  Inspect or tune the caches at runtime. `cache_stats` now reports `font_*` plus `glyph_capacity`, `glyph_entries`, and `glyph_hits` for observability.
- **`close()`**: Release caches and descriptors immediately
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
- **`metrics_demo.py`**: Request metrics-only JSON output

#### Performance Comparison

| Mode | Overhead | Render Time | Total | Use Case |
|------|----------|-------------|-------|----------|
| CLI Batch | 500ms | 50-75s (5000 jobs) | ~50s | Initial analysis |
| CLI Streaming | 10-20ms | 30-50ms | 40-70ms | Subprocess overhead |
| **Python Bindings** | **0ms** | **1-2ms** | **1-2ms** | Deep matching, ML pipelines |

**Speedup**: Python bindings are 30-50× faster than CLI streaming for repeated renders.

## Troubleshooting

- `scripts/batch_smoke.sh` should complete in ≤2 s after the first release build. If it keeps rebuilding, export `HAFORU_BIN=target/release/haforu` so the script skips `cargo build --release`.
- `haforu batch --max-fonts 0` disables the font cache and is almost never what you want; instead size the cache via `--max-fonts` and keep glyph reuse enabled with `--max-glyphs`.
- When the Python binding raises `ImportError`, reinstall the wheel (`uv run maturin develop` or `uv pip install haforu`) and re-run `haforu.is_available()` before constructing a session.
- High miss rates in streaming mode usually mean the glyph cache is undersized. Check `session.cache_stats()['glyph_entries']` / `['glyph_hits']` and bump `max_glyphs` or call `set_glyph_cache_size`.
- If CLI renders cannot locate fonts, pass `--base-dir` or use absolute font paths; the JSON contract always reports the sanitized `font.path` so you can audit what haforu actually used.

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

# 2-second JSONL smoke (uses scripts/jobs_smoke.jsonl)
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
