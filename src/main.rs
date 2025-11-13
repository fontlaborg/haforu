// this_file: external/haforu2/src/main.rs

//! Haforu2 CLI: Batch font renderer for FontSimi.
//!
//! Reads JSON job specifications from stdin, processes rendering jobs,
//! and outputs JSONL results to stdout.

use clap::{Parser, Subcommand};
use haforu2::{process_job_with_options, ExecutionOptions, FontLoader, JobSpec};
use haforu2::security;
use camino::Utf8PathBuf;
use rayon::prelude::*;
use std::io::{self, BufRead, Read, Write};
use std::sync::mpsc;

/// Haforu2: High-performance batch font renderer
#[derive(Parser)]
#[command(name = "haforu2")]
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
        #[arg(long, default_value = "512")]
        cache_size: usize,

        /// Number of parallel worker threads (0 = auto)
        #[arg(long, default_value = "0")]
        workers: usize,

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
        #[arg(long, default_value = "512")]
        cache_size: usize,

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
            cache_size,
            workers,
            verbose,
            base_dir,
            timeout_ms,
        } => {
            init_logging(verbose);
            let opts = ExecutionOptions {
                base_dir,
                timeout_ms: if timeout_ms == 0 { None } else { Some(timeout_ms) },
            };
            run_batch_mode(cache_size, workers, &opts)?;
        }
        Commands::Stream {
            cache_size,
            verbose,
            base_dir,
            timeout_ms,
        } => {
            init_logging(verbose);
            let opts = ExecutionOptions {
                base_dir,
                timeout_ms: if timeout_ms == 0 { None } else { Some(timeout_ms) },
            };
            run_streaming_mode(cache_size, &opts)?;
        }
        Commands::Validate { input } => {
            init_logging(false);
            run_validate(input)?;
        }
        Commands::Version => {
            println!("haforu2 {}", env!("CARGO_PKG_VERSION"));
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
fn run_batch_mode(cache_size: usize, workers: usize, opts: &ExecutionOptions) -> anyhow::Result<()> {
    log::info!("Starting batch mode (cache_size={}, workers={})", cache_size, workers);

    // Configure Rayon thread pool
    if workers > 0 {
        rayon::ThreadPoolBuilder::new()
            .num_threads(workers)
            .build_global()
            .ok();
    }

    // Read JobSpec from stdin with size validation
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut json = String::new();
    reader.read_to_string(&mut json)?;
    security::validate_json_size(&json, security::MAX_JSON_SIZE)?;
    let spec: JobSpec = serde_json::from_str(&json)?;

    log::info!("Loaded {} jobs from stdin", spec.jobs.len());

    // Validate job spec
    if let Err(e) = spec.validate() {
        log::error!("Invalid job specification: {}", e);
        return Err(e.into());
    }

    log::info!("Job specification validated successfully");

    // Create font loader (shared across threads via Arc internally)
    let font_loader = FontLoader::new(cache_size);

    // Process jobs in parallel with streaming output
    let (tx, rx) = mpsc::channel();

    // Spawn output thread
    let output_handle = std::thread::spawn(move || {
        let stdout = io::stdout();
        let mut handle = stdout.lock();

        for result in rx {
            let json = serde_json::to_string(&result).expect("Failed to serialize result");
            writeln!(handle, "{}", json).expect("Failed to write to stdout");
            handle.flush().expect("Failed to flush stdout");
        }
    });

    // Process jobs in parallel
    spec.jobs.par_iter().for_each_with(tx, |tx, job| {
        let result = process_job_with_options(job, &font_loader, opts);
        tx.send(result).ok();
    });

    // Wait for output thread to finish
    output_handle.join().expect("Output thread panicked");

    log::info!("Batch processing complete");

    Ok(())
}

/// Run in streaming mode: read jobs line-by-line (JSONL), output results immediately.
fn run_streaming_mode(cache_size: usize, opts: &ExecutionOptions) -> anyhow::Result<()> {
    log::info!("Starting streaming mode (cache_size={})", cache_size);

    let font_loader = FontLoader::new(cache_size);

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdout_handle = stdout.lock();

    for (line_no, line) in stdin.lock().lines().enumerate() {
        let line = line?;

        if line.trim().is_empty() {
            continue;
        }

        // Parse job
        let job: Result<haforu2::Job, _> = serde_json::from_str(&line);
        let job = match job {
            Ok(j) => j,
            Err(e) => {
                log::error!("Line {}: Invalid JSON: {}", line_no + 1, e);
                continue;
            }
        };

        // Validate job
        if let Err(e) = job.validate() {
            log::error!("Line {}: Invalid job: {}", line_no + 1, e);
            continue;
        }

        // Process job
        let result = process_job_with_options(&job, &font_loader, opts);

        // Output result
        let json = serde_json::to_string(&result)?;
        writeln!(stdout_handle, "{}", json)?;
        stdout_handle.flush()?;

        log::debug!("Processed job {} (id={})", line_no + 1, job.id);
    }

    log::info!("Streaming mode complete");

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
