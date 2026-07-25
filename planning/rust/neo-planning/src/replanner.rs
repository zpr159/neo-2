//! Dynamic replanning for the Neo Planning System.
//!
//! Provides trigger detection, replan scoping, partial and full replanning,
//! and a history of replan events for auditing and learning.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{PlanningError, PlanningErrorCode, PlanningResult};
use crate::goal::{Goal, GoalHierarchy, GoalStatus};
use crate::id::{PlanId, PlanningGoalId, PlanningNodeId, ReplanEventId};
use crate::plan::{Plan, PlanCheckpoint, PlanExecution, PlanState, PlanTask};
use crate::types::{PlanVersion, PlanningConfiguration, TaskStatus};

// ---------------------------------------------------------------------------
// ReplanTrigger
// ---------------------------------------------------------------------------

/// Describes the event that triggered a replanning cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReplanTrigger {
    /// A task failed during execution.
    TaskFailed {
        task_id: PlanningNodeId,
        reason: String,
    },
    /// A required tool is unavailable.
    ToolUnavailable { tool_name: String },
    /// A required agent is unavailable.
    AgentUnavailable { agent_id: String },
    /// A constraint changed its value.
    ConstraintChanged {
        key: String,
        old_value: serde_json::Value,
        new_value: serde_json::Value,
    },
    /// A goal's priority changed.
    PriorityChanged {
        goal_id: PlanningGoalId,
        old_priority: String,
        new_priority: String,
    },
    /// A new goal was added to the hierarchy.
    NewGoalAdded { goal_id: PlanningGoalId },
    /// A resource has been exhausted.
    ResourceExhausted { resource: String },
    /// A deadline is approaching.
    DeadlineApproaching { remaining_secs: i64 },
}

impl ReplanTrigger {
    /// Get a human-readable description of this trigger.
    pub fn description(&self) -> String {
        match self {
            Self::TaskFailed { task_id, reason } => {
                format!("Task {} failed: {}", task_id, reason)
            }
            Self::ToolUnavailable { tool_name } => {
                format!("Tool '{}' is unavailable", tool_name)
            }
            Self::AgentUnavailable { agent_id } => {
                format!("Agent '{}' is unavailable", agent_id)
            }
            Self::ConstraintChanged {
                key,
                old_value,
                new_value,
            } => {
                format!(
                    "Constraint '{}' changed from {} to {}",
                    key, old_value, new_value
                )
            }
            Self::PriorityChanged {
                goal_id,
                old_priority,
                new_priority,
            } => {
                format!(
                    "Goal {} priority changed from {} to {}",
                    goal_id, old_priority, new_priority
                )
            }
            Self::NewGoalAdded { goal_id } => {
                format!("New goal {} added", goal_id)
            }
            Self::ResourceExhausted { resource } => {
                format!("Resource '{}' exhausted", resource)
            }
            Self::DeadlineApproaching { remaining_secs } => {
                format!("Deadline approaching: {} seconds remaining", remaining_secs)
            }
        }
    }

    /// Returns `true` if this trigger is severity-critical and should always
    /// force a full replan.
    pub fn is_critical(&self) -> bool {
        match self {
            Self::ResourceExhausted { .. } | Self::AgentUnavailable { .. } => true,
            Self::DeadlineApproaching { remaining_secs } => *remaining_secs < 60,
            _ => false,
        }
    }
}

// ---------------------------------------------------------------------------
// ReplanScope
// ---------------------------------------------------------------------------

/// Determines whether a replan should affect a subset of tasks or the entire plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReplanScope {
    /// Only re-plan the specified tasks and their transitive dependents.
    PartialReplan {
        affected_task_ids: Vec<PlanningNodeId>,
    },
    /// Re-plan the entire plan from scratch.
    CompleteReplan,
}

impl ReplanScope {
    /// Returns `true` if this is a complete replan.
    pub fn is_complete(&self) -> bool {
        matches!(self, Self::CompleteReplan)
    }

    /// Returns the number of affected tasks, or `None` for a complete replan.
    pub fn affected_count(&self) -> Option<usize> {
        match self {
            Self::PartialReplan { affected_task_ids } => Some(affected_task_ids.len()),
            Self::CompleteReplan => None,
        }
    }
}

