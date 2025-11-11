---
this_file: external/haforu/TODO.md
---

# TODO.md

Always update @TODO.md and @README.md and @CLAUDE.md accordingly. Then proceed with the work!

## FontSimi Integration Tasks (CRITICAL - Phase 0)

### Core Streaming Features
- [ ] Implement `--streaming` mode flag for persistent process
  - [ ] Keep stdin/stdout open for continuous job processing
  - [ ] Read JSON jobs line-by-line from stdin
  - [ ] Write JSONL results line-by-line to stdout with immediate flush
  - [ ] Maintain font cache across all jobs in session
- [ ] Add streaming output for batch mode
  - [ ] Process jobs in parallel but write results as they complete
  - [ ] Don't wait for all jobs to finish before outputting
  - [ ] Include progress indicators in stderr (optional)
- [ ] Implement job ID preservation
  - [ ] Pass through job IDs from input to output
  - [ ] Handle missing IDs gracefully (generate UUID)

### Variable Font Support Enhancements
- [ ] Enhance variation coordinate handling
  - [ ] Accept variations as dict in job spec
  - [ ] Apply variations to font instance correctly
  - [ ] Cache instantiated variable fonts by coordinate hash
  - [ ] Report applied coordinates in output
- [ ] Add variation bounds validation
  - [ ] Check coordinates against font's axis limits
  - [ ] Clamp or error on out-of-bounds values
  - [ ] Include axis info in error messages

### Memory Management Features
- [ ] Add `--max-memory` flag (MB)
  - [ ] Track current memory usage via MemoryTracker
  - [ ] Clear LRU font cache when approaching limit
  - [ ] Report memory stats in JSONL output (optional)
- [ ] Optimize font caching for large batches
  - [ ] Increase default cache size to 512 fonts
  - [ ] Add cache hit/miss statistics to stderr
  - [ ] Implement font instance cache for variations

### Output Format Enhancements
- [ ] Enhance PGM output support
  - [ ] Ensure proper P5 binary format by default
  - [ ] Add P2 ASCII format option for debugging
  - [ ] Include actual rendered bbox in metadata
- [ ] Add base64 encoding for images
  - [ ] Support `output_format: "base64"` in job spec
  - [ ] Encode PGM data as base64 string
  - [ ] Include in JSONL under `rendering.data`
- [ ] Add glyph metrics to output
  - [ ] Include advance width, bearings
  - [ ] Add bounding box coordinates
  - [ ] Optional detailed shaping info

## FontSimi Integration Tasks (Phase 1)

