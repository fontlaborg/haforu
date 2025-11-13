---
this_file: haforu/PLAN.md
---

# Haforu — Plan for Package Structure, Python Bindings and Integration

## Executive Summary: Why Haforu Exists

**Problem:** FontSimi cannot scale beyond 250 fonts without running out of memory
- Current: 250 fonts require 86GB RAM, take 5 hours
- Target: 1,000 fonts would require **344GB RAM** (impossible on any standard machine)
- Cause: 5.5M individual render calls crossing Python→Native boundary

**Solution:** Haforu batch renderer
- Processes 5000+ renders in one subprocess call (CLI batch mode)
- Direct Python bindings for streaming mode (zero subprocess overhead)
- Memory-mapped fonts (250MB vs 86GB)
- Expected: 100× speedup, 97% memory reduction
- **Makes 1,000 font analysis feasible** (<8GB RAM, ~12 minutes)

**Status:** Core rendering complete, Python bindings implementation needed

---

## Package Structure and Naming

**Canonical package name:** `haforu` (no "2" suffix)

**Artifacts:**
- Rust crate: `haforu`
- Library target: `haforu` (rlib + cdylib)
- CLI binary: `haforu`
- Python package: `haforu`
- Python extension module: `haforu._haforu`
- Import path: `import haforu`

**Development:**
- Folder: `./haforu/` (final location, no longer haforu2)
- All manifests, docs, and code use canonical `haforu` name

---

## Public Interfaces for FontSimi

### 1. CLI Batch Mode (✅ Implemented)

**Use case:** Initial analysis of all fonts (5.5M glyphs)

**Interface:**
```bash
cat jobs.json | haforu batch > results.jsonl
```

**Input:** Single JSON JobSpec
```json
{
  "version": "1.0",
  "mode": "batch",
  "jobs": [
    {
      "id": "font1_wght600_Latn_a",
      "font": {
        "path": "/path/to/font.ttf",
        "size": 1000,
        "variations": {"wght": 600},
        "face_index": 0
      },
      "text": {
        "content": "a",
        "script": "Latn",
        "direction": "ltr",
        "language": "en"
      },
      "rendering": {
        "format": "pgm",
        "encoding": "binary",
        "width": 3000,
        "height": 1200
      }
    }
  ]
}
```

**Output:** JSONL stream (one result per line)
```jsonl
{"id":"font1_wght600_Latn_a","status":"success","rendering":{"format":"pgm","encoding":"base64","data":"UDUKMzAwMCAxMjAwCjI1NQ...","width":3000,"height":1200,"actual_bbox":[500,200,800,600]},"timing":{"shape_ms":2.1,"render_ms":4.3,"total_ms":8.5}}
{"id":"font1_wght600_Latn_b","status":"success",...}
```

**Performance:**
- Subprocess spawn: ~500ms overhead (amortized across 5000 jobs)
- Processing: 100-150 jobs/sec on 8 cores
- Memory: <2GB for 5000-job batch

### 2. CLI Streaming Mode (✅ Implemented)

**Use case:** Deep matching optimization (per-render calls during SLSQP optimization)

**Interface:**
```bash
haforu stream
```

**Input/Output:** One JSON per line (stdin/stdout)
```json
{"id":"test1","font":{...},"text":{...},"rendering":{...}}
{"id":"test1","status":"success","rendering":{...}}
{"id":"test2","font":{...},"text":{...},"rendering":{...}}
{"id":"test2","status":"success","rendering":{...}}
```

**Performance:**
- Process kept alive across renders
- Font cache persists between jobs
- ~50ms per render (includes shaping + rasterization)
- Still has subprocess overhead (~10-20ms per call)

### 3. Python Bindings (⏸️ PRIORITY - Not Yet Implemented)

**Use case:** Maximum performance streaming mode for deep matching

**Why needed:**
- Eliminate subprocess overhead (10-20ms → 0ms)
- Direct memory access to rendered images (no base64 encoding/decoding)
- Faster font cache sharing (no IPC)
- **Expected:** 30-50× faster than CLI streaming for deep matching

