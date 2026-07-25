use std::sync::Arc;

use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

use crate::bootstrap::NeoSystem;
use crate::error::{CliError, CliResult};

const HISTORY_FILE_SUFFIX: &str = "neo_history";
const HELP_TEXT: &str = "\
Available commands:
  help                    Show this help message
  status                  Show system status and module health
  goals                   List all executive goals
  tasks                   List all executive tasks
  sessions                List executive sessions
  mode                    Show current execution mode
  mode <mode>             Set execution mode (safe|interactive|autonomous|developer)
  memory <query>          Search cognitive memory
  knowledge <query>       Search the knowledge graph
  reasoning <query>       Run a reasoning session
  history                 Show command history
  clear                   Clear the screen
  quit / exit             Exit the shell";

fn history_path() -> std::path::PathBuf {
    dirs::data_local_dir()
        .or_else(dirs::data_dir)
        .or_else(|| dirs::home_dir().map(|h| h.join(".local").join("share")))
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(HISTORY_FILE_SUFFIX)
}

fn format_uptime(start: std::time::Instant) -> String {
    let secs = start.elapsed().as_secs();
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}

fn parse_mode(s: &str) -> Option<neo_executive::ExecutionMode> {
    match s.to_lowercase().as_str() {
        "safe" => Some(neo_executive::ExecutionMode::Safe),
        "interactive" => Some(neo_executive::ExecutionMode::Interactive),
        "autonomous" => Some(neo_executive::ExecutionMode::Autonomous),
        "developer" => Some(neo_executive::ExecutionMode::Developer),
        _ => None,
    }
}

async fn handle_command(line: &str, system: &Arc<NeoSystem>) -> CliResult<bool> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(false);
    }

    let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
    let cmd = parts[0].to_lowercase();
    let arg = parts.get(1).map(|s| s.trim());

    match cmd.as_str() {
        "help" | "h" | "?" => {
            println!("{HELP_TEXT}");
        }

        "status" | "st" => {
            let stats = system.runtime.statistics();
            let uptime_str = format_uptime(system.start_time);
            println!("Runtime:");
            println!("  State:          {}", stats.state);
            println!("  Uptime:         {uptime_str}");
            println!("  Services:       {} registered, {} running",
                stats.services_registered, stats.services_running);
            println!("  Tasks scheduled: {}", stats.tasks_scheduled);
            println!("  Events published: {}", stats.events_published);
            println!("  Plugins loaded:  {}", stats.plugins_loaded);
            println!("  Health:          {}", stats.health_status);

            let modules = system.module_status();
            println!("\nModules:");
            for (name, active) in &modules {
                let indicator = if *active { "OK  " } else { "FAIL" };
                println!("  [{indicator}] {name}");
            }

            let exec_summary = system.executive.inspect_execution();
            println!("\nExecutive:");
            println!("  Goals:  {} total", exec_summary.goals_created);
            println!("  Tasks:  {} total", exec_summary.tasks_created);
            println!("  Uptime: {}ms", exec_summary.uptime_ms);
        }

        "goals" | "goal" => {
            let goals = system.executive.goal_manager().all_goals();
            if goals.is_empty() {
                println!("No goals found.");
            } else {
                println!("{:<8} {:<50} {:<14} {:<12}", "ID", "Description", "State", "Priority");
                println!("{}", "-".repeat(86));
                for goal in &goals {
                    let id_str = goal.id.as_str();
                    let short_id = if id_str.len() > 7 { &id_str[..7] } else { &id_str };
                    let desc = if goal.description.len() > 48 {
                        format!("{}..", &goal.description[..48])
                    } else {
                        goal.description.clone()
                    };
                    println!("{:<8} {:<50} {:<14} {:<12}",
                        short_id, desc, format!("{:?}", goal.state), format!("{:?}", goal.priority));
                }
                println!("\nTotal: {} goals", goals.len());
            }
        }

        "tasks" | "task" => {
            let tasks = system.executive.task_manager().all_tasks();
            if tasks.is_empty() {
                println!("No tasks found.");
            } else {
                println!("{:<8} {:<40} {:<14} {:<12}", "ID", "Name", "State", "Priority");
                println!("{}", "-".repeat(76));
                for task in &tasks {
                    let id_str = task.id.as_str();
                    let short_id = if id_str.len() > 7 { &id_str[..7] } else { &id_str };
                    let name = if task.name.len() > 38 {
                        format!("{}..", &task.name[..38])
                    } else {
                        task.name.clone()
                    };
                    println!("{:<8} {:<40} {:<14} {:<12}",
                        short_id, name, format!("{:?}", task.state), format!("{:?}", task.priority));
                }
                println!("\nTotal: {} tasks", tasks.len());
            }
        }

        "sessions" | "session" => {
            let sessions = system.executive.session_manager().list_sessions();
            if sessions.is_empty() {
                println!("No sessions found.");
            } else {
                println!("{:<8} {:<14} {:<28} {:<20}", "ID", "State", "Created", "Goals/Tasks");
                println!("{}", "-".repeat(72));
                for session in &sessions {
                    let id_str = session.id.to_string();
                    let short_id = if id_str.len() > 7 { &id_str[..7] } else { &id_str };
                    println!("{:<8} {:<14} {:<28} {:<20}",
                        short_id,
                        format!("{:?}", session.state),
                        session.created_at.format("%Y-%m-%d %H:%M:%S"),
                        format!("{}/{}", session.goal_ids.len(), session.task_ids.len()));
                }
                println!("\nTotal: {} sessions", sessions.len());
            }
        }

        "mode" => {
            match arg {
                Some(mode_str) => {
                    match parse_mode(mode_str) {
                        Some(mode) => {
                            system.executive.context().set_mode(mode);
                            println!("Execution mode set to: {mode:?}");
                        }
                        None => {
                            println!("Unknown mode: '{mode_str}'");
                            println!("Valid modes: safe, interactive, autonomous, developer");
                        }
                    }
                }
                None => {
                    let mode = system.executive.context().mode();
                    println!("Current execution mode: {mode:?}");
                    println!("Usage: mode <safe|interactive|autonomous|developer>");
                }
            }
        }

        "memory" | "mem" => {
            match arg {
                Some(query) => {
                    match &system.memory {
                        Some(mem) => {
                            let request = neo_memory::SearchRequest {
                                query: Some(query.to_string()),
                                limit: Some(10),
                                ..neo_memory::SearchRequest::default()
                            };
                            match mem.search(request) {
                                Ok(response) => {
                                    if response.results.is_empty() {
                                        println!("No memories found for '{query}'.");
                                    } else {
                                        println!("Found {} memories for '{}':\n", response.total, query);
                                        for (i, result) in response.results.iter().enumerate() {
                                            println!("  {}. [{}] {} (importance: {:.2})",
                                                i + 1,
                                                result.tier,
                                                result.content_preview,
                                                result.importance);
                                        }
                                    }
                                }
                                Err(e) => {
                                    println!("Memory search error: {e}");
                                }
                            }
                        }
                        None => {
                            println!("Memory system is not available.");
                        }
                    }
                }
                None => {
                    println!("Usage: memory <query>");
                }
            }
        }

        "knowledge" | "kg" => {
            match arg {
                Some(query) => {
                    match &system.knowledge {
                        Some(kg) => {
                            let results = kg.search(query, 10);
                            if results.is_empty() {
                                println!("No knowledge found for '{query}'.");
                            } else {
                                println!("Found {} results for '{}':\n", results.len(), query);
                                for (i, result) in results.iter().enumerate() {
                                    println!("  {}. {} (score: {:.3})\n     {}",
                                        i + 1,
                                        result.label,
                                        result.score,
                                        result.explanation);
                                }
                            }
                        }
                        None => {
                            println!("Knowledge graph is not available.");
                        }
                    }
                }
                None => {
                    println!("Usage: knowledge <query>");
                }
            }
        }

        "reasoning" | "reason" => {
            match arg {
                Some(query) => {
                    match &system.reasoning {
                        Some(reasoner) => {
                            let request = neo_reasoning::ReasoningRequest::new(query.to_string());
                            match reasoner.start_session(request).await {
                                Ok(session_id) => {
                                    println!("Reasoning session started: {session_id}");
                                    let request2 = neo_reasoning::ReasoningRequest::new(query.to_string());
                                    match reasoner.execute_session(session_id, request2).await {
                                        Ok(response) => {
                                            println!("\nConclusion: {}", response.conclusion);
                                            println!("Confidence: {:.3}", response.confidence);
                                            println!("Strategy:   {}", response.strategy_used);
                                            println!("Depth:      {}", response.reasoning_depth);
                                            println!("Latency:    {}ms", response.latency_ms);
                                            if let Some(ref explanation) = response.explanation {
                                                println!("\nExplanation:\n  {explanation}");
                                            }
                                        }
                                        Err(e) => {
                                            println!("Reasoning execution error: {e}");
                                        }
                                    }
                                }
                                Err(e) => {
                                    println!("Failed to start reasoning session: {e}");
                                }
                            }
                        }
                        None => {
                            println!("Reasoning engine is not available.");
                        }
                    }
                }
                None => {
                    println!("Usage: reasoning <query>");
                }
            }
        }

        "history" => {
            println!("Command history is managed by the readline editor.");
            println!("Use Ctrl+R to search history, or press Up/Down arrows.");
        }

        "clear" | "cls" => {
            print!("\x1B[2J\x1B[1;1H");
        }

        "quit" | "exit" | "q" => {
            return Ok(true);
        }

        _ => {
            println!("Unknown command: '{cmd}'. Type 'help' for available commands.");
        }
    }

    Ok(false)
}

