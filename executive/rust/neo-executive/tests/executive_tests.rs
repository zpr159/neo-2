use std::collections::HashMap;

use chrono::{Duration, Utc};
use neo_executive::*;

// ─────────────────────────── Goal Tests ───────────────────────────

#[test]
fn test_goal_full_lifecycle() {
    let mgr = GoalManager::new();
    let goal = mgr.create_goal("build feature".to_string(), GoalPriority::High);
    let id = goal.id;
    assert_eq!(goal.state, GoalState::Proposed);

    mgr.accept_goal(id).unwrap();
    assert_eq!(mgr.get_goal(id).unwrap().state, GoalState::Accepted);

    mgr.start_planning(id).unwrap();
    assert_eq!(mgr.get_goal(id).unwrap().state, GoalState::Planning);

    mgr.start_executing(id).unwrap();
    assert_eq!(mgr.get_goal(id).unwrap().state, GoalState::Executing);

    mgr.update_progress(id, 0.5).unwrap();
    assert!((mgr.get_goal(id).unwrap().progress - 0.5).abs() < f32::EPSILON);

    mgr.complete_goal(id).unwrap();
    let completed = mgr.get_goal(id).unwrap();
    assert!(completed.state.is_terminal());
    assert_eq!(completed.progress, 1.0);
    assert_eq!(mgr.goal_count(), 1);
}

#[test]
fn test_goal_fail_and_cancel() {
    let mgr = GoalManager::new();

    let g1 = mgr.create_goal("will fail".to_string(), GoalPriority::Normal);
    mgr.accept_goal(g1.id).unwrap();
    mgr.start_executing(g1.id).unwrap();
    mgr.fail_goal(g1.id, "resource unavailable".to_string()).unwrap();
    assert_eq!(mgr.get_goal(g1.id).unwrap().state, GoalState::Failed);
    assert_eq!(
        mgr.get_goal(g1.id)
            .unwrap()
            .metadata
            .get("failure_reason")
            .and_then(|v| v.as_str()),
        Some("resource unavailable")
    );

    let g2 = mgr.create_goal("will cancel".to_string(), GoalPriority::Low);
    mgr.cancel_goal(g2.id).unwrap();
    assert!(mgr.get_goal(g2.id).unwrap().state.is_terminal());
}

#[test]
fn test_goal_pause_resume() {
    let mgr = GoalManager::new();
    let goal = mgr.create_goal("pausable".to_string(), GoalPriority::Normal);
    let id = goal.id;

    mgr.accept_goal(id).unwrap();
    mgr.start_executing(id).unwrap();
    mgr.pause_goal(id).unwrap();
    assert_eq!(mgr.get_goal(id).unwrap().state, GoalState::Paused);

    mgr.resume_goal(id).unwrap();
    assert_eq!(mgr.get_goal(id).unwrap().state, GoalState::Executing);
}

#[test]
fn test_goal_invalid_transition() {
    let mgr = GoalManager::new();
    let goal = mgr.create_goal("nope".to_string(), GoalPriority::Normal);
    // Cannot go from Proposed directly to Executing
    let result = mgr.transition_goal(goal.id, GoalState::Executing);
    assert!(result.is_err());
}

#[test]
fn test_goal_dependency_chain() {
    let mgr = GoalManager::new();
    let g1 = mgr.create_goal("foundation".to_string(), GoalPriority::Critical);
    let g2 = mgr.create_goal("middle".to_string(), GoalPriority::High);
    let g3 = mgr.create_goal("top".to_string(), GoalPriority::Normal);

    mgr.add_dependency(g2.id, g1.id).unwrap();
    mgr.add_dependency(g3.id, g2.id).unwrap();

    // Accept all
    mgr.accept_goal(g1.id).unwrap();
    mgr.accept_goal(g2.id).unwrap();
    mgr.accept_goal(g3.id).unwrap();

    // Only g1 is ready (no unmet dependencies)
    let ready = mgr.ready_goals();
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].id, g1.id);

    // Complete g1
    mgr.start_executing(g1.id).unwrap();
    mgr.complete_goal(g1.id).unwrap();

    let ready = mgr.ready_goals();
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].id, g2.id);

    // Complete g2
    mgr.start_executing(g2.id).unwrap();
    mgr.complete_goal(g2.id).unwrap();

    let ready = mgr.ready_goals();
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].id, g3.id);
}

#[test]
fn test_goal_self_dependency_rejected() {
    let mgr = GoalManager::new();
    let g = mgr.create_goal("self".to_string(), GoalPriority::Normal);
    let result = mgr.add_dependency(g.id, g.id);
    assert!(result.is_err());
}

#[test]
fn test_goal_decomposition() {
    let mgr = GoalManager::new();
    let goal = mgr.create_goal("decomposable".to_string(), GoalPriority::Normal);
    let id = goal.id;

    mgr.decompose_goal(
        id,
        vec![
            "step 1: design".to_string(),
            "step 2: implement".to_string(),
            "step 3: test".to_string(),
        ],
    )
    .unwrap();

    let goal = mgr.get_goal(id).unwrap();
    assert_eq!(goal.decomposition.len(), 3);
    assert_eq!(goal.decomposition[0].description, "step 1: design");
    assert_eq!(goal.decomposition[0].order, 0);
    assert!(!goal.decomposition[0].completed);

    // Complete first two steps
    let step0_id = goal.decomposition[0].id;
    let step1_id = goal.decomposition[1].id;
    let mut goal = goal;
    goal.complete_decomposition_step(step0_id);
    goal.complete_decomposition_step(step1_id);
    mgr.update_goal(goal).unwrap();

    let goal = mgr.get_goal(id).unwrap();
    assert!((goal.progress - (2.0 / 3.0)).abs() < 0.01);
}

#[test]
fn test_goal_priority_ordering() {
    let mgr = GoalManager::new();
    mgr.create_goal("background".to_string(), GoalPriority::Background);
    mgr.create_goal("low".to_string(), GoalPriority::Low);
    mgr.create_goal("normal".to_string(), GoalPriority::Normal);
    mgr.create_goal("high".to_string(), GoalPriority::High);
    mgr.create_goal("critical".to_string(), GoalPriority::Critical);

    let sorted = mgr.goals_by_priority();
    assert_eq!(sorted.len(), 5);
    assert_eq!(sorted[0].priority, GoalPriority::Critical);
    assert_eq!(sorted[1].priority, GoalPriority::High);
    assert_eq!(sorted[2].priority, GoalPriority::Normal);
    assert_eq!(sorted[3].priority, GoalPriority::Low);
    assert_eq!(sorted[4].priority, GoalPriority::Background);
}

#[test]
fn test_goal_sub_goals() {
    let mgr = GoalManager::new();
    let parent = mgr.create_goal("parent goal".to_string(), GoalPriority::High);
    let child1 = mgr.create_goal("child 1".to_string(), GoalPriority::Normal);
    let child2 = mgr.create_goal("child 2".to_string(), GoalPriority::Low);

    let mut parent_goal = mgr.get_goal(parent.id).unwrap();
    parent_goal.add_sub_goal(child1.id);
    parent_goal.add_sub_goal(child2.id);
    mgr.update_goal(parent_goal).unwrap();

    let children = mgr.sub_goals(parent.id).unwrap();
    assert_eq!(children.len(), 2);

    // Also update children's parent_id
    let mut c1 = mgr.get_goal(child1.id).unwrap();
    c1.parent_id = Some(parent.id);
    mgr.update_goal(c1).unwrap();

    let mut c2 = mgr.get_goal(child2.id).unwrap();
    c2.parent_id = Some(parent.id);
    mgr.update_goal(c2).unwrap();
}

#[test]
fn test_goal_terminal_states_block_transitions() {
    let mgr = GoalManager::new();
    let g = mgr.create_goal("done".to_string(), GoalPriority::Normal);
    mgr.cancel_goal(g.id).unwrap();
    assert!(GoalState::Cancelled.valid_transitions().is_empty());

    let g2 = mgr.create_goal("done2".to_string(), GoalPriority::Normal);
    mgr.accept_goal(g2.id).unwrap();
    mgr.start_executing(g2.id).unwrap();
    mgr.complete_goal(g2.id).unwrap();
    assert!(GoalState::Completed.valid_transitions().is_empty());
}

#[test]
fn test_goal_remove_terminal() {
    let mgr = GoalManager::new();
    let g = mgr.create_goal("removable".to_string(), GoalPriority::Normal);
    mgr.cancel_goal(g.id).unwrap();
    mgr.remove_goal(g.id).unwrap();
    assert_eq!(mgr.goal_count(), 0);

    // Cannot remove non-terminal
    let g2 = mgr.create_goal("active".to_string(), GoalPriority::Normal);
    let result = mgr.remove_goal(g2.id);
    assert!(result.is_err());
}

#[test]
fn test_goal_not_found() {
    let mgr = GoalManager::new();
    assert!(mgr.get_goal(GoalId::new()).is_err());
}

// ─────────────────────────── Task Tests ───────────────────────────

