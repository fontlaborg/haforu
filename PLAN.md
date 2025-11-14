---
this_file: haforu/PLAN.md
---

# FontSimi Integration Plan

## Objective
Deliver a zero-drama renderer that FontSimi can call from either the CLI or StreamingSession. Every job must emit deterministic JSONL with explicit status codes, validated variation coordinates, optional metrics-only payloads, and sub-millisecond warmed streaming performance.

## Workstreams

### 1. JSON Contract & Error Surfacing (src/main.rs, src/batch.rs, python/tests)
- Ensure every stdin line (batch JSON blobs and streaming JSONL lines) produces a serialized `JobResult`, even if parsing or validation fails.
- Normalize error messaging so the CLI, PyO3 bindings, and smoke scripts log once but return actionable `status="error"` payloads with `error` text.
- Provide helper constructors (e.g., `JobResult::error`) and thin wrappers inside Python bindings so fontsimi can rely on stable schema.
- Extend regression coverage: CLI streaming unit tests, Python test suite, and `scripts/batch_smoke.sh` must all assert that invalid jobs return JSON results rather than silent drops.

### 2. Variation Coordinate Validation (src/fonts.rs)
- Add `validate_coordinates()` with clamps for `wght` [100, 900] and `wdth` [50, 200], warn-and-drop unknown axes, and reuse standard axis metadata.
- Wire validation into `FontLoader::load_font` so both CLI and Python bindings inherit the behavior.
- Surface sanitized coordinates in the JSON/log output for debugging.
- Add focused unit tests covering in-range, out-of-range, and unknown-axis cases plus an integration test that confirms `skrifa` receives sanitized values.

### 3. Metrics-Only Output Mode (src/output.rs, src/main.rs, examples/python)
- Introduce a `MetricsResult` struct and `--format metrics` flag that short-circuits image encoding and emits density + beam measurements as JSON.
- Reuse the existing raster buffer to compute metrics without extra allocations; clamp runtime <0.2 ms/job.
- Update CLI help, README, and Python bindings to describe the new format, and add an example in `examples/python/metrics_demo.py` that doubles as a smoke test.
- Benchmark against the current image mode and record numbers in `WORK.md` + `CHANGELOG.md` once stable.

### 4. Streaming Session Reliability (src/render.rs, src/python/streaming.rs)
- Implement cache knobs (`max_fonts`, `max_glyphs`), warm-up hooks, and microsecond `is_available()` checks so fontsimi can decide between CLI and in-process.
- Add `StreamingSession::warm_up`, `ping`, and `close` behaviors that release descriptors and reset caches immediately.
- Stress-test with >1 000 sequential renders to guarantee <1 ms steady-state latency and no RSS creep (document in WORK/CHANGELOG).
- Ensure JSON schema parity between CLI and StreamingSession by sharing the output helper code.

### 5. Distribution & Tooling (scripts/batch_smoke.sh, wheels)
- Keep `scripts/batch_smoke.sh` + `jobs_smoke.json` green in ≤2 s to validate CLI contract before publishing.
- Maintain universal2/manylinux wheels via `maturin` and document exact install + `HAFORU_BIN` instructions for fontsimi integration.
- Update `PLAN.md`, `TODO.md`, `WORK.md`, and `CHANGELOG.md` every time the contract changes so downstream teams can sync quickly.

## Testing & Validation
- **Unit**: Rust modules (batch, fonts, streaming) plus Python bindings each gain targeted tests for new helpers.
- **Integration**: `scripts/batch_smoke.sh`, `examples/python/*.py`, and `smoke_test.rs` must run after every major change.
- **Performance**: Record jobs/sec + RSS for batch streaming and StreamingSession warm-up time; gate merges on regressions.
- **Edge cases**: Empty text, zero-sized canvases, bad fonts, missing axes, and invalid JSONL lines must all produce `status="error"` without panics.

## Success Criteria
- CLI streaming never drops jobs; every line results in a JSONL response within 10 ms.
- Variation clamps keep requests within spec and log actionable warnings.
- Metrics mode yields ≥10× faster runtimes for tentpole metrics and is selected via a single flag/API parameter.
- StreamingSession warm-up completes <50 ms, steady-state renders stay <1 ms, and caches can be tuned without rebuilding.
- Packaging + smoke tooling stay documented and reproducible so fontsimi can mirror the workflow.

## Out of Scope
- New render formats beyond PGM/PNG/metrics.
- Analytics, monitoring, or retry systems.
- Color/emoji/subpixel rendering features.
