# Haforu - High-Performance Font Shaping and Rendering System

**⚠️ ARCHITECTURE NOTE**: Haforu is designed as a **SINGLE unified CLI tool** that combines the functionality of HarfBuzz's `hb-shape` and `hb-view` tools, enhanced with JSON batch processing capabilities. Unlike HarfBuzz which provides separate executables, haforu uses **subcommands** (`haforu shape`, `haforu view`, `haforu process`) within one executable. See [PLAN.md](PLAN.md) for detailed architecture specification.

## Executive Summary

Haforu is a Rust-based font processing system designed for extreme performance and scalability. It provides a library, CLI tool, and Python bindings that enhance HarfBuzz-like functionality with batch processing capabilities, supporting the shaping and rendering of millions of text/font combinations efficiently.

### Key Capabilities
- **Batch Processing**: Process 10,000 texts × 1,000 font instances in a single JSON job
- **Variable Font Support**: Full support for variable font axes and instances
- **High Performance**: Zero-copy parsing, parallel processing, GPU-accelerated rendering
- **Smart Storage**: Sharded packfile system for storing ~10 million rendered images
- **Compatible Interface**: CLI emulates `hb-shape` and `hb-view` with enhanced features

## Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                    JSON Jobs Input                       │
│         (stdin: multiple fonts, texts, sizes)           │
└────────────────┬─────────────────────────────────────────┘
                 │
┌────────────────▼─────────────────────────────────────────┐
│                  Haforu Core Library                     │
│  ┌─────────────────────────────────────────────────┐   │
│  │ Font Manager (LRU Cache, Memory-Mapped Access)  │   │
│  └─────────────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────────────┐   │
│  │ Shaping Engine (HarfRust + Parallel Processing) │   │
│  └─────────────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────────────┐   │
│  │ Rendering Engine (Vello GPU-Accelerated)        │   │
│  └─────────────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────────────┐   │
│  │ Storage Backend (Sharded Packfiles + Index)     │   │
│  └─────────────────────────────────────────────────┘   │
└────────────────┬─────────────────────────────────────────┘
                 │
