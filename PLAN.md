---
this_file: external/haforu2/PLAN.md
---

# Haforu (canonical) — Plan for Package Structure and Integration

## Executive Summary: Why Haforu Exists

**Problem:** FontSimi cannot scale beyond 250 fonts without running out of memory
- Current: 250 fonts require 86GB RAM, take 5 hours
- Target: 1,000 fonts would require **344GB RAM** (impossible on any standard machine)
- Cause: 5.5M individual render calls crossing Python→Native boundary

**Solution:** Haforu batch renderer
- Processes 5000+ renders in one subprocess call
- Memory-mapped fonts (250MB vs 86GB)
- Expected: 100× speedup, 97% memory reduction
- **Makes 1,000 font analysis feasible** (<8GB RAM, ~12 minutes)

**Status:** 25% complete - blocked on 4-6 hours of API fixes

---

## Package Structure and Naming

Status: Adopt canonical package name `haforu` (no "2") and align Rust crate, Python bindings, and CLI to integrate cleanly with `fontsimi` as a high‑performance renderer++.

Why: `haforu2/` is the working folder for the next edition, but the publishable artifact names must be `haforu` across crate, module, binary, and Python package. This avoids split branding and simplifies integration in `fontsimi`.

---

## Canonical Naming Policy

- Crate name: `haforu`
- Library target: `haforu`
- CLI binary: `haforu`
- Python project name: `haforu`
- Python extension module: `haforu._haforu`
- Import path in Python: `import haforu`

Notes:
- The on‑disk folder here remains `haforu2/` during development. All code identifiers and published artifacts must drop the "2".
- Remove all `haforu2` identifiers in code, manifests, tests, docs, and build scripts.

---

## Manifest and Layout Requirements

- Cargo.toml
  - `[package].name = "haforu"`
  - `[lib].name = "haforu"`
  - `[[bin]].name = "haforu"`, `path = "src/main.rs"`
- pyproject.toml
  - `[project].name = "haforu"`
  - `[tool.maturin].module-name = "haforu._haforu"`
  - Update repository URLs once migrated
- .gitignore
  - Python artifacts under `python/haforu/`
- README/CLI usage
  - Replace all `haforu2` invocations with `haforu`

---

## Public Interfaces Required by fontsimi

1) CLI batch mode (renderer++)
   - stdin: single JSON `JobSpec` with `{"version":"1.0","jobs":[...]}`
   - stdout: JSONL, one `JobResult` per line, immediate flush
   - success result rendering payload:
     - `rendering.format = "pgm"`
     - `rendering.encoding = "base64"`
     - `rendering.data = <base64 P5 bytes>`
     - `rendering.width`, `rendering.height` (int)
     - `rendering.actual_bbox = [x0,y0,x1,y1]` (optional but recommended)
   - error result:
     - `status = "error"`, `error = <string>`

2) CLI streaming mode (deep matching optimization)
   - long‑lived process `haforu stream`
   - stdin: one `Job` JSON per line
   - stdout: one `JobResult` JSON per line (flush per job)
   - persistent font instance cache across jobs

3) Python bindings (maturin/pyo3)
   - module `haforu._haforu` exports minimal API:
     - `process_jobs(spec_json: str) -> Iterable[str]` yielding JSONL JobResult lines
     - `start_streaming() -> StreamingSession` with `send(job_json: str) -> str` and `close()`
   - Keep API tiny; `fontsimi` relies primarily on the CLI; bindings are a bonus.

---

## Compatibility Contract with fontsimi

- Output image: 8‑bit grayscale, PGM P5 in base64
- Metrics fidelity: images must produce identical Daidot metrics as existing renderers
- Determinism: repeatable results with identical inputs

### Performance Targets

**For 250 Fonts (Current Test Set):**
- Analysis time: <3 minutes (vs 5 hours current)
- Memory usage: <2GB (vs 86GB current)
- Success rate: 100% (vs frequent OOM crashes)

**For 1,000 Fonts (User Goal - Currently Impossible):**
- Analysis time: ~12 minutes (vs ~20 hours theoretical)
- Memory usage: <8GB (vs ~344GB impossible requirement)
- Success rate: 100% (vs 0% - always crashes)

**Baseline Targets:**
- Single render: <100ms
- 1,000 renders: <10s
- Memory footprint: <500MB during heavy batches
- No memory leaks over millions of renders

---

## Migration Tasks (rename everywhere)

1. Rename manifests
   - Cargo.toml: `haforu2` → `haforu` (package, lib, bin)
   - pyproject.toml: `haforu2` → `haforu`; module‑name → `haforu._haforu`

2. Code and paths
   - Adjust `use`, `mod`, and crate references to `haforu`
   - Update Python package paths to `python/haforu/`

3. Tooling and docs
   - README/CLI examples: `haforu2` → `haforu`
   - .gitignore patterns: `python/haforu/*`
   - CI/scripts expecting `haforu2` → `haforu`

4. Validation
   - `cargo build && cargo test`
   - `maturin develop` then `python -c "import haforu; print(haforu.__name__)"`
   - End‑to‑end smoke via `fontsimi` HaforuRenderer

---

## Risks and Edge Cases

- Name collision with legacy repo symlink `./haforu/`: treat as legacy; ensure PATH resolution prefers the new `haforu` binary under this workspace for dev.
- Windows multiprocessing: pin `workers = 1` when needed (documented in fontsimi).
- API drift in font crates: maintain a COMPILATION_FIXES.md mapping of upstream changes to code.

---

## Test Strategy

- Unit tests per module (batch, fonts, shaping, render, output)
- Integration test with real fonts (static and variable)
- Python smoke test for base64 PGM decode and numpy conversion
- fontsimi contract tests: JSONL schema, Daidot metric parity, perf baselines