#[test]
fn test_task_full_lifecycle() {
    let mgr = TaskManager::new();
    let task = mgr.create_task("implement".to_string());
    let id = task.id;
    assert_eq!(task.state, TaskState::Pending);
    assert_eq!(mgr.task_count(), 1);

    mgr.submit_task(task).unwrap();
    assert_eq!(mgr.get_task(id).unwrap().state, TaskState::Queued);
    assert_eq!(mgr.queue_depth(), 1);

    mgr.start_task(id, "worker-1".to_string()).unwrap();
    assert_eq!(mgr.get_task(id).unwrap().state, TaskState::Running);
    assert_eq!(mgr.get_task(id).unwrap().owner.as_deref(), Some("worker-1"));

    mgr.complete_task(id, serde_json::json!({"output": "42"}))
        .unwrap();
    let task = mgr.get_task(id).unwrap();
    assert!(task.state.is_terminal());
    assert_eq!(
        task.result,
        Some(serde_json::json!({"output": "42"}))
    );
}

#[test]
fn test_task_fail_and_cancel() {
    let mgr = TaskManager::new();
    let t1 = mgr.create_task("flaky".to_string())
        .with_retry_policy(RetryPolicy { max_retries: 0, ..RetryPolicy::default() });
    mgr.submit_task(t1.clone()).unwrap();
    mgr.start_task(t1.id, "w".to_string()).unwrap();
    mgr.fail_task(t1.id, "crashed".to_string()).unwrap();
    assert_eq!(mgr.get_task(t1.id).unwrap().state, TaskState::Failed);

    let t2 = mgr.create_task("cancel me".to_string());
    mgr.submit_task(t2.clone()).unwrap();
    mgr.cancel_task(t2.id).unwrap();
    assert!(mgr.get_task(t2.id).unwrap().state.is_terminal());
    assert_eq!(mgr.queue_depth(), 0);
}

#[test]
fn test_task_retry_with_policy() {
    let mgr = TaskManager::new();
    let task = mgr
        .create_task("retryable".to_string())
        .with_retry_policy(RetryPolicy {
            max_retries: 2,
            base_delay_ms: 10,
            max_delay_ms: 100,
            backoff_multiplier: 2.0,
        });
    let id = task.id;
    mgr.submit_task(task).unwrap();
    mgr.start_task(id, "w".to_string()).unwrap();

    // First failure: should retry
    let should_retry = mgr.fail_task(id, "err1".to_string()).unwrap();
    assert!(should_retry);
    assert_eq!(mgr.get_task(id).unwrap().retry_count, 1);

    // Re-queue from retrying
    let mut task = mgr.get_task(id).unwrap();
    task.transition(TaskState::Queued).unwrap();
    mgr.update_task(task).unwrap();
    mgr.start_task(id, "w".to_string()).unwrap();

    // Second failure: should retry again
    let should_retry = mgr.fail_task(id, "err2".to_string()).unwrap();
    assert!(should_retry);
    assert_eq!(mgr.get_task(id).unwrap().retry_count, 2);

    // Re-queue and fail a third time: no more retries
    let mut task = mgr.get_task(id).unwrap();
    task.transition(TaskState::Queued).unwrap();
    mgr.update_task(task).unwrap();
    mgr.start_task(id, "w".to_string()).unwrap();

    let should_retry = mgr.fail_task(id, "err3".to_string()).unwrap();
    assert!(!should_retry);
}

#[test]
fn test_task_retry_policy_delay_calculation() {
    let policy = RetryPolicy {
        max_retries: 5,
        base_delay_ms: 100,
        max_delay_ms: 10_000,
        backoff_multiplier: 2.0,
    };
    assert_eq!(policy.delay_for_attempt(0), 100);
    assert_eq!(policy.delay_for_attempt(1), 200);
    assert_eq!(policy.delay_for_attempt(2), 400);
    assert_eq!(policy.delay_for_attempt(3), 800);
    assert_eq!(policy.delay_for_attempt(4), 1600);

    // Capped at max_delay_ms
    let policy2 = RetryPolicy {
        max_retries: 10,
        base_delay_ms: 1000,
        max_delay_ms: 5000,
        backoff_multiplier: 10.0,
    };
    assert_eq!(policy2.delay_for_attempt(5), 5000);
}

#[test]
fn test_task_ownership_conflict() {
    let mgr = TaskManager::new();
    let task = mgr.create_task("exclusive".to_string());
    let id = task.id;
    mgr.submit_task(task).unwrap();

    mgr.start_task(id, "worker-A".to_string()).unwrap();
    let result = mgr.start_task(id, "worker-B".to_string());
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().code(),
        ExecutiveErrorCode::TaskOwnershipConflict
    );
}

#[test]
fn test_task_dependency_resolution() {
    let mgr = TaskManager::new();
    let t1 = mgr.create_task("prerequisite".to_string());
    let t2 = mgr.create_task("dependent".to_string());

    mgr.add_dependency(t2.id, t1.id).unwrap();
    mgr.submit_task(t1.clone()).unwrap();
    mgr.submit_task(t2.clone()).unwrap();

    // t1 has no deps -> ready, t2 depends on t1 (not completed) -> not ready
    let ready = mgr.ready_tasks();
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].id, t1.id);

    // Complete t1
    mgr.start_task(t1.id, "w".to_string()).unwrap();
    mgr.complete_task(t1.id, serde_json::json!(null)).unwrap();

    // Now t2 should be ready
    let ready = mgr.ready_tasks();
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].id, t2.id);
}

#[test]
fn test_task_priority_ordering() {
    let mgr = TaskManager::new();
    let t1 = mgr
        .create_task("bg".to_string())
        .with_priority(TaskPriority::Background);
    let t2 = mgr
        .create_task("low".to_string())
        .with_priority(TaskPriority::Low);
    let t3 = mgr
        .create_task("crit".to_string())
        .with_priority(TaskPriority::Critical);

    mgr.submit_task(t1).unwrap();
    mgr.submit_task(t2).unwrap();
    mgr.submit_task(t3).unwrap();

    let sorted = mgr.tasks_by_priority();
    assert_eq!(sorted[0].priority, TaskPriority::Critical);
    assert_eq!(sorted[1].priority, TaskPriority::Low);
    assert_eq!(sorted[2].priority, TaskPriority::Background);
}

#[test]
fn test_task_pause_resume() {
    let mgr = TaskManager::new();
    let task = mgr.create_task("pausable".to_string());
    let id = task.id;
    mgr.submit_task(task).unwrap();
    mgr.start_task(id, "w".to_string()).unwrap();

    mgr.pause_task(id).unwrap();
    assert_eq!(mgr.get_task(id).unwrap().state, TaskState::Paused);

    mgr.resume_task(id).unwrap();
    assert_eq!(mgr.get_task(id).unwrap().state, TaskState::Running);
}

#[test]
fn test_task_not_found() {
    let mgr = TaskManager::new();
    assert!(mgr.get_task(TaskId::new()).is_err());
}

// ─────────────────────────── Priority Engine Tests ───────────────────────────

#[test]
fn test_priority_urgency_calculation() {
    let engine = PriorityEngine::new();

    // Urgent: deadline in 30 minutes, critical priority
    let urgency = engine.calculate_urgency(
        Some(Utc::now() + Duration::minutes(30)),
        GoalPriority::Critical,
    );
    assert!(urgency > 0.8);

    // Relaxed: deadline in 7 days, low priority
    let relaxed = engine.calculate_urgency(
        Some(Utc::now() + Duration::days(7)),
        GoalPriority::Low,
    );
    assert!(relaxed < 0.5);

    // No deadline
    let no_deadline = engine.calculate_urgency(None, GoalPriority::Normal);
    assert!(no_deadline > 0.0 && no_deadline <= 1.0);
}

#[test]
fn test_priority_importance_calculation() {
    let engine = PriorityEngine::new();

    let high_importance = engine.calculate_importance(true, 3, 5);
    let low_importance = engine.calculate_importance(false, 0, 0);
    assert!(high_importance > low_importance);
}

#[test]
fn test_priority_resource_factor() {
    let engine = PriorityEngine::new();

    let low_util = vec![ResourceAvailability {
        resource_type: ResourceType::Cpu,
        available: 8,
        total: 16,
        utilization: 0.5,
    }];
    let high_util = vec![ResourceAvailability {
        resource_type: ResourceType::Cpu,
        available: 1,
        total: 16,
        utilization: 0.9375,
    }];

    let f_low = engine.calculate_resource_factor(&low_util);
    let f_high = engine.calculate_resource_factor(&high_util);
    assert!(f_low > f_high);

    // Empty resources
    let f_empty = engine.calculate_resource_factor(&[]);
    assert!((f_empty - 0.5).abs() < f64::EPSILON);
}

#[test]
fn test_priority_age_factor() {
    let engine = PriorityEngine::new();
    let young = engine.calculate_age_factor(Utc::now());
    let old = engine.calculate_age_factor(Utc::now() - Duration::days(7));
    assert!(old > young);
    assert!(young >= 0.0);
}

