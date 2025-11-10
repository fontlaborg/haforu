// this_file: src/main.rs
//! Haforu CLI - Font shaping and rendering tool

use anyhow::Result;
use clap::{Parser, Subcommand};
use haforu::{FontLoader, json_parser, logging};
use log::{error, info};
use std::io::{self, Read};
use std::time::Instant;

/// Haforu - Enhanced font shaping and rendering tool
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Set log level (error, warn, info, debug, trace)
    #[arg(short = 'l', long, global = true, default_value = "info")]
    log_level: String,

    /// Enable quiet mode (only errors)
    #[arg(short = 'q', long, global = true, conflicts_with = "log_level")]
    quiet: bool,

    /// Subcommand to execute
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Process JSON job specification from stdin
    Process {
        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,

        /// Output directory for rendered results
        #[arg(short, long, default_value = "./output")]
        output: String,
    },

    /// Validate JSON job specification
    Validate {
        /// Input file (uses stdin if not specified)
        #[arg(short, long)]
        input: Option<String>,
    },

    /// Show version information
    Version,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Configure logging using project logger
    let log_level = if cli.quiet { "error".to_string() } else { cli.log_level.clone() };
    logging::init_logging(&log_level, cli.quiet, true);

    match cli.command {
        Commands::Process { verbose, output } => {
            if verbose {
                info!("Processing jobs with output to: {}", output);
            }
            process_jobs(&output)?;
        }
        Commands::Validate { input } => {
            validate_spec(input)?;
        }
        Commands::Version => {
            println!("haforu version {}", haforu::VERSION);
            println!("Font shaping and rendering tool");
            println!("Compatible with HarfBuzz CLI tools");
        }
    }

    Ok(())
}

/// Process jobs from stdin
fn process_jobs(_output_dir: &str) -> Result<()> {
    // Read JSON from stdin
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;

    // Parse job specification
    let job_spec = match json_parser::parse_job_spec(&buffer) {
        Ok(spec) => spec,
        Err(e) => {
            error!("Failed to parse job specification: {}", e);
            return Err(e.into());
        }
    };

    info!("Processing {} jobs", job_spec.jobs.len());

    // Create font loader
    let _font_loader = FontLoader::new();

    // Process each job
    for job in &job_spec.jobs {
        let t0 = Instant::now();
        info!("Processing job: {}", job.id);

        // For now, just validate and print the job
        // Full implementation would:
        // 1. Load the font using font_loader
        // 2. Shape the text using TextShaper
        // 3. Render if requested using Vello
        // 4. Store results using StorageManager
        // 5. Output JSONL result

        let result = json_parser::JobResult {
            id: job.id.clone(),
            input: job.clone(),
            shaping_output: None,
            rendering_result: None,
            status: "pending".to_string(),
            error: Some("Not fully implemented yet".to_string()),
            processing_time_ms: t0.elapsed().as_millis() as u64,
        };

        // Output as JSONL
        let json_line = json_parser::serialize_job_result(&result)?;
        println!("{}", json_line);
    }

    info!("Finished processing all jobs");
    Ok(())
}

/// Validate job specification
fn validate_spec(input: Option<String>) -> Result<()> {
    let json = if let Some(path) = input {
        std::fs::read_to_string(path)?
    } else {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;
        buffer
    };

    match json_parser::parse_job_spec(&json) {
        Ok(spec) => {
            println!("✓ Valid job specification");
            println!("  Version: {}", spec.version);
            println!("  Jobs: {}", spec.jobs.len());
            println!("  Storage backend: {}", spec.storage.backend);
            Ok(())
        }
        Err(e) => {
            println!("✗ Invalid job specification: {}", e);
            Err(e.into())
        }
    }
}
