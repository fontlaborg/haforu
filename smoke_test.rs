// Quick smoke test with a real font
use std::collections::HashMap;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Use Arial-Black from test-fonts
    let font_path = PathBuf::from("../test-fonts/fonts-breakable/Arial-Black.ttf");
    
    if !font_path.exists() {
        eprintln!("Font not found: {}", font_path.display());
        return Err("Font file not found".into());
    }
    
    println!("Testing with font: {}", font_path.display());
    
    // Load the font
    let loader = haforu2::fonts::FontLoader::new(10);
    let coords = HashMap::new(); // No variation coordinates for static font
    
    let font_instance = loader.load_font(
        &camino::Utf8PathBuf::from_path_buf(font_path.clone()).unwrap(),
        &coords
    )?;
    
    println!("✓ Font loaded successfully");
    
    // Shape a simple character
    let shaper = haforu2::shaping::TextShaper::new();
    let shaped = shaper.shape(
        &font_instance,
        "A",
        100.0,  // 100pt
        &font_path
    )?;
    
    println!("✓ Text shaped: {} glyphs", shaped.glyphs.len());
    
    // Render it
    let rasterizer = haforu2::render::GlyphRasterizer::new();
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
    let pgm = haforu2::output::ImageOutput::write_pgm_binary(&pixels, 800, 600)?;
    println!("✓ Generated PGM ({} bytes)", pgm.len());
    
    // Encode to base64
    let b64 = haforu2::output::ImageOutput::encode_base64(&pgm);
    println!("✓ Base64 encoded ({} chars)", b64.len());
    
    println!("\n✅ All steps completed successfully!");
    println!("   Font: Arial-Black");
    println!("   Character: A");
    println!("   Size: 100pt");
    println!("   Canvas: 800×600");
    
    Ok(())
}