#[test]
fn test_priority_full_goal_score() {
    let engine = PriorityEngine::new();
    let score = engine.score_goal(
        Some(Utc::now() + Duration::hours(1)),
        GoalPriority::High,
        true,
        2,
        3,
        &[ResourceAvailability {
            resource_type: ResourceType::Cpu,
            available: 4,
            total: 8,
            utilization: 0.5,
        }],
        Utc::now() - Duration::hours(12),
    );
    assert!(score.total > 0.0 && score.total <= 1.0);
    assert!(score.urgency > 0.0);
    assert!(score.importance > 0.0);
}

#[test]
fn test_priority_conflict_resolution() {
    let engine = PriorityEngine::new();

    // PriorityFirst: higher score wins
    assert!(engine.resolve_conflict(0.9, 0.5, None, None));
    assert!(!engine.resolve_conflict(0.5, 0.9, None, None));

    // DeadlineFirst: earlier deadline wins
    engine.set_resolution_strategy(ConflictResolution::DeadlineFirst);
    let earlier = Some(Utc::now() + Duration::hours(1));
    let later = Some(Utc::now() + Duration::hours(24));
    assert!(engine.resolve_conflict(0.5, 0.9, earlier, later));
    assert!(!engine.resolve_conflict(0.9, 0.5, later, earlier));

    // One has deadline, other doesn't
    assert!(engine.resolve_conflict(0.5, 0.5, earlier, None));
    assert!(!engine.resolve_conflict(0.5, 0.5, None, earlier));
}

#[test]
fn test_priority_score_storage() {
    let engine = PriorityEngine::new();
    let gid = GoalId::new();
    let tid = TaskId::new();

    let score = PriorityScore::new(0.8, 0.7, 0.6, 0.5);
    engine.set_goal_score(gid, score);
    assert_eq!(engine.goal_score_count(), 1);

    let retrieved = engine.get_goal_score(gid).unwrap();
    assert!((retrieved.total - score.total).abs() < f64::EPSILON);

    let tscore = PriorityScore::new(0.3, 0.4, 0.5, 0.6);
    engine.set_task_score(tid, tscore);
    assert_eq!(engine.task_score_count(), 1);
    assert!(engine.get_task_score(tid).is_some());

    engine.clear_scores();
    assert_eq!(engine.goal_score_count(), 0);
    assert_eq!(engine.task_score_count(), 0);
}

#[test]
fn test_priority_rules() {
    let engine = PriorityEngine::new();
    engine.add_rule(PriorityRule {
        name: "boost-critical".to_string(),
        condition: "priority == critical".to_string(),
        adjustment: 0.2,
        active: true,
    });
    engine.add_rule(PriorityRule {
        name: "inactive-rule".to_string(),
        condition: "always".to_string(),
        adjustment: 0.1,
        active: false,
    });

    assert_eq!(engine.active_rules().len(), 1);
    assert!(engine.remove_rule("boost-critical"));
    assert_eq!(engine.active_rules().len(), 0);
}

#[test]
fn test_priority_task_urgency() {
    let engine = PriorityEngine::new();
    let score = engine.score_task(
        Some(Utc::now() + Duration::minutes(30)),
        TaskPriority::Critical,
        &[],
        Utc::now(),
    );
    assert!(score.total > 0.0);
    assert!(score.urgency > 0.7);
}

// ─────────────────────────── Attention Manager Tests ───────────────────────────

#[test]
fn test_attention_budget_lifecycle() {
    let mut budget = AttentionBudget::new(10.0);
    assert!(budget.can_allocate(5.0));
    assert!(!budget.can_allocate(11.0));
    assert!((budget.remaining() - 10.0).abs() < f64::EPSILON);

    assert!(budget.reserve(3.0));
    assert!((budget.remaining() - 7.0).abs() < f64::EPSILON);

    budget.commit(3.0);
    assert!((budget.consumed - 3.0).abs() < f64::EPSILON);

    budget.release(2.0);
    assert!((budget.consumed - 1.0).abs() < f64::EPSILON);
    assert!((budget.utilization() - 0.1).abs() < f64::EPSILON);
}

#[test]
fn test_attention_focus_and_clear() {
    let mgr = AttentionManager::new(20.0);
    let gid = GoalId::new();

    assert!(mgr.focus_on_goal(gid, "design phase".to_string(), 3.0));
    let focus = mgr.current_focus().unwrap();
    assert_eq!(focus.goal_id, Some(gid));
    assert_eq!(focus.description, "design phase");

    mgr.clear_focus();
    assert!(mgr.current_focus().is_none());
    assert_eq!(mgr.focus_history().len(), 1);
}

#[test]
fn test_attention_context_switching() {
    let mgr = AttentionManager::new(30.0);
    let g1 = GoalId::new();
    let g2 = GoalId::new();

    mgr.focus_on_goal(g1, "task A".to_string(), 2.0);
    mgr.focus_on_goal(g2, "task B".to_string(), 2.0);

    assert_eq!(mgr.context_switch_count(), 1);
    let switches = mgr.context_switch_history();
    assert_eq!(switches[0].from, Some("task A".to_string()));
    assert_eq!(switches[0].to, "task B");
}

#[test]
fn test_attention_interrupts() {
    let mgr = AttentionManager::new(10.0);

    let i1 = Interrupt::new(
        InterruptType::Critical,
        "sensor".to_string(),
        "temperature spike".to_string(),
    );
    let i2 = Interrupt::new(
        InterruptType::Normal,
        "monitor".to_string(),
        "heartbeat".to_string(),
    );

    mgr.queue_interrupt(i1);
    mgr.queue_interrupt(i2);
    assert!(mgr.has_pending_interrupts());
    assert_eq!(mgr.pending_interrupt_count(), 2);

    let next = mgr.peek_next_interrupt().unwrap();
    assert_eq!(next.interrupt_type, InterruptType::Critical);

    let processed = mgr.process_next_interrupt().unwrap();
    assert_eq!(processed.interrupt_type, InterruptType::Critical);
    assert_eq!(mgr.pending_interrupt_count(), 1);
    assert_eq!(mgr.processed_interrupts().len(), 1);
}

#[test]
fn test_attention_budget_enforcement() {
    let mgr = AttentionManager::new(5.0);
    let g1 = GoalId::new();
    let g2 = GoalId::new();

    assert!(mgr.focus_on_goal(g1, "expensive".to_string(), 4.0));
    // Budget is 5, consumed 4, remaining 1. Focus cost 4 won't fit.
    assert!(!mgr.focus_on_goal(g2, "over budget".to_string(), 4.0));
}

#[test]
fn test_attention_focus_stats() {
    let mgr = AttentionManager::new(20.0);
    let g1 = GoalId::new();
    let g2 = GoalId::new();

    mgr.focus_on_goal(g1, "coding".to_string(), 1.0);
    mgr.clear_focus();
    mgr.focus_on_goal(g2, "coding".to_string(), 1.0);
    mgr.clear_focus();

    let stats = mgr.focus_stats();
    assert_eq!(stats.get("coding"), Some(&2));
}

#[test]
fn test_attention_task_focus() {
    let mgr = AttentionManager::new(10.0);
    let tid = TaskId::new();
    assert!(mgr.focus_on_task(tid, "run tests".to_string(), 2.0));
    let focus = mgr.current_focus().unwrap();
    assert_eq!(focus.task_id, Some(tid));
}

// ─────────────────────────── Scheduler Tests ───────────────────────────

#[test]
fn test_scheduler_schedule_dequeue_complete() {
    let sched = ExecutiveScheduler::default();
    let engine = PriorityEngine::new();
    let task = Task::new("compute".to_string());

    let exec_id = sched.schedule_task(&task, &engine).unwrap();
    assert_eq!(sched.queue_depth(), 1);

    let exec = sched.dequeue_next().unwrap();
    assert_eq!(exec.task_id, task.id);
    assert_eq!(sched.active_count(), 1);
    assert_eq!(sched.queue_depth(), 0);

    sched.complete_execution(exec_id).unwrap();
    assert_eq!(sched.active_count(), 0);

    let stats = sched.statistics();
    assert_eq!(stats.total_scheduled, 1);
    assert_eq!(stats.total_completed, 1);
}

#[test]
fn test_scheduler_priority_ordering() {
    let sched = ExecutiveScheduler::default();
    let engine = PriorityEngine::new();

    let low = Task::new("low".to_string()).with_priority(TaskPriority::Low);
    let high = Task::new("high".to_string()).with_priority(TaskPriority::High);
    let crit = Task::new("crit".to_string()).with_priority(TaskPriority::Critical);

    sched.schedule_task(&low, &engine).unwrap();
    sched.schedule_task(&high, &engine).unwrap();
    sched.schedule_task(&crit, &engine).unwrap();

    // BinaryHeap is max-heap by Ord, so highest priority comes first
    let first = sched.dequeue_next().unwrap();
    assert_eq!(first.task_id, crit.id);
    let second = sched.dequeue_next().unwrap();
    assert_eq!(second.task_id, high.id);
}

