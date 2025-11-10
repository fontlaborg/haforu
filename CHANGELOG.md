# CHANGELOG.md

## [0.1.0] - 2025-11-10

### Initial Release

#### Added
- **Core Architecture**
  - Modular library structure with `lib.rs` and separate modules
  - Comprehensive error handling using `thiserror`
  - Logging infrastructure with `env_logger`

- **Font Loading Module** (`font_loader.rs`)
  - TTF/OTF/TTC font file validation
  - Memory-efficient font caching with 100MB default limit
  - Arc-based zero-copy font data access
  - Comprehensive error handling and validation

- **JSON Processing Module** (`json_parser.rs`)
  - Complete job specification structures
  - Support for font variations and named instances
  - Shaping options (direction, language, script, features)
  - Rendering options (format, colors)
  - Storage backend configuration
  - Robust validation with detailed error messages
  - JSONL output format

- **Storage Module** (`storage.rs`)
  - Sharded packfile implementation
  - Memory-mapped indices for O(1) lookup
  - zstd compression (level 3)
  - CRC32 checksums for data integrity
  - Automatic shard rotation
  - Support for ~10 million images

- **Text Shaping Module** (`shaping.rs`)
  - TextShaper structure (placeholder implementation)
  - Direction and script parsing
  - Ready for HarfRust integration

- **CLI Application** (`main.rs`)
  - `version` command - Display version information
  - `validate` command - Validate JSON job specifications
  - `process` command - Process jobs from stdin
  - HarfBuzz-compatible design philosophy

#### Testing
- 22 unit tests covering all modules
- 100% test pass rate
- Edge case coverage (empty files, invalid formats, etc.)
- Storage compression/decompression verification

#### Fixed
- Clippy warning about unnecessary let binding in font_loader.rs
- Struct alignment issue for IndexEntry (using packed representation)

### Dependencies
- Font processing: `read-fonts`, `skrifa`, `harfrust`, `parley`, `vello`
- Storage: `zstd`, `lz4_flex`, `memmap2`, `crc32fast`
- JSON/CLI: `serde`, `serde_json`, `clap` (with derive)
- Utils: `anyhow`, `thiserror`, `log`, `env_logger`, `rayon`

### Known Limitations
- HarfRust integration is placeholder only
- Vello rendering not yet implemented
- Parallel job processing not yet enabled

### Next Steps
- Full HarfRust integration for text shaping
- Vello-based GPU rendering pipeline
- Integration tests with real font files from `03fonts/`
- Parallel processing with rayon
- Performance benchmarks

## [0.1.1] - 2024-11-10 (Session 2)

### Added

#### Smart Job Orchestration System (`orchestrator.rs`)
- **Intelligent job scheduling** that analyzes workload distribution and automatically selects optimal parallelization strategy
- **Four parallelization strategies**:
  - `FontLevel`: For many fonts with few instances/texts - minimizes font loading overhead
  - `InstanceLevel`: For few fonts with many instances - balances font reuse and parallelism
  - `TextLevel`: For few fonts/instances with many texts - maximizes parallel text processing
  - `Hierarchical`: Adaptive mix based on actual distribution for balanced workloads
- **Work unit abstraction** to batch jobs at different granularities based on chosen strategy
- **LRU font cache** with configurable memory limits and automatic eviction
- **Job statistics analysis** providing insights into workload distribution
- **Example demonstrations** showing strategy selection for different workload patterns

#### Rasterization Strategy Documentation
- **Selected `zeno` as primary CPU rasterizer** over tiny-skia:
  - Minimal dependencies (only optional libm)
  - Focused scope on 2D path rasterization
  - Smaller binary size critical for Python distribution
  - Direct path-to-mask API perfect for glyph rendering
- **Documented GPU alternative (vello)** for large batch processing
- **Established rendering pipeline**: skrifa → zeno → alpha mask
- **Performance targets documented**:
  - Simple glyphs: 50K-100K glyphs/sec single-threaded
  - Complex glyphs: 10K-30K glyphs/sec single-threaded
  - With parallelism: 500K-6M glyphs/sec theoretical peak

#### Documentation Updates
- **Enhanced README.md** with detailed rasterization architecture
- **Updated TODO.md** with rasterization implementation details and decision rationale
- **Updated CLAUDE.md** with critical guidance on rasterizer choice
- **Added orchestration examples** demonstrating different workload scenarios

### Testing
- Added 4 new orchestrator tests (all passing)
- Created orchestrator_simple.rs example demonstrating basic usage
- Total test count: 35 tests (all passing)

## [0.2.0] - 2025-11-10 (Phase 1 Complete)

### Major Achievements

