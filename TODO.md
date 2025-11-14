---
this_file: TODO.md
---

# Haforu Task List

## Error Handling Consistency
- [ ] Test CLI batch mode with malformed JSON - ensure error JobResults, not crashes
- [ ] Test CLI stream mode with invalid JSONL lines - ensure error JobResults per line
- [ ] Test Python bindings with invalid job specs - ensure error JobResults returned
- [ ] Add regression tests for malformed inputs in `python/tests/test_errors.py`

## Variation Coordinate Validation
- [ ] Implement `validate_coordinates()` in `src/fonts.rs`
- [ ] Add clamping for `wght` [100, 900] with warnings
- [ ] Add clamping for `wdth` [50, 200] with warnings
- [ ] Warn and drop unknown axes (log to stderr)
- [ ] Wire validation into `FontLoader::load_font`
- [ ] Surface sanitized coordinates in `JobResult.font.variations`
- [ ] Add unit tests for in-range coordinates
- [ ] Add unit tests for out-of-range coordinates
- [ ] Add unit tests for unknown axes
- [ ] Add integration test confirming skrifa receives sanitized values

## Metrics Mode Reliability
- [ ] Verify metrics calculation is deterministic (same input = same output)
- [ ] Benchmark metrics mode runtime - ensure <0.2ms per job
- [ ] Verify `examples/python/metrics_demo.py` works correctly
- [ ] Update README with metrics mode documentation
- [ ] Add metrics mode examples to README quick reference
- [ ] Document speedup numbers in CHANGELOG

## Python StreamingSession Reliability
- [ ] Test `StreamingSession.warm_up()` with various fonts
- [ ] Test `StreamingSession.ping()` returns True on live session
- [ ] Test `haforu.is_available()` returns True after install
- [ ] Verify `max_fonts` parameter is respected
- [ ] Verify `max_glyphs` parameter is respected
- [ ] Stress test with 1000+ sequential renders
- [ ] Monitor RSS during stress test - ensure no memory leaks
- [ ] Verify JSON output matches CLI format exactly
- [ ] Document cache tuning in README Python section

## Cross-Platform Build Verification
- [ ] Test `cargo build --release` on macOS
- [ ] Test `cargo build --release` on Linux
- [ ] Test `cargo build --release` on Windows
- [ ] Test `maturin build --release` produces universal2 wheel on macOS
- [ ] Test `maturin build --release` produces manylinux wheel on Linux
- [ ] Test `maturin build --release` produces Windows wheel
- [ ] Verify Python wheels install without compiler
- [ ] Document prerequisites in README (Rust version, Python version)
- [ ] Ensure `scripts/build.sh` works on macOS and Linux
- [ ] Ensure `scripts/batch_smoke.sh` passes on all platforms
