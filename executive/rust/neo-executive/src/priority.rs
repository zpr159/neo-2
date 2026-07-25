use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::goal::{GoalId, GoalPriority};
use crate::task::{TaskId, TaskPriority};
use crate::error::{ExecutiveError, ExecutiveResult};
use crate::resource_coordination::ResourceType;

/// Dynamic priority score combining urgency, importance, and resource factors.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PriorityScore {
    pub urgency: f64,
    pub importance: f64,
    pub resource_factor: f64,
    pub age_factor: f64,
    pub total: f64,
}

impl PriorityScore {
    /// Create a new priority score.
    pub fn new(urgency: f64, importance: f64, resource_factor: f64, age_factor: f64) -> Self {
        let total = urgency * 0.4 + importance * 0.35 + resource_factor * 0.15 + age_factor * 0.1;
        Self {
            urgency,
            importance,
            resource_factor,
            age_factor,
            total,
        }
    }
}

/// Conflict resolution strategy for competing priorities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictResolution {
    PriorityFirst,
    DeadlineFirst,
    FairShare,
    OldestFirst,
    ResourceOptimal,
}

/// Priority rule that adjusts scores based on conditions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityRule {
    pub name: String,
    pub condition: String,
    pub adjustment: f64,
    pub active: bool,
}

/// Resource availability information for priority calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAvailability {
    pub resource_type: ResourceType,
    pub available: u64,
    pub total: u64,
    pub utilization: f64,
}

/// Priority engine that dynamically computes priorities based on urgency, importance, resource awareness, and conflict resolution.
#[derive(Clone)]
pub struct PriorityEngine {
    inner: Arc<PriorityEngineInner>,
}

struct PriorityEngineInner {
    rules: RwLock<Vec<PriorityRule>>,
    resolution_strategy: RwLock<ConflictResolution>,
    goal_scores: RwLock<HashMap<GoalId, PriorityScore>>,
    task_scores: RwLock<HashMap<TaskId, PriorityScore>>,
    deadlines_urgency_weight: RwLock<f64>,
    resource_sensitivity: RwLock<f64>,
}

