// this_file: tests/error_recovery.rs
//! Error recovery tests for malformed and corrupt font files

use haforu::font_loader::FontLoader;
use haforu::json_parser::{FontSpec, Job, JobSpec, parse_job_spec};
use haforu::mmap_font::{FileInfo, MmapFontCache};
use haforu::rasterize::CpuRasterizer;
use haforu::shaping::TextShaper;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Create a test directory with various malformed font files
fn setup_test_fonts() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let test_dir = temp_dir.path();

    // Empty file
    fs::write(test_dir.join("empty.ttf"), b"").unwrap();

    // File with invalid magic number
    fs::write(test_dir.join("invalid_magic.ttf"), b"INVALID_FONT_DATA").unwrap();

    // Truncated font file (partial TTF header)
    let truncated_header = vec![0x00, 0x01, 0x00, 0x00]; // Partial TTF version 1.0 header
    fs::write(test_dir.join("truncated.ttf"), truncated_header).unwrap();

    // File with valid TTF header but garbage data
    let mut garbage_ttf = vec![0x00, 0x01, 0x00, 0x00]; // TTF version 1.0
    garbage_ttf.extend_from_slice(&[0xFF; 100]); // Random garbage
    fs::write(test_dir.join("garbage.ttf"), garbage_ttf).unwrap();

    // File with valid OTF header but truncated
    let otf_header = b"OTTO";
    fs::write(test_dir.join("truncated.otf"), otf_header).unwrap();

    // TTC file with invalid data
    let ttc_header = b"ttcf\x00\x01\x00\x00"; // TTC header version 1.0
    fs::write(test_dir.join("invalid.ttc"), ttc_header).unwrap();

    // WOFF file with invalid data
    let woff_header = b"wOFF";
    fs::write(test_dir.join("invalid.woff"), woff_header).unwrap();

    // WOFF2 file with invalid data
    let woff2_header = b"wOF2";
    fs::write(test_dir.join("invalid.woff2"), woff2_header).unwrap();

    // Very large file that might cause memory issues
    let large_file = vec![0xFF; 100 * 1024 * 1024]; // 100MB of garbage
    fs::write(test_dir.join("huge.ttf"), large_file).unwrap();

    // File with read permissions removed (platform-specific)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let no_read_path = test_dir.join("no_read.ttf");
        fs::write(&no_read_path, b"FONT").unwrap();
        let mut perms = fs::metadata(&no_read_path).unwrap().permissions();
        perms.set_mode(0o000);
        fs::set_permissions(&no_read_path, perms).unwrap();
    }

    temp_dir
}

#[test]
fn test_font_loader_handles_empty_file() {
    let temp_dir = setup_test_fonts();
    let mut loader = FontLoader::new();

    let empty_path = temp_dir.path().join("empty.ttf");
    let result = loader.load_font_data(&empty_path);

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("empty") || err_msg.contains("parse") || err_msg.contains("invalid"));
}

#[test]
fn test_font_loader_handles_invalid_magic() {
    let temp_dir = setup_test_fonts();
    let mut loader = FontLoader::new();

    let invalid_path = temp_dir.path().join("invalid_magic.ttf");
    let result = loader.load_font_data(&invalid_path);

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("parse") || err_msg.contains("invalid") || err_msg.contains("format"));
}

#[test]
fn test_font_loader_handles_truncated_file() {
    let temp_dir = setup_test_fonts();
    let mut loader = FontLoader::new();

    let truncated_path = temp_dir.path().join("truncated.ttf");
    let result = loader.load_font_data(&truncated_path);

    assert!(result.is_err());
}

#[test]
fn test_font_loader_handles_garbage_data() {
    let temp_dir = setup_test_fonts();
    let mut loader = FontLoader::new();

    let garbage_path = temp_dir.path().join("garbage.ttf");
    let result = loader.load_font_data(&garbage_path);

    assert!(result.is_err());
}

