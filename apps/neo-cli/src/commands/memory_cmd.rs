use std::sync::Arc;

use crate::bootstrap::NeoSystem;
use crate::cli::MemoryAction;
use crate::error::CliResult;

pub async fn run(system: &Arc<NeoSystem>, action: &MemoryAction) -> CliResult<()> {
    match action {
        MemoryAction::Stats => {
            match &system.memory {
                Some(mem) => {
                    let health = mem.health();
                    println!("Memory Statistics");
                    println!("=================");
                    println!("  Status:      {}", health.status);
                    println!("  Total:       {} entries", health.total_memories);
                    println!("  Size:        {} bytes", health.total_bytes);
                    println!("  Cache hit:   {:.1}%", health.cache_hit_rate * 100.0);
                    println!("  Uptime:      {}s", health.uptime_secs);
                    if !health.per_tier.is_empty() {
                        println!();
                        println!("  Per tier:");
                        for (tier, count) in &health.per_tier {
                            println!("    {tier}: {count}");
                        }
                    }
                }
                None => {
                    println!("Memory system is not available.");
                }
            }
        }
        MemoryAction::Store { content } => {
            match &system.memory {
                Some(mem) => {
                    let request = neo_memory::StoreRequest {
                        tier: neo_memory::types::MemoryTier::Working,
                        content: serde_json::json!({"text": content}),
                        tags: Vec::new(),
                        importance: None,
                        priority: None,
                        namespace: None,
                        ttl_secs: None,
                        source: Some("cli".to_string()),
                    };
                    match mem.store(request) {
                        Ok(response) => {
                            println!("Stored (id: {})", response.id);
                        }
                        Err(e) => {
                            println!("Store error: {e}");
                        }
                    }
                }
                None => {
                    println!("Memory system is not available.");
                }
            }
        }
        MemoryAction::Search { query } => {
            match &system.memory {
                Some(mem) => {
                    let request = neo_memory::SearchRequest {
                        query: Some(query.clone()),
                        limit: Some(20),
                        ..neo_memory::SearchRequest::default()
                    };
                    match mem.search(request) {
                        Ok(response) => {
                            if response.results.is_empty() {
                                println!("No memories found for '{query}'.");
                            } else {
                                println!("Found {} results for '{}':", response.total, query);
                                println!();
                                for (i, result) in response.results.iter().enumerate() {
                                    println!("  {}. [{}] {}", i + 1, result.tier, result.content_preview);
                                    println!("     importance: {:.2}  accessed: {}",
                                        result.importance, result.access_count);
                                }
                            }
                        }
                        Err(e) => {
                            println!("Search error: {e}");
                        }
                    }
                }
                None => {
                    println!("Memory system is not available.");
                }
            }
        }
        MemoryAction::List => {
            match &system.memory {
                Some(_mem) => {
                    println!("Memory entries (up to 50):");
                    println!("(use 'neo memory search <query>' to search)");
                }
                None => {
                    println!("Memory system is not available.");
                }
            }
        }
    }
    Ok(())
}