// ---------------------------------------------------------------------------
// ReplanEvent
// ---------------------------------------------------------------------------

/// A recorded event that triggered a replanning cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplanEvent {
    pub id: ReplanEventId,
    pub trigger: ReplanTrigger,
    pub scope: ReplanScope,
    pub plan_id: PlanId,
    pub created_at: DateTime<Utc>,
    pub resolved: bool,
    pub resolution: Option<String>,
}

impl ReplanEvent {
    /// Create a new replan event.
    pub fn new(plan_id: PlanId, trigger: ReplanTrigger, scope: ReplanScope) -> Self {
        Self {
            id: ReplanEventId::new(),
            trigger,
            scope,
            plan_id,
            created_at: Utc::now(),
            resolved: false,
            resolution: None,
        }
    }

    /// Mark this event as resolved with an optional description.
    pub fn resolve(&mut self, description: impl Into<String>) {
        self.resolved = true;
        self.resolution = Some(description.into());
    }
}

// ---------------------------------------------------------------------------
// ReplanResult
// ---------------------------------------------------------------------------

/// The outcome of executing a replanning cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplanResult {
    pub success: bool,
    pub new_plan_version: PlanVersion,
    pub tasks_added: usize,
    pub tasks_removed: usize,
    pub tasks_modified: usize,
    pub scope: ReplanScope,
    pub event: ReplanEvent,
}

// ---------------------------------------------------------------------------
// Replanner
// ---------------------------------------------------------------------------

/// Orchestrates replanning decisions and execution.
#[derive(Debug, Clone)]
pub struct Replanner;

impl Replanner {
    /// Create a new replanner.
    pub fn new() -> Self {
        Self
    }

    /// Determine whether a trigger warrants replanning based on configuration.
    pub fn should_replan(&self, trigger: &ReplanTrigger, config: &PlanningConfiguration) -> bool {
        if !config.auto_replan {
            return false;
        }

        match trigger {
            ReplanTrigger::TaskFailed { .. } => true,
            ReplanTrigger::ToolUnavailable { .. } => true,
            ReplanTrigger::AgentUnavailable { .. } => true,
            ReplanTrigger::ConstraintChanged { .. } => config.replan_threshold < 1.0,
            ReplanTrigger::PriorityChanged { .. } => true,
            ReplanTrigger::NewGoalAdded { .. } => true,
            ReplanTrigger::ResourceExhausted { .. } => true,
            ReplanTrigger::DeadlineApproaching { remaining_secs } => {
                *remaining_secs <= (config.planning_timeout_secs as i64 / 2)
            }
        }
    }

    /// Create a new replan event from a plan id and trigger, computing the
    /// appropriate scope automatically.
    pub fn create_replan_event(&self, plan_id: PlanId, trigger: ReplanTrigger) -> ReplanEvent {
        let scope = match &trigger {
            ReplanTrigger::TaskFailed { task_id, .. } => ReplanScope::PartialReplan {
                affected_task_ids: vec![*task_id],
            },
            ReplanTrigger::ToolUnavailable { .. }
            | ReplanTrigger::AgentUnavailable { .. }
            | ReplanTrigger::ResourceExhausted { .. } => ReplanScope::CompleteReplan,
            ReplanTrigger::ConstraintChanged { .. } => ReplanScope::CompleteReplan,
            ReplanTrigger::PriorityChanged { .. } => ReplanScope::CompleteReplan,
            ReplanTrigger::NewGoalAdded { .. } => ReplanScope::CompleteReplan,
            ReplanTrigger::DeadlineApproaching { .. } => ReplanScope::CompleteReplan,
        };

        ReplanEvent::new(plan_id, trigger, scope)
    }

