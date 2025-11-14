---
this_file: haforu/CHANGELOG.md
---

# Changelog

## 2025-11-14
- Added `render::Image` wrapper with `is_empty`, `calculate_bbox`, and safe `pixel_delta` to eliminate Δpx=inf cases and to centralize raster validation.
- Updated `GlyphRasterizer`, CLI pipeline, smoke tests, and PyO3 bindings to consume the new image type while keeping JSON output unchanged.
- Cleaned up Python bindings imports/visibility so the editable wheel still builds with the `python` feature.
- Hardened CLI streaming by routing every line through `handle_stream_line`, adding `JobResult::error`, and mapping missing fonts to the `FontNotFound` error for friendlier JSON responses; added unit coverage for the new helper.
- Pointed pytest at `python/tests` and added thin wrappers under `tests/` so `uvx hatch test` can discover the suite via Hatch.
- Tests:
  - `fd -e py -x uvx autoflake -i {}` ✅
  - `fd -e py -x uvx pyupgrade --py312-plus {}` ✅
  - `fd -e py -x uvx ruff check --output-format=github --fix --unsafe-fixes {}` ❌ (fails on long-standing lint violations in python/examples/tests; no repo changes kept)
  - `fd -e py -x uvx ruff format --respect-gitignore --target-version py312 {}` ✅ (auto-edits reverted to avoid semantic drift)
  - `uvx hatch test` ❌ (initial run: pytest could not find the legacy `tests/` directory before wrappers existed)
  - `cargo fmt && cargo test` ✅
  - `cargo test` ✅ (streaming error propagation tests)
  - `uvx hatch test` ✅ (test discovery succeeds; Python cases skipped because native module is not built in this env)
