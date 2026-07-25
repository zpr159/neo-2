//! Planning analytics and metrics tracking.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// Aggregate planning analytics tracked via atomic counters.
#[derive(Debug)]
pub struct PlanningAnalytics {
    inner: Arc<PlanningAnalyticsInner>,
}

#[derive(Debug)]
struct PlanningAnalyticsInner {
    total_plans: AtomicU64,
    successful_plans: AtomicU64,
    failed_plans: AtomicU64,
    total_goals: AtomicU64,
    total_strategies_evaluated: AtomicU64,
    total_optimization_passes: AtomicU64,
    total_replans: AtomicU64,
    planning_latency_sum_ms: AtomicU64,
    planning_latency_count: AtomicU64,
    optimization_latency_sum_ms: AtomicU64,
    optimization_latency_count: AtomicU64,
}

impl PlanningAnalytics {
    /// Create a new analytics tracker with all counters at zero.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(PlanningAnalyticsInner {
                total_plans: AtomicU64::new(0),
                successful_plans: AtomicU64::new(0),
                failed_plans: AtomicU64::new(0),
                total_goals: AtomicU64::new(0),
                total_strategies_evaluated: AtomicU64::new(0),
                total_optimization_passes: AtomicU64::new(0),
                total_replans: AtomicU64::new(0),
                planning_latency_sum_ms: AtomicU64::new(0),
                planning_latency_count: AtomicU64::new(0),
                optimization_latency_sum_ms: AtomicU64::new(0),
                optimization_latency_count: AtomicU64::new(0),
            }),
        }
    }

    /// Record a plan generation event.
    pub fn record_plan_generation(&self, latency_ms: u64, success: bool) {
        self.inner.total_plans.fetch_add(1, Ordering::Relaxed);
        if success {
            self.inner.successful_plans.fetch_add(1, Ordering::Relaxed);
        } else {
            self.inner.failed_plans.fetch_add(1, Ordering::Relaxed);
        }
        self.inner
            .planning_latency_sum_ms
            .fetch_add(latency_ms, Ordering::Relaxed);
        self.inner
            .planning_latency_count
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record an optimization pass.
    pub fn record_optimization(&self, latency_ms: u64, _cost_reduction: f64) {
        self.inner
            .total_optimization_passes
            .fetch_add(1, Ordering::Relaxed);
        self.inner
            .optimization_latency_sum_ms
            .fetch_add(latency_ms, Ordering::Relaxed);
        self.inner
            .optimization_latency_count
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record a replan event.
    pub fn record_replan(&self) {
        self.inner.total_replans.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a strategy evaluation.
    pub fn record_strategy_evaluated(&self) {
        self.inner
            .total_strategies_evaluated
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Take a consistent snapshot of all counters.
    pub fn snapshot(&self) -> AnalyticsSnapshot {
        let total_plans = self.inner.total_plans.load(Ordering::Relaxed);
        let successful_plans = self.inner.successful_plans.load(Ordering::Relaxed);
        let failed_plans = self.inner.failed_plans.load(Ordering::Relaxed);
        let total_goals = self.inner.total_goals.load(Ordering::Relaxed);
        let total_strategies_evaluated = self
            .inner
            .total_strategies_evaluated
            .load(Ordering::Relaxed);
        let total_optimization_passes =
            self.inner.total_optimization_passes.load(Ordering::Relaxed);
        let total_replans = self.inner.total_replans.load(Ordering::Relaxed);
        let planning_latency_sum_ms = self.inner.planning_latency_sum_ms.load(Ordering::Relaxed);
        let planning_latency_count = self.inner.planning_latency_count.load(Ordering::Relaxed);
        let optimization_latency_sum_ms = self
            .inner
            .optimization_latency_sum_ms
            .load(Ordering::Relaxed);
        let optimization_latency_count = self
            .inner
            .optimization_latency_count
            .load(Ordering::Relaxed);

        let avg_planning_latency_ms = if planning_latency_count > 0 {
            planning_latency_sum_ms as f64 / planning_latency_count as f64
        } else {
            0.0
        };

        let avg_optimization_latency_ms = if optimization_latency_count > 0 {
            optimization_latency_sum_ms as f64 / optimization_latency_count as f64
        } else {
            0.0
        };

        let success_rate = if total_plans > 0 {
            successful_plans as f64 / total_plans as f64
        } else {
            0.0
        };

        let replan_rate = if total_plans > 0 {
            total_replans as f64 / total_plans as f64
        } else {
            0.0
        };

        AnalyticsSnapshot {
            total_plans,
            successful_plans,
            failed_plans,
            total_goals,
            total_strategies_evaluated,
            total_optimization_passes,
            total_replans,
            avg_planning_latency_ms,
            avg_optimization_latency_ms,
            success_rate,
            replan_rate,
        }
    }

    /// Reset all counters to zero.
    pub fn reset(&self) {
        self.inner.total_plans.store(0, Ordering::Relaxed);
        self.inner.successful_plans.store(0, Ordering::Relaxed);
        self.inner.failed_plans.store(0, Ordering::Relaxed);
        self.inner.total_goals.store(0, Ordering::Relaxed);
        self.inner
            .total_strategies_evaluated
            .store(0, Ordering::Relaxed);
        self.inner
            .total_optimization_passes
            .store(0, Ordering::Relaxed);
        self.inner.total_replans.store(0, Ordering::Relaxed);
        self.inner
            .planning_latency_sum_ms
            .store(0, Ordering::Relaxed);
        self.inner
            .planning_latency_count
            .store(0, Ordering::Relaxed);
        self.inner
            .optimization_latency_sum_ms
            .store(0, Ordering::Relaxed);
        self.inner
            .optimization_latency_count
            .store(0, Ordering::Relaxed);
    }
}

impl Clone for PlanningAnalytics {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl Default for PlanningAnalytics {
    fn default() -> Self {
        Self::new()
    }
}

/// A point-in-time snapshot of analytics counters.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnalyticsSnapshot {
    pub total_plans: u64,
    pub successful_plans: u64,
    pub failed_plans: u64,
    pub total_goals: u64,
    pub total_strategies_evaluated: u64,
    pub total_optimization_passes: u64,
    pub total_replans: u64,
    pub avg_planning_latency_ms: f64,
    pub avg_optimization_latency_ms: f64,
    pub success_rate: f64,
    pub replan_rate: f64,
}

/// Analytics for the optimization subsystem.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OptimizationAnalytics {
    pub pass_count: usize,
    pub total_cost_reduction: f64,
    pub total_duration_reduction_secs: i64,
    pub rules_applied: HashMap<String, usize>,
}

impl OptimizationAnalytics {
    /// Create a new empty tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a single optimization pass.
    pub fn record_pass(&mut self) {
        self.pass_count += 1;
    }

    /// Take a snapshot.
    pub fn snapshot(&self) -> OptimizationAnalyticsSnapshot {
        OptimizationAnalyticsSnapshot {
            pass_count: self.pass_count,
            total_cost_reduction: self.total_cost_reduction,
            total_duration_reduction_secs: self.total_duration_reduction_secs,
            rules_applied: self.rules_applied.clone(),
        }
    }
}

/// Snapshot of optimization analytics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OptimizationAnalyticsSnapshot {
    pub pass_count: usize,
    pub total_cost_reduction: f64,
    pub total_duration_reduction_secs: i64,
    pub rules_applied: HashMap<String, usize>,
}

/// Analytics for risk assessment.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RiskAnalytics {
    pub total_assessments: usize,
    pub risks_identified: usize,
    pub risks_mitigated: usize,
    pub avg_risk_score: f64,
}

