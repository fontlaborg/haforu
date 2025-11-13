---
this_file: haforu/PLAN.md
---

# Haforu Renderer Support for Multi-Stage Pipeline

## Critical Issues to Fix

### Issue 1: Infinite Pixel Delta Bug (CRITICAL)
- **Problem**: Returning Δpx=inf when renders fail or images empty
- **Root Cause**: Division by zero, no validation before comparison
- **Solution**: Add defensive checks, return 999999.0 instead of inf
- **Files**: `src/render.rs`, `src/metrics.rs`

### Issue 2: Variable Font Coordinate Accuracy
- **Problem**: Possible misinterpretation of axis values (wght, wdth)
- **Root Cause**: Non-standard axis scaling or incorrect application
- **Solution**: Verify standard scales (wght=100-900, wdth=50-200)
- **Files**: `src/fonts.rs`, `src/variations.rs`

### Issue 3: Error Status Propagation
- **Problem**: Failed renders return empty data instead of error status
- **Root Cause**: Missing error handling in render pipeline
- **Solution**: Return explicit `status: "error"` in JSONL
- **Files**: `src/output.rs`, `src/error.rs`

## New Features for Pipeline

### Metrics-Only Output Mode
- **Purpose**: Compute Daidot metrics without full image for tentpoles
- **Implementation**: Add `--format metrics` to return JSON metrics only
- **Benefits**: 10x faster tentpole analysis (no base64 encoding)
- **Fields**: width_px, height_px, density, h_beam, v_beam, d_beam

### Streaming Session Optimization
- **Purpose**: Reuse font loading across optimization iterations
- **Current**: Each render spawns new process
- **Improved**: Persistent session with cached fonts
- **Target**: <1ms render latency in tight loops

### Batch Mode Enhancements
- **Current Limit**: 5000 jobs per batch
- **New**: Stream processing for unlimited jobs
- **Memory**: Process and flush every 1000 jobs
- **Output**: Stream JSONL results as available

## Implementation Plan

### Phase 1: Fix Δpx=inf (Day 1)
```rust
// In src/render.rs
fn calculate_pixel_delta(img1: &Image, img2: &Image) -> f64 {
    if img1.is_empty() || img2.is_empty() {
        return 999999.0;
    }

    let delta = compute_delta(img1, img2);
    if delta.is_nan() || delta.is_infinite() {
        999999.0
    } else {
        delta.clamp(0.0, 999999.0)
    }
}
```

### Phase 2: Coordinate Validation (Day 2)
```rust
// In src/fonts.rs
fn validate_coordinates(coords: &HashMap<String, f32>) -> HashMap<String, f32> {
    let mut valid = HashMap::new();

    // Ensure standard scales
    if let Some(wght) = coords.get("wght") {
        valid.insert("wght", wght.clamp(100.0, 900.0));
    }
    if let Some(wdth) = coords.get("wdth") {
        valid.insert("wdth", wdth.clamp(50.0, 200.0));
    }

    // Log warnings for non-standard axes
    for (axis, value) in coords {
        if !STANDARD_AXES.contains(axis) {
            warn!("Ignoring non-standard axis: {}", axis);
        }
    }
    valid
}
```

### Phase 3: Metrics Mode (Day 3-4)
```rust
// In src/output.rs
#[derive(Serialize)]
struct MetricsResult {
    width_px: u32,
    height_px: u32,
    ref_height_px: u32,
    density: f32,
    h_beam: f32,
    v_beam: f32,
    d_beam: f32,
}

fn output_metrics(image: &Image, params: &RenderParams) -> String {
    let metrics = compute_metrics(image);
    serde_json::to_string(&metrics).unwrap()
}
```

### Phase 4: Streaming Session (Week 2)
- Implement font cache with LRU eviction
- Add session management to Python bindings
- Persistent process with stdin/stdout protocol
- Benchmark to ensure <1ms render latency

## Testing Requirements

### Regression Tests
- Arial Black at wght=900 produces correct weight
- Archivo coordinates map correctly to visual weight/width
- Failed renders return error status, not empty data
- Metrics match reference values within 1%

### Performance Tests
- Batch mode: 10,000 renders in <20s
- Streaming mode: 1000 iterations in <1s
- Metrics mode: 10x faster than image mode
- Memory: Stable at <500MB for any batch size

### Edge Case Tests
- Empty text → error status
- Invalid font → error status
- Huge size (10000px) → error or reasonable fallback
- Zero size → error status
- Missing axes → use defaults

## Success Metrics

### Immediate (Phase 1)
- ✅ Zero Δpx=inf in all test cases
- ✅ Error messages instead of silent failures
- ✅ Arial Black test passes

### Short-term (Phase 2-3)
- ✅ Metrics mode 10x faster
- ✅ Coordinates validated and logged
- ✅ Standard axis scales enforced

### Long-term (Phase 4)
- ✅ <1ms streaming render
- ✅ Unlimited batch size via streaming
- ✅ Memory stable under load

## Out of Scope
- New rendering features
- Additional output formats beyond PGM/PNG/metrics
- Color font support
- Emoji rendering
- Subpixel antialiasing