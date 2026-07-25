//! CLI entry point for the Neo AGI Operating System.
//!
//! Provides the top-level [`NeoCli`] parser built on clap derive macros,
//! a [`NeoCommand`] subcommand enum, and an interactive shell mode.

pub mod commands;
pub mod interactive;

use clap::Parser;

pub use commands::{CommandRegistry, InteractiveShell};
pub use interactive::InteractiveSession;

use crate::cli::commands::*;

/// Top-level CLI entry point for Neo.
///
/// Parse with [`NeoCli::parse_args`] and dispatch with [`NeoCli::execute`].
///
/// # Examples
///
/// ```no_run
/// use neo_core::cli::NeoCli;
/// NeoCli::parse_args().execute();
/// ```
#[derive(Parser)]
#[command(
    name = "neo",
    version,
    about = "Neo AGI Operating System — CLI and interactive shell",
    long_about = "The Neo CLI provides commands for chatting with AI agents, \
                  managing world state, memory, planning, workflows, tool \
                  discovery, benchmarking, and system diagnostics.\n\n\
                  Run without arguments to enter the interactive shell."
)]
pub struct CliArgs {
    /// The subcommand to execute.
    #[command(subcommand)]
    pub command: Option<NeoCommand>,

    /// Enable verbose logging output.
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Path to the Neo configuration file.
    #[arg(short, long, global = true, env = "NEO_CONFIG")]
    pub config: Option<String>,

    /// URL of the Neo client API server.
    #[arg(long, global = true, env = "NEO_CLIENT_URL", default_value = "http://localhost:8080")]
    pub client_url: String,
}

/// All available sub-commands for the Neo CLI.
#[derive(Debug, Clone, clap::Subcommand)]
pub enum NeoCommand {
    /// Send a message to an AI agent and receive a response.
    Chat(ChatCommand),

    /// Stream a response from an AI agent token-by-token.
    Stream(StreamCommand),

    /// Inspect and interact with the world state store.
    World(WorldCommand),

    /// Query and manage the long-term memory store.
    Memory(MemoryCommand),

    /// Create and manage execution plans.
    Planning(PlanningCommand),

    /// Start, cancel, and inspect workflows.
    Workflow(WorkflowCommand),

    /// Manage and inspect running agents.
    Agent(AgentCommand),

    /// Discover and list available tools.
    Tools(ToolsCommand),

    /// Run performance benchmarks against Neo subsystems.
    Benchmark(BenchmarkCommand),

    /// Show system status information.
    Status(StatusCommand),

    /// Display runtime metrics.
    Metrics(MetricsCommand),

    /// List configured LLM providers and their health.
    Providers(ProvidersCommand),

    /// List available models from configured providers.
    Models(ModelsCommand),

    /// Enter the interactive shell (default when no subcommand is given).
    Interactive,
}

impl CliArgs {
    /// Parse CLI arguments from `std::env::args_os()`.
    pub fn parse_args() -> Self {
        Self::parse()
    }

    /// Execute the parsed command.
    ///
    /// Dispatches to the appropriate handler or enters interactive mode
    /// when no subcommand is provided.
    pub fn execute(&self) {
        if self.verbose {
            tracing_subscriber::fmt()
                .with_env_filter(
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("debug")),
                )
                .init();
        } else {
            tracing_subscriber::fmt()
                .with_env_filter(
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
                )
                .init();
        }

        tracing::debug!(client_url = %self.client_url, verbose = self.verbose, "CLI starting");

        match &self.command {
            Some(cmd) => CommandRegistry::dispatch(cmd, &self.client_url),
            None => self.run_interactive(),
        }
    }

    /// Enter the interactive shell.
    fn run_interactive(&self) {
        let session = InteractiveSession::new(self.client_url.clone());
        session.run();
    }
}