#[test]
fn test_scheduler_preemption() {
    let sched = ExecutiveScheduler::new(SchedulingPolicy {
        enable_preemption: true,
        ..SchedulingPolicy::default()
    });
    let engine = PriorityEngine::new();
    let task = Task::new("preemptible".to_string());

    let exec_id = sched.schedule_task(&task, &engine).unwrap();
    sched.dequeue_next().unwrap();

    let preemptor = TaskId::new();
    let preempted = sched
        .preempt_execution(exec_id, "higher priority arrived".to_string(), preemptor)
        .unwrap();
    assert_eq!(preempted.task_id, task.id);
    assert_eq!(sched.active_count(), 0);
    assert_eq!(sched.queue_depth(), 1);

    let log = sched.preemption_log();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].preempted_task, task.id);
    assert_eq!(log[0].preempted_by, preemptor);

    let stats = sched.statistics();
    assert_eq!(stats.total_preempted, 1);
}

#[test]
fn test_scheduler_preemption_disabled() {
    let sched = ExecutiveScheduler::new(SchedulingPolicy {
        enable_preemption: false,
        ..SchedulingPolicy::default()
    });
    let engine = PriorityEngine::new();
    let task = Task::new("protected".to_string());

    let exec_id = sched.schedule_task(&task, &engine).unwrap();
    sched.dequeue_next().unwrap();

    let result = sched.preempt_execution(
        exec_id,
        "try".to_string(),
        TaskId::new(),
    );
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().code(),
        ExecutiveErrorCode::PreemptionDenied
    );
}

#[test]
fn test_scheduler_stats() {
    let sched = ExecutiveScheduler::default();
    let engine = PriorityEngine::new();
    let t1 = Task::new("t1".to_string());
    let t2 = Task::new("t2".to_string());

    let id1 = sched.schedule_task(&t1, &engine).unwrap();
    sched.schedule_task(&t2, &engine).unwrap();

    sched.dequeue_next().unwrap();
    sched.complete_execution(id1).unwrap();

    let stats = sched.statistics();
    assert_eq!(stats.total_scheduled, 2);
    assert_eq!(stats.total_completed, 1);
    assert_eq!(stats.active_executions, 0);
    assert_eq!(stats.current_queue_depth, 1);
}

#[test]
fn test_scheduler_dependency_tracking() {
    let sched = ExecutiveScheduler::default();
    let tm = TaskManager::new();

    let t1 = tm.create_task("dep1".to_string());
    let t2 = tm.create_task("dep2".to_string());

    tm.submit_task(t1.clone()).unwrap();
    tm.submit_task(t2.clone()).unwrap();
    sched.add_dependency(t2.id, t1.id);

    let executable = sched.executable_tasks(&tm);
    assert_eq!(executable.len(), 1);
    assert_eq!(executable[0].id, t1.id);
}

#[test]
fn test_scheduler_resource_awareness() {
    let sched = ExecutiveScheduler::new(SchedulingPolicy {
        enable_resource_awareness: true,
        ..SchedulingPolicy::default()
    });
    let rc = ResourceCoordinator::new();

    let mut reqs = HashMap::new();
    reqs.insert(ResourceType::Cpu, 4);
    reqs.insert(ResourceType::Ram, 1024);
    assert!(sched.can_schedule_with_resources(&reqs, &rc));

    let mut reqs2 = HashMap::new();
    reqs2.insert(ResourceType::Cpu, 100);
    assert!(!sched.can_schedule_with_resources(&reqs2, &rc));
}

#[test]
fn test_scheduler_policy_update() {
    let sched = ExecutiveScheduler::default();
    let original = sched.policy();
    assert!(original.enable_preemption);

    sched.set_policy(SchedulingPolicy {
        enable_preemption: false,
        ..SchedulingPolicy::default()
    });
    assert!(!sched.policy().enable_preemption);
}

// ─────────────────────────── Resource Coordination Tests ───────────────────────────

#[test]
fn test_resource_allocate_and_release() {
    let rc = ResourceCoordinator::new();
    let alloc = rc
        .allocate(ResourceType::Cpu, 2, "worker-1".to_string())
        .unwrap();
    assert_eq!(alloc.amount, 2);
    assert_eq!(rc.available(ResourceType::Cpu), 6);
    assert_eq!(rc.allocation_count(), 1);

    rc.release(&alloc).unwrap();
    assert_eq!(rc.available(ResourceType::Cpu), 8);
    assert_eq!(rc.allocation_count(), 0);
}

#[test]
fn test_resource_exhaustion() {
    let rc = ResourceCoordinator::new();
    let result = rc.allocate(ResourceType::Cpu, 100, "greedy".to_string());
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().code(),
        ExecutiveErrorCode::ResourceExhausted
    );
}

#[test]
fn test_resource_utilization() {
    let rc = ResourceCoordinator::new();
    let _alloc = rc
        .allocate(ResourceType::Cpu, 4, "w".to_string())
        .unwrap();
    let util = rc.utilization(ResourceType::Cpu);
    assert!((util - 0.5).abs() < f64::EPSILON);

    let _alloc2 = rc
        .allocate(ResourceType::Ram, 8192, "w".to_string())
        .unwrap();
    let ram_util = rc.utilization(ResourceType::Ram);
    assert!((ram_util - 0.25).abs() < f64::EPSILON);
}

#[test]
fn test_resource_can_satisfy() {
    let rc = ResourceCoordinator::new();
    let mut reqs = HashMap::new();
    reqs.insert(ResourceType::Cpu, 4);
    reqs.insert(ResourceType::Ram, 2048);
    assert!(rc.can_satisfy(&reqs));

    let mut reqs2 = HashMap::new();
    reqs2.insert(ResourceType::Cpu, 100);
    assert!(!rc.can_satisfy(&reqs2));
}

#[test]
fn test_model_allocation() {
    let rc = ResourceCoordinator::new();
    let alloc = rc
        .allocate_model("llama-7b".to_string(), 1, 4096, "inference".to_string())
        .unwrap();
    assert_eq!(alloc.model_id, "llama-7b");
    assert_eq!(alloc.gpu_count, 1);
    assert_eq!(alloc.ram_mb, 4096);
    assert_eq!(rc.model_allocations().len(), 1);

    // GPU and RAM should be consumed
    assert_eq!(rc.available(ResourceType::Gpu), 3);
    assert_eq!(rc.available(ResourceType::Ram), 32768 - 4096);
    assert_eq!(rc.available(ResourceType::ModelSlot), 3);

    rc.release_model(&alloc).unwrap();
    assert_eq!(rc.model_allocations().len(), 0);
}

#[test]
fn test_model_allocation_insufficient_resources() {
    let rc = ResourceCoordinator::new();
    let result = rc.allocate_model(
        "huge-model".to_string(),
        100,
        65536,
        "greedy".to_string(),
    );
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().code(),
        ExecutiveErrorCode::ModelAllocationFailed
    );
}

#[test]
fn test_inference_budget() {
    let rc = ResourceCoordinator::new();
    rc.consume_inference_budget(500).unwrap();
    let budget = rc.inference_budget();
    assert_eq!(budget.consumed_tokens, 500);
    assert_eq!(budget.remaining(), 999500);
    assert!(budget.can_consume(100));
}

#[test]
fn test_inference_budget_exceeded() {
    let rc = ResourceCoordinator::new();
    let result = rc.consume_inference_budget(2_000_000);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().code(),
        ExecutiveErrorCode::InferenceBudgetExceeded
    );
}

#[test]
fn test_resource_pool_statuses() {
    let rc = ResourceCoordinator::new();
    let statuses = rc.pool_statuses();
    assert_eq!(statuses.len(), 7);
    assert!(statuses.iter().all(|s| s.utilization == 0.0));
}

// ─────────────────────────── Execution Policy Tests ───────────────────────────

#[test]
fn test_safe_mode_policy() {
    let policy = ExecutionPolicy::safe_mode();
    assert!(policy.has_permission(&Permission::AccessMemory));
    assert!(policy.has_permission(&Permission::InvokeReasoning));
    assert!(!policy.has_permission(&Permission::ExecuteCode));
    assert!(!policy.has_permission(&Permission::UseTools));
    assert!(!policy.has_permission(&Permission::BypassSafetyChecks));
    assert!(!policy.allow_autonomous_actions);
    assert!(policy.audit_all_decisions);
    assert_eq!(policy.max_concurrent_goals, 1);
    assert_eq!(policy.max_concurrent_tasks, 2);
}

#[test]
fn test_interactive_mode_policy() {
    let policy = ExecutionPolicy::interactive_mode();
    assert!(policy.has_permission(&Permission::ExecuteCode));
    assert!(policy.has_permission(&Permission::UseTools));
    assert!(!policy.has_permission(&Permission::OverridePriority));
    assert!(!policy.has_permission(&Permission::BypassSafetyChecks));
    assert!(!policy.allow_autonomous_actions);
    assert_eq!(policy.max_concurrent_goals, 4);
    assert_eq!(policy.max_concurrent_tasks, 16);
}

#[test]
fn test_autonomous_mode_policy() {
    let policy = ExecutionPolicy::autonomous_mode();
    assert!(policy.has_permission(&Permission::OverridePriority));
    assert!(!policy.has_permission(&Permission::BypassSafetyChecks));
    assert!(policy.allow_autonomous_actions);
    assert_eq!(policy.max_concurrent_goals, 16);
}