### Performance Optimizations
- [ ] Optimize for single-glyph rendering (FontSimi's daidot)
  - [ ] Fast path for single character text
  - [ ] Skip unnecessary shaping for single glyphs
  - [ ] Optimize memory allocation for small renders
- [ ] Add batch size optimization
  - [ ] Auto-tune parallelization based on job count
  - [ ] Adaptive thread pool sizing
  - [ ] Memory-aware batch splitting

### Error Handling & Robustness
- [ ] Improve error recovery for batch jobs
  - [ ] Continue processing after individual job failures
  - [ ] Include error details in JSONL output
  - [ ] Summary statistics at end (success/failure counts)
- [ ] Add timeout handling
  - [ ] Per-job timeout (default 5s)
  - [ ] Skip stuck jobs and mark as timeout
  - [ ] Global timeout for entire batch

### Testing & Validation
- [ ] Create FontSimi integration tests
  - [ ] Test batch of 1000+ single-glyph renders
  - [ ] Verify streaming mode with continuous jobs
  - [ ] Test variable font coordinate application
  - [ ] Memory limit compliance tests
- [ ] Add benchmarks for FontSimi workloads
  - [ ] Benchmark: 5000 single-glyph renders
  - [ ] Compare with/without font caching
  - [ ] Memory usage over time graphs

## Original Tasks (Lower Priority)

### Build/Release Tasks
- [ ] Configure GitHub Secrets: `CRATES_IO_TOKEN`, `PYPI_TOKEN`
- [ ] Verify `./build.sh --release` runs on dev machine (cargo + maturin installed)
- [ ] Create test tag `v0.1.1-rc1` (do not publish) and confirm CI builds artifacts
- [ ] Create release tag `v0.1.1` and confirm:
  - [ ] GitHub Release created with Rust binaries and wheels
  - [ ] crates.io publish succeeds
  - [ ] PyPI publish succeeds
- [ ] Document outcome in `WORK.md` and `CHANGELOG.md`

## Completed Tasks ✓

### Initial Session
- [x] @issues/102.md - Fixed yanked read-fonts dependency
- [x] Fix Python bindings - export json_parser module for full API access
- [x] Clean up clippy warnings (unused fields, redundant closures, etc.)
- [x] Add error recovery tests for malformed/corrupt font files
- [x] Add performance benchmarks for critical paths
- [x] Improve CLI warnings in tests
- [x] Add memory usage tracking and optimization

## FontSimi Phase 2: Storage Backend Integration

### Packfile Storage for Pre-rendered Images
- [ ] Add `--storage-dir` flag for packfile output
  - [ ] Create sharded packfiles (5GB each)
  - [ ] Generate index for fast lookups
  - [ ] Support concurrent reads from multiple processes
- [ ] Implement storage ID references
  - [ ] Return storage ID instead of image data
  - [ ] Format: `shard_XXX/img_YYYYY`
  - [ ] Include in JSONL as alternative to base64
- [ ] Add query interface
  - [ ] `haforu query --storage-id shard_001/img_12345`
  - [ ] Return image data or metadata
  - [ ] Support batch queries

### Pre-rendering Support
- [ ] Add `--prerender` mode
  - [ ] Accept list of fonts and coordinate grids
  - [ ] Generate all combinations systematically
  - [ ] Store directly to packfiles
  - [ ] Progress tracking and resumability
- [ ] Optimize for Sobol grid points
  - [ ] Special handling for power-of-2 sample counts
  - [ ] Efficient iteration over design space
  - [ ] Parallel processing of grid points

## Next Immediate Steps (Priority Order)

1. **Implement --streaming mode** - Critical for optimization phase
2. **Add base64 output encoding** - Required for Python integration
3. **Enhance variable font support** - Essential for FontSimi matching
4. **Optimize single-glyph rendering** - Major performance win
5. **Add memory management** - Prevent OOM on large batches

## Key Success Metrics for FontSimi

- Reduce rendering calls: 5.5M → 1 batch call
- Memory usage: 86GB → <2GB
- Analysis time: 5 hours → 3 minutes
- Deep optimization: 30s → 0.6s per font pair
- Zero crashes on 250×85 font batches

## Completed Quality Improvements ✓

### Initial Session
- [x] Create basic font loading module with error handling and validation
  - Implemented font file loading using read-fonts with proper error handling
  - Added validation for font file format (TTF/OTF)
  - Created unit tests for valid and invalid font files
  - Added logging for debugging font loading issues

- [x] Implement JSON jobs specification parser with validation
  - Defined structs for jobs-spec format using serde
  - Implemented robust JSON parsing with detailed error messages
  - Added validation for required fields and value ranges
  - Created comprehensive tests for various JSON input scenarios

- [x] Set up project structure with proper module organization
  - Created lib.rs for library functionality separate from CLI
  - Organized modules: font_loader, json_parser, storage, shaping
  - Added documentation comments for all public APIs
  - Configured proper error types using thiserror

### Second Session
- [x] Add integration tests with real font files
  - Created tests using fonts from 03fonts/ directory
  - Tested font loading with actual TTF/OTF files
  - Verified font metadata extraction
  - Tested variable font axis handling

- [x] Implement proper logging configuration
  - Added configurable log levels via CLI flags (-l, -q)
  - Implemented structured logging with colors
  - Added timestamp support with chrono
  - Created debug vs release logging profiles

- [x] Add input sanitization and bounds checking
  - Validated file paths for directory traversal attacks
  - Added size limits for JSON input (10MB max)
  - Implemented text length validation (10K chars)
  - Added memory usage monitoring and limits

### Current Session
- [x] Create comprehensive README.md specification
- [x] Analyze fontgrep for efficient font processing patterns

## Phase 1: Foundation with Fontgrep Patterns

### Font Management (Priority: Critical)
- [x] Create font caching system
  - [ ] Support variable font instance caching

### Parallel Processing Infrastructure (Priority: Critical)
- [ ] Set up parallel directory traversal (from fontgrep)
  - [ ] Add `jwalk` dependency
  - [ ] Implement parallel font file discovery
  - [ ] Add file type filtering (TTF, OTF, TTC, OTC, WOFF, WOFF2)
  - [ ] Configure thread pool size based on CPU count

- [x] Implement job parallelization with Rayon
  - [ ] Add backpressure mechanism

### Text Shaping Integration (Priority: Critical)
- [x] Integrate HarfRust for text shaping

- [x] Implement shaping output format
  - [ ] Include timing metrics

## Phase 2: Unified haforu CLI Tool (HarfBuzz-Compatible)


### Single Unified haforu CLI (Priority: Critical)
**NOTE: ONE tool combining hb-shape and hb-view functionality - see [PLAN.md](PLAN.md)**
- [ ] Create unified CLI structure
  - [ ] Add `clap` dependency with derive macros
  - [ ] Implement subcommands: `shape`, `view`, `process` (batch)
  - [ ] Parse command-line arguments matching hb-shape and hb-view
  - [ ] Add --help with detailed descriptions for each command

- [ ] Implement `haforu shape` command (hb-shape compatibility)
  - [ ] Support all hb-shape flags (--direction, --language, --script)
  - [ ] Add --features flag with Python-esque syntax
  - [ ] Implement --variations for variable fonts
  - [ ] Add --output-format (text/json)

- [ ] Implement `haforu view` command (hb-view compatibility)
  - [ ] Support rendering flags (--font-size, --margin, --background)
  - [ ] Add bitmap output formats
    - [ ] PNG with full alpha channel support
    - [ ] PBM (Portable Bitmap) - 1-bit monochrome
      - [ ] ASCII format for human-readable output
      - [ ] Binary format for compact storage
    - [ ] PGM (Portable Graymap) - 8/16-bit grayscale
      - [ ] ASCII format for human-readable output
      - [ ] Binary format for compact storage
    - [ ] Direct stdout output support for pipe-friendly operation
  - [ ] Add vector output formats (svg/pdf)
  - [ ] Implement --output-file for rendered images
  - [ ] Support advanced rendering options
    - [ ] DPI setting (--dpi)
    - [ ] Antialiasing control (--antialiasing)
    - [ ] Hinting modes (--hinting=none|slight|medium|full)
    - [ ] Subpixel rendering (--subpixel=none|rgb|bgr|vrgb|vbgr)
    - [ ] Threshold for monochrome conversion (--threshold)
    - [ ] Dithering options (--dither)
    - [ ] Bit depth control (--bit-depth=1|8|16)
  - [ ] Support view-specific options (--show-extents)

- [ ] Implement `haforu process` command (batch JSON mode)
  - [ ] Read JSON jobs specification from stdin
  - [ ] Output JSONL results to stdout
  - [ ] Include storage backend references in output
  - [ ] Support streaming for large batches

### Error Handling Enhancement (Priority: High)
- [ ] Implement graceful error recovery (from fontgrep)
  - [ ] Continue processing on single font failure
  - [ ] Log errors with context (file path, error type)
  - [ ] Add --fail-fast flag for strict mode
  - [ ] Collect and report summary statistics

## Phase 3: Storage Backend Implementation

### Sharded Packfile System (Priority: High)
- [ ] Create packfile writer
  - [ ] Add `zstd` and `lz4_flex` dependencies
  - [ ] Implement append-only shard files
  - [ ] Create index entries (20 bytes: offset, len, w, h, crc)
  - [ ] Write footer with metadata
  - [ ] Support configurable compression levels

- [ ] Create packfile reader
  - [ ] Implement shard file memory mapping
  - [ ] Binary search in index for O(1) lookup
  - [ ] Decompress images on demand
  - [ ] Verify checksums in debug mode

- [ ] Implement shard management
  - [ ] Calculate shard ID from image ID
  - [ ] Create new shards when size limit reached (2-10 GiB)
  - [ ] Implement LRU cache of open shard mmaps
  - [ ] Add shard compaction utility

### Storage Query Tools (Priority: Medium)
- [ ] Create haforu-query CLI tool
  - [ ] List stored results with filters
  - [ ] Verify shard integrity
  - [ ] Export images by reference
  - [ ] Show storage statistics

## Phase 4: Rendering Integration

### Rasterization Pipeline (Priority: Critical)
- [x] Implement outline extraction with skrifa
  - [x] Add `skrifa` dependency for glyph outline extraction
  - [x] Use `skrifa::outline::DrawSettings` for zero-copy access
  - [x] Handle TrueType/CFF/CFF2 glyph formats
  - [ ] Cache frequently-used glyph outlines at specific sizes/variations

- [x] Integrate zeno for CPU rasterization (Primary Path - CHOSEN OVER tiny-skia)
  - [x] Add `zeno` dependency (minimal, no deps, focused on rasterization)
  - [x] Implement 256x anti-aliased rendering (8-bit alpha)
  - [x] Create ZenoPen adapter implementing skrifa's OutlinePen trait
  - [x] Parallelize rasterization with Rayon across glyphs/texts
  - [ ] Compile with `target-cpu=native` for optimal performance

- [ ] Implement glyph atlas caching
  - [ ] Create shared glyph atlas for pre-rasterized common glyphs
  - [ ] Store in memory-mapped packfiles with zstd compression
  - [ ] Implement LRU cache for hot glyphs
  - [ ] Use format: `{glyph_id}_{font_hash}_{size}_{variation_hash}` → bitmap

- [ ] Add vello GPU rendering (Alternative Path for Batch)
  - [ ] Add `vello` and `wgpu` dependencies
  - [ ] Create rendering module for batch GPU processing
  - [ ] Build scene graph from shaped glyphs
  - [ ] Use for large batches (10K+ texts) rendered together
  - [ ] Implement fallback to CPU when GPU unavailable

- [ ] Add output format support
  - [ ] Bitmap formats
    - [ ] PNG output with configurable quality and alpha channel
    - [ ] PBM (Portable Bitmap) writer
      - [ ] ASCII P1 format for text output
      - [ ] Binary P4 format for compact storage
      - [ ] Monochrome thresholding with configurable cutoff
      - [ ] Optional dithering (Floyd-Steinberg, ordered)
    - [ ] PGM (Portable Graymap) writer
      - [ ] ASCII P2 format for text output
      - [ ] Binary P5 format for compact storage
      - [ ] 8-bit and 16-bit depth support
      - [ ] Direct alpha channel to grayscale conversion
  - [ ] Vector formats
    - [ ] SVG vector output with path data
    - [ ] PDF document output with embedded fonts
  - [ ] Stdout streaming
    - [ ] Direct PBM/PGM output to stdout for piping
    - [ ] Buffered writing for performance
    - [ ] Format auto-detection from output destination
  - [ ] Direct-to-storage rendering with compression

### Unified haforu CLI Rendering Support (Priority: Medium)
**NOTE: Rendering is integrated into the unified `haforu` tool via `view` command**
- [ ] Enhance `haforu view` command features
  - [ ] Full hb-view CLI compatibility
  - [ ] Advanced rendering flags (DPI, subpixel, hinting)
  - [ ] Batch rendering via `haforu process` JSON mode
  - [ ] Direct storage backend integration for caching

## Phase 5: Python Bindings

### PyO3 Integration (Priority: Low)
- [ ] Create Python bindings package
  - [ ] Add `pyo3` and `maturin` dependencies
  - [ ] Define Python-facing API
  - [ ] Implement Engine class
  - [ ] Add batch processing methods

- [ ] Python packaging
  - [ ] Create pyproject.toml
  - [ ] Write Python type stubs
  - [ ] Add Python tests
  - [ ] Create wheel distribution

## Phase 6: Performance Optimization

### Profiling and Benchmarks (Priority: Low)
- [ ] Create comprehensive benchmarks
  - [ ] Font loading performance
  - [ ] Shaping throughput
  - [ ] Rendering speed
  - [ ] Storage I/O rates
  - [ ] Parallel scaling efficiency

- [ ] Profile and optimize hot paths
  - [ ] Use `perf` and `flamegraph`
  - [ ] Optimize memory allocations
  - [ ] Tune compression parameters
  - [ ] Optimize GPU shader code

### Advanced Features (Priority: Low)
- [ ] Add dictionary-based compression for similar images
- [ ] Implement predictive prefetching
- [ ] Add distributed processing support
- [ ] Create web API server mode

## Testing Requirements

### Unit Tests (Priority: Critical)
- [ ] Font loading edge cases
- [ ] JSON parsing validation

### Integration Tests (Priority: High)
- [ ] End-to-end batch processing
- [ ] CLI compatibility tests
- [ ] Parallel processing tests

### Performance Tests (Priority: Medium)
- [ ] Throughput benchmarks
- [ ] Memory usage tests
- [ ] Scaling efficiency tests
- [ ] Storage performance tests

## Documentation

### User Documentation (Priority: High)
- [ ] Write comprehensive CLI documentation
- [ ] Create Python API documentation
- [ ] Write troubleshooting guide

### Developer Documentation (Priority: Medium)
- [ ] Document architecture decisions
- [ ] Create contribution guide
- [ ] Add code style guide
- [ ] Write testing guide

## Configuration and Deployment

### Configuration System (Priority: Medium)
- [ ] Implement TOML configuration file support
- [ ] Add environment variable overrides
- [ ] Create default configuration
- [ ] Add configuration validation

### Packaging (Priority: Low)
- [ ] Create Debian/RPM packages
- [ ] Add Docker container
- [ ] Create Homebrew formula
- [ ] Add CI/CD pipelines

---

## Next Immediate Steps (Priority Order)

1. **Implement memory-mapped font loading** using patterns from fontgrep
2. **Integrate HarfRust** for text shaping
3. **Add skrifa + zeno rasterization pipeline** as primary rendering path
4. **Create haforu-shape CLI** with basic hb-shape compatibility
5. **Add parallel processing** with Rayon
6. **Implement glyph atlas caching** with LRU and memory-mapped storage
7. **Implement sharded packfile storage** backend

## Key Insights from fontgrep Analysis

### Patterns to Adopt
- **Zero-copy parsing**: Use memmap2 + borrowed references throughout
- **Parallel from the start**: Use jwalk for directory traversal
- **Fast-fail filtering**: Order operations from cheapest to most expensive
- **Graceful degradation**: Continue on errors, don't crash
- **Progressive output**: Stream results as they're found

### Improvements Over fontgrep
- **Add persistent caching**: fontgrep has none, we need it for 10M images
- **Batch processing**: fontgrep is single-query, we need JSON batch mode
- **Variable font instances**: fontgrep detects, we need to apply variations
- **Rendering pipeline**: fontgrep analyzes, we need to rasterize
- **Storage backend**: fontgrep streams output, we need persistent storage

## Rasterization Strategy (from issues/101.md)

### Primary Pipeline: skrifa + zeno (CPU)
- **Outline Extraction**: Use `skrifa::outline::DrawSettings` for zero-copy glyph access
- **CPU Rasterization**: Use `zeno` for pure Rust, high-performance 2D path rasterization
  - **Why zeno over tiny-skia**: Minimal dependencies (nearly zero), focused scope, smaller binary
  - **Why zeno over swash**: More lightweight, better for our specific glyph rasterization needs
- **Parallelization**: Rasterize different glyphs/texts on different threads via Rayon
- **Memory Efficiency**: 256x anti-aliased rendering with 8-bit alpha, minimal footprint
- **No GPU Dependencies**: Predictable performance, fast enough for glyph atlas generation

### Alternative: vello (GPU) for Batch Processing
- **Use Case**: Large batches rendered together (e.g., 10,000 texts → single GPU pass)
- **Trade-off**: Higher setup cost but scales better for massive parallelism
- **Best For**: Rendering many instances to a shared atlas

### Performance Targets
- **Simple glyphs (ASCII)**: ~50,000-100,000 glyphs/sec single-threaded
- **Complex glyphs (CJK, Arabic)**: ~10,000-30,000 glyphs/sec single-threaded
- **With 64-core parallelism**: 500K-6M glyphs/sec theoretical peak
- **Memory per instance**: ~200 bytes metadata, ~20 bytes/glyph shaped, ~1-16 KB/glyph rasterized

### Critical Optimizations
- **Avoid Double-Parsing**: Use skrifa's `FontRef` directly from `read-fonts`
- **SIMD in Rasterization**: Compile with `target-cpu=native` for zeno's SIMD
- **Batch Transformations**: Apply affine transforms before rasterization
- **Early Culling**: Check glyph bounding boxes before rasterization
- **Subpixel Positioning**: Quantize to 1/4 or 1/8 pixel to reduce cache misses

---

## References

- fontgrep implementation: `01code/fontgrep/`
- HarfRust shaping: `01code/harfrust/`
- Fontations stack: `01code/fontations/`
- Storage design: `400.md`
- HarfBuzz CLI reference: `hb-shape.txt`, `hb-view.txt`
