// this_file: tests/cli_stats.rs

use serde_json::{json, Value};
use std::path::PathBuf;

fn fixture_font() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata/fonts/Arial-Black.ttf")
}

fn metrics_job_spec(id: &str) -> String {
    let font = fixture_font();
    json!({
        "version": "1.0",
        "jobs": [{
            "id": id,
            "font": {
                "path": font,
                "size": 256,
                "variations": {}
            },
            "text": {
                "content": "A"
            },
            "rendering": {
                "format": "metrics",
                "encoding": "json",
                "width": 32,
                "height": 32
            }
        }]
    })
    .to_string()
}

fn parse_stats(stderr: &str) -> Value {
    for line in stderr.lines().rev() {
        if let Some(json) = line.strip_prefix("HAFORU_STATS ") {
            return serde_json::from_str(json).expect("stats payload should be JSON");
        }
    }
    panic!("stats prefix not found in stderr: {stderr}");
}

#[test]
fn batch_stats_flag_emits_summary_json() {
    let spec = metrics_job_spec("batch-stats");
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("haforu");
    cmd.arg("batch")
        .arg("--stats")
        .arg("--max-fonts")
        .arg("4");

    let output = cmd.write_stdin(spec).output().expect("command should run");
    assert!(
        output.status.success(),
        "CLI should exit successfully: {:?}",
        output
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let mut lines = stdout.lines();
    let result_line = lines.next().expect("one JSONL result");
    let result: Value =
        serde_json::from_str(result_line).expect("result line should parse as JSON");
    assert_eq!(result["id"], "batch-stats");
    assert_eq!(result["status"], "success");
    assert!(
        result.get("metrics").is_some(),
        "metrics job should include metrics payload: {result:?}"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    let stats = parse_stats(&stderr);
    assert_eq!(stats["kind"], "batch");
    assert_eq!(stats["jobs"], 1);
    assert!(
        stats["jobs_per_sec"].as_f64().unwrap_or_default() > 0.0,
        "jobs_per_sec should be positive: {stats}"
    );
}

#[test]
fn streaming_stats_reports_processed_and_errors() {
    let font_path = fixture_font();
    let ok_job = json!({
        "id": "stream-metrics",
        "font": {"path": font_path, "size": 128, "variations": {}},
        "text": {"content": "C"},
        "rendering": {"format": "metrics", "encoding": "json", "width": 32, "height": 32}
    });
    let bad_job = json!({
        "id": "bad-json",
        "font": {"path": "/no/such/font.ttf", "size": 1000, "variations": {}},
        "text": {"content": "A"},
        "rendering": {"format": "pgm", "encoding": "base64", "width": 0, "height": 64}
    });
    let payload = format!("{}\n{}\n", ok_job, bad_job);

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("haforu");
    cmd.arg("stream").arg("--stats");
    let output = cmd
        .write_stdin(payload)
        .output()
        .expect("command should run");
    assert!(
        output.status.success(),
        "stream command should exit successfully: {:?}",
        output
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let results: Vec<Value> = stdout
        .lines()
        .map(|line| serde_json::from_str(line).expect("valid result json"))
        .collect();
    assert_eq!(results.len(), 2);

    let metrics = results
        .iter()
        .find(|val| val["id"] == "stream-metrics")
        .expect("success payload present");
    assert_eq!(metrics["status"], "success");
    assert!(metrics.get("metrics").is_some());

    let error = results
        .iter()
        .find(|val| val["id"] == "bad-json")
        .expect("error payload present");
    assert_eq!(error["status"], "error");

    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    let stats = parse_stats(&stderr);
    assert_eq!(stats["kind"], "stream");
    assert_eq!(stats["processed"], 2);
    assert_eq!(stats["errors"], 1);
}

#[test]
fn diagnostics_command_outputs_json_report() {
    let output = assert_cmd::cargo::cargo_bin_cmd!("haforu")
        .arg("diagnostics")
        .arg("--format")
        .arg("json")
        .output()
        .expect("command should run");

    assert!(
        output.status.success(),
        "diagnostics command should succeed: {:?}",
        output
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let payload: Value =
        serde_json::from_str(&stdout).expect("diagnostics response should be JSON");
    assert_eq!(payload["status"], "ok");
    assert!(
        payload["cpu_count"].as_u64().unwrap_or_default() >= 1,
        "cpu_count should be >= 1: {payload}"
    );
    assert_eq!(payload["cli_version"], env!("CARGO_PKG_VERSION"));
    assert!(
        payload["default_max_fonts"].as_u64().unwrap_or_default() >= 1,
        "default_max_fonts should be provided"
    );
}