#[test]
fn test_developer_mode_policy() {
    let policy = ExecutionPolicy::developer_mode();
    assert!(policy.has_permission(&Permission::BypassSafetyChecks));
    assert!(policy.has_permission(&Permission::AccessHardware));
    assert!(policy.allow_autonomous_actions);
    assert_eq!(policy.max_concurrent_goals, 32);
    assert_eq!(policy.max_concurrent_tasks, 128);
}

#[test]
fn test_policy_engine_mode_switch() {
    let engine = PolicyEngine::new(ExecutionMode::Safe);
    assert_eq!(engine.current_mode(), ExecutionMode::Safe);
    assert!(!engine.current_policy().allow_autonomous_actions);

    engine.switch_mode(ExecutionMode::Autonomous);
    assert_eq!(engine.current_mode(), ExecutionMode::Autonomous);
    assert!(engine.current_policy().allow_autonomous_actions);
    assert_eq!(engine.policy_history().len(), 1);

    engine.switch_mode(ExecutionMode::Developer);
    assert!(engine.current_policy().has_permission(&Permission::BypassSafetyChecks));
}

#[test]
fn test_policy_permission_enforcement() {
    let engine = PolicyEngine::new(ExecutionMode::Safe);
    assert!(engine.enforce_permission(&Permission::AccessMemory).is_ok());
    assert!(engine.enforce_permission(&Permission::InvokeReasoning).is_ok());
    assert!(engine.enforce_permission(&Permission::ExecuteCode).is_err());
    assert_eq!(engine.violation_count(), 1);
    assert_eq!(engine.violations()[0].permission, Permission::ExecuteCode);
    assert!(engine.violations()[0].blocked);
}

#[test]
fn test_policy_confirmation_required() {
    let safe = PolicyEngine::new(ExecutionMode::Safe);
    assert!(!safe.requires_confirmation(0.05));
    assert!(safe.requires_confirmation(0.15));

    let interactive = PolicyEngine::new(ExecutionMode::Interactive);
    assert!(!interactive.requires_confirmation(0.3));
    assert!(interactive.requires_confirmation(0.6));
}

#[test]
fn test_policy_for_mode() {
    let modes = [
        ExecutionMode::Safe,
        ExecutionMode::Interactive,
        ExecutionMode::Autonomous,
        ExecutionMode::Developer,
    ];
    for mode in modes {
        let policy = ExecutionPolicy::for_mode(mode);
        assert_eq!(policy.mode, mode);
    }
}

// ─────────────────────────── Failure Recovery Tests ───────────────────────────

#[test]
fn test_checkpoint_create_and_resume() {
    let recovery = FailureRecovery::new();
    let tid = TaskId::new();

    let cp1 = recovery.create_checkpoint(
        tid,
        serde_json::json!({"step": 1, "data": "partial"}),
        1,
        "after step 1".to_string(),
    );
    assert_eq!(cp1.task_id, tid);
    assert_eq!(cp1.step_index, 1);

    let cp2 = recovery.create_checkpoint(
        tid,
        serde_json::json!({"step": 2, "data": "more"}),
        2,
        "after step 2".to_string(),
    );

    let resumed = recovery.resume_from_checkpoint(tid).unwrap().unwrap();
    assert_eq!(resumed.id, cp2.id);
    assert_eq!(resumed.step_index, 2);

    let all_cps = recovery.checkpoints(tid);
    assert_eq!(all_cps.len(), 2);
}

#[test]
fn test_strategy_determination() {
    let recovery = FailureRecovery::new();
    let tid = TaskId::new();

    // No history -> retry
    let strategy = recovery.determine_strategy(tid, "generic error");
    assert!(matches!(strategy, FallbackStrategy::Retry));

    // Register a custom fallback
    recovery.register_fallback(
        "timeout".to_string(),
        FallbackConfig {
            strategy: FallbackStrategy::Skip,
            max_retries: 1,
            retry_delay_ms: 500,
            alternative_description: None,
        },
    );

    // Exhaust retries -> use custom config
    for _ in 0..5 {
        recovery.record_recovery_attempt(
            tid,
            FallbackStrategy::Retry,
            false,
            None,
            10,
        );
    }
    recovery.set_global_max_retries(3);

    let strategy = recovery.determine_strategy(tid, "timeout");
    assert!(matches!(strategy, FallbackStrategy::Skip));
}

#[test]
fn test_degradation_tracking() {
    let recovery = FailureRecovery::new();
    assert_eq!(recovery.degradation_level(), DegradationLevel::None);

    // Manual set
    recovery.set_degradation_level(DegradationLevel::Moderate);
    assert_eq!(recovery.degradation_level(), DegradationLevel::Moderate);

    recovery.set_degradation_level(DegradationLevel::None);
    assert_eq!(recovery.degradation_level(), DegradationLevel::None);
}

#[test]
fn test_recovery_log() {
    let recovery = FailureRecovery::new();
    let tid = TaskId::new();

    recovery.record_recovery_attempt(tid, FallbackStrategy::Retry, true, None, 50);
    recovery.record_recovery_attempt(
        tid,
        FallbackStrategy::Retry,
        false,
        Some("crash".to_string()),
        100,
    );

    assert_eq!(recovery.total_recovery_attempts(), 2);
    assert_eq!(recovery.successful_recoveries(), 1);

    let state = recovery.recovery_state(tid).unwrap();
    assert_eq!(state.total_retries, 2);
}

#[test]
fn test_recovery_global_max_retries() {
    let recovery = FailureRecovery::new();
    assert_eq!(recovery.global_max_retries(), 3);

    recovery.set_global_max_retries(5);
    assert_eq!(recovery.global_max_retries(), 5);
}

#[test]
fn test_recovery_clear_task() {
    let recovery = FailureRecovery::new();
    let tid = TaskId::new();
    recovery.create_checkpoint(tid, serde_json::json!({}), 0, "init".to_string());
    assert!(recovery.recovery_state(tid).is_some());

    recovery.clear_task_recovery(tid);
    assert!(recovery.recovery_state(tid).is_none());
}

// ─────────────────────────── Analytics Tests ───────────────────────────

#[test]
fn test_analytics_latency_stats() {
    let analytics = ExecutiveAnalytics::new();
    analytics.record_task_latency("task1", 100.0);
    analytics.record_task_latency("task2", 200.0);
    analytics.record_task_latency("task3", 150.0);
    analytics.record_task_latency("task4", 50.0);

    let stats = analytics.task_latency_stats();
    assert_eq!(stats.count, 4);
    assert!((stats.min_ms - 50.0).abs() < f64::EPSILON);
    assert!((stats.max_ms - 200.0).abs() < f64::EPSILON);
    assert!((stats.avg_ms - 125.0).abs() < f64::EPSILON);
    assert!(stats.p50_ms > 0.0);
    assert!(stats.p95_ms > 0.0);
    assert!(stats.p99_ms > 0.0);
}

#[test]
fn test_analytics_empty_latency() {
    let analytics = ExecutiveAnalytics::new();
    let stats = analytics.task_latency_stats();
    assert_eq!(stats.count, 0);
    assert_eq!(stats.avg_ms, 0.0);
}

#[test]
fn test_analytics_decision_quality() {
    let analytics = ExecutiveAnalytics::new();
    analytics.record_decision_quality(0.8);
    analytics.record_decision_quality(0.9);
    analytics.record_decision_quality(0.7);

    let avg = analytics.decision_quality_average();
    assert!((avg - 0.8).abs() < f64::EPSILON);
    assert_eq!(analytics.decision_quality_count(), 3);
}

#[test]
fn test_analytics_goal_completions() {
    let analytics = ExecutiveAnalytics::new();
    analytics.record_goal_completion("goal 1");
    analytics.record_goal_completion("goal 2");
    assert_eq!(analytics.goal_completion_count(), 2);
}

#[test]
fn test_analytics_snapshot() {
    let analytics = ExecutiveAnalytics::new();
    analytics.record_task_latency("t1", 100.0);
    analytics.record_task_latency("t2", 200.0);
    analytics.record_decision_quality(0.85);

    let mut state = GlobalState::new();
    state.completed_goals = 10;
    state.failed_goals = 2;
    state.cancelled_goals = 1;
    state.completed_tasks = 50;
    state.failed_tasks = 5;
    state.cancelled_tasks = 3;

    let sched_stats = SchedulerStats {
        total_scheduled: 60,
        total_completed: 55,
        total_preempted: 2,
        total_failed: 3,
        avg_latency_ms: 150.0,
        max_latency_ms: 500.0,
        current_queue_depth: 2,
        active_executions: 3,
    };

    let snap = analytics.snapshot(&state, &sched_stats);
    assert_eq!(snap.total_goals_completed, 10);
    assert_eq!(snap.total_tasks_completed, 50);
    assert!((snap.goal_completion_rate - (10.0 / 13.0)).abs() < 0.01);
    assert!((snap.task_success_rate - (50.0 / 58.0)).abs() < 0.01);
    assert!((snap.decision_quality_avg - 0.85).abs() < f64::EPSILON);
    assert!((snap.scheduler_efficiency - (55.0 / 60.0)).abs() < 0.01);
    assert!((snap.task_latency_avg_ms - 150.0).abs() < f64::EPSILON);
}

