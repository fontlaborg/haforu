// this_file: tests/cli.rs
//! CLI integration tests for haforu binary

use assert_cmd::prelude::*;
use predicates::prelude::*;
use assert_cmd::Command;

/// Helper to run the `haforu` binary
fn bin() -> Command {
    Command::cargo_bin("haforu").expect("binary exists")
}

#[test]
fn test_cli_version_prints() {
    let mut cmd = bin();
    cmd.arg("version");
    cmd.env_remove("RUST_LOG");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("haforu version"));
}

#[test]
fn test_cli_validate_accepts_valid_json() {
    // Minimal valid spec
    let json = r#"{
        "version": "1.0.0",
        "jobs": [{
            "id": "job1",
            "font": {"path": "03fonts/Archivo[wdth,wght].ttf"},
            "text": "Hello",
            "size": 16.0
        }],
        "storage": {"backend": "filesystem", "output_path": "."}
    }"#;

    let mut cmd = bin();
    cmd.arg("validate");
    cmd.write_stdin(json);
    cmd.env_remove("RUST_LOG");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Valid job specification"));
}

#[test]
fn test_cli_process_emits_jsonl() {
    // Spec with two jobs; process should emit two JSON lines
    let json = r#"{
        "version": "1.0.0",
        "jobs": [
            {
                "id": "job1",
                "font": {"path": "03fonts/Archivo[wdth,wght].ttf"},
                "text": "Hello",
                "size": 16.0
            },
            {
                "id": "job2",
                "font": {"path": "03fonts/Archivo[wdth,wght].ttf"},
                "text": "World",
                "size": 18.0
            }
        ],
        "storage": {"backend": "filesystem", "output_path": "."}
    }"#;

    let mut cmd = bin();
    cmd.arg("process");
    cmd.arg("--output");
    cmd.arg("./out");
    cmd.write_stdin(json);
    cmd.env_remove("RUST_LOG");

    let output = cmd.assert().success().get_output().stdout.clone();
    let out = String::from_utf8_lossy(&output);
    assert!(out.contains("\"id\":\"job1\""), "missing job1 in output: {}", out);
    assert!(out.contains("\"id\":\"job2\""), "missing job2 in output: {}", out);
    assert!(out.contains("\"processing_time_ms\":"), "processing_time_ms not present in output: {}", out);
}
