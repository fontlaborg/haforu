// this_file: src/logging.rs
//! Logging configuration and utilities

use env_logger::Builder;
use log::{Level, LevelFilter};
use std::io::Write;

/// Configure logging for debug builds
#[cfg(debug_assertions)]
pub fn default_level() -> &'static str {
    "debug"
}

/// Configure logging for release builds
#[cfg(not(debug_assertions))]
pub fn default_level() -> &'static str {
    "info"
}

/// Initialize structured logging with custom format
pub fn init_logging(level: &str, quiet: bool, timestamps: bool) {
    let level_filter = if quiet {
        LevelFilter::Error
    } else {
        match level.to_lowercase().as_str() {
            "trace" => LevelFilter::Trace,
            "debug" => LevelFilter::Debug,
            "info" => LevelFilter::Info,
            "warn" | "warning" => LevelFilter::Warn,
            "error" => LevelFilter::Error,
            "off" => LevelFilter::Off,
            _ => {
                eprintln!("Invalid log level '{}', using 'info'", level);
                LevelFilter::Info
            }
        }
    };

    let mut builder = Builder::new();
    builder.filter_level(level_filter);

    // Custom format for better readability
    builder.format(move |buf, record| {
        let level_style = match record.level() {
            Level::Error => "\x1b[31m", // Red
            Level::Warn => "\x1b[33m",  // Yellow
            Level::Info => "\x1b[32m",  // Green
            Level::Debug => "\x1b[34m", // Blue
            Level::Trace => "\x1b[35m", // Magenta
        };
        let reset = "\x1b[0m";

        if timestamps {
            writeln!(
                buf,
                "{} {}{:5}{} [{}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                level_style,
                record.level(),
                reset,
                record.target(),
                record.args()
            )
        } else {
            writeln!(
                buf,
                "{}{:5}{} [{}] {}",
                level_style,
                record.level(),
                reset,
                record.target(),
                record.args()
            )
        }
    });

    // Parse any environment variables
    if let Ok(rust_log) = std::env::var("RUST_LOG") {
        builder.parse_filters(&rust_log);
    }

    builder.init();
}

/// Log a message with context about the current operation
#[macro_export]
macro_rules! log_context {
    ($level:expr, $context:expr, $($arg:tt)*) => {
        log::log!($level, "[{}] {}", $context, format!($($arg)*));
    };
}

/// Log timing information for an operation
pub struct Timer {
    name: String,
    start: std::time::Instant,
}

impl Timer {
    /// Start a new timer
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        log::debug!("Starting: {}", name);
        Self {
            name,
            start: std::time::Instant::now(),
        }
    }

    /// Log the elapsed time at the specified level
    pub fn log_elapsed(&self, level: Level) {
        let elapsed = self.start.elapsed();
        log::log!(
            level,
            "{} completed in {:.3}ms",
            self.name,
            elapsed.as_secs_f64() * 1000.0
        );
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        self.log_elapsed(Level::Debug);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_level() {
        #[cfg(debug_assertions)]
        assert_eq!(default_level(), "debug");

        #[cfg(not(debug_assertions))]
        assert_eq!(default_level(), "info");
    }

    #[test]
    fn test_timer_creation() {
        let _timer = Timer::new("test operation");
        // Timer will log when dropped
    }
}