impl PriorityEngine {
    /// Create a new priority engine.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(PriorityEngineInner {
                rules: RwLock::new(Vec::new()),
                resolution_strategy: RwLock::new(ConflictResolution::PriorityFirst),
                goal_scores: RwLock::new(HashMap::new()),
                task_scores: RwLock::new(HashMap::new()),
                deadlines_urgency_weight: RwLock::new(0.7),
                resource_sensitivity: RwLock::new(0.5),
            }),
        }
    }

    /// Calculate urgency for a goal based on deadline proximity.
    pub fn calculate_urgency(&self, deadline: Option<DateTime<Utc>>, priority: GoalPriority) -> f64 {
        let deadline_urgency = deadline.map_or(0.5, |d| {
            let now = Utc::now();
            let remaining = (d - now).num_seconds().max(0) as f64;
            let weight = *self.inner.deadlines_urgency_weight.read();
            if remaining < 3600.0 {
                1.0 * weight
            } else if remaining < 86400.0 {
                0.7 * weight
            } else {
                0.3 * weight
            }
        });

        let priority_factor = match priority {
            GoalPriority::Critical => 1.0,
            GoalPriority::High => 0.8,
            GoalPriority::Normal => 0.5,
            GoalPriority::Low => 0.3,
            GoalPriority::Background => 0.1,
        };

        (deadline_urgency + priority_factor) / 2.0
    }

    /// Calculate urgency for a task.
    pub fn calculate_task_urgency(
        &self,
        deadline: Option<DateTime<Utc>>,
        priority: TaskPriority,
    ) -> f64 {
        let deadline_urgency = deadline.map_or(0.5, |d| {
            let now = Utc::now();
            let remaining = (d - now).num_seconds().max(0) as f64;
            if remaining < 3600.0 {
                1.0
            } else if remaining < 86400.0 {
                0.7
            } else {
                0.3
            }
        });

        let priority_factor = match priority {
            TaskPriority::Critical => 1.0,
            TaskPriority::High => 0.8,
            TaskPriority::Normal => 0.5,
            TaskPriority::Low => 0.3,
            TaskPriority::Background => 0.1,
        };

        (deadline_urgency + priority_factor) / 2.0
    }

    /// Calculate importance based on goal hierarchy and dependencies.
    pub fn calculate_importance(
        &self,
        has_sub_goals: bool,
        dependency_count: usize,
        dependent_count: usize,
    ) -> f64 {
        let sub_goal_factor = if has_sub_goals { 0.8 } else { 0.4 };
        let dependency_factor = (dependency_count as f64 * 0.1).min(1.0);
        let dependent_factor = (dependent_count as f64 * 0.15).min(1.0);

        (sub_goal_factor + dependency_factor + dependent_factor) / 3.0
    }

    /// Calculate resource factor based on resource availability.
    pub fn calculate_resource_factor(
        &self,
        resources: &[ResourceAvailability],
    ) -> f64 {
        if resources.is_empty() {
            return 0.5;
        }

        let sensitivity = *self.inner.resource_sensitivity.read();
        let avg_availability: f64 = resources
            .iter()
            .map(|r| 1.0 - r.utilization)
            .sum::<f64>()
            / resources.len() as f64;

        avg_availability * sensitivity + 0.5 * (1.0 - sensitivity)
    }

    /// Calculate age factor (older tasks get slight priority boost).
    pub fn calculate_age_factor(&self, created_at: DateTime<Utc>) -> f64 {
        let age_secs = (Utc::now() - created_at).num_seconds() as f64;
        let hours = age_secs / 3600.0;
        (hours / 24.0).min(1.0)
    }

    /// Compute the full priority score for a goal.
    pub fn score_goal(
        &self,
        deadline: Option<DateTime<Utc>>,
        priority: GoalPriority,
        has_sub_goals: bool,
        dependency_count: usize,
        dependent_count: usize,
        resources: &[ResourceAvailability],
        created_at: DateTime<Utc>,
    ) -> PriorityScore {
        let urgency = self.calculate_urgency(deadline, priority);
        let importance = self.calculate_importance(has_sub_goals, dependency_count, dependent_count);
        let resource_factor = self.calculate_resource_factor(resources);
        let age_factor = self.calculate_age_factor(created_at);

        PriorityScore::new(urgency, importance, resource_factor, age_factor)
    }

    /// Compute the full priority score for a task.
    pub fn score_task(
        &self,
        deadline: Option<DateTime<Utc>>,
        priority: TaskPriority,
        resources: &[ResourceAvailability],
        created_at: DateTime<Utc>,
    ) -> PriorityScore {
        let urgency = self.calculate_task_urgency(deadline, priority);
        let importance = 0.5;
        let resource_factor = self.calculate_resource_factor(resources);
        let age_factor = self.calculate_age_factor(created_at);

        PriorityScore::new(urgency, importance, resource_factor, age_factor)
    }

    /// Store a computed score for a goal.
    pub fn set_goal_score(&self, goal_id: GoalId, score: PriorityScore) {
        self.inner.goal_scores.write().insert(goal_id, score);
    }

    /// Get a stored score for a goal.
    pub fn get_goal_score(&self, goal_id: GoalId) -> Option<PriorityScore> {
        self.inner.goal_scores.read().get(&goal_id).cloned()
    }

    /// Store a computed score for a task.
    pub fn set_task_score(&self, task_id: TaskId, score: PriorityScore) {
        self.inner.task_scores.write().insert(task_id, score);
    }

    /// Get a stored score for a task.
    pub fn get_task_score(&self, task_id: TaskId) -> Option<PriorityScore> {
        self.inner.task_scores.read().get(&task_id).cloned()
    }

    /// Resolve a conflict between two items using the configured strategy.
    pub fn resolve_conflict(&self, a_score: f64, b_score: f64, a_deadline: Option<DateTime<Utc>>, b_deadline: Option<DateTime<Utc>>) -> bool {
        let strategy = *self.inner.resolution_strategy.read();
        match strategy {
            ConflictResolution::PriorityFirst | ConflictResolution::ResourceOptimal => a_score >= b_score,
            ConflictResolution::DeadlineFirst => {
                match (a_deadline, b_deadline) {
                    (Some(a), Some(b)) => a <= b,
                    (Some(_), None) => true,
                    (None, Some(_)) => false,
                    (None, None) => a_score >= b_score,
                }
            }
            ConflictResolution::OldestFirst => a_score <= b_score,
            ConflictResolution::FairShare => a_score >= b_score,
        }
    }

    /// Set the conflict resolution strategy.
    pub fn set_resolution_strategy(&self, strategy: ConflictResolution) {
        *self.inner.resolution_strategy.write() = strategy;
    }

    /// Get the current conflict resolution strategy.
    pub fn resolution_strategy(&self) -> ConflictResolution {
        *self.inner.resolution_strategy.read()
    }

    /// Add a priority rule.
    pub fn add_rule(&self, rule: PriorityRule) {
        self.inner.rules.write().push(rule);
    }

    /// Get all active rules.
    pub fn active_rules(&self) -> Vec<PriorityRule> {
        self.inner
            .rules
            .read()
            .iter()
            .filter(|r| r.active)
            .cloned()
            .collect()
    }

    /// Remove a priority rule by name.
    pub fn remove_rule(&self, name: &str) -> bool {
        let mut rules = self.inner.rules.write();
        let len_before = rules.len();
        rules.retain(|r| r.name != name);
        rules.len() < len_before
    }

    /// Set the deadline urgency weight.
    pub fn set_deadline_urgency_weight(&self, weight: f64) {
        *self.inner.deadlines_urgency_weight.write() = weight.clamp(0.0, 1.0);
    }

    /// Set the resource sensitivity.
    pub fn set_resource_sensitivity(&self, sensitivity: f64) {
        *self.inner.resource_sensitivity.write() = sensitivity.clamp(0.0, 1.0);
    }

    /// Clear all stored scores.
    pub fn clear_scores(&self) {
        self.inner.goal_scores.write().clear();
        self.inner.task_scores.write().clear();
    }

    /// Get the number of stored goal scores.
    pub fn goal_score_count(&self) -> usize {
        self.inner.goal_scores.read().len()
    }

    /// Get the number of stored task scores.
    pub fn task_score_count(&self) -> usize {
        self.inner.task_scores.read().len()
    }
}

