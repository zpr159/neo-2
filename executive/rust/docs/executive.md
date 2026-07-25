# Neo Executive System

The Executive System coordinates all cognitive processes in the Neo AGI OS. It manages goal decomposition, task orchestration, prioritization, scheduling, resource allocation, and system-wide decision making.

## 1. Executive Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                     Executive API                                │
│  (high-level interface, sessions, export)                        │
├──────────┬──────────┬──────────┬──────────┬─────────────────────┤
│   Goal   │   Task   │ Priority │ Attention│     Scheduler       │
│  Manager │  Manager │  Engine  │  Manager │  (parallel, deps,   │
│(hierarchy│ (queue,  │(dynamic, │ (focus,  │   resource-aware,   │
│ decompose│  retry,  │ urgency, │  switch, │   preemption)       │
│ persist) │ deadline)│  resource)│ budget) │                     │
├──────────┴──────────┴──────────┴──────────┴─────────────────────┤
│              Decision Coordination                                │
│  (reasoning, memory, knowledge, inference, tools, merge)        │
├─────────────────────────────────────────────────────────────────┤
│              Resource Coordination                                │
│  (CPU, GPU, RAM, model allocation, inference budget)            │
├─────────────────────────────────────────────────────────────────┤
│  Execution Policies │ Failure Recovery │ Analytics              │
│  (safe, interactive,│ (retries,        │ (latency,              │
│   autonomous,       │  fallback,       │  completion,           │
│   developer)        │  checkpoint,     │  quality)              │
│                     │  degradation)    │                        │
└─────────────────────┴──────────────────┴────────────────────────┘
```

All subsystems are thread-safe (`Clone`-able via internal `Arc<RwLock<...>>` wrappers) and can be used concurrently from multiple threads.

### Module Overview

| Module | Purpose |
|---|---|
| `goal` | Goal lifecycle, hierarchy, decomposition, dependencies |
| `task` | Task lifecycle, queue, retry, ownership, deadlines |
| `priority` | Dynamic scoring (urgency, importance, resource, age) |
| `attention` | Focus management, context switching, interrupt handling, budget |
| `scheduler` | Task scheduling, parallel execution, preemption, resource awareness |
| `decision_coordination` | Invoke cognitive subsystems and merge results |
| `resource_coordination` | Hardware/logical resource pools, model allocation, inference budget |
| `policies` | Execution mode policies and permission enforcement |
| `recovery` | Checkpointing, fallback strategies, graceful degradation |
| `analytics` | Latency tracking, decision quality, system snapshots |
| `session` | Executive session grouping |
| `context` | Global state, environment, tools, capacity limits |
| `error` | Typed error codes and error construction |

---

## 2. Goal Lifecycle

Goals represent desired outcomes the executive works to achieve. Each goal follows a strict state machine:

```
Proposed → Accepted → Planning → Executing → Completed
                       Executing → Failed
                       Executing → Paused → Executing
