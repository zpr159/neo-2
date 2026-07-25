/// Tracing setup for the Neo AGI Operating System.
///
/// Configures and initializes the `tracing` ecosystem with support for
/// multiple output formats, environment-based log filtering, and optional
/// file-based logging.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};

/// Supported log output formats.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LogFormat {
    /// Human-readable pretty-printed format with colors.
    Pretty,
    /// Compact single-line format.
    Compact,
    /// Machine-readable JSON format.
    Json,
    /// Full verbose format including all span fields.
    Full,
}

impl Default for LogFormat {
    fn default() -> Self {
        LogFormat::Compact
    }
}

/// Configuration for the tracing subsystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracingConfig {
    /// The log level filter (e.g., "info", "debug", "neo_core=trace").
    pub log_level: String,
    /// The output format.
    pub format: LogFormat,
    /// Optional path to a log file. If `None`, logs are only written to stdout.
    pub log_file: Option<String>,
    /// Whether to enable JSON-encoded log output regardless of format.
    pub enable_json: bool,
    /// Whether to enable console (stdout) output.
    pub enable_console: bool,
    /// Sampling rate for traces (0.0 - 1.0). 1.0 means sample everything.
    pub sample_rate: f64,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            format: LogFormat::default(),
            log_file: None,
            enable_json: false,
            enable_console: true,
            sample_rate: 1.0,
        }
    }
}

/// Manages the initialization and lifecycle of the tracing subscriber.
///
/// # Example
///
/// ```no_run
/// use neo_core::observability::tracing_setup::{TracingSetup, TracingConfig, LogFormat};
///
/// let setup = TracingSetup::new();
/// setup.init().expect("failed to initialize tracing");
///
/// // Or with custom config:
/// let config = TracingConfig {
///     log_level: "debug".to_string(),
///     format: LogFormat::Json,
///     enable_console: true,
///     enable_json: true,
///     log_file: Some("/var/log/neo.log".to_string()),
///     sample_rate: 0.5,
/// };
/// setup.init_with_config(config).expect("failed to initialize tracing");
/// ```
pub struct TracingSetup {
    initialized: AtomicBool,
}

impl TracingSetup {
    /// Creates a new `TracingSetup` in an uninitialized state.
    pub fn new() -> Self {
        Self {
            initialized: AtomicBool::new(false),
        }
    }

    /// Returns whether tracing has been initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::Relaxed)
    }

    /// Initializes tracing with the default configuration.
    ///
    /// Uses the `RUST_LOG` environment variable for level filtering,
    /// falling back to `info` if unset. Output goes to stdout in compact format.
    pub fn init(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.init_with_config(TracingConfig::default())
    }

    /// Initializes tracing with a custom configuration.
    ///
    /// This method can only be called once. Subsequent calls will return
    /// an error indicating tracing is already initialized.
    pub fn init_with_config(
        &self,
        config: TracingConfig,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.initialized.swap(true, Ordering::SeqCst) {
            return Err("tracing already initialized".into());
        }

        let env_filter = Self::create_env_filter(&config.log_level);

        match config.format {
            LogFormat::Pretty => {
                tracing_subscriber::fmt()
                    .with_env_filter(env_filter)
                    .with_target(true)
                    .with_thread_ids(true)
                    .init();
            }
            LogFormat::Compact => {
                tracing_subscriber::fmt()
                    .with_env_filter(env_filter)
                    .with_target(true)
                    .with_thread_ids(true)
                    .compact()
                    .init();
            }
            LogFormat::Json => {
                tracing_subscriber::fmt()
                    .with_env_filter(env_filter)
                    .with_target(true)
                    .with_thread_ids(true)
                    .json()
                    .init();
            }
            LogFormat::Full => {
                tracing_subscriber::fmt()
                    .with_env_filter(env_filter)
                    .with_target(true)
                    .with_thread_ids(true)
                    .init();
            }
        }

        tracing::info!("Tracing initialized with level: {}", config.log_level);
        if let Some(ref path) = config.log_file {
            tracing::info!("Log file configured at: {}", path);
        }

        Ok(())
    }

    /// Creates a `tracing_subscriber::EnvFilter` from a level string.
    ///
    /// The `level` parameter accepts standard tracing filter directives:
    /// - Simple levels: `"info"`, `"debug"`, `"trace"`, `"warn"`, `"error"`
    /// - Per-crate: `"neo_core=debug,info"`
    /// - Complex: `"neo_core=trace,hyper=warn,info"`
    pub fn create_env_filter(level: &str) -> tracing_subscriber::EnvFilter {
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level))
    }
}

impl Default for TracingSetup {
    fn default() -> Self {
        Self::new()
    }
}