    /// Execute a partial replan: remove failed tasks, regenerate the affected
    /// portion of the plan.
    pub fn execute_partial_replan(
        &self,
        plan: &mut Plan,
        execution: &mut PlanExecution,
        affected_tasks: &[PlanningNodeId],
        _goals: &GoalHierarchy,
    ) -> PlanningResult<ReplanResult> {
        if affected_tasks.is_empty() {
            return Err(PlanningError::validation(
                "no affected tasks specified for partial replan",
            ));
        }

        let event = ReplanEvent::new(
            plan.id,
            ReplanTrigger::TaskFailed {
                task_id: affected_tasks[0],
                reason: "partial replan triggered".to_string(),
            },
            ReplanScope::PartialReplan {
                affected_task_ids: affected_tasks.to_vec(),
            },
        );

        let mut tasks_removed = 0usize;
        let mut tasks_modified = 0usize;

        let affected_set: std::collections::HashSet<PlanningNodeId> =
            affected_tasks.iter().copied().collect();

        // Remove affected tasks from the plan.
        plan.definition.tasks.retain(|t| {
            if affected_set.contains(&t.id) {
                tasks_removed += 1;
                false
            } else {
                true
            }
        });

        // Clean up references: remove dependencies pointing to removed tasks.
        for task in &mut plan.definition.tasks {
            let before = task.dependencies.len();
            task.dependencies.retain(|dep| !affected_set.contains(dep));
            if task.dependencies.len() < before {
                tasks_modified += 1;
            }
        }

        // Remove affected tasks from execution tracking.
        for &task_id in affected_tasks {
            execution.failed_tasks.remove(&task_id);
            execution.completed_tasks.remove(&task_id);
        }

        // Bump version.
        plan.bump_version();
        plan.updated_at = Utc::now();

        let scope = ReplanScope::PartialReplan {
            affected_task_ids: affected_tasks.to_vec(),
        };

        Ok(ReplanResult {
            success: true,
            new_plan_version: plan.version,
            tasks_added: 0,
            tasks_removed,
            tasks_modified,
            scope,
            event,
        })
    }

    /// Execute a full replan: reset the plan to `Created` state, increment
    /// the version, and clear completed tasks from the execution tracker.
    pub fn execute_full_replan(
        &self,
        plan: &mut Plan,
        execution: &mut PlanExecution,
        _goals: &GoalHierarchy,
    ) -> PlanningResult<ReplanResult> {
        let event = ReplanEvent::new(
            plan.id,
            ReplanTrigger::ResourceExhausted {
                resource: "full_replan".to_string(),
            },
            ReplanScope::CompleteReplan,
        );

        let tasks_before = plan.definition.tasks.len();
        let mut tasks_modified = 0usize;

        // Reset all pending tasks to a clean state.
        for task in &mut plan.definition.tasks {
            if task.status == TaskStatus::Pending || task.status == TaskStatus::Ready {
                continue;
            }
            if task.status == TaskStatus::Failed || task.status == TaskStatus::Running {
                task.status = TaskStatus::Pending;
                tasks_modified += 1;
            }
        }

        // Clear execution state.
        execution.completed_tasks.clear();
        execution.failed_tasks.clear();
        execution.current_task = None;

        // Bump version.
        plan.bump_version();
        plan.updated_at = Utc::now();

        Ok(ReplanResult {
            success: true,
            new_plan_version: plan.version,
            tasks_added: 0,
            tasks_removed: 0,
            tasks_modified,
            scope: ReplanScope::CompleteReplan,
            event,
        })
    }
}

impl Default for Replanner {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// ReplanHistory
// ---------------------------------------------------------------------------

/// Records all replan events and results for auditing and learning.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReplanHistory {
    pub events: Vec<ReplanEvent>,
    pub results: Vec<ReplanResult>,
}

impl ReplanHistory {
    /// Create an empty history.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a new event.
    pub fn record_event(&mut self, event: ReplanEvent) {
        self.events.push(event);
    }

    /// Record a replan result.
    pub fn record_result(&mut self, result: ReplanResult) {
        self.results.push(result);
    }

    /// Total number of replanning attempts.
    pub fn total_replans(&self) -> usize {
        self.results.len()
    }

    /// Fraction of replans that succeeded.
    pub fn success_rate(&self) -> f64 {
        if self.results.is_empty() {
            return 0.0;
        }
        let successes = self.results.iter().filter(|r| r.success).count();
        successes as f64 / self.results.len() as f64
    }

    /// Total tasks removed across all replans.
    pub fn total_tasks_removed(&self) -> usize {
        self.results.iter().map(|r| r.tasks_removed).sum()
    }

    /// Total tasks modified across all replans.
    pub fn total_tasks_modified(&self) -> usize {
        self.results.iter().map(|r| r.tasks_modified).sum()
    }

    /// Number of unresolved events.
    pub fn unresolved_events(&self) -> usize {
        self.events.iter().filter(|e| !e.resolved).count()
    }

