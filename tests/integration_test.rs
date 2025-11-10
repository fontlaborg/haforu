// this_file: tests/integration_test.rs
//! Integration tests for haforu using real font files

use haforu::{Error, FontLoader, json_parser};
use std::path::PathBuf;

/// Helper to get test font path
fn get_test_font_path(font_name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("03fonts")
        .join(font_name)
}

#[test]
fn test_load_archivo_variable_font() {
    let mut loader = FontLoader::new();
    let font_path = get_test_font_path("Archivo[wdth,wght].ttf");

    if !font_path.exists() {
        eprintln!("Skipping test: font file not found at {:?}", font_path);
        return;
    }

    let font_data = loader.load_font_data(&font_path).unwrap();
    assert!(!font_data.is_empty());

    // Verify we can create a FontRef from the data
    let font_ref = loader.create_font_ref(&font_data).unwrap();

    // Basic metadata checks
    assert!(font_ref.table_directory.num_tables() > 0);
}

#[test]
fn test_load_merriweather_variable_font() {
    let mut loader = FontLoader::new();
    let font_path = get_test_font_path("Merriweather[opsz,wdth,wght].ttf");

    if !font_path.exists() {
        eprintln!("Skipping test: font file not found at {:?}", font_path);
        return;
    }

    let font_data = loader.load_font_data(&font_path).unwrap();
    assert!(!font_data.is_empty());

    // Verify the font can be parsed
    let _font_ref = loader.create_font_ref(&font_data).unwrap();
}

#[test]
fn test_font_caching() {
    let mut loader = FontLoader::with_cache_size(10 * 1024 * 1024); // 10MB cache
    let font_path = get_test_font_path("Archivo[wdth,wght].ttf");

    if !font_path.exists() {
        eprintln!("Skipping test: font file not found at {:?}", font_path);
        return;
    }

    // First load
    assert_eq!(loader.cache_count(), 0);
    let _font_data1 = loader.load_font_data(&font_path).unwrap();
    assert_eq!(loader.cache_count(), 1);

    // Second load should use cache
    let initial_size = loader.cache_size();
    let _font_data2 = loader.load_font_data(&font_path).unwrap();
    assert_eq!(loader.cache_count(), 1);
    assert_eq!(loader.cache_size(), initial_size); // Size shouldn't change
}

#[test]
fn test_json_spec_with_real_font_paths() {
    let json = format!(
        r#"{{
        "version": "1.0.0",
        "jobs": [{{
            "id": "test_job",
            "font": {{
                "path": "03fonts/Archivo[wdth,wght].ttf",
                "variations": [
                    {{"tag": "wght", "value": 400}},
                    {{"tag": "wdth", "value": 100}}
                ]
            }},
            "text": "Test Text",
            "size": 24.0
        }}],
        "storage": {{
            "backend": "filesystem",
            "output_path": "./test_output"
        }}
    }}"#
    );

    let spec = json_parser::parse_job_spec(&json).unwrap();
    assert_eq!(spec.jobs.len(), 1);
    assert_eq!(spec.jobs[0].font.path, "03fonts/Archivo[wdth,wght].ttf");

    let variations = spec.jobs[0].font.variations.as_ref().unwrap();
    assert_eq!(variations.len(), 2);
    assert_eq!(variations[0].tag, "wght");
    assert_eq!(variations[0].value, 400.0);
}

#[test]
fn test_multiple_fonts_in_job_spec() {
    let json = r#"{
        "version": "1.0.0",
        "jobs": [
            {
                "id": "archivo_job",
                "font": {"path": "03fonts/Archivo[wdth,wght].ttf"},
                "text": "Archivo Test",
                "size": 16.0
            },
            {
                "id": "merriweather_job",
                "font": {"path": "03fonts/Merriweather[opsz,wdth,wght].ttf"},
                "text": "Merriweather Test",
                "size": 18.0
            },
            {
                "id": "playfair_job",
                "font": {"path": "03fonts/Playfair[opsz,wdth,wght].ttf"},
                "text": "Playfair Test",
                "size": 20.0
            }
        ],
        "storage": {"backend": "memory"}
    }"#;

    let spec = json_parser::parse_job_spec(json).unwrap();
    assert_eq!(spec.jobs.len(), 3);

    // Verify each job
    assert_eq!(spec.jobs[0].id, "archivo_job");
    assert_eq!(spec.jobs[1].id, "merriweather_job");
    assert_eq!(spec.jobs[2].id, "playfair_job");

    assert_eq!(spec.jobs[0].size, 16.0);
    assert_eq!(spec.jobs[1].size, 18.0);
    assert_eq!(spec.jobs[2].size, 20.0);
}

#[test]
fn test_invalid_font_path_handling() {
    let mut loader = FontLoader::new();
    let result = loader.load_font_data("nonexistent/font.ttf");

    assert!(result.is_err());
    match result.unwrap_err() {
        Error::Font(msg) => assert!(msg.contains("Failed to resolve path")),
        _ => panic!("Expected Font error"),
    }
}

#[test]
fn test_font_validation_with_all_test_fonts() {
    let test_fonts = [
        "AnekDevanagari[wdth,wght].ttf",
        "Archivo[wdth,wght].ttf",
        "IBMPlexSans-Italic[wdth,wght].ttf",
        "Merriweather[opsz,wdth,wght].ttf",
        "Playfair-Italic[opsz,wdth,wght].ttf",
        "Playfair[opsz,wdth,wght].ttf",
    ];

    let mut loader = FontLoader::new();
    let mut loaded_count = 0;

    for font_name in &test_fonts {
        let font_path = get_test_font_path(font_name);

        if !font_path.exists() {
            eprintln!("Font not found: {:?}", font_path);
            continue;
        }

        match loader.load_font_data(&font_path) {
            Ok(font_data) => {
                assert!(
                    !font_data.is_empty(),
                    "Font data should not be empty for {}",
                    font_name
                );

                // Verify we can parse it
                let font_ref = loader.create_font_ref(&font_data).unwrap();
                assert!(
                    font_ref.table_directory.num_tables() > 0,
                    "Font should have tables: {}",
                    font_name
                );

                loaded_count += 1;
                println!("Successfully loaded: {}", font_name);
            }
            Err(e) => {
                panic!("Failed to load font {}: {}", font_name, e);
            }
        }
    }

    // Ensure we tested at least some fonts
    assert!(loaded_count > 0, "No fonts were loaded in the test");
}
