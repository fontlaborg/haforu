---
this_file: haforu/TODO.md
---

# Haforu Implementation TODO

**Current Status:** Core rendering complete ✅ (23/23 tests passing)
**Next Priority:** Python bindings implementation (H3)
**Timeline:** 12-18 days for Python bindings
**Goal:** Enable zero-overhead rendering for FontSimi deep matching

---

## Phase P1: Python Module Infrastructure ✅ COMPLETE (1-2 days)

- [x] Create `src/python/` directory structure
- [x] Add `src/python/mod.rs` with PyO3 module definition
- [x] Create `src/python/types.rs` for type conversions
- [x] Add PyO3 to Cargo.toml dependencies (`pyo3 = { version = "0.22", optional = true, features = ["extension-module"] }`)
- [x] Add numpy support to Cargo.toml (`numpy = { version = "0.22", optional = true }`)
- [x] Update `[features]` section: `python = ["pyo3", "numpy"]`
- [x] Update `[lib]` crate-type: `["cdylib", "rlib"]` (already configured)
- [x] Create `python/` directory for Python package
- [x] Create `python/haforu/__init__.py` with version export
- [x] Create `python/haforu/__init__.pyi` type stubs file
- [x] Create `python/haforu/py.typed` marker file (PEP 561)
- [x] Create `pyproject.toml` with maturin build configuration (already configured)
- [x] Set up `[build-system]` with maturin (already configured)
- [x] Set up `[project]` metadata (name, version, dependencies) (already configured)
- [x] Add numpy to Python dependencies (`numpy>=1.24`) (already configured as `numpy>=1.20`)
- [x] Configure `[tool.maturin]` with `module-name = "haforu._haforu"` (already configured)
- [x] Test build: `uv tool install maturin` (already installed)
- [x] Test build: `maturin build --features python` (successful)
- [x] Test import: `python -c "import haforu; print(haforu.__version__)"` (successful, returns "2.0.0")
- [x] Verify module structure is correct (verified)

---

## Phase P2: Batch Mode Bindings ✅ COMPLETE (2-3 days)

- [x] Create `src/python/batch.rs` file
- [x] Implement `ProcessJobsIterator` struct
- [x] Add mpsc channel for async result streaming
- [x] Add background thread for job processing
- [x] Implement `ProcessJobsIterator::new()` with sequential processing (parallel TODO)
- [x] Implement `ProcessJobsIterator.__iter__()` method
- [x] Implement `ProcessJobsIterator.__next__()` method
- [x] Add `#[pyfunction] process_jobs()` wrapper function
- [x] Add JSON parsing with error conversion to PyValueError
- [x] Add job validation (version, empty list) with error conversion
- [x] Export `process_jobs` in `src/python/mod.rs`
- [x] Create `python/tests/test_batch.py` test file (7 tests)
- [x] Write test: basic batch processing structure
- [x] Write test: empty job list (raises ValueError)
- [x] Write test: invalid JSON error handling
- [x] Write test: invalid version error handling
- [x] Write test: result format validation
- [x] Write test: iterator protocol (__iter__, __next__)
- [x] Update `python/haforu/__init__.py` with `process_jobs` export
- [x] Add type stub for `process_jobs()` in `__init__.pyi`
- [x] Run tests: `pytest python/tests/test_batch.py`
- [x] Verify all tests pass (7/7 passing)

---

## Phase P3: Streaming Session API ✅ COMPLETE (1 hour)

- [x] Create `src/python/streaming.rs` file
- [x] Implement `StreamingSession` struct with `#[pyclass]`
- [x] Add `font_loader: Arc<Mutex<FontLoader>>` field (using FontLoader instead of Orchestrator)
- [x] Implement `StreamingSession::new()` with optional cache_size
- [x] Add `#[pymethods]` block for StreamingSession
- [x] Implement `render(&self, job_json: &str) -> PyResult<String>`
- [x] Add JSON parsing for single job
- [x] Add job processing through font_loader (using process_job function)
- [x] Add JSON serialization of result
- [x] Add error conversion (Rust → Python exceptions)
- [x] Implement `close(&self)` to clear font cache
- [x] Implement `__enter__()` context manager method
- [x] Implement `__exit__()` context manager method (with PyO3 0.22 signature)
- [x] Ensure thread safety with Arc<Mutex>
- [x] Export `StreamingSession` in `src/python/mod.rs`
- [x] Create `python/tests/test_streaming.py` test file
- [x] Write test: create session and close
- [x] Write test: context manager usage
- [x] Write test: single job render
- [x] Write test: multiple sequential renders (10 jobs tested)
- [x] Write test: error handling (missing font)
- [x] Write test: result format validation
- [x] Add `StreamingSession` to `python/haforu/__init__.py`
- [x] Add type stub for `StreamingSession` class in `__init__.pyi`
- [x] Run tests: `pytest python/tests/test_streaming.py` (11/11 passing)
- [x] Build with maturin: `maturin develop --features python` (successful)

