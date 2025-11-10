# PLAN.md - Haforu CLI Tool Comprehensive Specification

## Executive Overview

**Haforu** is a single, unified CLI tool that combines and enhances the functionality of HarfBuzz's `hb-shape` and `hb-view` tools. It processes JSON job specifications from stdin and outputs JSONL (JSON Lines) results to stdout, supporting batch processing of thousands of font/text combinations with optional rendering and storage.

## Architecture Overview

```
┌─────────────────────────────────────────────────┐
│                 haforu CLI                       │
│                                                  │
│  ┌───────────────────────────────────────────┐  │
│  │          Command Parser                   │  │
│  │  - Traditional mode (hb-shape/view compat)│  │
│  │  - Batch mode (JSON jobs from stdin)      │  │
│  └───────────────────────────────────────────┘  │
│                      ↓                           │
│  ┌───────────────────────────────────────────┐  │
│  │          Job Orchestrator                 │  │
│  │  - Parallelization strategy selection     │  │
│  │  - Work unit distribution                 │  │
│  │  - Resource management                    │  │
│  └───────────────────────────────────────────┘  │
│                      ↓                           │
│  ┌───────────────────────────────────────────┐  │
│  │          Processing Pipeline              │  │
│  │  1. Font loading (mmap_font.rs)           │  │
│  │  2. Text shaping (shaping.rs)             │  │
│  │  3. Rendering (rasterize.rs)              │  │
│  │  4. Storage (storage.rs)                  │  │
│  └───────────────────────────────────────────┘  │
│                      ↓                           │
│  ┌───────────────────────────────────────────┐  │
│  │          Output Formatter                 │  │
│  │  - JSONL for batch mode                   │  │
│  │  - Text/JSON for traditional mode         │  │
│  └───────────────────────────────────────────┘  │
└─────────────────────────────────────────────────┘
```

## CLI Interface Specification

### Command Structure

```bash
haforu [OPTIONS] [FONT_FILE] [TEXT]
```

### Operating Modes

#### 1. Traditional Mode (HarfBuzz Compatibility)

When called with font file and text arguments, operates like hb-shape/hb-view:

```bash
# Shape only (like hb-shape)
haforu font.ttf "Hello World" --no-render

# Shape and render (like hb-view)
haforu font.ttf "Hello World" -o output.png

# With variations
haforu --variations="wght=500,wdth=125" font.ttf "Text"
```

#### 2. Batch Mode (JSON Jobs)

When `--batch` flag is present or stdin is piped, reads JSON jobs specification:

```bash
# Read from stdin
echo '{"jobs":[...]}' | haforu --batch

# Read from file
haforu --batch < jobs.json

# With storage backend
haforu --batch --storage-dir=/data/cache --store-results
```

### Command-Line Options

#### Core Options
```
--batch                    Enable batch processing mode (JSON input)
--help, -h                 Show help message
--version, -V              Show version information
--verbose, -v              Increase verbosity (can be repeated)
--quiet, -q                Suppress non-error output
```

#### Font Options (see [hb-shape.txt](./hb-shape.txt))
```
--font-file=PATH           Font file path
--face-index=N             Face index in font file (default: 0)
--font-size=SIZE           Font size in points or 'upem'
--variations=LIST          Comma-separated variation settings (e.g., "wght=500,wdth=125")
--named-instance=N         Use named instance from variable font
```

#### Shaping Options (see [hb-shape.txt](./hb-shape.txt))
```
--direction=DIR            Text direction (ltr/rtl/ttb/btt/auto)
--language=LANG            BCP 47 language tag
--script=SCRIPT            ISO-15924 script tag
--features=LIST            OpenType features (e.g., "kern,liga,calt")
--no-shape                 Skip shaping (render only)
--shapers=LIST             Shaper preference list (default: ot,fallback)
```

