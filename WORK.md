---
this_file: haforu/WORK.md
---

# Haforu Work Notes

This file serves as a scratchpad for work-in-progress notes. It should be cleaned after completing tasks.

## Current Status

All Phase 1 and Phase 2 integration tasks are complete (100%).

Latest commit: `07861ba Phase 2: Complete production release automation`

Ready for next release tag (suggested: v2.1.0).

## 2025-11-18

- Replaced `scripts/build.sh` with a UV-driven pipeline that emits timestamped artifacts (`target/artifacts/<stamp>/`), wires in `cargo test`, `uvx hatch test`, and `scripts/batch_smoke.sh`, and hardens env detection (CPU jobs + registry overrides).
- Added a fixture-driven `scripts/run.sh` that replays `scripts/jobs_smoke.jsonl` through batch/metrics/stream (with optional Python StreamingSession demo) and logs summaries under `target/run-log/`.
- Documented both scripts in README/INSTALL and cleaned `.cargo/config.toml` so builds no longer rely on an undefined vendor source.

### Tests & Commands

- `bash scripts/build.sh` → **fails** because the current `src/main.rs` references removed symbols (`haforu::batch::FontSpec/TextSpec/RenderSpec`) and assumes optional `rendering.data` (errors E0422/E0308). The script now surfaces the failure cleanly; fixing those compile errors is outside this task.
