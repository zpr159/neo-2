use std::sync::Arc;

use crate::bootstrap::NeoSystem;
use crate::config::AppConfig;
use crate::error::{CliError, CliResult};

fn write_pid_file(path: &str) -> CliResult<()> {
    let pid = std::process::id();
    let content = pid.to_string();
    if let Some(parent) = std::path::Path::new(path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)?;
    Ok(())
}

fn remove_pid_file(path: &str) {
    let _ = std::fs::remove_file(path);
}

async fn watchdog_loop(
    system: &NeoSystem,
    config: &AppConfig,
    shutdown: tokio::sync::watch::Receiver<bool>,
) -> CliResult<()> {
    let interval = std::time::Duration::from_secs(config.daemon.watchdog_interval_secs);
    let mut shutdown_rx = shutdown;
    loop {
        tokio::select! {
            _ = tokio::time::sleep(interval) => {
                let running = system.runtime.is_running();
                if running {
                    tracing::debug!("watchdog: system healthy");
                } else {
                    tracing::warn!("watchdog: system not running");
                }
            }
            _ = shutdown_rx.changed() => {
                tracing::info!("watchdog: shutdown signal received");
                break;
            }
        }
    }
    Ok(())
}

pub async fn run(
    system: &Arc<NeoSystem>,
    config: &AppConfig,
    foreground: bool,
) -> CliResult<()> {
    write_pid_file(&config.daemon.pid_file)?;

    let _guard = PidFileGuard(&config.daemon.pid_file);

    println!("Neo daemon starting (PID: {})", std::process::id());
    println!("  PID file:    {}", config.daemon.pid_file);
    println!("  Watchdog:    {}s", config.daemon.watchdog_interval_secs);
    println!("  Auto-restart: {}", config.daemon.auto_restart);
    if foreground {
        println!("  Mode:        foreground");
    }
    println!();

    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    tokio::spawn({
        let shutdown_tx = shutdown_tx.clone();
        async move {
            let _ = tokio::signal::ctrl_c().await;
            tracing::info!("SIGINT received");
            let _ = shutdown_tx.send(true);
        }
    });

    #[cfg(unix)]
    {
        let shutdown_tx = shutdown_tx.clone();
        tokio::spawn(async move {
            use tokio::signal::unix::{signal, SignalKind};
            let mut sigterm = signal(SignalKind::terminate()).expect("failed to register SIGTERM handler");
            let mut sigquit = signal(SignalKind::quit()).expect("failed to register SIGQUIT handler");
            tokio::select! {
                _ = sigterm.recv() => {
                    tracing::info!("SIGTERM received");
                }
                _ = sigquit.recv() => {
                    tracing::info!("SIGQUIT received");
                }
            }
            let _ = shutdown_tx.send(true);
        });
    }

    let mut restart_count: u32 = 0;
    let max_restarts = config.daemon.max_restart_attempts;

    loop {
        tracing::info!("daemon watchdog loop starting");

        match watchdog_loop(system, config, shutdown_rx.clone()).await {
            Ok(()) => {
                tracing::info!("daemon shutting down gracefully");
                break;
            }
            Err(e) => {
                tracing::error!("watchdog error: {e}");
                if !config.daemon.auto_restart || restart_count >= max_restarts {
                    tracing::error!("max restart attempts ({max_restarts}) reached or auto-restart disabled");
                    return Err(CliError::daemon(format!("daemon failed: {e}")));
                }
                restart_count += 1;
                tracing::info!(
                    restart_count,
                    max_restarts,
                    delay_secs = config.daemon.restart_delay_secs,
                    "restarting daemon"
                );
                let delay = std::time::Duration::from_secs(config.daemon.restart_delay_secs);
                tokio::time::sleep(delay).await;
            }
        }
    }

    println!("Daemon stopped.");
    Ok(())
}

struct PidFileGuard<'a>(&'a str);

impl<'a> Drop for PidFileGuard<'a> {
    fn drop(&mut self) {
        remove_pid_file(self.0);
    }
}
