use std::sync::Arc;

use crate::bootstrap::NeoSystem;
use crate::error::CliResult;

fn format_duration(millis: u64) -> String {
    let secs = millis / 1000;
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}

fn print_header() {
    println!("\x1B[1;36m╔══════════════════════════════════════════════════════════╗");
    println!("║              Neo Developer Console                       ║");
    println!("╚══════════════════════════════════════════════════════════╝\x1B[0m");
    println!();
    println!("  Commands: help | refresh | quit");
    println!();
}

fn print_divider() {
    println!("\x1B[2m──────────────────────────────────────────────────────────────\x1B[0m");
}

fn print_runtime_stats(system: &NeoSystem) {
    let stats = system.runtime.statistics();
    let uptime_str = format_duration(stats.uptime_ms);

    println!("\x1B[1;33m  Runtime\x1B[0m");
    println!("    State:             {}", stats.state);
    println!("    Uptime:            {uptime_str}");
    println!("    Services:          {} registered / {} running",
        stats.services_registered, stats.services_running);
    println!("    Tasks scheduled:   {}", stats.tasks_scheduled);
    println!("    Events published:  {}", stats.events_published);
    println!("    Plugins loaded:    {}", stats.plugins_loaded);
    println!("    Health:            {}", stats.health_status);
}

fn print_memory_stats(system: &NeoSystem) {
    println!("\x1B[1;33m  Memory\x1B[0m");
    match &system.memory {
        Some(mem) => {
            let health = mem.health();
            println!("    Status:            {}", health.status);
            println!("    Total memories:    {}", health.total_memories);
            println!("    Total bytes:       {}", health.total_bytes);
            println!("    Cache hit rate:    {:.1}%", health.cache_hit_rate * 100.0);
            println!("    Uptime:            {}s", health.uptime_secs);

            if !health.per_tier.is_empty() {
                println!("    Per tier:");
                for (tier, count) in &health.per_tier {
                    println!("      {tier}: {count}");
                }
            }
        }
        None => {
            println!("    Status:            \x1B[31mnot available\x1B[0m");
        }
    }
}

fn print_knowledge_stats(system: &NeoSystem) {
    println!("\x1B[1;33m  Knowledge Graph\x1B[0m");
    match &system.knowledge {
        Some(kg) => {
            let metrics = kg.metrics();
            println!("    Entities:          {} (active: {})",
                metrics.entity_count, metrics.active_entity_count);
            println!("    Relations:         {} (active: {})",
                metrics.relation_count, metrics.active_relation_count);
            println!("    Namespaces:        {}", metrics.namespace_count);
            println!("    Avg confidence:    {:.3}", metrics.avg_entity_confidence);
            println!("    Avg importance:    {:.3}", metrics.avg_entity_importance);
            println!("    Total queries:     {}", metrics.total_queries);
            println!("    Avg query latency: {:.2}ms", metrics.avg_query_latency_ms);
            println!("    Extractions:       {}", metrics.total_extractions);
            println!("    Freshness:         {:.1}%", metrics.knowledge_freshness * 100.0);
            println!("    Consistency:       {:.1}%", metrics.consistency_score * 100.0);
        }
        None => {
            println!("    Status:            \x1B[31mnot available\x1B[0m");
        }
    }
}

fn print_executive_stats(system: &NeoSystem) {
    let summary = system.executive.inspect_execution();
    let goals = system.executive.goal_manager().all_goals();
    let tasks = system.executive.task_manager().all_tasks();

    println!("\x1B[1;33m  Executive\x1B[0m");
    println!("    Goals:             {} total ({} completed, {} failed, {} cancelled)",
        summary.goals_created,
        summary.goals_completed,
        summary.goals_failed,
        summary.goals_cancelled);
    println!("    Tasks:             {} total ({} completed, {} failed, {} cancelled)",
        summary.tasks_created,
        summary.tasks_completed,
        summary.tasks_failed,
        summary.tasks_cancelled);

    let active_goals: Vec<_> = goals.iter()
        .filter(|g| !g.state.is_terminal())
        .collect();
    if !active_goals.is_empty() {
        println!("    Active goals:");
        for goal in active_goals.iter().take(5) {
            let id_str = goal.id.as_str();
            let short_id = if id_str.len() > 7 { &id_str[..7] } else { &id_str };
            println!("      [{short_id}] {} ({:?}, {:.0}%)",
                goal.description, goal.priority, goal.progress * 100.0);
        }
        if active_goals.len() > 5 {
            println!("      ... and {} more", active_goals.len() - 5);
        }
    }

    let active_tasks: Vec<_> = tasks.iter()
        .filter(|t| !t.state.is_terminal())
        .collect();
    if !active_tasks.is_empty() {
        println!("    Active tasks:");
        for task in active_tasks.iter().take(5) {
            let id_str = task.id.as_str();
            let short_id = if id_str.len() > 7 { &id_str[..7] } else { &id_str };
            println!("      [{short_id}] {} ({:?})",
                task.name, task.state);
        }
        if active_tasks.len() > 5 {
            println!("      ... and {} more", active_tasks.len() - 5);
        }
    }

    let sessions = system.executive.session_manager().list_sessions();
    println!("    Sessions:          {} total", sessions.len());
}

fn print_stats(system: &NeoSystem) {
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    println!("\x1B[1m  [{now}]\x1B[0m");
    println!();
    print_runtime_stats(system);
    print_divider();
    print_memory_stats(system);
    print_divider();
    print_knowledge_stats(system);
    print_divider();
    print_executive_stats(system);
    print_divider();
    println!();
}

fn print_help() {
    println!("  Available commands:");
    println!("    help     Show this help message");
    println!("    refresh  Refresh and display stats");
    println!("    quit     Exit the developer console");
    println!();
}

/// Run the developer console.
pub async fn run(system: &Arc<NeoSystem>) -> CliResult<()> {
    print_header();

    let mut running = true;
    while running {
        print_stats(system);

        let mut input = String::new();
        match std::io::stdin().read_line(&mut input) {
            Ok(0) => {
                println!("EOF received. Exiting console.");
                break;
            }
            Ok(_) => {
                let trimmed = input.trim();
                match trimmed {
                    "help" | "h" | "?" => {
                        print_help();
                    }
                    "refresh" | "r" => {
                        // Stats will be printed at the top of the next loop iteration.
                    }
                    "quit" | "exit" | "q" => {
                        running = false;
                    }
                    "" => {
                        // Empty line: just refresh.
                    }
                    _ => {
                        println!("  Unknown command: '{trimmed}'. Type 'help' for commands.\n");
                    }
                }
            }
            Err(e) => {
                println!("  Failed to read input: {e}");
                running = false;
            }
        }
    }

    println!("Console closed.");
    Ok(())
}
