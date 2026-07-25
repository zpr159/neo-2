//! CLI command structs and placeholder implementations.
//!
//! Every command struct derives `clap::Args` and carries the flags/sub-commands
//! needed for that operation. Actual orchestration is wired up later; the
//! current implementations print what they *would* do.

use clap::{Args, Subcommand};

use crate::cli::NeoCommand;

// ---------------------------------------------------------------------------
// Chat
// ---------------------------------------------------------------------------

/// Send a message to an AI agent and receive a synchronous response.
///
/// When `--message` is omitted the CLI will prompt for input.
#[derive(Debug, Clone, Args)]
pub struct ChatCommand {
    /// The message to send to the agent.
    #[arg(short, long)]
    pub message: Option<String>,

    /// Resume an existing session by ID.
    #[arg(short, long)]
    pub session: Option<String>,

    /// Override the model used for this chat.
    #[arg(short, long)]
    pub model: Option<String>,
}

// ---------------------------------------------------------------------------
// Stream
// ---------------------------------------------------------------------------

/// Stream a response from an AI agent token-by-token.
#[derive(Debug, Clone, Args)]
pub struct StreamCommand {
    /// The message to send to the agent.
    #[arg(short, long)]
    pub message: Option<String>,

    /// Resume an existing session by ID.
    #[arg(short, long)]
    pub session: Option<String>,
}

// ---------------------------------------------------------------------------
// World
// ---------------------------------------------------------------------------

/// Inspect and interact with the world state store.
#[derive(Debug, Clone, Args)]
pub struct WorldCommand {
    #[command(subcommand)]
    pub subcommand: WorldSubcommand,
}

/// Sub-commands for the world state store.
#[derive(Debug, Clone, Subcommand)]
pub enum WorldSubcommand {
    /// List all entities in the world state.
    List,
    /// Get a specific entity by its identifier.
    Get {
        /// Entity ID to retrieve.
        id: String,
    },
    /// Take a snapshot of the current world state.
    Snapshot,
    /// Run a simulation for N steps.
    Simulate {
        /// Number of simulation steps.
        #[arg(short, long, default_value_t = 1)]
        steps: usize,
    },
}

// ---------------------------------------------------------------------------
// Memory
// ---------------------------------------------------------------------------

/// Query and manage the long-term memory store.
#[derive(Debug, Clone, Args)]
pub struct MemoryCommand {
    #[command(subcommand)]
    pub subcommand: MemorySubcommand,
}

/// Sub-commands for the memory store.
#[derive(Debug, Clone, Subcommand)]
pub enum MemorySubcommand {
    /// Search memory for entries matching a query.
    Search {
        /// Search query string.
        #[arg(short, long)]
        query: String,
    },
    /// Store a new entry in memory.
    Store {
        /// Content to store.
        #[arg(short, long)]
        content: String,
    },
    /// Show memory usage statistics.
    Stats,
}

// ---------------------------------------------------------------------------
// Planning
// ---------------------------------------------------------------------------

/// Create and manage execution plans.
#[derive(Debug, Clone, Args)]
pub struct PlanningCommand {
    #[command(subcommand)]
    pub subcommand: PlanningSubcommand,
}

/// Sub-commands for the planning subsystem.
#[derive(Debug, Clone, Subcommand)]
pub enum PlanningSubcommand {
    /// Create a new plan for the given goal.
    Create {
        /// Goal description.
        #[arg(short, long)]
        goal: String,
    },
    /// Retrieve a plan by ID.
    Get {
        /// Plan ID to retrieve.
        id: String,
    },
    /// List all active plans.
    List,
}

// ---------------------------------------------------------------------------
// Workflow
// ---------------------------------------------------------------------------

/// Start, cancel, and inspect workflows.
#[derive(Debug, Clone, Args)]
pub struct WorkflowCommand {
    #[command(subcommand)]
    pub subcommand: WorkflowSubcommand,
}

/// Sub-commands for the workflow subsystem.
#[derive(Debug, Clone, Subcommand)]
pub enum WorkflowSubcommand {
    /// Start a workflow by name.
    Start {
        /// Workflow name to start.
        name: String,
    },
    /// Cancel a running workflow.
    Cancel {
        /// Workflow execution ID.
        id: String,
    },
    /// Show the status of a workflow execution.
    Status {
        /// Workflow execution ID.
        id: String,
    },
}

// ---------------------------------------------------------------------------
// Agent
// ---------------------------------------------------------------------------

/// Manage and inspect running agents.
#[derive(Debug, Clone, Args)]
pub struct AgentCommand {
    #[command(subcommand)]
    pub subcommand: AgentSubcommand,
}

/// Sub-commands for agent management.
#[derive(Debug, Clone, Subcommand)]
pub enum AgentSubcommand {
    /// List all registered agents.
    List,
    /// Start an agent by ID.
    Start {
        /// Agent ID to start.
        id: String,
    },
    /// Stop a running agent.
    Stop {
        /// Agent ID to stop.
        id: String,
    },
    /// Show the status of an agent.
    Status {
        /// Agent ID to inspect.
        id: String,
    },
}

