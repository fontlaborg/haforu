---
this_file: haforu/PLAN.md
---

# Haforu Renderer Accuracy Fix

## Critical Issue
Haforu is producing incorrect matches with infinite pixel deltas (Δpx=inf) and wrong weight values. While width values are closer to correct (100-125), weights are far too light (100-400 instead of 900).

## Root Causes to Fix
1. **Pixel delta returns infinity**: Division by zero or failed comparison in pixel delta calculation
2. **Inconsistent metrics**: Haforu metrics differ from other renderers, causing optimizer confusion
3. **Failed renders not handled**: Empty or failed renders might be causing inf values
4. **Weight axis interpretation**: Haforu may be interpreting weight values differently

## Milestones

### 1. Fix Infinite Pixel Delta Bug (CRITICAL)
- **Goal**: Never return inf for pixel comparisons
- **Actions**:
  - Add defensive checks in render comparison logic
  - Handle case where one or both renders fail/are empty
  - Replace inf with large finite value (999999.0) as fallback
  - Add logging to identify when/why inf occurs
  - Ensure both images have valid dimensions before comparison
  - Check for division by zero in similarity calculations
- **Success**: No inf values in any render comparison

### 2. Standardize Metric Calculation
- **Goal**: Haforu produces metrics matching CoreText/HarfBuzz within 1%
- **Actions**:
  - Audit density calculation: ensure (ink_pixels / total_pixels) formula
  - Verify rendered_width calculation uses consistent pixel bounds
  - Check aspect ratio calculation matches other renderers
  - Ensure grayscale thresholding is consistent (>0 vs >127)
  - Add metric comparison logging for debugging
- **Success**: Same font/text produces metrics within 1% across all renderers

### 3. Improve Error Handling
- **Goal**: Gracefully handle render failures without breaking matching
- **Actions**:
  - Return error status in JobResult when render fails
  - Provide meaningful error messages (font not found, invalid coordinates, etc.)
  - Add retry logic for transient failures
  - Validate font coordinates are within valid ranges before rendering
  - Test with edge cases (empty text, huge sizes, invalid fonts)
- **Success**: All errors handled gracefully with informative messages

### 4. Fix Variable Font Coordinate Handling
- **Goal**: Correctly interpret and apply variable font axes
- **Actions**:
  - Verify weight axis uses standard 100-900 scale
  - Ensure width axis uses standard 50-200 scale
  - Filter out non-standard axes (TRAK, custom axes)
  - Add coordinate normalization if needed
  - Log actual vs requested coordinates for debugging
- **Success**: Requested coordinates match rendered results

### 5. Add Render Validation
- **Goal**: Validate renders are correct before returning
- **Actions**:
  - Check rendered image is non-empty
  - Verify dimensions match requested size
  - Ensure actual_bbox is within image bounds
  - Validate base64 encoding is correct
  - Add checksum/hash for render reproducibility
- **Success**: All renders pass validation checks

### 6. Performance With Correctness
- **Goal**: Maintain <2ms render time while fixing accuracy
- **Actions**:
  - Keep defensive checks lightweight
  - Use fast-path for common cases
  - Avoid unnecessary allocations in error handling
  - Profile to ensure fixes don't regress performance
  - Cache validation results when possible
- **Success**: <2ms render time with correct results

## Testing Requirements
- Render Arial Black at various sizes and verify consistent metrics
- Compare Haforu metrics with CoreText/HarfBuzz for 10+ fonts
- Test variable font coordinates across full range
- Verify error handling with invalid inputs
- Ensure no memory leaks in error paths

## Success Metrics
- ✅ Zero infinite pixel deltas in all comparisons
- ✅ Metrics match other renderers within 1% tolerance
- ✅ Arial Black → Archivo matches at wdth=100±5, wght=900±50
- ✅ All renders complete in <2ms
- ✅ Error messages are actionable and specific
- ✅ No memory leaks or resource exhaustion

## Out of Scope
- New rendering features or modes
- Additional output formats beyond PGM/PNG
- Performance optimizations that compromise correctness
- Support for color fonts or emoji