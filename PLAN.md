---
this_file: haforu/PLAN.md
---

# Haforu Fast-Path Plan

## Scope
Keep Haforu ready for fontsimi by delivering a zero-drama renderer stack that blasts through batches and exposes just enough knobs for integration.

## Constraints
- Prioritize render latency and throughput; no new validation layers, telemetry, or enterprise packaging workflows.
- CLI must behave like a pure stream processor (stdin JSONL → stdout JSONL) so fontsimi can keep the pipe full.
- Python bindings stay small: expose only what fontsimi needs (StreamingSession, warm-up, cache stats).

## Milestones
1. **StreamingSession reliability.**
   - Expose cache sizing + eviction stats, add an explicit warm-up helper, and make `close()` drop every file handle instantly.
   - Provide a cheap `ping()` or `render_blank()` so fontsimi can pre-flight sessions before deep optimization.
   - Surface an `is_available()` probe that fontsimi can call without importing heavy modules.
   - _Status:_ Completed via cache stats/setters, `warm_up`, close guard, and module/class availability probes.
2. **Batch CLI ergonomics.**
   - Accept stdin JSONL, flush stdout per job, and exit at the first hard failure; Haforu should never hang waiting for Python.
   - Add a runtime `--jobs N` knob so parallelism can be tuned without recompiling.
   - Ship a tiny `jobs_smoke.json` + `scripts/batch_smoke.sh` that runs in ~2 s to prove the binary is healthy.
   - _Status:_ Completed with JSONL parser, `--jobs` alias, and the new smoke script + fixture.
3. **Distribution + handshake.**
   - Produce universal2 macOS and manylinux wheels via `maturin build --features python` and publish them so fontsimi installs without a compiler.
   - Document the two blessed install commands (`uv pip install haforu`, `cargo install haforu`) and the env var (`HAFORU_BIN`) wiring fontsimi expects.
   - Mirror the warm-up/probe helper names in the docs so both repos stay in sync.
   - _Status:_ Commands + env var documented; warm-up/probe names mirrored with fontsimi integration hooks.

## Success Metrics
- StreamingSession steady-state renders in ≤2 ms with cache warm after the ping helper runs.
- CLI pushes >100 jobs/sec with stdin/stdout streaming and honors the `--jobs` knob without restart.
- Installing via pip + cargo works on macOS arm64/x86_64 and Linux x86_64 without manual patches.

## Out of Scope
- Elaborate benchmarking suites, additional language bindings, or configuration systems beyond what fontsimi strictly needs.
