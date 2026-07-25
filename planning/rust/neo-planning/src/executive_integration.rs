//! Executive integration for the planning system.
//!
//! Provides types and structures for delegating plan execution to the
//! executive layer, tracking execution progress, and producing reports.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{PlanningError, PlanningResult};
use crate::id::{PlanId, PlanningGoalId, StrategyId};
use crate::plan::PlanState;
use crate::types::{ExecutionBudget, PlanMetrics, PlanStatistics};

// ---------------------------------------------------------------------------
// ExecutiveDelegation
// ---------------------------------------------------------------------------

/// Describes a delegation of plan execution to the executive layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutiveDelegation {
    pub id: String,
    pub plan_id: PlanId,
    pub goal_id: Option<PlanningGoalId>,
    pub strategy_id: Option<StrategyId>,
    pub priority: u32,
    pub budget: ExecutionBudget,
    pub state: PlanState,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl ExecutiveDelegation {
    /// Create a new delegation.
    pub fn new(id: impl Into<String>, plan_id: PlanId) -> Self {
        Self {
            id: id.into(),
            plan_id,
            goal_id: None,
            strategy_id: None,
            priority: 2,
            budget: ExecutionBudget::default(),
            state: PlanState::Created,
            metadata: HashMap::new(),
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
        }
    }

    /// Attach a goal id.
    #[must_use]
    pub fn with_goal_id(mut self, goal_id: PlanningGoalId) -> Self {
        self.goal_id = Some(goal_id);
        self
    }

    /// Attach a strategy id.
    #[must_use]
    pub fn with_strategy_id(mut self, strategy_id: StrategyId) -> Self {
        self.strategy_id = Some(strategy_id);
        self
    }

    /// Set the priority.
    #[must_use]
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    /// Set the budget.
    #[must_use]
    pub fn with_budget(mut self, budget: ExecutionBudget) -> Self {
        self.budget = budget;
        self
    }

    /// Add metadata.
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Mark as started.
    pub fn start(&mut self) {
        self.state = PlanState::Executing;
        self.started_at = Some(Utc::now());
    }

    /// Mark as completed.
    pub fn complete(&mut self) {
        self.state = PlanState::Completed;
        self.completed_at = Some(Utc::now());
    }

    /// Mark as failed.
    pub fn fail(&mut self) {
        self.state = PlanState::Failed;
        self.completed_at = Some(Utc::now());
    }

    /// Mark as cancelled.
    pub fn cancel(&mut self) {
        self.state = PlanState::Cancelled;
        self.completed_at = Some(Utc::now());
    }

    /// Check whether the delegation is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        self.state.is_terminal()
    }

    /// Get the elapsed time since creation, in seconds.
    pub fn elapsed_secs(&self) -> i64 {
        let end = self.completed_at.unwrap_or_else(Utc::now);
        (end - self.created_at).num_seconds()
    }
}

// ---------------------------------------------------------------------------
// ExecutionProgress
// ---------------------------------------------------------------------------

/// Tracks the live progress of a delegation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionProgress {
    pub delegation_id: String,
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
    pub running_tasks: usize,
    pub pending_tasks: usize,
    pub overall_progress: f64,
    pub current_phase: String,
    pub updated_at: DateTime<Utc>,
}

impl ExecutionProgress {
    /// Create a new progress tracker.
    pub fn new(delegation_id: impl Into<String>, total_tasks: usize) -> Self {
        Self {
            delegation_id: delegation_id.into(),
            total_tasks,
            completed_tasks: 0,
            failed_tasks: 0,
            running_tasks: 0,
            pending_tasks: total_tasks,
            overall_progress: 0.0,
            current_phase: String::new(),
            updated_at: Utc::now(),
        }
    }

    /// Set the current phase.
    #[must_use]
    pub fn with_phase(mut self, phase: impl Into<String>) -> Self {
        self.current_phase = phase.into();
        self
    }

    /// Mark a task as completed.
    pub fn task_completed(&mut self) {
        if self.pending_tasks > 0 {
            self.pending_tasks -= 1;
        }
        self.completed_tasks += 1;
        self.recompute_progress();
        self.updated_at = Utc::now();
    }

    /// Mark a task as failed.
    pub fn task_failed(&mut self) {
        if self.pending_tasks > 0 {
            self.pending_tasks -= 1;
        }
        self.failed_tasks += 1;
        self.recompute_progress();
        self.updated_at = Utc::now();
    }

