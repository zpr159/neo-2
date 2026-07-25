use std::path::Path;

use crate::config::AppConfig;
use crate::error::{CliError, CliResult};

struct CheckResult {
    name: String,
    pass: bool,
    message: String,
}

fn check_config_valid(config: &AppConfig) -> CheckResult {
    if config.core.environment.is_empty() {
        return CheckResult {
            name: "Configuration".to_string(),
            pass: false,
            message: "Environment is empty".to_string(),
        };
    }
    if config.core.data_dir.is_empty() {
        return CheckResult {
            name: "Configuration".to_string(),
            pass: false,
            message: "Data directory is empty".to_string(),
        };
    }
    CheckResult {
        name: "Configuration".to_string(),
        pass: true,
        message: format!("environment={}", config.core.environment),
    }
}

fn check_data_dir(config: &AppConfig) -> CheckResult {
    let path = Path::new(&config.core.data_dir);
    match std::fs::create_dir_all(path) {
        Ok(()) => {
            let test_file = path.join(".neo_doctor_test");
            match std::fs::write(&test_file, b"test") {
                Ok(()) => {
                    let _ = std::fs::remove_file(&test_file);
                    CheckResult {
                        name: "Data Directory".to_string(),
                        pass: true,
                        message: format!("{} (writable)", path.display()),
                    }
                }
                Err(e) => CheckResult {
                    name: "Data Directory".to_string(),
                    pass: false,
                    message: format!("{} (not writable: {e})", path.display()),
                },
            }
        }
        Err(e) => CheckResult {
            name: "Data Directory".to_string(),
            pass: false,
            message: format!("{} (create failed: {e})", path.display()),
        },
    }
}

fn check_log_dir(config: &AppConfig) -> CheckResult {
    let path = Path::new(&config.logging.log_dir);
    match std::fs::create_dir_all(path) {
        Ok(()) => CheckResult {
            name: "Log Directory".to_string(),
            pass: true,
            message: format!("{}", path.display()),
        },
        Err(e) => CheckResult {
            name: "Log Directory".to_string(),
            pass: false,
            message: format!("{} (create failed: {e})", path.display()),
        },
    }
}

fn check_runtime() -> CheckResult {
    let runtime_config = neo_runtime::RuntimeConfiguration::development();
    let runtime = neo_runtime::RuntimeManager::new(runtime_config);
    match runtime.initialize() {
        Ok(()) => {
            CheckResult {
                name: "Runtime".to_string(),
                pass: true,
                message: "initialized successfully".to_string(),
            }
        }
        Err(e) => CheckResult {
            name: "Runtime".to_string(),
            pass: false,
            message: format!("initialization failed: {e}"),
        },
    }
}

fn check_executive() -> CheckResult {
    let _api = neo_executive::ExecutiveApi::new(neo_executive::ExecutionMode::Interactive);
    CheckResult {
        name: "Executive".to_string(),
        pass: true,
        message: "initialized successfully".to_string(),
    }
}

fn check_memory() -> CheckResult {
    let config = neo_memory::UnifiedMemoryConfig::default();
    match neo_memory::CognitiveMemoryManager::new(config) {
        Ok(_mem) => CheckResult {
            name: "Memory".to_string(),
            pass: true,
            message: "initialized successfully".to_string(),
        },
        Err(e) => CheckResult {
            name: "Memory".to_string(),
            pass: false,
            message: format!("initialization failed: {e}"),
        },
    }
}

fn check_knowledge() -> CheckResult {
    let _kg = neo_knowledge_graph::NeoKnowledgeGraph::new();
    CheckResult {
        name: "Knowledge Graph".to_string(),
        pass: true,
        message: "initialized successfully".to_string(),
    }
}

fn check_reasoning() -> CheckResult {
    let _r = neo_reasoning::ReasoningOrchestrator::new(neo_reasoning::ReasoningConfig::default());
    CheckResult {
        name: "Reasoning".to_string(),
        pass: true,
        message: "initialized successfully".to_string(),
    }
}

pub async fn run(config: &AppConfig) -> CliResult<()> {
    println!("Neo System Doctor v{}", crate::config::VERSION);
    println!();

    let config_clone = config.clone();
    let checks: Vec<CheckResult> = tokio::task::spawn_blocking(move || {
        vec![
            check_config_valid(&config_clone),
            check_data_dir(&config_clone),
            check_log_dir(&config_clone),
            check_runtime(),
            check_executive(),
            check_memory(),
            check_knowledge(),
            check_reasoning(),
        ]
    })
    .await
    .map_err(|e| CliError::custom(format!("doctor task failed: {e}")))?;

    let mut all_pass = true;
    for check in &checks {
        let (icon, color) = if check.pass {
            ("\u{2713}", "\x1b[32m")
        } else {
            ("\u{2717}", "\x1b[31m")
        };
        let reset = "\x1b[0m";
        println!("  {color}{icon}{reset} {:<20} {}", check.name, check.message);
        if !check.pass {
            all_pass = false;
        }
    }

    println!();
    if all_pass {
        println!("All checks passed.");
        Ok(())
    } else {
        Err(CliError::custom("some checks failed"))
    }
}
