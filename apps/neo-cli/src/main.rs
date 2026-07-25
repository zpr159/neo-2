//! Neo AGI OS — Command-line interface and application entry point.
//!
//! This binary provides the `neo` command which bootstraps, configures,
//! and orchestrates the entire Neo system from a single executable.

#![allow(clippy::module_name_repetitions)]
#![allow(unreachable_pub)]
#![allow(dead_code)]
#![allow(missing_docs)]

mod banner;
mod bootstrap;
mod chat;
mod cli;
mod commands;
mod config;
mod console;
mod daemon;
mod doctor;
mod error;
mod logging;
mod server;
mod shell;

use clap::Parser;
use cli::{Cli, Commands};
use config::AppConfig;

fn main() {
    let cli = Cli::parse();

    let config_path = cli.config.as_deref();
    let config = AppConfig::load_with_env(config_path).unwrap_or_else(|e| {
        eprintln!("Failed to load config: {e}");
        AppConfig::default()
    });

    if let Err(e) = logging::initialize_with_file(&config.logging) {
        eprintln!("Failed to initialize logging: {e}");
    }

    tracing::info!("neo cli starting");

    let (exit_code, system) = {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to create tokio runtime");

        let result = rt.block_on(async_main(cli, config));

        drop(rt);

        result
    };

    drop(system);

    std::process::exit(exit_code);
}

async fn async_main(
    cli: cli::Cli,
    config: AppConfig,
) -> (i32, Option<std::sync::Arc<bootstrap::NeoSystem>>) {
    match cli.command {
        Commands::Shell => cmd_shell(config).await,
        Commands::Chat { message } => cmd_chat(config, message).await,
        Commands::Server { bind, port } => cmd_server(config, bind, port).await,
        Commands::Daemon { foreground } => cmd_daemon(config, foreground).await,
        Commands::Dev => cmd_dev(config).await,
        Commands::Status => cmd_status(config).await,
        Commands::Version => {
            let _ = commands::version::run();
            (0, None)
        }
        Commands::Doctor => (cmd_doctor(config).await.0, None),
        Commands::Benchmark { duration_secs } => cmd_benchmark(config, duration_secs).await,
        Commands::Config { action } => {
            let _ = commands::config_cmd::run(&action, &config);
            (0, None)
        }
        Commands::Models => {
            let _ = commands::models::run();
            (0, None)
        }
        Commands::Memory { action } => cmd_memory(config, action).await,
        Commands::Graph { action } => cmd_graph(config, action).await,
        Commands::Reasoning { query } => cmd_reasoning(config, query).await,
    }
}

async fn cmd_shell(config: AppConfig) -> (i32, Option<std::sync::Arc<bootstrap::NeoSystem>>) {
    banner::print_startup_banner(&config);
    let system = match bootstrap_blocking(config).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            return (1, None);
        }
    };
    banner::print_module_status(&system.module_status());
    let code = match shell::run(&system).await {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{e}");
            1
        }
    };
    (code, Some(system))
}

async fn cmd_chat(
    config: AppConfig,
    message: Option<String>,
) -> (i32, Option<std::sync::Arc<bootstrap::NeoSystem>>) {
    banner::print_startup_banner(&config);
    let system = match bootstrap_blocking(config).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            return (1, None);
        }
    };
    banner::print_module_status(&system.module_status());
    if let Some(msg) = message {
        println!("Neo> {msg}");
        (0, Some(system))
    } else {
        let code = match chat::run(&system).await {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("{e}");
                1
            }
        };
        (code, Some(system))
    }
}

async fn cmd_server(
    config: AppConfig,
    bind: String,
    port: u16,
) -> (i32, Option<std::sync::Arc<bootstrap::NeoSystem>>) {
    banner::print_startup_banner(&config);
    let system = match bootstrap_blocking(config).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            return (1, None);
        }
    };
    banner::print_module_status(&system.module_status());
    let code = match server::run(&system, &bind, port).await {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{e}");
            1
        }
    };
    (code, Some(system))
}