impl RiskAnalytics {
    /// Create a new empty tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a risk assessment.
    pub fn record_assessment(&mut self) {
        self.total_assessments += 1;
    }

    /// Take a snapshot.
    pub fn snapshot(&self) -> RiskAnalyticsSnapshot {
        RiskAnalyticsSnapshot {
            total_assessments: self.total_assessments,
            risks_identified: self.risks_identified,
            risks_mitigated: self.risks_mitigated,
            avg_risk_score: self.avg_risk_score,
        }
    }
}

/// Snapshot of risk analytics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RiskAnalyticsSnapshot {
    pub total_assessments: usize,
    pub risks_identified: usize,
    pub risks_mitigated: usize,
    pub avg_risk_score: f64,
}

/// Analytics for strategy generation and selection.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StrategyAnalytics {
    pub strategies_generated: usize,
    pub strategies_selected: usize,
    pub avg_evaluation_score: f64,
}

impl StrategyAnalytics {
    /// Create a new empty tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a strategy generation event.
    pub fn record_generation(&mut self) {
        self.strategies_generated += 1;
    }

    /// Record a strategy selection event.
    pub fn record_selection(&mut self) {
        self.strategies_selected += 1;
    }

    /// Take a snapshot.
    pub fn snapshot(&self) -> StrategyAnalyticsSnapshot {
        StrategyAnalyticsSnapshot {
            strategies_generated: self.strategies_generated,
            strategies_selected: self.strategies_selected,
            avg_evaluation_score: self.avg_evaluation_score,
        }
    }
}

