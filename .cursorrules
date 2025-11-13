---
this_file: haforu/CLAUDE.md
---

# Haforu Builder Notes

## Repository Snapshot (from llms summaries)
- `src/` — Rust core (`fonts.rs`, `render.rs`, `shaping.rs`, `batch.rs`, `streaming.rs`, `output.rs`). Keep modules tiny; everything funnels into deterministic JSONL output.
- `python/` — PyO3 bindings (`haforu/__init__.py`, stubs, tests). Only expose `StreamingSession`, `warm_up`, cache stats, and fast availability probes.
- `examples/python/` — smoke demos (batch, streaming, numpy). Update them only when the API truly changes.
- `target/` & `wheels/` — build artifacts; clean as needed but avoid touching during normal dev.
- `scripts/` (add) — host `batch_smoke.sh` plus `jobs_smoke.json` so fontsimi devs can validate the CLI in ~2 s.
- `testdata/fonts/` — minimal fixtures for shaping/raster tests.

## Mission
Deliver a zero-drama renderer that feeds fontsimi’s analyzer and deep matcher at full speed: stdin JSONL → stdout JSONL for the CLI, plus a warmed StreamingSession for Python. Anything not required for that goal gets cut.

## Development Priorities
1. **StreamingSession reliability.** Cache knobs (`max_fonts`, `max_glyphs`), `warm_up()` helper, `close()` that frees descriptors immediately, `is_available()` probe that returns in microseconds.
2. **Batch CLI ergonomics.** Stream jobs from stdin (2–4k batches), flush stdout per job, exit on first fatal error, and expose `--jobs N` to tune parallel workers without recompiles.
3. **Distribution.** Produce universal2 macOS and manylinux wheels via `maturin`, plus a `cargo install haforu` path. Document the exact commands fontsimi should echo and how to set `HAFORU_BIN`.
4. **Integration contract.** Rendering payload stays base64 PGM (8-bit). Fields: `id`, `status`, `width`, `height`, `actual_bbox?`, `data`. Keep schema backward compatible; announce changes in both repos.

## Working Methods
- **Keep it small.** Short functions, flat modules, explicit data paths. If a helper doesn’t reduce latency, delete it.
- **Measure constantly.** Use `cargo run --release -- batch < jobs_smoke.json` and note jobs/sec + RSS. No enterprise benchmarking rigs.
- **Tight test loop.** Rely on lightweight Rust unit tests + the bundled smoke scripts. Only add new tests when a bug slips through or a contract changes.
- **Shared vocabulary.** Mirror helper names (`warm_up`, `ping`, `is_available`) with fontsimi so integration code stays trivial.
- **Error handling = guidance.** Fail fast with concise JSON errors (id + message). Don’t add retry loops or backoff systems.

## Minimal Testing Stack
- `cargo test --lib render` for raster correctness.
- `cargo test streaming::tests::smoke` for JSONL path.
- `examples/python/*.py` doubles as sanity checks for bindings.
- `scripts/batch_smoke.sh` (runs haforu CLI on bundled `jobs_smoke.json`, expects success in ~2 s). Share elapsed times in WORK.md when relevant.

## Integration Checklist with fontsimi
1. Maintain `haforu::is_available()` so fontsimi can choose bindings vs CLI instantly.
2. Guarantee StreamingSession warm-up/ping exists and is cheap; fontsimi calls it before deep optimization.
3. Document CLI env requirements (fonts, HB data, HAFORU_BIN) so fontsimi can surface actionable errors.
4. Keep JSON schema versioned; bump a `version` field only when absolutely necessary and update fontsimi in lockstep.
5. Any change that affects analyzer batching or deep renders must be reflected in `PLAN.md`, `TODO.md`, and `WORK.md` here and in fontsimi.

## Daily Workflow Template
1. Read `haforu/PLAN.md` + `haforu/TODO.md`, jot intent in `haforu/WORK.md` (clear after completion).
2. Implement only the next bottleneck fix (StreamingSession knobs, batch CLI streaming, packaging). Avoid parallel tasking.
3. Run `cargo test` for touched modules and the smoke scripts (CLI + python) to confirm latency hasn’t regressed.
4. Update docs (PLAN/TODO/CHANGELOG) immediately; keep this CLAUDE file current so future agents share the same map.

## Mindset
- No enterprise scaffolding: no analytics, metrics dashboards, or exhaustive validation passes.
- Prefer deleting flags and configs; hard-code sane defaults that favor speed.
- Push expensive work into Rust; keep Python bindings thin wrappers.
- Every change should remove latency, memory, or integration friction. If it doesn’t, don’t ship it.