┌────────────────▼─────────────────────────────────────────┐
│                    JSONL Output                          │
│      (stdout: results with storage references)          │
└──────────────────────────────────────────────────────────┘
```

## Core Components

### 1. Font Management System

#### Font Loading Strategy (from fontgrep patterns)
- **Memory-mapped access** using `memmap2` for zero-copy font parsing
- **LRU cache** of loaded `FontRef` objects (configurable size, default: 256 fonts)
- **Parallel directory traversal** using `jwalk` for font discovery
- **Support for**: TTF, OTF, TTC (TrueType Collections), OTC, WOFF, WOFF2

#### Variable Font Handling
- **Axis application**: Apply variation settings (e.g., "wght=500,wdth=125")
- **Named instances**: Support for predefined instances
- **Instance caching**: Cache instantiated variations to avoid redundant computations

### 2. Text Shaping Engine

#### HarfRust Integration
- **Core library**: Uses HarfRust (Rust port of HarfBuzz) for text shaping
- **Zero dependencies**: No FreeType, ICU, or external C libraries
- **Font backend**: Uses `read-fonts` for parsing, compatible with skrifa
- **Performance**: ~25% slower than HarfBuzz C++, but fully safe and parallelizable

#### Shaping Features
- **Direction support**: LTR, RTL, TTB, BTT
- **Script detection**: Automatic or explicit via ISO-15924 tags
- **Language tagging**: BCP 47 language tags
- **OpenType features**: Full feature support (kern, liga, calt, etc.)
- **Cluster levels**: 0-3 for different cluster merging strategies
- **Context support**: Text before/after for proper shaping

### 3. Rendering System

#### Primary Path: skrifa + zeno (CPU Rasterization)
- **Outline Extraction**: `skrifa::outline::DrawSettings` for zero-copy glyph access
- **CPU Rasterizer**: `zeno` - pure Rust, minimal dependencies, focused 2D path rasterization
  - Chosen over tiny-skia: lighter weight, no unnecessary features, smaller binary
- **Antialiasing**: 256x anti-aliased rendering with 8-bit alpha channel
- **Parallelization**: Thread-per-glyph using Rayon work-stealing queue
- **Memory Efficiency**: Minimal footprint (~0 dependencies), no GPU requirements
- **Performance**: ~50K-100K simple glyphs/sec single-threaded

#### Alternative Path: vello (GPU Batch Processing)
- **Use Case**: Large batches (10K+ texts) rendered together
- **GPU Acceleration**: wgpu compute shaders for massive parallelism
- **Scene Complexity**: Handles complex 2D scenes interactively
- **Trade-offs**: Higher setup cost, better throughput for batch operations

#### Output Format Support
- **Bitmap Formats**:
  - **PBM**: Portable Bitmap (1-bit monochrome) - ASCII or binary
  - **PGM**: Portable Graymap (8/16-bit grayscale) - ASCII or binary
  - **PNG**: Portable Network Graphics with full alpha channel
- **Vector Formats**:
  - **SVG**: Scalable Vector Graphics
  - **PDF**: Portable Document Format
- **Stdout Support**: Direct output to stdout for PBM/PGM formats (pipe-friendly)
- **Format Selection**: Auto-detect from file extension or explicit `--output-format`

#### Rendering Pipeline
1. **Shape text** → glyph IDs and positions (HarfRust)
2. **Extract outlines** → skrifa reads glyph data (zero-copy)
3. **Convert to paths** → Transform skrifa outlines to zeno/vello format
4. **Apply optimizations**:
   - Early culling via bounding box checks
   - Batch affine transformations
   - Subpixel positioning quantization (1/4 or 1/8)
5. **Rasterize**:
   - CPU path: zeno with SIMD (`target-cpu=native`)
   - GPU path: vello compute shaders
6. **Compress** → zstd level 1-3 or LZ4
7. **Store** → Packfile shard or output path

#### Glyph Atlas Caching
- **Pre-rasterization**: Common glyphs at popular sizes
- **Storage Format**: `{glyph_id}_{font_hash}_{size}_{variation_hash}`
- **Memory-mapped**: Packfiles with zstd compression
- **LRU Cache**: Hot glyphs kept in memory
- **Lazy Loading**: Rasterize on first use, cache for reuse

### 4. Storage Backend (from 400.md)

#### Sharded Packfile Architecture
```
Shard File Structure (2-10 GiB each):
+---------------------+
| Compressed Images   |  <- Concatenated compressed blobs
| img0, img1, img2... |
+---------------------+
| Index Table         |  <- Fixed 20-byte entries
| [offset,len,w,h,crc]|
+---------------------+
| Footer              |  <- Magic, version, count, index_offset
+---------------------+
```

#### Storage Specifications
- **Shard size**: 5,000-20,000 images per shard file
- **Compression**: zstd level 1-3 (default) or LZ4 for lowest latency
- **Index entry**: 20 bytes (offset:u64, len:u32, width:u16, height:u16, crc:u32)
- **Memory mapping**: Index mmapped for O(1) lookups
- **Concurrency**: Read-only shards support unlimited concurrent readers
- **Cache**: Process-wide LRU of open shard mmaps (default: 256 shards)

#### Image Specifications
- **Dimensions**: 200-400px height × 3000-6000px width
- **Format**: 8-bit grayscale or 1-bit monochrome (bit-packed)
- **Size**: ~0.5-2.3 MiB uncompressed, ~0.2-1.2 MiB compressed
- **Total capacity**: ~10 million images (~3-6 TiB compressed)

### 5. Job Processing System

#### JSON Job Specification Format
```json
{
  "jobs": [
    {
      "id": "job_001",
      "font": {
        "path": "/path/to/font.ttf",
        "variations": {"wght": 500, "wdth": 125},
        "size": 16,
        "ppem": [96, 96]
      },
      "text": {
        "content": "Hello World",
        "direction": "ltr",
        "language": "en",
        "script": "Latn",
        "features": ["kern", "liga"]
      },
      "output": {
        "shape": true,
        "render": true,
        "format": "png",
        "storage": "packfile"
      },
      "rendering": {
        "dpi": 96,
        "antialiasing": true,
        "hinting": "full",
        "subpixel": "none",
        "threshold": 128,
        "dither": false,
        "bit_depth": 8,
        "encoding": "binary"
      }
    }
  ]
}
```

#### JSONL Output Format
```json
{"id":"job_001","input":{...},"shape":{"glyphs":[...]},"render":{"storage_ref":"shard_042/img_31415","width":4200,"height":300},"timing":{"shape_ms":1.2,"render_ms":3.4}}
```

### 6. Parallelization Strategy

#### Multi-Level Parallelism
1. **Job-level**: Process independent jobs in parallel (Rayon thread pool)
2. **Font-level**: Load multiple fonts concurrently
3. **Text-level**: Shape multiple texts per font in parallel
4. **Render-level**: GPU parallel compute for rendering
5. **Storage-level**: Parallel compression and shard writing

#### Resource Management
- **Thread pool size**: Configurable, defaults to CPU count
- **Memory limits**: Configurable max memory usage
- **Queue depth**: Bounded channels to prevent memory explosion
- **Backpressure**: Automatic throttling when storage can't keep up

## CLI Interface

**IMPORTANT: haforu is a SINGLE unified CLI tool that combines hb-shape and hb-view functionality with JSON batch processing capabilities. All operations are accessed through subcommands of the single `haforu` executable.**

### Unified haforu CLI Tool

```bash
# Show available commands
haforu --help
haforu shape --help  # hb-shape compatible
haforu view --help   # hb-view compatible
haforu process --help # batch JSON processing (new)
haforu query --help  # storage query operations (new)
```

### haforu shape - Text Shaping (hb-shape compatible)

```bash
# Basic usage matching hb-shape
haforu shape font.ttf "Hello World"

