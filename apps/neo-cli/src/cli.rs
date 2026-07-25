use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(
    name = "neo",
    version,
    about = "Neo AGI Operating System",
    long_about = None,
    after_help = "Use 'neo <command> --help' for more information on a specific command."
)]
pub(crate) struct Cli {
    #[arg(short, long, global = true)]
    pub config: Option<std::path::PathBuf>,

    #[arg(short, long, global = true, action = clap::ArgAction::SetTrue)]
    pub verbose: bool,

    #[arg(long, global = true, default_value = "info")]
    pub log_level: String,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Start interactive REPL shell
    Shell,
    /// Interactive chat with Neo
    Chat {
        #[arg(short, long)]
        message: Option<String>,
    },
    /// Start HTTP server
    Server {
        #[arg(short, long, default_value = "127.0.0.1")]
        bind: String,
        #[arg(short, long, default_value_t = 8080)]
        port: u16,
    },
    /// Start as background daemon
    Daemon {
        #[arg(short, long)]
        foreground: bool,
    },
    /// Open developer console with live statistics
    Dev,
    /// Show system status
    Status,
    /// Show version information
    Version,
    /// Run system diagnostics
    Doctor,
    /// Run performance benchmarks
    Benchmark {
        #[arg(short, long, default_value = "30")]
        duration_secs: u64,
    },
    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Show loaded models
    Models,
    /// Manage cognitive memory
    Memory {
        #[command(subcommand)]
        action: MemoryAction,
    },
    /// Manage knowledge graph
    Graph {
        #[command(subcommand)]
        action: GraphAction,
    },
    /// Run a reasoning query
    Reasoning {
        query: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum ConfigAction {
    /// Show current configuration
    Show,
    /// Set a configuration value
    Set {
        key: String,
        value: String,
    },
    /// Generate default configuration file
    Init,
    /// Validate current configuration
    Validate,
    /// Open config file in editor
    Edit,
}

#[derive(Subcommand)]
pub(crate) enum MemoryAction {
    /// Show memory statistics
    Stats,
    /// Store a new memory
    Store {
        content: String,
    },
    /// Search memory
    Search {
        query: String,
    },
    /// List recent memories
    List,
}

#[derive(Subcommand)]
pub(crate) enum GraphAction {
    /// Show knowledge graph statistics
    Stats,
    /// List entities
    Entities,
    /// Search knowledge graph
    Search {
        query: String,
    },
    /// Create a new entity
    Create {
        #[arg(short = 't', long)]
        entity_type: String,
        label: String,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum OutputFormat {
    Text,
    Json,
    Table,
}