// ---------------------------------------------------------------------------
// Tools
// ---------------------------------------------------------------------------

/// Discover and list available tools.
#[derive(Debug, Clone, Args)]
pub struct ToolsCommand {
    /// List all registered tools.
    #[arg(short, long)]
    pub list: bool,

    /// Run the tool discovery process.
    #[arg(short, long)]
    pub discover: bool,
}

// ---------------------------------------------------------------------------
// Benchmark
// ---------------------------------------------------------------------------

/// Run performance benchmarks against Neo subsystems.
#[derive(Debug, Clone, Args)]
pub struct BenchmarkCommand {
    /// Name of the benchmark suite to run (default: all).
    #[arg(short, long)]
    pub suite: Option<String>,

    /// Number of iterations for each benchmark.
    #[arg(short, long)]
    pub iterations: Option<usize>,
}

// ---------------------------------------------------------------------------
// Status
// ---------------------------------------------------------------------------

/// Show system status information.
#[derive(Debug, Clone, Args)]
pub struct StatusCommand {
    /// Show detailed status including subsystem health.
    #[arg(short, long)]
    pub detailed: bool,
}

// ---------------------------------------------------------------------------
// Metrics
// ---------------------------------------------------------------------------

/// Display runtime metrics.
#[derive(Debug, Clone, Args)]
pub struct MetricsCommand {
    /// Output format: text, json, or prometheus.
    #[arg(short, long)]
    pub format: Option<String>,

    /// Continuously watch metrics (refresh every second).
    #[arg(short, long)]
    pub watch: bool,
}

// ---------------------------------------------------------------------------
// Providers
// ---------------------------------------------------------------------------

/// List configured LLM providers and their health.
#[derive(Debug, Clone, Args)]
pub struct ProvidersCommand {
    /// Include provider health checks in the output.
    #[arg(short, long)]
    pub health: bool,
}

// ---------------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------------

/// List available models from configured providers.
#[derive(Debug, Clone, Args)]
pub struct ModelsCommand {
    /// Filter models by provider name.
    #[arg(short, long)]
    pub provider: Option<String>,
}

// ---------------------------------------------------------------------------
// InteractiveShell
// ---------------------------------------------------------------------------

/// Launches an interactive REPL when no subcommand is provided.
pub struct InteractiveShell;

impl InteractiveShell {
    /// Print the welcome banner to stdout.
    pub fn print_welcome() {
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║              Neo AGI Operating System — CLI                 ║");
        println!("║              Type 'help' for available commands              ║");
        println!("║              Type 'exit' or 'quit' to leave                 ║");
        println!("╚══════════════════════════════════════════════════════════════╝");
        println!();
    }

    /// Print the list of available commands.
    pub fn print_help() {
        println!("Available commands:");
        println!("  chat      - Send a message to an AI agent");
        println!("  stream    - Stream a response token-by-token");
        println!("  world     - Inspect world state");
        println!("  memory    - Query long-term memory");
        println!("  planning  - Manage execution plans");
        println!("  workflow  - Start / cancel / inspect workflows");
        println!("  agent     - Manage running agents");
        println!("  tools     - Discover and list tools");
        println!("  benchmark - Run performance benchmarks");
        println!("  status    - Show system status");
        println!("  metrics   - Display runtime metrics");
        println!("  providers - List LLM providers");
        println!("  models    - List available models");
        println!("  help      - Show this help message");
        println!("  exit      - Exit the shell");
        println!();
    }
}

// ---------------------------------------------------------------------------
// CommandRegistry — dispatch table
// ---------------------------------------------------------------------------

/// Central dispatch: maps a [`NeoCommand`] variant to its handler.
pub struct CommandRegistry;

impl CommandRegistry {
    /// Dispatch the given command against the specified client URL.
    pub fn dispatch(command: &NeoCommand, client_url: &str) {
        match command {
            NeoCommand::Chat(cmd) => Self::handle_chat(cmd, client_url),
            NeoCommand::Stream(cmd) => Self::handle_stream(cmd, client_url),
            NeoCommand::World(cmd) => Self::handle_world(cmd, client_url),
            NeoCommand::Memory(cmd) => Self::handle_memory(cmd, client_url),
            NeoCommand::Planning(cmd) => Self::handle_planning(cmd, client_url),
            NeoCommand::Workflow(cmd) => Self::handle_workflow(cmd, client_url),
            NeoCommand::Agent(cmd) => Self::handle_agent(cmd, client_url),
            NeoCommand::Tools(cmd) => Self::handle_tools(cmd, client_url),
            NeoCommand::Benchmark(cmd) => Self::handle_benchmark(cmd, client_url),
            NeoCommand::Status(cmd) => Self::handle_status(cmd, client_url),
            NeoCommand::Metrics(cmd) => Self::handle_metrics(cmd, client_url),
            NeoCommand::Providers(cmd) => Self::handle_providers(cmd, client_url),
            NeoCommand::Models(cmd) => Self::handle_models(cmd, client_url),
            NeoCommand::Interactive => {
                let session = crate::cli::interactive::InteractiveSession::new(client_url.to_string());
                session.run();
            }
        }
    }