Any non-terminal    → Cancelled
```

### State Transitions

| From | Valid Transitions |
|---|---|
| `Proposed` | `Accepted`, `Cancelled` |
| `Accepted` | `Planning`, `Executing`, `Cancelled` |
| `Planning` | `Executing`, `Failed`, `Cancelled` |
| `Executing` | `Paused`, `Completed`, `Failed`, `Cancelled` |
| `Paused` | `Executing`, `Cancelled` |
| `Completed` | *(terminal)* |
| `Failed` | *(terminal)* |
| `Cancelled` | *(terminal)* |

### GoalManager API

```rust
let gm = GoalManager::new();
let goal = gm.create_goal("description".into(), GoalPriority::High);
gm.accept_goal(goal.id)?;
gm.start_planning(goal.id)?;
gm.start_executing(goal.id)?;
gm.complete_goal(goal.id)?;
```

**Key methods:**

- `create_goal(description, priority)` — Creates and registers a goal in `Proposed` state
- `accept_goal(id)` — Transitions to `Accepted`
- `start_planning(id)` — Transitions to `Planning`
- `start_executing(id)` — Transitions to `Executing`
- `pause_goal(id)` / `resume_goal(id)` — Pause and resume
- `complete_goal(id)` — Transitions to `Completed`, sets progress to 1.0
- `fail_goal(id, reason)` — Transitions to `Failed`, stores reason in metadata
- `cancel_goal(id)` — Transitions to `Cancelled`
- `goals_by_priority()` — Returns active goals sorted by priority (highest first)
- `ready_goals()` — Returns accepted goals whose dependencies are all completed
- `overdue_goals()` — Returns goals past their deadline
- `goal_count()` — Total goal count

### Dependencies

Goals can depend on other goals. Dependencies are validated for cycles:

```rust
gm.add_dependency(g2.id, g1.id)?; // g2 depends on g1
// Self-dependency is rejected:
gm.add_dependency(g1.id, g1.id)?; // Err(GoalDependencyCycle)
// Circular dependencies are detected and rejected
```

### Decomposition

Goals can be decomposed into ordered steps:

```rust
gm.decompose_goal(goal.id, vec![
    "design".into(),
    "implement".into(),
    "test".into(),
])?;
// Progress auto-updates as steps are completed
```

### Priority Levels

| Level | Score |
|---|---|
| `Critical` | 4 |
| `High` | 3 |
| `Normal` | 2 |
| `Low` | 1 |
| `Background` | 0 |

### Sub-Goals

Goals support parent-child relationships:

```rust
let mut parent = gm.get_goal(parent_id)?;
parent.add_sub_goal(child_id);
gm.update_goal(parent)?;
let children = gm.sub_goals(parent_id)?;
```

---

## 3. Task Lifecycle

Tasks represent units of work. They follow a state machine:

```
Pending → Queued → Running → Completed
                    Running → Failed → Retrying → Queued
                    Running → TimedOut → Retrying
                    Running → Paused → Running
Any non-terminal → Cancelled
```

### TaskManager API

```rust
let tm = TaskManager::new();
let task = tm.create_task("build feature".into());
tm.submit_task(task)?;     // Pending → Queued
tm.start_task(id, "worker-1".into())?;  // Queued → Running
tm.complete_task(id, json!({"ok": true}))?;  // Running → Completed
```

**Key methods:**

- `create_task(name)` — Creates a task in `Pending` state
- `submit_task(task)` — Transitions to `Queued`, adds to queue
- `start_task(id, owner)` — Claims ownership and transitions to `Running`
- `complete_task(id, result)` — Transitions to `Completed` with result
- `fail_task(id, error)` — Transitions to `Failed`, auto-retries if allowed; returns `bool` indicating retry
- `cancel_task(id)` — Transitions to `Cancelled`, removes from queue
- `ready_tasks()` — Queued tasks whose dependencies are completed
- `tasks_by_priority()` — Active tasks sorted by priority
- `tasks_by_owner(owner)` — Tasks owned by a specific worker

### Ownership

Tasks can only be started by one owner at a time:

```rust
tm.start_task(id, "worker-A".into())?;
tm.start_task(id, "worker-B".into())?; // Err(TaskOwnershipConflict)
```

### RetryPolicy

```rust
let task = Task::new("flaky".into()).with_retry_policy(RetryPolicy {
    max_retries: 3,
    base_delay_ms: 1000,
    max_delay_ms: 30_000,
    backoff_multiplier: 2.0,
});
```

Delay calculation: `min(base * multiplier^attempt, max_delay)`

When `fail_task` is called and retries remain, the task transitions to `Retrying` and the caller receives `true`.

### Deadline Management

```rust
let task = Task::new("urgent".into())
    .with_deadline(Utc::now() + Duration::hours(1))
    .with_timeout_ms(3600_000);

// check_deadlines() auto-marks overdue running tasks as TimedOut
let timed_out = tm.check_deadlines();
```

---

## 4. Scheduler

The `ExecutiveScheduler` manages task scheduling with priority-based ordering, parallel execution, resource awareness, and preemption.

### SchedulingPolicy

```rust
SchedulingPolicy {
    max_parallel: 8,
    enable_preemption: true,
    enable_resource_awareness: true,
    deadline_weight: 0.7,
    starvation_threshold_secs: 300,
}
```

### Scheduling Flow

```rust
let sched = ExecutiveScheduler::new(policy);
let engine = PriorityEngine::new();