    /// Mark a task as running.
    pub fn task_started(&mut self) {
        if self.pending_tasks > 0 {
            self.pending_tasks -= 1;
        }
        self.running_tasks += 1;
        self.updated_at = Utc::now();
    }

    /// Mark a running task as completed.
    pub fn running_task_completed(&mut self) {
        if self.running_tasks > 0 {
            self.running_tasks -= 1;
        }
        self.completed_tasks += 1;
        self.recompute_progress();
        self.updated_at = Utc::now();
    }

    fn recompute_progress(&mut self) {
        if self.total_tasks == 0 {
            self.overall_progress = 1.0;
        } else {
            self.overall_progress = self.completed_tasks as f64 / self.total_tasks as f64;
        }
    }

    /// Check whether all tasks are done (completed or failed).
    pub fn is_complete(&self) -> bool {
        self.completed_tasks + self.failed_tasks >= self.total_tasks
    }
}

// ---------------------------------------------------------------------------
// ExecutiveReport
// ---------------------------------------------------------------------------

/// A report summarizing the outcome of a delegation execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutiveReport {
    pub delegation_id: String,
    pub plan_id: PlanId,
    pub success: bool,
    pub statistics: PlanStatistics,
    pub metrics: PlanMetrics,
    pub error: Option<String>,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
    pub total_tasks: usize,
    pub duration_secs: i64,
    pub generated_at: DateTime<Utc>,
}

impl ExecutiveReport {
    /// Create a successful report.
    pub fn success(
        delegation_id: impl Into<String>,
        plan_id: PlanId,
        statistics: PlanStatistics,
        metrics: PlanMetrics,
        duration_secs: i64,
    ) -> Self {
        Self {
            delegation_id: delegation_id.into(),
            plan_id,
            success: true,
            statistics: statistics.clone(),
            metrics,
            error: None,
            completed_tasks: statistics.completed_tasks,
            failed_tasks: statistics.failed_tasks,
            total_tasks: statistics.total_tasks,
            duration_secs,
            generated_at: Utc::now(),
        }
    }

    /// Create a failed report.
    pub fn failure(
        delegation_id: impl Into<String>,
        plan_id: PlanId,
        statistics: PlanStatistics,
        metrics: PlanMetrics,
        error: impl Into<String>,
        duration_secs: i64,
    ) -> Self {
        Self {
            delegation_id: delegation_id.into(),
            plan_id,
            success: false,
            statistics: statistics.clone(),
            metrics,
            error: Some(error.into()),
            completed_tasks: statistics.completed_tasks,
            failed_tasks: statistics.failed_tasks,
            total_tasks: statistics.total_tasks,
            duration_secs,
            generated_at: Utc::now(),
        }
    }

    /// Get the completion percentage.
    pub fn completion_percentage(&self) -> f64 {
        if self.total_tasks == 0 {
            return 100.0;
        }
        (self.completed_tasks as f64 / self.total_tasks as f64) * 100.0
    }
}

// ---------------------------------------------------------------------------
// ExecutiveIntegration
// ---------------------------------------------------------------------------

/// Manages the lifecycle of delegations and produces reports.
#[derive(Debug, Clone)]
pub struct ExecutiveIntegration {
    delegations: Vec<ExecutiveDelegation>,
    progress: HashMap<String, ExecutionProgress>,
}

impl ExecutiveIntegration {
    /// Create a new executive integration.
    pub fn new() -> Self {
        Self {
            delegations: Vec::new(),
            progress: HashMap::new(),
        }
    }

    /// Create and register a new delegation.
    pub fn create_delegation(&mut self, delegation: ExecutiveDelegation) -> String {
        let id = delegation.id.clone();
        let total_tasks = delegation
            .metadata
            .get("total_tasks")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(0);
        self.progress
            .insert(id.clone(), ExecutionProgress::new(&id, total_tasks));
        self.delegations.push(delegation);
        id
    }

    /// Get a delegation by id.
    pub fn get_delegation(&self, id: &str) -> PlanningResult<&ExecutiveDelegation> {
        self.delegations.iter().find(|d| d.id == id).ok_or_else(|| {
            PlanningError::new(
                crate::error::PlanningErrorCode::PlanNotFound,
                format!("delegation '{}' not found", id),
            )
        })
    }