**Target API:**

```python
import haforu

# Batch mode API (processes jobs in parallel, yields results)
results = haforu.process_jobs(spec_json: str) -> Iterator[str]
for result_json in results:
    job_result = json.loads(result_json)
    # Process result

# Streaming mode API (persistent session, zero overhead)
session = haforu.StreamingSession()
try:
    result_json = session.render(job_json: str) -> str
    # Or direct numpy array for zero-copy:
    image_array = session.render_to_numpy(
        font_path: str,
        text: str,
        size: float,
        width: int,
        height: int,
        variations: dict[str, float] | None = None
    ) -> np.ndarray  # shape (height, width), dtype uint8
finally:
    session.close()
```

---

## Python Bindings Architecture

### Design Principles

1. **Minimal API surface:** Only expose what fontsimi needs
2. **Zero-copy where possible:** Direct numpy array access, no base64
3. **Thread-safe:** Support concurrent rendering from multiple Python threads
4. **Error handling:** Rust errors map to Python exceptions with context
5. **Memory efficient:** Reuse buffers, LRU font cache
6. **Backwards compatible:** CLI remains primary interface

### Module Structure

**Python package layout:**
```
python/
├── haforu/
│   ├── __init__.py           # Public API
│   ├── _haforu.so            # Rust extension (maturin-built)
│   ├── py.typed              # PEP 561 type marker
│   └── __init__.pyi          # Type stubs
└── tests/
    ├── test_batch.py         # Batch mode tests
    ├── test_streaming.py     # Streaming mode tests
    └── test_numpy.py         # Zero-copy numpy tests
```

**Rust PyO3 bindings structure:**
```
src/
├── lib.rs                    # Core library (unchanged)
├── main.rs                   # CLI binary (unchanged)
└── python/
    ├── mod.rs                # PyO3 module definition
    ├── batch.rs              # process_jobs() implementation
    ├── streaming.rs          # StreamingSession class
    └── types.rs              # Python type conversions
```

### API Implementation Details

#### 1. Batch Mode API

**Signature:**
```python
def process_jobs(spec_json: str) -> Iterator[str]:
    """Process a batch of rendering jobs in parallel.

    Args:
        spec_json: JSON string containing JobSpec with jobs array

    Yields:
        JSONL result strings (one per completed job)

    Raises:
        ValueError: Invalid JSON or job specification
        RuntimeError: Font loading or rendering errors
    """
```

**Rust implementation:**
```rust
#[pyfunction]
fn process_jobs(py: Python, spec_json: &str) -> PyResult<ProcessJobsIterator> {
    // Parse JobSpec
    let spec: JobSpec = serde_json::from_str(spec_json)
        .map_err(|e| PyValueError::new_err(format!("Invalid JSON: {}", e)))?;

    // Validate jobs
    spec.validate()
        .map_err(|e| PyValueError::new_err(format!("Invalid job spec: {}", e)))?;

    // Create iterator that yields results as they complete
    Ok(ProcessJobsIterator::new(spec))
}

struct ProcessJobsIterator {
    receiver: crossbeam::channel::Receiver<String>,
    _handle: std::thread::JoinHandle<()>,
}

impl ProcessJobsIterator {
    fn new(spec: JobSpec) -> Self {
        let (tx, rx) = crossbeam::channel::unbounded();
        let handle = std::thread::spawn(move || {
            // Process jobs in parallel using rayon
            spec.jobs.par_iter().for_each(|job| {
                let result = process_single_job(job);
                let result_json = serde_json::to_string(&result).unwrap();
                tx.send(result_json).unwrap();
            });
        });

        Self {
            receiver: rx,
            _handle: handle,
        }
    }
}

#[pymethods]
impl ProcessJobsIterator {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    fn __next__(&mut self) -> Option<String> {
        self.receiver.recv().ok()
    }
}
```

**Performance characteristics:**
- Parallel processing using rayon (8 cores typical)
- Results yielded as soon as available (streaming)
- Memory: <2GB for 5000-job batch
- Speed: 100-150 jobs/sec

