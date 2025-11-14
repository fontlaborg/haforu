---
this_file: docs/REPOSITORY.md
---

# Repository Structure & Best Practices

This document describes haforu's repository layout, build configuration, and adherence to Rust + PyO3 + Hatch best practices.

## Overview

Haforu is a **hybrid Rust + Python project** using:
- **Rust** - Core rendering engine (`src/`)
- **PyO3** - Python bindings (`src/python/`)
- **Maturin** - Build backend for Python wheels
- **Hatch** - Python testing and tooling
- **Fire** - Python CLI framework

## Directory Structure

```
haforu/
├── .cargo/
│   └── config.toml          # Cargo configuration, build profiles, aliases
├── .github/                  # GitHub Actions workflows (to be added)
├── benches/
│   └── cli.rs               # Criterion benchmarks for CLI hot paths
├── docs/
│   ├── CLI-USAGE.md         # Comprehensive CLI documentation
│   └── REPOSITORY.md        # This file
├── examples/
│   └── python/              # Python usage examples
├── python/
│   ├── haforu/
│   │   ├── __init__.py      # Python package entry point
│   │   ├── __main__.py      # Fire CLI implementation
│   │   └── _version.py      # Auto-generated version (hatch-vcs)
│   └── tests/               # Python test suite (pytest)
├── scripts/
│   ├── build.sh             # Comprehensive build script
│   ├── run.sh               # Multi-mode test runner
│   ├── profile-cli.sh       # Performance profiling
│   ├── regression-test.sh   # Performance regression gates
│   └── test-cli-parity.sh   # Python/Rust CLI parity tests
├── src/
│   ├── batch.rs             # Job specification structs
│   ├── error.rs             # Error types
│   ├── fonts.rs             # Font loader with caching
│   ├── lib.rs               # Public API
│   ├── main.rs              # Rust CLI
│   ├── output.rs            # Image output (PGM/PNG)
│   ├── python/              # PyO3 bindings
│   ├── render.rs            # Rasterization
│   ├── security.rs          # Security validation
│   └── shaping.rs           # HarfBuzz shaping
├── testdata/
│   └── fonts/               # Test fonts
├── Cargo.toml               # Rust package manifest
├── pyproject.toml           # Python package manifest
├── CHANGELOG.md             # Release notes
├── PLAN.md                  # Project planning
├── TODO.md                  # Task tracking
└── WORK.md                  # Work session notes
```

## Configuration Files

### Cargo.toml

**Purpose:** Rust package manifest and build configuration

**Key Sections:**
- `[package]` - Metadata (name, version, description, license)
- `[lib]` - Library configuration (`cdylib` for Python, `rlib` for Rust)
- `[[bin]]` - Binary target (haforu CLI)
- `[[bench]]` - Benchmark configuration
- `[dependencies]` - Production dependencies
- `[dev-dependencies]` - Test/benchmark dependencies
- `[features]` - Feature flags (default, python)
- `[profile.*]` - Build profiles (dev, release)

**Best Practices Applied:**
- ✅ Separate `cdylib` and `rlib` targets
- ✅ Explicit feature flags for Python bindings
- ✅ Optimized release profile (LTO, strip, single codegen-unit)
- ✅ Rust edition 2021
- ✅ MSRV documented (1.70)

### pyproject.toml

**Purpose:** Python package manifest and tool configuration

**Key Sections:**
- `[build-system]` - Maturin backend with hatch-vcs
- `[project]` - Package metadata
- `[project.scripts]` - Console entry points (`haforu-py`)
- `[tool.maturin]` - Maturin configuration
- `[tool.pytest.ini_options]` - Pytest configuration
- `[tool.hatch.version]` - VCS-based versioning
- `[tool.ruff]` - Linting configuration
- `[tool.mypy]` - Type checking configuration

**Best Practices Applied:**
- ✅ Dynamic versioning from git tags (hatch-vcs)
- ✅ Separate Python source directory (`python/`)
- ✅ Console entry point for CLI (`haforu-py`)
- ✅ Platform-specific optional dependencies
- ✅ Modern Python 3.8+ support
- ✅ Comprehensive metadata (classifiers, keywords)

### .cargo/config.toml

**Purpose:** Cargo build configuration and platform-specific settings

**Key Features:**
- Build profiles (dev, release, release-with-debug, bench, dist)
- Platform-specific rustflags
- Linker optimizations (LLD on Linux)
- Profiling support (frame pointers, debug symbols)
- Command aliases for convenience
- Network configuration (retries, git CLI)

**Best Practices Applied:**
- ✅ Frame pointers enabled for profiling
- ✅ Platform-specific optimizations
- ✅ Multiple build profiles for different use cases
- ✅ LTO and size optimizations for distribution
- ✅ Split debug info on macOS for faster builds
- ✅ Sparse registry protocol for faster downloads

## Build System

### Rust Build

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# With Python bindings
cargo build --release --features python

# Run tests
cargo test

# Run benchmarks
cargo bench
```

### Python Build

```bash
# Development install (editable)
pip install -e .

# Build wheels
maturin build --release

