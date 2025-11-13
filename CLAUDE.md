---
this_file: haforu2/CLAUDE.md
---

Claude Guide — haforu (canonical)

Purpose: make the new, canonical `haforu` renderer (developed here in `haforu2/`, published as `haforu`) the high‑performance batch/stream rendering backend for `fontsimi`. This file aligns with `haforu2/PLAN.md`, `haforu2/TODO.md`, and root `PLAN.md`/`TODO.md`.

Canonical Naming Policy
- Crate: `haforu`; Library: `haforu`; CLI binary: `haforu`; Python package/module: `haforu` / `haforu._haforu`.
- Do not introduce `haforu2` identifiers in code, manifests, imports, or docs. `haforu2/` is a workspace path only.

What `haforu` Must Provide (contracts required by fontsimi)
- Batch mode: stdin single JSON JobSpec `{"version":"1.0","jobs":[...]}` → stdout JSONL JobResult per line (flush per job).
- Streaming mode: `haforu stream` keeps a long‑lived process; JSON in/out one line at a time; persistent instance cache.
- Rendering payload: base64 PGM P5 (8‑bit grayscale) with `width`, `height`; optional `actual_bbox` `[x0,y0,x1,y1]`.
- Deterministic output; images must yield identical Daidot metrics to current renderers.

Immediate Objectives (H2 focus)
- Unblock H2 API compilation/runtime issues (skrifa/harfrust/zeno surface) — estimated 4–6 hours — this unlocks the entire integration timeline.
- Implement JSON parsing, font loading with variations, shaping, rasterization, and JSONL emission per `haforu2/PLAN.md`.
- Provide minimal Python bindings via pyo3/maturin only if needed; fontsimi primarily uses the CLI.

Guiding Principles (from AGENTS.md, applied here)
- Keep the public surface tiny, stable, and tested. Every function gets a test. Edge and error cases are first‑class.
- Prefer zero‑copy and simple data paths. Avoid abstractions “for flexibility”.
- Delete non‑essential code. No analytics/monitoring/enterprise scaffolding.
- Measure performance; don’t guess. Validate memory usage and determinism.

Data Structures (reference)
- `JobSpec { version, mode?, config?, jobs[] }`
- `Job { id, font{path,size,variations?}, text{content,script?}, rendering{format:"pgm",encoding:"base64",width,height} }`
- `JobResult { id, status:"success"|"error", rendering{format,encoding,data,width,height,actual_bbox?}, error? }`

Testing Checklist
- Unit tests for JSON parsing, font loading, shaping, rasterization, and PGM encoding.
- Integration tests: JSON → render → JSONL; decode base64 PGM, verify dims/pixels; variable font coordinates; error cases.
- Performance smoke: 1000 renders <10s; peak RSS <500MB; no leaks over millions of renders.
- FontSimi compatibility: run sample jobs from fontsimi’s HaforuRenderer; validate schema and image fidelity.

Dev Commands
- Build/tests: `cargo build --release`, `cargo test`, `cargo clippy`, `cargo fmt`.
- CLI smoke: `echo '{"version":"1.0","jobs":[...]}' | cargo run -- batch > out.jsonl`.
- Streaming smoke: `echo '{"id":"t1",...}' | cargo run -- stream`.

Migration Notes
- Rename manifests to `haforu` (Cargo.toml lib/bin, pyproject `project.name` and `tool.maturin.module-name`).
- Update all `use`/`mod` paths and docs; remove any `haforu2` identifiers.
- Publish artifacts as `haforu`; coordinate with fontsimi integration tests before release.

Hard Rules
- No serif/sans classification or style heuristics — matching stays metric‑based downstream.
- Keep `this_file` markers at the top of all source/docs.
- Small, flat modules; functions <20 lines when practical; test everything.

If in Doubt
- Re‑read `haforu2/PLAN.md` and `haforu2/TODO.md`. Unblock H2 first; keep the CLI contracts stable for fontsimi.