// Schedule a task (computes priority score, adds to heap)
let exec_id = sched.schedule_task(&task, &engine)?;

// Dequeue the highest-priority task
let execution = sched.dequeue_next()?;

// Complete execution
sched.complete_execution(exec_id)?;
```

### Priority Ordering

Tasks are scheduled into a max-heap ordered by `PriorityScore.total`. The highest-scored task is dequeued first.

### Preemption

When enabled, a running execution can be preempted by a higher-priority task:

```rust
let preempted = sched.preempt_execution(
    exec_id,
    "higher priority arrived".into(),
    preemptor_task_id,
)?;
// Preempted task is re-queued with a fresh scheduled_at timestamp
```

Critical-priority tasks are never preemptible. The preemption log is available via `preemption_log()`.

### Resource-Aware Scheduling

```rust
let rc = ResourceCoordinator::new();
let mut reqs = HashMap::new();
reqs.insert(ResourceType::Cpu, 4);
reqs.insert(ResourceType::Ram, 8192);

if sched.can_schedule_with_resources(&reqs, &rc) {
    // Safe to schedule
}
```

### SchedulerStats

Tracks total scheduled, completed, preempted, and failed executions along with latency metrics.

---

## 5. Priority Engine

The `PriorityEngine` dynamically computes priority scores based on four factors:

```
total = urgency × 0.4 + importance × 0.35 + resource_factor × 0.15 + age_factor × 0.1
```

### Urgency

Based on deadline proximity and static priority level:

- Deadline < 1 hour: urgency = 1.0
- Deadline < 1 day: urgency = 0.7
- Deadline > 1 day: urgency = 0.3
- No deadline: urgency = 0.5

Combined with the priority factor (Critical=1.0, High=0.8, etc.).

### Importance

Based on goal hierarchy and dependency graph position:

```rust
engine.calculate_importance(
    has_sub_goals,      // boost if has children
    dependency_count,   // more deps → more important
    dependent_count,    // more dependents → more important
);
```

### Resource Factor

Inversely proportional to resource utilization. Higher availability = higher priority boost.

### Age Factor

Older tasks get a slight priority boost (up to 1.0 after 24 hours).

### Conflict Resolution

| Strategy | Behavior |
|---|---|
| `PriorityFirst` | Higher score wins (default) |
| `DeadlineFirst` | Earlier deadline wins |
| `FairShare` | Same as PriorityFirst (extensible) |
| `OldestFirst` | Lower score (older) wins |
| `ResourceOptimal` | Same as PriorityFirst (extensible) |

```rust
engine.set_resolution_strategy(ConflictResolution::DeadlineFirst);
let a_wins = engine.resolve_conflict(0.5, 0.9, earlier_deadline, later_deadline);
```

### Score Storage

Scores can be stored and retrieved for later use:

```rust
engine.set_goal_score(goal_id, score);
let stored = engine.get_goal_score(goal_id);
```

### Priority Rules

Custom rules can adjust scores based on conditions:

```rust
engine.add_rule(PriorityRule {
    name: "boost-critical".into(),
    condition: "priority == critical".into(),
    adjustment: 0.2,
    active: true,
});
```

---

## 6. Attention Manager

The `AttentionManager` controls where the executive focuses its processing capacity, manages context switching, handles interrupts, and enforces an attention budget.

### Focus

```rust
let am = AttentionManager::new(100.0); // budget of 100 units