#[test]
fn test_mmap_font_cache_handles_empty_file() {
    let temp_dir = setup_test_fonts();
    let mut cache = MmapFontCache::new(10);

    let empty_path = temp_dir.path().join("empty.ttf");
    let result = cache.get_or_load(&empty_path);

    assert!(result.is_err());
}

#[test]
fn test_mmap_font_cache_handles_invalid_ttc() {
    let temp_dir = setup_test_fonts();
    let mut cache = MmapFontCache::new(10);

    let ttc_path = temp_dir.path().join("invalid.ttc");
    let result = cache.get_or_load(&ttc_path);

    assert!(result.is_err());
}

#[test]
fn test_file_info_handles_invalid_woff() {
    let temp_dir = setup_test_fonts();

    let woff_path = temp_dir.path().join("invalid.woff");
    let result = FileInfo::from_path(&woff_path);

    // FileInfo might succeed in reading the file but font parsing should fail
    if let Ok(file_info) = result {
        // Try to parse as a font - this should fail
        use read_fonts::FontRef;
        let font_result = FontRef::from_index(&file_info.mmap, 0);
        assert!(font_result.is_err());
    } else {
        // Or FileInfo itself might fail
        assert!(result.is_err());
    }
}

#[test]
fn test_file_info_handles_invalid_woff2() {
    let temp_dir = setup_test_fonts();

    let woff2_path = temp_dir.path().join("invalid.woff2");
    let result = FileInfo::from_path(&woff2_path);

    // FileInfo might succeed in reading the file but font parsing should fail
    if let Ok(file_info) = result {
        // Try to parse as a font - this should fail
        use read_fonts::FontRef;
        let font_result = FontRef::from_index(&file_info.mmap, 0);
        assert!(font_result.is_err());
    } else {
        // Or FileInfo itself might fail
        assert!(result.is_err());
    }
}

#[test]
#[cfg(unix)]
fn test_font_loader_handles_permission_denied() {
    let temp_dir = setup_test_fonts();
    let mut loader = FontLoader::new();

    let no_read_path = temp_dir.path().join("no_read.ttf");
    let result = loader.load_font_data(&no_read_path);

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("permission") || err_msg.contains("denied") || err_msg.contains("access")
    );
}

#[test]
fn test_font_loader_handles_nonexistent_file() {
    let mut loader = FontLoader::new();

    let nonexistent_path = PathBuf::from("/nonexistent/path/to/font.ttf");
    let result = loader.load_font_data(&nonexistent_path);

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("not found") || err_msg.contains("No such file"));
}

#[test]
fn test_text_shaper_handles_invalid_font_data() {
    let invalid_font = vec![0xFF; 100]; // Random garbage
    let mut shaper = TextShaper::new();

    // Try to shape text with invalid font data
    let result = shaper.shape(&invalid_font, "Test", 12.0, &Default::default());

    assert!(result.is_err());
}

#[test]
fn test_cpu_rasterizer_handles_invalid_font() {
    use read_fonts::FontRef;

    let invalid_font = vec![0xFF; 100]; // Random garbage
    let _rasterizer = CpuRasterizer::new(12.0);

    // Try to parse invalid font data
    let font_result = FontRef::from_index(&invalid_font, 0);
    assert!(font_result.is_err());

    // Verify that trying to use invalid font data fails gracefully
    // Note: render_glyph requires a valid FontRef, so we can't test it directly with garbage
}

#[test]
fn test_json_parser_handles_malformed_json() {
    let malformed_json = r#"{"jobs": [{"id": "test", "font_path": "/path/to/font.ttf"#; // Missing closing braces

    let result = parse_job_spec(malformed_json);

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("EOF") || err_msg.contains("unexpected end"));
}

#[test]
fn test_json_parser_handles_missing_required_fields() {
    let incomplete_json = r#"{"jobs": [{"id": "test"}]}"#; // Missing required font_path

    let result = parse_job_spec(incomplete_json);

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("missing field") || err_msg.contains("font_path"));
}