#### Rendering Options (see [hb-view.txt](./hb-view.txt))
```
--no-render                Skip rendering (shape only)
--output, -o PATH          Output file path (PNG/SVG/PDF/PBM/PGM or stdout)
--output-format=FORMAT     Output format selection:
                          Bitmap: png, pbm, pbm-ascii, pbm-binary,
                                 pgm, pgm-ascii, pgm-binary
                          Vector: svg, pdf
                          Data: json
--foreground=COLOR         Text color (hex: RRGGBB or RRGGBBAA)
--background=COLOR         Background color
--margin=SIZE              Margin around output (default: 16)
--font-extents=VALUES      Set ascent/descent/line-gap
--dpi=VALUE               Resolution for rasterization (default: 96)
--antialiasing=BOOL       Enable/disable antialiasing (default: true)
--hinting=MODE            Hinting: none|slight|medium|full (default: full)
--subpixel=MODE           Subpixel: none|rgb|bgr|vrgb|vbgr (default: none)
--threshold=VALUE         Monochrome threshold 0-255 (default: 128)
--dither=BOOL             Enable dithering for monochrome (default: false)
--bit-depth=VALUE         Bit depth: 1|8|16 (default: 8)
--encoding=MODE           PBM/PGM encoding: ascii|binary (default: binary)
```

#### Storage Options (see [400.md](./400.md))
```
--storage-dir=PATH         Storage directory for packfiles
--store-results            Store rendered results in database
--retrieve=ID              Retrieve stored result by ID
--compression=ALGO         Compression (zstd:3, lz4, none)
--shard-size=N             Images per shard (default: 10000)
```

#### Parallelization Options
```
--threads=N                Thread pool size (0=auto)
--strategy=STRAT           Parallelization strategy (auto/font/instance/text)
--max-parallel=N           Max parallel jobs
--batch-size=N             Jobs per batch
```

## JSON Jobs Specification Format

### Input Format (jobs-spec)

The input is a JSON object containing an array of jobs. Each job specifies:
- Font configuration (path, variations, size)
- Text content and shaping parameters
- Output requirements (shape data, rendering, storage)

```json
{
  "version": "1.0",
  "defaults": {
    "font": {
      "size": 16,
      "ppem": [96, 96]
    },
    "shaping": {
      "direction": "ltr",
      "language": "en",
      "script": "Latn"
    },
    "rendering": {
      "format": "png",
      "foreground": "#000000",
      "background": "#FFFFFF"
    },
    "output": {
      "include_shaping": true,
      "include_rendering": true,
      "store_in_db": false
    }
  },
  "jobs": [
    {
      "id": "job_001",
      "font": {
        "path": "/path/to/font.ttf",
        "face_index": 0,
        "variations": {
          "wght": 500,
          "wdth": 125
        },
        "size": 24,
        "named_instance": null
      },
      "text": {
        "content": "Hello World",
        "direction": "ltr",
        "language": "en",
        "script": "Latn",
        "features": ["kern", "liga", "calt"],
        "cluster_level": 0
      },
      "rendering": {
        "format": "png",
        "output_path": "/output/job_001.png",
        "foreground": "#000000",
        "background": "#FFFFFF",
        "margin": 16,
        "dpi": 96
      },
      "output": {
        "include_shaping": true,
        "include_rendering": true,
        "store_in_db": true,
        "output_path": "/output/job_001.png"
      }
    }
  ]
}
```

### Output Format (JSONL jobs-result)

Each line is a complete JSON object representing one job result:

```json
{"id":"job_001","status":"success","input":{"font":{"path":"/path/to/font.ttf","size":24},"text":{"content":"Hello World"}},"shaping":{"glyphs":[{"glyph_id":43,"cluster":0,"x_advance":576,"y_advance":0,"x_offset":0,"y_offset":0},{"glyph_id":72,"cluster":1,"x_advance":512,"y_advance":0,"x_offset":0,"y_offset":0}],"direction":"ltr","script":"Latn","language":"en"},"rendering":{"output_path":"/output/job_001.png","width":4200,"height":300,"format":"png","storage_id":"shard_042/img_31415"},"timing":{"total_ms":4.7,"shape_ms":1.2,"render_ms":3.4,"store_ms":0.1},"metadata":{"font_family":"Roboto","font_version":"2.138","glyph_count":72}}
```

