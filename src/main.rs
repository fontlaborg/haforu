// this_file: src/main.rs

//! Haforu CLI: Batch font renderer for FontSimi.
//!
//! Reads JSON job specifications from stdin, processes rendering jobs,
//! and outputs JSONL results to stdout.

use base64::engine::general_purpose::STANDARD as BASE64_ENGINE;
use base64::Engine;
use camino::Utf8PathBuf;
use clap::{Parser, Subcommand, ValueEnum};
use haforu::security;
use haforu::{
    batch::{FontConfig, Job, JobResult, JobSpec, RenderingConfig, TextConfig},
    process_job_with_options, ExecutionOptions, FontLoader,
};
use rayon::{current_num_threads, prelude::*};
use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;
use std::io::{self, BufRead, Read, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::time::Instant;

mod input;

const DEFAULT_MAX_FONTS: usize = 512;
const DEFAULT_MAX_GLYPHS: usize = 2048;
const STATS_PREFIX: &str = "HAFORU_STATS";

/// Haforu: High-performance batch font renderer
#[derive(Parser)]
#[command(name = "haforu")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Global log output format
    #[arg(long = "log-format", value_enum, default_value_t = LogFormat::Text, global = true)]
    log_format: LogFormat,
    /// Subcommand to execute
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Process a batch of rendering jobs from stdin (JSON)
    Batch {
        /// Font cache size (number of font instances)
        #[arg(long = "max-fonts", default_value_t = DEFAULT_MAX_FONTS, alias = "cache-size")]
        max_fonts: usize,

        /// Glyph cache entries retained across renders (0 disables)
        #[arg(long = "max-glyphs", default_value_t = DEFAULT_MAX_GLYPHS)]
        max_glyphs: usize,

        /// Number of parallel worker threads (0 = auto)
        #[arg(long = "jobs", default_value = "0", alias = "workers")]
        jobs: usize,

        /// Enable verbose logging
        #[arg(short, long)]
        verbose: bool,

        /// Constrain font paths to this base directory
        #[arg(long)]
        base_dir: Option<Utf8PathBuf>,

        /// Per-job timeout in milliseconds (0 disables)
        #[arg(long, default_value = "0")]
        timeout_ms: u64,

        /// Emit throughput + error stats to stderr as JSON
        #[arg(long)]
        stats: bool,
    },

    /// Process jobs from stdin in streaming mode (JSONL input)
    Stream {
        /// Font cache size (number of font instances)
        #[arg(long = "max-fonts", default_value_t = DEFAULT_MAX_FONTS, alias = "cache-size")]
        max_fonts: usize,

        /// Glyph cache entries retained across renders (0 disables)
        #[arg(long = "max-glyphs", default_value_t = DEFAULT_MAX_GLYPHS)]
        max_glyphs: usize,

        /// Enable verbose logging
        #[arg(short, long)]
        verbose: bool,

        /// Constrain font paths to this base directory
        #[arg(long)]
        base_dir: Option<Utf8PathBuf>,

        /// Per-job timeout in milliseconds (0 disables)
        #[arg(long, default_value = "0")]
        timeout_ms: u64,

        /// Emit throughput + error stats to stderr as JSON
        #[arg(long)]
        stats: bool,
    },

    /// Validate a JSON job specification from a file (or stdin if omitted)
    Validate {
        /// Input file path (reads stdin if not provided)
        #[arg(short, long)]
        input: Option<Utf8PathBuf>,
    },

    /// Print version information
    Version,

    /// Print CLI diagnostics and defaults
    Diagnostics {
        /// Output format (text or JSON)
        #[arg(long = "format", value_enum, default_value_t = DiagnosticsFormat::Text)]
        format: DiagnosticsFormat,
    },

    /// Render text using HarfBuzz-compatible syntax
    Render {
        /// Font file path
        #[arg(short = 'f', long = "font-file", alias = "font")]
        font_file: Utf8PathBuf,

        /// Font size in points
        #[arg(short = 's', long = "font-size", alias = "size", default_value = "72")]
        font_size: f32,

        /// Text to render
        #[arg(short = 't', long = "text")]
        text: String,

        /// Font variations (e.g., "wght=700,wdth=100")
        #[arg(long = "variations", alias = "var")]
        variations: Option<String>,

        /// Output file (stdout if not specified)
        #[arg(short = 'o', long = "output-file", alias = "output")]
        output_file: Option<Utf8PathBuf>,

        /// Output format (pgm, png, metrics)
        #[arg(long = "format", default_value = "pgm")]
        format: String,

        /// Canvas width
        #[arg(long = "width", default_value = "800")]
        width: u32,

        /// Canvas height
        #[arg(long = "height", default_value = "200")]
        height: u32,

        /// Script tag (e.g., latn, arab, deva)
        #[arg(long = "script")]
        script: Option<String>,

        /// Language tag (e.g., en, ar, hi)
        #[arg(long = "language", alias = "lang")]
        language: Option<String>,

        /// Text direction (ltr, rtl, ttb, btt)
        #[arg(long = "direction", alias = "dir", default_value = "ltr")]
        direction: String,

        /// OpenType features (e.g., "liga,kern,calt")
        #[arg(long = "features", alias = "feat")]
        features: Option<String>,

        /// Show HarfBuzz-compatible help
        #[arg(long = "help-harfbuzz", action = clap::ArgAction::SetTrue)]
        help_harfbuzz: bool,

        /// Enable verbose logging
        #[arg(short, long)]
        verbose: bool,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum LogFormat {
    Text,
    Json,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum DiagnosticsFormat {
    Text,
    Json,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Batch {
            max_fonts,
            max_glyphs,
            jobs,
            verbose,
            base_dir,
            timeout_ms,
            stats,
        } => {
            init_logging(verbose, cli.log_format);
            let mut opts = ExecutionOptions::new(
                base_dir,
                if timeout_ms == 0 {
                    None
                } else {
                    Some(timeout_ms)
                },
            );
            if max_glyphs > 0 {
                opts.set_glyph_cache_capacity(max_glyphs);
            }
            run_batch_mode(max_fonts, jobs, &opts, stats)?;
        }
        Commands::Stream {
            max_fonts,
            max_glyphs,
            verbose,
            base_dir,
            timeout_ms,
            stats,
        } => {
            init_logging(verbose, cli.log_format);
            let mut opts = ExecutionOptions::new(
                base_dir,
                if timeout_ms == 0 {
                    None
                } else {
                    Some(timeout_ms)
                },
            );
            if max_glyphs > 0 {
                opts.set_glyph_cache_capacity(max_glyphs);
            }
            run_streaming_mode(max_fonts, &opts, stats)?;
        }
        Commands::Validate { input } => {
            init_logging(false, cli.log_format);
            run_validate(input)?;
        }
        Commands::Version => {
            println!("haforu {}", env!("CARGO_PKG_VERSION"));
            println!("Rust font renderer for FontSimi integration");
        }
        Commands::Diagnostics { format } => {
            run_diagnostics(format)?;
        }
        Commands::Render {
            font_file,
            font_size,
            text,
            variations,
            output_file,
            format,
            width,
            height,
            script,
            language,
            direction,
            features,
            help_harfbuzz,
            verbose,
        } => {
            if help_harfbuzz {
                print_harfbuzz_help();
                return Ok(());
            }

            init_logging(verbose, cli.log_format);

            // Parse variations from string format
            let mut var_map = std::collections::HashMap::new();
            if let Some(vars) = variations {
                for pair in vars.split(',') {
                    if let Some((key, val)) = pair.split_once('=') {
                        if let Ok(value) = val.parse::<f32>() {
                            var_map.insert(key.to_string(), value);
                        }
                    }
                }
            }

            let mut feature_list = Vec::new();
            if let Some(list) = features {
                for item in list.split(',') {
                    let trimmed = item.trim();
                    if !trimmed.is_empty() {
                        feature_list.push(trimmed.to_string());
                    }
                }
            }
            let font_size_u32 = font_size.max(1.0).round() as u32;

            let job = Job {
                id: "render".to_string(),
                font: FontConfig {
                    path: font_file,
                    size: font_size_u32,
                    variations: var_map,
                },
                text: TextConfig {
                    content: text,
                    script,
                    direction: Some(direction),
                    language,
                    features: feature_list,
                },
                rendering: RenderingConfig {
                    format: format.clone(),
                    encoding: if format == "metrics" {
                        "json".to_string()
                    } else {
                        "base64".to_string()
                    },
                    width,
                    height,
                },
            };

            // Process the job
            let opts = ExecutionOptions::new(None, None);
            let font_loader = Arc::new(FontLoader::new(1));
            let result = process_job_with_options(&job, &font_loader, &opts);

            // Output the result
            if let Some(output_path) = output_file {
                if format == "metrics" {
                    // Write JSON metrics to file
                    std::fs::write(&output_path, serde_json::to_string_pretty(&result)?)?;
                    println!("Metrics written to: {}", output_path);
                } else {
                    if result.status == "success" {
                        if let Some(rendering) = result.rendering {
                            let image_bytes = BASE64_ENGINE.decode(rendering.data)?;
                            std::fs::write(&output_path, image_bytes)?;
                            println!("Image written to: {}", output_path);
                        }
                    } else {
                        eprintln!("Render failed: {}", result.error.unwrap_or_default());
                        std::process::exit(1);
                    }
                }
            } else {
                // Output to stdout
                if format == "metrics" {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else if result.status == "success" {
                    if let Some(rendering) = result.rendering {
                        let image_bytes = BASE64_ENGINE.decode(rendering.data)?;
                        std::io::stdout().write_all(&image_bytes)?;
                    }
                } else {
                    eprintln!("Render failed: {}", result.error.unwrap_or_default());
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}

/// Initialize logging based on verbosity.
fn print_harfbuzz_help() {
    println!("Haforu HarfBuzz-Compatible Mode");
    println!("================================");
    println!();
    println!("Usage: haforu render [OPTIONS] --font-file <FONT_FILE> --text <TEXT>");
    println!();
    println!("This mode provides HarfBuzz-compatible command-line options for easier migration");
    println!("from existing HarfBuzz-based workflows.");
    println!();
    println!("Common HarfBuzz-compatible options:");
    println!("  -f, --font-file <PATH>      Font file to use");
    println!("  -s, --font-size <SIZE>      Font size in points (default: 72)");
    println!("  -t, --text <STRING>         Text to render");
    println!("      --variations <VARS>     Font variations (e.g., 'wght=700,wdth=100')");
    println!("  -o, --output-file <PATH>    Output file (stdout if not specified)");
    println!("      --format <FORMAT>       Output format: pgm, png, metrics (default: pgm)");
    println!("      --width <WIDTH>         Canvas width (default: 800)");
    println!("      --height <HEIGHT>       Canvas height (default: 200)");
    println!("      --script <SCRIPT>       Script tag (e.g., latn, arab, deva)");
    println!("      --language <LANG>       Language tag (e.g., en, ar, hi)");
    println!("      --direction <DIR>       Text direction: ltr, rtl, ttb, btt (default: ltr)");
    println!("      --features <FEATURES>   OpenType features (e.g., 'liga,kern,calt')");
    println!();
    println!("Examples:");
    println!("  # Basic rendering");
    println!("  haforu render -f font.ttf -t 'Hello World' -o output.pgm");
    println!();
    println!("  # With variations");
    println!("  haforu render -f font.ttf -t 'Text' --variations 'wght=700' -s 48");
    println!();
    println!("  # Metrics only");
    println!("  haforu render -f font.ttf -t 'A' --format metrics");
    println!();
    println!("  # RTL text");
    println!("  haforu render -f font.ttf -t 'مرحبا' --direction rtl --script arab --language ar");
    println!();
    println!("For more information, see: https://github.com/fontsimi/haforu");
}

fn init_logging(verbose: bool, log_format: LogFormat) {
    let level = if verbose { "debug" } else { "info" };
    let mut builder =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(level));
    match log_format {
        LogFormat::Text => {
            builder.format_timestamp_millis();
        }
        LogFormat::Json => {
            builder.format(|buf, record| {
                let ts = buf.timestamp_millis().to_string();
                let payload = json!({
                    "ts": ts,
                    "level": record.level().to_string(),
                    "target": record.target(),
                    "message": record.args().to_string(),
                });
                writeln!(buf, "{payload}")
            });
        }
    }
    let _ = builder.try_init();
}

/// Run in batch mode: read entire JobSpec from stdin, process in parallel, output JSONL.
fn run_batch_mode(
    max_fonts: usize,
    workers: usize,
    opts: &ExecutionOptions,
    stats: bool,
) -> anyhow::Result<()> {
    let glyph_cache = opts.glyph_cache_capacity();
    log::info!(
        "Starting batch mode (max_fonts={}, glyph_cache={}, jobs={})",
        max_fonts,
        glyph_cache,
        workers
    );

    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut payload = String::new();
    reader.read_to_string(&mut payload)?;
    security::validate_json_size(&payload, security::MAX_JSON_SIZE)?;

    let jobs = input::parse_jobs_payload(&payload)?;
    log::info!("Loaded {} jobs from stdin", jobs.len());

    let started = Instant::now();
    let summary = process_jobs_parallel(jobs, max_fonts, workers, opts)?;
    let elapsed = started.elapsed();
    log::info!(
        "Batch complete: {} jobs ({} success, {} errors) in {:.2?}",
        summary.total,
        summary.successes,
        summary.errors,
        elapsed
    );

    if stats {
        let report = BatchStatsReport {
            kind: "batch",
            jobs: summary.total,
            successes: summary.successes,
            errors: summary.errors,
            duration_ms: elapsed.as_millis(),
            jobs_per_sec: throughput(summary.total, elapsed),
            worker_threads: summary.worker_threads,
            glyph_cache,
        };
        emit_stats(&report)?;
    }

    Ok(())
}

/// Run in streaming mode: read jobs line-by-line (JSONL), output results immediately.
fn run_streaming_mode(
    max_fonts: usize,
    opts: &ExecutionOptions,
    stats: bool,
) -> anyhow::Result<()> {
    let glyph_cache = opts.glyph_cache_capacity();
    log::info!(
        "Starting streaming mode (max_fonts={}, glyph_cache={})",
        max_fonts,
        glyph_cache
    );

    let font_loader = FontLoader::new(max_fonts);

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdout_handle = stdout.lock();
    let mut counters = StreamCounters::default();
    let started = Instant::now();

    for (line_no, line) in stdin.lock().lines().enumerate() {
        let line = line?;

        if let Some(result) = handle_stream_line(&line, line_no, &font_loader, opts) {
            counters.processed += 1;
            if result.status == "success" {
                counters.successes += 1;
            } else {
                counters.errors += 1;
            }
            let json = serde_json::to_string(&result)?;
            writeln!(stdout_handle, "{}", json)?;
            stdout_handle.flush()?;

            if result.status == "success" {
                log::debug!("Processed job {} (id={})", line_no + 1, result.id);
            }
        }
    }

    let elapsed = started.elapsed();
    log::info!(
        "Streaming mode complete: {} processed ({} success, {} errors)",
        counters.processed,
        counters.successes,
        counters.errors
    );
    if stats {
        let report = StreamStatsReport {
            kind: "stream",
            processed: counters.processed,
            successes: counters.successes,
            errors: counters.errors,
            duration_ms: elapsed.as_millis(),
            jobs_per_sec: throughput(counters.processed, elapsed),
            glyph_cache,
        };
        emit_stats(&report)?;
    }

    Ok(())
}

fn handle_stream_line(
    line: &str,
    line_no: usize,
    font_loader: &FontLoader,
    opts: &ExecutionOptions,
) -> Option<JobResult> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let fallback_id = format!("line-{}", line_no + 1);
    let parsed_value = match serde_json::from_str::<serde_json::Value>(trimmed) {
        Ok(value) => value,
        Err(err) => {
            log::error!("Line {}: Invalid JSON: {}", line_no + 1, err);
            return Some(JobResult::error(
                fallback_id,
                format!("Invalid JSON: {err}"),
            ));
        }
    };

    let job_id = parsed_value
        .get("id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| fallback_id.clone());

    let job: Job = match serde_json::from_value(parsed_value) {
        Ok(job) => job,
        Err(err) => {
            log::error!("Line {}: Invalid job JSON: {}", line_no + 1, err);
            return Some(JobResult::error(job_id, format!("Invalid job JSON: {err}")));
        }
    };

    if let Err(err) = job.validate() {
        log::error!("Line {}: Invalid job: {}", line_no + 1, err);
        return Some(JobResult::error(job.id.clone(), err.to_string()));
    }

    Some(process_job_with_options(&job, font_loader, opts))
}

fn process_jobs_parallel(
    jobs: Vec<Job>,
    max_fonts: usize,
    workers: usize,
    opts: &ExecutionOptions,
) -> anyhow::Result<BatchRunSummary> {
    if jobs.is_empty() {
        anyhow::bail!("No jobs supplied");
    }

    if workers > 0 {
        rayon::ThreadPoolBuilder::new()
            .num_threads(workers)
            .build_global()
            .ok();
    }

    let font_loader = Arc::new(FontLoader::new(max_fonts));
    let opts = Arc::new(opts.clone());
    let total = jobs.len();
    let successes = Arc::new(AtomicUsize::new(0));
    let errors = Arc::new(AtomicUsize::new(0));

    let (tx, rx) = mpsc::channel();

    let output_handle = std::thread::spawn(move || {
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        for result in rx {
            let json = serde_json::to_string(&result).expect("Failed to serialize result");
            writeln!(handle, "{}", json).expect("Failed to write to stdout");
            handle.flush().expect("Failed to flush stdout");
        }
    });

    jobs.into_par_iter().for_each(|job| {
        let loader = Arc::clone(&font_loader);
        let opts = Arc::clone(&opts);
        let result = process_job_with_options(&job, loader.as_ref(), opts.as_ref());
        if result.status == "success" {
            successes.fetch_add(1, Ordering::Relaxed);
        } else {
            errors.fetch_add(1, Ordering::Relaxed);
        }
        let _ = tx.send(result);
    });

    drop(tx);
    output_handle.join().expect("Output thread panicked");

    log::info!("Batch processing complete ({} jobs)", total);
    Ok(BatchRunSummary {
        total,
        successes: successes.load(Ordering::Relaxed),
        errors: errors.load(Ordering::Relaxed),
        worker_threads: if workers > 0 {
            workers
        } else {
            rayon::current_num_threads()
        },
    })
}

/// Validate a JSON spec from file or stdin and print summary.
fn run_validate(input: Option<Utf8PathBuf>) -> anyhow::Result<()> {
    let json = if let Some(path) = input {
        std::fs::read_to_string(path.as_std_path())?
    } else {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        buf
    };

    security::validate_json_size(&json, security::MAX_JSON_SIZE)?;
    let spec: JobSpec = serde_json::from_str(&json)?;
    spec.validate()?;
    println!("✓ Valid job specification");
    println!("  Version: {}", spec.version);
    println!("  Jobs: {}", spec.jobs.len());
    Ok(())
}

#[derive(Serialize)]
struct DiagnosticsReport {
    status: &'static str,
    cli_version: &'static str,
    cpu_count: usize,
    default_max_fonts: usize,
    default_max_glyphs: usize,
    max_jobs_per_spec: usize,
    max_json_bytes: usize,
}

fn run_diagnostics(format: DiagnosticsFormat) -> anyhow::Result<()> {
    let cpu_count = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let report = DiagnosticsReport {
        status: "ok",
        cli_version: env!("CARGO_PKG_VERSION"),
        cpu_count,
        default_max_fonts: DEFAULT_MAX_FONTS,
        default_max_glyphs: DEFAULT_MAX_GLYPHS,
        max_jobs_per_spec: security::MAX_JOBS_PER_SPEC,
        max_json_bytes: security::MAX_JSON_SIZE,
    };

    match format {
        DiagnosticsFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        DiagnosticsFormat::Text => {
            println!("haforu {}", report.cli_version);
            println!("Status       : {}", report.status);
            println!("CPU threads  : {}", report.cpu_count);
            println!("Cache defaults: fonts={} glyphs={}", report.default_max_fonts, report.default_max_glyphs);
            println!(
                "Security      : max_jobs_per_spec={} max_json_size={} bytes (~{} MiB)",
                report.max_jobs_per_spec,
                report.max_json_bytes,
                report.max_json_bytes / (1024 * 1024)
            );
        }
    }

    Ok(())
}

#[derive(Debug, Serialize)]
struct BatchStatsReport {
    kind: &'static str,
    jobs: usize,
    successes: usize,
    errors: usize,
    duration_ms: u128,
    jobs_per_sec: f64,
    worker_threads: usize,
    glyph_cache: usize,
}

#[derive(Debug, Serialize)]
struct StreamStatsReport {
    kind: &'static str,
    processed: usize,
    successes: usize,
    errors: usize,
    duration_ms: u128,
    jobs_per_sec: f64,
    glyph_cache: usize,
}

#[derive(Default)]
struct StreamCounters {
    processed: usize,
    successes: usize,
    errors: usize,
}

struct BatchRunSummary {
    total: usize,
    successes: usize,
    errors: usize,
    worker_threads: usize,
}

fn throughput(count: usize, duration: std::time::Duration) -> f64 {
    if count == 0 {
        return 0.0;
    }
    let secs = duration.as_secs_f64();
    if secs <= f64::EPSILON {
        count as f64
    } else {
        count as f64 / secs
    }
}

fn emit_stats<T: Serialize>(stats: &T) -> anyhow::Result<()> {
    let payload = serde_json::to_string(stats)?;
    eprintln!("{STATS_PREFIX} {payload}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use haforu::{GlyphRasterizer, TextShaper};

    #[test]
    fn test_module_structure() {
        let _ = haforu::batch::JobSpec {
            version: "1.0".to_string(),
            jobs: vec![],
        };
        let _ = FontLoader::new(512);
        let _ = TextShaper::new();
        let _ = GlyphRasterizer::new();
    }

    #[test]
    fn handle_stream_line_skips_blank_lines() {
        let loader = FontLoader::new(4);
        let opts = ExecutionOptions::default();
        assert!(handle_stream_line("   ", 0, &loader, &opts).is_none());
    }

    #[test]
    fn handle_stream_line_returns_error_for_invalid_json() {
        let loader = FontLoader::new(4);
        let opts = ExecutionOptions::default();
        let result = handle_stream_line("{not valid}", 0, &loader, &opts)
            .expect("Invalid JSON should emit an error result");
        assert_eq!(result.id, "line-1");
        assert_eq!(result.status, "error");
        let msg = result.error.unwrap_or_default();
        assert!(msg.contains("Invalid JSON"), "message: {msg}");
    }

    #[test]
    fn handle_stream_line_validates_job_structure() {
        let loader = FontLoader::new(4);
        let opts = ExecutionOptions::default();
        let json = r#"{"id":"abc","font":{"path":"/tmp/missing.ttf","size":0,"variations":{}},"text":{"content":""},"rendering":{"format":"pgm","encoding":"base64","width":0,"height":0}}"#;
        let result = handle_stream_line(json, 2, &loader, &opts)
            .expect("Invalid job should emit an error result");
        assert_eq!(result.id, "abc");
        assert_eq!(result.status, "error");
        let msg = result.error.unwrap_or_default();
        assert!(
            msg.contains("Invalid render") || msg.contains("Invalid job"),
            "message: {msg}"
        );
    }

    #[test]
    fn handle_stream_line_reports_font_errors() {
        let loader = FontLoader::new(4);
        let opts = ExecutionOptions::default();
        let json = r#"{"id":"font-miss","font":{"path":"/tmp/definitely-missing.ttf","size":1000,"variations":{}},"text":{"content":"A"},"rendering":{"format":"pgm","encoding":"base64","width":10,"height":10}}"#;
        let result = handle_stream_line(json, 5, &loader, &opts)
            .expect("Missing font should surface as error result");
        assert_eq!(result.id, "font-miss");
        assert_eq!(result.status, "error");
        let msg = result.error.unwrap_or_default();
        assert!(msg.contains("Font file not found"), "message: {msg}");
    }
}
