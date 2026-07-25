use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::core::{
    ExecutionId, NodeId, NodeState, WorkflowId, WorkflowResultOutput, WorkflowState,
};
use crate::error::WorkflowResult;

/// Analytics for a single node execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeAnalytics {
    pub node_id: NodeId,
    pub name: String,
    pub state: NodeState,
    pub duration_ms: u64,
    pub retries: u32,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}

/// Analytics for a complete workflow execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionAnalytics {
    pub execution_id: ExecutionId,
    pub workflow_id: WorkflowId,
    pub state: WorkflowState,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: u64,
    pub total_nodes: usize,
    pub completed_nodes: usize,
    pub failed_nodes: usize,
    pub skipped_nodes: usize,
    pub total_retries: u32,
    pub node_analytics: Vec<NodeAnalytics>,
    pub result: Option<WorkflowResultOutput>,
}

impl ExecutionAnalytics {
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.total_nodes
    }

    #[must_use]
    pub fn success_rate(&self) -> f64 {
        if self.total_nodes == 0 {
            return 0.0;
        }
        self.completed_nodes as f64 / self.total_nodes as f64
    }

    #[must_use]
    pub fn average_node_duration_ms(&self) -> f64 {
        if self.node_analytics.is_empty() {
            return 0.0;
        }
        let total: u64 = self.node_analytics.iter().map(|n| n.duration_ms).sum();
        total as f64 / self.node_analytics.len() as f64
    }

    #[must_use]
    pub fn slowest_node(&self) -> Option<&NodeAnalytics> {
        self.node_analytics.iter().max_by_key(|n| n.duration_ms)
    }

    #[must_use]
    pub fn fastest_node(&self) -> Option<&NodeAnalytics> {
        self.node_analytics
            .iter()
            .filter(|n| n.duration_ms > 0)
            .min_by_key(|n| n.duration_ms)
    }

    #[must_use]
    pub fn failed_nodes_list(&self) -> Vec<&NodeAnalytics> {
        self.node_analytics
            .iter()
            .filter(|n| n.state == NodeState::Failed)
            .collect()
    }
}

/// Aggregated analytics across multiple workflow executions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowAnalytics {
    pub workflow_id: WorkflowId,
    pub total_executions: usize,
    pub successful_executions: usize,
    pub failed_executions: usize,
    pub average_duration_ms: f64,
    pub average_success_rate: f64,
    pub total_retries: u32,
    pub execution_history: Vec<ExecutionAnalytics>,
}

impl WorkflowAnalytics {
    #[must_use]
    pub fn new(workflow_id: WorkflowId) -> Self {
        Self {
            workflow_id,
            total_executions: 0,
            successful_executions: 0,
            failed_executions: 0,
            average_duration_ms: 0.0,
            average_success_rate: 0.0,
            total_retries: 0,
            execution_history: Vec::new(),
        }
    }

    pub fn record_execution(&mut self, analytics: ExecutionAnalytics) {
        self.total_executions += 1;
        if analytics.state == WorkflowState::Completed {
            self.successful_executions += 1;
        } else {
            self.failed_executions += 1;
        }
        self.total_retries += analytics.total_retries;

        // Update averages
        let n = self.total_executions as f64;
        self.average_duration_ms =
            (self.average_duration_ms * (n - 1.0) + analytics.duration_ms as f64) / n;
        self.average_success_rate =
            (self.average_success_rate * (n - 1.0) + analytics.success_rate()) / n;

        self.execution_history.push(analytics);
    }

    #[must_use]
    pub fn overall_success_rate(&self) -> f64 {
        if self.total_executions == 0 {
            return 0.0;
        }
        self.successful_executions as f64 / self.total_executions as f64
    }

    #[must_use]
    pub fn recent_executions(&self, count: usize) -> &[ExecutionAnalytics] {
        let start = self.execution_history.len().saturating_sub(count);
        &self.execution_history[start..]
    }
}

/// Analytics collector that builds analytics during execution.
#[derive(Debug)]
pub struct AnalyticsCollector {
    node_start_times: std::collections::HashMap<NodeId, DateTime<Utc>>,
    node_end_times: std::collections::HashMap<NodeId, DateTime<Utc>>,
    node_states: std::collections::HashMap<NodeId, NodeState>,
    node_names: std::collections::HashMap<NodeId, String>,
    node_retries: std::collections::HashMap<NodeId, u32>,
}

impl AnalyticsCollector {
    #[must_use]
    pub fn new() -> Self {
        Self {
            node_start_times: std::collections::HashMap::new(),
            node_end_times: std::collections::HashMap::new(),
            node_states: std::collections::HashMap::new(),
            node_names: std::collections::HashMap::new(),
            node_retries: std::collections::HashMap::new(),
        }
    }

    pub fn register_node(&mut self, node_id: NodeId, name: String) {
        self.node_names.insert(node_id, name);
        self.node_states.insert(node_id, NodeState::Pending);
    }

    pub fn record_node_start(&mut self, node_id: NodeId) {
        self.node_start_times.insert(node_id, Utc::now());
        self.node_states.insert(node_id, NodeState::Running);
    }

    pub fn record_node_complete(&mut self, node_id: NodeId) {
        self.node_end_times.insert(node_id, Utc::now());
        self.node_states.insert(node_id, NodeState::Completed);
    }

    pub fn record_node_failure(&mut self, node_id: NodeId) {
        self.node_end_times.insert(node_id, Utc::now());
        self.node_states.insert(node_id, NodeState::Failed);
    }