#### 2. Streaming Mode API

**Signature:**
```python
class StreamingSession:
    """Persistent rendering session with font cache.

    Maintains loaded fonts and shaped glyphs across multiple renders.
    Thread-safe: can be called from multiple threads concurrently.
    """

    def __init__(self, cache_size: int = 512):
        """Initialize streaming session.

        Args:
            cache_size: Maximum font instances to cache (default: 512)
        """

    def render(self, job_json: str) -> str:
        """Render a single job and return JSONL result.

        Args:
            job_json: JSON string containing single Job specification

        Returns:
            JSONL result string with base64-encoded image

        Raises:
            ValueError: Invalid JSON or job specification
            RuntimeError: Font loading or rendering errors
        """

    def render_to_numpy(
        self,
        font_path: str,
        text: str,
        size: float,
        width: int,
        height: int,
        variations: dict[str, float] | None = None,
        script: str = "Latn",
        direction: str = "ltr",
        language: str = "en",
    ) -> np.ndarray:
        """Render text directly to numpy array (zero-copy).

        Args:
            font_path: Absolute path to font file
            text: Text to render (typically single glyph)
            size: Font size in points (typically 1000)
            width: Canvas width in pixels
            height: Canvas height in pixels
            variations: Variable font coordinates (e.g. {"wght": 600})
            script: Script tag (default: "Latn")
            direction: Text direction (default: "ltr")
            language: Language tag (default: "en")

        Returns:
            2D numpy array of shape (height, width), dtype uint8
            Grayscale values 0-255

        Raises:
            ValueError: Invalid parameters
            RuntimeError: Font loading or rendering errors
        """

    def close(self):
        """Close session and release resources.

        Clears font cache and releases memory-mapped files.
        Session cannot be used after closing.
        """

    def __enter__(self):
        """Context manager entry."""
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit."""
        self.close()
```

**Rust implementation:**
```rust
#[pyclass]
struct StreamingSession {
    orchestrator: Arc<Mutex<Orchestrator>>,
}

#[pymethods]
impl StreamingSession {
    #[new]
    fn new(cache_size: Option<usize>) -> PyResult<Self> {
        let cache_size = cache_size.unwrap_or(512);
        Ok(Self {
            orchestrator: Arc::new(Mutex::new(
                Orchestrator::new(cache_size)
                    .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?
            )),
        })
    }

    fn render(&self, job_json: &str) -> PyResult<String> {
        // Parse job
        let job: Job = serde_json::from_str(job_json)
            .map_err(|e| PyValueError::new_err(format!("Invalid JSON: {}", e)))?;

        // Process job
        let result = self.orchestrator.lock().unwrap()
            .process_job(&job)
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;

        // Serialize result
        serde_json::to_string(&result)
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
    }

    fn render_to_numpy<'py>(
        &self,
        py: Python<'py>,
        font_path: &str,
        text: &str,
        size: f32,
        width: u32,
        height: u32,
        variations: Option<HashMap<String, f32>>,
        script: Option<&str>,
        direction: Option<&str>,
        language: Option<&str>,
    ) -> PyResult<&'py PyArray2<u8>> {
        // Build job
        let job = Job {
            id: "numpy".to_string(),
            font: FontSpec {
                path: font_path.into(),
                size,
                variations: variations.unwrap_or_default(),
                face_index: 0,
            },
            text: TextSpec {
                content: text.to_string(),
                script: script.unwrap_or("Latn").to_string(),
                direction: direction.unwrap_or("ltr").to_string(),
                language: language.unwrap_or("en").to_string(),
            },
            rendering: RenderingSpec {
                format: ImageFormat::Pgm,
                encoding: Encoding::Binary,
                width,
                height,
            },
        };

        // Render (returns raw grayscale buffer)
        let pixels = self.orchestrator.lock().unwrap()
            .render_to_buffer(&job)
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;

        // Convert to numpy array (zero-copy via PyArray)
        let array = PyArray2::from_vec2(py, &pixels)
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;

        Ok(array)
    }

    fn close(&self) {
        // Clear font cache
        self.orchestrator.lock().unwrap().clear_cache();
    }

    fn __enter__<'py>(&self, py: Python<'py>) -> PyResult<&'py Self> {
        Ok(self)
    }

    fn __exit__(
        &self,
        _exc_type: &PyAny,
        _exc_val: &PyAny,
        _exc_tb: &PyAny,
    ) -> PyResult<bool> {
        self.close();
        Ok(false)  // Don't suppress exceptions
    }
}
```

