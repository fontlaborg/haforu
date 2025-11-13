---
this_file: external/haforu/TODO.md
---

# 🚨 DEPRECATION NOTICE: Legacy haforu repo

This repository is deprecated in favor of the clean‑slate implementation residing at `../haforu2/`, which will be published and imported as `haforu` (no "2"). Keep this only until the new package is ready; do not invest in new features here.

Minimal tasks for transition:

- [ ] Freeze feature work; accept only migration‑critical patches
- [ ] Document how to point `fontsimi` to the new `haforu` binary once built
- [ ] After `haforu` (new) is validated in `fontsimi`, remove this folder/symlink

# 🚀 CRITICAL PRIORITY: HAFORU RUST RENDERING IMPLEMENTATION

**Status:** Foundation complete (JSON parsing, font loading stubs). BEGIN RENDERING NOW.
**Expected Impact:** Enable 100× speedup for FontSimi (5h → 3min analysis)
**Timeline:** 12-18 days for H2 complete

**Note:** This TODO tracks Haforu Rust tasks. See @../../TODO.md for FontSimi Python tasks.

---

## H2 — Haforu Rust Rendering Implementation ⚡ START HERE

**Goal:** Make Haforu actually render fonts and return base64-encoded PGM images

**Location:** `src/` (Rust implementation)

**Status:** READY TO BEGIN - All prerequisites met

### Prerequisites (Complete) ✅
- [x] Rust project structure established
- [x] JSON job specification parser (serde-based)
- [x] Font loading infrastructure (read-fonts, skrifa)
- [x] HarfRust shaping integration (basic)
- [x] Zeno rasterization integration (basic)
- [x] CLI framework (clap-based)

---

## H2.1: Implement JSON Job Processing (2-3 days) ⚡ CRITICAL

**Files:** `src/json_parser.rs`, `src/main.rs`

**Goal:** Parse JSON job specifications from stdin and extract all required parameters

### Task 1: Complete JobSpec Data Structures (4 hours)

**File:** `src/json_parser.rs` (lines 1-100)

- [ ] Define complete `JobSpec` struct matching FontSimi format
  ```rust
  #[derive(Debug, Clone, Deserialize)]
  pub struct JobSpec {
      pub version: String,
      pub jobs: Vec<Job>,
  }

  #[derive(Debug, Clone, Deserialize)]
  pub struct Job {
      pub id: String,
      pub font: FontConfig,
      pub text: TextConfig,
      pub rendering: RenderingConfig,
  }

  #[derive(Debug, Clone, Deserialize)]
  pub struct FontConfig {
      pub path: PathBuf,
      pub size: u32,  // Font size in points (1000 for FontSimi)
      #[serde(default)]
      pub variations: HashMap<String, f32>,  // Variable font coordinates
  }

  #[derive(Debug, Clone, Deserialize)]
  pub struct TextConfig {
      pub content: String,
      pub script: Option<String>,
  }

  #[derive(Debug, Clone, Deserialize)]
  pub struct RenderingConfig {
      pub format: String,  // "pgm"
      pub encoding: String,  // "binary"
      pub width: u32,  // 3000
      pub height: u32,  // 1200
  }
  ```

- [ ] Add validation for required fields
  - [ ] Verify `id` is non-empty string
  - [ ] Verify `font.path` exists and is readable
  - [ ] Verify `font.size` > 0 and < 10000
  - [ ] Verify `text.content` is non-empty and < 10000 chars
  - [ ] Verify `rendering.format` is "pgm"
  - [ ] Verify `rendering.width` and `rendering.height` > 0 and < 10000

### Task 2: Implement JSON Parsing from stdin (4 hours)

**File:** `src/main.rs` (lines 50-150)

- [ ] Read JSON from stdin in batch mode
  ```rust
  fn read_job_spec_from_stdin() -> Result<JobSpec, Error> {
      let stdin = io::stdin();
      let reader = stdin.lock();
      let spec: JobSpec = serde_json::from_reader(reader)
          .map_err(|e| Error::InvalidInput(format!("JSON parse error: {}", e)))?;
      Ok(spec)
  }
  ```

- [ ] Validate job spec structure
  - [ ] Check version is "1.0"
  - [ ] Check jobs array is non-empty
  - [ ] Validate each job using schema

- [ ] Handle malformed JSON gracefully
  - [ ] Return descriptive error messages
  - [ ] Include line/column numbers if available
  - [ ] Suggest fixes for common mistakes

### Task 3: Unit Tests for JSON Parsing (2 hours)

**File:** `src/json_parser.rs` (lines 200-400)

- [ ] Test valid job spec parsing
  - [ ] Single job with all fields
  - [ ] Multiple jobs (100+)
  - [ ] Variable font coordinates present

- [ ] Test invalid job spec rejection
  - [ ] Missing required fields
  - [ ] Invalid field values (negative sizes, etc.)
  - [ ] Malformed JSON syntax