/// Run the interactive REPL shell.
pub async fn run(system: &Arc<NeoSystem>) -> CliResult<()> {
    let hist_path = history_path();

    let rl_config = rustyline::config::Builder::new()
        .max_history_size(10_000)
        .map_err(|e| CliError::custom(format!("failed to configure history size: {e}")))?
        .history_ignore_dups(true)
        .map_err(|e| CliError::custom(format!("failed to configure history dedup: {e}")))?
        .tab_stop(4)
        .build();

    let mut rl: DefaultEditor = DefaultEditor::with_config(rl_config).map_err(|e| {
        CliError::custom(format!("failed to create readline editor: {e}"))
    })?;

    if hist_path.exists() {
        if let Err(e) = rl.load_history(&hist_path) {
            tracing::warn!("failed to load history from {}: {e}", hist_path.display());
        }
    }

    println!("Neo AGI Shell v{}", crate::config::VERSION);
    println!("Type 'help' for available commands.\n");

    let prompt = &system.config.shell.prompt;

    let mut running = true;
    while running {
        match rl.readline(prompt) {
            Ok(line) => {
                let _ = rl.add_history_entry(line.as_str());
                match handle_command(&line, system).await {
                    Ok(should_exit) => {
                        if should_exit {
                            running = false;
                        }
                    }
                    Err(e) => {
                        println!("Error: {e}");
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("^C");
            }
            Err(ReadlineError::Eof) => {
                println!("^D");
                running = false;
            }
            Err(e) => {
                println!("Readline error: {e}");
                running = false;
            }
        }
    }

    if let Err(e) = rl.save_history(&hist_path) {
        tracing::warn!("failed to save history to {}: {e}", hist_path.display());
    }

    println!("Goodbye.");
    Ok(())
}
