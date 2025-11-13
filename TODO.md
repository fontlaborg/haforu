---
this_file: haforu/TODO.md
---

## Phase 1: Fix Δpx=inf Bug (Day 1)

### Render Validation
- [ ] Add empty image check in `src/render.rs` before pixel comparison
- [ ] Implement `is_empty()` method for Image struct
- [ ] Check dimensions are non-zero before calculating delta
- [ ] Return 999999.0 for empty/invalid renders
- [ ] Add nan/inf check after delta calculation
- [ ] Clamp final delta to [0.0, 999999.0]
- [ ] Test with edge cases (empty text, zero size)

### Error Propagation
- [ ] Add `status` field to JobResult struct
- [ ] Return `status: "error"` for failed renders in JSONL
- [ ] Include error message in result
- [ ] Test error handling with invalid fonts
- [ ] Verify Python bindings handle error status

## Phase 2: Coordinate Validation (Day 2)

### Axis Standardization
- [ ] Create `validate_coordinates()` function in `src/fonts.rs`
- [ ] Clamp wght to [100.0, 900.0] range
- [ ] Clamp wdth to [50.0, 200.0] range
- [ ] Define STANDARD_AXES constant array
- [ ] Log warnings for non-standard axes
- [ ] Filter out TRAK and custom axes
- [ ] Test with fonts using non-standard ranges

### Coordinate Logging
- [ ] Add debug logging for requested coordinates
- [ ] Log actual coordinates after validation
- [ ] Log axis values being applied to font
- [ ] Create coordinate comparison test
- [ ] Verify skrifa applies variations correctly

## Phase 3: Metrics Mode (Days 3-4)

### Metrics Output Format
- [ ] Create MetricsResult struct in `src/output.rs`
- [ ] Add width_px, height_px, ref_height_px fields
- [ ] Add density calculation (ink_pixels / total_pixels)
- [ ] Add h_beam, v_beam, d_beam calculations
- [ ] Implement `compute_metrics()` function
- [ ] Add `--format metrics` CLI option
- [ ] Return JSON instead of base64 image

### Beam Cast Implementation
- [ ] Implement horizontal_beam_measure()
- [ ] Implement vertical_beam_measure()
- [ ] Implement diagonal_beam_measure()
- [ ] Test beam calculations match Python version
- [ ] Optimize for performance (target <0.2ms)

### Performance Testing
- [ ] Benchmark metrics mode vs image mode
- [ ] Verify 10x speedup for metrics-only
- [ ] Test with 10,000 metric calculations
- [ ] Memory profiling for metrics mode

## Phase 4: Streaming Session (Week 2)

### Font Cache
- [ ] Implement LRU cache for loaded fonts
- [ ] Set cache size limit (50 fonts)
- [ ] Add cache hit/miss metrics
- [ ] Implement font eviction
- [ ] Test cache performance

### Session Protocol
- [ ] Design stdin/stdout protocol for streaming
- [ ] Implement persistent process mode
- [ ] Add session management to Python bindings
- [ ] Create session warm-up/ping commands
- [ ] Handle session lifecycle (create/destroy)

### Performance Optimization
- [ ] Benchmark streaming vs CLI mode
- [ ] Target <1ms render latency
- [ ] Profile memory usage during long sessions
- [ ] Test with 1000 sequential renders
- [ ] Optimize font loading/caching

## Phase 5: Batch Enhancements

### Streaming Batch Processing
- [ ] Remove 5000 job limit
- [ ] Process jobs in chunks of 1000
- [ ] Stream JSONL output as available
- [ ] Implement memory-bounded processing
- [ ] Add progress reporting

### Memory Management
- [ ] Monitor memory during batch processing
- [ ] Implement automatic garbage collection
- [ ] Test with 100,000 job batch
- [ ] Ensure stable memory usage <500MB

## Testing & Validation

### Regression Tests
- [ ] Arial Black at wght=900 renders correctly
- [ ] Archivo coordinates produce expected visual
- [ ] All error cases return proper status
- [ ] Metrics match reference values ±1%
- [ ] No memory leaks in error paths

### Edge Case Tests
- [ ] Empty text → error status
- [ ] Invalid font path → error status
- [ ] Size = 0 → error status
- [ ] Size = 10000 → reasonable fallback
- [ ] Missing axes → use defaults
- [ ] Corrupted font → graceful failure

### Performance Benchmarks
- [ ] Batch: 10,000 renders in <20s
- [ ] Streaming: 1000 iterations in <1s
- [ ] Metrics: 10x faster than images
- [ ] Memory: <500MB for any workload
- [ ] Startup: <50ms for first render

## Documentation

- [ ] Document error status format
- [ ] Document metrics output format
- [ ] Add streaming protocol spec
- [ ] Update CLI help text
- [ ] Add Python bindings examples
- [ ] Create troubleshooting guide