---
this_file: haforu/CHANGELOG.md
---

# Changelog

## 2025-11-17
- Shared the glyph-result cache across the CLI and bindings with `--max-fonts/--max-glyphs` knobs, plus a safe `ExecutionOptions::new` constructor so downstream callers no longer touch cache internals directly.
- The PyO3 `StreamingSession` picked up parity helpers (`max_fonts/max_glyphs`, JSON-first `render`, `ping`, glyph-aware `cache_stats`, `set_glyph_cache_size`), and both the Rust + Python suites cover 1 200 cached renders to prove the <1 ms steady-state path.
- README/PLAN/TODO/WORK/CHANGELOG now describe the cache tuning workflow, smoke expectations, troubleshooting tips, and the exact `maturin` commands for universal2 + manylinux wheels / `HAFORU_BIN`.
- `scripts/batch_smoke.sh` stops rebuilding on every run (detects `target/release/haforu` up front) so the JSON contract runs finish in ~1 s once the release binary exists.
- Tests:
  - `cargo test` ✅
  - `uvx hatch test` ✅ (native module unavailable ⇒ expected skips)
  - `scripts/batch_smoke.sh` ✅ (~1.0 s steady state; the first run still pays for `cargo build --release`)

## 2025-11-16
- Added `MetricsOutput` to the JSON contract plus new `Image::density/beam` helpers so `rendering.format="metrics"` reuses the raster buffer to emit normalized density/beam measurements instead of base64 blobs; CLI and StreamingSession now share the implementation via `process_job_with_options`.
- Expanded the smoke suite (`scripts/jobs_smoke.jsonl`/`batch_smoke.sh`), Python bindings/tests, and the new `examples/python/metrics_demo.py` to assert the metrics-only schema and ensure the `rendering` field stays absent in this mode.
- Documented the workflow (README/PLAN/TODO/WORK) so downstream tools know how to request metrics and what fields to inspect.
- Tests:
  - `cargo test` ✅ (initially failed as expected until the metrics logic landed; final run passes all 33 lib + 9 CLI tests)
  - `uvx hatch test` ✅ (37 skipped – native module unavailable in this env)
  - `scripts/batch_smoke.sh` ✅ (~108 s on the first run because it rebuilt `target/release/haforu`; steady-state stays ~3.5 s)

## 2025-11-15
- Batch/streaming jobs now defer validation to `process_job_with_options`, guaranteeing every job (including invalid specs) emits a structured `JobResult`. `JobResult` gained an optional `font` payload that surfaces the sanitized font path + applied variations for downstream debugging.
- `process_job` and the PyO3 bindings now share the same validation/error path, and new Python tests cover invalid rendering parameters so bindings return JSON errors instead of `ValueError`s.
- `scripts/jobs_smoke.jsonl` moved under `scripts/` with an intentionally invalid job, and `scripts/batch_smoke.sh` now asserts the CLI JSON contract via a lightweight Python check (failing fast if an error is silently dropped).
- Added IBM Plex variable font fixture plus Rust unit/integration tests that prove variation clamps/drops feed sanitized coordinates into skrifa and the emitted JSON.
- README updated with the new `font` metadata description and refreshed smoke test instructions, and `testdata/fonts/README.md` now documents the bundled fixtures/licensing.
- Tests:
  - `cargo fmt` ✅
  - `cargo test` ✅ (29 lib tests, 9 CLI tests, 1 doc test)
  - `uvx hatch test` ✅ (35 skipped – native module unavailable in this env)
  - `scripts/batch_smoke.sh` ✅ (first run 105s due to `cargo build --release`, steady-state ~3.5s)

## 2025-11-14
- Added `render::Image` wrapper with `is_empty`, `calculate_bbox`, and safe `pixel_delta` to eliminate Δpx=inf cases and to centralize raster validation.
- Updated `GlyphRasterizer`, CLI pipeline, smoke tests, and PyO3 bindings to consume the new image type while keeping JSON output unchanged.
- Cleaned up Python bindings imports/visibility so the editable wheel still builds with the `python` feature.
- Hardened CLI streaming by routing every line through `handle_stream_line`, adding `JobResult::error`, and mapping missing fonts to the `FontNotFound` error for friendlier JSON responses; added unit coverage for the new helper.
- Pointed pytest at `python/tests` and added thin wrappers under `tests/` so `uvx hatch test` can discover the suite via Hatch.
- Tests:
  - `fd -e py -x uvx autoflake -i {}` ✅
  - `fd -e py -x uvx pyupgrade --py312-plus {}` ✅
  - `fd -e py -x uvx ruff check --output-format=github --fix --unsafe-fixes {}` ❌ (fails on long-standing lint violations in python/examples/tests; no repo changes kept)
  - `fd -e py -x uvx ruff format --respect-gitignore --target-version py312 {}` ✅ (auto-edits reverted to avoid semantic drift)
  - `uvx hatch test` ❌ (initial run: pytest could not find the legacy `tests/` directory before wrappers existed)
  - `cargo fmt && cargo test` ✅
  - `cargo test` ✅ (streaming error propagation tests)
  - `uvx hatch test` ✅ (test discovery succeeds; Python cases skipped because native module is not built in this env)

### Variation Validation Updates
- Changed: Unknown variation axes are now warned and dropped instead of causing errors, preventing unnecessary job failures when extra axes are supplied.
- Changed: Apply conservative clamps for `wght` [100, 900] and `wdth` [50, 200], intersected with font-provided bounds.
- Added: Unit test `load_static_font_drops_unknown_axes` in `src/fonts.rs` to verify static fonts ignore provided coordinates.