# Build and install
maturin develop --release

# Run tests
pytest python/tests/

# Or using hatch
hatch test
```

### Unified Build Script

```bash
# Comprehensive build + test + package
./scripts/build.sh

# Quick development build
./build.sh
```

## Version Management

### Current Approach

**Rust:** Manual versioning in `Cargo.toml`
- Version: `2.0.0`
- Updated manually for releases

**Python:** Dynamic versioning via hatch-vcs
- Reads from git tags
- Auto-generates `python/haforu/_version.py`
- Configured in `pyproject.toml`

### Recommended Approach (TODO)

Use **tag-driven versioning** for both:

1. Create git tag: `git tag -a v2.1.0 -m "Release 2.1.0"`
2. Push tag: `git push origin v2.1.0`
3. GitHub Actions:
   - Builds Rust binary
   - Builds Python wheels
   - Creates GitHub Release
   - Publishes to crates.io and PyPI

**Benefits:**
- Single source of truth (git tags)
- Automated releases
- No manual version bumps
- Consistent Rust + Python versions

## Testing Strategy

### Rust Tests

**Location:** Inline in `src/` modules
**Run:** `cargo test`

**Coverage:**
- 36 library tests (unit + integration)
- 13 main tests (CLI commands)
- Tests for: batch, fonts, render, streaming, output

### Python Tests

**Location:** `python/tests/`
**Run:** `pytest` or `hatch test`

**Coverage:**
- 65 tests total
- Tests for: batch, errors, numpy, streaming
- Includes bindings tests and CLI tests

### Integration Tests

**Scripts in `scripts/`:**
- `batch_smoke.sh` - Smoke tests for CLI contract
- `profile-cli.sh` - Performance profiling
- `regression-test.sh` - Performance regression detection
- `test-cli-parity.sh` - Python/Rust CLI parity (20 tests)

### Performance Tests

**Benchmarks:**
- `benches/cli.rs` - Criterion benchmarks (note: has serde version conflicts)
- `scripts/profile-cli.sh` - Hyperfine-based profiling
- `scripts/regression-test.sh` - Threshold-based gates

**Metrics:**
- Startup time: ~6.5ms
- Batch (100 jobs): ~7-8ms
- Streaming (1000 lines): ~10ms
- Metrics render: ~6ms
- PGM render: ~7.5ms

## Code Organization

### Rust Modules

**Principle:** Small, focused modules with clear responsibilities

- `batch.rs` - Data structures only (Job, JobSpec, JobResult)
- `fonts.rs` - Font loading and caching (FontLoader, FontInstance)
- `shaping.rs` - Text shaping (TextShaper, ShapedText)
- `render.rs` - Rasterization (GlyphRasterizer, Image)
- `output.rs` - Image encoding (ImageOutput, PGM/PNG)
- `error.rs` - Error types (Error, Result)
- `security.rs` - Security validation
- `lib.rs` - Public API and orchestration
- `main.rs` - CLI implementation

**Best Practices:**
- ✅ Separation of concerns
- ✅ Clear module boundaries
- ✅ Public API in `lib.rs`
- ✅ Minimal inter-module dependencies
- ✅ Comprehensive documentation

### Python Package

**Structure:**
- `python/haforu/__init__.py` - Public API (process_jobs, StreamingSession)
- `python/haforu/__main__.py` - Fire CLI (HaforuCLI class)
- `python/haforu/_version.py` - Auto-generated version
- `python/tests/` - Test suite

**Best Practices:**
- ✅ Clear public API
- ✅ Separate CLI implementation
- ✅ Type stubs for bindings
- ✅ Comprehensive tests

### PyO3 Bindings

**Location:** `src/python/`

**Pattern:**
- Thin wrappers around Rust API
- Python-friendly types (str, dict, list)
- Error conversion to Python exceptions (where appropriate)
- Iterator protocol for streaming

**Best Practices:**
- ✅ Minimal business logic in bindings
- ✅ Delegate to Rust core
- ✅ Pythonic API surface
- ✅ GIL release for CPU-intensive operations

## File Annotations

### `this_file` Convention

Every source file includes a header comment with its path relative to repository root:

**Rust:**
```rust
// this_file: src/main.rs
```

**Python:**
```python
# this_file: python/haforu/__init__.py
```

**Markdown:**
```markdown
---
this_file: docs/REPOSITORY.md
---
```

**Purpose:**
- Clear file identity
- Easy navigation
- Copy-paste context preservation
- LLM-friendly codebase exploration

## Development Workflow

### Local Development

1. **Clone repository:**
   ```bash
   git clone https://github.com/fontsimi/haforu.git
   cd haforu
   ```

2. **Build and test:**
   ```bash
   # Build everything
   ./scripts/build.sh

   # Run tests
   cargo test
   pytest python/tests/

   # Run smoke tests
   ./scripts/run.sh smoke
   ```

3. **Make changes:**
   - Edit Rust code in `src/`
   - Edit Python code in `python/haforu/`
   - Update tests
   - Update documentation

4. **Verify changes:**
   ```bash
   # Rust tests
   cargo test

   # Python tests
   hatch test

   # CLI parity
   ./scripts/test-cli-parity.sh

   # Performance
   ./scripts/regression-test.sh
   ```

5. **Update documentation:**
   - Update `CHANGELOG.md`
   - Update `WORK.md` with notes
   - Update `TODO.md` if applicable

### Release Workflow (Manual)

1. **Update versions:**
   - Bump version in `Cargo.toml`
   - Create git tag: `git tag -a v2.1.0 -m "Release 2.1.0"`

2. **Build and test:**
   ```bash
   ./scripts/build.sh
   cargo test
   hatch test
   ```

3. **Build artifacts:**
   ```bash
   # Rust binary
   cargo build --release

   # Python wheels
   maturin build --release
   ```

4. **Publish:**
   ```bash
   # Rust crate
   cargo publish

   # Python wheels
   maturin publish
   ```

5. **Tag and push:**
   ```bash
   git push origin v2.1.0
   ```

### Release Workflow (Automated - TODO)

GitHub Actions workflow triggered by `v*` tags:
1. Checkout code
2. Build Rust binary (multi-platform)
3. Build Python wheels (manylinux, macOS, Windows)
4. Run full test suite
5. Create GitHub Release
6. Publish to crates.io
7. Publish to PyPI

## Best Practices Compliance

### Rust Workspace Best Practices

✅ **Applied:**
- Clear module structure
- Comprehensive Cargo.toml
- Feature flags for optional dependencies
- Optimized release profile
- Platform-specific configuration
- Benchmark configuration

❌ **Not Applied (by design):**
- Workspace structure (single crate is appropriate for this project)
- Multiple binary targets (only one CLI needed)

### PyO3 Best Practices

✅ **Applied:**
- Feature flag for Python bindings
- Separate source directory for Python code
- Type conversions at binding boundary
- GIL management
- Python-friendly API

✅ **Excellent:**
- Streaming API with proper iterator protocol
- Error handling with custom exceptions
- Cache management from Python

### Hatch Best Practices

✅ **Applied:**
- VCS-based versioning
- pytest configuration
- ruff configuration
- mypy configuration
- Development dependencies

❌ **Not Applied (low priority):**
- Multiple test environments
- Matrix testing across Python versions

### Maturin Best Practices

✅ **Applied:**
- Correct build backend configuration
- Python source directory specified
- Module name configured
- Feature selection

✅ **Excellent:**
- Universal2 wheels for macOS
- Manylinux wheels for Linux
- Windows wheel support

## Deviations from Canonical Structure

### 1. Single Crate vs Workspace

**Current:** Single Rust crate at repository root
**Canonical:** Workspace with multiple crates

**Justification:**
- Project is cohesive single unit
- No need for sub-crates
- Simpler build and dependency management
- Appropriate for project size

### 2. PyO3 Bindings Location

**Current:** `src/python/` (within Rust crate)
**Alternative:** Separate `bindings/` directory

**Justification:**
- Keeps bindings close to Rust code
- Clear feature flag boundary
- Works well with Maturin
- Standard pattern for PyO3 projects

### 3. Python Package Location

**Current:** `python/` directory at root
**Alternative:** `py-haforu/` or separate repository

**Justification:**
- Maturin convention
- Clear separation from Rust source
- Supports independent Python development
- Recommended by Maturin documentation

## Performance Characteristics

### Build Times

**Rust (Release):**
- Clean build: ~2-3 minutes
- Incremental: ~10-30 seconds

**Python (Wheel):**
- Clean build: ~2-3 minutes (includes Rust)
- Maturin develop: ~30 seconds

**Full CI Build:**
- Estimated: ~10-15 minutes (multi-platform)

### Binary Sizes

**Rust Binary:**
- Release (stripped): ~2.4MB
- Release (with debug): ~15MB

**Python Wheel:**
- macOS (universal2): ~5-6MB
- Linux (manylinux): ~4-5MB
- Windows: ~3-4MB

### Runtime Performance

**CLI Startup:** ~6.5ms
**Throughput:** ~100k jobs/sec (streaming, metrics-only)
**Memory:** <100MB for typical workloads

## Known Issues

1. **Criterion Benchmarks:** serde_core version conflicts (low priority)
2. **Windows Testing:** Limited Windows CI coverage
3. **Documentation:** Some internal APIs undocumented

## Future Improvements

### High Priority
1. ✅ CLI documentation (DONE)
2. ✅ Performance profiling scripts (DONE)
3. ✅ CLI parity testing (DONE)
4. GitHub Actions CI/CD
5. Automated releases

### Medium Priority
1. Windows build testing
2. Cross-compilation support
3. Additional benchmarks
4. Coverage reporting

### Low Priority
1. Workspace structure (if project grows)
2. Separate PyO3 bindings crate
3. Cargo-vcs for Rust versioning
4. Pre-commit hooks

## See Also

- [CLI Usage Guide](./CLI-USAGE.md)
- [Installation Guide](../INSTALL.md)
- [Project Plan](../PLAN.md)
- [TODO List](../TODO.md)
