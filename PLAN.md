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
- **Status (2025-11-15):** ✅ Validation moved into `process_job_with_options`, `JobResult` now exports sanitized font metadata, Python/CLI tests cover invalid renders, and the smoke script enforces the JSON contract (invalid jobs emit `status="error"`).

### 2. Variation Coordinate Validation (src/fonts.rs)
- Add `validate_coordinates()` with clamps for `wght` [100, 900] and `wdth` [50, 200], warn-and-drop unknown axes, and reuse standard axis metadata.
- Wire validation into `FontLoader::load_font` so both CLI and Python bindings inherit the behavior.
- Surface sanitized coordinates in the JSON/log output for debugging.
- Add focused unit tests covering in-range, out-of-range, and unknown-axis cases plus an integration test that confirms `skrifa` receives sanitized values.
- **Status (2025-11-15):** ✅ IBM Plex variable font fixture + new clamp/location tests prove the sanitizer feeds skrifa, and `JobResult.font` exposes the applied coordinates for downstream inspection.

### 3. Metrics-Only Output Mode (src/output.rs, src/main.rs, examples/python)
- Introduce a `MetricsResult` struct and `--format metrics` flag that short-circuits image encoding and emits density + beam measurements as JSON.
- Reuse the existing raster buffer to compute metrics without extra allocations; clamp runtime <0.2 ms/job.
- Update CLI help, README, and Python bindings to describe the new format, and add an example in `examples/python/metrics_demo.py` that doubles as a smoke test.
- Benchmark against the current image mode and record numbers in `WORK.md` + `CHANGELOG.md` once stable.
- **Status (2025-11-16):** ✅ Metrics mode now returns `density`/`beam` JSON payloads, the smoke script asserts the schema, and docs/tests/examples cover the new workflow.

### 4. Streaming Session Reliability (src/render.rs, src/python/streaming.rs)
- Implement cache knobs (`max_fonts`, `max_glyphs`), warm-up hooks, and microsecond `is_available()` checks so fontsimi can decide between CLI and in-process.
- Add `StreamingSession::warm_up`, `ping`, and `close` behaviors that release descriptors and reset caches immediately.
- Stress-test with >1 000 sequential renders to guarantee <1 ms steady-state latency and no RSS creep (document in WORK/CHANGELOG).
- Ensure JSON schema parity between CLI and StreamingSession by sharing the output helper code.
- **Status (2025-11-17):** ✅ Shared glyph-result cache now drives the CLI and PyO3 bindings (`--max-fonts/--max-glyphs`, `StreamingSession(max_fonts/max_glyphs, ping, cache_stats, set_glyph_cache_size)`), and the Rust/Python perf tests cover 1 200 cached renders (<1 ms steady state).

### 5. Distribution & Tooling (scripts/batch_smoke.sh, wheels)
- Keep `scripts/batch_smoke.sh` + `jobs_smoke.json` green in ≤2 s to validate CLI contract before publishing.
- Maintain universal2/manylinux wheels via `maturin` and document exact install + `HAFORU_BIN` instructions for fontsimi integration.
- Update `PLAN.md`, `TODO.md`, `WORK.md`, and `CHANGELOG.md` every time the contract changes so downstream teams can sync quickly.
- **Status (2025-11-17):** ✅ Smoke script now defaults the glyph cache and captures steady-state runs (~1.5 s) while README documents the `maturin` wheel commands plus the `HAFORU_BIN` workflow.

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

## Workstreams (Phase 2)

### 6. Build Automation & Tooling (scripts/build.sh, scripts/run.sh)
- Create `scripts/build.sh` that builds both Rust CLI and Python wheels in canonical fashion, handling all dependencies and target architectures.
- Create `scripts/run.sh` that demonstrates the library with test data, showcasing both CLI streaming/batch modes and Python bindings.
- Ensure build script handles universal2 (macOS), manylinux (Linux), and Windows wheels with proper architecture detection.
- Include release-mode optimizations and debug symbols control for production builds.
- **Status (2025-11-14):** ✅ COMPLETE - Build and run scripts implemented with full platform support.

