---
this_file: PLAN.md
---

# Haforu Development Plan

## Objective

Deliver a fast, reliable font renderer for CLI and Python with deterministic JSONL output, validated variation coordinates, and sub-millisecond streaming performance.

## Active Work

### 1. Error Handling Consistency

**Goal:** Every job produces a serialized `JobResult`, even if parsing or validation fails.

- Ensure CLI batch mode never drops jobs silently
- Ensure CLI stream mode returns error JobResults for invalid JSONL
- Ensure Python bindings return error JobResults (not exceptions) for invalid jobs
- Add regression tests for malformed inputs across all interfaces

### 2. Variation Coordinate Validation

**Goal:** Clamp variation coordinates to valid ranges and warn on unknown axes.

- Add `validate_coordinates()` in `src/fonts.rs`:
  - Clamp `wght` to [100, 900]
  - Clamp `wdth` to [50, 200]
  - Warn and drop unknown axes
- Wire validation into `FontLoader::load_font`
- Surface sanitized coordinates in JobResult JSON for debugging
- Add unit tests for in-range, out-of-range, and unknown-axis cases

### 3. Metrics-Only Output Reliability

**Goal:** `--format metrics` mode is stable, fast, and well-documented.

- Verify metrics calculation is deterministic
- Ensure runtime stays <0.2ms per job
- Document metrics mode in README with examples
- Add example in `examples/python/metrics_demo.py`
- Benchmark metrics mode vs image mode and document speedup

### 4. Python StreamingSession Reliability

**Goal:** Python bindings provide <1ms steady-state rendering with cache control.

- Verify `warm_up()`, `ping()`, and `is_available()` work correctly
- Test cache knobs (`max_fonts`, `max_glyphs`) are respected
- Stress-test with >1000 sequential renders to verify no RSS creep
- Ensure JSON schema matches CLI output exactly
- Document cache tuning in README

### 5. Cross-Platform Build Verification

**Goal:** Ensure builds work on macOS, Linux, and Windows without manual intervention.

- Verify `cargo build --release` works on all platforms
- Verify `maturin build --release` produces working wheels
- Test Python bindings install correctly from wheels
- Document platform-specific prerequisites in README
- Keep `scripts/build.sh` and `scripts/batch_smoke.sh` working

## Testing Strategy

- **Unit Tests** - Rust modules (`cargo test`) and Python bindings (`pytest`)
- **Integration Tests** - `scripts/batch_smoke.sh` validates CLI contract in <2s
- **Performance Tests** - `scripts/profile-cli.sh` catches regressions
- **Edge Cases** - Empty text, zero-sized canvas, missing fonts, invalid variations

## Performance Targets

- CLI batch (1000 jobs): <10s on 8 cores
- CLI streaming: <10ms per job including startup
- Python StreamingSession: <2ms per job (warmed cache)
- Metrics mode: <0.2ms per job

## Success Criteria

1. ✅ CLI never drops jobs - every input line produces a JSONL output
2. ✅ Variation coordinates stay within spec with clear warnings
3. ✅ Metrics mode achieves 5-10× speedup over image modes
4. ✅ Python StreamingSession provides <2ms renders with cache control
5. ✅ All platforms build and run without manual fixes

## Out of Scope

- New output formats beyond PGM/PNG/metrics
- Retry logic, circuit breakers, or resilience patterns
- Analytics, monitoring, or telemetry systems
- Color, emoji, or subpixel rendering
- Complex configuration systems
- Automatic version tagging or release automation
- Repository structure reorganization