am.focus_on_goal(goal_id, "design phase".into(), 3.0);
am.focus_on_task(task_id, "run tests".into(), 2.0); // triggers context switch
am.clear_focus(); // returns focus to history
```

### Attention Budget

The budget prevents over-allocation of attention:

```rust
let budget = AttentionBudget::new(10.0);
budget.can_allocate(5.0);   // true
budget.can_allocate(11.0);  // false
budget.reserve(3.0);        // reserve capacity
budget.commit(3.0);         // convert reservation to consumption
budget.release(2.0);        // free consumed budget
budget.remaining();         // remaining capacity
budget.utilization();       // consumed / total
```

### Context Switching

Every focus change is tracked with timestamps and reason:

```rust
let switches = am.context_switch_history();
// ContextSwitchEvent { timestamp, from, to, reason, duration_ms }
```

### Interrupt Handling

Interrupts are queued and processed in order:

```rust
am.queue_interrupt(Interrupt::new(
    InterruptType::Critical,
    "sensor".into(),
    "temperature spike".into(),
));

while am.has_pending_interrupts() {
    let interrupt = am.process_next_interrupt().unwrap();
    // Handle interrupt
}
```

### Focus Statistics

```rust
let stats = am.focus_stats(); // HashMap<String, u64>
// Maps focus description → count of times focused
```

---

## 7. Decision Coordination

The `DecisionCoordinator` invokes cognitive subsystems (Reasoning Engine, Memory, Knowledge Graph, Inference Engine, Tools) and merges their outputs into a unified decision.

### Making a Decision

```rust
let coordinator = DecisionCoordinator::new();
let context = ExecutiveContext::new(ExecutionMode::Autonomous);

let request = DecisionRequest {
    id: "arch-choice".into(),
    description: "choose architecture".into(),
    options: vec![
        DecisionOption { id: "a".into(), description: "monolith".into(),
            estimated_cost: 2.0, estimated_benefit: 6.0, risk_level: 0.2 },
        DecisionOption { id: "b".into(), description: "micro".into(),
            estimated_cost: 5.0, estimated_benefit: 9.0, risk_level: 0.5 },
    ],
    context: HashMap::new(),
    constraints: vec!["budget < 10".into()],
};

let result = coordinator.make_decision(&request, &context).await?;
// result.selected_option, result.confidence, result.sources
```

### Individual Subsystem Invocation

Each subsystem can be invoked separately:

```rust
let reasoning = coordinator.invoke_reasoning(&request, &context).await?;
let memory = coordinator.invoke_memory(&request, &context).await?;
let knowledge = coordinator.invoke_knowledge(&request, &context).await?;
let inference = coordinator.invoke_inference(&request, &context).await?;
```

### Merging Results

```rust
let merged = coordinator.merge_results(
    Some(reasoning),
    Some(memory),
    Some(knowledge),
    Some(inference),
    tool_results,
);
// merged.confidence = average of all subsystem confidences
// merged.merged_output = combined JSON object
```

### Decision Sources

```rust
enum DecisionSource {
    Reasoning,
    Memory,
    Knowledge,
    Inference,
    Tool(String),
}
```

### Tool Integration

Register tools and invoke them during decision making:

```rust
coordinator.register_tool("shell".into(), "run commands".into());
let result = coordinator.invoke_tool("shell", &input, &context).await?;
```

---

## 8. Resource Coordination

The `ResourceCoordinator` manages hardware and logical resource pools, model allocation, and inference budget.

### Resource Types

| Type | Default Pool |
|---|---|
| `Cpu` | 8 units |
| `Gpu` | 4 units |
| `Ram` | 32,768 MB |
| `Disk` | 1,024,000 MB |
| `NetworkBandwidth` | 1,000 units |
| `ModelSlot` | 4 slots |
| `InferenceBudget` | 1,000,000 tokens |

### Allocation and Release

```rust
let rc = ResourceCoordinator::new();
let alloc = rc.allocate(ResourceType::Cpu, 2, "worker-1".into())?;
// available(Cpu) is now 6