# With variations and features
haforu shape --variations="wght=500,wdth=125" --features="kern,liga" font.ttf "Text"

# Output formats (text or JSON)
haforu shape --output-format=json font.ttf "Text"

# With shaping options
haforu shape --direction=rtl --language=ar --script=Arab font.ttf "مرحبا"
```

### haforu view - Text Rendering (hb-view compatible)

```bash
# Basic rendering matching hb-view
haforu view font.ttf "Hello World" -o output.png

# With rendering options
haforu view --font-size=32 --margin=20 --background=#FFFFFF font.ttf "Text" -o image.png

# Output formats
haforu view --output-format=svg font.ttf "Text" -o output.svg
haforu view --output-format=pdf font.ttf "Text" -o output.pdf
haforu view --output-format=pgm font.ttf "Text" -o output.pgm  # Grayscale
haforu view --output-format=pbm font.ttf "Text" -o output.pbm  # Monochrome

# Direct stdout output (pipe-friendly)
haforu view --output-format=pgm-ascii font.ttf "Text" > output.pgm
haforu view --output-format=pbm-binary font.ttf "Text" | other-tool

# Advanced rendering options
haforu view --dpi=300 --subpixel=rgb --hinting=full font.ttf "Text" -o output.png
haforu view --threshold=128 --dither font.ttf "Text" -o output.pbm
```

### haforu process - Batch Processing (NEW)

```bash
# Process JSON jobs specification from stdin, output JSONL to stdout
echo '{"jobs":[...]}' | haforu process

# With storage backend for caching
echo '{"jobs":[...]}' | haforu process --storage-backend=packfile --storage-dir=/data/cache

# Stream processing for large batches
cat large_batch.json | haforu process --stream --parallel=16 > results.jsonl

# Include both shaping and rendering in output
echo '{"jobs":[...]}' | haforu process --shape --render --storage > results.jsonl
```

### haforu query - Storage Query Operations (NEW)

```bash
# List stored results
haforu query --list --filter="font:Roboto"

# Verify storage integrity
haforu query --verify --shard=042

# Export stored images by reference
haforu query --export --refs=refs.txt --output-dir=./export/

# Get statistics about storage
haforu query --stats
```

### Unified Features Across All Commands

All haforu commands share common options:

```bash
# Common font options
haforu [shape|view|process] --font-file=path --face-index=0

# Common variation options
haforu [shape|view|process] --variations="wght=500" --named-instance=3

# Common text options
haforu [shape|view|process] --text="Content" --text-file=input.txt

# Common output options
haforu [shape|view|process] -o output_file --output-format=json

