use std::path::Path;

use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::config::LoggingConfig;
use crate::error::CliResult;

/// Initialize logging with console output only.
///
/// # Errors
///
/// Returns [`CliError::Io`] if the tracing subscriber fails to initialize.
pub(crate) fn initialize(config: &LoggingConfig) -> CliResult<()> {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&config.level));

    let registry = tracing_subscriber::registry().with(env_filter);

    match config.format.as_str() {
        "json" => {
            let layer = fmt::layer().json().with_target(true).with_thread_ids(true);
            registry.with(layer).init();
        }
        "compact" => {
            let layer = fmt::layer()
                .compact()
                .with_target(true)
                .with_thread_ids(false);
            registry.with(layer).init();
        }
        "pretty" | _ => {
            let layer = fmt::layer()
                .pretty()
                .with_target(true)
                .with_thread_ids(false);
            registry.with(layer).init();
        }
    }

    tracing::info!("logging initialized");
    Ok(())
}

/// Initialize logging with both console and file output.
///
/// # Errors
///
/// Returns [`CliError::Io`] if the log directory cannot be created or the
/// tracing subscriber fails to initialize.
pub(crate) fn initialize_with_file(config: &LoggingConfig) -> CliResult<()> {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&config.level));

    let registry = tracing_subscriber::registry().with(env_filter);

    if config.enable_file_logging {
        let log_dir = Path::new(&config.log_dir);
        std::fs::create_dir_all(log_dir)?;

        let file_appender = tracing_appender::rolling::daily(&config.log_dir, "neo.log");
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

        let file_layer = fmt::layer()
            .with_writer(non_blocking)
            .with_ansi(false)
            .with_target(true);

        let console_layer = fmt::layer()
            .with_target(true)
            .with_thread_ids(false);

        registry.with(file_layer).with(console_layer).init();

        std::mem::forget(_guard);
    } else {
        let layer = fmt::layer()
            .pretty()
            .with_target(true)
            .with_thread_ids(false);
        registry.with(layer).init();
    }

    tracing::info!("logging initialized");
    Ok(())
}
