# TODO.md

Always update @TODO.md and @README.md and @CLAUDE.md accordingly. Then proceed with the work! 

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
- [ ] Implement memory-mapped font loading (from fontgrep)
  - [ ] Add `memmap2` dependency
  - [ ] Create `FileInfo` struct with mmapped data
  - [ ] Implement zero-copy `FontRef` creation
  - [ ] Add support for TTC/OTC collections
  - [ ] Validate fonts upfront before processing

- [ ] Create font caching system
  - [ ] Implement LRU cache for loaded `FontRef` objects
  - [ ] Make cache size configurable (default: 256 fonts)
  - [ ] Add metrics for cache hit/miss rates
  - [ ] Support variable font instance caching

### Parallel Processing Infrastructure (Priority: Critical)
- [ ] Set up parallel directory traversal (from fontgrep)
  - [ ] Add `jwalk` dependency
  - [ ] Implement parallel font file discovery
  - [ ] Add file type filtering (TTF, OTF, TTC, OTC, WOFF, WOFF2)
  - [ ] Configure thread pool size based on CPU count

- [ ] Implement job parallelization with Rayon
  - [ ] Add `rayon` and `num_cpus` dependencies
  - [ ] Create thread pool with configurable size
  - [ ] Implement work-stealing job queue
  - [ ] Add backpressure mechanism

### Text Shaping Integration (Priority: Critical)
- [ ] Integrate HarfRust for text shaping
  - [ ] Add `harfrust` dependency (or build from 01code/harfrust)
  - [ ] Create shaping module with HarfRust integration
  - [ ] Map font data from read-fonts to HarfRust format
  - [ ] Implement shaping configuration (direction, script, language)
  - [ ] Add OpenType feature support

- [ ] Implement shaping output format
  - [ ] Create structs for glyph info (ID, cluster, advance)
  - [ ] Match hb-shape output format
  - [ ] Add JSON serialization for shaped results
  - [ ] Include timing metrics

## Phase 2: Enhanced CLI with HarfBuzz Compatibility

### haforu-shape CLI (Priority: High)
- [ ] Create basic CLI structure
  - [ ] Add `clap` dependency with derive macros
  - [ ] Parse command-line arguments matching hb-shape
  - [ ] Support font file and text arguments
  - [ ] Add --help with detailed descriptions

- [ ] Implement hb-shape compatibility
  - [ ] Support all hb-shape flags (--direction, --language, --script)
  - [ ] Add --features flag with Python-esque syntax
  - [ ] Implement --variations for variable fonts
  - [ ] Add --output-format (text/json)

- [ ] Add batch processing mode
  - [ ] Add --batch flag for JSON input mode
  - [ ] Read JSON jobs from stdin
  - [ ] Output JSONL results to stdout
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
- [ ] Implement outline extraction with skrifa
  - [ ] Add `skrifa` dependency for glyph outline extraction
  - [ ] Use `skrifa::outline::DrawSettings` for zero-copy access
  - [ ] Handle TrueType/CFF/CFF2 glyph formats
  - [ ] Cache frequently-used glyph outlines at specific sizes/variations

- [ ] Integrate zeno for CPU rasterization (Primary Path - CHOSEN OVER tiny-skia)
  - [ ] Add `zeno` dependency (minimal, no deps, focused on rasterization)
  - [ ] Implement 256x anti-aliased rendering (8-bit alpha)
  - [ ] Create ZenoPen adapter implementing skrifa's OutlinePen trait
  - [ ] Parallelize rasterization with Rayon across glyphs/texts
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
  - [ ] PNG output with configurable quality
  - [ ] SVG vector output
  - [ ] PDF document output
  - [ ] Direct-to-storage rendering with compression

### haforu-view CLI (Priority: Medium)
- [ ] Create haforu-view tool
  - [ ] Match hb-view CLI interface
  - [ ] Add rendering-specific flags
  - [ ] Support batch rendering mode
  - [ ] Add storage backend integration

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
- [ ] Shaping correctness tests
- [ ] Storage integrity tests

### Integration Tests (Priority: High)
- [ ] End-to-end batch processing
- [ ] CLI compatibility tests
- [ ] Storage round-trip tests
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
- [ ] Add usage examples
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