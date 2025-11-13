---
this_file: haforu/TODO.md
---

## Milestone 1: Fix Infinite Pixel Delta Bug

- [ ] Add defensive check in `src/render.rs` before pixel comparison
- [ ] Check if rendered images are empty before calculating similarity
- [ ] Add validation that image dimensions are non-zero
- [ ] Implement safe division that returns 999999.0 on zero denominator
- [ ] Add logging to identify when inf values occur
- [ ] Create `SafePixelDelta` struct that guarantees finite values
- [ ] Add bounds checking to clamp pixel delta to [0, 999999]
- [ ] Test with edge cases: empty text, invalid font, zero size
- [ ] Add unit test for pixel comparison with various inputs
- [ ] Verify Python bindings handle large finite values correctly
- [ ] Add error field to JobResult for partial failures
- [ ] Create regression test that previously returned inf
- [ ] Document the pixel delta calculation algorithm
- [ ] Add debug mode that saves failed renders for inspection

## Milestone 2: Standardize Metric Calculation

- [ ] Audit density calculation in `src/render.rs`
- [ ] Ensure density = (pixels > threshold) / total_pixels
- [ ] Standardize grayscale threshold to >0 for ink detection
- [ ] Verify rendered_width uses ink bounds, not glyph advance
- [ ] Check aspect ratio calculation matches other renderers
- [ ] Add metric calculation unit tests
- [ ] Create reference renders with known metric values
- [ ] Compare Haforu metrics with CoreText for 10 fonts
- [ ] Add metric logging in verbose mode
- [ ] Ensure consistent coordinate rounding
- [ ] Document exact metric formulas in code comments
- [ ] Add integration test comparing with reference implementation
- [ ] Create debug output showing intermediate calculation steps

## Milestone 3: Improve Error Handling

- [ ] Add comprehensive error types in `src/error.rs`
- [ ] Return specific error for font file not found
- [ ] Return error for invalid variable coordinates
- [ ] Return error for unsupported font format
- [ ] Add retry logic for transient failures
- [ ] Validate coordinates are within font's design space
- [ ] Add timeout for hung render operations
- [ ] Implement graceful degradation for partial failures
- [ ] Add error context with font path and requested coordinates
- [ ] Create error recovery strategies
- [ ] Test error handling with corrupted fonts
- [ ] Test with missing font files
- [ ] Test with invalid UTF-8 in text
- [ ] Add error statistics tracking
- [ ] Document all error codes and their meanings

## Milestone 4: Fix Variable Font Coordinate Handling

- [ ] Verify weight axis interpretation (100-900 scale)
- [ ] Verify width axis interpretation (50-200 scale)
- [ ] Add axis normalization if font uses non-standard ranges
- [ ] Filter out non-standard axes (TRAK, custom)
- [ ] Create axis whitelist in `src/fonts.rs`
- [ ] Log warnings for ignored axes
- [ ] Add coordinate clamping to valid ranges
- [ ] Test with fonts having non-standard axis ranges
- [ ] Verify skrifa correctly applies variations
- [ ] Add debug output showing actual vs requested coordinates
- [ ] Create test with known variable fonts
- [ ] Document standard axis ranges
- [ ] Add axis validation before rendering
- [ ] Test with static fonts (no variations)

## Milestone 5: Add Render Validation

- [ ] Check rendered image is non-empty
- [ ] Verify image dimensions match requested size
- [ ] Ensure actual_bbox is within image bounds
- [ ] Validate actual_bbox has positive width/height
- [ ] Add checksum for render reproducibility
- [ ] Verify base64 encoding is correct
- [ ] Add size sanity checks (not too large/small)
- [ ] Test that same input produces same output
- [ ] Add validation for PGM header format
- [ ] Check for memory corruption in image buffer
- [ ] Verify pixel values are in valid range [0, 255]
- [ ] Add render validation in debug builds
- [ ] Create validation test suite
- [ ] Document validation requirements

## Milestone 6: Performance With Correctness

- [ ] Profile defensive checks to minimize overhead
- [ ] Keep validation on fast path lightweight
- [ ] Use branch prediction hints for common cases
- [ ] Avoid allocations in error handling
- [ ] Cache validation results when possible
- [ ] Use SIMD for pixel operations where applicable
- [ ] Benchmark before/after adding safety checks
- [ ] Ensure <2ms render time is maintained
- [ ] Profile memory allocations
- [ ] Optimize hot paths identified by profiler
- [ ] Add performance regression tests
- [ ] Document performance critical sections
- [ ] Create benchmarks for each operation
- [ ] Monitor cache hit rates

## Python Bindings Updates

- [ ] Update `StreamingSession` to handle errors gracefully
- [ ] Add validation in `render_to_numpy` method
- [ ] Ensure errors propagate to Python correctly
- [ ] Add Python-side inf/nan checking
- [ ] Update error messages to be Python-friendly
- [ ] Add type hints for all methods
- [ ] Create Python tests for error cases
- [ ] Document error handling in Python API
- [ ] Add examples showing error handling
- [ ] Test memory safety with invalid inputs
- [ ] Verify no memory leaks in error paths
- [ ] Add Python-side logging integration

## CLI Updates

- [ ] Add `--validate` flag for render validation
- [ ] Add `--max-retries` option for transient failures
- [ ] Improve error messages in JSONL output
- [ ] Add `--debug` flag for detailed logging
- [ ] Support `--axis-whitelist` parameter
- [ ] Add `--safe-mode` for maximum validation
- [ ] Create `--test` command for self-diagnosis
- [ ] Add version info to help output
- [ ] Support environment variables for defaults
- [ ] Add progress indicator for batch mode
- [ ] Document all CLI options

## Testing

- [ ] Create `tests/test_metrics.rs` for metric validation
- [ ] Create `tests/test_errors.rs` for error handling
- [ ] Create `tests/test_variations.rs` for variable fonts
- [ ] Add integration tests with real fonts
- [ ] Test with fuzzing for edge cases
- [ ] Add regression tests for fixed bugs
- [ ] Create performance benchmarks
- [ ] Test memory safety with valgrind/ASAN
- [ ] Add CI tests for multiple platforms
- [ ] Create test fixtures with known outputs
- [ ] Document how to run tests
- [ ] Add test coverage reporting

## Documentation

- [ ] Update README with accuracy improvements
- [ ] Document pixel delta calculation
- [ ] Explain metric standardization
- [ ] Add troubleshooting guide
- [ ] Create FAQ for common issues
- [ ] Document error codes
- [ ] Add architecture diagram
- [ ] Explain coordinate handling
- [ ] Document performance characteristics
- [ ] Add examples for each use case
- [ ] Create migration guide for API changes
- [ ] Add changelog entries