---

## Phase P4: Zero-Copy Numpy Integration ✅ COMPLETE (2 hours)

- [x] Add `numpy` crate to Cargo.toml dependencies
- [x] Add `use numpy::PyArray2` import
- [x] Implement `render_to_numpy()` in `StreamingSession`
- [x] Add parameters: `font_path, text, size, width, height`
- [x] Add optional parameters: `variations, script, direction, language`
- [x] Load font through FontLoader (reuses cache)
- [x] Shape text with TextShaper
- [x] Rasterize with GlyphRasterizer
- [x] Convert buffer to PyArray2<u8> using `from_vec2_bound()` (zero-copy)
- [x] Return `Bound<'py, PyArray2<u8>>` to Python (PyO3 0.22 API)
- [x] Create `python/tests/test_numpy.py` test file (12 tests)
- [x] Write test: basic numpy rendering
- [x] Write test: verify array shape (height, width)
- [x] Write test: verify dtype is uint8
- [x] Write test: verify array is contiguous (zero-copy)
- [x] Write test: verify array flags (c_contiguous)
- [x] Write test: variable font rendering
- [x] Write test: compare numpy output with base64 decoded output
- [x] Add docstrings for `render_to_numpy()`
- [x] Add type stub for `render_to_numpy()` method
- [x] Run tests: `pytest python/tests/test_numpy.py` (12/12 passing)
- [x] Build with maturin: `maturin develop --features python` (successful)
- [x] Run full test suite: 30/30 tests passing (7 batch + 12 numpy + 11 streaming)

---

## Phase P5: Error Handling & Edge Cases (1-2 days)

- [x] Create `src/python/errors.rs` for error conversions
- [x] Map haforu::Error variants to Python exceptions (IO→`PyIOError`, validation→`PyValueError`, runtime→`PyRuntimeError`)
- [x] Add context to error messages (font path; optional job ID in converter)
- [x] Implement `From<HaforuError> for PyErr` for ergonomic `?` usage
- [x] Update `process_jobs()` error handling (validate version before strict schema)
- [ ] Update `StreamingSession::render()` to use centralized converter (currently returns friendly messages)
- [ ] Update `StreamingSession::render_to_numpy()` to use centralized converter (currently returns friendly messages)
- [x] Create `python/tests/test_errors.py` test file
- [x] Write test: missing font file error
- [x] Write test: corrupted font file error
- [x] Write test: invalid JSON syntax error
- [x] Write test: missing required field error (version)
- [x] Write test: invalid version handling
- [x] Write test: invalid dimensions / size (guarded; error type flexible)
- [x] Write test: empty text content handling (graceful)
- [x] Write test: invalid variation coordinates / types
- [x] Write test: verify error messages are helpful and include context
- [x] Run full tests: 57 passed

Notes:
- Batch mode JSON errors now prioritise version validation to avoid schema-parse noise.
- Centralized error conversion supports optional job context for future use.

---

## Phase P6: Documentation & Examples ✅ COMPLETE (~2 hours)

- [x] Write comprehensive docstrings for `process_jobs()` (already complete in Rust)
- [x] Write comprehensive docstrings for `StreamingSession` class (already complete in Rust)
- [x] Write comprehensive docstrings for `render()` method (already complete in Rust)
- [x] Write comprehensive docstrings for `render_to_numpy()` method (already complete in Rust)
- [x] Write comprehensive docstrings for `close()` method (already complete in Rust)
- [x] Update `python/haforu/__init__.pyi` with complete type stubs (already complete)
- [x] Add parameter types and return types to all stubs (already complete)
- [x] Add docstrings to type stubs (already complete)
- [x] Create `examples/python/` directory
- [x] Create `examples/python/batch_demo.py` example (169 lines with comprehensive comments)
- [x] Create `examples/python/streaming_demo.py` example (127 lines with comprehensive comments)
- [x] Create `examples/python/numpy_demo.py` example (175 lines with comprehensive comments)
- [x] Create `examples/python/error_handling_demo.py` example (226 lines with comprehensive comments)
- [x] Add comments explaining each example (extensive inline documentation)
- [x] Update `README.md` with Python bindings section (replaced "Future" section)
- [x] Add installation instructions (`maturin develop --features python`)
- [x] Add quick start example (batch mode)
- [x] Add quick start example (streaming mode)
- [x] Add performance comparison table (CLI vs Python)
- [x] Document API reference (all functions and classes with parameters, returns, raises)
- [x] Document error handling patterns (comprehensive in error_handling_demo.py)
- [x] Run example scripts to verify they work (all 4 examples tested and passing)
- [x] Verify README examples are copy-paste ready (tested)