# Logging and debugging
haforu [any-command] --verbose --log-level=debug
```

## Python Bindings

```python
import haforu

# Initialize with storage backend
engine = haforu.Engine(
    storage_dir="/data/cache",
    cache_size=512,  # Font cache size
    thread_count=16
)

# Single job
result = engine.shape_and_render(
    font_path="font.ttf",
    text="Hello World",
    variations={"wght": 500},
    size=16
)

# Batch processing
jobs = [
    {
        "font": {"path": "font.ttf", "size": 16},
        "text": {"content": text}
    }
    for text in texts
]

results = engine.process_batch(jobs, parallel=True)

# Retrieve from storage
image = engine.retrieve("shard_042/img_31415")
```

## Performance Targets

### Throughput Goals
- **Font loading**: < 1ms per font (memory-mapped)
- **Text shaping**: < 0.5ms per 100 characters
- **Rendering**: < 5ms per image (GPU-accelerated)
- **Storage write**: > 500 MB/s compressed throughput
- **Storage read**: < 1ms random access latency

### Scalability Metrics
- **Fonts**: Handle 1,000+ simultaneous font instances
- **Texts**: Process 10,000+ unique texts per batch
- **Images**: Store and retrieve from 10M+ image corpus
- **Concurrency**: Scale to 64+ CPU cores
- **Memory**: Operate within 16 GB RAM for typical workloads

## Dependencies

### Core Rust Crates
| Crate | Purpose | Why Chosen |
|-------|---------|------------|
| `read-fonts` | Font parsing | Zero-copy, safe, fontations ecosystem |
| `skrifa` | Font metrics & glyphs | Mid-level API, integrates with read-fonts |
| `harfrust` | Text shaping | Pure Rust HarfBuzz port, no C dependencies |
| `parley` | Text layout | Rich text support, integrates with above |
| `zeno` | CPU rasterization | Pure Rust, high-performance, SIMD-optimized |
| `vello` | GPU rendering | High-performance 2D rendering for batches |
| `rayon` | Parallelization | Data parallelism, work stealing |
| `jwalk` | Directory traversal | Parallel filesystem walking |
| `memmap2` | Memory mapping | Zero-copy file access |
| `zstd` | Compression | Best ratio/speed for images |
| `lz4_flex` | Alternative compression | Lowest latency option |
| `serde`/`serde_json` | JSON processing | De facto standard |
| `clap` | CLI parsing | Type-safe, feature-rich |
| `pyo3` | Python bindings | Safe Python integration |

## Project Structure

```
haforu/
├── haforu-core/          # Core library
│   ├── src/
│   │   ├── font.rs       # Font loading and caching
│   │   ├── shaping.rs    # HarfRust integration
│   │   ├── rendering.rs  # Vello rendering
│   │   ├── storage.rs    # Packfile backend
│   │   ├── job.rs        # Job processing
│   │   └── lib.rs        # Public API
│   └── Cargo.toml
│
├── haforu-cli/           # CLI tools
│   ├── src/
│   │   ├── shape.rs      # haforu-shape
│   │   ├── view.rs       # haforu-view
│   │   ├── query.rs      # haforu-query
│   │   └── main.rs       # CLI dispatch
│   └── Cargo.toml
│
├── haforu-python/        # Python bindings
│   ├── src/
│   │   └── lib.rs        # PyO3 bindings
│   ├── haforu/
│   │   └── __init__.py   # Python module
│   └── Cargo.toml
│
├── tests/                # Integration tests
├── benches/              # Performance benchmarks
├── examples/             # Usage examples
└── docs/                 # Documentation
```

## Testing Strategy

### Unit Tests
- Font loading edge cases
- Shaping correctness vs HarfBuzz
- Storage integrity checks
- Compression round-trips

### Integration Tests
- End-to-end batch processing
- Storage and retrieval cycles
- CLI compatibility with hb-shape/hb-view
- Python bindings functionality

### Performance Tests
- Benchmark vs HarfBuzz C++
- Parallel scaling efficiency
- Storage throughput limits
- Memory usage under load

### Test Data
- Use fonts from `03fonts/` directory
- Variable fonts for axis testing
- Complex scripts for shaping tests
- Large batches for stress testing

## Development Phases

### Phase 1: Foundation (Current)
- [x] Basic project structure
- [x] Font loading with read-fonts
- [x] JSON job specification parsing
- [ ] Integration with HarfRust for shaping
- [ ] Basic CLI with hb-shape compatibility

### Phase 2: Parallel Processing
- [ ] Rayon-based job parallelization
- [ ] Font cache with LRU eviction
- [ ] Batch processing pipeline
- [ ] JSONL output streaming

### Phase 3: Rendering
- [ ] Vello integration for GPU rendering
- [ ] Multiple output formats (PNG, SVG, PDF)
- [ ] Rendering cache layer

### Phase 4: Storage Backend
- [ ] Sharded packfile implementation
- [ ] Memory-mapped index
- [ ] Compression pipeline (zstd/LZ4)
- [ ] Storage query tools

### Phase 5: Production Features
- [ ] Python bindings with PyO3
- [ ] Comprehensive error handling
- [ ] Performance monitoring
- [ ] Documentation and examples

### Phase 6: Optimization
- [ ] Profile and optimize hot paths
- [ ] GPU rendering optimizations
- [ ] Storage compaction tools
- [ ] Memory usage optimization

## Configuration

### Environment Variables
```bash
HAFORU_CACHE_DIR=/data/haforu/cache      # Storage directory
HAFORU_MAX_MEMORY=16G                    # Memory limit
HAFORU_THREADS=32                        # Thread pool size
HAFORU_LOG_LEVEL=info                    # Logging verbosity
HAFORU_COMPRESSION=zstd:3                # Compression algorithm
```

### Configuration File (haforu.toml)
```toml
[storage]
backend = "packfile"
directory = "/data/haforu/cache"
shard_size = 10000
compression = "zstd"
compression_level = 3