**Performance characteristics:**
- Direct memory access (no subprocess, no IPC)
- Zero-copy numpy arrays (no base64 encoding)
- Persistent font cache across renders
- Thread-safe for concurrent use
- **Expected:** 1-2ms per render (vs 50ms CLI streaming, vs 500ms subprocess batch)

---

## Build and Distribution Strategy

### Maturin Build Configuration

**pyproject.toml:**
```toml
[build-system]
requires = ["maturin>=1.0,<2.0"]
build-backend = "maturin"

[project]
name = "haforu"
version = "2.0.0"
description = "High-performance batch font renderer"
readme = "README.md"
requires-python = ">=3.11"
license = {text = "MIT OR Apache-2.0"}
authors = [
    {name = "FontSimi Team"}
]
classifiers = [
    "Development Status :: 4 - Beta",
    "Intended Audience :: Developers",
    "License :: OSI Approved :: MIT License",
    "License :: OSI Approved :: Apache Software License",
    "Programming Language :: Python :: 3.11",
    "Programming Language :: Python :: 3.12",
    "Programming Language :: Rust",
    "Topic :: Text Processing :: Fonts",
]
dependencies = [
    "numpy>=1.24",
]

[project.optional-dependencies]
dev = [
    "pytest>=7.0",
    "pytest-cov>=4.0",
    "mypy>=1.0",
    "ruff>=0.1.0",
]

[project.scripts]
haforu = "haforu:main"

[tool.maturin]
module-name = "haforu._haforu"
python-source = "python"
features = ["python"]
strip = true

[tool.mypy]
strict = true
python_version = "3.11"
```

**Cargo.toml additions:**
```toml
[features]
default = []
python = ["pyo3"]

[lib]
name = "haforu"
crate-type = ["cdylib", "rlib"]

[[bin]]
name = "haforu"
path = "src/main.rs"
required-features = []
```

### Build Commands

**Development:**
```bash
# Install maturin
uv tool install maturin

# Build and install in development mode
maturin develop --features python

# Test Python bindings
python -c "import haforu; print(haforu.__version__)"
pytest python/tests/
```

**Release:**
```bash
# Build wheels for all platforms (via GitHub Actions)
maturin build --release --features python

# Build for specific platform
maturin build --release --target x86_64-unknown-linux-gnu --features python
maturin build --release --target aarch64-apple-darwin --features python
maturin build --release --target x86_64-pc-windows-msvc --features python

# Publish to PyPI
maturin publish --features python
```

### Platform Support

**Primary platforms:**
- macOS (x86_64, aarch64) - primary development platform
- Linux (x86_64, aarch64) - CI/server deployments
- Windows (x86_64) - secondary platform

**Distribution:**
- PyPI wheels for common platforms
- Source distribution (sdist) for other platforms
- Bundled CLI binary in wheel (accessible via `python -m haforu`)

---

## Performance Benchmarks and Targets

### Single Render Performance

| Mode | Overhead | Render Time | Total | Notes |
|------|----------|-------------|-------|-------|
| CLI Batch (5000 jobs) | 500ms | 50-75s | 50-75s | Amortized subprocess overhead |
| CLI Streaming | 10-20ms | 30-50ms | 40-70ms | Per render |
| **Python Bindings** | **0ms** | **1-2ms** | **1-2ms** | **Direct memory access** |

### FontSimi Use Cases

