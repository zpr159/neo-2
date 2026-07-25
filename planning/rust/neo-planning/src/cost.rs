//! Cost estimation and tracking for the planning system.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::{PlanningError, PlanningResult};
use crate::id::{CostEstimateId, PlanningNodeId};
use crate::types::{ExecutionBudget, ResourceRequirements};

/// Detailed breakdown of costs for a task or plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostBreakdown {
    pub execution_time_secs: f64,
    pub cpu_units: u32,
    pub memory_mb: u64,
    pub tool_invocations: u32,
    pub network_cost: f64,
    pub cloud_cost: f64,
    pub token_usage: u64,
    pub energy_consumption_wh: f64,
    pub custom_costs: HashMap<String, f64>,
}

impl Default for CostBreakdown {
    fn default() -> Self {
        Self {
            execution_time_secs: 0.0,
            cpu_units: 0,
            memory_mb: 0,
            tool_invocations: 0,
            network_cost: 0.0,
            cloud_cost: 0.0,
            token_usage: 0,
            energy_consumption_wh: 0.0,
            custom_costs: HashMap::new(),
        }
    }
}

impl CostBreakdown {
    pub fn new() -> Self {
        Self::default()
    }

    /// Compute the total monetary cost from all cost components.
    pub fn total_cost(&self) -> f64 {
        let base = self.network_cost + self.cloud_cost;
        self.custom_costs.values().fold(base, |acc, v| acc + v)
    }

    pub fn add_custom(mut self, name: impl Into<String>, cost: f64) -> Self {
        self.custom_costs.insert(name.into(), cost);
        self
    }

    pub fn with_execution_time(mut self, secs: f64) -> Self {
        self.execution_time_secs = secs;
        self
    }

    pub fn with_cpu(mut self, units: u32) -> Self {
        self.cpu_units = units;
        self
    }

    pub fn with_memory(mut self, mb: u64) -> Self {
        self.memory_mb = mb;
        self
    }

    pub fn with_tool_invocations(mut self, count: u32) -> Self {
        self.tool_invocations = count;
        self
    }

    pub fn with_network_cost(mut self, cost: f64) -> Self {
        self.network_cost = cost;
        self
    }

    pub fn with_cloud_cost(mut self, cost: f64) -> Self {
        self.cloud_cost = cost;
        self
    }

    pub fn with_token_usage(mut self, tokens: u64) -> Self {
        self.token_usage = tokens;
        self
    }

    pub fn with_energy(mut self, wh: f64) -> Self {
        self.energy_consumption_wh = wh;
        self
    }
}

