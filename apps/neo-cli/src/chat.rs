use std::sync::Arc;

use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

use crate::bootstrap::NeoSystem;
use crate::error::{CliError, CliResult};

const PROMPT: &str = "Neo> ";

async fn process_message(input: &str, system: &Arc<NeoSystem>) -> CliResult<String> {
    let mut context_parts: Vec<String> = Vec::new();

    if let Some(ref mem) = system.memory {
        let search = neo_memory::SearchRequest {
            query: Some(input.to_string()),
            limit: Some(5),
            ..neo_memory::SearchRequest::default()
        };
        if let Ok(response) = mem.search(search) {
            for result in &response.results {
                context_parts.push(format!("memory: {}", result.content_preview));
            }
        }
    }

    let request = neo_reasoning::ReasoningRequest::new(input.to_string());

    match &system.reasoning {
        Some(reasoner) => match reasoner.start_session(request.clone()).await {
            Ok(session_id) => match reasoner.execute_session(session_id, request).await {
                Ok(response) => {
                    let mut output = response.conclusion;
                    if !context_parts.is_empty() {
                        output.push_str("\n\n[Related memory]");
                        for part in &context_parts {
                            output.push_str(&format!("\n  - {part}"));
                        }
                    }
                    Ok(output)
                }
                Err(e) => Ok(format!("Reasoning error: {e}\nBased on available context: {}", context_parts.join("; "))),
            },
            Err(e) => Ok(format!("Could not start reasoning session: {e}")),
        },
        None => {
            if context_parts.is_empty() {
                Ok("No reasoning or memory systems available. I can only echo your message.".to_string())
            } else {
                Ok(format!("Context found:\n{}", context_parts.join("\n")))
            }
        }
    }
}

fn print_welcome() {
    println!("Neo Chat v{}", crate::config::VERSION);
    println!("Type your message and press Enter. Commands: /clear, /history, /quit");
    println!();
}

pub async fn run(system: &Arc<NeoSystem>) -> CliResult<()> {
    let mut rl = DefaultEditor::new().map_err(|e| {
        CliError::custom(format!("failed to create readline: {e}"))
    })?;

    let hist_path = dirs::data_local_dir()
        .or_else(dirs::data_dir)
        .or_else(|| dirs::home_dir().map(|h| h.join(".local").join("share")))
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("neo_chat_history");

    if hist_path.exists() {
        let _ = rl.load_history(&hist_path);
    }

    print_welcome();

    let mut history: Vec<(String, String)> = Vec::new();
    let mut running = true;

    while running {
        match rl.readline(PROMPT) {
            Ok(line) => {
                let trimmed = line.trim().to_string();
                if trimmed.is_empty() {
                    continue;
                }

                match trimmed.as_str() {
                    "/quit" | "/exit" | "/q" => {
                        running = false;
                    }
                    "/clear" => {
                        history.clear();
                        print!("\x1B[2J\x1B[1;1H");
                    }
                    "/history" => {
                        if history.is_empty() {
                            println!("No conversation history.");
                        } else {
                            for (i, (user, assistant)) in history.iter().enumerate() {
                                println!("{}. User: {user}", i + 1);
                                println!("   Neo:  {assistant}");
                                println!();
                            }
                        }
                    }
                    _ => {
                        let _ = rl.add_history_entry(trimmed.as_str());
                        match process_message(&trimmed, system).await {
                            Ok(response) => {
                                println!("{response}");
                                history.push((trimmed, response));
                            }
                            Err(e) => {
                                println!("Error: {e}");
                            }
                        }
                        println!();
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
                return Err(CliError::custom(format!("readline error: {e}")));
            }
        }
    }

    let _ = rl.save_history(&hist_path);
    println!("Chat session ended.");
    Ok(())
}