    /// Get progress for a delegation.
    pub fn get_progress(&self, delegation_id: &str) -> PlanningResult<&ExecutionProgress> {
        self.progress.get(delegation_id).ok_or_else(|| {
            PlanningError::new(
                crate::error::PlanningErrorCode::PlanNotFound,
                format!("progress for delegation '{}' not found", delegation_id),
            )
        })
    }

    /// Update progress for a delegation.
    pub fn update_progress(
        &mut self,
        delegation_id: &str,
        f: impl FnOnce(&mut ExecutionProgress),
    ) -> PlanningResult<()> {
        if let Some(progress) = self.progress.get_mut(delegation_id) {
            f(progress);
            Ok(())
        } else {
            Err(PlanningError::new(
                crate::error::PlanningErrorCode::PlanNotFound,
                format!("progress for delegation '{}' not found", delegation_id),
            ))
        }
    }

    /// Get all delegations.
    pub fn all_delegations(&self) -> &[ExecutiveDelegation] {
        &self.delegations
    }

    /// Get delegations by state.
    pub fn delegations_by_state(&self, state: PlanState) -> Vec<&ExecutiveDelegation> {
        self.delegations
            .iter()
            .filter(|d| d.state == state)
            .collect()
    }

    /// Get active (non-terminal) delegations.
    pub fn active_delegations(&self) -> Vec<&ExecutiveDelegation> {
        self.delegations
            .iter()
            .filter(|d| !d.is_terminal())
            .collect()
    }

    /// Generate a report for a delegation.
    pub fn generate_report(&self, delegation_id: &str) -> PlanningResult<ExecutiveReport> {
        let delegation = self.get_delegation(delegation_id)?;
        let progress = self.get_progress(delegation_id)?;

        let stats = PlanStatistics {
            total_tasks: progress.total_tasks,
            completed_tasks: progress.completed_tasks,
            failed_tasks: progress.failed_tasks,
            pending_tasks: progress.pending_tasks,
            running_tasks: progress.running_tasks,
            ..Default::default()
        };

        let duration = delegation.elapsed_secs();

        if delegation.state == PlanState::Completed {
            Ok(ExecutiveReport::success(
                delegation_id,
                delegation.plan_id,
                stats,
                PlanMetrics::default(),
                duration,
            ))
        } else if delegation.state == PlanState::Failed {
            let error = delegation
                .metadata
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error")
                .to_string();
            Ok(ExecutiveReport::failure(
                delegation_id,
                delegation.plan_id,
                stats,
                PlanMetrics::default(),
                error,
                duration,
            ))
        } else {
            Err(PlanningError::new(
                crate::error::PlanningErrorCode::PlanInvalidState,
                format!(
                    "cannot generate report for delegation '{}' in state {:?}",
                    delegation_id, delegation.state
                ),
            ))
        }
    }
}

impl Default for ExecutiveIntegration {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ExecutiveDelegation tests

    #[test]
    fn delegation_creation() {
        let d = ExecutiveDelegation::new("d1", PlanId::new());
        assert_eq!(d.id, "d1");
        assert_eq!(d.state, PlanState::Created);
        assert!(!d.is_terminal());
    }

    #[test]
    fn delegation_builder() {
        let goal_id = PlanningGoalId::new();
        let strategy_id = StrategyId::new();
        let d = ExecutiveDelegation::new("d", PlanId::new())
            .with_goal_id(goal_id)
            .with_strategy_id(strategy_id)
            .with_priority(1)
            .with_budget(ExecutionBudget::default())
            .with_metadata("k", serde_json::json!(42));
        assert_eq!(d.goal_id, Some(goal_id));
        assert_eq!(d.strategy_id, Some(strategy_id));
        assert_eq!(d.priority, 1);
        assert_eq!(d.metadata.get("k").unwrap(), 42);
    }

    #[test]
    fn delegation_lifecycle() {
        let mut d = ExecutiveDelegation::new("d", PlanId::new());
        assert_eq!(d.state, PlanState::Created);
        assert!(d.started_at.is_none());

        d.start();
        assert_eq!(d.state, PlanState::Executing);
        assert!(d.started_at.is_some());

        d.complete();
        assert_eq!(d.state, PlanState::Completed);
        assert!(d.is_terminal());
        assert!(d.completed_at.is_some());
    }