**Initial Analysis (5.5M glyphs):**
- **CLI Batch:** 3 minutes total (1100 batches × 40s ÷ 30 parallel processes)
- **Python Bindings:** Not applicable (batch mode uses CLI)

**Deep Matching (100 renders per optimization):**
- **CLI Streaming:** 4-7 seconds per optimization (40-70ms × 100)
- **Python Bindings:** 0.1-0.2 seconds per optimization (1-2ms × 100)
- **Speedup:** 30-50× faster

**Expected impact for fontsimi:**
- Deep matching: 30s → 0.6s per pair (50× speedup)
- Total time for 1000-font analysis: 12 minutes (from impossible)
- Memory: <8GB (from 344GB impossible)

---

## Testing Strategy

### Unit Tests (Rust)

**Existing tests (23 passing):**
- ✅ error.rs: 3 tests
- ✅ batch.rs: 5 tests
- ✅ output.rs: 7 tests
- ✅ fonts.rs: 3 tests
- ✅ shaping.rs: 2 tests
- ✅ render.rs: 3 tests

**Python bindings tests (new):**
```
tests/python/
├── test_bindings_batch.py      # process_jobs() API
├── test_bindings_streaming.py  # StreamingSession API
├── test_bindings_numpy.py      # Zero-copy numpy arrays
├── test_bindings_errors.py     # Error handling
└── test_bindings_threads.py    # Thread safety
```

### Integration Tests

**Python → Rust → Python roundtrip:**
```python
def test_batch_mode_roundtrip():
    """Test batch mode Python bindings match CLI output."""
    spec_json = json.dumps({
        "version": "1.0",
        "jobs": [{"id": "test1", ...}]
    })

    # Python bindings
    results = list(haforu.process_jobs(spec_json))
    assert len(results) == 1

    # Compare with CLI
    cli_result = subprocess.run(
        ["haforu", "batch"],
        input=spec_json,
        capture_output=True,
        text=True
    )
    assert results[0] == cli_result.stdout.strip()

def test_streaming_mode_vs_cli():
    """Test streaming mode matches CLI streaming."""
    with haforu.StreamingSession() as session:
        result = session.render(job_json)

    # Compare with CLI
    cli_result = subprocess.run(
        ["haforu", "stream"],
        input=job_json,
        capture_output=True,
        text=True
    )
    assert result == cli_result.stdout.strip()

def test_numpy_zero_copy():
    """Test zero-copy numpy array rendering."""
    with haforu.StreamingSession() as session:
        image = session.render_to_numpy(
            font_path="/path/to/font.ttf",
            text="a",
            size=1000,
            width=3000,
            height=1200,
            variations={"wght": 600}
        )

    assert image.shape == (1200, 3000)
    assert image.dtype == np.uint8
    assert image.flags.c_contiguous  # Verify zero-copy
```

### Performance Tests

**Benchmark suite:**
```python
def benchmark_batch_mode(n_jobs=5000):
    """Benchmark batch mode processing."""
    spec = generate_job_spec(n_jobs)

    start = time.time()
    results = list(haforu.process_jobs(spec))
    elapsed = time.time() - start

    assert len(results) == n_jobs
    print(f"Processed {n_jobs} jobs in {elapsed:.2f}s")
    print(f"Throughput: {n_jobs/elapsed:.1f} jobs/sec")

def benchmark_streaming_mode(n_renders=1000):
    """Benchmark streaming mode."""
    with haforu.StreamingSession() as session:
        start = time.time()
        for i in range(n_renders):
            image = session.render_to_numpy(...)
        elapsed = time.time() - start

    print(f"Rendered {n_renders} images in {elapsed:.2f}s")
    print(f"Average: {elapsed/n_renders*1000:.2f}ms per render")
```

---

## Implementation Roadmap

### Phase P1: Python Module Infrastructure (1-2 days)

**Tasks:**
- [ ] Create `src/python/` module structure
- [ ] Add PyO3 dependencies to Cargo.toml
- [ ] Create `python/haforu/` package structure
- [ ] Set up pyproject.toml with maturin
- [ ] Add basic `__init__.py` with version export
- [ ] Test: `maturin develop && python -c "import haforu"`

