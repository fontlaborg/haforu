this_file: haforu/WORK.md
---

# 2025-11-14 /work Iteration

## Scope
- Address Phase 1 items from `TODO.md`: add render validation guardrails so Δpx never returns `inf`.
- Introduce unit tests first for `Image` wrapper + pixel delta clamp behavior.
- Integrate checks into Rust + Python paths, then run the `/test` command stack.

## Immediate Tasks
- [x] Research/confirm existing crates for pixel comparison to avoid reinventing wheels.
- [x] Add regression tests covering empty images, mismatched dimensions, and NaN delta edge cases.
- [x] Implement minimal `Image` helper (width/height/pixels) with `is_empty`, `calculate_bbox`, `pixel_delta` (clamped to `[0, 999999]` and fall back for invalid inputs).
- [x] Wire new helper into rasterizer + bindings without impacting perf (watch for allocations).
- [x] Execute `/test` workflow (formatting tools + `uvx hatch test` + `cargo test` as needed) and capture results.

## Risks / Notes
- Rendering is hot-path; additional validation must stay O(n) with no extra allocations beyond what already exists.
- Need to ensure Python bindings still hand back contiguous arrays after signature changes.
- Toolchain commands in `/test` touch entire tree; be prepared for long runtimes and potential lint churn.

## Execution Notes
- Looked at crates.io candidates (`image-diff`, `ks-image-compare`) for existing delta helpers; both depend on `image::DynamicImage` and would force extra conversions, so we kept a narrow in-house routine tailored to the grayscale buffers.
- Added `Image` wrapper + companion tests first, then threaded it through `GlyphRasterizer`, CLI, and PyO3 bindings.
- `fd -e py -x uvx ruff check --fix --unsafe-fixes {}` mutated several Python examples/tests; reverted those files to keep semantic noise out while still recording the failure reason below.

## Test Log
- `fd -e py -x uvx autoflake -i {}` ✅
- `fd -e py -x uvx pyupgrade --py312-plus {}` ✅
- `fd -e py -x uvx ruff check --output-format=github --fix --unsafe-fixes {}` ❌ fails immediately with dozens of pre-existing lint hits (A004/F401/S108/F821 etc. across python/{haforu,tests} and examples; logged output, no code kept).
- `fd -e py -x uvx ruff format --respect-gitignore --target-version py312 {}` ✅ (ran, but reverted auto-edits on Python scripts/tests to avoid semantic churn).
- `uvx hatch test` ❌ fails because `pyproject.toml` points to non-existent `tests/` directory (pytest error: “file or directory not found: tests”).
- `cargo fmt` + `cargo test` ✅ (24 Rust unit tests + 3 CLI parser tests + doc test all pass).