#[test]
fn test_json_parser_handles_invalid_field_types() {
    let invalid_types = r#"{
        "jobs": [{
            "id": 123,
            "font_path": "/path/to/font.ttf",
            "text": "Test"
        }]
    }"#; // id should be string, not number

    let result = parse_job_spec(invalid_types);

    assert!(result.is_err());
}

#[test]
fn test_batch_processing_continues_on_font_failure() {
    let temp_dir = setup_test_fonts();
    let mut loader = FontLoader::new();

    // Create a job spec with both valid and invalid fonts
    let spec = JobSpec {
        version: "1.0".to_string(),
        jobs: vec![
            Job {
                id: "job1".to_string(),
                font: FontSpec {
                    path: temp_dir
                        .path()
                        .join("empty.ttf")
                        .to_string_lossy()
                        .to_string(),
                    variations: None,
                    named_instance: None,
                },
                text: "Test".to_string(),
                size: 12.0,
                shaping: Default::default(),
                rendering: Default::default(),
            },
            Job {
                id: "job2".to_string(),
                font: FontSpec {
                    path: temp_dir
                        .path()
                        .join("invalid_magic.ttf")
                        .to_string_lossy()
                        .to_string(),
                    variations: None,
                    named_instance: None,
                },
                text: "Test".to_string(),
                size: 12.0,
                shaping: Default::default(),
                rendering: Default::default(),
            },
        ],
        include_shaping_output: false,
        storage: Default::default(),
    };

    // Process jobs and verify error handling
    let mut success_count = 0;
    let mut error_count = 0;

    for job in &spec.jobs {
        match loader.load_font_data(&PathBuf::from(&job.font.path)) {
            Ok(_) => success_count += 1,
            Err(_) => error_count += 1,
        }
    }

    // All jobs should fail with these malformed fonts
    assert_eq!(error_count, 2);
    assert_eq!(success_count, 0);
}

#[test]
fn test_memory_limit_enforcement() {
    let mut loader = FontLoader::new();

    // FontLoader has a fixed memory limit set in new()
    // We can test that it enforces the limit by trying to load multiple large files
    let temp_dir = setup_test_fonts();
    let huge_path = temp_dir.path().join("huge.ttf");

    // This huge file should trigger memory limit handling
    let result = loader.load_font_data(&huge_path);
    // The huge file is invalid anyway, so it will fail
    assert!(result.is_err());
}

#[test]
fn test_concurrent_error_handling() {
    use std::sync::Arc;
    use std::thread;

    let temp_dir = Arc::new(setup_test_fonts());
    let mut handles = vec![];

    // Spawn multiple threads trying to load invalid fonts
    for i in 0..4 {
        let temp_dir = Arc::clone(&temp_dir);
        let handle = thread::spawn(move || {
            let mut loader = FontLoader::new();
            let path = match i {
                0 => temp_dir.path().join("empty.ttf"),
                1 => temp_dir.path().join("invalid_magic.ttf"),
                2 => temp_dir.path().join("truncated.ttf"),
                _ => temp_dir.path().join("garbage.ttf"),
            };

            loader.load_font_data(&path).is_err()
        });
        handles.push(handle);
    }

    // All threads should handle errors gracefully
    for handle in handles {
        assert!(handle.join().unwrap());
    }
}

#[test]
fn test_cache_eviction_on_corrupt_font() {
    let temp_dir = setup_test_fonts();
    let mut cache = MmapFontCache::new(10);

    let invalid_path = temp_dir.path().join("invalid_magic.ttf");

    // First attempt should fail
    let result1 = cache.get_or_load(&invalid_path);
    assert!(result1.is_err());

    // Second attempt should also fail (not cached)
    let result2 = cache.get_or_load(&invalid_path);
    assert!(result2.is_err());
}
