---
this_file: haforu/CLAUDE.md
---

# Haforu Builder Notes

IMPORTANT: When you're working, REGULARLY remind me & yourself which folder you're working in and what project you're working on.

## Repository Structure
- `src/` — Rust core (`fonts.rs`, `render.rs`, `shaping.rs`, `batch.rs`, `streaming.rs`, `output.rs`). Keep modules tiny; everything funnels into deterministic JSONL output.
- `python/` — PyO3 bindings (`haforu/__init__.py`, stubs, tests). Only expose `StreamingSession`, `warm_up`, cache stats, and fast availability probes.
- `examples/python/` — smoke demos (batch, streaming, numpy). Update only when API truly changes.
- `scripts/` — `batch_smoke.sh` plus `jobs_smoke.json` for CLI validation in ~2 s.
- `testdata/fonts/` — minimal fixtures for shaping/raster tests.

## Mission
Deliver a zero-drama renderer that feeds fontsimi's analyzer and deep matcher at full speed: stdin JSONL → stdout JSONL for CLI, plus warmed StreamingSession for Python. Anything not required for that goal gets cut.

## Current Priorities (from PLAN.md)
1. **JSON Contract & Error Surfacing**: Every stdin line produces serialized `JobResult` even on failure; normalize error messaging across CLI/PyO3/smoke scripts.
2. **Variation Coordinate Validation**: Clamp `wght` [100, 900] and `wdth` [50, 200], warn-and-drop unknown axes, wire into `FontLoader::load_font`.
3. **Metrics-Only Output Mode**: `--format metrics` flag emits density + beam measurements as JSON, runtime <0.2 ms/job.
4. **StreamingSession Reliability**: Cache knobs (`max_fonts`, `max_glyphs`), warm-up hooks, microsecond `is_available()` checks, <1 ms steady-state latency.
5. **Distribution & Tooling**: Keep `scripts/batch_smoke.sh` green in ≤2 s, maintain universal2/manylinux wheels via `maturin`, document `HAFORU_BIN` workflow.

## Phase 3 Workstreams
- **Rust CLI Efficiency**: Audit `src/main.rs` for batch/stream/render/diagnostics commands with HarfBuzz-compatible flags plus cache-tuning knobs; profile hot paths, add benchmarks/regression tests.
- **Python Fire CLI Parity**: Reconfirm Fire CLI exposes same subcommands/flags as Rust CLI; harden argument validation, add regression tests.
- **Repository Canonicalization**: Compare layout with canonical Rust workspace + PyO3 + hatch guidance; update `.cargo/config.toml`, `pyproject.toml`, docs.
- **Build Reliability**: Define reproducible build pipeline spanning `cargo`, `maturin`, Hatch; update GitHub Actions workflows with cache-friendly steps and smoke-test gates.
- **Automatic SemVer**: Adopt Hatch VCS + cargo-vcs tagging; wire GitHub Actions to watch `vX.Y.Z` tags, run canonical build, push to PyPI/crates.io.

## Working Methods
- **Keep it small**: Short functions, flat modules, explicit data paths. If a helper doesn't reduce latency, delete it.
- **Measure constantly**: Use `cargo run --release -- batch < jobs_smoke.json` and note jobs/sec + RSS. No enterprise benchmarking rigs.
- **Tight test loop**: Rely on lightweight Rust unit tests + bundled smoke scripts. Only add new tests when a bug slips through or a contract changes.
- **Shared vocabulary**: Mirror helper names (`warm_up`, `ping`, `is_available`) with fontsimi so integration code stays trivial.
- **Error handling = guidance**: Fail fast with concise JSON errors (id + message). Don't add retry loops or backoff systems.

## Testing Stack
- `cargo test --lib render` for raster correctness.
- `cargo test streaming::tests::smoke` for JSONL path.
- `examples/python/*.py` doubles as sanity checks for bindings.
- `scripts/batch_smoke.sh` (runs haforu CLI on bundled `jobs_smoke.json`, expects success in ~2 s). Share elapsed times in WORK.md when relevant.

## Integration Checklist with fontsimi
1. Maintain `haforu::is_available()` so fontsimi can choose bindings vs CLI instantly.
2. Guarantee StreamingSession warm-up/ping exists and is cheap; fontsimi calls it before deep optimization.
3. Document CLI env requirements (fonts, HB data, HAFORU_BIN) so fontsimi can surface actionable errors.
4. Keep JSON schema versioned; bump `version` field only when absolutely necessary and update fontsimi in lockstep.
5. Any change affecting analyzer batching or deep renders must be reflected in `PLAN.md`, `TODO.md`, and `WORK.md` here and in fontsimi.

## Daily Workflow
1. Read `PLAN.md` + `TODO.md`, jot intent in `WORK.md` (clear after completion).
2. Implement only the next bottleneck fix. Avoid parallel tasking.
3. Run `cargo test` for touched modules and smoke scripts (CLI + python) to confirm latency hasn't regressed.
4. Update docs (PLAN/TODO/CHANGELOG) immediately; keep this CLAUDE file current.

## Mindset
- No enterprise scaffolding: no analytics, metrics dashboards, or exhaustive validation passes.
- Prefer deleting flags and configs; hard-code sane defaults that favor speed.
- Push expensive work into Rust; keep Python bindings thin wrappers.
- Every change should remove latency, memory, or integration friction. If it doesn't, don't ship it.
