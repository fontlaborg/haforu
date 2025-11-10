// this_file: benches/performance.rs
//! Performance benchmarks for critical paths

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use haforu::font_loader::FontLoader;
use haforu::json_parser::{parse_job_spec, Job, JobSpec, FontSpec};
use haforu::memory_tracker::{MemoryTracker, BatchProcessor};
use std::path::PathBuf;
use std::sync::Arc;

fn get_test_font_path() -> PathBuf {
    PathBuf::from("03fonts/Archivo/Archivo[wdth,wght].ttf")
}

fn bench_font_loading(c: &mut Criterion) {
    let font_path = get_test_font_path();

    c.bench_function("font_loading", |b| {
        let mut loader = FontLoader::new();
        b.iter(|| {
            loader.clear_cache();
            black_box(loader.load_font_data(&font_path).unwrap());
        });
    });

    // Benchmark cached font loading
    c.bench_function("font_loading_cached", |b| {
        let mut loader = FontLoader::new();
        // Prime the cache
        loader.load_font_data(&font_path).unwrap();

        b.iter(|| {
            black_box(loader.load_font_data(&font_path).unwrap());
        });
    });
}

fn bench_memory_tracking(c: &mut Criterion) {
    c.bench_function("memory_tracker_allocate", |b| {
        let tracker = MemoryTracker::new(1000); // 1GB limit
        b.iter(|| {
            tracker.allocate(1024).unwrap();
            tracker.deallocate(1024);
        });
    });

    c.bench_function("batch_processor_workers", |b| {
        let tracker = Arc::new(MemoryTracker::new(1000));
        let processor = BatchProcessor::new(tracker.clone(), 1000);
        b.iter(|| {
            black_box(processor.concurrent_workers());
        });
    });
}

fn bench_json_parsing(c: &mut Criterion) {
    let json_specs = vec![
        ("small", r#"{"version":"1.0","jobs":[{"id":"1","font":{"path":"font.ttf"},"text":"Hello","size":16}]}"#),
        ("medium", r#"{"version":"1.0","jobs":[
            {"id":"1","font":{"path":"font.ttf"},"text":"Hello","size":16},
            {"id":"2","font":{"path":"font.ttf"},"text":"World","size":24},
            {"id":"3","font":{"path":"font.ttf"},"text":"Test","size":32},
            {"id":"4","font":{"path":"font.ttf"},"text":"Benchmark","size":48},
            {"id":"5","font":{"path":"font.ttf"},"text":"Performance","size":64}
        ]}"#),
    ];

    let mut group = c.benchmark_group("json_parsing");
    for (name, spec) in json_specs {
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            spec,
            |b, spec| {
                b.iter(|| {
                    black_box(parse_job_spec(spec).unwrap());
                });
            },
        );
    }
    group.finish();
}

fn bench_batch_processing(c: &mut Criterion) {
    let batch_sizes = vec![1, 10, 100];

    let mut group = c.benchmark_group("batch_processing");
    group.sample_size(10); // Reduce sample size for large batches

    for batch_size in batch_sizes {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_jobs", batch_size)),
            &batch_size,
            |b, &batch_size| {
                let jobs: Vec<Job> = (0..batch_size)
                    .map(|i| Job {
                        id: format!("job{}", i),
                        font: FontSpec {
                            path: "font.ttf".to_string(),
                            variations: None,
                            named_instance: None,
                        },
                        text: format!("Text number {}", i),
                        size: 16.0,
                        shaping: Default::default(),
                        rendering: Default::default(),
                    })
                    .collect();

                let spec = JobSpec {
                    version: "1.0".to_string(),
                    jobs: jobs.clone(),
                    include_shaping_output: false,
                    storage: Default::default(),
                };

                b.iter(|| {
                    black_box(&spec);
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_font_loading,
    bench_memory_tracking,
    bench_json_parsing,
    bench_batch_processing
);
criterion_main!(benches);