// this_file: benches/cli.rs

use camino::Utf8PathBuf;
use criterion::{criterion_group, criterion_main, Criterion};
use haforu::{
    batch::{FontConfig, Job, RenderingConfig, TextConfig},
    process_job_with_options, ExecutionOptions, FontLoader,
};
use std::collections::HashMap;

fn metrics_job() -> Job {
    let font_path =
        Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata/fonts/Arial-Black.ttf");
    Job {
        id: "bench".to_string(),
        font: FontConfig {
            path: font_path,
            size: 256,
            variations: HashMap::new(),
        },
        text: TextConfig {
            content: "A".to_string(),
            script: Some("Latn".to_string()),
            direction: None,
            language: None,
            features: Vec::new(),
        },
        rendering: RenderingConfig {
            format: "metrics".to_string(),
            encoding: "json".to_string(),
            width: 64,
            height: 64,
        },
    }
}

fn bench_metrics_job(c: &mut Criterion) {
    let job = metrics_job();
    let loader = FontLoader::new(32);
    let opts = ExecutionOptions::default();
    c.bench_function("process_metrics_job", |b| {
        b.iter(|| {
            let _ = process_job_with_options(&job, &loader, &opts);
        });
    });
}

criterion_group!(cli_benches, bench_metrics_job);
criterion_main!(cli_benches);
