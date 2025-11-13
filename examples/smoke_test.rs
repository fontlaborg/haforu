// this_file: external/haforu2/examples/smoke_test.rs
//
// Quick smoke test with a real font file

use camino::Utf8PathBuf;
use haforu2::fonts::FontLoader;
use haforu2::output::ImageOutput;
use haforu2::render::GlyphRasterizer;
use haforu2::shaping::TextShaper;
use std::collections::HashMap;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Use absolute path to fontsimi's test font (since haforu2 is in separate repo)
    let font_path = PathBuf::from("/Users/adam/Developer/vcs/github.docrepair-fonts/fontsimi/micro/fonts/originals/Arial-Black.ttf");

    if !font_path.exists() {
        eprintln!("Font not found at: {}", font_path.display());
        eprintln!("This smoke test requires Arial-Black.ttf from the fontsimi project");
        return Err("Font file not found - please ensure fontsimi/micro/fonts/originals/Arial-Black.ttf exists".into());
    }

    println!("Testing with font: {}", font_path.display());

    // Load the font
    let loader = FontLoader::new(10);
    let coords = HashMap::new(); // No variation coordinates for static font

    let font_instance = loader.load_font(
        &Utf8PathBuf::from_path_buf(font_path.clone()).unwrap(),
        &coords
    )?;

    println!("✓ Font loaded successfully");

    // Shape a simple character
    let shaper = TextShaper::new();
    let shaped = shaper.shape(
        &font_instance,
        "A",
        100.0,  // 100pt
        &font_path
    )?;

    println!("✓ Text shaped: {} glyphs", shaped.glyphs.len());

    // Render it
    let rasterizer = GlyphRasterizer::new();
    let pixels = rasterizer.render_text(
        &font_instance,
        &shaped,
        800,    // width
        600,    // height
        0.0,    // tracking
        &font_path
    )?;

    println!("✓ Rendered {} pixels", pixels.len());

    // Generate PGM
    let pgm = ImageOutput::write_pgm_binary(&pixels, 800, 600)?;
    println!("✓ Generated PGM ({} bytes)", pgm.len());

    // Encode to base64
    let b64 = ImageOutput::encode_base64(&pgm);
    println!("✓ Base64 encoded ({} chars)", b64.len());

    println!("\n✅ All steps completed successfully!");
    println!("   Font: Arial-Black");
    println!("   Character: A");
    println!("   Size: 100pt");
    println!("   Canvas: 800×600");

    Ok(())
}