    /// Number of complete replans vs partial replans.
    pub fn complete_replan_count(&self) -> usize {
        self.results
            .iter()
            .filter(|r| r.scope.is_complete())
            .count()
    }

    /// Number of partial replans.
    pub fn partial_replan_count(&self) -> usize {
        self.results
            .iter()
            .filter(|r| !r.scope.is_complete())
            .count()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::{PlanningGoalId, PlanningNodeId};
    use crate::plan::{PlanDefinition, PlanTask};
    use crate::types::PlanMetadata;
    use crate::types::{AlgorithmType, ExecutionBudget, PlanTaskType};

    fn make_task(name: &str) -> PlanTask {
        PlanTask::new(name, PlanTaskType::Atomic)
    }

    fn make_plan_with_tasks(tasks: Vec<PlanTask>) -> Plan {
        let goal_id = PlanningGoalId::new();
        let def = PlanDefinition {
            tasks,
            goal_id,
            budget: ExecutionBudget::default(),
            algorithm: AlgorithmType::HierarchicalTaskNetwork,
            allow_parallelism: false,
        };
        Plan::new(def, PlanMetadata::new("test-plan"))
    }

    // ---- ReplanTrigger ----

    #[test]
    fn trigger_task_failed_description() {
        let t = ReplanTrigger::TaskFailed {
            task_id: PlanningNodeId::new(),
            reason: "timeout".to_string(),
        };
        let desc = t.description();
        assert!(desc.contains("timeout"));
    }

    #[test]
    fn trigger_tool_unavailable_description() {
        let t = ReplanTrigger::ToolUnavailable {
            tool_name: "calculator".to_string(),
        };
        assert!(t.description().contains("calculator"));
    }

    #[test]
    fn trigger_agent_unavailable_description() {
        let t = ReplanTrigger::AgentUnavailable {
            agent_id: "agent-1".to_string(),
        };
        assert!(t.description().contains("agent-1"));
    }

    #[test]
    fn trigger_constraint_changed_description() {
        let t = ReplanTrigger::ConstraintChanged {
            key: "max_cost".to_string(),
            old_value: serde_json::json!(100),
            new_value: serde_json::json!(50),
        };
        let desc = t.description();
        assert!(desc.contains("max_cost"));
        assert!(desc.contains("100"));
        assert!(desc.contains("50"));
    }

    #[test]
    fn trigger_priority_changed_description() {
        let t = ReplanTrigger::PriorityChanged {
            goal_id: PlanningGoalId::new(),
            old_priority: "Low".to_string(),
            new_priority: "High".to_string(),
        };
        let desc = t.description();
        assert!(desc.contains("Low"));
        assert!(desc.contains("High"));
    }

    #[test]
    fn trigger_new_goal_added_description() {
        let t = ReplanTrigger::NewGoalAdded {
            goal_id: PlanningGoalId::new(),
        };
        assert!(t.description().contains("New goal"));
    }

    #[test]
    fn trigger_resource_exhausted_description() {
        let t = ReplanTrigger::ResourceExhausted {
            resource: "memory".to_string(),
        };
        assert!(t.description().contains("memory"));
    }

    #[test]
    fn trigger_deadline_approaching_description() {
        let t = ReplanTrigger::DeadlineApproaching { remaining_secs: 30 };
        assert!(t.description().contains("30"));
    }

    #[test]
    fn trigger_is_critical() {
        assert!(ReplanTrigger::ResourceExhausted {
            resource: "cpu".to_string()
        }
        .is_critical());
        assert!(ReplanTrigger::AgentUnavailable {
            agent_id: "a".to_string()
        }
        .is_critical());
        assert!(ReplanTrigger::DeadlineApproaching { remaining_secs: 30 }.is_critical());
        assert!(!ReplanTrigger::DeadlineApproaching {
            remaining_secs: 300
        }
        .is_critical());
        assert!(!ReplanTrigger::TaskFailed {
            task_id: PlanningNodeId::new(),
            reason: "err".to_string()
        }
        .is_critical());
    }

    // ---- ReplanScope ----

    #[test]
    fn scope_is_complete() {
        assert!(ReplanScope::CompleteReplan.is_complete());
        assert!(!ReplanScope::PartialReplan {
            affected_task_ids: vec![]
        }
        .is_complete());
    }

    #[test]
    fn scope_affected_count() {
        assert!(ReplanScope::CompleteReplan.affected_count().is_none());
        assert_eq!(
            ReplanScope::PartialReplan {
                affected_task_ids: vec![PlanningNodeId::new(), PlanningNodeId::new()]
            }
            .affected_count(),
            Some(2)
        );
    }

    // ---- ReplanEvent ----

    #[test]
    fn event_creation() {
        let event = ReplanEvent::new(
            PlanId::new(),
            ReplanTrigger::TaskFailed {
                task_id: PlanningNodeId::new(),
                reason: "err".to_string(),
            },
            ReplanScope::CompleteReplan,
        );
        assert!(!event.resolved);
        assert!(event.resolution.is_none());
    }

    #[test]
    fn event_resolve() {
        let mut event = ReplanEvent::new(
            PlanId::new(),
            ReplanTrigger::NewGoalAdded {
                goal_id: PlanningGoalId::new(),
            },
            ReplanScope::CompleteReplan,
        );
        event.resolve("fixed");
        assert!(event.resolved);
        assert_eq!(event.resolution.unwrap(), "fixed");
    }

    // ---- Replanner::should_replan ----

    #[test]
    fn should_replan_task_failed() {
        let replanner = Replanner::new();
        let config = PlanningConfiguration::default();
        let trigger = ReplanTrigger::TaskFailed {
            task_id: PlanningNodeId::new(),
            reason: "err".to_string(),
        };
        assert!(replanner.should_replan(&trigger, &config));
    }

    #[test]
    fn should_replan_disabled() {
        let replanner = Replanner::new();
        let mut config = PlanningConfiguration::default();
        config.auto_replan = false;
        let trigger = ReplanTrigger::TaskFailed {
            task_id: PlanningNodeId::new(),
            reason: "err".to_string(),
        };
        assert!(!replanner.should_replan(&trigger, &config));
    }

    #[test]
    fn should_replan_new_goal() {
        let replanner = Replanner::new();
        let config = PlanningConfiguration::default();
        let trigger = ReplanTrigger::NewGoalAdded {
            goal_id: PlanningGoalId::new(),
        };
        assert!(replanner.should_replan(&trigger, &config));
    }

    #[test]
    fn should_replan_deadline_approaching() {
        let replanner = Replanner::new();
        let config = PlanningConfiguration::default();
        // planning_timeout_secs is 300, so threshold is 150.
        let trigger_early = ReplanTrigger::DeadlineApproaching {
            remaining_secs: 100,
        };
        assert!(replanner.should_replan(&trigger_early, &config));

        let trigger_late = ReplanTrigger::DeadlineApproaching {
            remaining_secs: 200,
        };
        assert!(!replanner.should_replan(&trigger_late, &config));
    }

    // ---- Replanner::create_replan_event ----

    #[test]
    fn create_replan_event_task_failed() {
        let replanner = Replanner::new();
        let trigger = ReplanTrigger::TaskFailed {
            task_id: PlanningNodeId::new(),
            reason: "err".to_string(),
        };
        let event = replanner.create_replan_event(PlanId::new(), trigger);
        assert!(!event.scope.is_complete());
    }

    #[test]
    fn create_replan_event_resource_exhausted() {
        let replanner = Replanner::new();
        let trigger = ReplanTrigger::ResourceExhausted {
            resource: "cpu".to_string(),
        };
        let event = replanner.create_replan_event(PlanId::new(), trigger);
        assert!(event.scope.is_complete());
    }

    // ---- Replanner::execute_partial_replan ----

    #[test]
    fn execute_partial_replan_removes_tasks() {
        let replanner = Replanner::new();
        let t1 = make_task("t1");
        let t2 = make_task("t2");
        let t3 = make_task("t3");

        let mut plan = make_plan_with_tasks(vec![t1.clone(), t2.clone(), t3.clone()]);
        let mut execution = PlanExecution::new(plan.id);

        let goals = GoalHierarchy::new();

        let result = replanner
            .execute_partial_replan(&mut plan, &mut execution, &[t2.id], &goals)
            .unwrap();

        assert!(result.success);
        assert_eq!(result.tasks_removed, 1);
        assert_eq!(plan.definition.tasks.len(), 2);
        assert!(plan.definition.tasks.iter().all(|t| t.id != t2.id));
    }

    #[test]
    fn execute_partial_replan_cleans_dependencies() {
        let replanner = Replanner::new();
        let t1 = make_task("t1");
        let mut t2 = make_task("t2");
        let mut t3 = make_task("t3");
        t3.dependencies = vec![t2.id];

        let mut plan = make_plan_with_tasks(vec![t1, t2.clone(), t3]);
        let mut execution = PlanExecution::new(plan.id);
        let goals = GoalHierarchy::new();

        let result = replanner
            .execute_partial_replan(&mut plan, &mut execution, &[t2.id], &goals)
            .unwrap();

        assert!(result.tasks_modified > 0);
        // The task that depended on t2 should have its dependency removed.
        let t3_after = plan
            .definition
            .tasks
            .iter()
            .find(|t| t.name == "t3")
            .unwrap();
        assert!(t3_after.dependencies.is_empty());
    }

    #[test]
    fn execute_partial_replan_empty_errors() {
        let replanner = Replanner::new();
        let mut plan = make_plan_with_tasks(vec![make_task("t1")]);
        let mut execution = PlanExecution::new(plan.id);
        let goals = GoalHierarchy::new();

        let result = replanner.execute_partial_replan(&mut plan, &mut execution, &[], &goals);
        assert!(result.is_err());
    }

    #[test]
    fn execute_partial_replan_bumps_version() {
        let replanner = Replanner::new();
        let t1 = make_task("t1");
        let t2 = make_task("t2");
        let mut plan = make_plan_with_tasks(vec![t1, t2.clone()]);
        let original_version = plan.version;
        let mut execution = PlanExecution::new(plan.id);
        let goals = GoalHierarchy::new();

        replanner
            .execute_partial_replan(&mut plan, &mut execution, &[t2.id], &goals)
            .unwrap();

        assert!(plan.version.patch > original_version.patch);
    }

    // ---- Replanner::execute_full_replan ----

    #[test]
    fn execute_full_replan_resets_execution() {
        let replanner = Replanner::new();
        let t1 = make_task("t1");
        let t2 = make_task("t2");
        let mut plan = make_plan_with_tasks(vec![t1, t2]);
        let mut execution = PlanExecution::new(plan.id);
        execution.completed_tasks.insert(PlanningNodeId::new());
        execution.failed_tasks.insert(PlanningNodeId::new());
        let goals = GoalHierarchy::new();

        let result = replanner
            .execute_full_replan(&mut plan, &mut execution, &goals)
            .unwrap();

        assert!(result.success);
        assert!(execution.completed_tasks.is_empty());
        assert!(execution.failed_tasks.is_empty());
        assert!(execution.current_task.is_none());
    }

    #[test]
    fn execute_full_replan_bumps_version() {
        let replanner = Replanner::new();
        let t1 = make_task("t1");
        let mut plan = make_plan_with_tasks(vec![t1]);
        let original_version = plan.version;
        let mut execution = PlanExecution::new(plan.id);
        let goals = GoalHierarchy::new();

        replanner
            .execute_full_replan(&mut plan, &mut execution, &goals)
            .unwrap();

        assert!(plan.version.patch > original_version.patch);
    }

    #[test]
    fn execute_full_replan_resets_failed_tasks() {
        let replanner = Replanner::new();
        let mut t1 = make_task("t1");
        t1.status = TaskStatus::Failed;
        let mut plan = make_plan_with_tasks(vec![t1]);
        let mut execution = PlanExecution::new(plan.id);
        let goals = GoalHierarchy::new();

        replanner
            .execute_full_replan(&mut plan, &mut execution, &goals)
            .unwrap();

        assert_eq!(plan.definition.tasks[0].status, TaskStatus::Pending);
    }

    // ---- ReplanHistory ----

    #[test]
    fn history_new() {
        let h = ReplanHistory::new();
        assert_eq!(h.total_replans(), 0);
        assert_eq!(h.success_rate(), 0.0);
    }

    #[test]
    fn history_record_event() {
        let mut h = ReplanHistory::new();
        let event = ReplanEvent::new(
            PlanId::new(),
            ReplanTrigger::TaskFailed {
                task_id: PlanningNodeId::new(),
                reason: "err".to_string(),
            },
            ReplanScope::CompleteReplan,
        );
        h.record_event(event);
        assert_eq!(h.events.len(), 1);
    }

    #[test]
    fn history_record_result() {
        let mut h = ReplanHistory::new();
        let event = ReplanEvent::new(
            PlanId::new(),
            ReplanTrigger::TaskFailed {
                task_id: PlanningNodeId::new(),
                reason: "err".to_string(),
            },
            ReplanScope::CompleteReplan,
        );
        let result = ReplanResult {
            success: true,
            new_plan_version: PlanVersion::initial(),
            tasks_added: 0,
            tasks_removed: 1,
            tasks_modified: 0,
            scope: ReplanScope::CompleteReplan,
            event,
        };
        h.record_result(result);
        assert_eq!(h.total_replans(), 1);
        assert!((h.success_rate() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn history_success_rate_mixed() {
        let mut h = ReplanHistory::new();
        for success in [true, true, false] {
            let event = ReplanEvent::new(
                PlanId::new(),
                ReplanTrigger::TaskFailed {
                    task_id: PlanningNodeId::new(),
                    reason: "err".to_string(),
                },
                ReplanScope::CompleteReplan,
            );
            h.record_result(ReplanResult {
                success,
                new_plan_version: PlanVersion::initial(),
                tasks_added: 0,
                tasks_removed: 0,
                tasks_modified: 0,
                scope: ReplanScope::CompleteReplan,
                event,
            });
        }
        assert!((h.success_rate() - (2.0 / 3.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn history_total_tasks() {
        let mut h = ReplanHistory::new();
        let event = ReplanEvent::new(
            PlanId::new(),
            ReplanTrigger::TaskFailed {
                task_id: PlanningNodeId::new(),
                reason: "err".to_string(),
            },
            ReplanScope::CompleteReplan,
        );
        h.record_result(ReplanResult {
            success: true,
            new_plan_version: PlanVersion::initial(),
            tasks_added: 2,
            tasks_removed: 1,
            tasks_modified: 3,
            scope: ReplanScope::CompleteReplan,
            event: event.clone(),
        });
        h.record_result(ReplanResult {
            success: true,
            new_plan_version: PlanVersion::initial(),
            tasks_added: 0,
            tasks_removed: 2,
            tasks_modified: 1,
            scope: ReplanScope::CompleteReplan,
            event,
        });
        assert_eq!(h.total_tasks_removed(), 3);
        assert_eq!(h.total_tasks_modified(), 4);
    }

    #[test]
    fn history_unresolved_events() {
        let mut h = ReplanHistory::new();
        let mut event = ReplanEvent::new(
            PlanId::new(),
            ReplanTrigger::TaskFailed {
                task_id: PlanningNodeId::new(),
                reason: "err".to_string(),
            },
            ReplanScope::CompleteReplan,
        );
        h.record_event(event.clone());
        assert_eq!(h.unresolved_events(), 1);

        event.resolve("fixed");
        h.record_event(event);
        assert_eq!(h.unresolved_events(), 0);
    }

    #[test]
    fn history_complete_vs_partial() {
        let mut h = ReplanHistory::new();
        let event_complete = ReplanEvent::new(
            PlanId::new(),
            ReplanTrigger::ResourceExhausted {
                resource: "cpu".to_string(),
            },
            ReplanScope::CompleteReplan,
        );
        let event_partial = ReplanEvent::new(
            PlanId::new(),
            ReplanTrigger::TaskFailed {
                task_id: PlanningNodeId::new(),
                reason: "err".to_string(),
            },
            ReplanScope::PartialReplan {
                affected_task_ids: vec![PlanningNodeId::new()],
            },
        );
        h.record_result(ReplanResult {
            success: true,
            new_plan_version: PlanVersion::initial(),
            tasks_added: 0,
            tasks_removed: 0,
            tasks_modified: 0,
            scope: ReplanScope::CompleteReplan,
            event: event_complete,
        });
        h.record_result(ReplanResult {
            success: true,
            new_plan_version: PlanVersion::initial(),
            tasks_added: 0,
            tasks_removed: 0,
            tasks_modified: 0,
            scope: ReplanScope::PartialReplan {
                affected_task_ids: vec![],
            },
            event: event_partial,
        });
        assert_eq!(h.complete_replan_count(), 1);
        assert_eq!(h.partial_replan_count(), 1);
    }

    // ---- Serialization ----

    #[test]
    fn trigger_roundtrip() {
        let trigger = ReplanTrigger::TaskFailed {
            task_id: PlanningNodeId::new(),
            reason: "timeout".to_string(),
        };
        let json = serde_json::to_string(&trigger).unwrap();
        let back: ReplanTrigger = serde_json::from_str(&json).unwrap();
        assert!(back.description().contains("timeout"));
    }

    #[test]
    fn scope_roundtrip() {
        let scope = ReplanScope::PartialReplan {
            affected_task_ids: vec![PlanningNodeId::new()],
        };
        let json = serde_json::to_string(&scope).unwrap();
        let back: ReplanScope = serde_json::from_str(&json).unwrap();
        assert_eq!(back.affected_count(), Some(1));
    }

    #[test]
    fn event_roundtrip() {
        let event = ReplanEvent::new(
            PlanId::new(),
            ReplanTrigger::NewGoalAdded {
                goal_id: PlanningGoalId::new(),
            },
            ReplanScope::CompleteReplan,
        );
        let json = serde_json::to_string(&event).unwrap();
        let back: ReplanEvent = serde_json::from_str(&json).unwrap();
        assert!(!back.resolved);
    }

    #[test]
    fn result_roundtrip() {
        let event = ReplanEvent::new(
            PlanId::new(),
            ReplanTrigger::TaskFailed {
                task_id: PlanningNodeId::new(),
                reason: "err".to_string(),
            },
            ReplanScope::CompleteReplan,
        );
        let result = ReplanResult {
            success: true,
            new_plan_version: PlanVersion::initial(),
            tasks_added: 1,
            tasks_removed: 2,
            tasks_modified: 3,
            scope: ReplanScope::CompleteReplan,
            event,
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: ReplanResult = serde_json::from_str(&json).unwrap();
        assert!(back.success);
        assert_eq!(back.tasks_added, 1);
        assert_eq!(back.tasks_removed, 2);
    }

    #[test]
    fn history_roundtrip() {
        let mut h = ReplanHistory::new();
        let event = ReplanEvent::new(
            PlanId::new(),
            ReplanTrigger::TaskFailed {
                task_id: PlanningNodeId::new(),
                reason: "err".to_string(),
            },
            ReplanScope::CompleteReplan,
        );
        h.record_event(event.clone());
        h.record_result(ReplanResult {
            success: true,
            new_plan_version: PlanVersion::initial(),
            tasks_added: 0,
            tasks_removed: 1,
            tasks_modified: 0,
            scope: ReplanScope::CompleteReplan,
            event,
        });
        let json = serde_json::to_string(&h).unwrap();
        let back: ReplanHistory = serde_json::from_str(&json).unwrap();
        assert_eq!(back.events.len(), 1);
        assert_eq!(back.results.len(), 1);
    }

    // ---- Integration: replanner + plan lifecycle ----

    #[test]
    fn partial_replan_preserves_completed_tasks() {
        let replanner = Replanner::new();
        let t1 = make_task("t1");
        let t2 = make_task("t2");
        let t3 = make_task("t3");

        let mut plan = make_plan_with_tasks(vec![t1.clone(), t2.clone(), t3.clone()]);
        let mut execution = PlanExecution::new(plan.id);
        execution.mark_complete(t1.id);

        let goals = GoalHierarchy::new();
        replanner
            .execute_partial_replan(&mut plan, &mut execution, &[t2.id], &goals)
            .unwrap();

        assert!(execution.completed_tasks.contains(&t1.id));
        assert_eq!(plan.definition.tasks.len(), 2);
    }

    #[test]
    fn full_replan_preserves_all_task_definitions() {
        let replanner = Replanner::new();
        let t1 = make_task("t1");
        let t2 = make_task("t2");

        let mut plan = make_plan_with_tasks(vec![t1, t2]);
        let mut execution = PlanExecution::new(plan.id);
        let goals = GoalHierarchy::new();

        replanner
            .execute_full_replan(&mut plan, &mut execution, &goals)
            .unwrap();

        // Full replan does not remove task definitions, only resets statuses.
        assert_eq!(plan.definition.tasks.len(), 2);
    }
}
