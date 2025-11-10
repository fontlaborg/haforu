// this_file: examples/shape_and_render.rs
//! Example: Shape and render a line of text to a simple PGM image
//!
//! Usage:
//!   cargo run --example shape_and_render
//!
//! Optional env vars:
//!   HAFORU_EXAMPLE_FONT=/path/to/font.ttf
//!   HAFORU_EXAMPLE_TEXT="Your text here"

use haforu::{FontLoader};
use haforu::json_parser::ShapingOptions;
use haforu::rasterize::CpuRasterizer;
use haforu::shaping::TextShaper;
use read_fonts::FontRef;
use std::env;
use std::fs::File;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let font_path = env::var("HAFORU_EXAMPLE_FONT")
        .unwrap_or_else(|_| "03fonts/Archivo[wdth,wght].ttf".to_string());
    let text = env::var("HAFORU_EXAMPLE_TEXT")
        .unwrap_or_else(|_| "Hello, Haforu!".to_string());
    let size = 24.0;

    // Load font
    let mut loader = FontLoader::new();
    let font_bytes = loader.load_font_data(&font_path)?;
    let font_ref = FontRef::new(&font_bytes)?;

    // Shape text
    let mut shaper = TextShaper::new();
    let shaped = shaper.shape(&font_bytes, &text, size, &ShapingOptions::default())?;

    // Rasterize
    let raster = CpuRasterizer::new(size);
    let glyphs = raster.render_shaped_glyphs(&font_ref, &shaped.glyphs, size)?;
    let (bitmap, width, height) = raster.render_text_line(&font_ref, &glyphs)?;

    // Write PGM (portable graymap) for quick viewing
    let out_path = "./example_output.pgm";
    let mut f = File::create(out_path)?;
    // PGM header (plain)
    writeln!(f, "P2")?;
    writeln!(f, "{} {}", width, height)?;
    writeln!(f, "255")?;
    // Write pixels
    for (i, px) in bitmap.iter().enumerate() {
        write!(f, "{} ", px)?;
        if (i + 1) % (width as usize) == 0 {
            writeln!(f)?;
        }
    }
    f.flush()?;

    println!("Wrote {} ({}x{})", out_path, width, height);
    Ok(())
}

