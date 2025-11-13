---
this_file: external/haforu2/COMPILATION_FIXES.md
---

# Haforu2 Compilation Fixes Required

**Status:** Initial foundation code written, needs API compatibility fixes
**Timeline:** ~4-6 hours to fix all compilation errors

---

## Current Issues (25 compilation errors)

The code was written based on API assumptions that don't match the actual crate versions. The following modules have API mismatches:

### 1. src/fonts.rs (~6 errors)

**Issues:**
- `font_ref.axes()` returns `AxisCollection` which is not an iterator
- Missing imports for `Size` and `LocationRef` from skrifa
- `font.head()` method doesn't exist on `FontRef`
- `font.glyph_metrics()` requires `MetadataProvider` trait in scope

**Fixes Required:**
- Import `skrifa::instance::{Size, LocationRef}`
- Import `skrifa::MetadataProvider`
- Fix axes iteration: `font_ref.axes().iter()` or similar
- Fix head access: use proper skrifa API
- Add `MetadataProvider` trait imports

### 2. src/shaping.rs (~8 errors)

**Issues:**
- `harfbuzz` v0.6 has different API than v0.4
- Missing imports: `Face`, `Font`, `GlyphBuffer`, `UnicodeBuffer`
- `Blob::with_bytes()` doesn't exist, use `Blob::new_read_only()`
- `font_ref.table_data` is a method, not a field: use `table_data()`
- `harfbuzz::shape()` function signature different
- HarfBuzz types need different imports

**Fixes Required:**
- Update harfbuzz imports to match v0.6 API
- Change `Blob::with_bytes()` → `Blob::new_read_only()`
- Change `font_ref.table_data` → `font_ref.table_data()`
- Update `harfbuzz::shape()` call to match v0.6 API
- Review harfbuzz v0.6 documentation for correct types

### 3. src/render.rs (~8 errors)

**Issues:**
- `zeno::Path` type doesn't exist (it's `zeno::PathData`)
- `PathBuilder::finish()` returns `PathData`, not `Path`
- `Mask::fill()` signature different
- `Mask::get_alpha()` doesn't exist (need to use `as_slice()` or similar)
- `Command` enum has different variant signatures
- Missing imports for zeno types

**Fixes Required:**
- Change `zeno::Path` → `zeno::PathData` or whatever zeno v0.3 provides
- Update `PathBuilder` usage to match zeno v0.3 API
- Fix `Mask` rasterization: use correct zeno v0.3 API
- Fix `Command` enum usage (likely different tuple variants)
- Review zeno v0.3 documentation for correct API

### 4. Minor Issues in Other Files (~3 errors)

**Issues:**
- Various type mismatches in function signatures
- Missing trait bounds

**Fixes Required:**
- Review and fix type signatures
- Add missing trait imports

---

## Fix Strategy

### Phase 1: Review Actual Crate APIs (2 hours)

1. **skrifa v0.22 API:**
   ```bash
   cargo doc --package skrifa --open
   ```
   - Check how to iterate axes
   - Check glyph metrics API
   - Check head table access
   - Check LocationRef and Size usage

2. **harfbuzz v0.6 API:**
   ```bash
   cargo doc --package harfbuzz --open
   ```
   - Check Blob API
   - Check Face/Font creation
   - Check Buffer types
   - Check shape() function signature

3. **zeno v0.3 API:**
   ```bash
   cargo doc --package zeno --open
   ```
   - Check PathBuilder/PathData
   - Check Mask rasterization API
   - Check Command enum variants

### Phase 2: Fix Each Module (2-3 hours)

1. **Fix fonts.rs** (30 min)
   - Add missing imports
   - Fix axes iteration
   - Fix metrics access
   - Test with `cargo build --lib`

2. **Fix shaping.rs** (1 hour)
   - Update harfbuzz imports and usage
   - Fix Blob creation
   - Fix shape() call
   - Test with `cargo build --lib`

3. **Fix render.rs** (1 hour)
   - Update zeno types
   - Fix PathBuilder usage
   - Fix Mask rasterization
   - Test with `cargo build --lib`

4. **Fix remaining issues** (30 min)
   - Fix any remaining type mismatches
   - Test with `cargo build --lib`

### Phase 3: Run Tests (30 min)

1. **Unit tests:**
   ```bash
   cargo test
   ```

2. **Fix test failures:**
   - Most tests should pass once compilation succeeds
   - May need minor adjustments to test data

---

## Alternative Approach: Use Existing Haforu1 Code

If API fixes take too long, we could:

1. Copy working font loading code from `external/haforu/src/mmap_font.rs`
2. Copy working shaping code from `external/haforu/src/shaping.rs`
3. Copy working rendering code from `external/haforu/src/rasterize.rs`
4. Adapt to clean haforu2 architecture

**Estimated time:** 2-3 hours (faster than fixing APIs from scratch)

**Trade-off:** Less clean separation, but proven working code

---

## Recommended Next Steps

**Option A: Fix APIs (Cleaner, ~4-6 hours)**
1. Review each crate's documentation
2. Fix imports and function calls systematically
3. Run tests and verify

**Option B: Port Haforu1 Code (Faster, ~2-3 hours)**
1. Copy working implementations from haforu/
2. Adapt to haforu2 structure
3. Clean up and test

**Recommendation:** Start with Option A (fix APIs) because:
- Code structure is already better organized
- APIs are more recent/idiomatic
- Learning exercise for correct API usage
- If stuck after 3 hours, switch to Option B

---

## Current Status

- ✅ Project structure created
- ✅ All modules scaffolded with proper logic
- ✅ Error handling defined
- ✅ Documentation written
- ❌ Compilation fails (25 errors)
- ⏸️ Testing blocked on compilation

**Next:** Start Phase 1 API review (2 hours)

---

## Success Criteria

- [ ] `cargo build` succeeds with 0 errors
- [ ] `cargo test` passes all unit tests
- [ ] `cargo clippy` shows no warnings
- [ ] Code structure remains clean and modular