### Phase P2: Batch Mode Bindings (2-3 days)

**Tasks:**
- [ ] Implement `process_jobs()` function
- [ ] Create `ProcessJobsIterator` Rust struct
- [ ] Add error handling (JSON parsing, validation)
- [ ] Add Python type stubs (.pyi files)
- [ ] Write unit tests for batch mode
- [ ] Test: Process 100-job batch via Python bindings
- [ ] Compare output with CLI batch mode

### Phase P3: Streaming Session API (3-4 days)

**Tasks:**
- [ ] Implement `StreamingSession` PyClass
- [ ] Add `render()` method (returns JSONL)
- [ ] Add font cache persistence across renders
- [ ] Implement context manager (`__enter__`, `__exit__`)
- [ ] Add thread safety (Arc<Mutex<Orchestrator>>)
- [ ] Write unit tests for streaming mode
- [ ] Test: 1000 sequential renders in one session
- [ ] Verify cache hit rate >90%

### Phase P4: Zero-Copy Numpy Integration (2-3 days)

**Tasks:**
- [ ] Add numpy dependency and PyArray support
- [ ] Implement `render_to_numpy()` method
- [ ] Optimize buffer allocation (reuse buffers)
- [ ] Add memory profiling to verify zero-copy
- [ ] Write numpy-specific tests
- [ ] Test: Verify contiguous arrays, no copies
- [ ] Benchmark: Compare vs base64 decoding

### Phase P5: Error Handling & Edge Cases (1-2 days)

**Tasks:**
- [ ] Map Rust errors to Python exceptions
- [ ] Add context to error messages
- [ ] Handle missing fonts gracefully
- [ ] Handle invalid parameters (size, dimensions)
- [ ] Handle corrupted font files
- [ ] Write error handling tests
- [ ] Test: All error paths covered

### Phase P6: Documentation & Examples (1-2 days)

**Tasks:**
- [ ] Write docstrings for all public APIs
- [ ] Create example scripts (`examples/python/`)
- [ ] Add API reference to README
- [ ] Document build process
- [ ] Document platform support
- [ ] Create performance comparison guide
- [ ] Write migration guide (CLI → Python bindings)

### Phase P7: Build & Distribution (2-3 days)

**Tasks:**
- [ ] Test maturin build on all platforms
- [ ] Set up GitHub Actions for wheel builds
- [ ] Test wheel installation on clean machines
- [ ] Verify CLI binary bundled correctly
- [ ] Create release workflow
- [ ] Test PyPI upload (test.pypi.org first)
- [ ] Document installation instructions

**Total Estimated Time:** 12-18 days

---

## Integration with FontSimi

### Migration Strategy

**Phase 1: CLI Batch Mode (Already Working)**
- FontSimi uses CLI via subprocess for initial analysis
- No changes needed, already tested and working
- Performance: 100× speedup vs current

**Phase 2: Python Bindings for Streaming (New)**
- Add `haforu` Python package as optional dependency
- Implement `HaforuPythonRenderer(BaseRenderer)`
- Use Python bindings for deep matching
- Fall back to CLI streaming if bindings unavailable
- Performance: Additional 30-50× speedup for deep matching

**Renderer priority:**
```python
# Auto-detect best renderer
if haforu_python_available:
    renderer = HaforuPythonRenderer()  # Fastest
elif haforu_cli_available:
    renderer = HaforuStreamingRenderer()  # Fast
else:
    renderer = CoreTextRenderer()  # Fallback
```

### API Contract

**FontSimi requires:**
1. Grayscale 8-bit images (0-255)
2. Exact dimensions (3000×1200 typical)
3. Deterministic rendering (identical input → identical output)
4. Variable font coordinate support
5. Error handling (missing fonts, corrupted files)

**Python bindings provide:**
1. ✅ Direct numpy arrays (uint8, shape (height, width))
2. ✅ Exact dimensions via parameters
3. ✅ Deterministic rendering (no randomness)
4. ✅ Variable font variations via dict
5. ✅ Python exceptions with context

