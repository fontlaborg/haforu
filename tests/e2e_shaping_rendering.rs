// this_file: tests/e2e_shaping_rendering.rs
//! End-to-end tests: shape text with HarfRust and rasterize with skrifa+zeno

use haforu::json_parser::ShapingOptions;
use haforu::rasterize::CpuRasterizer;
use haforu::shaping::TextShaper;
use haforu::storage::StorageManager;
use haforu::{FontLoader, Result};
use read_fonts::FontRef;

fn load_font_bytes(path: &str) -> Result<Vec<u8>> {
    let mut loader = FontLoader::new();
    let data = loader.load_font_data(path)?;
    Ok((*data).clone())
}

#[test]
fn test_shape_and_render_archivo_hello_line() {
    // Arrange
    let font_path = "03fonts/Archivo[wdth,wght].ttf";
    let font_bytes = load_font_bytes(font_path).expect("load font bytes");
    let font_ref = FontRef::new(&font_bytes).expect("font ref");

    let mut shaper = TextShaper::new();
    let opts = ShapingOptions::default();
    let text = "Hello";
    let size = 24.0;

    // Act: shape
    let shaped = shaper
        .shape(&font_bytes, text, size, &opts)
        .expect("shape text");
    assert!(shaped.glyphs.len() > 0, "expected some glyphs for 'Hello'");

    // Act: rasterize glyphs and compose line
    let raster = CpuRasterizer::new(size);
    let glyphs = raster
        .render_shaped_glyphs(&font_ref, &shaped.glyphs, size)
        .expect("render glyphs");
    let (bitmap, width, height) = raster
        .render_text_line(&font_ref, &glyphs)
        .expect("compose line");

    // Assert
    assert!(
        width > 0 && height > 0,
        "image dimensions should be non-zero"
    );
    let sum: u64 = bitmap.iter().map(|&v| v as u64).sum();
    assert!(sum > 0, "bitmap should contain non-zero alpha values");
}

#[test]
fn test_shape_devanagari_basic() {
    // Arrange
    let font_path = "03fonts/AnekDevanagari[wdth,wght].ttf";
    let font_bytes = load_font_bytes(font_path).expect("load font bytes");
    let mut shaper = TextShaper::new();

    let mut opts = ShapingOptions::default();
    opts.script = Some("Deva".to_string());
    opts.direction = "ltr".to_string();

    let text = "नमस्ते"; // "Namaste"

    // Act
    let shaped = shaper
        .shape(&font_bytes, text, 20.0, &opts)
        .expect("shape devanagari");

    // Assert
    assert!(
        shaped.glyphs.len() > 0,
        "expected glyphs for Devanagari text"
    );
}

#[test]
fn test_storage_roundtrip_with_render() {
    use tempfile::tempdir;

    // Arrange
    let font_path = "03fonts/Archivo[wdth,wght].ttf";
    let font_bytes = load_font_bytes(font_path).expect("load font bytes");
    let font_ref = FontRef::new(&font_bytes).expect("font ref");
    let mut shaper = TextShaper::new();
    let opts = ShapingOptions::default();
    let size = 18.0;

    let shaped = shaper
        .shape(&font_bytes, "Test", size, &opts)
        .expect("shape text");
    let raster = CpuRasterizer::new(size);
    let glyphs = raster
        .render_shaped_glyphs(&font_ref, &shaped.glyphs, size)
        .expect("render glyphs");
    let (bitmap, width, height) = raster
        .render_text_line(&font_ref, &glyphs)
        .expect("compose line");

    // Store and retrieve via storage manager
    let dir = tempdir().expect("tempdir");
    let mut storage = StorageManager::new(dir.path(), 100).expect("storage manager");
    let id = storage
        .store_image(&bitmap, width as u16, height as u16)
        .expect("store image");
    storage.finalize().expect("finalize shard");
    let roundtrip = storage.get_image(&id).expect("retrieve image");

    assert_eq!(roundtrip, bitmap, "stored and retrieved data must match");
}