impl Default for PriorityEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urgency_calculation() {
        let engine = PriorityEngine::new();

        let urgent = engine.calculate_urgency(
            Some(Utc::now() + chrono::Duration::minutes(30)),
            GoalPriority::Critical,
        );
        assert!(urgent > 0.8);

        let relaxed = engine.calculate_urgency(
            Some(Utc::now() + chrono::Duration::days(7)),
            GoalPriority::Low,
        );
        assert!(relaxed < 0.5);
    }

    #[test]
    fn importance_calculation() {
        let engine = PriorityEngine::new();
        let high = engine.calculate_importance(true, 3, 5);
        let low = engine.calculate_importance(false, 0, 0);
        assert!(high > low);
    }

    #[test]
    fn resource_factor() {
        let engine = PriorityEngine::new();
        let resources = vec![ResourceAvailability {
            resource_type: ResourceType::Cpu,
            available: 8,
            total: 16,
            utilization: 0.5,
        }];

        let factor = engine.calculate_resource_factor(&resources);
        assert!(factor > 0.0 && factor <= 1.0);
    }

    #[test]
    fn age_factor() {
        let engine = PriorityEngine::new();
        let young = engine.calculate_age_factor(Utc::now());
        let old = engine.calculate_age_factor(Utc::now() - chrono::Duration::days(7));
        assert!(old > young);
    }

    #[test]
    fn score_goal() {
        let engine = PriorityEngine::new();
        let score = engine.score_goal(
            Some(Utc::now() + chrono::Duration::hours(1)),
            GoalPriority::High,
            true,
            2,
            3,
            &[],
            Utc::now() - chrono::Duration::hours(12),
        );
        assert!(score.total > 0.0 && score.total <= 1.0);
    }

    #[test]
    fn conflict_resolution() {
        let engine = PriorityEngine::new();
        assert!(engine.resolve_conflict(0.9, 0.5, None, None));
        assert!(!engine.resolve_conflict(0.5, 0.9, None, None));
    }

    #[test]
    fn rules_management() {
        let engine = PriorityEngine::new();
        engine.add_rule(PriorityRule {
            name: "test".to_string(),
            condition: "always".to_string(),
            adjustment: 0.1,
            active: true,
        });
        assert_eq!(engine.active_rules().len(), 1);
        assert!(engine.remove_rule("test"));
        assert_eq!(engine.active_rules().len(), 0);
    }

    #[test]
    fn score_storage() {
        let engine = PriorityEngine::new();
        let gid = GoalId::new();
        let score = PriorityScore::new(0.8, 0.7, 0.6, 0.5);
        engine.set_goal_score(gid, score);
        assert!(engine.get_goal_score(gid).is_some());
    }
}