rc.release(&alloc)?;
// available(Cpu) is back to 8
```

### Exhaustion Handling

```rust
let result = rc.allocate(ResourceType::Cpu, 100, "greedy".into());
// Err(ResourceExhausted)
```

### Model Allocation

Model allocation atomically reserves GPU, RAM, and a model slot:

```rust
let model = rc.allocate_model(
    "llama-7b".into(),
    1,      // GPU count
    4096,   // RAM MB
    "inference".into(),
)?;
// If any resource is insufficient, all partial allocations are rolled back
```

### Inference Budget

```rust
rc.consume_inference_budget(500)?;
let budget = rc.inference_budget();
// budget.consumed_tokens = 500
// budget.remaining() = 999500
```

Budget periods: `PerSecond`, `PerMinute`, `PerHour`, `PerDay`, `Unlimited`.

### Resource Satisfaction Check

```rust
let mut reqs = HashMap::new();
reqs.insert(ResourceType::Cpu, 4);
reqs.insert(ResourceType::Ram, 2048);
rc.can_satisfy(&reqs); // true if all resources available
```

### Pool Status

```rust
let statuses = rc.pool_statuses(); // Vec<ResourcePoolStatus>
// Each: { resource_type, total, available, allocated, utilization }
```

---

## 9. Execution Policies

The `PolicyEngine` enforces execution policies based on the current mode. Policies control permissions, concurrency limits, and resource budgets.

### Modes

| Mode | Permissions | Concurrency | Autonomous Actions |
|---|---|---|---|
| `Safe` | Memory, Reasoning, Network only | 1 goal / 2 tasks | No |
| `Interactive` | + Code, Files, GPU, Inference, Tools | 4 / 16 | No |
| `Autonomous` | + OverridePriority, Hardware | 16 / 64 | Yes |
| `Developer` | + BypassSafetyChecks | 32 / 128 | Yes |

### Permission Enforcement

```rust
let engine = PolicyEngine::new(ExecutionMode::Safe);
engine.enforce_permission(&Permission::AccessMemory)?; // Ok
engine.enforce_permission(&Permission::ExecuteCode)?;  // Err(PolicyViolation)
```

### Mode Switching

```rust
engine.switch_mode(ExecutionMode::Autonomous);
// All policy settings update immediately
```

### Confirmation Requirements

Policies define a risk threshold above which user confirmation is required:

```rust
engine.requires_confirmation(0.3); // false (below 0.5 threshold in Interactive)
engine.requires_confirmation(0.6); // true (above threshold)
```

### Violations

All denied permissions are recorded:

```rust
let violations = engine.violations(); // Vec<PolicyViolation>
// Each: { timestamp, permission, description, blocked }
```

---

## 10. Failure Recovery

The `FailureRecovery` system handles task failures through checkpointing, retry management, fallback strategies, and graceful degradation.

### Checkpointing

```rust
let recovery = FailureRecovery::new();
let cp = recovery.create_checkpoint(
    task_id,
    serde_json::json!({"step": 3, "data": "partial"}),
    3,
    "after processing".into(),
);

// Resume from latest checkpoint
let resumed = recovery.resume_from_checkpoint(task_id)?.unwrap();
assert_eq!(resumed.step_index, 3);
```

### Fallback Strategies

| Strategy | Description |
|---|---|
| `Retry` | Attempt again (default) |
| `Skip` | Skip the failed step |
| `UseAlternative` | Execute alternative path |
| `DegradeGracefully` | Continue with reduced capability |
| `FailFast` | Abort immediately |

### Strategy Determination

```rust
let strategy = recovery.determine_strategy(task_id, "timeout");
// Returns Retry if retries remain, otherwise degradation-aware strategy
```

Custom fallback configurations can be registered:

```rust
recovery.register_fallback("timeout".into(), FallbackConfig {
    strategy: FallbackStrategy::DegradeGracefully,
    max_retries: 1,
    retry_delay_ms: 500,
    alternative_description: None,
});
```

### Degradation Levels

| Level | Meaning |
|---|---|
| `None` | All systems operational |
| `Minor` | < 5% tasks degraded |
| `Moderate` | 5-15% tasks degraded |
| `Severe` | 15-35% tasks degraded |
| `Critical` | > 35% tasks degraded |

Degradation level is automatically adjusted based on the ratio of degraded tasks.

### Recovery Tracking

```rust
recovery.record_recovery_attempt(task_id, FallbackStrategy::Retry, true, None, 50);
assert_eq!(recovery.total_recovery_attempts(), 1);
assert_eq!(recovery.successful_recoveries(), 1);
```

---

## 11. Analytics

`ExecutiveAnalytics` provides comprehensive monitoring and metrics collection.

### Task Latency Tracking

```rust
let analytics = ExecutiveAnalytics::new();
analytics.record_task_latency("inference", 150.0);
analytics.record_task_latency("reasoning", 200.0);

