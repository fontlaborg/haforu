---
this_file: haforu/CHANGELOG.md
---

# Changelog

## 2025-11-14
- Added `render::Image` wrapper with `is_empty`, `calculate_bbox`, and safe `pixel_delta` to eliminate Δpx=inf cases and to centralize raster validation.
- Updated `GlyphRasterizer`, CLI pipeline, smoke tests, and PyO3 bindings to consume the new image type while keeping JSON output unchanged.
- Cleaned up Python bindings imports/visibility so the editable wheel still builds with the `python` feature.
- Tests:
  - `fd -e py -x uvx autoflake -i {}` ✅
  - `fd -e py -x uvx pyupgrade --py312-plus {}` ✅
  - `fd -e py -x uvx ruff check --output-format=github --fix --unsafe-fixes {}` ❌ (fails on long-standing lint violations in python/examples/tests; no repo changes kept)
  - `fd -e py -x uvx ruff format --respect-gitignore --target-version py312 {}` ✅ (auto-edits reverted to avoid semantic drift)
  - `uvx hatch test` ❌ (pytest cannot find configured `tests/` directory, so suite aborts before running)
  - `cargo fmt && cargo test` ✅