    fn handle_chat(cmd: &ChatCommand, client_url: &str) {
        tracing::info!(session = ?cmd.session, model = ?cmd.model, "chat command");
        match &cmd.message {
            Some(msg) => println!("[chat] ({}) → {}", client_url, msg),
            None => println!("[chat] prompt mode — enter your message:"),
        }
    }

    fn handle_stream(cmd: &StreamCommand, client_url: &str) {
        tracing::info!(session = ?cmd.session, "stream command");
        match &cmd.message {
            Some(msg) => println!("[stream] ({}) → {}", client_url, msg),
            None => println!("[stream] prompt mode — enter your message:"),
        }
    }

    fn handle_world(cmd: &WorldCommand, client_url: &str) {
        tracing::info!("world command");
        match &cmd.subcommand {
            WorldSubcommand::List => println!("[world] listing entities via {}", client_url),
            WorldSubcommand::Get { id } => println!("[world] get entity '{}' via {}", id, client_url),
            WorldSubcommand::Snapshot => println!("[world] taking snapshot via {}", client_url),
            WorldSubcommand::Simulate { steps } => println!("[world] simulating {} steps via {}", steps, client_url),
        }
    }

    fn handle_memory(cmd: &MemoryCommand, client_url: &str) {
        tracing::info!("memory command");
        match &cmd.subcommand {
            MemorySubcommand::Search { query } => println!("[memory] searching '{}' via {}", query, client_url),
            MemorySubcommand::Store { content } => println!("[memory] storing '{}' via {}", content, client_url),
            MemorySubcommand::Stats => println!("[memory] stats via {}", client_url),
        }
    }

    fn handle_planning(cmd: &PlanningCommand, client_url: &str) {
        tracing::info!("planning command");
        match &cmd.subcommand {
            PlanningSubcommand::Create { goal } => println!("[planning] creating plan for '{}' via {}", goal, client_url),
            PlanningSubcommand::Get { id } => println!("[planning] get plan '{}' via {}", id, client_url),
            PlanningSubcommand::List => println!("[planning] listing plans via {}", client_url),
        }
    }

    fn handle_workflow(cmd: &WorkflowCommand, client_url: &str) {
        tracing::info!("workflow command");
        match &cmd.subcommand {
            WorkflowSubcommand::Start { name } => println!("[workflow] starting '{}' via {}", name, client_url),
            WorkflowSubcommand::Cancel { id } => println!("[workflow] cancelling '{}' via {}", id, client_url),
            WorkflowSubcommand::Status { id } => println!("[workflow] status of '{}' via {}", id, client_url),
        }
    }

    fn handle_agent(cmd: &AgentCommand, client_url: &str) {
        tracing::info!("agent command");
        match &cmd.subcommand {
            AgentSubcommand::List => println!("[agent] listing agents via {}", client_url),
            AgentSubcommand::Start { id } => println!("[agent] starting '{}' via {}", id, client_url),
            AgentSubcommand::Stop { id } => println!("[agent] stopping '{}' via {}", id, client_url),
            AgentSubcommand::Status { id } => println!("[agent] status of '{}' via {}", id, client_url),
        }
    }

    fn handle_tools(cmd: &ToolsCommand, client_url: &str) {
        tracing::info!(list = cmd.list, discover = cmd.discover, "tools command");
        if cmd.list {
            println!("[tools] listing tools via {}", client_url);
        }
        if cmd.discover {
            println!("[tools] running discovery via {}", client_url);
        }
        if !cmd.list && !cmd.discover {
            println!("[tools] no action specified — use --list or --discover");
        }
    }

    fn handle_benchmark(cmd: &BenchmarkCommand, client_url: &str) {
        let suite = cmd.suite.as_deref().unwrap_or("all");
        let iters = cmd.iterations.unwrap_or(100);
        tracing::info!(suite, iterations = iters, "benchmark command");
        println!("[benchmark] running suite '{}' ({} iterations) via {}", suite, iters, client_url);
    }

    fn handle_status(cmd: &StatusCommand, client_url: &str) {
        tracing::info!(detailed = cmd.detailed, "status command");
        println!("[status] detailed={} via {}", cmd.detailed, client_url);
    }

    fn handle_metrics(cmd: &MetricsCommand, client_url: &str) {
        let fmt = cmd.format.as_deref().unwrap_or("text");
        tracing::info!(format = fmt, watch = cmd.watch, "metrics command");
        println!("[metrics] format='{}', watch={} via {}", fmt, cmd.watch, client_url);
    }

    fn handle_providers(cmd: &ProvidersCommand, client_url: &str) {
        tracing::info!(health = cmd.health, "providers command");
        println!("[providers] health={} via {}", cmd.health, client_url);
    }

    fn handle_models(cmd: &ModelsCommand, client_url: &str) {
        let provider = cmd.provider.as_deref().unwrap_or("all");
        tracing::info!(provider, "models command");
        println!("[models] provider='{}' via {}", provider, client_url);
    }
}