/// Snapshot of strategy analytics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StrategyAnalyticsSnapshot {
    pub strategies_generated: usize,
    pub strategies_selected: usize,
    pub avg_evaluation_score: f64,
}

/// Analytics for task execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionAnalytics {
    pub tasks_executed: usize,
    pub tasks_succeeded: usize,
    pub tasks_failed: usize,
    pub avg_task_duration_ms: f64,
}

impl ExecutionAnalytics {
    /// Create a new empty tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a task execution event.
    pub fn record_task_execution(&mut self, duration_ms: u64, success: bool) {
        self.tasks_executed += 1;
        if success {
            self.tasks_succeeded += 1;
        } else {
            self.tasks_failed += 1;
        }
        let total = self.tasks_executed as f64;
        let prev_total_duration = self.avg_task_duration_ms * (total - 1.0);
        self.avg_task_duration_ms = (prev_total_duration + duration_ms as f64) / total;
    }

    /// Take a snapshot.
    pub fn snapshot(&self) -> ExecutionAnalyticsSnapshot {
        ExecutionAnalyticsSnapshot {
            tasks_executed: self.tasks_executed,
            tasks_succeeded: self.tasks_succeeded,
            tasks_failed: self.tasks_failed,
            avg_task_duration_ms: self.avg_task_duration_ms,
        }
    }
}

