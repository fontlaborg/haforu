---
this_file: PLAN.md
---

## Objective

Deliver a zero-drama renderer that other packages can call from either the CLI or StreamingSession. Every job must emit deterministic JSONL with explicit status codes, validated variation coordinates, optional metrics-only payloads, and sub-millisecond warmed streaming performance.

## Active Workstreams

### JSON Contract & Error Surfacing (src/main.rs, src/batch.rs, python/tests)

- Every stdin line (batch JSON blobs and streaming JSONL lines) produces a serialized `JobResult`, even if parsing or validation fails.
- Normalize error messaging: CLI, PyO3 bindings, and smoke scripts log once but return actionable `status="error"` payloads with `error` text.
- Provide helper constructors (e.g., `JobResult::error`) and thin wrappers inside Python bindings for stable schema.
- Extend regression coverage: CLI streaming unit tests, Python test suite, and `scripts/batch_smoke.sh` must assert that invalid jobs return JSON results rather than silent drops.

### Variation Coordinate Validation (src/fonts.rs)

- Add `validate_coordinates()` with clamps for `wght` [100, 900] and `wdth` [50, 200], warn-and-drop unknown axes, reuse standard axis metadata.
- Wire validation into `FontLoader::load_font` so both CLI and Python bindings inherit the behavior.
- Surface sanitized coordinates in JSON/log output for debugging.
- Add focused unit tests covering in-range, out-of-range, and unknown-axis cases plus integration test confirming `skrifa` receives sanitized values.

### Metrics-Only Output Mode (src/output.rs, src/main.rs, examples/python)

- Introduce `MetricsResult` struct and `--format metrics` flag that short-circuits image encoding and emits density + beam measurements as JSON.
- Reuse existing raster buffer to compute metrics without extra allocations; clamp runtime <0.2 ms/job.
- Update CLI help, README, and Python bindings to describe the new format; add example in `examples/python/metrics_demo.py` that doubles as smoke test.
- Benchmark against current image mode and record numbers in `WORK.md` + `CHANGELOG.md` once stable.

### Streaming Session Reliability (src/render.rs, src/python/streaming.rs)

- Implement cache knobs (`max_fonts`, `max_glyphs`), warm-up hooks, and microsecond `is_available()` checks so other packages can decide between CLI and in-process.
- Add `StreamingSession::warm_up`, `ping`, and `close` behaviors that release descriptors and reset caches immediately.
- Stress-test with >1 000 sequential renders to guarantee <1 ms steady-state latency and no RSS creep (document in WORK/CHANGELOG).
- Ensure JSON schema parity between CLI and StreamingSession by sharing output helper code.

### Distribution & Tooling (scripts/batch_smoke.sh, wheels)

- Keep `scripts/batch_smoke.sh` + `jobs_smoke.json` green in ≤2 s to validate CLI contract before publishing.
- Maintain universal2/manylinux wheels via `maturin` and document exact install + `HAFORU_BIN` instructions for other packages integration.
- Update `PLAN.md`, `TODO.md`, `WORK.md`, and `CHANGELOG.md` every time the contract changes so downstream teams can sync quickly.

## Testing & Validation

- **Unit**: Rust modules (batch, fonts, streaming) plus Python bindings each gain targeted tests for new helpers.
- **Integration**: `scripts/batch_smoke.sh`, `examples/python/*.py`, and `smoke_test.rs` must run after every major change.
- **Performance**: Record jobs/sec + RSS for batch streaming and StreamingSession warm-up time; gate merges on regressions.
- **Edge cases**: Empty text, zero-sized canvases, bad fonts, missing axes, and invalid JSONL lines must all produce `status="error"` without panics.

## Success Criteria

- CLI streaming never drops jobs; every line results in a JSONL response within 10 ms.
- Variation clamps keep requests within spec and log actionable warnings.
- Metrics mode yields ≥10× faster runtimes for tentpole metrics and is selected via a single flag/API parameter.
- StreamingSession warm-up completes <50 ms, steady-state renders stay <1 ms, and caches can be tuned without rebuilding.
- Packaging + smoke tooling stay documented and reproducible so other packages can mirror the workflow.

## Out of Scope

- New render formats beyond PGM/PNG/metrics.
- Analytics, monitoring, or retry systems.
- Color/emoji/subpixel rendering features.

## Phase 3: Release Hardening & Tooling Parity

### Rust CLI Efficiency & Feature Parity

- Audit `src/main.rs` to ensure CLI exposes batch, stream, render, and diagnostics commands with HarfBuzz-compatible flags plus cache-tuning knobs; fill gaps surfaced during review.
- Profile hot paths (argument parsing, job dispatch, JSONL streaming) and add benchmarks/regression tests to quantify improvements; fold in structured logging for release debugging without hurting latency.
- Produce documentation updates (+examples) showing CLI usage parity with other package expectations, including streaming JSON contracts and failure guidance.

### Python Fire CLI Parity

- Reconfirm Fire-based CLI under `python/haforu` exposes same subcommands/flags as Rust CLI, including advanced render settings, cache knobs, and metrics streaming.
- Harden argument validation, streaming/batch wrappers, and output formatting so `haforu-py` can stand in for Rust binary in CI; add regression tests plus usage docs tied to other package workflows.
- Ensure packaging installs console entry points (e.g., `haforu-py`) and that `python -m haforu --help` stays fast even when native module is missing.

### Repository Canonicalization

- Compare current layout (Rust crate at root, PyO3 bindings under `src/python`, Python package under `python/`) with canonical Rust workspace + PyO3 + hatch project guidance; document deviations and clean up paths/modules accordingly.
- Introduce or update `.cargo/config.toml`, `pyproject.toml`, and tooling metadata so both ecosystems follow best practices (workspace members, lint/test configs, `this_file` annotations).
- Ensure docs (`README`, `ARCHITECTURE`, `PLAN`, `INSTALL`) reflect canonical layout and reference updated scripts/configs; keep repo length manageable (<200 lines per file where possible).

### Local & GitHub Actions Build Reliability

- Define reproducible build pipeline spanning `cargo`, `maturin`, and Hatch so contributors can build/test locally (macOS/Linux/Windows) using `./build.sh` plus documented prerequisites.
- Update or add GitHub Actions workflows to mirror local pipeline, ensuring release artifacts (Rust binaries, wheels) are produced deterministically with cache-friendly steps and smoke-test gates.
- Capture artifact layout, cache keys, and release promotion steps inside PLAN/TODO; keep WORK.md logging CI runs for transparency.

### Automatic SemVer & Tag-Driven Releases

- Adopt Hatch VCS (Python) and cargo-vcs tagging for Rust so both crates derive versions from git tags; wire into `pyproject.toml`, `Cargo.toml`, and helper scripts.
- Teach GitHub Actions to watch for `vX.Y.Z` tags, run canonical build, update changelogs, create GitHub Releases, and push artifacts to PyPI/crates.io automatically.
- Add validation tooling (pre-commit or script) that asserts working tree is clean, tests pass, and changelog is updated before allowing tag push, preventing release drift.