### Field Specifications

#### Job ID (`id`)
- **Type**: String
- **Required**: Yes
- **Description**: Unique identifier for the job
- **Example**: `"job_001"`, `"test_42_arial_bold"`

#### Font Configuration (`font`)
- **path** (string, required): Absolute or relative path to font file
- **face_index** (integer): Index of face in collection (default: 0)
- **variations** (object): Variable font axis settings as key-value pairs
- **size** (number): Font size in points (default: 16)
- **named_instance** (integer|null): Named instance index for variable fonts

#### Text Configuration (`text`)
- **content** (string, required): Text to shape/render
- **direction** (string): Text direction - "ltr", "rtl", "ttb", "btt", "auto"
- **language** (string): BCP 47 language tag (e.g., "en", "ar-SA")
- **script** (string): ISO-15924 script code (e.g., "Latn", "Arab")
- **features** (array): OpenType feature tags to enable
- **cluster_level** (integer): HarfBuzz cluster level (0-2)

#### Rendering Configuration (`rendering`)
- **format** (string): Output format - "png", "svg", "pdf"
- **output_path** (string): Where to save rendered image
- **foreground** (string): Text color as hex (#RRGGBB or #RRGGBBAA)
- **background** (string): Background color
- **margin** (integer): Pixels of margin around text
- **dpi** (integer): Resolution for rasterization

#### Output Control (`output`)
- **include_shaping** (boolean): Include shaping data in output
- **include_rendering** (boolean): Perform rendering
- **store_in_db** (boolean): Store in packfile database
- **output_path** (string): File path for rendered output

## Implementation Modules

### 1. Main CLI Entry Point ([src/main.rs](./src/main.rs))

```rust
use clap::{Parser, Subcommand};
use haforu::{JobOrchestrator, JobSpec};

#[derive(Parser)]
#[command(name = "haforu")]
#[command(about = "High-performance font shaping and rendering")]
struct Cli {
    #[arg(long)]
    batch: bool,

    // Font file for traditional mode
    font_file: Option<String>,

    // Text for traditional mode
    text: Option<String>,

    // ... other options
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.batch || is_stdin_piped() {
        run_batch_mode(cli)
    } else {
        run_traditional_mode(cli)
    }
}
```

### 2. Job Orchestrator ([src/orchestrator.rs](./src/orchestrator.rs))

Manages parallel job execution with intelligent work distribution:

```rust
pub struct JobOrchestrator {
    font_cache: MmapFontCache,
    shaper_pool: ShaperPool,
    renderer_pool: RendererPool,
    storage: StorageManager,
    thread_pool: ThreadPool,
}

impl JobOrchestrator {
    pub fn process_jobs(&mut self, spec: JobSpec) -> Vec<JobResult> {
        // 1. Analyze workload
        let stats = self.analyze_jobs(&spec);

        // 2. Select parallelization strategy
        let strategy = self.determine_strategy(&stats);

        // 3. Create work units
        let work_units = self.create_work_units(&spec, strategy);

        // 4. Execute in parallel
        work_units.par_iter()
            .map(|unit| self.process_work_unit(unit))
            .collect()
    }
}
```

### 3. Font Management ([src/mmap_font.rs](./src/mmap_font.rs))

Zero-copy font loading with memory mapping:

```rust
pub struct FileInfo {
    pub path: PathBuf,
    pub mmap: Arc<Mmap>,
    pub font_type: FontType,
    pub font_count: u32,
}

impl FileInfo {
    pub fn from_path(path: &Path) -> Result<Self> {
        // Memory-map the font file
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        // Detect font type and validate
        let font_type = FontType::from_data(&mmap)?;

        Ok(FileInfo { ... })
    }

    pub fn get_font(&self, index: u32) -> Result<FontRef> {
        // Return zero-copy FontRef
    }
}
```

### 4. Text Shaping ([src/shaping.rs](./src/shaping.rs))

HarfRust integration for text shaping:

```rust
pub struct TextShaper {
    cached_font_data: Option<(Vec<u8>, ShaperData)>,
}

impl TextShaper {
    pub fn shape(&mut self,
        font_data: &[u8],
        text: &str,
        size: f32,
        options: &ShapingOptions
    ) -> Result<ShapingOutput> {
        // Create HarfRust font
        let font = HarfRustFontRef::from_index(font_data, 0)?;

        // Configure buffer
        let mut buffer = UnicodeBuffer::new();
        buffer.push_str(text);
        buffer.set_direction(parse_direction(&options.direction)?);

        // Shape with features
        let shaper = /* ... */;
        let glyph_buffer = shaper.shape(buffer, &features);

        // Extract results
        Ok(ShapingOutput { glyphs, ... })
    }
}
```

### 5. Rendering ([src/rasterize.rs](./src/rasterize.rs))

CPU rasterization with skrifa + zeno:

```rust
pub struct CpuRasterizer {
    subpixel_precision: u32,
}

impl CpuRasterizer {
    pub fn render_glyph(
        &self,
        font: &FontRef,
        glyph_id: GlyphId,
        size: f32,
    ) -> Result<RenderedGlyph> {
        // Extract outline with skrifa
        let glyph = font.outline_glyphs().get(glyph_id)?;

        // Convert to zeno path
        let mut pen = ZenoPen::new();
        glyph.draw(settings, &mut pen)?;

        // Rasterize with zeno
        let mask = Mask::new(&path)
            .size(width, height)
            .render();

        Ok(RenderedGlyph { ... })
    }
}
```

### 6. Storage Backend ([src/storage.rs](./src/storage.rs))

Sharded packfile system for efficient storage:

```rust
pub struct StorageManager {
    shards: HashMap<u32, Shard>,
    current_shard: u32,
    config: StorageConfig,
}

impl StorageManager {
    pub fn store_image(&mut self, image: &[u8], metadata: ImageMetadata) -> Result<String> {
        // Compress image
        let compressed = zstd::compress(image, 3)?;

        // Get or create shard
        let shard = self.get_current_shard()?;

        // Append to shard
        let offset = shard.append(compressed)?;

        // Return storage ID
        Ok(format!("shard_{:03}/img_{:05}", shard.id, offset))
    }

    pub fn retrieve_image(&self, storage_id: &str) -> Result<Vec<u8>> {
        // Parse storage ID
        let (shard_id, img_offset) = parse_storage_id(storage_id)?;

        // Get shard
        let shard = self.shards.get(&shard_id)?;

        // Read and decompress
        let compressed = shard.read_at(img_offset)?;
        Ok(zstd::decompress(compressed)?)
    }
}
```

## Testing Strategy

### Unit Tests

Each module should have comprehensive unit tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shape_simple_text() {
        let shaper = TextShaper::new();
        let result = shaper.shape(font_data, "Hello", 16.0, &options);
        assert_eq!(result.glyphs.len(), 5);
    }

    #[test]
    fn test_render_glyph() {
        let rasterizer = CpuRasterizer::new(16.0);
        let glyph = rasterizer.render_glyph(&font, glyph_id, 16.0);
        assert!(glyph.is_ok());
    }
}
```

### Integration Tests

End-to-end tests in `tests/` directory:

```rust
// tests/cli_test.rs
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_traditional_mode() {
    let mut cmd = Command::cargo_bin("haforu").unwrap();
    cmd.arg("font.ttf")
       .arg("Hello World")
       .arg("--no-render");

    cmd.assert()
       .success()
       .stdout(predicate::str::contains("glyph"));
}