let stats = analytics.task_latency_stats();
// LatencyStats { avg_ms, min_ms, max_ms, p50_ms, p95_ms, p99_ms, count }
```

### Decision Quality

```rust
analytics.record_decision_quality(0.85);
analytics.record_decision_quality(0.92);
let avg = analytics.decision_quality_average(); // 0.885
```

### System Snapshot

```rust
let snap = analytics.snapshot(&global_state, &scheduler_stats);
// AnalyticsSnapshot {
//     uptime_ms, task_latency_avg_ms, task_latency_p95_ms, task_latency_p99_ms,
//     total_goals_completed, goal_completion_rate,
//     total_tasks_completed, task_success_rate,
//     decision_quality_avg, scheduler_efficiency,
//     resource_utilization, active_goals, active_tasks,
// }
```

### Data Management

```rust
analytics.record_scheduler_snapshot(scheduler_stats);
analytics.record_system_snapshot(global_state);
analytics.record_resource_utilization(utilization_map);
analytics.clear(); // reset all data
```

---

## 12. API Reference

### ExecutiveApi

The `ExecutiveApi` provides the high-level interface combining all subsystems:

```rust
let api = ExecutiveApi::new(ExecutionMode::Autonomous);
```

**Goal Operations:**

| Method | Description |
|---|---|
| `create_goal(description, priority)` | Create and register a goal |
| `pause_goal(goal_id)` | Pause an executing goal |
| `resume_goal(goal_id)` | Resume a paused goal |
| `complete_goal(goal_id)` | Complete a goal, record analytics |
| `cancel_goal(goal_id)` | Cancel a goal, record cancellation |

**Task Operations:**

| Method | Description |
|---|---|
| `submit_task(name, priority, goal_id)` | Create, configure, and submit a task |
| `complete_task(task_id, result)` | Complete a task with result |
| `cancel_task(task_id)` | Cancel a task |

**Session Operations:**

| Method | Description |
|---|---|
| `create_session()` | Create a new executive session |

**Inspection and Export:**

| Method | Description |
|---|---|
| `inspect_execution()` | Returns `ExecutionSummary` with counts |
| `export_execution_summary()` | Returns full JSON summary |

**Subsystem Access:**

| Method | Returns |
|---|---|
| `goal_manager()` | `&GoalManager` |
| `task_manager()` | `&TaskManager` |
| `session_manager()` | `&SessionManager` |
| `context()` | `&ExecutiveContext` |
| `scheduler()` | `&ExecutiveScheduler` |
| `analytics()` | `&ExecutiveAnalytics` |
| `recovery()` | `&FailureRecovery` |
| `policy_engine()` | `&PolicyEngine` |

### ExecutiveContext

Manages global state, environment, tools, and capacity limits:

```rust
let ctx = ExecutiveContext::new(ExecutionMode::Autonomous);
ctx.record_goal_completed();
ctx.record_inference_call();
ctx.register_tool("shell".into());
ctx.set_variable("env".into(), json!("production"));
let state = ctx.global_state();
```

### SessionManager

Groups related goals and tasks:

```rust
let sm = SessionManager::new();
let session = sm.create_session();
// Session lifecycle: Created → Active → Paused/Completed/Failed/Cancelled
```

### Error Handling

All operations return `ExecutiveResult<T>` (alias for `Result<T, ExecutiveError>`):

```rust
use neo_executive::{ExecutiveError, ExecutiveErrorCode};

match result {
    Ok(value) => { /* use value */ }
    Err(e) => {
        match e.code() {
            ExecutiveErrorCode::GoalNotFound => { /* handle */ }
            ExecutiveErrorCode::ResourceExhausted => { /* handle */ }
            _ => { /* generic */ }
        }
    }
}
```