---

## Phase P7: Build & Distribution (2-3 days)

- [ ] Test local build: `maturin develop --features python`
- [ ] Test wheel build: `maturin build --release --features python`
- [ ] Verify wheel contents (CLI binary included)
- [ ] Test wheel installation: `pip install target/wheels/*.whl`
- [ ] Test import after wheel install
- [ ] Create `.github/workflows/build-wheels.yml`
- [ ] Add matrix build for platforms (macOS, Linux, Windows)
- [ ] Add matrix build for Python versions (3.11, 3.12)
- [ ] Add maturin build step with `--release --features python`
- [ ] Add artifact upload for wheels
- [ ] Test GitHub Actions workflow locally (act)
- [ ] Create release workflow (`release.yml`)
- [ ] Add version tagging step
- [ ] Add changelog generation step
- [ ] Add PyPI upload step (test.pypi.org first)
- [ ] Test release workflow with test.pypi.org
- [ ] Verify wheel installs from test.pypi.org
- [ ] Document installation from PyPI in README
- [ ] Document development installation in CONTRIBUTING.md
- [ ] Create RELEASING.md with release checklist
- [ ] Test multi-platform builds (macOS x86_64, aarch64)
- [ ] Test multi-platform builds (Linux x86_64, aarch64)
- [ ] Test multi-platform builds (Windows x86_64)
- [ ] Verify all platform wheels work on clean machines
- [ ] Document platform-specific issues (if any)

---

## Testing & Validation

- [ ] Run full Rust test suite: `cargo test --features python`
- [ ] Run full Python test suite: `pytest python/tests/ -v`
- [ ] Run integration tests: `pytest python/tests/test_integration.py`
- [ ] Run performance benchmarks
- [ ] Verify batch mode: 100-150 jobs/sec
- [ ] Verify streaming mode: 1-2ms per render
- [ ] Verify memory usage: <2GB for 5000-job batch
- [ ] Test memory leaks: 1M renders with no growth
- [ ] Test thread safety: concurrent renders from multiple threads
- [ ] Run mypy: `mypy python/haforu/ --strict`
- [ ] Run type checking on examples
- [ ] Run clippy: `cargo clippy --features python -- -D warnings`
- [ ] Run fmt: `cargo fmt --check`
- [ ] Verify all clippy warnings resolved
- [ ] Verify code is properly formatted

---

## Success Criteria

### Technical Metrics
- [ ] All Rust tests passing (23+ tests)
- [ ] All Python tests passing (20+ tests)
- [ ] Batch mode: 100-150 jobs/sec ✅
- [ ] Streaming mode: 1-2ms per render ✅
- [ ] Memory: <2GB for 5000-job batch ✅
- [ ] No memory leaks over 1M renders ✅
- [ ] Thread-safe concurrent rendering ✅

### Integration Metrics (with FontSimi)
- [ ] CLI batch mode working (already done) ✅
- [ ] Python bindings rendering matches CLI output ✅
- [ ] Identical Daidot metrics vs CoreText/HarfBuzz ✅
- [ ] Deep matching: 50× faster with Python bindings ✅
- [ ] FontSimi integration tests passing ✅

### Quality Metrics
- [ ] Type-safe Python API (mypy strict mode) ✅
- [ ] Comprehensive error handling ✅
- [ ] Documentation complete with examples ✅
- [ ] Multi-platform wheels available ✅
- [ ] PyPI package published ✅

---

## IMMEDIATE NEXT STEPS

**Start here:**
1. Phase P1: Python module infrastructure (1-2 days)
2. Phase P2: Batch mode bindings (2-3 days)
3. Phase P3: Streaming session API (3-4 days)

**Then:**
4. Phase P4: Zero-copy numpy integration (2-3 days)
5. Phase P5: Error handling (1-2 days)
6. Phase P6: Documentation (1-2 days)
7. Phase P7: Build & distribution (2-3 days)

**Timeline:** 12-18 days total

**Blocking:** Nothing - ready to start immediately

---

**Document Version:** 2.0
**Last Updated:** 2025-11-13
**Status:** Ready for Python bindings implementation
**Next Task:** P1.1 - Create `src/python/` directory structure