#[test]
fn test_batch_mode() {
    let mut cmd = Command::cargo_bin("haforu").unwrap();
    cmd.arg("--batch")
       .write_stdin(r#"{"jobs":[...]}"#);

    cmd.assert()
       .success()
       .stdout(predicate::str::contains("\"status\":\"success\""));
}
```

### Performance Benchmarks

Using criterion for benchmarking:

```rust
// benches/shaping_bench.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_shaping(c: &mut Criterion) {
    c.bench_function("shape_100_chars", |b| {
        b.iter(|| {
            shaper.shape(black_box(font_data), black_box(text), 16.0, &options)
        });
    });
}
```

## Error Handling

### Error Types

```rust
#[derive(Error, Debug)]
pub enum HaforuError {
    #[error("Font error: {0}")]
    Font(String),

    #[error("Shaping error: {0}")]
    Shaping(String),

    #[error("Rendering error: {0}")]
    Rendering(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}
```

### Error Recovery

- Continue processing other jobs on single job failure
- Log errors with full context
- Include error details in JSONL output
- Provide `--fail-fast` option for strict mode

## Performance Optimization

### Parallelization Strategies

1. **Font-Level**: Best for many fonts with few instances
2. **Instance-Level**: Best for few fonts with many variations
3. **Text-Level**: Best for few fonts with many texts
4. **Hierarchical**: Adaptive based on workload analysis

### Caching Layers

1. **Font Cache**: Memory-mapped fonts (LRU, 256 fonts)
2. **Shaper Cache**: Reuse ShaperData for same font
3. **Glyph Cache**: Pre-rendered common glyphs
4. **Storage Cache**: Open shard mmaps (LRU, 256 shards)

### Performance Targets

- Font loading: < 1ms (memory-mapped)
- Text shaping: < 0.5ms per 100 chars
- Glyph rendering: < 0.1ms per glyph
- Storage write: > 500 MB/s compressed
- Parallel scaling: > 80% efficiency up to 64 cores

## Configuration

### Configuration File (`~/.haforu/config.toml`)

```toml
[general]
default_mode = "batch"
verbose = false
threads = 0  # 0 = auto-detect

[fonts]
cache_size = 256
search_paths = ["/usr/share/fonts", "~/.fonts"]

[shaping]
default_shaper = "ot"
default_direction = "auto"
default_language = "en"

[rendering]
default_format = "png"
default_dpi = 96
cpu_rasterizer = "zeno"  # or "gpu" for vello

[storage]
backend = "packfile"
directory = "~/.haforu/cache"
compression = "zstd"
compression_level = 3
shard_size = 10000

[performance]
batch_size = 1000
max_parallel_jobs = 100
memory_limit_gb = 16
```

## Deployment

### Building

```bash
# Development build
cargo build

# Release build with optimizations
cargo build --release --features "simd"

# With specific CPU optimizations
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

### Installation

```bash
# Install locally
cargo install --path .

# Install from crates.io (future)
cargo install haforu
```

### Docker

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/haforu /usr/local/bin/
ENTRYPOINT ["haforu"]
```

## Future Enhancements

### Phase 1 (Current)
- [x] Core library structure
- [x] Memory-mapped font loading
- [x] HarfRust integration
- [x] CPU rasterization with zeno
- [ ] Unified CLI tool
- [ ] Batch processing from JSON

### Phase 2
- [ ] GPU rendering with vello
- [ ] Python bindings with PyO3
- [ ] Web server mode
- [ ] Distributed processing

### Phase 3
- [ ] Font subsetting
- [ ] Color font support
- [ ] Advanced caching strategies
- [ ] Cloud storage backends

## References

- [HarfBuzz Documentation](https://harfbuzz.github.io/)
- [Fontations Project](https://github.com/googlefonts/fontations)
- [HarfRust Source](./01code/harfrust/)
- [Storage Design](./400.md)
- [Example Fonts](./03fonts/)