async fn cmd_daemon(
    config: AppConfig,
    foreground: bool,
) -> (i32, Option<std::sync::Arc<bootstrap::NeoSystem>>) {
    banner::print_startup_banner(&config);
    let daemon_config = config.clone();
    let system = match bootstrap_blocking(config).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            return (1, None);
        }
    };
    banner::print_module_status(&system.module_status());
    let code = match daemon::run(&system, &daemon_config, foreground).await {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{e}");
            1
        }
    };
    (code, Some(system))
}

async fn cmd_dev(config: AppConfig) -> (i32, Option<std::sync::Arc<bootstrap::NeoSystem>>) {
    banner::print_startup_banner(&config);
    let system = match bootstrap_blocking(config).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            return (1, None);
        }
    };
    banner::print_module_status(&system.module_status());
    let code = match console::run(&system).await {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{e}");
            1
        }
    };
    (code, Some(system))
}

async fn cmd_status(config: AppConfig) -> (i32, Option<std::sync::Arc<bootstrap::NeoSystem>>) {
    banner::print_startup_banner(&config);
    let system = match bootstrap_blocking(config).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            return (1, None);
        }
    };
    banner::print_module_status(&system.module_status());
    let code = match commands::status::run(&system).await {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{e}");
            1
        }
    };
    (code, Some(system))
}

async fn cmd_doctor(config: AppConfig) -> (i32, Option<std::sync::Arc<bootstrap::NeoSystem>>) {
    banner::print_startup_banner(&config);
    let result = doctor::run(&config).await;
    match result {
        Ok(()) => (0, None),
        Err(_) => (1, None),
    }
}

async fn cmd_benchmark(
    config: AppConfig,
    duration_secs: u64,
) -> (i32, Option<std::sync::Arc<bootstrap::NeoSystem>>) {
    banner::print_startup_banner(&config);
    let system = match bootstrap_blocking(config).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            return (1, None);
        }
    };
    banner::print_module_status(&system.module_status());
    let code = match commands::benchmark::run(&system, duration_secs).await {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{e}");
            1
        }
    };
    (code, Some(system))
}

async fn cmd_memory(
    config: AppConfig,
    action: cli::MemoryAction,
) -> (i32, Option<std::sync::Arc<bootstrap::NeoSystem>>) {
    banner::print_startup_banner(&config);
    let system = match bootstrap_blocking(config).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            return (1, None);
        }
    };
    banner::print_module_status(&system.module_status());
    let code = match commands::memory_cmd::run(&system, &action).await {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{e}");
            1
        }
    };
    (code, Some(system))
}

async fn cmd_graph(
    config: AppConfig,
    action: cli::GraphAction,
) -> (i32, Option<std::sync::Arc<bootstrap::NeoSystem>>) {
    banner::print_startup_banner(&config);
    let system = match bootstrap_blocking(config).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            return (1, None);
        }
    };
    banner::print_module_status(&system.module_status());
    let code = match commands::graph_cmd::run(&system, &action).await {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{e}");
            1
        }
    };
    (code, Some(system))
}

async fn cmd_reasoning(
    config: AppConfig,
    query: String,
) -> (i32, Option<std::sync::Arc<bootstrap::NeoSystem>>) {
    banner::print_startup_banner(&config);
    let system = match bootstrap_blocking(config).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            return (1, None);
        }
    };
    banner::print_module_status(&system.module_status());
    let code = match commands::reasoning_cmd::run(&system, &query).await {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{e}");
            1
        }
    };
    (code, Some(system))
}

async fn bootstrap_blocking(
    config: AppConfig,
) -> Result<std::sync::Arc<bootstrap::NeoSystem>, error::CliError> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    std::thread::spawn(move || {
        let result = (|| -> error::CliResult<bootstrap::NeoSystem> {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            rt.block_on(bootstrap::NeoSystem::bootstrap(config))
        })();
        let _ = tx.send(result);
    });
    let system = rx
        .await
        .map_err(|e| error::CliError::bootstrap(format!("bootstrap channel closed: {e}")))??;
    Ok(std::sync::Arc::new(system))
}
