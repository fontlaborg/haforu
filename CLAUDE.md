# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

The haforu project aims to create a set of Rust crates that provide:
- A library, CLI tool, and Python bindings for font shaping and rendering
- Emulation and enhancement of HarfBuzz tools (`hb-shape` and `hb-view`)
- JSON-based batch processing for multiple fonts, variations, sizes, and texts
- High-performance storage and retrieval of pre-rendered font results

## Key Project Requirements

### Input/Output Format
- **Input**: JSON from stdin as "jobs-spec" containing:
  - Multiple fonts and variable font instances
  - Multiple sizes and texts
  - Shaping and rendering parameters
- **Output**: JSONL (JSON Lines) where each line contains:
  - Original input parameters
  - Shaping output (similar to hb-shape) if requested
  - Rendering identifier (file path or database reference)

### Storage System (from 400.md)
The project requires fast storage of ~10 million monochrome images using:
- **Sharded packfiles** with memory-mapped index
- **Compression**: zstd level 1-3 or LZ4
- **Shard size**: 5k-20k images per file (2-10 GiB)
- **Index format**: offset, length, width, height, checksum (20 bytes per image)
- Alternative storage backends: LMDB/MDBX, RocksDB, or SQLite

## Core Dependencies

### Font Processing Stack
1. **fontations** (`01code/fontations/`): Core font parsing and manipulation
   - `font-types`: Common OpenType type definitions
   - `read-fonts`: High-performance, zero-copy font parser
   - `write-fonts`: Font modification and writing
   - `skrifa`: Mid-level library for metadata and glyph loading

2. **harfrust** (`01code/harfrust/`): Rust port of HarfBuzz text shaping
   - Uses `read-fonts` for parsing
   - No external dependencies (no FreeType, ICU, etc.)
   - ~25% slower than HarfBuzz C++

3. **parley** (`01code/parley/`): Rich text layout
   - Uses Fontique for font enumeration/fallback
   - Integrates HarfRust for shaping
   - Skrifa for font reading

4. **vello** (`01code/vello/`): GPU-accelerated 2D renderer
   - Uses wgpu for GPU compute
   - Can render large 2D scenes interactively

## Development Commands

### Building the Project
```bash
# Initialize a new Rust project (if needed)
cargo init --name haforu

# Add core dependencies
cargo add read-fonts skrifa harfrust parley vello
cargo add zstd lz4_flex memmap2  # For storage backend
cargo add serde serde_json       # For JSON processing
cargo add clap                   # For CLI arguments

# Build the project
cargo build --release

# Run tests
cargo test

# Run a specific test
cargo test test_name

# Check code without building
cargo check

# Format code
cargo fmt

# Run clippy for linting
cargo clippy
```

### Working with Example Fonts
Test fonts are available in `03fonts/`:
- Variable fonts: `AnekDevanagari[wdth,wght].ttf`, `Archivo[wdth,wght].ttf`, etc.
- Use these for testing shaping and rendering implementations

## Architecture Considerations

### Zero-Copy Philosophy
The fontations ecosystem uses zero-copy parsing throughout:
- `FontData<'a>` provides safe byte slice borrowing
- `TableRef<'a, T>` enforces lifetime-based table access
- No allocation or copying during parsing
- This approach prevents buffer overflows and use-after-free bugs

### JSON Jobs Processing Pipeline
1. **Parse JSON jobs** from stdin
2. **For each job**:
   - Load font (with caching)
   - Apply variations if specified
   - Shape text using harfrust
   - Render if requested using vello
   - Store results in database/filesystem
3. **Output JSONL** results to stdout

### Storage Backend Architecture
- Use sharded files to avoid millions of individual files
- Memory-map indices for O(1) lookups
- Compress images with zstd/LZ4
- Keep shards immutable once written
- Implement process-wide LRU cache for open shards

## Key Implementation Notes

1. **Font Loading**: Use `read-fonts::FontRef` as the entry point
2. **Shaping**: HarfRust always uses UnitsPerEm; scale results manually
3. **Variable Fonts**: Handle variations via axis tags (e.g., "wght=500")
4. **Performance**: Leverage parallel processing with rayon where appropriate
5. **Error Handling**: Use Result types throughout; avoid panics

## Rasterization Architecture (CRITICAL)

### Primary CPU Path: skrifa + zeno
- **Use `zeno` for CPU rasterization**, NOT tiny-skia
- **Reasoning**: zeno is minimal, focused, and has zero dependencies
- **Pipeline**: skrifa outline extraction → zeno path building → alpha mask
- **Performance**: Compile with `target-cpu=native` for SIMD optimizations

### Why zeno over tiny-skia:
1. **Minimal dependencies**: zeno has only optional libm vs tiny-skia's multiple deps
2. **Focused scope**: Pure rasterization vs full 2D rendering we don't need
3. **Smaller binary**: Critical for Python package distribution
4. **Simpler integration**: Direct path-to-mask API perfect for glyph rendering

### GPU Path: vello (for batches)
- Use vello only for large batches (10K+ glyphs)
- Higher setup cost but better throughput for massive parallelism

### Implementation Pattern:
```rust
// Create adapter from skrifa's OutlinePen to zeno's path builder
struct ZenoPen { /* ... */ }
impl OutlinePen for ZenoPen { /* ... */ }

// Rasterize: skrifa → zeno → alpha mask
let mask = Mask::new(&path)
    .size(width, height)
    .format(Format::Alpha)
    .render();
```

## Reference Material

- **Book on Rust font processing**: See `02book/` for comprehensive guide
- **HarfBuzz CLI reference**: `hb-shape.txt` and `hb-view.txt` for CLI interface patterns
- **Storage patterns**: `400.md` for detailed database implementation strategies

---



Inside this folder we want a spec and implementation of a set of "haforu-" Rust crates: 

- a lib and CLI tool and a Python bindings package where the CLI emulates or follows the CLI of @./hb-shape.txt and @./hb-view.txt but enhances it so that it takes a JSON from stdin as a "jobs-spec" for shaping & rendering of multiple fonts, variable font instances, sizes, and texts, and outputs a JSONL "jobs-result" where each JSON line is a job result that repeats the input params, and includes a shaping output similar to hb-shape (if requested) and includes an identifier to the rendering, which may be a path to file (if requested) or some pointed to the output database (if requested). 
- For output database see @./400.md —— the package should also allow for fast retrieval of the prerendered results from the database or filesystem, so the database or filesystem can be used as a cache for the prerendered results.
- We want the code to be extremely fast, using all parallelism and other Rust features to the fullest, but also safe
- We want the code to just as easily be able to render 10,000 different texts using one font instance or one text using 10,000 different font instances coming from hundreds of different variable fonts.

References: 

- @./01code/ contains a large colleciton of Rust repos that are useful
- @./02book/ contains a book about Rust font usage and text shaping (you can then research more detailed info inside the @./01code/ folder ) ./01code-tldr.txt is a compact overview of the @./01code/ folder.
- @./03fonts/ contains a small colleciton of font files that you can use to test the package