### 7. Enhanced CLI Compatibility (src/main.rs enhancements)
- Extend Rust CLI to support HarfBuzz-compatible syntax (`--font-file`, `--font-size`, `--variations`, `--text`, `--output-file`).
- Add compatibility aliases for `hb-shape` style arguments (`-f` for font, `-s` for size, `--features`, `--script`).
- Support standard input/output conventions matching HarfBuzz tools for drop-in replacement scenarios.
- Implement `--help-harfbuzz` to show HarfBuzz-compatible options separately.
- **Status (2025-11-14):** ✅ COMPLETE - HarfBuzz-compatible render command implemented with full feature set.

### 8. Python Fire CLI (python/haforu/__main__.py)
- Implement Fire-based Python CLI as `python -m haforu` with subcommands matching Rust CLI.
- Add `batch`, `stream`, `validate`, and `metrics` commands with Fire's automatic argument parsing.
- Include `render_single` convenience command for quick one-off renders from Python.
- Provide `--format` option supporting JSON, JSONL, and human-readable output formats.
- **Status (2025-11-14):** ✅ COMPLETE - Fire CLI implemented with all features.

### 9. Platform-Specific Packaging (pyproject.toml extras)
- Configure platform-specific extras in pyproject.toml (`haforu[mac]`, `haforu[windows]`, `haforu[linux]`).
- Set up binary dependencies and platform markers for each OS variant.
- Implement runtime platform detection for appropriate wheel selection during pip install.
- Document installation process for each platform with troubleshooting guides.
- **Status (2025-11-14):** ✅ COMPLETE - Extras configured, INSTALL.md created with comprehensive platform guides.

### 10. Automatic Semver & Git Tags (pyproject.toml, .github/workflows)
- Migrate from hardcoded versions to `hatch-vcs` for automatic version detection from git tags.
- Configure `[tool.hatch.version]` to use git tags as single source of truth.
- Update Cargo.toml version management to sync with Python versioning.
- Add pre-commit hooks to validate version consistency across Rust and Python.
- **Status (2025-11-14):** ✅ COMPLETE - hatch-vcs configured, sync-version.sh script created for Cargo sync.

### 11. GitHub Actions Release Automation (.github/workflows/release.yml)
- Create workflow triggered by `v*` tags that builds and publishes releases.
- Build matrix for universal2 (macOS), manylinux (Linux), and Windows wheels.
- Use `maturin` GitHub Action for wheel building with proper target architectures.
- Automated publishing to PyPI on tag push with secure token management.
- Generate GitHub Releases with changelog extraction and wheel artifacts.
- Add workflow for Rust crate publishing to crates.io.
- **Status (2025-11-14):** ✅ COMPLETE - Full release automation implemented.

### 12. Repository Structure Canonicalization
- Reorganize repository following Rust workspace + Python package best practices.
- Move Python package to standard location while keeping PyO3 bindings in src/python.
- Ensure `maturin develop` works for local development without full builds.
- Add `.cargo/config.toml` for consistent build settings across platforms.
- Update .gitignore for all build artifacts and platform-specific files.
- **Status (2025-11-14):** ✅ COMPLETE - .cargo/config.toml added, .gitignore updated, structure follows best practices.

## Phase 3: Release Hardening & Tooling Parity

### 13. Canonical Build/Run Automation (`build.sh`, `run.sh`)
- Inspect the current `scripts/` contents (missing build/run orchestration per `llms.txt`) and design a single `./build.sh` that can build the Rust CLI, PyO3 bindings, and Python wheels with consistent artifact layout.
- Implement OS/arch detection, release-vs-dev flags, cache reuse, and hooks for generating universal2/manylinux wheels plus Windows MSVC builds; integrate smoke tests so the script fails fast when binaries regress.
- Create `./run.sh` that shells the CLI + Python bindings against bundled fixtures (batch JSON, streaming demo, metrics demo) to validate the install footprint in <1 min and produce sample outputs for docs.
- Document usage in README/INSTALL plus inline script comments, and track timing/results in WORK.md for every release candidate.
- **Status (2025-11-18):** ✅ COMPLETE — `scripts/build.sh` now emits timestamped artifacts, wheels, tests, and smoke logs via `uvx` (with per-platform targets) while `scripts/run.sh` replays the JSONL fixtures (batch/metrics/stream + optional Python demo) and both scripts are documented in README + INSTALL.