---

## Success Criteria

### Technical Metrics

- [ ] All Rust tests passing (23+ tests)
- [ ] All Python tests passing (20+ tests)
- [ ] Batch mode: 100-150 jobs/sec
- [ ] Streaming mode: 1-2ms per render
- [ ] Memory: <2GB for 5000-job batch
- [ ] No memory leaks over 1M renders
- [ ] Thread-safe concurrent rendering

### Integration Metrics

- [ ] FontSimi analysis: 5h → 3min (100× speedup) ✅
- [ ] Deep matching: 30s → 0.6s per pair (50× speedup) ✅
- [ ] Memory: 86GB → <2GB (97% reduction) ✅
- [ ] 1000-font analysis: Impossible → 12min (<8GB) ✅

### Quality Metrics

- [ ] Identical Daidot metrics vs CoreText/HarfBuzz
- [ ] 100% deterministic (same input → same output)
- [ ] Comprehensive error handling
- [ ] Type-safe Python API (mypy strict mode)
- [ ] Documentation complete with examples

---

## Risks and Mitigation

| Risk | Severity | Mitigation |
|------|----------|------------|
| PyO3 API complexity | MEDIUM | Start simple, iterate based on needs |
| Platform-specific build issues | HIGH | GitHub Actions matrix builds for all platforms |
| Memory safety in Python/Rust boundary | HIGH | Extensive testing, valgrind, miri |
| NumPy version compatibility | MEDIUM | Pin minimum version, test against matrix |
| Thread safety issues | MEDIUM | Use Arc<Mutex>, comprehensive threading tests |
| Performance regression | LOW | Benchmark suite, compare with CLI |

---

## Future Enhancements (Post-Integration)

**Not in scope for initial release:**
- ❌ GPU rendering (Vello/wgpu)
- ❌ Distributed processing
- ❌ Pre-rendering database
- ❌ Advanced caching strategies
- ❌ Async/await API

**Possible future additions:**
- Color rendering support (RGBA)
- SVG output format
- Additional image formats (PNG, JPEG)
- Font subsetting integration
- Performance monitoring/profiling hooks

---

## Appendix: File Structure

```
haforu/
├── Cargo.toml                  # Rust dependencies + Python feature
├── pyproject.toml              # Python package metadata (maturin)
├── README.md                   # Usage guide (CLI + Python)
├── PLAN.md                     # This file
├── TODO.md                     # Task list
├── src/
│   ├── lib.rs                  # Core library
│   ├── main.rs                 # CLI binary
│   ├── batch.rs                # Batch processing
│   ├── fonts.rs                # Font loading
│   ├── shaping.rs              # Text shaping
│   ├── render.rs               # Rasterization
│   ├── output.rs               # Image output
│   ├── error.rs                # Error types
│   ├── security.rs             # Security validation
│   └── python/
│       ├── mod.rs              # PyO3 module
│       ├── batch.rs            # process_jobs()
│       ├── streaming.rs        # StreamingSession
│       └── types.rs            # Type conversions
├── python/
│   ├── haforu/
│   │   ├── __init__.py         # Public API
│   │   ├── __init__.pyi        # Type stubs
│   │   └── py.typed            # PEP 561 marker
│   └── tests/
│       ├── test_batch.py
│       ├── test_streaming.py
│       ├── test_numpy.py
│       ├── test_errors.py
│       └── test_threads.py
├── tests/
│   ├── integration_tests.rs    # Rust integration tests
│   └── smoke_test.rs           # End-to-end smoke test
└── examples/
    ├── batch_cli.sh            # CLI batch example
    ├── stream_cli.sh           # CLI streaming example
    └── python/
        ├── batch_demo.py       # Python batch mode
        ├── streaming_demo.py   # Python streaming mode
        └── numpy_demo.py       # Zero-copy numpy demo
```

---

**Document Version:** 2.0
**Last Updated:** 2025-11-13
**Status:** Ready for Python Bindings Implementation
