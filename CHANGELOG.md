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