#[test]
fn test_analytics_scheduler_snapshot() {
    let analytics = ExecutiveAnalytics::new();
    let stats = SchedulerStats::default();
    analytics.record_scheduler_snapshot(stats.clone());
    let latest = analytics.latest_scheduler_stats().unwrap();
    assert_eq!(latest.total_scheduled, 0);
}

#[test]
fn test_analytics_clear() {
    let analytics = ExecutiveAnalytics::new();
    analytics.record_task_latency("t", 100.0);
    analytics.record_goal_completion("g");
    analytics.record_decision_quality(0.5);
    analytics.clear();
    assert_eq!(analytics.task_latency_count(), 0);
    assert_eq!(analytics.goal_completion_count(), 0);
    assert_eq!(analytics.decision_quality_count(), 0);
}

// ─────────────────────────── Executive API Tests ───────────────────────────

#[test]
fn test_api_goal_to_task_workflow() {
    let api = ExecutiveApi::new(ExecutionMode::Autonomous);

    let goal = api
        .create_goal("ship feature".to_string(), GoalPriority::High)
        .unwrap();
    assert_eq!(goal.state, GoalState::Proposed);

    api.goal_manager().accept_goal(goal.id).unwrap();
    api.goal_manager().start_executing(goal.id).unwrap();

    let task = api
        .submit_task("write code".to_string(), TaskPriority::High, Some(goal.id))
        .unwrap();
    assert_eq!(task.state, TaskState::Queued);

    api.task_manager().start_task(task.id, "worker".to_string()).unwrap();
    api.complete_task(task.id, serde_json::json!({"done": true}))
        .unwrap();

    api.complete_goal(goal.id).unwrap();

    let summary = api.inspect_execution();
    assert_eq!(summary.goals_created, 1);
    assert_eq!(summary.goals_completed, 1);
    assert_eq!(summary.tasks_created, 1);
    assert_eq!(summary.tasks_completed, 1);
}

#[test]
fn test_api_cancel_goal() {
    let api = ExecutiveApi::new(ExecutionMode::Autonomous);
    let goal = api
        .create_goal("cancel me".to_string(), GoalPriority::Low)
        .unwrap();
    api.cancel_goal(goal.id).unwrap();
    assert!(api.goal_manager().get_goal(goal.id).unwrap().state.is_terminal());

    let summary = api.inspect_execution();
    assert_eq!(summary.goals_cancelled, 1);
}

#[test]
fn test_api_export_summary() {
    let api = ExecutiveApi::new(ExecutionMode::Developer);
    let _goal = api
        .create_goal("exported".to_string(), GoalPriority::Normal)
        .unwrap();

    let export = api.export_execution_summary().unwrap();
    assert!(export.is_object());
    assert!(export.get("session").is_some());
    assert!(export.get("global_state").is_some());
    assert!(export.get("execution_mode").is_some());
    assert!(export.get("analytics").is_some());
}

#[test]
fn test_api_session() {
    let api = ExecutiveApi::new(ExecutionMode::Interactive);
    let session = api.create_session();
    assert_eq!(session.state, SessionState::Created);
}

#[test]
fn test_api_pause_resume_goal() {
    let api = ExecutiveApi::new(ExecutionMode::Autonomous);
    let goal = api
        .create_goal("pausable".to_string(), GoalPriority::Normal)
        .unwrap();

    api.goal_manager().accept_goal(goal.id).unwrap();
    api.goal_manager().start_executing(goal.id).unwrap();

    api.pause_goal(goal.id).unwrap();
    assert_eq!(
        api.goal_manager().get_goal(goal.id).unwrap().state,
        GoalState::Paused
    );

    api.resume_goal(goal.id).unwrap();
    assert_eq!(
        api.goal_manager().get_goal(goal.id).unwrap().state,
        GoalState::Executing
    );
}

#[test]
fn test_api_subsystem_access() {
    let api = ExecutiveApi::new(ExecutionMode::Autonomous);
    // Verify all subsystem accessors return valid references
    let _ = api.goal_manager();
    let _ = api.task_manager();
    let _ = api.session_manager();
    let _ = api.context();
    let _ = api.scheduler();
    let _ = api.analytics();
    let _ = api.recovery();
    let _ = api.policy_engine();
}

// ─────────────────────────── Session Manager Tests ───────────────────────────

#[test]
fn test_session_lifecycle() {
    let mut session = Session::new();
    assert_eq!(session.state, SessionState::Created);

    session.activate().unwrap();
    assert_eq!(session.state, SessionState::Active);

    session.pause().unwrap();
    assert_eq!(session.state, SessionState::Paused);

    session.activate().unwrap();
    session.complete().unwrap();
    assert!(session.state.is_terminal());
}

#[test]
fn test_session_fail_and_cancel() {
    let mut session = Session::new();
    session.activate().unwrap();

    session.fail().unwrap();
    assert_eq!(session.state, SessionState::Failed);

    let mut session2 = Session::new();
    session2.activate().unwrap();
    session2.cancel().unwrap();
    assert!(session2.state.is_terminal());
}

#[test]
fn test_session_terminal_blocks() {
    let mut session = Session::new();
    session.activate().unwrap();
    session.complete().unwrap();

    assert!(session.activate().is_err());
    assert!(session.pause().is_err());
    assert!(session.fail().is_err());
    assert!(session.cancel().is_err());
}

#[test]
fn test_session_goal_and_task_tracking() {
    let mut session = Session::new();
    let gid = GoalId::new();
    let tid = TaskId::new();

    session.add_goal(gid);
    session.add_task(tid);
    assert_eq!(session.goal_ids.len(), 1);
    assert_eq!(session.task_ids.len(), 1);

    session.remove_goal(gid);
    session.remove_task(tid);
    assert!(session.goal_ids.is_empty());
    assert!(session.task_ids.is_empty());
}

#[test]
fn test_session_snapshot() {
    let mut session = Session::new();
    session.set_metadata("key".to_string(), serde_json::json!("value"));
    let snap = session.snapshot();
    assert_eq!(snap.session_id, session.id);
    assert_eq!(snap.state, SessionState::Created);
    assert_eq!(
        snap.metadata.get("key"),
        Some(&serde_json::json!("value"))
    );
}

#[test]
fn test_session_manager() {
    let mgr = SessionManager::new();
    let s1 = mgr.create_session();
    let mut s2 = mgr.create_session();
    s2.activate().unwrap();
    mgr.update_session(s2);

    assert_eq!(mgr.session_count(), 2);
    assert_eq!(mgr.active_sessions().len(), 1);
    assert!(mgr.get_session(s1.id).is_some());

    let removed = mgr.remove_session(s1.id);
    assert!(removed.is_some());
    assert_eq!(mgr.session_count(), 1);

    let counts = mgr.sessions_by_state();
    assert_eq!(counts.get(&SessionState::Active), Some(&1));
}

// ─────────────────────────── Context Tests ───────────────────────────

#[test]
fn test_context_mode() {
    let ctx = ExecutiveContext::new(ExecutionMode::Safe);
    assert_eq!(ctx.mode(), ExecutionMode::Safe);

    ctx.set_mode(ExecutionMode::Developer);
    assert_eq!(ctx.mode(), ExecutionMode::Developer);
}

#[test]
fn test_context_goals_and_tasks() {
    let ctx = ExecutiveContext::new(ExecutionMode::Autonomous);
    let goal = Goal::new("ctx goal".to_string(), GoalPriority::Normal);
    let gid = goal.id;
    ctx.add_goal(goal);
    assert!(ctx.get_goal(gid).is_some());
    assert_eq!(ctx.all_goals().len(), 1);
    assert_eq!(ctx.active_goals().len(), 1);

    ctx.remove_goal(gid);
    assert!(ctx.get_goal(gid).is_none());

    let task = Task::new("ctx task".to_string());
    let tid = task.id;
    ctx.add_task(task);
    assert!(ctx.get_task(tid).is_some());
    assert_eq!(ctx.all_tasks().len(), 1);
}

#[test]
fn test_context_global_state() {
    let ctx = ExecutiveContext::new(ExecutionMode::Autonomous);
    ctx.record_goal_completed();
    ctx.record_goal_completed();
    ctx.record_task_failed();
    ctx.record_inference_call();
    ctx.record_reasoning_call();
    ctx.record_memory_access();
    ctx.record_knowledge_access();

    let state = ctx.global_state();
    assert_eq!(state.completed_goals, 2);
    assert_eq!(state.failed_tasks, 1);
    assert_eq!(state.total_inference_calls, 1);
    assert_eq!(state.total_reasoning_calls, 1);
    assert_eq!(state.total_memory_accesses, 1);
    assert_eq!(state.total_knowledge_accesses, 1);
}

#[test]
fn test_context_tools() {
    let ctx = ExecutiveContext::new(ExecutionMode::Developer);
    ctx.register_tool("shell".to_string());
    ctx.register_tool("editor".to_string());
    assert!(ctx.has_tool("shell"));
    assert!(ctx.has_tool("editor"));
    assert!(!ctx.has_tool("missing"));
    assert_eq!(ctx.available_tools().len(), 2);

    // Dedup
    ctx.register_tool("shell".to_string());
    assert_eq!(ctx.available_tools().len(), 2);
}

