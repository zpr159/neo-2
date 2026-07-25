# `neo-executive` — Executive Orchestrator

> Core orchestration engine for the Neo system. Manages goals, tasks, scheduling,
> decision coordination, resource allocation, and failure recovery across the
> entire agent lifecycle.

---

## Table of Contents

1. [Executive Architecture](#1-executive-architecture)
2. [Goal Lifecycle](#2-goal-lifecycle)
3. [Task Lifecycle](#3-task-lifecycle)
4. [Scheduler](#4-scheduler)
5. [Priority Engine](#5-priority-engine)
6. [Attention Manager](#6-attention-manager)
7. [Decision Coordination](#7-decision-coordination)
8. [Resource Coordination](#8-resource-coordination)
9. [Execution Policies](#9-execution-policies)
10. [Failure Recovery](#10-failure-recovery)
11. [Analytics](#11-analytics)
12. [API Reference](#12-api-reference)

---

## 1. Executive Architecture

### High-Level Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                          ExecutiveApi                                │
│   pub fn run(exec, ctx, goal) → ExecutionSummary                    │
└────────────────────────────┬────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│                       Goal Lifecycle                                │
│  ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐        │
│  │ Proposed │──▶│ Accepted │──▶│ Planning │──▶│Executing │        │
│  └──────────┘   └──────────┘   └──────────┘   └────┬─────┘        │
│                                                      │              │
│                        ┌──────────┐   ┌──────────┐  │              │
│                        │Completed │◀──┤ Paused   │◀─┘              │
│                        └──────────┘   └──────────┘                  │
│                         │ Failed │    │ Cancelled │                  │
│                         └────────┘    └───────────┘                 │
└────────────────────────────┬────────────────────────────────────────┘
                             │ decomposes into
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        Task Lifecycle                                │
│  Created → Queued → Running → Completed / Failed / Paused / Cancel  │
└──────────┬──────────────────┬───────────────────────────┬───────────┘
           │                  │                           │
           ▼                  ▼                           ▼
┌──────────────────┐ ┌─────────────────┐  ┌──────────────────────────┐
│    Scheduler     │ │ Priority Engine │  │   Attention Manager       │
│                  │ │                 │  │                            │
│ • priority queue │ │ • urgency       │  │ • focus selection          │
│ • parallel exec  │ │ • importance    │  │ • context switching        │
│ • preemption     │ │ • resource cost │  │ • interrupt handling       │
│ • dep tracking   │ │ • age decay     │  │ • budget enforcement       │
└───────┬──────────┘ └────────┬────────┘  └─────────────┬─────────────┘
        │                     │                         │
        └──────────┬──────────┘                         │
                   ▼                                    │
┌─────────────────────────────────────────────────────────────────────┐
│                  Decision Coordination                               │
│  reasoning ◄──► memory ◄──► knowledge ◄──► inference                │
│       │              │              │              │                 │
│       └────── tool invocation ◄─────┘              │                 │
│                       │                             │                 │
│                       ▼                             ▼                 │
│              ┌─────────────┐            ┌────────────────┐          │
│              │  tool call   │            │  result merge   │          │
│              └─────────────┘            └────────────────┘          │
└────────────────────────────┬────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│                   Resource Coordination                              │
│  ┌──────┐ ┌──────┐ ┌─────┐ ┌──────┐ ┌─────────┐ ┌──────────────┐  │
│  │ CPU  │ │ GPU  │ │ RAM │ │ Disk │ │ Network │ │ ModelSlot    │  │
│  │ Pool │ │ Pool │ │Pool │ │ Pool │ │  Pool   │ │ InferenceBud │  │
│  └──────┘ └──────┘ └─────┘ └──────┘ └─────────┘ └──────────────┘  │
└────────────────────────────┬────────────────────────────────────────┘
                             │
              ┌──────────────┴──────────────┐
              ▼                             ▼
┌──────────────────────┐    ┌───────────────────────────────────────┐
│  Execution Policies  │    │         Failure Recovery               │
│                      │    │                                       │
│  Safe / Interactive  │    │  checkpoints · resume · retry tracking │
│  Autonomous / Dev    │    │  fallback strategies · degradation     │
│  permissions · audit │    │  levels · rollback                     │
└──────────────────────┘    └───────────────────────────────────────┘
              │                             │
              └──────────────┬──────────────┘
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         Analytics                                   │
│  latency stats · decision quality · goal completions               │
│  scheduler snapshots · resource utilization · error rates           │
└─────────────────────────────────────────────────────────────────────┘
```

### Module Map

| Module | Responsibility |
|---|---|
| `error` | Unified error types (`ExecutiveError`, `RecoveryAction`) |
| `goal` | Goal creation, state machine, decomposition, persistence |
| `task` | Task creation, ownership, retry policy, dependency graph |
| `session` | Execution session lifecycle, context snapshots |
| `context` | Runtime context: user info, environment, history window |
| `priority` | Composite priority scoring (urgency, importance, resource, age) |
| `attention` | Focus selection, context-switch budgeting, interrupt policy |
| `scheduler` | Priority queue, parallel slot management, preemption |
| `decision_coordination` | Reasoning/memory/knowledge/inference integration |
| `resource_coordination` | Pool-based resource allocation and release |
| `policies` | Execution mode definitions, permission enforcement |
| `recovery` | Checkpoints, retry tracking, fallback strategies, degradation |
| `analytics` | Latency stats, quality metrics, scheduler snapshots |
| `api` | Top-level `ExecutiveApi` and `ExecutionSummary` |

### Data Flow

1. A **goal** enters via `ExecutiveApi::submit_goal`.
2. The goal is decomposed into **tasks** by the planner.
3. The **priority engine** scores each task.
4. The **scheduler** enqueues tasks and dispatches to available **resource slots**.
5. **Decision coordination** resolves reasoning at each task step.
6. **Attention management** decides which task owns the cognitive focus.
7. **Recovery** checkpoints progress and handles failures.
8. **Analytics** records the entire execution trace.

---

## 2. Goal Lifecycle

### States

```
                    ┌──────────┐
                    │ Proposed │
                    └────┬─────┘
                         │ accept()
                         ▼
                    ┌──────────┐
             ┌─────│ Accepted │─────┐
             │     └──────────┘     │
             │ plan()          cancel()
             ▼                      ▼
        ┌──────────┐          ┌───────────┐
        │ Planning │          │ Cancelled │
        └────┬─────┘          └───────────┘
             │ execute()
             ▼
       ┌────────────┐
  ┌───▶│ Executing  │◀───┐
  │    └──┬─────┬───┘    │
  │       │     │        │
  │  pause()  complete() │
  │       │     │        │
  │       ▼     ▼        │
  │  ┌────────┐ ┌───────────┐
  │  │ Paused │ │ Completed │
  │  └───┬────┘ └───────────┘
  │      │ resume()
  └──────┘
             │ fail()
             ▼
        ┌──────────┐
        │  Failed  │
        └──────────┘
```

### Goal Structure

```rust
use neo_executive::goal::{Goal, GoalState, GoalId};

let goal = Goal::new(
    "Research and summarize the latest Rust async patterns",
)
// Attach optional parent for hierarchical decomposition
.with_parent(parent_goal_id)
// Set a deadline (ISO 8601)
.with_deadline("2026-08-01T00:00:00Z")
// Assign metadata for policy routing
.with_metadata("team", "research")
.with_metadata("max_budget_tokens", "50000");

assert_eq!(goal.state(), GoalState::Proposed);
```

### State Transitions

| From | To | Trigger | Guard |
|---|---|---|---|
| `Proposed` | `Accepted` | `accept()` | Policy allows goal type |
| `Accepted` | `Planning` | `plan()` | Resources available |
| `Planning` | `Executing` | `execute()` | Tasks decomposed |
| `Executing` | `Paused` | `pause()` | No checkpoint lock held |
| `Paused` | `Executing` | `resume()` | Resources still available |
| `Executing` | `Completed` | `complete()` | All terminal tasks done |
| `Executing` | `Failed` | `fail()` | Unrecoverable error |
| Any non-terminal | `Cancelled` | `cancel()` | Caller has permission |

### Decomposition

Goals decompose into sub-goals and ultimately into tasks:

```rust
use neo_executive::goal::{Goal, Decomposition};

let decomposition = Goal::decompose(parent_goal, &[
    Decomposition::SubGoal("Gather raw data".into()),
    Decomposition::SubGoal("Analyze findings".into()),
    Decomposition::Task("Write summary document".into()),
]);

// Sub-goals inherit parent metadata; tasks inherit priority floor
assert_eq!(decomposition.sub_goals.len(), 2);
assert_eq!(decomposition.tasks.len(), 1);
```

### Dependencies

Goals can depend on other goals:

```rust
use neo_executive::goal::Goal;

let b = Goal::new("Process results")
    .depends_on(goal_a.id())
    .depends_on(goal_c.id());

// Goal B cannot enter Planning until A and C are Completed
```

### Persistence

Goals survive process restarts via serializable state snapshots:

```rust
use neo_executive::goal::Goal;
use serde_json;

let snapshot = serde_json::to_string(&goal)?;
let restored: Goal = serde_json::from_str(&snapshot)?;
assert_eq!(restored.state(), goal.state());
```

---

## 3. Task Lifecycle

### States

```
 ┌─────────┐    enqueue()    ┌─────────┐   dispatch()   ┌─────────┐
 │ Created │───────────────▶│  Queued │───────────────▶│ Running │
 └─────────┘                └─────────┘                └────┬────┘
                                                            │
                              ┌──────────┬──────────┬───────┴───────┐
                              ▼          ▼          ▼               ▼
                         ┌─────────┐ ┌────────┐ ┌─────────┐  ┌──────────┐
                         │Completed│ │ Failed │ │ Paused  │  │ Cancelled│
                         └─────────┘ └────────┘ └────┬────┘  └──────────┘
                                                     │
                                                     ▼ retry()
                                                 ┌─────────┐
                                                 │  Queued │
                                                 └─────────┘
```

### Task Structure

```rust
use neo_executive::task::{Task, TaskState, TaskId, RetryPolicy};

let task = Task::new(
    "Extract key metrics from report",
)
// Assign ownership to a specific agent or worker
.with_owner("agent::analyst-01")
// Configure retry behaviour
.with_retry_policy(RetryPolicy {
    max_retries: 3,
    backoff: Backoff::Exponential {
        base_ms: 100,
        max_ms: 10_000,
        factor: 2.0,
    },
    retry_on: vec![TaskErrorKind::Timeout, TaskErrorKind::TransientNetwork],
})
// Declare dependencies
.depends_on(TaskId::from("extract-raw-data"))
// Set a timeout
.with_timeout(Duration::from_secs(300))
// Set a resource hint for scheduling
.with_resource_hint(ResourceHint::GpuSlots(1))
// Set priority (overrides inherited goal priority if higher)
.with_priority(Priority::High);

assert_eq!(task.state(), TaskState::Created);
```

### Retry Policy

```rust
use neo_executive::task::RetryPolicy;

let aggressive = RetryPolicy {
    max_retries: 5,
    backoff: Backoff::Exponential {
        base_ms: 50,
        max_ms: 30_000,
        factor: 2.5,
    },
    retry_on: vec![
        TaskErrorKind::Timeout,
        TaskErrorKind::TransientNetwork,
        TaskErrorKind::RateLimited,
        TaskErrorKind::OutOfMemory,
    ],
};

let conservative = RetryPolicy {
    max_retries: 1,
    backoff: Backoff::Fixed { delay_ms: 5000 },
    retry_on: vec![TaskErrorKind::TransientNetwork],
};

// Backoff strategies
enum Backoff {
    Fixed { delay_ms: u64 },
    Exponential { base_ms: u64, max_ms: u64, factor: f64 },
    Linear { base_ms: u64, increment_ms: u64, max_ms: u64 },
    Custom(fn(u32) -> Duration),
}
```

### Ownership

Each task can be claimed by exactly one worker. Ownership transfers are atomic:

```rust
use neo_executive::task::{Task, Ownership};

let task = Task::new("Summarize findings");

// Initial assignment
task.claim("worker-alpha")?;
assert_eq!(task.owner(), Some("worker-alpha"));

// Transfer to another worker (e.g., during load balancing)
task.transfer("worker-beta", TransferReason::LoadBalance)?;
assert_eq!(task.owner(), Some("worker-beta"));

// Ownership can also be contested for fault tolerance
task.contest("worker-gamma", ContestReason::WorkerUnreachable)?;
```

### Dependencies

```rust
use neo_executive::task::Task;

let write_report = Task::new("Write report")
    .depends_on(TaskId::from("gather-data"))
    .depends_on(TaskId::from("analyze-data"));

// The scheduler will not dispatch write_report until both
// gather-data and analyze-data reach TaskState::Completed.
```

### Priority

Tasks carry a composite priority computed by the Priority Engine (§5). Higher-priority tasks preempt lower-priority ones in the scheduler.

```rust
use neo_executive::priority::Priority;

let priority = Priority::compute(&PriorityInput {
    urgency: 0.8,
    importance: 0.9,
    resource_cost: 0.2,
    age_normalized: 0.3,
    weights: &PriorityWeights::default(),
});

// priority.score() returns a value in [0.0, 1.0]
assert!(priority.score() > 0.7);
```

---

## 4. Scheduler

### Design

The scheduler maintains a min-heap priority queue and dispatches tasks to available resource slots. It supports:

- **Priority-ordered dispatch** — highest-priority task runs first.
- **Parallel execution** — multiple tasks run concurrently up to slot limits.
- **Preemption** — a newly enqueued higher-priority task can pause a running lower-priority task.
- **Dependency tracking** — tasks with unmet dependencies remain queued.
- **Resource-aware scheduling** — tasks declare resource needs; the scheduler only dispatches when resources are available.

### Core Loop

```
┌────────────────────────────────────────────────────────────┐
│                    Scheduler Main Loop                      │
│                                                            │
│  loop {                                                    │
│      1. Promote tasks whose dependencies are satisfied     │
│      2. Check for preemption candidates                    │
│      3. If free slots > 0:                                 │
│         a. Pop highest-priority ready task                 │
│         b. Verify resource availability                    │
│         c. Dispatch (transition Queued → Running)          │
│      4. If preemption needed:                              │
│         a. Select lowest-priority running task             │
│         b. Pause it (transition Running → Paused)          │
│         c. Release its resources                           │
│         d. Dispatch the new high-priority task             │
│      5. Collect completed/failed results                   │
│      6. Update analytics snapshot                          │
│  }                                                         │
└────────────────────────────────────────────────────────────┘
```

### Usage

```rust
use neo_executive::scheduler::{Scheduler, SchedulerConfig};
use std::time::Duration;

let scheduler = Scheduler::new(SchedulerConfig {
    max_parallel_tasks: 8,
    preemption_enabled: true,
    tick_interval: Duration::from_millis(100),
    resource_check_interval: Duration::from_millis(250),
});

// Enqueue tasks
scheduler.enqueue(task_a);
scheduler.enqueue(task_b);
scheduler.enqueue(task_c);

// Run the scheduler (blocking)
let results = scheduler.run_until_all_complete()?;

// Or run a single tick for integration with async runtimes
scheduler.tick()?;
```

### Preemption

```rust
use neo_executive::scheduler::{Scheduler, PreemptionPolicy};

let scheduler = Scheduler::new(SchedulerConfig {
    preemption_enabled: true,
    ..Default::default()
});

// Configure when preemption is allowed
scheduler.set_preemption_policy(PreemptionPolicy {
    // Only preempt if the new task's priority exceeds the running
    // task's priority by at least this margin
    priority_margin: 0.2,
    // Don't preempt tasks that have been running for less than this
    min_run_time: Duration::from_secs(5),
    // Maximum number of preemptions per task lifetime
    max_preemptions: 3,
    // Never preempt tasks with these tags
    protected_tags: vec!["critical".into(), "checkpoint-locked".into()],
});
```

### Dependency Tracking

```rust
use neo_executive::scheduler::Scheduler;
use neo_executive::task::{Task, TaskId};

let mut scheduler = Scheduler::default();

let t1 = Task::new("Step 1");
let t2 = Task::new("Step 2").depends_on(t1.id());
let t3 = Task::new("Step 3").depends_on(t2.id());

scheduler.enqueue(t1);
scheduler.enqueue(t2);
scheduler.enqueue(t3);

// t2 and t3 remain queued until their dependencies complete.
// The scheduler tracks the full dependency DAG and promotes
// tasks automatically when predecessors finish.
```

### Resource-Aware Scheduling

```rust
use neo_executive::scheduler::{Scheduler, SlotPool};
use neo_executive::resource::{ResourcePool, ResourceType};

let mut scheduler = Scheduler::default();

// Register resource pools
scheduler.register_pool(ResourcePool::new(ResourceType::GpuSlots, 4));
scheduler.register_pool(ResourcePool::new(ResourceType::InferenceBudget, 1000));

// A task requesting 2 GPU slots will only be dispatched
// when at least 2 slots are free
let gpu_task = Task::new("Run inference batch")
    .with_resource_hint(ResourceHint::GpuSlots(2))
    .with_resource_hint(ResourceHint::InferenceBudget(500));

scheduler.enqueue(gpu_task);
// Scheduler checks: GPU free = 4 >= 2 ✓, InferenceBudget free = 1000 >= 500 ✓
```

---

## 5. Priority Engine

### Composite Scoring

Priority is computed from four signals:

| Signal | Range | Description |
|---|---|---|
| **Urgency** | `[0.0, 1.0]` | Time-sensitivity based on deadline proximity |
| **Importance** | `[0.0, 1.0]` | Business/user-defined importance weight |
| **Resource Cost** | `[0.0, 1.0]` | Inverse of resource cost (cheaper tasks ranked higher for fairness) |
| **Age** | `[0.0, 1.0]` | Normalized time since creation (prevents starvation) |

### Formula

```
score = w_u × urgency + w_i × importance + w_r × (1 - resource_cost) + w_a × age
```

Default weights: `w_u = 0.35`, `w_i = 0.40`, `w_r = 0.10`, `w_a = 0.15`.

### Usage

```rust
use neo_executive::priority::{PriorityEngine, PriorityInput, PriorityWeights};

let engine = PriorityEngine::new(PriorityWeights {
    urgency: 0.35,
    importance: 0.40,
    resource_cost: 0.10,
    age: 0.15,
});

let priority = engine.score(&PriorityInput {
    urgency: 0.9,        // deadline is very soon
    importance: 0.7,     // moderately important
    resource_cost: 0.1,  // cheap to run
    age: 0.5,            // has been waiting for a while
});

assert!(priority.score() > 0.6);
assert_eq!(priority.rank(), PriorityRank::High);
```

### Conflict Resolution

When two tasks have identical composite scores:

```rust
use neo_executive::priority::{ConflictResolver, ConflictStrategy};

let resolver = ConflictResolver::new(ConflictStrategy::Fifo);
// Other strategies:
// ConflictStrategy::Lifo         — most recently enqueued wins
// ConflictStrategy::Random       — random tiebreak
// ConflictStrategy::LowestCost   — prefer cheaper tasks
// ConflictStrategy::Custom(fn)   — user-provided comparator

let winner = resolver.resolve(&task_a, &task_b);
```

### Priority Rules

Rules allow overriding automatic scoring:

```rust
use neo_executive::priority::{PriorityRule, RuleCondition};

engine.add_rule(PriorityRule {
    name: "critical-user-boost".into(),
    condition: RuleCondition::MetadataEquals {
        key: "user_tier".into(),
        value: "enterprise".into(),
    },
    override_score: Some(0.95),
    boost: None,
    tag: Some("critical".into()),
});

engine.add_rule(PriorityRule {
    name: "stale-task-promotion".into(),
    condition: RuleCondition::AgeExceeds(Duration::from_secs(600)),
    override_score: None,
    boost: Some(0.2),   // add 0.2 to computed score
    tag: Some("starvation-guard".into()),
});
```

### Priority Ranks

```rust
pub enum PriorityRank {
    Critical,   // score >= 0.9
    High,       // score >= 0.7
    Normal,     // score >= 0.4
    Low,        // score >= 0.2
    Background, // score < 0.2
}
```

---

## 6. Attention Manager

The Attention Manager determines which task or goal receives the system's primary cognitive focus at any moment.

### Focus Selection

```rust
use neo_executive::attention::{AttentionManager, FocusCriteria};

let attention = AttentionManager::new(FocusCriteria {
    // Maximum concurrent focus targets
    max_focus: 1,
    // Minimum time before a context switch is allowed
    min_focus_duration: Duration::from_secs(30),
    // Priority threshold below which tasks are backgrounded
    focus_threshold: 0.6,
});

// The attention manager continuously evaluates all running tasks
// and selects the one that should hold primary focus
let focus = attention.current_focus();
// Returns the TaskId with the highest composite score among
// tasks meeting the focus threshold
```

### Context Switching

```rust
use neo_executive::attention::{AttentionManager, SwitchPolicy};

attention.set_switch_policy(SwitchPolicy {
    // Cost of switching (simulated latency penalty)
    switch_cost_ms: 500,
    // Maximum switches per minute
    max_switches_per_minute: 6,
    // Hysteresis: new task must exceed current by this margin
    switch_threshold: 0.15,
    // Don't switch away from tasks in these states
    protected_states: vec![TaskState::CheckpointLocked],
});

// Manual context switch
let result = attention.switch_to(task_b_id)?;
// Returns Err if switch is blocked by policy

// Force switch (bypasses policy, for critical interrupts)
attention.force_switch(task_c_id, ForceReason::CriticalInterrupt)?;
```

### Interrupt Handling

```rust
use neo_executive::attention::{AttentionManager, InterruptPolicy};

attention.set_interrupt_policy(InterruptPolicy {
    // Which priority levels can interrupt
    interruptible_below: PriorityRank::Normal,
    // Maximum interrupt duration before forced yield
    max_interrupt_duration: Duration::from_secs(10),
    // Enable nested interrupts
    nested_interrupts: false,
    // Always allow these interrupt sources
    always_allow: vec!["system.shutdown".into(), "resource.exhausted".into()],
});

// Raise an interrupt
attention.raise_interrupt(Interrupt {
    source: "user.cancel-request".into(),
    priority: PriorityRank::High,
    reason: "User requested cancellation".into(),
});
```

### Budget Enforcement

```rust
use neo_executive::attention::{AttentionManager, BudgetConfig};

attention.set_budget(BudgetConfig {
    // Maximum total cognitive budget per cycle (in abstract units)
    cycle_budget: 1000,
    // Budget allocation per priority rank
    allocation: HashMap::from([
        (PriorityRank::Critical, 400),
        (PriorityRank::High, 300),
        (PriorityRank::Normal, 200),
        (PriorityRank::Low, 80),
        (PriorityRank::Background, 20),
    ]),
    // Allow borrowing from lower-priority buckets
    allow_borrowing: true,
    // Maximum borrow ratio
    max_borrow_ratio: 0.5,
});

// Query remaining budget
let remaining = attention.budget_remaining();
assert!(remaining > 0);

// Enforce budget — demotes tasks that exceeded their allocation
attention.enforce_budget();
```

---

## 7. Decision Coordination

Decision Coordination integrates multiple cognitive subsystems to produce coherent decisions at each execution step.

### Architecture

```
┌────────────────────────────────────────────────────┐
│              DecisionCoordinator                    │
│                                                     │
│  ┌────────────┐  ┌────────────┐  ┌──────────────┐  │
│  │ Reasoning  │  │   Memory   │  │  Knowledge   │  │
│  │  Engine    │  │   Store    │  │    Base      │  │
│  └─────┬──────┘  └─────┬──────┘  └──────┬───────┘  │
│        │               │                │           │
│        └───────────────┼────────────────┘           │
│                        ▼                            │
│              ┌─────────────────┐                    │
│              │  Inference Hub  │                    │
│              └────────┬────────┘                    │
│                       │                             │
│          ┌────────────┼────────────┐                │
│          ▼            ▼            ▼                │
│  ┌──────────────┐ ┌────────┐ ┌──────────┐          │
│  │ Tool Invoke  │ │ Validate│ │Result    │          │
│  │   Handler    │ │  Gate  │ │ Merger   │          │
│  └──────────────┘ └────────┘ └──────────┘          │
└────────────────────────────────────────────────────┘
```

### Usage

```rust
use neo_executive::decision_coordination::{
    DecisionCoordinator, DecisionInput, DecisionOutput,
};

let coordinator = DecisionCoordinator::new(DecisionConfig {
    // Which subsystems are active
    reasoning_enabled: true,
    memory_enabled: true,
    knowledge_enabled: true,
    inference_enabled: true,
    // Maximum decision latency before fallback
    max_decision_latency: Duration::from_secs(30),
    // Minimum confidence threshold for a decision
    confidence_threshold: 0.6,
});

let decision = coordinator.decide(&DecisionInput {
    task_id: TaskId::from("analyze-data"),
    context: &execution_context,
    available_tools: &tool_registry,
    history: &recent_history,
})?;

match decision {
    DecisionOutput::Action { action, confidence, reasoning } => {
        println!("Decision: {:?} (confidence: {:.2})", action, confidence);
        println!("Reasoning: {}", reasoning);
    }
    DecisionOutput::NeedsMoreInfo { missing } => {
        println!("Cannot decide — need: {:?}", missing);
    }
    DecisionOutput::Fallback { strategy } => {
        println!("Decision subsystem degraded — using: {:?}", strategy);
    }
}
```

### Tool Invocation

```rust
use neo_executive::decision_coordination::ToolRegistry;

let mut registry = ToolRegistry::new();

registry.register(Tool {
    name: "web_search".into(),
    description: "Search the web for information".into(),
    parameters: json_schema!({
        "query": { "type": "string", "required": true }
    }),
    cost: ToolCost {
        latency_ms: 2000,
        tokens: 500,
        requires_network: true,
        requires_gpu: false,
    },
    safety: SafetyLevel::Interactive,
});

// The coordinator selects tools based on cost, safety, and relevance
let tool_call = coordinator.select_tool(&task_context, &registry)?;
```

### Result Merging

When multiple subsystems produce conflicting outputs:

```rust
use neo_executive::decision_coordination::MergeStrategy;

coordinator.set_merge_strategy(MergeStrategy::WeightedVote {
    // Weights for each subsystem's opinion
    weights: HashMap::from([
        ("reasoning".into(), 0.4),
        ("memory".into(), 0.25),
        ("knowledge".into(), 0.2),
        ("inference".into(), 0.15),
    ]),
    // Minimum total weight in agreement to accept
    min_agreement: 0.6,
    // If agreement is below this, escalate to human
    escalation_threshold: 0.4,
});
```

---

## 8. Resource Coordination

### Resource Pools

| Pool | Unit | Typical Capacity |
|---|---|---|
| `CpuCores` | cores | 8–64 |
| `GpuSlots` | slots | 0–8 |
| `RamBytes` | bytes | 8–128 GiB |
| `DiskBytes` | bytes | 100 GiB–10 TiB |
| `NetworkBandwidth` | Mbps | 100–10,000 |
| `ModelSlots` | slots | 0–4 |
| `InferenceBudget` | tokens/cycle | 1,000–100,000 |

### Pool Management

```rust
use neo_executive::resource::{
    ResourcePool, ResourceType, AllocationId,
};

let mut pool = ResourcePool::new(ResourceType::GpuSlots, 4);

// Allocate
let alloc_id: AllocationId = pool.allocate(
    TaskId::from("run-inference"),
    2,  // request 2 GPU slots
)?;

assert_eq!(pool.free(), 2);
assert_eq!(pool.used(), 2);

// Release
pool.release(alloc_id)?;
assert_eq!(pool.free(), 4);
```

### Cross-Pool Coordination

```rust
use neo_executive::resource::{ResourceManager, ResourceRequest};

let mut manager = ResourceManager::new();
manager.register(ResourcePool::new(ResourceType::CpuCores, 16));
manager.register(ResourcePool::new(ResourceType::GpuSlots, 4));
manager.register(ResourcePool::new(ResourceType::RamBytes, 32 * 1024 * 1024 * 1024));
manager.register(ResourcePool::new(ResourceType::InferenceBudget, 10_000));

// Atomic multi-pool allocation
let reservation = manager.reserve(&ResourceRequest {
    task_id: TaskId::from("llm-inference-batch"),
    requirements: vec![
        ResourceRequirement::new(ResourceType::CpuCores, 4),
        ResourceRequirement::new(ResourceType::GpuSlots, 2),
        ResourceRequirement::new(ResourceType::RamBytes, 4 * 1024 * 1024 * 1024),
        ResourceRequirement::new(ResourceType::InferenceBudget, 5000),
    ],
    // If any pool lacks capacity, release all and return error
    atomic: true,
    // How long to wait for resources to become available
    timeout: Duration::from_secs(30),
})?;

// Use resources...
// ...

// Release everything atomically
reservation.release()?;
```

### Contention Handling

```rust
use neo_executive::resource::{ContentionPolicy, WaitStrategy};

manager.set_contention_policy(ContentionPolicy {
    wait_strategy: WaitStrategy::PriorityQueue,
    // Higher-priority tasks jump the wait queue
    max_wait_time: Duration::from_secs(60),
    // If wait exceeds this, try to preempt lowest-priority holder
    preempt_after: Duration::from_secs(30),
    // Maximum number of tasks waiting per pool
    max_waiters: 32,
});
```

---

## 9. Execution Policies

### Policy Modes

| Mode | Description |
|---|---|
| `Safe` | No external actions. Read-only analysis, local computation only. |
| `Interactive` | Ask user confirmation before each external action. |
| `Autonomous` | Execute external actions within configured permission boundaries. |
| `Developer` | Full access. All actions permitted. No confirmation prompts. |

### Policy Structure

```rust
use neo_executive::policies::{ExecutionPolicy, PolicyMode, Permission};

let policy = ExecutionPolicy::new(PolicyMode::Autonomous)
    // Grant specific permissions
    .grant(Permission::NetworkAccess {
        allowed_domains: vec!["api.example.com".into()],
        rate_limit: Some(RateLimit { max_requests: 100, per: Duration::from_secs(60) }),
    })
    .grant(Permission::FileSystemAccess {
        paths: vec!["/tmp/neo-workspace/**".into()],
        modes: vec![FileMode::Read, FileMode::Write],
    })
    .grant(Permission::ToolUse {
        allowed_tools: vec!["web_search".into(), "calculator".into()],
    })
    // Deny specific actions
    .deny(Permission::ShellExecution)
    .deny(Permission::NetworkAccess {
        allowed_domains: vec!["*.internal".into()],
        rate_limit: None,
    })
    // Set resource limits
    .with_resource_limits(ResourceLimits {
        max_cpu_percent: 50.0,
        max_memory_bytes: 4 * 1024 * 1024 * 1024,
        max_inference_tokens_per_cycle: 5000,
        max_disk_write_bytes: 100 * 1024 * 1024,
    });
```

### Policy Enforcement

```rust
use neo_executive::policies::{PolicyEnforcer, EnforcementAction};

let enforcer = PolicyEnforcer::new(policy);

// Check before action
match enforcer.check(&Action::NetworkRequest {
    url: "https://api.example.com/data".into(),
    method: HttpMethod::Get,
}) {
    EnforcementAction::Allow => { /* proceed */ }
    EnforcementAction::Deny { reason } => {
        eprintln!("Blocked: {}", reason);
    }
    EnforcementAction::Confirm { reason, action } => {
        // In Interactive mode, prompt user
        let confirmed = prompt_user(&reason);
        if !confirmed {
            enforcer.deny(action);
        }
    }
    EnforcementAction::Log { reason, action } => {
        // In Autonomous mode, log and proceed
        tracing::warn!(reason, ?action, "Policy-mediated action");
    }
}
```

### Mode Transitions

```rust
use neo_executive::policies::{ExecutionPolicy, PolicyMode};

let mut policy = ExecutionPolicy::new(PolicyMode::Safe);

// Promote to Interactive (requires explicit intent)
policy.transition_to(PolicyMode::Interactive)?;

// Promote to Autonomous (requires authorization)
policy.transition_to(PolicyMode::Autonomous, AuthContext {
    user_id: "admin".into(),
    auth_token: "...",
    reason: "Automated data pipeline execution".into(),
})?;

// Demote back to Safe (always allowed)
policy.transition_to(PolicyMode::Safe)?;
```

---

## 10. Failure Recovery

### Checkpoints

```rust
use neo_executive::recovery::{Checkpoint, CheckpointManager};

let checkpoint_mgr = CheckpointManager::new(CheckpointConfig {
    // How often to auto-checkpoint
    auto_checkpoint_interval: Duration::from_secs(30),
    // Maximum checkpoints to retain per goal
    max_checkpoints_per_goal: 10,
    // Storage backend
    storage: CheckpointStorage::FileSystem {
        path: "/var/neo/checkpoints".into(),
        compression: Compression::Zstd,
    },
});

// Manual checkpoint
let checkpoint: Checkpoint = checkpoint_mgr.save(&CheckpointInput {
    goal_id: goal.id(),
    task_states: &task_registry.snapshot(),
    resource_allocations: &resource_manager.snapshot(),
    scheduler_state: &scheduler.snapshot(),
    metadata: HashMap::from([
        ("step".into(), "post-analysis".into()),
    ]),
})?;

println!("Checkpoint saved: {}", checkpoint.id());
```

### Resume from Checkpoint

```rust
use neo_executive::recovery::RecoveryManager;

let recovery = RecoveryManager::new(&checkpoint_mgr);

// Find the latest checkpoint for a goal
let checkpoint = recovery.latest_checkpoint(goal.id())?;

// Resume execution from that point
let resume_plan = recovery.plan_resume(&checkpoint)?;

// Resume plan includes:
// - Tasks to re-run (those that were in-progress)
// - Tasks to skip (those that completed before checkpoint)
// - Resources to re-acquire
// - State to reconstruct
for step in &resume_plan.steps {
    match step {
        ResumeStep::ReRunTask(task_id) => { /* re-enqueue */ }
        ResumeStep::SkipTask(task_id) => { /* mark complete */ }
        ResumeStep::ReacquireResource(res) => { /* allocate */ }
        ResumeStep::RestoreState(key, value) => { /* inject */ }
    }
}
```

### Retry Tracking

```rust
use neo_executive::recovery::{RetryTracker, RetryRecord};

let tracker = RetryTracker::new();

// Record a retry attempt
tracker.record(&RetryRecord {
    task_id: TaskId::from("fetch-data"),
    attempt: 2,
    max_attempts: 5,
    error: TaskError::Timeout(Duration::from_secs(30)),
    backoff_next: Duration::from_secs(400),
    timestamp: Utc::now(),
});

// Query retry history for a task
let history = tracker.history_for(&TaskId::from("fetch-data"));
assert_eq!(history.len(), 2);

// Check if task should be escalated (too many retries)
if tracker.should_escalate(&TaskId::from("fetch-data")) {
    // Mark as failed permanently, notify user, try fallback
}
```

### Fallback Strategies

```rust
use neo_executive::recovery::{FallbackStrategy, FallbackChain};

let chain = FallbackChain::new(vec![
    FallbackStrategy::Retry {
        max_retries: 3,
        backoff: Backoff::Exponential { base_ms: 100, max_ms: 5000, factor: 2.0 },
    },
    FallbackStrategy::UseAlternateTool {
        primary: "premium-api".into(),
        fallback: "free-api".into(),
    },
    FallbackStrategy::Degrade {
        level: DegradationLevel::ReducedQuality,
        description: "Switch to lower-quality but faster model".into(),
    },
    FallbackStrategy::SkipWithPlaceholder {
        placeholder: serde_json::json!({ "status": "skipped", "reason": "unavailable" }),
    },
    FallbackStrategy::EscalateToUser {
        message: "All automated recovery attempts failed. Manual intervention required.".into(),
    },
]);

// Execute fallback chain for a failed task
let result = chain.execute(&TaskId::from("fetch-data"), &initial_error)?;
```

### Degradation Levels

```rust
pub enum DegradationLevel {
    /// No degradation — full functionality
    None,
    /// Reduced quality but same latency (e.g., smaller model)
    ReducedQuality,
    /// Increased latency but same quality (e.g., batch instead of streaming)
    IncreasedLatency,
    /// Subset of features (e.g., disable non-essential analyses)
    FeatureReduction,
    /// Minimal functionality — core operations only
    Minimal,
    /// System is effectively non-functional
    Critical,
}
```

---

## 11. Analytics

### Latency Stats

```rust
use neo_executive::analytics::{Analytics, LatencyStats};

let analytics = Analytics::new();

// Record latency measurements
analytics.record_latency("task_execution", Duration::from_millis(450));
analytics.record_latency("task_execution", Duration::from_millis(320));
analytics.record_latency("task_execution", Duration::from_millis(510));

// Query aggregated stats
let stats: LatencyStats = analytics.latency_stats("task_execution");
// stats.mean, stats.p50, stats.p95, stats.p99, stats.min, stats.max
assert!(stats.p95 < Duration::from_secs(1));
```

### Decision Quality

```rust
use neo_executive::analytics::DecisionQualityMetrics;

analytics.record_decision_quality(DecisionQualityRecord {
    task_id: TaskId::from("plan-route"),
    confidence: 0.85,
    was_correct: Some(true),  // confirmed by outcome
    latency: Duration::from_millis(120),
    subsystems_used: vec!["reasoning".into(), "knowledge".into()],
    timestamp: Utc::now(),
});

let quality = analytics.decision_quality_summary();
// quality.total_decisions
// quality.avg_confidence
// quality.accuracy_rate (when outcome is known)
// quality.avg_latency
```

### Goal Completions

```rust
use neo_executive::analytics::GoalCompletionMetrics;

analytics.record_goal_completion(GoalCompletionRecord {
    goal_id: goal.id(),
    outcome: GoalOutcome::Completed,
    duration: Duration::from_secs(120),
    task_count: 5,
    retry_count: 1,
    checkpoint_count: 3,
    resources_used: ResourceUsage {
        cpu_seconds: 45.0,
        gpu_seconds: 12.0,
        ram_peak_bytes: 2 * 1024 * 1024 * 1024,
        inference_tokens: 8500,
    },
    timestamp: Utc::now(),
});

let completions = analytics.goal_completions_summary(Duration::from_secs(3600));
// completions.total, completions.success_rate, completions.avg_duration
```

### Scheduler Snapshots

```rust
use neo_executive::analytics::SchedulerSnapshot;

// Take a snapshot of current scheduler state
let snapshot = scheduler.take_snapshot();
analytics.record_scheduler_snapshot(SchedulerSnapshot {
    timestamp: Utc::now(),
    queued_count: snapshot.queued_count,
    running_count: snapshot.running_count,
    paused_count: snapshot.paused_count,
    avg_queue_wait: snapshot.avg_queue_wait,
    preemptions_this_cycle: snapshot.preemptions,
    resource_utilization: snapshot.utilization,
});

// Query scheduler performance over time
let trends = analytics.scheduler_trends(Duration::from_secs(600));
// trends.avg_queue_depth, trends.throughput_per_minute,
// trends.preemption_rate, trends.utilization_trend
```

### Querying Analytics

```rust
use neo_executive::analytics::{Analytics, TimeRange};

// Time-windowed queries
let range = TimeRange {
    start: Utc::now() - Duration::from_secs(3600),
    end: Utc::now(),
};

let report = analytics.generate_report(&range);
// report.latency_summary
// report.decision_quality
// report.goal_completions
// report.scheduler_performance
// report.resource_utilization
// report.error_rate
// report.error_breakdown_by_type
```

---

## 12. API Reference

### `ExecutiveApi`

The top-level entry point for all executive operations.

```rust
use neo_executive::api::{ExecutiveApi, ExecutiveContext, ExecutionSummary};

// Create the API with default configuration
let api = ExecutiveApi::new(ExecutiveConfig::default())?;

// Or with custom configuration
let api = ExecutiveApi::new(ExecutiveConfig {
    max_concurrent_goals: 4,
    default_policy: ExecutionPolicy::new(PolicyMode::Autonomous),
    checkpoint_interval: Duration::from_secs(60),
    analytics_retention: Duration::from_secs(86400),
    resource_limits: ResourceLimits::default(),
})?;
```

#### `submit_goal`

```rust
pub async fn submit_goal(
    &self,
    ctx: &ExecutiveContext,
    goal: Goal,
) -> Result<GoalSubmission, ExecutiveError>;

// Usage
let submission = api.submit_goal(&ctx, Goal::new("Build a REST API")).await?;
println!("Goal submitted: {}", submission.goal_id);
```

#### `run`

Synchronous convenience method that submits a goal and waits for completion.

```rust
pub fn run(
    &self,
    ctx: &ExecutiveContext,
    goal: Goal,
) -> Result<ExecutionSummary, ExecutiveError>;

// Usage
let summary = api.run(&ctx, Goal::new("Analyze quarterly data"))?;
println!("Status: {:?}", summary.outcome);
println!("Duration: {:?}", summary.duration);
println!("Tasks executed: {}", summary.task_count);
```

#### `pause_goal`

```rust
pub async fn pause_goal(
    &self,
    goal_id: GoalId,
) -> Result<(), ExecutiveError>;

api.pause_goal(submission.goal_id).await?;
```

#### `resume_goal`

```rust
pub async fn resume_goal(
    &self,
    goal_id: GoalId,
) -> Result<(), ExecutiveError>;

api.resume_goal(submission.goal_id).await?;
```

#### `cancel_goal`

```rust
pub async fn cancel_goal(
    &self,
    goal_id: GoalId,
    reason: &str,
) -> Result<(), ExecutiveError>;

api.cancel_goal(submission.goal_id, "No longer needed").await?;
```

#### `status`

```rust
pub async fn status(
    &self,
    goal_id: GoalId,
) -> Result<GoalStatus, ExecutiveError>;

let status = api.status(submission.goal_id).await?;
// status.state, status.active_tasks, status.progress_pct
```

#### `checkpoint`

```rust
pub async fn checkpoint(
    &self,
    goal_id: GoalId,
) -> Result<Checkpoint, ExecutiveError>;

let cp = api.checkpoint(submission.goal_id).await?;
```

#### `resume_from_checkpoint`

```rust
pub async fn resume_from_checkpoint(
    &self,
    checkpoint_id: CheckpointId,
) -> Result<GoalResumption, ExecutiveError>;

let resumption = api.resume_from_checkpoint(cp.id()).await?;
```

#### `analytics_report`

```rust
pub fn analytics_report(
    &self,
    range: &TimeRange,
) -> AnalyticsReport;

let report = api.analytics_report(&TimeRange::last_hour());
```

#### `set_policy`

```rust
pub fn set_policy(
    &self,
    goal_id: GoalId,
    policy: ExecutionPolicy,
) -> Result<(), ExecutiveError>;

api.set_policy(
    submission.goal_id,
    ExecutionPolicy::new(PolicyMode::Interactive),
)?;
```

### `ExecutiveContext`

Carries per-execution metadata.

```rust
pub struct ExecutiveContext {
    /// Unique execution session identifier
    pub session_id: SessionId,
    /// User or system that initiated this execution
    pub caller: Caller,
    /// Environment variables and configuration
    pub environment: Environment,
    /// Execution history window (recent decisions, actions)
    pub history: HistoryWindow,
    /// Active policy for this execution
    pub policy: ExecutionPolicy,
    /// Custom metadata passed through to all subsystems
    pub metadata: HashMap<String, Value>,
}

// Construction
let ctx = ExecutiveContext::new(Caller::User {
    user_id: "alice".into(),
    roles: vec!["admin".into()],
})
.with_metadata("project", "quarterly-analysis")
.with_history_window(HistoryWindow::new(Duration::from_secs(300)));
```

### `ExecutionSummary`

Returned when a goal reaches a terminal state.

```rust
pub struct ExecutionSummary {
    /// The goal that was executed
    pub goal_id: GoalId,
    /// Final outcome
    pub outcome: GoalOutcome,
    /// Total wall-clock duration
    pub duration: Duration,
    /// Number of tasks created and executed
    pub task_count: u32,
    /// Number of retries across all tasks
    pub retry_count: u32,
    /// Number of checkpoints created
    pub checkpoint_count: u32,
    /// Number of preemptions that occurred
    pub preemption_count: u32,
    /// Peak resource usage during execution
    pub peak_resources: ResourceUsage,
    /// Aggregated latency statistics
    pub latency_stats: LatencyStats,
    /// Decision quality metrics
    pub decision_quality: DecisionQualityMetrics,
    /// Any recovery actions that were taken
    pub recovery_actions: Vec<RecoveryAction>,
    /// Sub-goals that were created and their outcomes
    pub sub_goal_results: Vec<SubGoalResult>,
    /// Final degradation level (None if fully successful)
    pub degradation_level: DegradationLevel,
    /// Timestamp of completion
    pub completed_at: DateTime<Utc>,
}

// Pretty-print a summary
impl fmt::Display for ExecutionSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Goal {} completed in {:?} — {} tasks, {} retries, {:?}",
            self.goal_id, self.duration, self.task_count,
            self.retry_count, self.outcome
        )
    }
}
```

### Error Types

```rust
pub enum ExecutiveError {
    GoalNotFound(GoalId),
    TaskNotFound(TaskId),
    InvalidStateTransition { from: GoalState, to: GoalState, goal_id: GoalId },
    PolicyViolation { action: Action, reason: String },
    ResourceExhausted { resource: ResourceType, requested: u64, available: u64 },
    CheckpointError(CheckpointError),
    RetryExhausted { task_id: TaskId, attempts: u32 },
    DependencyCycleDetected(Vec<TaskId>),
    PreemptionFailed { task_id: TaskId, reason: String },
    Timeout { operation: String, duration: Duration },
    SerializationError(String),
    ChannelClosed(String),
    Internal(String),
}

impl ExecutiveError {
    /// Determine the appropriate recovery action for this error
    pub fn recovery_action(&self) -> Option<RecoveryAction> {
        match self {
            Self::ResourceExhausted { .. } => Some(RecoveryAction::RetryAfterDelay(Duration::from_secs(5))),
            Self::Timeout { .. } => Some(RecoveryAction::RetryWithBackoff),
            Self::RetryExhausted { .. } => Some(RecoveryAction::EscalateToUser),
            Self::PolicyViolation { .. } => None, // cannot recover
            Self::DependencyCycleDetected(_) => Some(RecoveryAction::LogAndSkip),
            _ => Some(RecoveryAction::CheckpointAndRetry),
        }
    }
}
```

---

## Quick Start

```rust
use neo_executive::api::{ExecutiveApi, ExecutiveContext, ExecutionSummary};
use neo_executive::goal::Goal;
use neo_executive::policies::{ExecutionPolicy, PolicyMode};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api = ExecutiveApi::default()?;
    let ctx = ExecutiveContext::default();
    let goal = Goal::new("Summarize the latest AI research papers");

    let summary: ExecutionSummary = api.run(&ctx, goal)?;
    println!("{}", summary);

    Ok(())
}
```

---

*Generated for `neo-executive` — module versions as of the current workspace.*