/// A cost estimate produced by the cost model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimate {
    pub id: CostEstimateId,
    pub node_id: Option<PlanningNodeId>,
    pub breakdown: CostBreakdown,
    pub confidence: f64,
    pub methodology: String,
    pub created_at: DateTime<Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl CostEstimate {
    pub fn new(methodology: impl Into<String>) -> Self {
        Self {
            id: CostEstimateId::new(),
            node_id: None,
            breakdown: CostBreakdown::new(),
            confidence: 1.0,
            methodology: methodology.into(),
            created_at: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_node_id(mut self, node_id: PlanningNodeId) -> Self {
        self.node_id = Some(node_id);
        self
    }

    pub fn with_breakdown(mut self, breakdown: CostBreakdown) -> Self {
        self.breakdown = breakdown;
        self
    }

    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Comparison result of an estimate against a budget.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostComparison {
    pub within_budget: bool,
    pub utilization_pct: f64,
    pub overage: f64,
    pub recommendation: String,
}

/// Cost estimation engine.
#[derive(Debug, Clone)]
pub struct CostModel;

impl CostModel {
    pub fn new() -> Self {
        Self
    }

    /// Estimate cost for a single task.
    pub fn estimate_task_cost(
        &self,
        task_cost: f64,
        duration_secs: u64,
        resources: &ResourceRequirements,
    ) -> CostEstimate {
        let breakdown = CostBreakdown::new()
            .with_execution_time(duration_secs as f64)
            .with_cpu(resources.cpu_units)
            .with_memory(resources.memory_mb)
            .with_tool_invocations(resources.tool_requirements.len() as u32);

        let mut estimate = CostEstimate::new("task_level")
            .with_breakdown(breakdown)
            .with_confidence(0.8);
        let mut meta = HashMap::new();
        meta.insert("base_task_cost".to_string(), serde_json::json!(task_cost));
        estimate.metadata = meta;
        estimate
    }

    /// Aggregate sequential task costs into a plan-level estimate.
    pub fn estimate_plan_cost(&self, task_costs: &[CostEstimate]) -> CostEstimate {
        if task_costs.is_empty() {
            return CostEstimate::new("empty_plan");
        }

        let total: f64 = task_costs.iter().map(|e| e.breakdown.total_cost()).sum();
        let avg_confidence: f64 =
            task_costs.iter().map(|e| e.confidence).sum::<f64>() / task_costs.len() as f64;

        let breakdown = CostBreakdown::new().with_cloud_cost(total);
        CostEstimate::new("plan_aggregate")
            .with_breakdown(breakdown)
            .with_confidence(avg_confidence)
    }

    /// Estimate cost for parallel task groups (takes max cost per group).
    pub fn estimate_parallel_cost(&self, parallel_groups: &[Vec<CostEstimate>]) -> CostEstimate {
        if parallel_groups.is_empty() {
            return CostEstimate::new("empty_parallel");
        }

        let total: f64 = parallel_groups
            .iter()
            .map(|group| {
                group
                    .iter()
                    .map(|e| e.breakdown.total_cost())
                    .fold(0.0f64, f64::max)
            })
            .sum();

        let breakdown = CostBreakdown::new().with_cloud_cost(total);
        CostEstimate::new("parallel_aggregate").with_breakdown(breakdown)
    }

    /// Compare an estimate against an execution budget.
    pub fn compare_to_budget(
        &self,
        estimate: &CostEstimate,
        budget: &ExecutionBudget,
    ) -> CostComparison {
        let cost = estimate.breakdown.total_cost();
        let max = budget.max_cost;
        let within = cost <= max;
        let utilization = if max > 0.0 { (cost / max) * 100.0 } else { 0.0 };
        let overage = if cost > max { cost - max } else { 0.0 };
        let recommendation = if within {
            format!("Within budget ({:.1}% utilization)", utilization)
        } else {
            format!("Over budget by {:.2} — consider reducing scope", overage)
        };

        CostComparison {
            within_budget: within,
            utilization_pct: utilization,
            overage,
            recommendation,
        }
    }
}

/// Trend direction for cost estimates over time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CostTrend {
    Improving,
    Stable,
    Worsening,
    InsufficientData,
}

/// Thread-safe history of cost estimates.
#[derive(Debug)]
pub struct CostHistory {
    estimates: std::sync::Mutex<Vec<CostEstimate>>,
}

impl Clone for CostHistory {
    fn clone(&self) -> Self {
        let data = self
            .estimates
            .lock()
            .expect("cost history lock poisoned")
            .clone();
        Self {
            estimates: std::sync::Mutex::new(data),
        }
    }
}

impl CostHistory {
    pub fn new() -> Self {
        Self {
            estimates: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Record a new cost estimate.
    pub fn record(&self, estimate: CostEstimate) {
        self.estimates
            .lock()
            .expect("cost history lock poisoned")
            .push(estimate);
    }

    /// Return the N most recent estimates.
    pub fn recent_estimates(&self, n: usize) -> Vec<CostEstimate> {
        let guard = self.estimates.lock().expect("cost history lock poisoned");
        let start = guard.len().saturating_sub(n);
        guard[start..].to_vec()
    }

    /// Compute the average cost across all recorded estimates.
    pub fn average_cost(&self) -> f64 {
        let guard = self.estimates.lock().expect("cost history lock poisoned");
        if guard.is_empty() {
            return 0.0;
        }
        let sum: f64 = guard.iter().map(|e| e.breakdown.total_cost()).sum();
        sum / guard.len() as f64
    }

    /// Determine the cost trend over recorded estimates.
    pub fn cost_trend(&self) -> CostTrend {
        let guard = self.estimates.lock().expect("cost history lock poisoned");
        if guard.len() < 3 {
            return CostTrend::InsufficientData;
        }

        let len = guard.len();
        let third = len / 3;
        let avg_slice = |start: usize, end: usize| -> f64 {
            let slice = &guard[start..end];
            let sum: f64 = slice.iter().map(|e| e.breakdown.total_cost()).sum();
            sum / slice.len() as f64
        };

        let first = avg_slice(0, third);
        let last = avg_slice(len - third, len);
        let delta = last - first;
        let threshold = first.abs().max(last.abs()) * 0.1;

        if delta < -threshold {
            CostTrend::Improving
        } else if delta > threshold {
            CostTrend::Worsening
        } else {
            CostTrend::Stable
        }
    }
}

impl Default for CostHistory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cost_breakdown_defaults() {
        let cb = CostBreakdown::default();
        assert_eq!(cb.execution_time_secs, 0.0);
        assert!(cb.custom_costs.is_empty());
    }

    #[test]
    fn cost_breakdown_total_cost() {
        let cb = CostBreakdown::new()
            .with_network_cost(10.0)
            .with_cloud_cost(20.0)
            .add_custom("license", 5.0);
        assert!((cb.total_cost() - 35.0).abs() < f64::EPSILON);
    }

    #[test]
    fn cost_breakdown_builder_chain() {
        let cb = CostBreakdown::new()
            .with_execution_time(60.0)
            .with_cpu(4)
            .with_memory(1024)
            .with_tool_invocations(12)
            .with_network_cost(1.5)
            .with_cloud_cost(3.0)
            .with_token_usage(5000)
            .with_energy(42.0);
        assert!((cb.execution_time_secs - 60.0).abs() < f64::EPSILON);
        assert_eq!(cb.cpu_units, 4);
        assert_eq!(cb.memory_mb, 1024);
        assert_eq!(cb.tool_invocations, 12);
        assert_eq!(cb.token_usage, 5000);
    }

    #[test]
    fn cost_estimate_builder() {
        let est = CostEstimate::new("heuristic")
            .with_confidence(0.9)
            .with_breakdown(CostBreakdown::new().with_cloud_cost(50.0));
        assert_eq!(est.methodology, "heuristic");
        assert!((est.confidence - 0.9).abs() < f64::EPSILON);
        assert!((est.breakdown.total_cost() - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn cost_estimate_confidence_clamped() {
        let est = CostEstimate::new("test").with_confidence(2.0);
        assert!((est.confidence - 1.0).abs() < f64::EPSILON);

        let est = CostEstimate::new("test").with_confidence(-1.0);
        assert!((est.confidence - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn cost_model_estimate_task() {
        let model = CostModel::new();
        let res = ResourceRequirements {
            cpu_units: 2,
            memory_mb: 512,
            tool_requirements: vec!["bash".into()],
            ..Default::default()
        };
        let est = model.estimate_task_cost(10.0, 120, &res);
        assert_eq!(est.breakdown.cpu_units, 2);
        assert_eq!(est.breakdown.memory_mb, 512);
        assert_eq!(est.breakdown.tool_invocations, 1);
        assert!((est.breakdown.execution_time_secs - 120.0).abs() < f64::EPSILON);
    }

    #[test]
    fn cost_model_plan_cost_aggregate() {
        let model = CostModel::new();
        let e1 = CostEstimate::new("a").with_breakdown(CostBreakdown::new().with_cloud_cost(10.0));
        let e2 = CostEstimate::new("b").with_breakdown(CostBreakdown::new().with_cloud_cost(20.0));
        let plan = model.estimate_plan_cost(&[e1, e2]);
        assert!((plan.breakdown.total_cost() - 30.0).abs() < f64::EPSILON);
    }

    #[test]
    fn cost_model_plan_cost_empty() {
        let model = CostModel::new();
        let plan = model.estimate_plan_cost(&[]);
        assert_eq!(plan.methodology, "empty_plan");
    }

    #[test]
    fn cost_model_parallel_cost() {
        let model = CostModel::new();
        let g1 = vec![
            CostEstimate::new("a").with_breakdown(CostBreakdown::new().with_cloud_cost(10.0)),
            CostEstimate::new("b").with_breakdown(CostBreakdown::new().with_cloud_cost(30.0)),
        ];
        let g2 =
            vec![CostEstimate::new("c").with_breakdown(CostBreakdown::new().with_cloud_cost(20.0))];
        let est = model.estimate_parallel_cost(&[g1, g2]);
        // max of group 1 (30) + max of group 2 (20) = 50
        assert!((est.breakdown.total_cost() - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn cost_model_compare_within_budget() {
        let model = CostModel::new();
        let est = CostEstimate::new("x").with_breakdown(CostBreakdown::new().with_cloud_cost(80.0));
        let budget = ExecutionBudget {
            max_cost: 100.0,
            ..Default::default()
        };
        let cmp = model.compare_to_budget(&est, &budget);
        assert!(cmp.within_budget);
        assert!((cmp.utilization_pct - 80.0).abs() < f64::EPSILON);
        assert!((cmp.overage).abs() < f64::EPSILON);
    }

    #[test]
    fn cost_model_compare_over_budget() {
        let model = CostModel::new();
        let est =
            CostEstimate::new("x").with_breakdown(CostBreakdown::new().with_cloud_cost(150.0));
        let budget = ExecutionBudget {
            max_cost: 100.0,
            ..Default::default()
        };
        let cmp = model.compare_to_budget(&est, &budget);
        assert!(!cmp.within_budget);
        assert!((cmp.overage - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn cost_history_average() {
        let history = CostHistory::new();
        history.record(
            CostEstimate::new("a").with_breakdown(CostBreakdown::new().with_cloud_cost(10.0)),
        );
        history.record(
            CostEstimate::new("b").with_breakdown(CostBreakdown::new().with_cloud_cost(30.0)),
        );
        assert!((history.average_cost() - 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn cost_history_recent() {
        let history = CostHistory::new();
        for i in 0..5 {
            history.record(
                CostEstimate::new(format!("t{}", i))
                    .with_breakdown(CostBreakdown::new().with_cloud_cost(i as f64)),
            );
        }
        let recent = history.recent_estimates(2);
        assert_eq!(recent.len(), 2);
    }

    #[test]
    fn cost_history_empty() {
        let history = CostHistory::new();
        assert_eq!(history.average_cost(), 0.0);
        assert_eq!(history.cost_trend(), CostTrend::InsufficientData);
    }

    #[test]
    fn cost_history_trend_stable() {
        let history = CostHistory::new();
        for _ in 0..5 {
            history.record(
                CostEstimate::new("x").with_breakdown(CostBreakdown::new().with_cloud_cost(100.0)),
            );
        }
        assert_eq!(history.cost_trend(), CostTrend::Stable);
    }

    #[test]
    fn cost_history_trend_worsening() {
        let history = CostHistory::new();
        for i in 0..6 {
            history.record(
                CostEstimate::new(format!("t{}", i))
                    .with_breakdown(CostBreakdown::new().with_cloud_cost(i as f64 * 100.0)),
            );
        }
        assert_eq!(history.cost_trend(), CostTrend::Worsening);
    }

    #[test]
    fn cost_history_trend_improving() {
        let history = CostHistory::new();
        for i in 0..6 {
            let cost = 600.0 - (i as f64 * 100.0);
            history.record(
                CostEstimate::new(format!("t{}", i))
                    .with_breakdown(CostBreakdown::new().with_cloud_cost(cost)),
            );
        }
        assert_eq!(history.cost_trend(), CostTrend::Improving);
    }
}