#[test]
fn test_context_environment() {
    let ctx = ExecutiveContext::new(ExecutionMode::Interactive);
    ctx.set_variable("key".to_string(), serde_json::json!("value"));
    assert_eq!(
        ctx.get_variable("key"),
        Some(serde_json::json!("value"))
    );
    assert!(ctx.get_variable("missing").is_none());
    assert_eq!(ctx.environment().len(), 1);
}

#[test]
fn test_context_capacity() {
    let ctx = ExecutiveContext::new(ExecutionMode::Autonomous);
    ctx.set_max_concurrent_goals(2);
    ctx.set_max_concurrent_tasks(4);

    assert_eq!(ctx.max_concurrent_goals(), 2);
    assert_eq!(ctx.max_concurrent_tasks(), 4);
    assert!(ctx.can_accept_goal());
    assert!(ctx.can_accept_task());
}

#[test]
fn test_context_resource_utilization() {
    let ctx = ExecutiveContext::new(ExecutionMode::Autonomous);
    ctx.set_resource_utilization("cpu".to_string(), 0.75);
    ctx.set_resource_utilization("gpu".to_string(), 0.50);
    let state = ctx.global_state();
    assert_eq!(state.resource_utilization.get("cpu"), Some(&0.75));
    assert_eq!(state.resource_utilization.get("gpu"), Some(&0.50));
}

#[test]
fn test_context_uptime() {
    let ctx = ExecutiveContext::new(ExecutionMode::Safe);
    let uptime = ctx.uptime_ms();
    assert!(uptime < 1000); // Should be very small right after creation
}

// ─────────────────────────── Decision Coordination Tests ───────────────────────────

#[tokio::test]
async fn test_decision_make_decision() {
    let coordinator = DecisionCoordinator::new();
    let context = ExecutiveContext::new(ExecutionMode::Autonomous);

    let request = DecisionRequest {
        id: "arch-decision".to_string(),
        description: "choose architecture".to_string(),
        options: vec![
            DecisionOption {
                id: "monolith".to_string(),
                description: "monolithic architecture".to_string(),
                estimated_cost: 2.0,
                estimated_benefit: 6.0,
                risk_level: 0.2,
            },
            DecisionOption {
                id: "microservices".to_string(),
                description: "microservices architecture".to_string(),
                estimated_cost: 5.0,
                estimated_benefit: 9.0,
                risk_level: 0.5,
            },
        ],
        context: HashMap::new(),
        constraints: vec!["team size < 10".to_string()],
    };

    let result = coordinator.make_decision(&request, &context).await.unwrap();
    assert!(result.confidence > 0.0);
    assert!(result.selected_option.is_some());
    assert!(!result.sources.is_empty());
    assert_eq!(result.request_id, "arch-decision");

    // Decision should be recorded
    assert_eq!(coordinator.decision_count(), 1);
    assert_eq!(coordinator.recent_decisions().len(), 1);
}

#[tokio::test]
async fn test_decision_invoke_subsystems() {
    let coordinator = DecisionCoordinator::new();
    let context = ExecutiveContext::new(ExecutionMode::Autonomous);

    let request = DecisionRequest {
        id: "test".to_string(),
        description: "test".to_string(),
        options: vec![DecisionOption {
            id: "a".to_string(),
            description: "opt A".to_string(),
            estimated_cost: 1.0,
            estimated_benefit: 5.0,
            risk_level: 0.1,
        }],
        context: HashMap::new(),
        constraints: vec![],
    };

    let reasoning = coordinator.invoke_reasoning(&request, &context).await.unwrap();
    assert!(reasoning.is_object());

    let memory = coordinator.invoke_memory(&request, &context).await.unwrap();
    assert!(memory.is_object());

    let knowledge = coordinator.invoke_knowledge(&request, &context).await.unwrap();
    assert!(knowledge.is_object());

    let inference = coordinator.invoke_inference(&request, &context).await.unwrap();
    assert!(inference.is_object());

    // Context should have recorded calls
    let state = context.global_state();
    assert_eq!(state.total_reasoning_calls, 1);
    assert_eq!(state.total_memory_accesses, 1);
    assert_eq!(state.total_knowledge_accesses, 1);
    assert_eq!(state.total_inference_calls, 1);
}

#[tokio::test]
async fn test_decision_tool_invocation() {
    let coordinator = DecisionCoordinator::new();
    let context = ExecutiveContext::new(ExecutionMode::Developer);
    context.register_tool("shell".to_string());

    let input = serde_json::json!({"command": "ls"});
    let result = coordinator
        .invoke_tool("shell", &input, &context)
        .await
        .unwrap();
    assert!(result.is_object());

    // Unregistered tool should fail
    let result = coordinator
        .invoke_tool("unknown", &input, &context)
        .await;
    assert!(result.is_err());
}

#[test]
fn test_decision_merge_results() {
    let coordinator = DecisionCoordinator::new();

    let reasoning = serde_json::json!({"analysis": {"best_option": "a"}});
    let memory = serde_json::json!({"recommendation": "proceed"});
    let knowledge = serde_json::json!({"knowledge_confidence": 0.8});
    let inference = serde_json::json!({"inference_confidence": 0.9});

    let mut tools = HashMap::new();
    tools.insert("shell".to_string(), serde_json::json!({"status": "ok"}));

    let merged = coordinator.merge_results(
        Some(reasoning),
        Some(memory),
        Some(knowledge),
        Some(inference),
        tools,
    );

    assert!(merged.confidence > 0.0);
    assert!(merged.merged_output.is_object());
    assert!(merged.reasoning_result.is_some());
    assert!(merged.memory_result.is_some());
    assert!(merged.knowledge_result.is_some());
    assert!(merged.inference_result.is_some());
    assert_eq!(merged.tool_results.len(), 1);
}

#[test]
fn test_decision_merge_empty() {
    let coordinator = DecisionCoordinator::new();
    let merged = coordinator.merge_results(None, None, None, None, HashMap::new());
    assert_eq!(merged.confidence, 0.0);
    assert!(merged.tool_results.is_empty());
}

#[test]
fn test_decision_tool_registry() {
    let coordinator = DecisionCoordinator::new();
    coordinator.register_tool("shell".to_string(), "run commands".to_string());
    coordinator.register_tool("editor".to_string(), "edit files".to_string());

    let tools = coordinator.registered_tools();
    assert_eq!(tools.len(), 2);
    assert!(tools.contains_key("shell"));
}

#[test]
fn test_decision_confidence_threshold() {
    let coordinator = DecisionCoordinator::new();
    assert!((coordinator.confidence_threshold() - 0.5).abs() < f64::EPSILON);

    coordinator.set_confidence_threshold(0.8);
    assert!((coordinator.confidence_threshold() - 0.8).abs() < f64::EPSILON);

    // Clamping
    coordinator.set_confidence_threshold(2.0);
    assert!((coordinator.confidence_threshold() - 1.0).abs() < f64::EPSILON);
}

// ─────────────────────────── Error Tests ───────────────────────────

#[test]
fn test_error_creation_and_display() {
    let err = ExecutiveError::new(ExecutiveErrorCode::GoalNotFound, "gone");
    assert_eq!(err.code(), ExecutiveErrorCode::GoalNotFound);
    assert_eq!(err.message(), "gone");

    let err_with_ctx = err.with_context("in session abc");
    assert_eq!(err_with_ctx.context().len(), 1);

    let display = format!("{}", err_with_ctx);
    assert!(display.contains("goal not found"));
    assert!(display.contains("in session abc"));
}

#[test]
fn test_error_helpers() {
    let e1 = ExecutiveError::goal_not_found("g1");
    assert_eq!(e1.code(), ExecutiveErrorCode::GoalNotFound);

    let e2 = ExecutiveError::task_not_found("t1");
    assert_eq!(e2.code(), ExecutiveErrorCode::TaskNotFound);

    let e3 = ExecutiveError::session_not_found("s1");
    assert_eq!(e3.code(), ExecutiveErrorCode::SessionNotFound);

    let e4 = ExecutiveError::policy_violation("blocked");
    assert_eq!(e4.code(), ExecutiveErrorCode::PolicyViolation);

    let e5 = ExecutiveError::internal("oops");
    assert_eq!(e5.code(), ExecutiveErrorCode::InternalError);
}

// ─────────────────────────── Goal State Machine Tests ───────────────────────────

#[test]
fn test_goal_state_valid_transitions() {
    assert!(GoalState::Proposed.can_transition_to(GoalState::Accepted));
    assert!(GoalState::Proposed.can_transition_to(GoalState::Cancelled));
    assert!(!GoalState::Proposed.can_transition_to(GoalState::Executing));

    assert!(GoalState::Accepted.can_transition_to(GoalState::Planning));
    assert!(GoalState::Accepted.can_transition_to(GoalState::Executing));
    assert!(!GoalState::Accepted.can_transition_to(GoalState::Completed));

    assert!(GoalState::Executing.can_transition_to(GoalState::Paused));
    assert!(GoalState::Executing.can_transition_to(GoalState::Completed));
    assert!(GoalState::Executing.can_transition_to(GoalState::Failed));

    assert!(GoalState::Paused.can_transition_to(GoalState::Executing));
    assert!(!GoalState::Paused.can_transition_to(GoalState::Completed));

    assert!(!GoalState::Completed.can_transition_to(GoalState::Executing));
    assert!(!GoalState::Failed.can_transition_to(GoalState::Accepted));
}

