---
this_file: CLAUDE.md
---

# Haforu Development Guide

**Working on:** Haforu font renderer at `/Users/adam/Developer/vcs/github.fontlaborg/haforu`

## Core Mission

Fast, deterministic font rendering for CLI and Python. That's it.

- **Input:** Font path + text + rendering params (size, variations, canvas dimensions)
- **Output:** Rendered glyph as PGM/PNG/metrics JSON
- **Interfaces:** Rust CLI + Python bindings
- **Performance:** Sub-millisecond warmed rendering, parallel batch processing

## Project Structure

```
src/
├── main.rs         # CLI (batch, stream, render commands)
├── lib.rs          # Public API
├── batch.rs        # Job specs and results
├── fonts.rs        # Font loading + LRU cache
├── shaping.rs      # HarfBuzz text shaping
├── render.rs       # Zeno rasterization
├── output.rs       # PGM/PNG/metrics encoding
└── error.rs        # Error types

python/
├── haforu/         # PyO3 bindings
│   ├── __init__.py     # StreamingSession API
│   └── __main__.py     # Fire-based CLI wrapper
└── tests/          # Python test suite
```

## What We Do

1. **CLI Tool** - Read jobs from stdin (JSON batch or JSONL stream), render glyphs in parallel, output JSONL results
2. **Python Bindings** - Persistent `StreamingSession` for sub-ms rendering with font caching
3. **Three Output Modes:**
   - `pgm` - Grayscale image (Netpbm format)
   - `png` - PNG image
   - `metrics` - Just density + beam measurements (10× faster)

## What We Don't Do

- No complex build systems or release automation
- No repository structure bikeshedding
- No analytics, monitoring, or telemetry
- No elaborate caching strategies beyond simple LRU
- No retry logic, circuit breakers, or resilience patterns
- No extensive validation beyond basics

## Development Workflow

### Before Starting Work

1. Read `PLAN.md` - What needs to be done
2. Read `TODO.md` - Flat task list
3. Update `WORK.md` - Note what you're working on

### Making Changes

1. **Keep it simple** - If it doesn't improve performance or fix a bug, don't add it
2. **Test locally:**
   ```bash
   cargo test                    # Rust unit tests
   cargo run --release -- batch < scripts/jobs_smoke.jsonl  # CLI smoke test
   ```
3. **Python changes:**
   ```bash
   uv pip install -e .           # Install dev build
   python -m pytest python/tests # Python tests
   ```

### After Changes

1. Update `WORK.md` with what you did
2. Update `CHANGELOG.md` with user-visible changes
3. Check off items in `TODO.md` and `PLAN.md`

## Code Principles

- **Flat modules** - No deep abstraction hierarchies
- **Explicit data flow** - Job → Load font → Shape text → Render → Output
- **Fast paths** - Memory-mapped fonts, LRU caches, parallel processing
- **Deterministic errors** - Every failed job returns JSON with `status: "error"`
- **Zero-copy where possible** - memmap2 for fonts, direct numpy arrays in Python

## Testing

- **Unit tests** - Rust modules test their core logic
- **Smoke tests** - `scripts/jobs_smoke.jsonl` validates CLI contract
- **Python tests** - `python/tests/` validates bindings and error handling

## Performance Targets

- Single render: <10ms cold, <2ms warm (Python bindings)
- Batch (1000 jobs): <10s on 8 cores
- Streaming: <1ms per job (warmed cache)
- Metrics mode: <0.2ms per job

## Common Tasks

### Build CLI
```bash
cargo build --release
export HAFORU_BIN="$PWD/target/release/haforu"
```

### Build Python Wheels
```bash
uv tool install maturin
uv run maturin develop          # Dev install
uv run maturin build --release  # Build wheel
```

### Run Smoke Tests
```bash
./scripts/batch_smoke.sh        # CLI validation (~2s)
python -m pytest python/tests   # Python tests
```

### Profile Performance
```bash
# CLI hot paths
./scripts/profile-cli.sh

# Python bindings
python examples/python/streaming_demo.py
```

## Integration with FontSimi

This renderer exists to serve fontsimi's font matching pipeline:

1. **Batch Analysis** - Process thousands of glyphs via CLI streaming
2. **Deep Matching** - Use Python `StreamingSession` for <1ms repeated renders
3. **Metrics Mode** - Skip image encoding for similarity scoring

## Anti-Patterns to Avoid

- Adding commands that aren't about rendering glyphs
- Creating abstraction layers "for future extensibility"
- Adding configuration for configuration's sake
- Building "production-ready" infrastructure (it's a tool, not a service)
- Implementing features nobody asked for

## Golden Rule

**If it doesn't make rendering faster or more reliable, don't add it.**
