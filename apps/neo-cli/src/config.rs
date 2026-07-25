use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{CliError, CliResult};

pub const APP_NAME: &str = "neo";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DEFAULT_CONFIG_FILE: &str = "neo.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub core: CoreConfig,
    pub logging: LoggingConfig,
    pub network: NetworkConfig,
    pub shell: ShellConfig,
    pub server: ServerConfig,
    pub daemon: DaemonConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreConfig {
    pub environment: String,
    pub debug: bool,
    pub data_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
    pub output: String,
    pub enable_file_logging: bool,
    pub log_dir: String,
    pub max_file_size_mb: u64,
    pub max_files: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub bind_address: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellConfig {
    pub history_size: usize,
    pub enable_completion: bool,
    pub prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub enable_rest: bool,
    pub enable_websocket: bool,
    pub enable_grpc: bool,
    pub max_connections: usize,
    pub request_timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub pid_file: String,
    pub auto_restart: bool,
    pub max_restart_attempts: u32,
    pub restart_delay_secs: u64,
    pub watchdog_interval_secs: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            core: CoreConfig {
                environment: "development".to_string(),
                debug: true,
                data_dir: "./data".to_string(),
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "pretty".to_string(),
                output: "stdout".to_string(),
                enable_file_logging: true,
                log_dir: "./logs".to_string(),
                max_file_size_mb: 100,
                max_files: 10,
            },
            network: NetworkConfig {
                bind_address: "127.0.0.1".to_string(),
                port: 8080,
            },
            shell: ShellConfig {
                history_size: 10_000,
                enable_completion: true,
                prompt: "neo> ".to_string(),
            },
            server: ServerConfig {
                enable_rest: true,
                enable_websocket: true,
                enable_grpc: false,
                max_connections: 128,
                request_timeout_secs: 30,
            },
            daemon: DaemonConfig {
                pid_file: "./neo.pid".to_string(),
                auto_restart: true,
                max_restart_attempts: 3,
                restart_delay_secs: 5,
                watchdog_interval_secs: 10,
            },
        }
    }
}

impl AppConfig {
    pub fn load(config_path: Option<&Path>) -> CliResult<Self> {
        let path = match config_path {
            Some(p) => p.to_path_buf(),
            None => Self::default_config_path(),
        };

        if path.exists() {
            let content = std::fs::read_to_string(&path).map_err(|e| {
                CliError::config(format!(
                    "failed to read config '{}': {e}",
                    path.display()
                ))
            })?;
            let config: AppConfig = toml::from_str(&content).map_err(|e| {
                CliError::config(format!(
                    "failed to parse config '{}': {e}",
                    path.display()
                ))
            })?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    pub fn load_with_env(config_path: Option<&Path>) -> CliResult<Self> {
        let mut config = Self::load(config_path)?;
        config.apply_env_overrides();
        Ok(config)
    }

    fn apply_env_overrides(&mut self) {
        if let Ok(val) = std::env::var("NEO_ENVIRONMENT") {
            self.core.environment = val;
        }
        if let Ok(val) = std::env::var("NEO_DEBUG") {
            self.core.debug = val.parse().unwrap_or(self.core.debug);
        }
        if let Ok(val) = std::env::var("NEO_LOG_LEVEL") {
            self.logging.level = val;
        }
        if let Ok(val) = std::env::var("NEO_PORT") {
            if let Ok(port) = val.parse() {
                self.network.port = port;
            }
        }
        if let Ok(val) = std::env::var("NEO_BIND_ADDRESS") {
            self.network.bind_address = val;
        }
        if let Ok(val) = std::env::var("NEO_DATA_DIR") {
            self.core.data_dir = val;
        }
        if let Ok(val) = std::env::var("NEO_LOG_DIR") {
            self.logging.log_dir = val;
        }
    }

    pub fn default_config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(APP_NAME)
            .join(DEFAULT_CONFIG_FILE)
    }

    pub fn data_dir(&self) -> PathBuf {
        PathBuf::from(&self.core.data_dir)
    }

    pub fn log_dir(&self) -> PathBuf {
        PathBuf::from(&self.logging.log_dir)
    }

    pub fn is_production(&self) -> bool {
        self.core.environment == "production"
    }

    pub fn save(&self, path: &Path) -> CliResult<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| CliError::config(format!("failed to serialize config: {e}")))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
        Ok(())
    }
}