/// Snapshot of execution analytics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionAnalyticsSnapshot {
    pub tasks_executed: usize,
    pub tasks_succeeded: usize,
    pub tasks_failed: usize,
    pub avg_task_duration_ms: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planning_analytics_new() {
        let a = PlanningAnalytics::new();
        let snap = a.snapshot();
        assert_eq!(snap.total_plans, 0);
        assert_eq!(snap.successful_plans, 0);
        assert_eq!(snap.failed_plans, 0);
        assert_eq!(snap.success_rate, 0.0);
    }

    #[test]
    fn planning_analytics_record_generation() {
        let a = PlanningAnalytics::new();
        a.record_plan_generation(100, true);
        a.record_plan_generation(200, false);
        let snap = a.snapshot();
        assert_eq!(snap.total_plans, 2);
        assert_eq!(snap.successful_plans, 1);
        assert_eq!(snap.failed_plans, 1);
        assert!((snap.avg_planning_latency_ms - 150.0).abs() < f64::EPSILON);
        assert!((snap.success_rate - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn planning_analytics_record_optimization() {
        let a = PlanningAnalytics::new();
        a.record_optimization(50, 10.0);
        a.record_optimization(30, 5.0);
        let snap = a.snapshot();
        assert_eq!(snap.total_optimization_passes, 2);
        assert!((snap.avg_optimization_latency_ms - 40.0).abs() < f64::EPSILON);
    }

    #[test]
    fn planning_analytics_record_replan() {
        let a = PlanningAnalytics::new();
        a.record_plan_generation(100, true);
        a.record_plan_generation(100, true);
        a.record_replan();
        let snap = a.snapshot();
        assert_eq!(snap.total_replans, 1);
        assert!((snap.replan_rate - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn planning_analytics_record_strategy() {
        let a = PlanningAnalytics::new();
        a.record_strategy_evaluated();
        a.record_strategy_evaluated();
        a.record_strategy_evaluated();
        let snap = a.snapshot();
        assert_eq!(snap.total_strategies_evaluated, 3);
    }

    #[test]
    fn planning_analytics_reset() {
        let a = PlanningAnalytics::new();
        a.record_plan_generation(100, true);
        a.record_replan();
        a.reset();
        let snap = a.snapshot();
        assert_eq!(snap.total_plans, 0);
        assert_eq!(snap.total_replans, 0);
    }

    #[test]
    fn planning_analytics_clone() {
        let a = PlanningAnalytics::new();
        a.record_plan_generation(100, true);
        let b = a.clone();
        let snap = b.snapshot();
        assert_eq!(snap.total_plans, 1);
    }

    #[test]
    fn planning_analytics_default() {
        let a = PlanningAnalytics::default();
        let snap = a.snapshot();
        assert_eq!(snap.total_plans, 0);
    }

    #[test]
    fn optimization_analytics() {
        let mut a = OptimizationAnalytics::new();
        a.record_pass();
        a.record_pass();
        a.rules_applied.insert("rule1".to_string(), 3);
        let snap = a.snapshot();
        assert_eq!(snap.pass_count, 2);
        assert_eq!(snap.rules_applied.get("rule1").unwrap(), &3);
    }

    #[test]
    fn optimization_analytics_default() {
        let a = OptimizationAnalytics::default();
        assert_eq!(a.pass_count, 0);
    }

    #[test]
    fn risk_analytics() {
        let mut a = RiskAnalytics::new();
        a.record_assessment();
        a.risks_identified = 5;
        a.risks_mitigated = 3;
        a.avg_risk_score = 0.42;
        let snap = a.snapshot();
        assert_eq!(snap.total_assessments, 1);
        assert_eq!(snap.risks_identified, 5);
        assert_eq!(snap.risks_mitigated, 3);
        assert!((snap.avg_risk_score - 0.42).abs() < f64::EPSILON);
    }

    #[test]
    fn strategy_analytics() {
        let mut a = StrategyAnalytics::new();
        a.record_generation();
        a.record_generation();
        a.record_selection();
        a.avg_evaluation_score = 0.85;
        let snap = a.snapshot();
        assert_eq!(snap.strategies_generated, 2);
        assert_eq!(snap.strategies_selected, 1);
        assert!((snap.avg_evaluation_score - 0.85).abs() < f64::EPSILON);
    }

    #[test]
    fn execution_analytics() {
        let mut a = ExecutionAnalytics::new();
        a.record_task_execution(100, true);
        a.record_task_execution(200, false);
        a.record_task_execution(300, true);
        let snap = a.snapshot();
        assert_eq!(snap.tasks_executed, 3);
        assert_eq!(snap.tasks_succeeded, 2);
        assert_eq!(snap.tasks_failed, 1);
        assert!((snap.avg_task_duration_ms - 200.0).abs() < f64::EPSILON);
    }

    #[test]
    fn execution_analytics_empty() {
        let a = ExecutionAnalytics::new();
        let snap = a.snapshot();
        assert_eq!(snap.tasks_executed, 0);
        assert_eq!(snap.avg_task_duration_ms, 0.0);
    }

    #[test]
    fn snapshot_serialization_roundtrip() {
        let mut a = PlanningAnalytics::new();
        a.record_plan_generation(100, true);
        a.record_replan();
        let snap = a.snapshot();
        let json = serde_json::to_string(&snap).unwrap();
        let back: AnalyticsSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(back.total_plans, 1);
        assert_eq!(back.total_replans, 1);
    }

    #[test]
    fn execution_analytics_serialization_roundtrip() {
        let mut a = ExecutionAnalytics::new();
        a.record_task_execution(50, true);
        let snap = a.snapshot();
        let json = serde_json::to_string(&snap).unwrap();
        let back: ExecutionAnalyticsSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(back.tasks_succeeded, 1);
    }
}
