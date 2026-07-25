use std::sync::Arc;
use std::time::Instant;

use crate::bootstrap::NeoSystem;
use crate::error::CliResult;

pub async fn run(system: &Arc<NeoSystem>, duration_secs: u64) -> CliResult<()> {
    println!("Benchmarking for {duration_secs}s...");
    println!();

    let deadline = Instant::now() + std::time::Duration::from_secs(duration_secs);

    let mut goal_count: u64 = 0;
    while Instant::now() < deadline {
        let _ = system.executive.create_goal(
            format!("benchmark-goal-{goal_count}"),
            neo_executive::GoalPriority::Normal,
        );
        goal_count += 1;
    }
    let goal_elapsed = deadline.elapsed().as_secs_f64().max(0.001);
    let goal_ops = goal_count as f64 / goal_elapsed;

    let deadline2 = Instant::now() + std::time::Duration::from_secs(duration_secs);
    let mut task_count: u64 = 0;
    while Instant::now() < deadline2 {
        let _ = system.executive.submit_task(
            format!("benchmark-task-{task_count}"),
            neo_executive::TaskPriority::Normal,
            None,
        );
        task_count += 1;
    }
    let task_elapsed = deadline2.elapsed().as_secs_f64().max(0.001);
    let task_ops = task_count as f64 / task_elapsed;

    let mut inspect_count: u64 = 0;
    let deadline3 = Instant::now() + std::time::Duration::from_secs(duration_secs);
    while Instant::now() < deadline3 {
        let _ = system.executive.inspect_execution();
        inspect_count += 1;
    }
    let inspect_elapsed = deadline3.elapsed().as_secs_f64().max(0.001);
    let inspect_ops = inspect_count as f64 / inspect_elapsed;

    println!("Results ({duration_secs}s):");
    println!("  Goal creation:  {goal_count} ops ({goal_ops:.1} ops/s)");
    println!("  Task submission: {task_count} ops ({task_ops:.1} ops/s)");
    println!("  Inspection:     {inspect_count} ops ({inspect_ops:.1} ops/s)");
    println!();

    Ok(())
}
