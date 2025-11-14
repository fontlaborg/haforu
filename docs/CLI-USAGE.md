---
this_file: docs/CLI-USAGE.md
---

# Haforu CLI Usage Guide

Complete reference for the haforu command-line interface. Both Rust CLI (`haforu`) and Python CLI (`python -m haforu`) support identical commands and JSON contracts.

## Table of Contents

- [Quick Start](#quick-start)
- [Commands](#commands)
  - [batch](#batch---batch-processing)
  - [stream](#stream---streaming-mode)
  - [render](#render---single-render)
  - [validate](#validate---job-validation)
  - [diagnostics](#diagnostics---system-info)
  - [version](#version---version-display)
- [JSON Contract](#json-contract)
- [Streaming JSON (JSONL)](#streaming-json-jsonl)
- [Error Handling](#error-handling)
- [Performance Tuning](#performance-tuning)
- [Examples](#examples)

## Quick Start

```bash
# Render a single glyph (metrics only)
haforu render -f font.ttf -s 256 -t "A" --format metrics

# Batch process multiple jobs
echo '{"version":"1.0","jobs":[...]}' | haforu batch

# Stream JSONL jobs
cat jobs.jsonl | haforu stream

# Validate a job specification
haforu validate < jobs.json

# Display system diagnostics
haforu diagnostics
```

## Commands

### batch - Batch Processing

Process multiple rendering jobs from a single JSON input.

**Usage:**
```bash
haforu batch [OPTIONS] < input.json
```

**Options:**
- `--max-fonts <N>` - Maximum fonts to cache (default: 512)
- `--max-glyphs <N>` - Maximum glyphs to cache (default: 2048)
- `--jobs <N>` - Number of parallel workers (default: CPU count)
- `--timeout-ms <MS>` - Per-job timeout in milliseconds (0 = disabled)
- `--base-dir <PATH>` - Restrict font paths to this directory
- `--stats` - Emit throughput statistics to stderr

**Input Format:**
```json
{
  "version": "1.0",
  "jobs": [
    {
      "id": "job-1",
      "font": {
        "path": "/path/to/font.ttf",
        "size": 256,
        "variations": {"wght": 700, "wdth": 100}
      },
      "text": {
        "content": "A",
        "script": "Latn",
        "language": "en",
        "direction": "ltr",
        "features": ["liga=0", "kern"]
      },
      "rendering": {
        "format": "metrics",
        "encoding": "json",
        "width": 64,
        "height": 64
      }
    }
  ]
}
```

**Output:** JSONL stream (one result per line)
```json
{"id":"job-1","status":"success","metrics":{"density":0.627,"beam":0.0144},"font":{"path":"/path/to/font.ttf"},"timing":{"shape_ms":0.0,"render_ms":0.0,"total_ms":0.06}}
```

**Example:**
```bash
# Process batch with custom cache settings
haforu batch --max-fonts 128 --max-glyphs 1024 < jobs.json > results.jsonl

# With statistics
haforu batch --stats < jobs.json 2>stats.json > results.jsonl
```

### stream - Streaming Mode

Process jobs one at a time from JSONL input (no job array wrapper).

**Usage:**
```bash
haforu stream [OPTIONS] < input.jsonl
```

**Options:**
- `--max-fonts <N>` - Maximum fonts to cache (default: 512)
- `--max-glyphs <N>` - Maximum glyphs to cache (default: 2048)
- `--stats` - Emit statistics to stderr

**Input Format:** One job per line (JSONL)
```json
{"id":"job-1","font":{"path":"font.ttf","size":256,"variations":{}},"text":{"content":"A"},"rendering":{"format":"metrics","encoding":"json","width":64,"height":64}}
{"id":"job-2","font":{"path":"font.ttf","size":256,"variations":{}},"text":{"content":"B"},"rendering":{"format":"metrics","encoding":"json","width":64,"height":64}}
```

**Output:** JSONL stream (one result per line)
```json
{"id":"job-1","status":"success","metrics":{"density":0.627,"beam":0.0144},"font":{"path":"font.ttf"},"timing":{"shape_ms":0.0,"render_ms":0.0,"total_ms":0.05}}
{"id":"job-2","status":"success","metrics":{"density":0.523,"beam":0.0122},"font":{"path":"font.ttf"},"timing":{"shape_ms":0.0,"render_ms":0.0,"total_ms":0.04}}
```

**Example:**
```bash
# Stream processing
cat jobs.jsonl | haforu stream > results.jsonl

# With warm cache for maximum throughput
haforu stream --max-fonts 256 --max-glyphs 2048 < large-jobs.jsonl
```

### render - Single Render

Render a single text string (convenience command with HarfBuzz-compatible syntax).

**Usage:**
```bash
haforu render -f FONT -s SIZE -t TEXT [OPTIONS]
```

**Required Options:**
- `-f, --font-file <PATH>` - Path to font file
- `-s, --font-size <SIZE>` - Font size in points
- `-t, --text <TEXT>` - Text to render

**Optional:**
- `--format <FORMAT>` - Output format: pgm, png, metrics (default: pgm)
- `--variations <SPEC>` - Font variations: "wght=700,wdth=100" or JSON
- `--script <SCRIPT>` - Script hint (e.g., "Latn", "Arab")
- `--language <LANG>` - Language tag (e.g., "en", "ar")
- `--direction <DIR>` - Text direction: ltr, rtl, ttb, btt (default: ltr)
- `--features <FEATURES>` - OpenType features: "liga=0,kern"
- `--width <W>` - Canvas width in pixels (default: 800)
- `--height <H>` - Canvas height in pixels (default: 200)
- `-o, --output-file <PATH>` - Output file (stdout if not specified)

**Examples:**
```bash
# Metrics only (fast)
haforu render -f font.ttf -s 256 -t "A" --format metrics

# PGM image output
haforu render -f font.ttf -s 72 -t "Hello" --format pgm -o output.pgm

# Variable font with variations
haforu render -f variable.ttf -s 256 -t "A" \
  --variations "wght=700,wdth=100" --format metrics

# Arabic text with shaping
haforu render -f arabic.ttf -s 72 -t "مرحبا" \
  --script Arab --language ar --direction rtl --format png -o arabic.png

# With OpenType features
haforu render -f font.ttf -s 72 -t "ffi" \
  --features "liga=1" --format pgm
```

**HarfBuzz Compatibility:**
The `render` command uses HarfBuzz-compatible flag names for easy migration:
- `-f` / `--font-file` (like `hb-view --font-file`)
- `-s` / `--font-size` (like `hb-view --font-size`)
- `-t` / `--text` (like `hb-view --text`)
- `--variations` (like `hb-view --variations`)

Use `haforu render --help-harfbuzz` for migration examples.

### validate - Job Validation

Validate a JSON job specification without executing it.

**Usage:**
```bash
haforu validate [FILE]
```

**Example:**
```bash
# Validate from stdin
haforu validate < jobs.json

# Validate from file
haforu validate jobs.json

# Check if valid (exit code 0 = valid, 1 = invalid)
if haforu validate jobs.json; then
  echo "Valid"
fi
```

**Output on success:**
```
✓ Valid job specification
  Version: 1.0
  Jobs: 10
```

**Output on failure:**
```
Validation failed:
  - Job [2]: Missing 'font.path'
  - Job [5]: 'font.size' must be a number
```

### diagnostics - System Info

Display system diagnostics and default settings.

**Usage:**
```bash
haforu diagnostics [--format FORMAT]
```

**Options:**
- `--format <FORMAT>` - Output format: text (default) or json

**Example:**
```bash
# Human-readable
haforu diagnostics

# JSON format
haforu diagnostics --format json
```

**Output (text):**
```
haforu 2.0.0
Status       : ok
CPU threads  : 8
Cache defaults: fonts=512 glyphs=2048
Security     : max_json_size=100MB
```

**Output (JSON):**
```json
{
  "status": "ok",
  "version": "2.0.0",
  "cpu_count": 8,
  "cache_defaults": {
    "max_fonts": 512,
    "max_glyphs": 2048
  },
  "security": {
    "max_json_size": 104857600
  }
}
```

### version - Version Display

Print version information.

**Usage:**
```bash
haforu version
# or
haforu --version
```

**Output:**
```
haforu 2.0.0
```

## JSON Contract

### Job Specification

A complete job specification:

```json
{
  "id": "unique-job-id",
  "font": {
    "path": "/absolute/path/to/font.ttf",
    "size": 256,
    "variations": {
      "wght": 700.0,
      "wdth": 100.0
    }
  },
  "text": {
    "content": "A",
    "script": "Latn",
    "language": "en",
    "direction": "ltr",
    "features": ["liga=0", "kern"]
  },
  "rendering": {
    "format": "metrics",
    "encoding": "json",
    "width": 64,
    "height": 64
  }
}
```

**Field Reference:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | Yes | Unique job identifier |
| `font.path` | string | Yes | Absolute path to font file |
| `font.size` | number | Yes | Font size in points (typically 256 for metrics) |
| `font.variations` | object | No | Variable font coordinates (axis → value) |
| `text.content` | string | Yes | Text to render |
| `text.script` | string | No | Script hint (ISO 15924 code, e.g., "Latn", "Arab") |
| `text.language` | string | No | Language tag (e.g., "en", "ar") |
| `text.direction` | string | No | Text direction: "ltr", "rtl", "ttb", "btt" |
| `text.features` | array | No | OpenType features (e.g., ["liga=0", "kern"]) |
| `rendering.format` | string | Yes | Output format: "pgm", "png", "metrics" |
| `rendering.encoding` | string | Yes | Encoding: "base64" (images) or "json" (metrics) |
| `rendering.width` | number | Yes | Canvas width in pixels |
| `rendering.height` | number | Yes | Canvas height in pixels |

### Job Result

Successful result:

```json
{
  "id": "job-1",
  "status": "success",
  "metrics": {
    "density": 0.6270029105392156,
    "beam": 0.014404296875
  },
  "font": {
    "path": "/path/to/font.ttf",
    "variations": {"wght": 700.0}
  },
  "timing": {
    "shape_ms": 0.0,
    "render_ms": 0.0,
    "total_ms": 0.06
  }
}
```

Error result:

```json
{
  "id": "job-2",
  "status": "error",
  "error": "Font file not found: /missing/font.ttf"
}
```

**Result Fields:**

| Field | Type | Always Present | Description |
|-------|------|----------------|-------------|
| `id` | string | Yes | Job identifier (from input) |
| `status` | string | Yes | "success" or "error" |
| `error` | string | Only on error | Error message |
| `metrics` | object | Only on success (metrics mode) | Density and beam measurements |
| `rendering` | object | Only on success (image mode) | Image data and metadata |
| `font` | object | Only on success | Sanitized font info |
| `timing` | object | Only on success | Performance timings |

## Streaming JSON (JSONL)

Haforu uses **JSON Lines (JSONL)** format for streaming:
- One JSON object per line
- Each line is a complete, valid JSON object
- No commas between lines
- Newline-delimited (`\n`)

**Benefits:**
- ✅ Process jobs incrementally (low memory)
- ✅ Start producing results immediately
- ✅ Handle unlimited job counts
- ✅ Easy to generate/parse line-by-line

**Example Producer (Bash):**
```bash
#!/bin/bash
for i in {1..1000}; do
  cat <<EOF
{"id":"job-$i","font":{"path":"font.ttf","size":256,"variations":{}},"text":{"content":"$i"},"rendering":{"format":"metrics","encoding":"json","width":64,"height":64}}
EOF
done | haforu stream
```

**Example Consumer (Python):**
```python
import json
import sys

# Read JSONL results
for line in sys.stdin:
    result = json.loads(line)
    if result['status'] == 'success':
        print(f"{result['id']}: density={result['metrics']['density']:.4f}")
    else:
        print(f"{result['id']}: ERROR - {result['error']}", file=sys.stderr)
```

## Error Handling

Haforu guarantees that **every input line produces an output line**, even on errors.

### Error Categories

1. **Parse Errors** - Invalid JSON
```json
{"id":"unknown","status":"error","error":"Invalid JSON: expected value at line 1 column 1"}
```

2. **Validation Errors** - Missing required fields
```json
{"id":"job-1","status":"error","error":"Missing required field: font.path"}
```

3. **Font Errors** - Font file issues
```json
{"id":"job-2","status":"error","error":"Font file not found: /missing/font.ttf"}
```

4. **Rendering Errors** - Rendering failures
```json
{"id":"job-3","status":"error","error":"Failed to shape text: unsupported script"}
```

### Error Handling Patterns

**Pattern 1: Filter successful jobs**
```bash
haforu batch < jobs.json | jq 'select(.status == "success")'
```

**Pattern 2: Count errors**
```bash
haforu stream < jobs.jsonl | jq -r 'select(.status == "error") | .id' | wc -l
```

**Pattern 3: Separate success/error streams**
```bash
haforu batch < jobs.json | \
  tee >(jq 'select(.status == "success")' > success.jsonl) | \
  jq 'select(.status == "error")' > errors.jsonl
```

**Pattern 4: Retry failed jobs**
```bash
# Extract failed job IDs
haforu batch < jobs.json | \
  jq -r 'select(.status == "error") | .id' > failed-ids.txt

# Regenerate and retry
cat jobs.json | jq '.jobs |= map(select(.id | IN($ids[])))' \
  --slurpfile ids failed-ids.txt | \
  haforu batch
```

## Performance Tuning

### Cache Configuration

**Rule of thumb:**
- `--max-fonts`: Number of unique fonts in your dataset
- `--max-glyphs`: ~50-100 per font for typical use

**Examples:**
```bash
# Small dataset (10 fonts, 50 glyphs each)
haforu batch --max-fonts 16 --max-glyphs 512 < jobs.json

# Large dataset (100 fonts, 1000 glyphs each)
haforu batch --max-fonts 128 --max-glyphs 2048 < jobs.json
```

### Parallel Processing

Use `--jobs` to control parallelism:

```bash
# Single-threaded (good for debugging)
haforu batch --jobs 1 < jobs.json

# Max parallelism (default: CPU count)
haforu batch --jobs 0 < jobs.json

# Custom worker count
haforu batch --jobs 4 < jobs.json
```

### Metrics-Only Mode

For fast bulk analysis, use `format: "metrics"`:

**Performance:**
- Metrics mode: ~0.2ms per job
- Image mode: ~2-5ms per job
- **Speedup: 10-25×**

```bash
# Convert jobs to metrics-only
jq '.jobs[].rendering.format = "metrics"' < jobs.json | haforu batch
```

### Statistics Monitoring

Enable `--stats` to monitor throughput:

```bash
haforu batch --stats < large-jobs.json 2> stats.json > results.jsonl
```

**Stats output (JSON to stderr):**
```json
{
  "jobs_processed": 1000,
  "jobs_success": 985,
  "jobs_error": 15,
  "elapsed_ms": 2453.2,
  "jobs_per_sec": 407.6,
  "cache": {
    "fonts_loaded": 12,
    "fonts_cached": 12,
    "glyphs_cached": 850
  }
}
```

## Examples

### Example 1: Batch Process with Metrics

```bash
cat > jobs.json <<'EOF'
{
  "version": "1.0",
  "jobs": [
    {
      "id": "arial-A",
      "font": {"path": "Arial.ttf", "size": 256, "variations": {}},
      "text": {"content": "A"},
      "rendering": {"format": "metrics", "encoding": "json", "width": 64, "height": 64}
    },
    {
      "id": "arial-B",
      "font": {"path": "Arial.ttf", "size": 256, "variations": {}},
      "text": {"content": "B"},
      "rendering": {"format": "metrics", "encoding": "json", "width": 64, "height": 64}
    }
  ]
}
EOF

haforu batch < jobs.json | jq '.metrics'
```

**Output:**
```json
{"density":0.6270029105392156,"beam":0.014404296875}
{"density":0.5234375,"beam":0.01220703125}
```

### Example 2: Variable Font Exploration

```bash
# Generate jobs for different weights
for wght in 100 200 300 400 500 600 700 800 900; do
  cat <<EOF
{"id":"wght-$wght","font":{"path":"Variable.ttf","size":256,"variations":{"wght":$wght}},"text":{"content":"A"},"rendering":{"format":"metrics","encoding":"json","width":64,"height":64}}
EOF
done | haforu stream | jq '{id, density: .metrics.density}'
```

**Output:**
```json
{"id":"wght-100","density":0.421}
{"id":"wght-200","density":0.453}
{"id":"wght-300","density":0.489}
...
```

### Example 3: Parallel Batch with Error Handling

```bash
#!/bin/bash
set -euo pipefail

# Process jobs with error handling
haforu batch --jobs 8 --max-fonts 64 < large-jobs.json | \
  tee results.jsonl | \
  jq -r 'select(.status == "error") | "\(.id): \(.error)"' | \
  tee errors.log

# Check if any errors occurred
if [ -s errors.log ]; then
  echo "Errors occurred during processing:"
  cat errors.log
  exit 1
fi

echo "All jobs completed successfully"
```

### Example 4: Stream Processing Pipeline

```bash
# Generate jobs → process → aggregate metrics
seq 1 1000 | \
  awk '{print "{\"id\":\"" $1 "\",\"font\":{\"path\":\"font.ttf\",\"size\":256,\"variations\":{}},\"text\":{\"content\":\"" $1 "\"},\"rendering\":{\"format\":\"metrics\",\"encoding\":\"json\",\"width\":64,\"height\":64}}"}' | \
  haforu stream | \
  jq -s '{total: length, avg_density: ([.[].metrics.density] | add / length)}'
```

### Example 5: Font Comparison

```bash
# Compare same glyph across multiple fonts
for font in Font1.ttf Font2.ttf Font3.ttf; do
  haforu render -f "$font" -s 256 -t "A" --format metrics | \
    jq --arg font "$font" '{font: $font, density, beam}'
done
```

**Output:**
```json
{"font":"Font1.ttf","density":0.627,"beam":0.0144}
{"font":"Font2.ttf","density":0.543,"beam":0.0122}
{"font":"Font3.ttf","density":0.689,"beam":0.0156}
```

## See Also

- [Python API Documentation](./PYTHON-API.md)
- [Architecture Overview](../README.md#architecture)
- [Installation Guide](../INSTALL.md)
- [Performance Benchmarks](../WORK.md)

## Support

- **Issues**: https://github.com/fontlaborg/haforu/issues
- **Examples**: See `examples/` directory in the repository
- **Tests**: See `scripts/test-cli-parity.sh` for comprehensive CLI tests