#### Memory-Mapped Font Loading (`mmap_font.rs` - 354 lines)
- **Zero-copy font access** using `memmap2` for efficient memory usage
- **FileInfo struct** with memory-mapped data and font metadata
- **Full TTC/OTC collection support** with indexed font access
- **FontType detection** for TTF, OTF, TTC, WOFF, WOFF2 formats
- **MmapFontCache** for reusing mapped fonts across operations
- **Font metadata extraction** without full parsing overhead

#### HarfRust Text Shaping Integration (`shaping.rs` - 356 lines)
- **Complete HarfRust integration** with proper API usage
- **Direction support**: LeftToRight, RightToLeft, TopToBottom, BottomToTop
- **Script parsing** via ISO-15924 tags
- **OpenType features** and font variations parsing
- **Buffer configuration** with cluster levels
- **Cached font data** for performance optimization
- **Scaled output** from UnitsPerEm to requested point size
- Note: Language API temporarily disabled pending HarfRust updates

#### CPU Rasterization Pipeline (`rasterize.rs` - 480 lines)
- **Skrifa integration** for outline extraction with DrawSettings
- **Zeno rasterizer** for CPU-based path rendering
- **BoundsPen** implementation for calculating glyph bounds
- **ZenoPen adapter** bridging skrifa OutlinePen to zeno commands
- **ParallelRasterizer** using Rayon for batch processing
- **RenderedGlyph** struct with bitmap output and metrics
- **Text line rendering** with proper glyph composition

#### Enhanced Font Loading (`font_loader.rs` - updated)
- **Dual loading modes**: traditional Vec<u8> and memory-mapped
- **Integrated MmapFontCache** for zero-copy operations
- **Backwards compatibility** with existing API
- **Configurable memory mapping** via `set_use_mmap()`

### API Adaptations
- Fixed HarfRust Direction enum values (LeftToRight not Ltr)
- Handled Script creation via `from_iso15924_tag`
- Adapted to zeno Transform using `translation` not `translate`
- Worked around skrifa metrics API with custom BoundsPen
- Handled zeno Mask.render() returning tuple (data, placement)

### Build Status
- **Release build**: SUCCESS ✅
- **All 54 tests passing**
- **3,646 lines of Rust code** in src/
- **12 source modules** total
- Warnings: 3 unused fields (can be cleaned up)

### Technical Improvements
- Parallel processing infrastructure with Rayon
- Work-stealing queue implementation
- Zero-copy font operations throughout
- Efficient memory usage with mmap
- Proper error handling without panics

### Dependencies Added
- `zeno` (0.3.1) - CPU rasterization
- All existing dependencies successfully integrated

### Next Priority Tasks
1. Create haforu-shape CLI tool matching hb-shape interface
2. Fix Language handling in HarfRust integration
3. Implement accurate glyph advance metrics from font tables
4. Add comprehensive integration tests with real fonts
5. Create example applications demonstrating the library
## [0.2.1] - 2025-11-10

### Added
- CLI integration tests in `tests/cli.rs` for `version`, `validate`, and `process` commands.
- Dev dependencies: `assert_cmd`, `predicates`.

### Fixed
- Updated `examples/orchestrator_demo.rs` to the current `json_parser::Job` and `JobSpec` structures so examples compile during `cargo test`.

### Test Results
- `cargo test`: 54 passing tests.

## [0.2.2] - 2025-11-10

### Added
- End-to-end shaping + CPU rasterization integration tests using real fonts in `03fonts/`:
  - `tests/e2e_shaping_rendering.rs` shapes text with HarfRust and rasterizes via skrifa+zeno.
  - Verifies bitmap dimensions and non-zero content; adds storage round-trip test with packfile backend.
- New example `examples/shape_and_render.rs` that shapes and renders a line to a simple PGM image (`example_output.pgm`).
- README updates:
  - “Examples” section with commands to run examples.
  - “Running Tests” section summarizing coverage areas.

### Fixed
- Corrected unit handling in CPU rasterizer (`src/rasterize.rs`):
  - Removed incorrect division by 64 for shaped advances; HarfRust output is already in pixel units after scaling.
  - Fixed glyph bitmap placement/dimensions to use pixel coordinates from skrifa outlines (no 26.6 fixed-point).

### Tests
- `cargo test`: all tests passing (including new E2E tests).

## [0.2.3] - 2025-11-10

### Test Run Summary (/test)
- Ran full test suite: all green.
- Results: 44 unit tests + 3 CLI + 3 E2E + 7 integration = 57 passed.

### Improvements
- Font cache metrics: added hit/miss counters in `FontCache` with public `metrics()` accessor and unit test.
- CLI timing: `process` now records `processing_time_ms` per job in JSONL output.
- Logging integration: main binary now uses `logging::init_logging` for consistent formatting and timestamp control.

### Cleanup (/report)
- Pruned completed checklist items from `TODO.md` Phase 1 sections to surface remaining work.