    pub fn record_retry(&mut self, node_id: NodeId) {
        *self.node_retries.entry(node_id).or_insert(0) += 1;
    }

    #[must_use]
    pub fn build_analytics(
        &self,
        execution_id: ExecutionId,
        workflow_id: WorkflowId,
        state: WorkflowState,
        started_at: Option<DateTime<Utc>>,
        completed_at: Option<DateTime<Utc>>,
    ) -> ExecutionAnalytics {
        let node_analytics: Vec<NodeAnalytics> = self
            .node_names
            .iter()
            .map(|(node_id, name)| {
                let start = self.node_start_times.get(node_id).copied();
                let end = self.node_end_times.get(node_id).copied();
                let duration = match (start, end) {
                    (Some(s), Some(e)) => (e - s).num_milliseconds().max(0) as u64,
                    _ => 0,
                };
                NodeAnalytics {
                    node_id: *node_id,
                    name: name.clone(),
                    state: self
                        .node_states
                        .get(node_id)
                        .copied()
                        .unwrap_or(NodeState::Pending),
                    duration_ms: duration,
                    retries: self.node_retries.get(node_id).copied().unwrap_or(0),
                    start_time: start,
                    end_time: end,
                }
            })
            .collect();

        let total_nodes = node_analytics.len();
        let completed_nodes = node_analytics
            .iter()
            .filter(|n| n.state == NodeState::Completed)
            .count();
        let failed_nodes = node_analytics
            .iter()
            .filter(|n| n.state == NodeState::Failed)
            .count();
        let skipped_nodes = node_analytics
            .iter()
            .filter(|n| n.state == NodeState::Skipped)
            .count();
        let total_retries: u32 = node_analytics.iter().map(|n| n.retries).sum();

        let duration_ms = match (started_at, completed_at) {
            (Some(s), Some(e)) => (e - s).num_milliseconds().max(0) as u64,
            (Some(s), None) => (Utc::now() - s).num_milliseconds().max(0) as u64,
            _ => 0,
        };

        ExecutionAnalytics {
            execution_id,
            workflow_id,
            state,
            started_at,
            completed_at,
            duration_ms,
            total_nodes,
            completed_nodes,
            failed_nodes,
            skipped_nodes,
            total_retries,
            node_analytics,
            result: None,
        }
    }
}

impl Default for AnalyticsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn execution_analytics_success_rate() {
        let analytics = ExecutionAnalytics {
            execution_id: ExecutionId::new(),
            workflow_id: WorkflowId::new(),
            state: WorkflowState::Completed,
            started_at: None,
            completed_at: None,
            duration_ms: 1000,
            total_nodes: 10,
            completed_nodes: 8,
            failed_nodes: 1,
            skipped_nodes: 1,
            total_retries: 2,
            node_analytics: vec![],
            result: None,
        };
        assert!((analytics.success_rate() - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn workflow_analytics_aggregation() {
        let wf_id = WorkflowId::new();
        let mut wa = WorkflowAnalytics::new(wf_id);

        let a1 = ExecutionAnalytics {
            execution_id: ExecutionId::new(),
            workflow_id: wf_id,
            state: WorkflowState::Completed,
            started_at: None,
            completed_at: None,
            duration_ms: 100,
            total_nodes: 5,
            completed_nodes: 5,
            failed_nodes: 0,
            skipped_nodes: 0,
            total_retries: 0,
            node_analytics: vec![],
            result: None,
        };
        wa.record_execution(a1);
        assert_eq!(wa.total_executions, 1);
        assert_eq!(wa.successful_executions, 1);
        assert!((wa.overall_success_rate() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn analytics_collector() {
        let mut collector = AnalyticsCollector::new();
        let nid = NodeId::new();
        collector.register_node(nid, "test".into());
        collector.record_node_start(nid);
        collector.record_node_complete(nid);

        let analytics = collector.build_analytics(
            ExecutionId::new(),
            WorkflowId::new(),
            WorkflowState::Completed,
            Some(Utc::now() - chrono::Duration::seconds(1)),
            Some(Utc::now()),
        );
        assert_eq!(analytics.total_nodes, 1);
        assert_eq!(analytics.completed_nodes, 1);
    }

    #[test]
    fn slowest_fastest_node() {
        let analytics = ExecutionAnalytics {
            execution_id: ExecutionId::new(),
            workflow_id: WorkflowId::new(),
            state: WorkflowState::Completed,
            started_at: None,
            completed_at: None,
            duration_ms: 0,
            total_nodes: 3,
            completed_nodes: 3,
            failed_nodes: 0,
            skipped_nodes: 0,
            total_retries: 0,
            node_analytics: vec![
                NodeAnalytics {
                    node_id: NodeId::new(),
                    name: "a".into(),
                    state: NodeState::Completed,
                    duration_ms: 100,
                    retries: 0,
                    start_time: None,
                    end_time: None,
                },
                NodeAnalytics {
                    node_id: NodeId::new(),
                    name: "b".into(),
                    state: NodeState::Completed,
                    duration_ms: 50,
                    retries: 0,
                    start_time: None,
                    end_time: None,
                },
                NodeAnalytics {
                    node_id: NodeId::new(),
                    name: "c".into(),
                    state: NodeState::Completed,
                    duration_ms: 200,
                    retries: 0,
                    start_time: None,
                    end_time: None,
                },
            ],
            result: None,
        };
        assert_eq!(analytics.slowest_node().unwrap().name, "c");
        assert_eq!(analytics.fastest_node().unwrap().name, "b");
    }
}