- [ ] Test edge cases
  - [ ] Empty jobs array
  - [ ] Very long text content (9999 chars)
  - [ ] Many variation axes (20+)

**Estimated Time:** 10-12 hours (1.5 days)

---

## H2.2: Implement Font Loading with Variations (2-3 days) ⚡ CRITICAL

**Files:** `src/mmap_font.rs`, `src/orchestrator.rs`

**Goal:** Load fonts using read-fonts/skrifa and apply variable font coordinates

### Task 1: Memory-Mapped Font Loading (6 hours)

**File:** `src/mmap_font.rs` (lines 1-200)

- [ ] Implement font file memory mapping
  ```rust
  pub struct MmapFont {
      path: PathBuf,
      mmap: Arc<Mmap>,
      font_ref: FontRef<'static>,
  }

  impl MmapFont {
      pub fn new(path: &Path) -> Result<Self, Error> {
          let file = File::open(path)?;
          let mmap = unsafe { Mmap::map(&file)? };
          let mmap_arc = Arc::new(mmap);

          // SAFETY: mmap is Arc and will live as long as MmapFont
          let font_data: &'static [u8] = unsafe {
              std::slice::from_raw_parts(
                  mmap_arc.as_ptr(),
                  mmap_arc.len()
              )
          };

          let font_ref = FontRef::new(font_data)?;

          Ok(MmapFont {
              path: path.to_path_buf(),
              mmap: mmap_arc,
              font_ref,
          })
      }
  }
  ```

- [ ] Validate font file format
  - [ ] Check magic bytes (TTF, OTF, TTC, WOFF, WOFF2)
  - [ ] Verify font tables exist
  - [ ] Check font is not corrupted

- [ ] Handle font collections (TTC/OTC)
  - [ ] Extract face index 0 by default
  - [ ] Support face_index parameter

### Task 2: Variable Font Coordinate Application (8 hours)

**File:** `src/mmap_font.rs` (lines 200-400)

- [ ] Extract font variation axes
  ```rust
  pub fn get_variation_axes(&self) -> Vec<(Tag, AxisInfo)> {
      self.font_ref.axes()
          .map(|axis| {
              let tag = axis.tag();
              let info = AxisInfo {
                  min: axis.min_value(),
                  max: axis.max_value(),
                  default: axis.default_value(),
              };
              (tag, info)
          })
          .collect()
  }
  ```

- [ ] Apply variation coordinates to font instance
  ```rust
  pub fn apply_variations(&self, coords: &HashMap<String, f32>) -> Result<FontRef, Error> {
      let axes = self.get_variation_axes();
      let location: Vec<(Tag, f32)> = coords.iter()
          .filter_map(|(tag_str, value)| {
              let tag = Tag::from_bytes(tag_str.as_bytes()).ok()?;
              // Clamp to axis bounds
              let clamped = axes.iter()
                  .find(|(t, _)| *t == tag)
                  .map(|(_, info)| value.clamp(info.min, info.max))
                  .unwrap_or(*value);
              Some((tag, clamped))
          })
          .collect();

      // Create font instance with variations
      let instance = self.font_ref.clone_with_variations(&location)?;
      Ok(instance)
  }
  ```

- [ ] Implement font instance caching
  - [ ] Create `FontCache` struct with LRU eviction
  - [ ] Key: `(PathBuf, HashMap<String, f32>)` tuple
  - [ ] Value: `Arc<FontRef>`
  - [ ] Max size: 512 fonts (configurable)

### Task 3: Static Font Handling (2 hours)

**File:** `src/mmap_font.rs` (lines 400-500)

- [ ] Detect if font is static (no variation axes)
  ```rust
  pub fn is_variable(&self) -> bool {
      self.font_ref.axes().count() > 0
  }
  ```

- [ ] Handle static fonts with variations parameter
  - [ ] Ignore variations if font is static
  - [ ] Log warning if variations provided for static font
  - [ ] Return font as-is

### Task 4: Error Handling (2 hours)

**File:** `src/mmap_font.rs` (lines 500-600)

- [ ] Handle font loading failures
  - [ ] File not found: descriptive error with path
  - [ ] Invalid font format: include file format detected
  - [ ] Corrupted font: explain which table is invalid

- [ ] Handle variation application failures
  - [ ] Unknown axis: list available axes in error
  - [ ] Out of bounds: show bounds and provided value
  - [ ] No variation support: explain font is static

### Task 5: Unit Tests (4 hours)

**File:** `tests/font_loading_tests.rs` (new file)

- [ ] Test static font loading
  - [ ] Load Arial-Black.ttf from test-fonts
  - [ ] Verify font metadata extraction
  - [ ] Test render with static font

- [ ] Test variable font loading
  - [ ] Load Playfair[opsz,wdth,wght].ttf from test-fonts
  - [ ] Extract variation axes
  - [ ] Apply valid coordinates

- [ ] Test variation bounds clamping
  - [ ] Provide out-of-bounds wght value
  - [ ] Verify clamped to axis limits
  - [ ] Verify no error raised

