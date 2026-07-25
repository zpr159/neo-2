//! Interactive REPL shell for the Neo CLI.
//!
//! Reads line-by-line input from stdin and dispatches recognised command
//! prefixes. History is kept in-memory for the lifetime of the session.

use std::io::{self, BufRead, Write};

use crate::cli::commands::InteractiveShell;

/// An interactive CLI session.
///
/// Created via [`InteractiveSession::new`] and started with [`InteractiveSession::run`].
pub struct InteractiveSession {
    /// Accumulated input history for this session.
    pub history: Vec<String>,
    /// Optional session identifier (set after the first successful exchange).
    pub session_id: Option<String>,
    /// Base URL of the Neo client API.
    pub client_url: String,
}

impl InteractiveSession {
    /// Create a new interactive session targeting the given client URL.
    pub fn new(client_url: String) -> Self {
        Self {
            history: Vec::new(),
            session_id: None,
            client_url,
        }
    }

    /// Run the interactive REPL until the user exits.
    pub fn run(&self) {
        InteractiveShell::print_welcome();

        let stdin = io::stdin();
        let mut reader = stdin.lock();

        loop {
            self.print_prompt();
            io::stdout().flush().unwrap_or(());

            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    // EOF
                    println!();
                    println!("Goodbye!");
                    break;
                }
                Ok(_) => {
                    let trimmed = line.trim().to_string();
                    if trimmed.is_empty() {
                        continue;
                    }
                    if self.process_input(&trimmed) {
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("[error] failed to read input: {}", e);
                    break;
                }
            }
        }
    }

    /// Print the REPL prompt.
    fn print_prompt(&self) {
        match &self.session_id {
            Some(sid) => print!("neo [{}] > ", &sid[..8.min(sid.len())]),
            None => print!("neo > "),
        }
    }

    /// Process a single line of user input.
    ///
    /// Returns `true` when the session should terminate.
    fn process_input(&self, input: &str) -> bool {
        let parts: Vec<&str> = input.splitn(2, ' ').collect();
        let command = parts[0].to_lowercase();
        let arg = parts.get(1).map(|s| s.to_string());

        match command.as_str() {
            "chat" => {
                match &arg {
                    Some(msg) => println!("[chat] ({}) → {}", self.client_url, msg),
                    None => println!("[chat] prompt mode — enter your message:"),
                }
                false
            }
            "stream" => {
                match &arg {
                    Some(msg) => println!("[stream] ({}) → {}", self.client_url, msg),
                    None => println!("[stream] prompt mode — enter your message:"),
                }
                false
            }
            "world" => {
                println!("[world] subcommands: list, get <id>, snapshot, simulate [N]");
                false
            }
            "memory" => {
                println!("[memory] subcommands: search <query>, store <content>, stats");
                false
            }
            "planning" | "plan" => {
                println!("[planning] subcommands: create <goal>, get <id>, list");
                false
            }
            "workflow" => {
                println!("[workflow] subcommands: start <name>, cancel <id>, status <id>");
                false
            }
            "agent" => {
                println!("[agent] subcommands: list, start <id>, stop <id>, status <id>");
                false
            }
            "tools" => {
                println!("[tools] subcommands: list, discover");
                false
            }
            "benchmark" => {
                println!("[benchmark] usage: benchmark [suite] [iterations]");
                false
            }
            "status" => {
                println!("[status] system status via {}", self.client_url);
                false
            }
            "metrics" => {
                println!("[metrics] usage: metrics [format] [--watch]");
                false
            }
            "providers" => {
                println!("[providers] listing providers via {}", self.client_url);
                false
            }
            "models" => {
                println!("[models] listing models via {}", self.client_url);
                false
            }
            "help" | "?" => {
                InteractiveShell::print_help();
                false
            }
            "exit" | "quit" | "q" => {
                println!("Goodbye!");
                true
            }
            _ => {
                eprintln!("Unknown command: '{}'. Type 'help' for available commands.", command);
                false
            }
        }
    }

    /// Print the welcome banner to stdout.
    pub fn print_welcome(&self) {
        InteractiveShell::print_welcome();
    }

    /// Print the list of available commands.
    pub fn print_help(&self) {
        InteractiveShell::print_help();
    }

    /// Save the current history to a file at the given path.
    ///
    /// This is a best-effort operation; errors are logged but not propagated.
    pub fn save_history(&self, path: &str) {
        use std::fs::File;

        match File::create(path) {
            Ok(mut file) => {
                for entry in &self.history {
                    let _ = writeln!(file, "{}", entry);
                }
                tracing::info!(count = self.history.len(), path, "saved history");
            }
            Err(e) => {
                tracing::warn!(path, error = %e, "failed to save history");
            }
        }
    }
}
