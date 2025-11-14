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