[cache]
font_cache_size = 256
shard_cache_size = 256
memory_limit_gb = 16

[performance]
thread_count = 0  # 0 = auto-detect
gpu_enabled = true
batch_size = 1000

[logging]
level = "info"
file = "/var/log/haforu.log"
```

## Error Handling

### Error Categories
1. **Font Errors**: Invalid font files, missing glyphs
2. **Shaping Errors**: Unsupported scripts, feature conflicts
3. **Rendering Errors**: GPU failures, memory exhaustion
4. **Storage Errors**: Disk full, corruption detected
5. **Input Errors**: Invalid JSON, unsupported parameters

### Recovery Strategies
- **Graceful degradation**: Skip failed jobs, continue batch
- **Retry logic**: Configurable retries for transient failures
- **Fallback options**: CPU rendering if GPU fails
- **Detailed logging**: Error context for debugging
- **Validation upfront**: Catch errors early in pipeline

## Security Considerations

### Input Validation
- Sanitize file paths (prevent directory traversal)
- Limit JSON input size (default: 10MB)
- Validate text length (default max: 10K chars)
- Font file size limits (default: 100MB)

### Resource Limits
- Memory usage caps
- Thread pool bounds
- Storage quota management
- Rate limiting for API usage

## License

MIT License - See LICENSE file for details

## Contributing

See CONTRIBUTING.md for guidelines

## References

- HarfBuzz Documentation: https://harfbuzz.github.io/
- Fontations Project: https://github.com/googlefonts/fontations
- Vello Renderer: https://github.com/linebender/vello
- Storage Architecture: See 400.md for detailed packfile design
## Examples

- Shape and render a line of text to a simple PGM image:

  - Build and run: `cargo run --example shape_and_render`
  - Optional env vars:
    - `HAFORU_EXAMPLE_FONT` to set a font path (defaults to `03fonts/Archivo[wdth,wght].ttf`)
    - `HAFORU_EXAMPLE_TEXT` to set the text (defaults to `"Hello, Haforu!"`)
  - Output: writes `example_output.pgm` in the project root

- Orchestrator demos:
  - `cargo run --example orchestrator_simple`
  - `cargo run --example orchestrator_demo`

## Running Tests

- Unit and integration tests: `cargo test`
- Tests exercise JSON parsing, font loading, storage packfiles, orchestrator logic, and end-to-end shaping + CPU rasterization using fonts in `03fonts/`.
