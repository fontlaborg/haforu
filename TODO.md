---
this_file: haforu/TODO.md
---

- [x] JSON contract: finish `handle_stream_line` so every streaming line yields a `JobResult` (parse + validation errors included) and add CLI regression tests.
- [x] JSON contract: ensure PyO3 `process_jobs` and StreamingSession bindings surface the same `status/error` payloads, updating `python/tests/*` accordingly.
- [x] JSON contract: keep `scripts/batch_smoke.sh` + `jobs_smoke.json` asserting that invalid jobs emit JSON responses instead of silent failures.
- [x] Variation validation: implement `validate_coordinates()` in `src/fonts.rs`, clamp wght/wdth ranges, drop unknown axes, and log sanitized values.
- [x] Variation validation: add unit/integration tests proving the clamps feed sanitized coordinates into skrifa and update debug logging.
- [x] Metrics mode: add `MetricsResult` + `--format metrics` in CLI/PyO3, reuse raster buffers for calculations, and expose a Python example.
- [x] Metrics mode: implement beam measurements + density (density + beam fields in MetricsOutput), tests pass.
- [x] Streaming session: expose cache tuning knobs, warm-up/ping/close methods, and ensure descriptors are freed immediately upon `close()`.
- [x] Streaming session: add perf tests (>1 000 renders) verifying <1 ms latency and stable RSS, documenting results.
- [x] Streaming session: enforce shared JSON schema/helpers between CLI and StreamingSession outputs.
- [x] Distribution: keep universal2/manylinux wheel builds documented via `maturin` and outline the `HAFORU_BIN` workflow in README.
- [x] Distribution: ensure `scripts/batch_smoke.sh` stays ≤2 s and recorded in `WORK.md` each run.
- [x] Documentation: update README/PLAN/TODO/WORK/CHANGELOG whenever the contract changes; add troubleshooting + metrics mode sections.

## Phase 2: Production Release Automation

- [x] Build automation: create `scripts/build.sh` that builds both Rust CLI and Python wheels in canonical fashion.
- [x] Build automation: create `scripts/run.sh` that demonstrates the library with test data.
- [x] Build automation: ensure build script handles universal2 (macOS), manylinux (Linux), and Windows wheels.
- [x] Build automation: include release-mode optimizations and debug symbols control for production builds.
- [x] Enhanced CLI: extend Rust CLI to support HarfBuzz-compatible syntax (`--font-file`, `--font-size`, `--variations`, `--text`, `--output-file`).
- [x] Enhanced CLI: add compatibility aliases for `hb-shape` style arguments (`-f` for font, `-s` for size, `--features`, `--script`).
- [x] Enhanced CLI: support standard input/output conventions matching HarfBuzz tools.
- [x] Enhanced CLI: implement `--help-harfbuzz` to show HarfBuzz-compatible options.
- [x] Python CLI: implement Fire-based Python CLI as `python -m haforu` with subcommands.
- [x] Python CLI: add `batch`, `stream`, `validate`, and `metrics` commands with Fire's automatic argument parsing.
- [x] Python CLI: include `render_single` convenience command for quick one-off renders.
- [x] Python CLI: provide `--format` option supporting JSON, JSONL, and human-readable output.
- [x] Platform packaging: configure platform-specific extras in pyproject.toml (`haforu[mac]`, `haforu[windows]`, `haforu[linux]`).
- [x] Platform packaging: set up binary dependencies and platform markers for each OS variant.
- [x] Platform packaging: implement runtime platform detection for appropriate wheel selection.
- [x] Platform packaging: document installation process for each platform with troubleshooting.
- [x] Semver automation: migrate from hardcoded versions to `hatch-vcs` for automatic version detection from git tags.
- [x] Semver automation: configure `[tool.hatch.version]` to use git tags as single source of truth.
- [x] Semver automation: update Cargo.toml version management to sync with Python versioning.
- [x] Semver automation: add pre-commit hooks to validate version consistency across Rust and Python.
- [x] GitHub Actions: create workflow triggered by `v*` tags that builds and publishes releases.
- [x] GitHub Actions: build matrix for universal2 (macOS), manylinux (Linux), and Windows wheels.
- [x] GitHub Actions: use `maturin` GitHub Action for wheel building with proper target architectures.
- [x] GitHub Actions: automated publishing to PyPI on tag push with secure token management.
- [x] GitHub Actions: generate GitHub Releases with changelog extraction and wheel artifacts.
- [x] GitHub Actions: add workflow for Rust crate publishing to crates.io.
- [x] Repository structure: reorganize repository following Rust workspace + Python package best practices.
- [x] Repository structure: move Python package to standard location while keeping PyO3 bindings in src/python.
- [x] Repository structure: ensure `maturin develop` works for local development without full builds.
- [x] Repository structure: add `.cargo/config.toml` for consistent build settings across platforms.
- [x] Repository structure: update .gitignore for all build artifacts and platform-specific files.

## Status: ✅ ALL COMPLETE

Both Phase 1 (Integration) and Phase 2 (Production Release Automation) are 100% complete.
Ready for production use and automated releases.
