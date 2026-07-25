//! # Neo Executive System
//!
//! The Executive System coordinates all cognitive processes in the Neo AGI OS.
//! It is responsible for goal management, prioritization, scheduling,
//! task orchestration, and system-wide decision making.
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────┐
//! │                     Executive Manager                             │
//! │  (lifecycle, session, global state, mode)                        │
//! ├──────────┬──────────┬──────────┬──────────┬─────────────────────┤
//! │   Goal   │   Task   │ Priority │ Attention│     Scheduler       │
//! │  Manager │  Manager │  Engine  │  Manager │  (parallel, deps,   │
//! │(hierarchy│ (queue,  │(dynamic, │ (focus,  │   resource-aware,   │
//! │ decompose│  retry,  │ urgency, │  switch, │   preemption)       │
//! │ persist) │ deadline)│  resource)│ budget) │                     │
//! ├──────────┴──────────┴──────────┴──────────┴─────────────────────┤
//! │              Decision Coordination                                │
//! │  (reasoning, memory, knowledge, inference, tools, merge)        │
//! ├─────────────────────────────────────────────────────────────────┤
//! │              Resource Coordination                                │
//! │  (CPU, GPU, RAM, model allocation, inference budget)            │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  Execution Policies │ Failure Recovery │ Analytics │ API        │
//! │  (safe, interactive,│ (retries,        │ (latency, │ (goals,    │
//! │   autonomous,       │  fallback,       │  comple-  │  tasks,    │
//! │   developer)        │  checkpoint,     │  tion,    │  inspect,  │
//! │                     │  degradation)    │  quality) │  export)   │
//! └─────────────────────┴──────────────────┴───────────┴────────────┘
//! ```

pub mod error;
pub mod goal;
pub mod task;
pub mod session;
pub mod context;
pub mod priority;
pub mod attention;
pub mod scheduler;
pub mod decision_coordination;
pub mod resource_coordination;
pub mod policies;
pub mod recovery;
pub mod analytics;
pub mod api;

pub use error::{ExecutiveError, ExecutiveResult, ExecutiveErrorCode};
pub use goal::{Goal, GoalId, GoalState, GoalPriority, GoalManager, GoalDecompositionStep, GoalPersistence};
pub use task::{
    Task, TaskId, TaskState, TaskPriority, TaskManager, RetryPolicy,
};
pub use session::{
    Session, SessionId, SessionState, SessionManager, SessionSnapshot,
};
pub use context::{
    ExecutiveContext, ExecutionMode, GlobalState,
};
pub use priority::{
    PriorityEngine, PriorityScore, ConflictResolution, PriorityRule, ResourceAvailability,
};
pub use attention::{
    AttentionManager, AttentionId, Focus, Interrupt, InterruptType, AttentionBudget,
    ContextSwitchEvent,
};
pub use scheduler::{
    ExecutiveScheduler, ScheduleId, ScheduledExecution, SchedulerStats, SchedulingPolicy,
    PreemptionEvent,
};
pub use decision_coordination::{
    DecisionCoordinator, DecisionRequest, DecisionOption, DecisionResult, DecisionSource,
    MergedResult,
};
pub use resource_coordination::{
    ResourceCoordinator, ResourceType, ResourceAllocation, ResourcePoolStatus,
    ModelAllocation, InferenceBudget, BudgetPeriod,
};
pub use policies::{
    ExecutionPolicy, PolicyEngine, Permission, PolicyViolation,
};
pub use recovery::{
    FailureRecovery, Checkpoint, CheckpointId, FallbackStrategy, FallbackConfig,
    RecoveryAttempt, TaskRecoveryState, DegradationLevel,
};
pub use analytics::{
    ExecutiveAnalytics, AnalyticsSnapshot, LatencyStats,
};
pub use api::ExecutiveApi;