    #[test]
    fn delegation_fail() {
        let mut d = ExecutiveDelegation::new("d", PlanId::new());
        d.start();
        d.fail();
        assert_eq!(d.state, PlanState::Failed);
        assert!(d.is_terminal());
    }

    #[test]
    fn delegation_cancel() {
        let mut d = ExecutiveDelegation::new("d", PlanId::new());
        d.cancel();
        assert_eq!(d.state, PlanState::Cancelled);
        assert!(d.is_terminal());
    }

    #[test]
    fn delegation_elapsed_secs() {
        let d = ExecutiveDelegation::new("d", PlanId::new());
        let elapsed = d.elapsed_secs();
        assert!(elapsed >= 0);
    }

    // ExecutionProgress tests

    #[test]
    fn progress_creation() {
        let p = ExecutionProgress::new("d1", 10);
        assert_eq!(p.delegation_id, "d1");
        assert_eq!(p.total_tasks, 10);
        assert_eq!(p.pending_tasks, 10);
        assert!((p.overall_progress - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn progress_builder() {
        let p = ExecutionProgress::new("d", 5).with_phase("init");
        assert_eq!(p.current_phase, "init");
    }

    #[test]
    fn progress_task_completed() {
        let mut p = ExecutionProgress::new("d", 3);
        p.task_completed();
        assert_eq!(p.completed_tasks, 1);
        assert_eq!(p.pending_tasks, 2);
        assert!((p.overall_progress - 1.0 / 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn progress_task_failed() {
        let mut p = ExecutionProgress::new("d", 3);
        p.task_failed();
        assert_eq!(p.failed_tasks, 1);
        assert_eq!(p.pending_tasks, 2);
    }

    #[test]
    fn progress_task_started() {
        let mut p = ExecutionProgress::new("d", 3);
        p.task_started();
        assert_eq!(p.running_tasks, 1);
        assert_eq!(p.pending_tasks, 2);
    }

    #[test]
    fn progress_running_task_completed() {
        let mut p = ExecutionProgress::new("d", 3);
        p.task_started();
        p.running_task_completed();
        assert_eq!(p.running_tasks, 0);
        assert_eq!(p.completed_tasks, 1);
        assert!((p.overall_progress - 1.0 / 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn progress_is_complete() {
        let mut p = ExecutionProgress::new("d", 2);
        assert!(!p.is_complete());
        p.task_completed();
        assert!(!p.is_complete());
        p.task_completed();
        assert!(p.is_complete());
    }

    #[test]
    fn progress_is_complete_with_failures() {
        let mut p = ExecutionProgress::new("d", 2);
        p.task_completed();
        p.task_failed();
        assert!(p.is_complete());
    }

    #[test]
    fn progress_zero_tasks() {
        let p = ExecutionProgress::new("d", 0);
        assert!(p.is_complete());
        assert!((p.overall_progress - 1.0).abs() < f64::EPSILON);
    }

    // ExecutiveReport tests

    #[test]
    fn report_success() {
        let r = ExecutiveReport::success(
            "d1",
            PlanId::new(),
            PlanStatistics::default(),
            PlanMetrics::default(),
            60,
        );
        assert!(r.success);
        assert!(r.error.is_none());
        assert_eq!(r.duration_secs, 60);
    }

    #[test]
    fn report_failure() {
        let r = ExecutiveReport::failure(
            "d1",
            PlanId::new(),
            PlanStatistics::default(),
            PlanMetrics::default(),
            "boom",
            30,
        );
        assert!(!r.success);
        assert_eq!(r.error.unwrap(), "boom");
        assert_eq!(r.duration_secs, 30);
    }

    #[test]
    fn report_completion_percentage() {
        let mut stats = PlanStatistics::default();
        stats.total_tasks = 10;
        stats.completed_tasks = 5;
        let r = ExecutiveReport::success("d", PlanId::new(), stats, PlanMetrics::default(), 0);
        assert!((r.completion_percentage() - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn report_completion_percentage_zero_tasks() {
        let r = ExecutiveReport::success(
            "d",
            PlanId::new(),
            PlanStatistics::default(),
            PlanMetrics::default(),
            0,
        );
        assert!((r.completion_percentage() - 100.0).abs() < f64::EPSILON);
    }

    // ExecutiveIntegration tests

    #[test]
    fn integration_new() {
        let i = ExecutiveIntegration::new();
        assert!(i.all_delegations().is_empty());
    }

    #[test]
    fn integration_create_delegation() {
        let mut i = ExecutiveIntegration::new();
        let mut d = ExecutiveDelegation::new("d1", PlanId::new());
        d.metadata
            .insert("total_tasks".to_string(), serde_json::json!(5));
        let id = i.create_delegation(d);
        assert_eq!(id, "d1");
        assert_eq!(i.all_delegations().len(), 1);
    }

    #[test]
    fn integration_get_delegation() {
        let mut i = ExecutiveIntegration::new();
        i.create_delegation(ExecutiveDelegation::new("d1", PlanId::new()));
        assert!(i.get_delegation("d1").is_ok());
        assert!(i.get_delegation("missing").is_err());
    }

    #[test]
    fn integration_get_progress() {
        let mut i = ExecutiveIntegration::new();
        i.create_delegation(ExecutiveDelegation::new("d1", PlanId::new()));
        assert!(i.get_progress("d1").is_ok());
        assert!(i.get_progress("missing").is_err());
    }

    #[test]
    fn integration_update_progress() {
        let mut i = ExecutiveIntegration::new();
        i.create_delegation(ExecutiveDelegation::new("d1", PlanId::new()));
        i.update_progress("d1", |p| {
            p.task_completed();
        })
        .unwrap();
        let p = i.get_progress("d1").unwrap();
        assert_eq!(p.completed_tasks, 1);
    }

    #[test]
    fn integration_delegations_by_state() {
        let mut i = ExecutiveIntegration::new();
        i.create_delegation(ExecutiveDelegation::new("d1", PlanId::new()));
        i.create_delegation(ExecutiveDelegation::new("d2", PlanId::new()));
        i.delegations_by_state(PlanState::Created);
        assert_eq!(i.delegations_by_state(PlanState::Created).len(), 2);
    }

    #[test]
    fn integration_active_delegations() {
        let mut i = ExecutiveIntegration::new();
        let mut d1 = ExecutiveDelegation::new("d1", PlanId::new());
        d1.cancel();
        i.create_delegation(d1);
        i.create_delegation(ExecutiveDelegation::new("d2", PlanId::new()));
        assert_eq!(i.active_delegations().len(), 1);
        assert_eq!(i.active_delegations()[0].id, "d2");
    }

    #[test]
    fn integration_generate_report_completed() {
        let mut i = ExecutiveIntegration::new();
        let mut d = ExecutiveDelegation::new("d1", PlanId::new());
        d.metadata
            .insert("total_tasks".to_string(), serde_json::json!(2));
        d.start();
        d.complete();
        i.create_delegation(d);
        i.update_progress("d1", |p| {
            p.task_completed();
            p.task_completed();
        })
        .unwrap();

        let report = i.generate_report("d1").unwrap();
        assert!(report.success);
        assert_eq!(report.total_tasks, 2);
    }

    #[test]
    fn integration_generate_report_pending_errors() {
        let mut i = ExecutiveIntegration::new();
        i.create_delegation(ExecutiveDelegation::new("d1", PlanId::new()));
        // State is Created, not terminal
        let result = i.generate_report("d1");
        assert!(result.is_err());
    }

    // Serialization tests

    #[test]
    fn delegation_serialization_roundtrip() {
        let d = ExecutiveDelegation::new("d1", PlanId::new()).with_priority(3);
        let json = serde_json::to_string(&d).unwrap();
        let back: ExecutiveDelegation = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "d1");
        assert_eq!(back.priority, 3);
    }

    #[test]
    fn progress_serialization_roundtrip() {
        let mut p = ExecutionProgress::new("d1", 10);
        p.task_completed();
        p.task_started();
        let json = serde_json::to_string(&p).unwrap();
        let back: ExecutionProgress = serde_json::from_str(&json).unwrap();
        assert_eq!(back.completed_tasks, 1);
        assert_eq!(back.running_tasks, 1);
    }

    #[test]
    fn report_serialization_roundtrip() {
        let r = ExecutiveReport::success(
            "d1",
            PlanId::new(),
            PlanStatistics::default(),
            PlanMetrics::default(),
            60,
        );
        let json = serde_json::to_string(&r).unwrap();
        let back: ExecutiveReport = serde_json::from_str(&json).unwrap();
        assert!(back.success);
    }
}
