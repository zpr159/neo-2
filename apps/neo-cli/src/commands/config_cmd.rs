use crate::cli::ConfigAction;
use crate::config::AppConfig;
use crate::error::{CliError, CliResult};

pub fn run(action: &ConfigAction, config: &AppConfig) -> CliResult<()> {
    match action {
        ConfigAction::Show => {
            let content = toml::to_string_pretty(config)
                .map_err(|e| CliError::config(format!("failed to serialize: {e}")))?;
            println!("{content}");
        }
        ConfigAction::Set { key, value } => {
            println!("Setting {key} = {value}");
            println!("Note: runtime config changes are not persisted automatically.");
            println!("Use 'neo config init' to generate a config file.");
        }
        ConfigAction::Init => {
            let default = AppConfig::default();
            let content = toml::to_string_pretty(&default)
                .map_err(|e| CliError::config(format!("failed to serialize: {e}")))?;
            let path = std::path::PathBuf::from("neo.toml");
            std::fs::write(&path, content)?;
            println!("Config file created: {}", path.display());
        }
        ConfigAction::Validate => {
            let mut valid = true;

            if config.core.environment.is_empty() {
                println!("  \u{2717} core.environment is empty");
                valid = false;
            } else {
                println!("  \u{2713} core.environment = {}", config.core.environment);
            }

            if config.core.data_dir.is_empty() {
                println!("  \u{2717} core.data_dir is empty");
                valid = false;
            } else {
                println!("  \u{2713} core.data_dir = {}", config.core.data_dir);
            }

            if config.logging.level.is_empty() {
                println!("  \u{2717} logging.level is empty");
                valid = false;
            } else {
                println!("  \u{2713} logging.level = {}", config.logging.level);
            }

            if config.network.port == 0 {
                println!("  \u{2717} network.port is 0");
                valid = false;
            } else {
                println!("  \u{2713} network.port = {}", config.network.port);
            }

            if config.shell.history_size == 0 {
                println!("  \u{2717} shell.history_size is 0");
                valid = false;
            } else {
                println!("  \u{2713} shell.history_size = {}", config.shell.history_size);
            }

            println!();
            if valid {
                println!("Configuration is valid.");
            } else {
                println!("Configuration has errors.");
            }
        }
        ConfigAction::Edit => {
            let path = AppConfig::default_config_path();
            if path.exists() {
                println!("Opening config at: {}", path.display());
                println!("(Editor not available in this context; edit manually)");
            } else {
                println!("No config file found at: {}", path.display());
                println!("Run 'neo config init' to create one.");
            }
        }
    }
    Ok(())
}