- [ ] Test font cache
  - [ ] Load same font 10 times
  - [ ] Verify only one mmap created
  - [ ] Test cache eviction (513th font)

**Estimated Time:** 22-24 hours (2.5-3 days)

---

## H2.3: Implement Text Shaping (2-3 days) ⚡ CRITICAL

**Files:** `src/shaping.rs`, `src/orchestrator.rs`

**Goal:** Use HarfRust to shape text into positioned glyphs

### Task 1: HarfRust Integration (8 hours)

**File:** `src/shaping.rs` (lines 1-200)

- [ ] Create TextShaper struct
  ```rust
  pub struct TextShaper {
      // No state needed - HarfRust is stateless
  }

  impl TextShaper {
      pub fn new() -> Self {
          TextShaper {}
      }

      pub fn shape(
          &self,
          font: &FontRef,
          text: &str,
          font_size: f32,
      ) -> Result<ShapedText, Error> {
          // Implementation below
      }
  }
  ```

- [ ] Implement text shaping with HarfRust
  ```rust
  pub fn shape(
      &self,
      font: &FontRef,
      text: &str,
      font_size: f32,
  ) -> Result<ShapedText, Error> {
      use harfbuzz_rs::*;

      // Create HarfBuzz font from skrifa FontRef
      let hb_font = Font::from_bytes(font.data());
      let hb_face = hb_font.face();

      // Create buffer and add text
      let mut buffer = UnicodeBuffer::new();
      buffer.push_str(text);
      buffer.set_direction(Direction::LeftToRight);
      buffer.set_script(Script::Latin);  // TODO: detect from text or use param
      buffer.set_language(Language::from_str("en"));

      // Shape
      let glyph_buffer = harfbuzz_rs::shape(&hb_font, buffer, &[]);

      // Extract glyph positions
      let positions = glyph_buffer.glyph_positions();
      let infos = glyph_buffer.glyph_infos();

      let glyphs: Vec<ShapedGlyph> = infos.iter()
          .zip(positions.iter())
          .map(|(info, pos)| ShapedGlyph {
              glyph_id: info.codepoint,
              x_advance: pos.x_advance as f32,
              y_advance: pos.y_advance as f32,
              x_offset: pos.x_offset as f32,
              y_offset: pos.y_offset as f32,
          })
          .collect();

      Ok(ShapedText { glyphs, font_size })
  }
  ```

### Task 2: Shaping Data Structures (2 hours)

**File:** `src/shaping.rs` (lines 200-300)

- [ ] Define ShapedGlyph struct
  ```rust
  #[derive(Debug, Clone)]
  pub struct ShapedGlyph {
      pub glyph_id: u32,
      pub x_advance: f32,
      pub y_advance: f32,
      pub x_offset: f32,
      pub y_offset: f32,
  }
  ```

- [ ] Define ShapedText struct
  ```rust
  #[derive(Debug, Clone)]
  pub struct ShapedText {
      pub glyphs: Vec<ShapedGlyph>,
      pub font_size: f32,
  }

  impl ShapedText {
      pub fn total_advance_width(&self) -> f32 {
          self.glyphs.iter().map(|g| g.x_advance).sum()
      }

      pub fn bounding_box(&self) -> (f32, f32, f32, f32) {
          // Calculate min/max x/y from all glyph positions
          // Return (min_x, min_y, max_x, max_y)
      }
  }
  ```

### Task 3: Empty String Handling (2 hours)

**File:** `src/shaping.rs` (lines 300-350)

- [ ] Handle empty string input
  ```rust
  pub fn shape(...) -> Result<ShapedText, Error> {
      if text.is_empty() {
          return Ok(ShapedText {
              glyphs: vec![],
              font_size,
          });
      }
      // ... normal shaping
  }
  ```

- [ ] Handle whitespace-only strings
  - [ ] Return shaped result (HarfBuzz handles this)
  - [ ] Verify glyphs have proper advances

### Task 4: Single-Glyph Optimization (4 hours)

**File:** `src/shaping.rs` (lines 350-450)

