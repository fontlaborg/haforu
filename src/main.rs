// this_file: src/main.rs

//! Haforu CLI: Batch font renderer for FontSimi.
//!
//! Reads JSON job specifications from stdin, processes rendering jobs,
//! and outputs JSONL results to stdout.

use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use haforu::security;
use haforu::{
    batch::Job, process_job_with_options, ExecutionOptions, FontLoader, JobResult, JobSpec,
};
use rayon::prelude::*;
use std::io::{self, BufRead, Read, Write};
use std::sync::{mpsc, Arc};

mod input;

/// Haforu: High-performance batch font renderer
#[derive(Parser)]
#[command(name = "haforu")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Subcommand to execute
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Process a batch of rendering jobs from stdin (JSON)
    Batch {
        /// Font cache size (number of font instances)
        #[arg(long = "max-fonts", default_value = "512", alias = "cache-size")]
        max_fonts: usize,

        /// Glyph cache entries retained across renders (0 disables)
        #[arg(long = "max-glyphs", default_value = "2048")]
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
    },

    /// Process jobs from stdin in streaming mode (JSONL input)
    Stream {
        /// Font cache size (number of font instances)
        #[arg(long = "max-fonts", default_value = "512", alias = "cache-size")]
        max_fonts: usize,

        /// Glyph cache entries retained across renders (0 disables)
        #[arg(long = "max-glyphs", default_value = "2048")]
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
    },

    /// Validate a JSON job specification from a file (or stdin if omitted)
    Validate {
        /// Input file path (reads stdin if not provided)
        #[arg(short, long)]
        input: Option<Utf8PathBuf>,
    },

    /// Print version information
    Version,
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
        } => {
            init_logging(verbose);
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
            run_batch_mode(max_fonts, jobs, &opts)?;
        }
        Commands::Stream {
            max_fonts,
            max_glyphs,
            verbose,
            base_dir,
            timeout_ms,
        } => {
            init_logging(verbose);
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
            run_streaming_mode(max_fonts, &opts)?;
        }
        Commands::Validate { input } => {
            init_logging(false);
            run_validate(input)?;
        }
        Commands::Version => {
            println!("haforu {}", env!("CARGO_PKG_VERSION"));
            println!("Rust font renderer for FontSimi integration");
        }
    }

    Ok(())
}

/// Initialize logging based on verbosity.
fn init_logging(verbose: bool) {
    let level = if verbose { "debug" } else { "info" };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(level))
        .format_timestamp_millis()
        .init();
}

/// Run in batch mode: read entire JobSpec from stdin, process in parallel, output JSONL.
fn run_batch_mode(max_fonts: usize, workers: usize, opts: &ExecutionOptions) -> anyhow::Result<()> {
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

    process_jobs_parallel(jobs, max_fonts, workers, opts)
}

/// Run in streaming mode: read jobs line-by-line (JSONL), output results immediately.
fn run_streaming_mode(max_fonts: usize, opts: &ExecutionOptions) -> anyhow::Result<()> {
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

    for (line_no, line) in stdin.lock().lines().enumerate() {
        let line = line?;

        if let Some(result) = handle_stream_line(&line, line_no, &font_loader, opts) {
            let json = serde_json::to_string(&result)?;
            writeln!(stdout_handle, "{}", json)?;
            stdout_handle.flush()?;

            if result.status == "success" {
                log::debug!("Processed job {} (id={})", line_no + 1, result.id);
            }
        }
    }

    log::info!("Streaming mode complete");

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
) -> anyhow::Result<()> {
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
        let _ = tx.send(result);
    });

    drop(tx);
    output_handle.join().expect("Output thread panicked");

    log::info!("Batch processing complete ({} jobs)", total);
    Ok(())
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