#[test]
fn test_task_state_valid_transitions() {
    assert!(TaskState::Pending.can_transition_to(TaskState::Queued));
    assert!(TaskState::Pending.can_transition_to(TaskState::Cancelled));
    assert!(!TaskState::Pending.can_transition_to(TaskState::Running));

    assert!(TaskState::Running.can_transition_to(TaskState::Completed));
    assert!(TaskState::Running.can_transition_to(TaskState::Failed));
    assert!(TaskState::Running.can_transition_to(TaskState::TimedOut));
    assert!(TaskState::Running.can_transition_to(TaskState::Paused));

    assert!(TaskState::Failed.can_transition_to(TaskState::Retrying));
    assert!(!TaskState::Failed.can_transition_to(TaskState::Running));

    assert!(TaskState::Retrying.can_transition_to(TaskState::Queued));
    assert!(TaskState::Retrying.can_transition_to(TaskState::Failed));

    assert!(!TaskState::Completed.can_transition_to(TaskState::Running));
}

// ─────────────────────────── Cross-Subsystem Integration Tests ───────────────────────────

#[test]
fn test_goal_with_dependencies_and_priority_engine() {
    let gm = GoalManager::new();
    let engine = PriorityEngine::new();

    let g1 = gm.create_goal("base".to_string(), GoalPriority::Normal);
    let g2 = gm.create_goal("dependent".to_string(), GoalPriority::High);
    gm.add_dependency(g2.id, g1.id).unwrap();

    let score1 = engine.score_goal(
        None,
        g1.priority,
        false,
        0,
        1, // g2 depends on g1
        &[],
        g1.created_at,
    );
    engine.set_goal_score(g1.id, score1);

    let score2 = engine.score_goal(
        None,
        g2.priority,
        false,
        1,
        0, // g2 depends on g1
        &[],
        g2.created_at,
    );
    engine.set_goal_score(g2.id, score2);

    assert!(engine.get_goal_score(g1.id).is_some());
    assert!(engine.get_goal_score(g2.id).is_some());
    // g2 has higher priority so higher urgency
    assert!(score2.total > score1.total);
}

#[test]
fn test_scheduler_with_resources() {
    let sched = ExecutiveScheduler::default();
    let engine = PriorityEngine::new();
    let rc = ResourceCoordinator::new();

    let task = Task::new("resource-hungry".to_string());
    let exec_id = sched.schedule_task(&task, &engine).unwrap();

    let exec = sched.dequeue_next().unwrap();
    assert_eq!(exec.id, exec_id);

    // Allocate some resources
    let _alloc = rc.allocate(ResourceType::Cpu, 6, "task".to_string()).unwrap();

    let mut reqs = HashMap::new();
    reqs.insert(ResourceType::Cpu, 3);
    // Only 2 CPUs left, need 3 -> cannot satisfy
    assert!(!sched.can_schedule_with_resources(&reqs, &rc));

    sched.complete_execution(exec_id).unwrap();
}

#[test]
fn test_recovery_with_task_failure() {
    let recovery = FailureRecovery::new();
    let tm = TaskManager::new();

    let task = tm
        .create_task("risky".to_string())
        .with_retry_policy(RetryPolicy {
            max_retries: 3,
            base_delay_ms: 10,
            max_delay_ms: 100,
            backoff_multiplier: 2.0,
        });
    let tid = task.id;

    // Create checkpoint before failure
    recovery.create_checkpoint(
        tid,
        serde_json::json!({"progress": "50%"}),
        2,
        "halfway".to_string(),
    );

    tm.submit_task(task).unwrap();
    tm.start_task(tid, "worker".to_string()).unwrap();

    // Fail and record
    tm.fail_task(tid, "crash".to_string()).unwrap();
    recovery.record_recovery_attempt(tid, FallbackStrategy::Retry, false, Some("crash".to_string()), 200);

    let state = recovery.recovery_state(tid).unwrap();
    assert_eq!(state.total_retries, 1);
    assert_eq!(state.checkpoints.len(), 1);

    let resumed = recovery.resume_from_checkpoint(tid).unwrap().unwrap();
    assert_eq!(resumed.step_index, 2);
}

#[test]
fn test_analytics_with_scheduler_and_goals() {
    let analytics = ExecutiveAnalytics::new();
    let gm = GoalManager::new();
    let sched = ExecutiveScheduler::default();
    let engine = PriorityEngine::new();

    let goal = gm.create_goal("tracked".to_string(), GoalPriority::Normal);
    gm.accept_goal(goal.id).unwrap();
    gm.start_executing(goal.id).unwrap();
    gm.complete_goal(goal.id).unwrap();
    analytics.record_goal_completion(&goal.description);

    let task = Task::new("tracked task".to_string());
    let exec_id = sched.schedule_task(&task, &engine).unwrap();
    analytics.record_task_latency(&task.name, 42.0);
    analytics.record_decision_quality(0.92);

    let sched_stats = sched.statistics();
    let global = GlobalState::new();
    let snap = analytics.snapshot(&global, &sched_stats);
    assert_eq!(snap.total_goals_completed, 0); // GlobalState not updated
    assert_eq!(analytics.goal_completion_count(), 1);
    assert!((analytics.decision_quality_average() - 0.92).abs() < f64::EPSILON);
}

// ─────────────────────────── Goal Priority Score Test ───────────────────────────

#[test]
fn test_goal_priority_score_values() {
    assert_eq!(GoalPriority::Critical.score(), 4);
    assert_eq!(GoalPriority::High.score(), 3);
    assert_eq!(GoalPriority::Normal.score(), 2);
    assert_eq!(GoalPriority::Low.score(), 1);
    assert_eq!(GoalPriority::Background.score(), 0);
}

#[test]
fn test_task_priority_score_values() {
    assert_eq!(TaskPriority::Critical.score(), 4);
    assert_eq!(TaskPriority::High.score(), 3);
    assert_eq!(TaskPriority::Normal.score(), 2);
    assert_eq!(TaskPriority::Low.score(), 1);
    assert_eq!(TaskPriority::Background.score(), 0);
}

// ─────────────────────────── Inference Budget Tests ───────────────────────────

#[test]
fn test_inference_budget_can_consume() {
    let budget = InferenceBudget::new(1000, BudgetPeriod::PerHour);
    assert!(budget.can_consume(500));
    assert!(!budget.can_consume(1001));
    assert_eq!(budget.remaining(), 1000);
}

// ─────────────────────────── Goal Time Remaining ───────────────────────────

#[test]
fn test_goal_deadline_and_overdue() {
    let mut goal = Goal::new("deadline test".to_string(), GoalPriority::Normal);

    // No deadline
    assert!(!goal.is_overdue());
    assert!(goal.time_remaining_secs().is_none());

    // Future deadline
    goal.deadline = Some(Utc::now() + Duration::hours(1));
    assert!(!goal.is_overdue());
    assert!(goal.time_remaining_secs().unwrap() > 0);

    // Past deadline
    goal.deadline = Some(Utc::now() - Duration::hours(1));
    assert!(goal.is_overdue());
    assert_eq!(goal.time_remaining_secs().unwrap(), 0);
}

#[test]
fn test_goal_builder_chain() {
    let parent_id = GoalId::new();
    let dep_id = GoalId::new();

    let goal = Goal::new("chained".to_string(), GoalPriority::High)
        .with_parent(parent_id)
        .with_dependency(dep_id)
        .with_context("key".to_string(), serde_json::json!("val"))
        .with_deadline(Utc::now() + Duration::days(1));

    assert_eq!(goal.parent_id, Some(parent_id));
    assert!(goal.dependencies.contains(&dep_id));
    assert_eq!(goal.context.get("key"), Some(&serde_json::json!("val")));
    assert!(goal.deadline.is_some());
}

#[test]
fn test_task_builder_chain() {
    let goal_id = GoalId::new();
    let dep_id = TaskId::new();

    let task = Task::new("chained".to_string())
        .with_description("a task".to_string())
        .with_priority(TaskPriority::Critical)
        .with_goal(goal_id)
        .with_deadline(Utc::now() + Duration::hours(2))
        .with_timeout_ms(5000)
        .with_dependency(dep_id)
        .with_tag("important".to_string())
        .with_context("env".to_string(), serde_json::json!("prod"));

    assert_eq!(task.description, "a task");
    assert_eq!(task.priority, TaskPriority::Critical);
    assert_eq!(task.goal_id, Some(goal_id));
    assert!(task.deadline.is_some());
    assert_eq!(task.timeout_ms, Some(5000));
    assert!(task.dependencies.contains(&dep_id));
    assert!(task.tags.contains(&"important".to_string()));
}