- [ ] Detect single-character text (FontSimi's common case)
  ```rust
  pub fn shape_fast_path(...) -> Result<ShapedText, Error> {
      if text.chars().count() == 1 {
          // Fast path: skip complex shaping
          let ch = text.chars().next().unwrap();
          let glyph_id = font.charmap().map(ch)?;
          let advance = font.glyph_metrics(glyph_id).advance_width;

          return Ok(ShapedText {
              glyphs: vec![ShapedGlyph {
                  glyph_id,
                  x_advance: advance as f32,
                  y_advance: 0.0,
                  x_offset: 0.0,
                  y_offset: 0.0,
              }],
              font_size,
          });
      }
      // ... normal shaping
  }
  ```

- [ ] Benchmark fast path vs normal shaping
  - [ ] Expect 5-10× speedup for single glyphs
  - [ ] Critical for FontSimi's daidot analysis (52 single glyphs per segment)

### Task 5: Unit Tests (4 hours)

**File:** `tests/shaping_tests.rs` (new file)

- [ ] Test shaping "Hello World"
  - [ ] Verify 11 glyphs returned (including space)
  - [ ] Verify advance widths are positive
  - [ ] Verify glyph IDs are valid

- [ ] Test shaping empty string
  - [ ] Verify returns empty glyphs array
  - [ ] Verify no crash or error

- [ ] Test shaping single character "a"
  - [ ] Verify fast path used (via timing or debug flag)
  - [ ] Verify result identical to normal shaping
  - [ ] Verify glyph ID matches charmap

- [ ] Test shaping complex scripts
  - [ ] Arabic: "مرحبا" (right-to-left, ligatures)
  - [ ] Verify glyph ordering
  - [ ] Verify ligature formation

**Estimated Time:** 20-24 hours (2.5-3 days)

---

## H2.4: Implement Glyph Rasterization (3-4 days) ⚡ CRITICAL

**Files:** `src/rasterize.rs`, `src/orchestrator.rs`

**Goal:** Rasterize glyphs using skrifa + zeno and composite onto canvas

### Task 1: Glyph Outline Extraction (6 hours)

**File:** `src/rasterize.rs` (lines 1-200)

- [ ] Create GlyphRasterizer struct
  ```rust
  pub struct GlyphRasterizer {
      // Configuration
  }

  impl GlyphRasterizer {
      pub fn new() -> Self {
          GlyphRasterizer {}
      }
  }
  ```

- [ ] Extract glyph outlines using skrifa
  ```rust
  pub fn extract_outline(
      &self,
      font: &FontRef,
      glyph_id: u32,
      font_size: f32,
  ) -> Result<Vec<PathElement>, Error> {
      use skrifa::outline::{DrawSettings, OutlinePen};

      let settings = DrawSettings::unhinted(font_size, LocationRef::default());
      let mut pen = ZenoPen::new();

      font.outline_glyphs()
          .get(GlyphId::new(glyph_id as u16))
          .ok_or(Error::GlyphNotFound(glyph_id))?
          .draw(settings, &mut pen)?;

      Ok(pen.finish())
  }
  ```

- [ ] Create ZenoPen adapter for skrifa's OutlinePen trait
  ```rust
  struct ZenoPen {
      path: Vec<PathElement>,
      current_point: (f32, f32),
  }

  impl OutlinePen for ZenoPen {
      fn move_to(&mut self, x: f32, y: f32) {
          self.path.push(PathElement::MoveTo { x, y });
          self.current_point = (x, y);
      }

      fn line_to(&mut self, x: f32, y: f32) {
          self.path.push(PathElement::LineTo { x, y });
          self.current_point = (x, y);
      }

      fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
          self.path.push(PathElement::QuadTo { cx, cy, x, y });
          self.current_point = (x, y);
      }

      fn curve_to(&mut self, cx1: f32, cy1: f32, cx2: f32, cy2: f32, x: f32, y: f32) {
          self.path.push(PathElement::CubicTo { cx1, cy1, cx2, cy2, x, y });
          self.current_point = (x, y);
      }

      fn close(&mut self) {
          self.path.push(PathElement::Close);
      }
  }
  ```

### Task 2: Zeno Rasterization (8 hours)

**File:** `src/rasterize.rs` (lines 200-400)

- [ ] Rasterize path using zeno
  ```rust
  pub fn rasterize_path(
      &self,
      path: &[PathElement],
      width: u32,
      height: u32,
  ) -> Result<Vec<u8>, Error> {
      use zeno::{Mask, PathBuilder, Transform};

      // Build zeno path
      let mut builder = PathBuilder::new();
      for element in path {
          match element {
              PathElement::MoveTo { x, y } => builder.move_to(*x, *y),
              PathElement::LineTo { x, y } => builder.line_to(*x, *y),
              PathElement::QuadTo { cx, cy, x, y } => builder.quad_to(*cx, *cy, *x, *y),
              PathElement::CubicTo { cx1, cy1, cx2, cy2, x, y } => {
                  builder.cubic_to(*cx1, *cy1, *cx2, *cy2, *x, *y)
              }
              PathElement::Close => builder.close(),
          }
      }
      let path = builder.finish();

      // Rasterize with 256x antialiasing (8-bit alpha)
      let mut mask = Mask::new(width as usize, height as usize);
      mask.fill(&path, Transform::identity());

      // Convert to grayscale image (8-bit)
      let pixels = mask.as_slice()
          .iter()
          .map(|alpha| *alpha)
          .collect();

      Ok(pixels)
  }
  ```

- [ ] Handle empty paths gracefully
  - [ ] Return blank image (all zeros)
  - [ ] Don't crash or error

### Task 3: Glyph Compositing (10 hours)

**File:** `src/rasterize.rs` (lines 400-700)

- [ ] Create canvas for full text rendering
  ```rust
  pub fn render_text(
      &self,
      font: &FontRef,
      shaped: &ShapedText,
      width: u32,
      height: u32,
  ) -> Result<Vec<u8>, Error> {
      let mut canvas = vec![0u8; (width * height) as usize];

      let mut x_pos = 0.0f32;
      let baseline_y = height as f32 * 0.75;  // Place baseline at 75% height

      for glyph in &shaped.glyphs {
          // Extract and rasterize glyph
          let outline = self.extract_outline(font, glyph.glyph_id, shaped.font_size)?;
          let glyph_pixels = self.rasterize_path(&outline, width, height)?;

          // Calculate glyph position
          let glyph_x = (x_pos + glyph.x_offset) as i32;
          let glyph_y = (baseline_y + glyph.y_offset) as i32;

          // Composite glyph onto canvas
          self.composite_glyph(
              &mut canvas,
              &glyph_pixels,
              glyph_x,
              glyph_y,
              width,
              height,
          )?;

          x_pos += glyph.x_advance;
      }

      Ok(canvas)
  }
  ```

- [ ] Implement glyph compositing with alpha blending
  ```rust
  fn composite_glyph(
      &self,
      canvas: &mut [u8],
      glyph: &[u8],
      x: i32,
      y: i32,
      canvas_width: u32,
      canvas_height: u32,
  ) -> Result<(), Error> {
      // Composite with alpha blending
      for gy in 0..canvas_height {
          for gx in 0..canvas_width {
              let canvas_x = x + gx as i32;
              let canvas_y = y + gy as i32;

              if canvas_x < 0 || canvas_y < 0
                  || canvas_x >= canvas_width as i32
                  || canvas_y >= canvas_height as i32 {
                  continue;
              }

              let glyph_idx = (gy * canvas_width + gx) as usize;
              let canvas_idx = (canvas_y as u32 * canvas_width + canvas_x as u32) as usize;

              let src_alpha = glyph[glyph_idx];
              let dst = canvas[canvas_idx];

              // Alpha blending: dst + src * (1 - dst_alpha/255)
              let blended = dst.saturating_add(
                  ((src_alpha as u16 * (255 - dst) as u16) / 255) as u8
              );
              canvas[canvas_idx] = blended;
          }
      }

      Ok(())
  }
  ```

### Task 4: Tracking Support (4 hours)

**File:** `src/rasterize.rs` (lines 700-800)

- [ ] Add tracking parameter to render_text()
  ```rust
  pub fn render_text(
      &self,
      font: &FontRef,
      shaped: &ShapedText,
      width: u32,
      height: u32,
      tracking: f32,  // Additional spacing in ems
  ) -> Result<Vec<u8>, Error> {
      // ... existing code ...

      for glyph in &shaped.glyphs {
          // ... existing code ...

          x_pos += glyph.x_advance + (tracking * shaped.font_size);
      }

      Ok(canvas)
  }
  ```

### Task 5: Unit Tests (4 hours)

**File:** `tests/rasterization_tests.rs` (new file)

- [ ] Test single glyph rendering
  - [ ] Render "A" at 100pt
  - [ ] Verify image is not all zeros
  - [ ] Verify image dimensions correct

- [ ] Test multi-glyph rendering
  - [ ] Render "Hello" at 50pt
  - [ ] Verify glyphs properly spaced
  - [ ] Verify no overlapping artifacts

- [ ] Test empty string
  - [ ] Render "" at any size
  - [ ] Verify returns blank image (all zeros)
  - [ ] Verify no crash

- [ ] Test tracking
  - [ ] Render "AB" with tracking=0.0
  - [ ] Render "AB" with tracking=0.1
  - [ ] Verify second image has wider spacing

**Estimated Time:** 32-36 hours (4-4.5 days)

---

## H2.5: Implement PGM Output Format (1-2 days)

**Files:** `src/output.rs` (new file), `src/orchestrator.rs`

**Goal:** Generate PGM P5 binary format and base64-encode for JSONL output

### Task 1: PGM Format Writer (6 hours)

**File:** `src/output.rs` (lines 1-200)

- [ ] Create PGM writer
  ```rust
  pub struct PgmWriter;

  impl PgmWriter {
      pub fn write_pgm_binary(
          pixels: &[u8],
          width: u32,
          height: u32,
      ) -> Result<Vec<u8>, Error> {
          let mut output = Vec::new();

          // PGM P5 header
          writeln!(&mut output, "P5")?;
          writeln!(&mut output, "{} {}", width, height)?;
          writeln!(&mut output, "255")?;

          // Binary pixel data
          output.extend_from_slice(pixels);

          Ok(output)
      }
  }
  ```

- [ ] Validate PGM format correctness
  - [ ] Header format matches specification
  - [ ] Pixel data matches declared dimensions
  - [ ] All pixels in valid range [0, 255]

### Task 2: Base64 Encoding (4 hours)

**File:** `src/output.rs` (lines 200-300)

- [ ] Implement base64 encoding
  ```rust
  use base64::{engine::general_purpose, Engine as _};

  pub fn encode_pgm_base64(pgm_data: &[u8]) -> String {
      general_purpose::STANDARD.encode(pgm_data)
  }
  ```

- [ ] Add size optimization
  - [ ] Compress PGM data before base64 encoding (optional)
  - [ ] Compare size: raw PGM vs compressed
  - [ ] Document tradeoffs

### Task 3: Bounding Box Calculation (4 hours)

**File:** `src/output.rs` (lines 300-400)

- [ ] Calculate actual rendered bounding box
  ```rust
  pub fn calculate_bbox(pixels: &[u8], width: u32, height: u32) -> (u32, u32, u32, u32) {
      let mut min_x = width;
      let mut min_y = height;
      let mut max_x = 0u32;
      let mut max_y = 0u32;

      for y in 0..height {
          for x in 0..width {
              let idx = (y * width + x) as usize;
              if pixels[idx] > 0 {
                  min_x = min_x.min(x);
                  min_y = min_y.min(y);
                  max_x = max_x.max(x);
                  max_y = max_y.max(y);
              }
          }
      }

      if min_x > max_x {
          // All pixels are zero (blank image)
          return (0, 0, 0, 0);
      }

      (min_x, min_y, max_x - min_x + 1, max_y - min_y + 1)
  }
  ```

### Task 4: Unit Tests (2 hours)

**File:** `tests/output_tests.rs` (new file)

- [ ] Test PGM format generation
  - [ ] Generate PGM from test pixel data
  - [ ] Verify header format
  - [ ] Verify pixel data appended correctly

- [ ] Test base64 encoding
  - [ ] Encode small image (10×10)
  - [ ] Decode and verify matches original
  - [ ] Test empty image (all zeros)

- [ ] Test bounding box calculation
  - [ ] Image with content in center
  - [ ] Image with content at edges
  - [ ] Blank image (all zeros)

**Estimated Time:** 16-18 hours (2-2.25 days)

---

## H2.6: Implement JSONL Output (1-2 days)

**Files:** `src/orchestrator.rs`, `src/main.rs`

**Goal:** Format results as JSONL and write to stdout immediately

### Task 1: JobResult Data Structure (4 hours)

**File:** `src/orchestrator.rs` (lines 1-100)

- [ ] Define JobResult struct
  ```rust
  #[derive(Debug, Clone, Serialize)]
  pub struct JobResult {
      pub id: String,
      pub status: String,  // "success" or "error"
      pub rendering: Option<RenderingOutput>,
      pub error: Option<String>,
      pub timing: TimingInfo,
      pub memory: Option<MemoryInfo>,
  }

  #[derive(Debug, Clone, Serialize)]
  pub struct RenderingOutput {
      pub format: String,  // "pgm"
      pub encoding: String,  // "base64"
      pub data: String,  // Base64-encoded PGM
      pub width: u32,
      pub height: u32,
      pub actual_bbox: (u32, u32, u32, u32),  // (x, y, w, h)
  }

  #[derive(Debug, Clone, Serialize)]
  pub struct TimingInfo {
      pub shape_ms: f64,
      pub render_ms: f64,
      pub total_ms: f64,
  }

  #[derive(Debug, Clone, Serialize)]
  pub struct MemoryInfo {
      pub font_cache_mb: f64,
      pub total_mb: f64,
  }
  ```

### Task 2: Result Formatting & Output (6 hours)

**File:** `src/orchestrator.rs` (lines 100-300)

- [ ] Implement JSONL formatting
  ```rust
  pub fn format_job_result(result: &JobResult) -> String {
      serde_json::to_string(result).unwrap()
  }
  ```

- [ ] Write JSONL to stdout with immediate flush
  ```rust
  pub fn write_job_result(result: &JobResult) -> Result<(), Error> {
      let json_line = format_job_result(result);
      let stdout = io::stdout();
      let mut handle = stdout.lock();

      writeln!(handle, "{}", json_line)?;
      handle.flush()?;  // CRITICAL: Flush immediately for streaming

      Ok(())
  }
  ```

- [ ] Handle write errors gracefully
  - [ ] Detect broken pipe (FontSimi closed stdin)
  - [ ] Return error but don't crash
  - [ ] Log error to stderr

### Task 3: Progressive Output During Batch (4 hours)

**File:** `src/orchestrator.rs` (lines 300-500)

- [ ] Output results as jobs complete (not wait for all)
  ```rust
  pub fn process_jobs_streaming(spec: JobSpec) -> Result<(), Error> {
      use rayon::prelude::*;

      // Process in parallel
      let (tx, rx) = std::sync::mpsc::channel();

      std::thread::spawn(move || {
          spec.jobs.par_iter()
              .for_each(|job| {
                  let start = Instant::now();

                  let result = process_single_job(job);

                  tx.send(result).ok();
              });
      });

      // Output results as they arrive
      for result in rx {
          write_job_result(&result)?;
      }

      Ok(())
  }
  ```

### Task 4: Unit Tests (2 hours)

**File:** `tests/jsonl_output_tests.rs` (new file)

- [ ] Test JSONL formatting
  - [ ] Format successful result
  - [ ] Verify valid JSON
  - [ ] Verify ends with newline

- [ ] Test error result formatting
  - [ ] Format error result
  - [ ] Verify status="error"
  - [ ] Verify error message included

- [ ] Test streaming output
  - [ ] Process 10 jobs
  - [ ] Verify 10 JSONL lines written
  - [ ] Verify immediate flush (timing test)

**Estimated Time:** 16-18 hours (2-2.25 days)

---

## H2.7: Error Handling & Edge Cases (1-2 days)

**Files:** All modules

**Goal:** Handle all failure modes gracefully and continue processing

### Task 1: Job-Level Error Handling (6 hours)

**File:** `src/orchestrator.rs` (lines 500-700)

- [ ] Wrap each job processing in error handler
  ```rust
  pub fn process_single_job(job: &Job) -> JobResult {
      let start = Instant::now();

      let result = (|| -> Result<RenderingOutput, Error> {
          // Load font
          let font = MmapFont::new(&job.font.path)?;

          // Apply variations
          let instance = if !job.font.variations.is_empty() {
              font.apply_variations(&job.font.variations)?
          } else {
              font.font_ref.clone()
          };

          // Shape text
          let shaper = TextShaper::new();
          let shaped = shaper.shape(&instance, &job.text.content, job.font.size as f32)?;

          // Rasterize
          let rasterizer = GlyphRasterizer::new();
          let pixels = rasterizer.render_text(
              &instance,
              &shaped,
              job.rendering.width,
              job.rendering.height,
              0.0,  // No tracking
          )?;

          // Generate PGM
          let pgm = PgmWriter::write_pgm_binary(&pixels, job.rendering.width, job.rendering.height)?;
          let base64 = encode_pgm_base64(&pgm);
          let bbox = calculate_bbox(&pixels, job.rendering.width, job.rendering.height);

          Ok(RenderingOutput {
              format: "pgm".to_string(),
              encoding: "base64".to_string(),
              data: base64,
              width: job.rendering.width,
              height: job.rendering.height,
              actual_bbox: bbox,
          })
      })();

      let elapsed = start.elapsed();

      match result {
          Ok(output) => JobResult {
              id: job.id.clone(),
              status: "success".to_string(),
              rendering: Some(output),
              error: None,
              timing: TimingInfo {
                  shape_ms: 0.0,  // TODO: Instrument
                  render_ms: 0.0,
                  total_ms: elapsed.as_secs_f64() * 1000.0,
              },
              memory: None,
          },
          Err(e) => JobResult {
              id: job.id.clone(),
              status: "error".to_string(),
              rendering: None,
              error: Some(format!("{}", e)),
              timing: TimingInfo {
                  shape_ms: 0.0,
                  render_ms: 0.0,
                  total_ms: elapsed.as_secs_f64() * 1000.0,
              },
              memory: None,
          },
      }
  }
  ```

### Task 2: Continue on Failure (2 hours)

**File:** `src/orchestrator.rs` (lines 700-800)

- [ ] Ensure failed jobs don't stop batch processing
  - [ ] Catch all errors in process_single_job()
  - [ ] Return error result (status="error")
  - [ ] Continue processing remaining jobs

- [ ] Log errors to stderr
  - [ ] Include job ID in error message
  - [ ] Include error details
  - [ ] Don't pollute stdout (JSONL stream)

### Task 3: Font Loading Error Handling (4 hours)

**File:** `src/mmap_font.rs` (lines 600-700)

- [ ] Handle missing font files
  - [ ] Return Error::FontNotFound with path
  - [ ] Include helpful message: "Font file not found: /path/to/font.ttf"

- [ ] Handle corrupted font files
  - [ ] Catch read-fonts parsing errors
  - [ ] Return Error::InvalidFont with details
  - [ ] Include which table is corrupted

- [ ] Handle invalid font format
  - [ ] Detect unsupported formats (WOFF2, etc.)
  - [ ] Return Error::UnsupportedFormat
  - [ ] Suggest conversion to TTF/OTF

### Task 4: Unit Tests (4 hours)

**File:** `tests/error_handling_tests.rs` (new file)

- [ ] Test missing font file
  - [ ] Create job with nonexistent path
  - [ ] Verify status="error"
  - [ ] Verify error message includes path

- [ ] Test corrupted font
  - [ ] Create invalid TTF file
  - [ ] Process job
  - [ ] Verify error result returned

- [ ] Test batch with partial failures
  - [ ] Process 10 jobs: 8 valid, 2 invalid
  - [ ] Verify 8 success results
  - [ ] Verify 2 error results
  - [ ] Verify all 10 results written to stdout

**Estimated Time:** 16-18 hours (2-2.25 days)

---

## H2 Testing & Integration (2-3 days) ⚡ FINAL VALIDATION

**Files:** `tests/integration_tests.rs` (new file)

**Goal:** End-to-end testing of complete rendering pipeline

### Task 1: Integration Tests (8 hours)

- [ ] Test complete pipeline: JSON → render → JSONL
  - [ ] Single job with static font
  - [ ] Single job with variable font + coordinates
  - [ ] Batch of 100 jobs

- [ ] Test against real fonts from test-fonts/
  - [ ] Arial-Black.ttf (static)
  - [ ] Playfair[opsz,wdth,wght].ttf (variable)
  - [ ] Verify rendered images are non-blank

- [ ] Test all error paths
  - [ ] Missing font file
  - [ ] Invalid JSON
  - [ ] Empty text
  - [ ] Out-of-bounds variations

### Task 2: FontSimi Compatibility Tests (6 hours)

- [ ] Test exact FontSimi job format
  - [ ] Use real job spec from HaforuRenderer
  - [ ] Verify JSONL output matches expected format
  - [ ] Verify base64 PGM can be decoded by Python

- [ ] Performance baseline
  - [ ] Single render: <100ms total
  - [ ] Batch of 1000: <10s total
  - [ ] Memory usage: <500MB for 1000 renders

### Task 3: Manual Testing (4 hours)

- [ ] Build release binary: `cargo build --release`
- [ ] Test from command line:
  ```bash
  echo '{"version":"1.0","jobs":[{"id":"test1","font":{"path":"test-fonts/Arial-Black.ttf","size":1000},"text":{"content":"A"},"rendering":{"format":"pgm","encoding":"binary","width":3000,"height":1200}}]}' | ./target/release/haforu --batch
  ```

- [ ] Verify JSONL output
- [ ] Decode base64 PGM and inspect visually

**Estimated Time:** 18-24 hours (2.25-3 days)

---

## H2 Summary & Timeline

**Total Estimated Time:** 12-18 days for complete H2 implementation

**Task Breakdown:**
- H2.1: JSON Job Processing (1.5 days)
- H2.2: Font Loading with Variations (2.5-3 days)
- H2.3: Text Shaping (2.5-3 days)
- H2.4: Glyph Rasterization (4-4.5 days)
- H2.5: PGM Output Format (2-2.25 days)
- H2.6: JSONL Output (2-2.25 days)
- H2.7: Error Handling (2-2.25 days)
- H2 Integration Testing (2.25-3 days)

**Critical Path:** H2.1 → H2.2 → H2.3 → H2.4 → H2.5 → H2.6 → H2.7 → Integration

**Dependencies:**
- All tasks sequential (each builds on previous)
- Integration testing requires H2.1-H2.7 complete

**Success Criteria:**
- [ ] Single render completes <100ms
- [ ] Batch of 1000 renders completes <10s
- [ ] Memory usage <500MB for 1000 renders
- [ ] JSONL output matches FontSimi expected format
- [ ] All error paths handled gracefully
- [ ] Zero crashes on valid or invalid input

---

## H4: Streaming Mode (After FontSimi H3 Complete)

**Goal:** Keep process alive for continuous job processing during deep matching

**Status:** DEPRIORITIZED until H2 complete and H3 (FontSimi batch pipeline) complete

### H4.1: Streaming Mode Implementation (2-3 days)

**Files:** `src/main.rs`, `src/orchestrator.rs`

- [ ] Add `--streaming` flag to CLI
- [ ] Keep stdin/stdout open for continuous jobs
- [ ] Read jobs line-by-line from stdin (JSONL format)
- [ ] Write results to stdout immediately (one line per job)
- [ ] Maintain font cache across all jobs in session
- [ ] Handle EOF gracefully (keep running until explicit exit)

---

## DEPRIORITIZED TASKS (Postponed Until H2-H5 Complete)

### Phase 2-6: Advanced Features
- [ ] GPU rendering with Vello
- [ ] Python bindings (PyO3)
- [ ] Storage backend (packfiles)
- [ ] Pre-rendering support
- [ ] Performance optimizations beyond H2

**Do NOT work on these until H2 is COMPLETE and validated by FontSimi.**

---

## IMMEDIATE NEXT STEPS ⚡

**Current Status:** Foundation complete. START H2.1 NOW.

**Next Actions:**
1. **START HERE:** Begin H2.1 JSON job processing (1.5 days)
2. Complete H2.2-H2.7 sequentially
3. Run integration tests
4. Signal FontSimi team: "H2 complete, ready for validation"
5. Wait for FontSimi validation results
6. Fix any issues found during validation
7. Move to H4 streaming mode (H3 is Python-only)

**Estimated Timeline:** 12-18 days to H2 complete + 4 days FontSimi validation = 16-22 days total

**Success Metric:** FontSimi Python integration tests pass with Haforu rendering.
