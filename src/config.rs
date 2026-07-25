use serde::{Deserialize, Serialize};

use crate::error::{NeoError, NeoResult};
use crate::language::config::LanguageEngineConfig;
use crate::research::config::ResearchConfig;
use crate::types::Environment;

/// Top-level Neo configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeoConfig {
    pub core: CoreConfig,
    pub runtime: RuntimeConfig,
    pub neural: NeuralConfig,
    pub inference: InferenceConfig,
    pub memory: MemoryConfig,
    pub knowledge_graph: KnowledgeGraphConfig,
    pub reasoning: ReasoningConfig,
    pub executive: ExecutiveConfig,
    pub agents: AgentsConfig,
    pub security: SecurityConfig,
    pub logging: LoggingConfig,
    pub tracing: TracingConfig,
    pub network: NetworkConfig,
    pub distributed: DistributedConfig,
    pub plugins: PluginsConfig,
    pub language: LanguageEngineConfig,
    pub research: ResearchConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreConfig {
    pub name: String,
    pub version: String,
    pub environment: Environment,
    pub data_dir: String,
    pub log_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub max_threads: usize,
    pub async_runtime: String,
    pub shutdown_timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuralConfig {
    pub enabled: bool,
    pub backend: String,
    pub device: String,
    pub precision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceConfig {
    pub max_batch_size: usize,
    pub timeout_ms: u64,
    pub cache_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub max_heap_bytes: usize,
    pub max_stack_bytes: usize,
    pub gc_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGraphConfig {
    pub enabled: bool,
    pub storage_path: String,
    pub max_nodes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningConfig {
    pub engine: String,
    pub max_depth: usize,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutiveConfig {
    pub max_concurrent_tasks: usize,
    pub preemption_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentsConfig {
    pub max_agents: usize,
    pub heartbeat_interval_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub sandbox_default_level: String,
    pub allowed_permissions: Vec<String>,
    pub encryption_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
    pub output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracingConfig {
    pub enabled: bool,
    pub endpoint: String,
    pub sample_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub listen_address: String,
    pub listen_port: u16,
    pub max_connections: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedConfig {
    pub enabled: bool,
    pub node_id: String,
    pub cluster_peers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginsConfig {
    pub enabled: bool,
    pub directory: String,
    pub auto_load: Vec<String>,
}

impl Default for NeoConfig {
    fn default() -> Self {
        Self {
            core: CoreConfig {
                name: "neo".to_string(),
                version: "0.1.0".to_string(),
                environment: Environment::Development,
                data_dir: "./data".to_string(),
                log_dir: "./logs".to_string(),
            },
            runtime: RuntimeConfig {
                max_threads: num_cpus().max(4),
                async_runtime: "tokio".to_string(),
                shutdown_timeout_secs: 30,
            },
            neural: NeuralConfig {
                enabled: true,
                backend: "cpu".to_string(),
                device: "auto".to_string(),
                precision: "fp32".to_string(),
            },
            inference: InferenceConfig {
                max_batch_size: 32,
                timeout_ms: 5000,
                cache_size: 1024,
            },
            memory: MemoryConfig {
                max_heap_bytes: 1 << 30,
                max_stack_bytes: 8 * (1 << 20),
                gc_threshold: 0.75,
            },
            knowledge_graph: KnowledgeGraphConfig {
                enabled: true,
                storage_path: "./data/knowledge_graph".to_string(),
                max_nodes: 1_000_000,
            },
            reasoning: ReasoningConfig {
                engine: "default".to_string(),
                max_depth: 128,
                timeout_ms: 10000,
            },
            executive: ExecutiveConfig {
                max_concurrent_tasks: 16,
                preemption_enabled: true,
            },
            agents: AgentsConfig {
                max_agents: 64,
                heartbeat_interval_secs: 10,
            },
            security: SecurityConfig {
                sandbox_default_level: "permissive".to_string(),
                allowed_permissions: vec!["read".to_string(), "write".to_string()],
                encryption_enabled: true,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "pretty".to_string(),
                output: "stdout".to_string(),
            },
            tracing: TracingConfig {
                enabled: false,
                endpoint: "http://localhost:4317".to_string(),
                sample_rate: 0.1,
            },
            network: NetworkConfig {
                listen_address: "127.0.0.1".to_string(),
                listen_port: 7600,
                max_connections: 1024,
            },
            distributed: DistributedConfig {
                enabled: false,
                node_id: "node-1".to_string(),
                cluster_peers: Vec::new(),
            },
            plugins: PluginsConfig {
                enabled: true,
                directory: "./plugins".to_string(),
                auto_load: Vec::new(),
            },
            language: LanguageEngineConfig::default(),
            research: ResearchConfig::default(),
        }
    }
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

impl NeoConfig {
    /// Load configuration for a given profile by overlaying environment-specific
    /// values on top of the compiled defaults.
    pub fn load(profile: &str) -> NeoResult<Self> {
        let mut config = config::Config::builder();

        let defaults_toml = include_str!("defaults.toml");
        config = config.add_source(config::File::from_str(defaults_toml, config::FileFormat::Toml));

        let profile_path = std::path::PathBuf::from(format!("config/{}.toml", profile));
        if profile_path.exists() {
            config = config.add_source(config::File::from(profile_path));
        }

        let env_source = config::Environment::with_prefix("NEO")
            .try_parsing(true)
            .separator("__");
        config = config.add_source(env_source);

        let built = config
            .build()
            .map_err(|e| NeoError::Config(e.to_string()))?;

        let value: toml::Value = built
            .try_deserialize()
            .map_err(|e| NeoError::Config(e.to_string()))?;

        let result: NeoConfig =
            serde_json::from_value(serde_json::to_value(value).map_err(NeoError::Serialization)?)
                .map_err(|e| NeoError::Config(e.to_string()))?;

        Ok(result)
    }

    /// Build configuration purely from NEO_* environment variables.
    pub fn from_env() -> NeoResult<Self> {
        let mut config = Self::default();

        if let Ok(val) = std::env::var("NEO__CORE__NAME") {
            config.core.name = val;
        }
        if let Ok(val) = std::env::var("NEO__CORE__VERSION") {
            config.core.version = val;
        }
        if let Ok(val) = std::env::var("NEO__CORE__DATA_DIR") {
            config.core.data_dir = val;
        }
        if let Ok(val) = std::env::var("NEO__CORE__LOG_DIR") {
            config.core.log_dir = val;
        }
        if let Ok(val) = std::env::var("NEO__RUNTIME__MAX_THREADS") {
            if let Ok(n) = val.parse() {
                config.runtime.max_threads = n;
            }
        }
        if let Ok(val) = std::env::var("NEO__NETWORK__LISTEN_PORT") {
            if let Ok(n) = val.parse() {
                config.network.listen_port = n;
            }
        }
        if let Ok(val) = std::env::var("NEO__NETWORK__LISTEN_ADDRESS") {
            config.network.listen_address = val;
        }
        if let Ok(val) = std::env::var("NEO__LOGGING__LEVEL") {
            config.logging.level = val;
        }
        if let Ok(val) = std::env::var("NEO__SECURITY__ENCRYPTION_ENABLED") {
            if let Ok(b) = val.parse() {
                config.security.encryption_enabled = b;
            }
        }

        // Language engine configuration
        if let Ok(val) = std::env::var("NEO__LANGUAGE__ACTIVE_PROVIDER") {
            config.language.active_provider = val;
        }
        if let Ok(val) = std::env::var("NEO__LANGUAGE__TIMEOUT_MS") {
            if let Ok(n) = val.parse() {
                config.language.timeout_ms = n;
            }
        }
        if let Ok(val) = std::env::var("NEO__LANGUAGE__RETRY_COUNT") {
            if let Ok(n) = val.parse() {
                config.language.retry_count = n;
            }
        }
        if let Ok(val) = std::env::var("NEO__LANGUAGE__ENABLE_FAILOVER") {
            if let Ok(b) = val.parse() {
                config.language.enable_failover = b;
            }
        }

        Ok(config)
    }
}