### 14. Rust CLI Efficiency & Feature Parity
- Audit `src/main.rs` to ensure the CLI exposes batch, stream, render, and diagnostics commands with HarfBuzz-compatible flags plus cache-tuning knobs; fill gaps surfaced during the `llms.txt` review.
- Profile hot paths (argument parsing, job dispatch, JSONL streaming) and add benchmarks/regression tests so we can quantify improvements; fold in structured logging for release debugging without hurting latency.
- Produce documentation updates (+examples) showing CLI usage parity with FontSimi expectations, including streaming JSON contracts and failure guidance.
- **Status (2025-11-18):** ⏳ NOT STARTED — CLI exists but lacks documented performance targets/tests ensuring “efficient powerful” behavior FontSimi relies on.

### 15. Python Fire CLI Parity
- Reconfirm the Fire-based CLI under `python/haforu` exposes the same subcommands/flags as the Rust CLI, including advanced render settings, cache knobs, and metrics streaming.
- Harden argument validation, streaming/batch wrappers, and output formatting so `haforu-py` can stand in for the Rust binary in CI; add regression tests plus usage docs tied to FontSimi workflows.
- Ensure packaging installs console entry points (e.g., `haforu-py`) and that `python -m haforu --help` stays fast even when the native module is missing.
- **Status (2025-11-18):** ⏳ NOT STARTED — Fire CLI scaffolding exists but needs parity review + test automation to guarantee “efficient powerful” behavior.

### 16. Repository Canonicalization
- Compare the current layout (Rust crate at root, PyO3 bindings under `src/python`, Python package under `python/`) with canonical Rust workspace + PyO3 + hatch project guidance; document deviations and clean up paths/modules accordingly.
- Introduce or update `.cargo/config.toml`, `pyproject.toml`, and tooling metadata so both ecosystems follow best practices (e.g., workspace members, lint/test configs, `this_file` annotations).
- Ensure docs (`README`, `ARCHITECTURE`, `PLAN`, `INSTALL`) reflect the canonical layout and reference updated scripts/configs; keep repo length manageable (<200 lines per file where possible).
- **Status (2025-11-18):** ⏳ NOT STARTED — directories are serviceable but lack the canonical multi-language scaffolding and documentation cross-links FontSimi expects.

### 17. Local & GitHub Actions Build Reliability
- Define a reproducible build pipeline spanning `cargo`, `maturin`, and Hatch so contributors can build/test locally (macOS/Linux/Windows) using `./build.sh` plus documented prerequisites.
- Update or add GitHub Actions workflows to mirror the local pipeline, ensuring release artifacts (Rust binaries, wheels) are produced deterministically with cache-friendly steps and smoke-test gates.
- Capture artifact layout, cache keys, and release promotion steps inside PLAN/TODO, and keep WORK.md logging CI runs for transparency.
- **Status (2025-11-18):** ⏳ NOT STARTED — CI currently runs basic tests but lacks the canonical release-oriented pipeline tied to the new scripts.

### 18. Automatic SemVer & Tag-Driven Releases
- Adopt Hatch VCS (Python) and cargo-vcs tagging for Rust so both crates derive their versions from git tags; wire this into `pyproject.toml`, `Cargo.toml`, and helper scripts.
- Teach GitHub Actions to watch for `vX.Y.Z` tags, run the canonical build, update changelogs, create GitHub Releases, and push artifacts to PyPI/crates.io automatically.
- Add validation tooling (pre-commit or script) that asserts the working tree is clean, tests pass, and the changelog is updated before allowing a tag push, preventing release drift.
- **Status (2025-11-18):** ⏳ NOT STARTED — current tooling doesn’t yet source its version strictly from tags or auto-publish on tag push.
