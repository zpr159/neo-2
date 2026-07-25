use std::sync::Arc;

use crate::bootstrap::NeoSystem;
use crate::error::CliResult;

pub async fn run(system: &Arc<NeoSystem>) -> CliResult<()> {
    let stats = system.runtime.statistics();
    let uptime = system.start_time.elapsed();
    let uptime_secs = uptime.as_secs();

    println!("System Status");
    println!("=============");
    println!();

    println!("Runtime:");
    println!("  State:             {:?}", stats.state);
    println!("  Uptime:            {}s ({}h {}m)",
        uptime_secs,
        uptime_secs / 3600,
        (uptime_secs % 3600) / 60);
    println!("  Services:          {} registered, {} running",
        stats.services_registered, stats.services_running);
    println!("  Tasks scheduled:   {}", stats.tasks_scheduled);
    println!("  Events published:  {}", stats.events_published);
    println!("  Plugins loaded:    {}", stats.plugins_loaded);
    println!("  Health:            {}", stats.health_status);

    println!();
    println!("Modules:");
    let modules = system.module_status();
    for (name, active) in &modules {
        let (icon, color) = if *active {
            ("\u{2713}", "\x1b[32m")
        } else {
            ("\u{2717}", "\x1b[31m")
        };
        let reset = "\x1b[0m";
        println!("  {color}{icon}{reset} {name}");
    }

    let exec_summary = system.executive.inspect_execution();
    println!();
    println!("Executive:");
    println!("  Goals:    {} total ({} completed, {} failed, {} cancelled)",
        exec_summary.goals_created,
        exec_summary.goals_completed,
        exec_summary.goals_failed,
        exec_summary.goals_cancelled);
    println!("  Tasks:    {} total ({} completed, {} failed, {} cancelled)",
        exec_summary.tasks_created,
        exec_summary.tasks_completed,
        exec_summary.tasks_failed,
        exec_summary.tasks_cancelled);
    println!("  Sessions: {}", system.executive.session_manager().session_count());
    println!("  Uptime:   {}ms", exec_summary.uptime_ms);

    if let Some(ref mem) = system.memory {
        let health = mem.health();
        println!();
        println!("Memory:");
        println!("  Status:    {}", health.status);
        println!("  Total:     {} entries", health.total_memories);
        println!("  Size:      {} bytes", health.total_bytes);
        println!("  Cache hit: {:.1}%", health.cache_hit_rate * 100.0);
    }

    if let Some(ref kg) = system.knowledge {
        let metrics = kg.metrics();
        println!();
        println!("Knowledge Graph:");
        println!("  Entities:  {} ({} active)", metrics.entity_count, metrics.active_entity_count);
        println!("  Relations: {} ({} active)", metrics.relation_count, metrics.active_relation_count);
        println!("  Queries:   {}", metrics.total_queries);
    }

    println!();
    Ok(())
}
