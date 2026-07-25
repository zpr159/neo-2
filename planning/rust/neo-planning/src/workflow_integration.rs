//! Workflow synthesis from plans.
//!
//! Converts [`Plan`] and [`PlanGraph`] representations into a concrete
//! workflow definition suitable for execution by the workflow engine.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::{PlanningError, PlanningResult};
use crate::graph::{PlanGraph, PlanningNodeType};
use crate::id::{PlanId, PlanningNodeId};
use crate::plan::Plan;
use crate::types::ResourceRequirements;

// ---------------------------------------------------------------------------
// WorkflowNodeType
// ---------------------------------------------------------------------------

/// The kind of workflow node produced by synthesis.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkflowNodeType {
    Task,
    Gate,
    ParallelSplit,
    ParallelJoin,
    Decision,
    Event,
    SubWorkflow,
}

impl std::fmt::Display for WorkflowNodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Task => write!(f, "task"),
            Self::Gate => write!(f, "gate"),
            Self::ParallelSplit => write!(f, "parallel_split"),
            Self::ParallelJoin => write!(f, "parallel_join"),
            Self::Decision => write!(f, "decision"),
            Self::Event => write!(f, "event"),
            Self::SubWorkflow => write!(f, "sub_workflow"),
        }
    }
}

// ---------------------------------------------------------------------------
// WorkflowNodeDef
// ---------------------------------------------------------------------------

/// A single node in a synthesised workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNodeDef {
    pub id: PlanningNodeId,
    pub name: String,
    pub node_type: WorkflowNodeType,
    pub description: String,
    pub estimated_cost: f64,
    pub estimated_duration_secs: u64,
    pub resource_requirements: ResourceRequirements,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

impl WorkflowNodeDef {
    /// Create a new workflow node definition.
    pub fn new(id: PlanningNodeId, name: impl Into<String>, node_type: WorkflowNodeType) -> Self {
        Self {
            id,
            name: name.into(),
            node_type,
            description: String::new(),
            estimated_cost: 0.0,
            estimated_duration_secs: 0,
            resource_requirements: ResourceRequirements::default(),
            metadata: HashMap::new(),
            created_at: Utc::now(),
        }
    }

    /// Set the description.
    #[must_use]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set the estimated cost.
    #[must_use]
    pub fn with_cost(mut self, cost: f64) -> Self {
        self.estimated_cost = cost;
        self
    }

    /// Set the estimated duration.
    #[must_use]
    pub fn with_duration(mut self, secs: u64) -> Self {
        self.estimated_duration_secs = secs;
        self
    }

    /// Set resource requirements.
    #[must_use]
    pub fn with_resources(mut self, req: ResourceRequirements) -> Self {
        self.resource_requirements = req;
        self
    }

    /// Add metadata.
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

// ---------------------------------------------------------------------------
// WorkflowEdgeDef
// ---------------------------------------------------------------------------

/// A directed edge in a synthesised workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEdgeDef {
    pub from: PlanningNodeId,
    pub to: PlanningNodeId,
    pub label: Option<String>,
    pub condition: Option<String>,
}

impl WorkflowEdgeDef {
    /// Create a new edge.
    pub fn new(from: PlanningNodeId, to: PlanningNodeId) -> Self {
        Self {
            from,
            to,
            label: None,
            condition: None,
        }
    }

    /// Set the label.
    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set the condition.
    #[must_use]
    pub fn with_condition(mut self, condition: impl Into<String>) -> Self {
        self.condition = Some(condition.into());
        self
    }
}

// ---------------------------------------------------------------------------
// RollbackStrategy
// ---------------------------------------------------------------------------

/// Strategy for rolling back a workflow on failure.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RollbackStrategy {
    /// Undo completed nodes in reverse order.
    ReverseOrder,
    /// Jump back to the most recent checkpoint.
    ToLastCheckpoint,
    /// Abort without rollback.
    NoRollback,
    /// Custom rollback defined by name.
    Custom(String),
}

impl Default for RollbackStrategy {
    fn default() -> Self {
        Self::ReverseOrder
    }
}

impl std::fmt::Display for RollbackStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReverseOrder => write!(f, "reverse_order"),
            Self::ToLastCheckpoint => write!(f, "to_last_checkpoint"),
            Self::NoRollback => write!(f, "no_rollback"),
            Self::Custom(name) => write!(f, "custom({})", name),
        }
    }
}

// ---------------------------------------------------------------------------
// CheckpointPlanner
// ---------------------------------------------------------------------------

/// Decides where to insert checkpoints in a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointPlanner {
    /// Maximum number of nodes between checkpoints.
    pub max_nodes_between_checkpoints: usize,
    /// Maximum estimated cost between checkpoints.
    pub max_cost_between_checkpoints: f64,
    /// Whether to always checkpoint at parallel join nodes.
    pub checkpoint_at_joins: bool,
}

impl Default for CheckpointPlanner {
    fn default() -> Self {
        Self {
            max_nodes_between_checkpoints: 5,
            max_cost_between_checkpoints: 100.0,
            checkpoint_at_joins: true,
        }
    }
}

impl CheckpointPlanner {
    /// Create a new checkpoint planner.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set max nodes between checkpoints.
    #[must_use]
    pub fn with_max_nodes(mut self, n: usize) -> Self {
        self.max_nodes_between_checkpoints = n;
        self
    }

    /// Set max cost between checkpoints.
    #[must_use]
    pub fn with_max_cost(mut self, cost: f64) -> Self {
        self.max_cost_between_checkpoints = cost;
        self
    }

    /// Decide whether a node should be a checkpoint.
    pub fn should_checkpoint(
        &self,
        node: &WorkflowNodeDef,
        nodes_since_last_checkpoint: usize,
        cost_since_last_checkpoint: f64,
    ) -> bool {
        if self.checkpoint_at_joins && node.node_type == WorkflowNodeType::ParallelJoin {
            return true;
        }
        nodes_since_last_checkpoint >= self.max_nodes_between_checkpoints
            || cost_since_last_checkpoint >= self.max_cost_between_checkpoints
    }

    /// Plan checkpoints over a list of ordered nodes.
    pub fn plan_checkpoints(&self, nodes: &[WorkflowNodeDef]) -> Vec<PlanningNodeId> {
        let mut checkpoints = Vec::new();
        let mut since_last = 0usize;
        let mut cost_since = 0.0f64;

        for node in nodes {
            if self.should_checkpoint(node, since_last, cost_since) {
                checkpoints.push(node.id);
                since_last = 0;
                cost_since = 0.0;
            }
            since_last += 1;
            cost_since += node.estimated_cost;
        }
        checkpoints
    }
}

// ---------------------------------------------------------------------------
// WorkflowSynthesisResult
// ---------------------------------------------------------------------------

/// Output of workflow synthesis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSynthesisResult {
    pub plan_id: PlanId,
    pub nodes: Vec<WorkflowNodeDef>,
    pub edges: Vec<WorkflowEdgeDef>,
    pub checkpoints: Vec<PlanningNodeId>,
    pub rollback_strategy: RollbackStrategy,
    pub total_estimated_cost: f64,
    pub total_estimated_duration_secs: u64,
    pub node_count: usize,
    pub edge_count: usize,
    pub synthesized_at: DateTime<Utc>,
}

impl WorkflowSynthesisResult {
    /// Get the node with the given id, if present.
    pub fn node(&self, id: &PlanningNodeId) -> Option<&WorkflowNodeDef> {
        self.nodes.iter().find(|n| n.id == *id)
    }

    /// Get all edges originating from a node.
    pub fn edges_from(&self, id: &PlanningNodeId) -> Vec<&WorkflowEdgeDef> {
        self.edges.iter().filter(|e| e.from == *id).collect()
    }

    /// Get all edges targeting a node.
    pub fn edges_to(&self, id: &PlanningNodeId) -> Vec<&WorkflowEdgeDef> {
        self.edges.iter().filter(|e| e.to == *id).collect()
    }

    /// Return start nodes (no incoming edges).
    pub fn start_nodes(&self) -> Vec<&WorkflowNodeDef> {
        self.nodes
            .iter()
            .filter(|n| self.edges.iter().all(|e| e.to != n.id))
            .collect()
    }

    /// Return end nodes (no outgoing edges).
    pub fn end_nodes(&self) -> Vec<&WorkflowNodeDef> {
        self.nodes
            .iter()
            .filter(|n| self.edges.iter().all(|e| e.from != n.id))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// WorkflowSynthesizer
// ---------------------------------------------------------------------------

/// Synthesises a workflow definition from a [`Plan`] and its [`PlanGraph`].
#[derive(Debug, Clone)]
pub struct WorkflowSynthesizer {
    checkpoint_planner: CheckpointPlanner,
    rollback_strategy: RollbackStrategy,
}

impl WorkflowSynthesizer {
    /// Create a new synthesizer with default settings.
    pub fn new() -> Self {
        Self {
            checkpoint_planner: CheckpointPlanner::default(),
            rollback_strategy: RollbackStrategy::default(),
        }
    }

    /// Set the checkpoint planner.
    #[must_use]
    pub fn with_checkpoint_planner(mut self, planner: CheckpointPlanner) -> Self {
        self.checkpoint_planner = planner;
        self
    }

    /// Set the rollback strategy.
    #[must_use]
    pub fn with_rollback_strategy(mut self, strategy: RollbackStrategy) -> Self {
        self.rollback_strategy = strategy;
        self
    }

    /// Synthesise a workflow from a plan and its graph.
    pub fn synthesize(
        &self,
        plan: &Plan,
        graph: &PlanGraph,
    ) -> PlanningResult<WorkflowSynthesisResult> {
        if graph.is_empty() {
            return Err(PlanningError::new(
                crate::error::PlanningErrorCode::WorkflowSynthesisFailed,
                "cannot synthesize workflow from empty graph",
            ));
        }

        let nodes = self.build_nodes(graph)?;
        let edges = self.build_edges(graph)?;
        let checkpoints = self.checkpoint_planner.plan_checkpoints(&nodes);

        let total_cost = nodes.iter().map(|n| n.estimated_cost).sum();
        let total_duration = nodes.iter().map(|n| n.estimated_duration_secs).sum();

        Ok(WorkflowSynthesisResult {
            plan_id: plan.id,
            nodes,
            edges,
            checkpoints,
            rollback_strategy: self.rollback_strategy.clone(),
            total_estimated_cost: total_cost,
            total_estimated_duration_secs: total_duration,
            node_count: graph.node_count(),
            edge_count: graph.edge_count(),
            synthesized_at: Utc::now(),
        })
    }

    /// Synthesise from just a graph (no plan context).
    pub fn synthesize_graph(
        &self,
        plan_id: PlanId,
        graph: &PlanGraph,
    ) -> PlanningResult<WorkflowSynthesisResult> {
        if graph.is_empty() {
            return Err(PlanningError::new(
                crate::error::PlanningErrorCode::WorkflowSynthesisFailed,
                "cannot synthesize workflow from empty graph",
            ));
        }

        let nodes = self.build_nodes(graph)?;
        let edges = self.build_edges(graph)?;
        let checkpoints = self.checkpoint_planner.plan_checkpoints(&nodes);

        let total_cost = nodes.iter().map(|n| n.estimated_cost).sum();
        let total_duration = nodes.iter().map(|n| n.estimated_duration_secs).sum();

        Ok(WorkflowSynthesisResult {
            plan_id,
            nodes,
            edges,
            checkpoints,
            rollback_strategy: self.rollback_strategy.clone(),
            total_estimated_cost: total_cost,
            total_estimated_duration_secs: total_duration,
            node_count: graph.node_count(),
            edge_count: graph.edge_count(),
            synthesized_at: Utc::now(),
        })
    }

    fn build_nodes(&self, graph: &PlanGraph) -> PlanningResult<Vec<WorkflowNodeDef>> {
        graph
            .nodes()
            .map(|n| {
                let node_type = self.map_node_type(n.node_type);
                Ok(WorkflowNodeDef::new(n.id, &n.name, node_type)
                    .with_description(&n.description)
                    .with_cost(n.estimated_cost)
                    .with_duration(n.estimated_duration_secs)
                    .with_resources(n.resource_requirements.clone()))
            })
            .collect()
    }

    fn build_edges(&self, graph: &PlanGraph) -> PlanningResult<Vec<WorkflowEdgeDef>> {
        let mut edges = Vec::new();
        for node in graph.nodes() {
            for graph_edge in graph.edges_from(&node.id) {
                let mut edge = WorkflowEdgeDef::new(graph_edge.from, graph_edge.to);
                edge.label = graph_edge.label.clone();
                edges.push(edge);
            }
        }
        Ok(edges)
    }

    fn map_node_type(&self, nt: PlanningNodeType) -> WorkflowNodeType {
        match nt {
            PlanningNodeType::Task => WorkflowNodeType::Task,
            PlanningNodeType::Decision => WorkflowNodeType::Decision,
            PlanningNodeType::ParallelSplit => WorkflowNodeType::ParallelSplit,
            PlanningNodeType::ParallelJoin => WorkflowNodeType::ParallelJoin,
            PlanningNodeType::Milestone => WorkflowNodeType::Event,
            PlanningNodeType::Start => WorkflowNodeType::Event,
            PlanningNodeType::End => WorkflowNodeType::Event,
            PlanningNodeType::Condition => WorkflowNodeType::Decision,
            PlanningNodeType::Loop => WorkflowNodeType::Task,
        }
    }
}

impl Default for WorkflowSynthesizer {
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
    use crate::graph::{PlanGraph, PlanningEdge, PlanningNode};
    use crate::id::{PlanId, PlanningEdgeId};
    use crate::plan::PlanDefinition;
    use crate::types::PlanMetadata;
    use crate::types::AlgorithmType;

    fn make_graph() -> (PlanGraph, PlanningNodeId, PlanningNodeId, PlanningNodeId) {
        let start = PlanningNodeId::new();
        let task = PlanningNodeId::new();
        let end = PlanningNodeId::new();
        let mut g = PlanGraph::new();
        g.add_node(PlanningNode::new(start, "start", PlanningNodeType::Start));
        g.add_node(
            PlanningNode::new(task, "task", PlanningNodeType::Task)
                .with_estimated_cost(50.0)
                .with_estimated_duration_secs(300),
        );
        g.add_node(PlanningNode::new(end, "end", PlanningNodeType::End));
        g.add_edge(PlanningEdge::new(PlanningEdgeId::new(), start, task))
            .unwrap();
        g.add_edge(PlanningEdge::new(PlanningEdgeId::new(), task, end))
            .unwrap();
        (g, start, task, end)
    }

    fn make_plan() -> Plan {
        let def = PlanDefinition::new(
            crate::id::PlanningGoalId::new(),
            AlgorithmType::HierarchicalTaskNetwork,
        );
        Plan::new(def, PlanMetadata::new("test"))
    }

    // WorkflowNodeType tests

    #[test]
    fn workflow_node_type_display() {
        assert_eq!(WorkflowNodeType::Task.to_string(), "task");
        assert_eq!(
            WorkflowNodeType::ParallelSplit.to_string(),
            "parallel_split"
        );
        assert_eq!(WorkflowNodeType::SubWorkflow.to_string(), "sub_workflow");
    }

    // WorkflowNodeDef tests

    #[test]
    fn node_def_creation() {
        let id = PlanningNodeId::new();
        let n = WorkflowNodeDef::new(id, "n", WorkflowNodeType::Task);
        assert_eq!(n.id, id);
        assert_eq!(n.name, "n");
        assert_eq!(n.node_type, WorkflowNodeType::Task);
        assert!(n.description.is_empty());
    }

    #[test]
    fn node_def_builder() {
        let n = WorkflowNodeDef::new(PlanningNodeId::new(), "x", WorkflowNodeType::Gate)
            .with_description("desc")
            .with_cost(42.0)
            .with_duration(100)
            .with_metadata("k", serde_json::json!("v"));
        assert_eq!(n.description, "desc");
        assert!((n.estimated_cost - 42.0).abs() < f64::EPSILON);
        assert_eq!(n.estimated_duration_secs, 100);
        assert_eq!(n.metadata.get("k").unwrap(), "v");
    }

    // WorkflowEdgeDef tests

    #[test]
    fn edge_def_creation() {
        let from = PlanningNodeId::new();
        let to = PlanningNodeId::new();
        let e = WorkflowEdgeDef::new(from, to);
        assert_eq!(e.from, from);
        assert_eq!(e.to, to);
        assert!(e.label.is_none());
        assert!(e.condition.is_none());
    }

    #[test]
    fn edge_def_builder() {
        let e = WorkflowEdgeDef::new(PlanningNodeId::new(), PlanningNodeId::new())
            .with_label("yes")
            .with_condition("x > 0");
        assert_eq!(e.label.as_deref(), Some("yes"));
        assert_eq!(e.condition.as_deref(), Some("x > 0"));
    }

    // RollbackStrategy tests

    #[test]
    fn rollback_strategy_display() {
        assert_eq!(RollbackStrategy::ReverseOrder.to_string(), "reverse_order");
        assert_eq!(
            RollbackStrategy::ToLastCheckpoint.to_string(),
            "to_last_checkpoint"
        );
        assert_eq!(RollbackStrategy::NoRollback.to_string(), "no_rollback");
        assert_eq!(
            RollbackStrategy::Custom("foo".to_string()).to_string(),
            "custom(foo)"
        );
    }

    #[test]
    fn rollback_strategy_default() {
        assert_eq!(RollbackStrategy::default(), RollbackStrategy::ReverseOrder);
    }

    // CheckpointPlanner tests

    #[test]
    fn checkpoint_planner_default() {
        let p = CheckpointPlanner::default();
        assert_eq!(p.max_nodes_between_checkpoints, 5);
        assert!(p.checkpoint_at_joins);
    }

    #[test]
    fn checkpoint_planner_builder() {
        let p = CheckpointPlanner::new()
            .with_max_nodes(3)
            .with_max_cost(50.0);
        assert_eq!(p.max_nodes_between_checkpoints, 3);
        assert!((p.max_cost_between_checkpoints - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn checkpoint_planner_at_join() {
        let p = CheckpointPlanner::default();
        let node = WorkflowNodeDef::new(PlanningNodeId::new(), "j", WorkflowNodeType::ParallelJoin);
        assert!(p.should_checkpoint(&node, 0, 0.0));
    }

    #[test]
    fn checkpoint_planner_by_count() {
        let p = CheckpointPlanner::default();
        let node = WorkflowNodeDef::new(PlanningNodeId::new(), "t", WorkflowNodeType::Task);
        assert!(!p.should_checkpoint(&node, 0, 0.0));
        assert!(p.should_checkpoint(&node, 5, 0.0));
    }

    #[test]
    fn checkpoint_planner_by_cost() {
        let p = CheckpointPlanner::default().with_max_cost(10.0);
        let node = WorkflowNodeDef::new(PlanningNodeId::new(), "t", WorkflowNodeType::Task);
        assert!(p.should_checkpoint(&node, 0, 10.0));
    }

    #[test]
    fn checkpoint_planner_plan_checkpoints() {
        let p = CheckpointPlanner::default().with_max_nodes(2);
        let nodes: Vec<WorkflowNodeDef> = (0..5)
            .map(|i| {
                WorkflowNodeDef::new(
                    PlanningNodeId::new(),
                    format!("n{}", i),
                    WorkflowNodeType::Task,
                )
            })
            .collect();
        let cps = p.plan_checkpoints(&nodes);
        assert!(!cps.is_empty());
    }

    // WorkflowSynthesisResult tests

    #[test]
    fn synthesis_result_node_lookup() {
        let (g, _, _, _) = make_graph();
        let plan = make_plan();
        let synth = WorkflowSynthesizer::new();
        let result = synth.synthesize(&plan, &g).unwrap();
        assert!(result.node(&result.nodes[0].id).is_some());
        assert!(result.node(&PlanningNodeId::new()).is_none());
    }

    #[test]
    fn synthesis_result_edges_from() {
        let (g, start, _, _) = make_graph();
        let plan = make_plan();
        let synth = WorkflowSynthesizer::new();
        let result = synth.synthesize(&plan, &g).unwrap();
        let out = result.edges_from(&start);
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn synthesis_result_edges_to() {
        let (g, _, _, end) = make_graph();
        let plan = make_plan();
        let synth = WorkflowSynthesizer::new();
        let result = synth.synthesize(&plan, &g).unwrap();
        let incoming = result.edges_to(&end);
        assert_eq!(incoming.len(), 1);
    }

    #[test]
    fn synthesis_result_start_and_end_nodes() {
        let (g, _, _, _) = make_graph();
        let plan = make_plan();
        let synth = WorkflowSynthesizer::new();
        let result = synth.synthesize(&plan, &g).unwrap();
        assert_eq!(result.start_nodes().len(), 1);
        assert_eq!(result.end_nodes().len(), 1);
    }

    // WorkflowSynthesizer tests

    #[test]
    fn synthesizer_basic() {
        let (g, _, _, _) = make_graph();
        let plan = make_plan();
        let synth = WorkflowSynthesizer::new();
        let result = synth.synthesize(&plan, &g).unwrap();
        assert_eq!(result.plan_id, plan.id);
        assert_eq!(result.node_count, 3);
        assert_eq!(result.edge_count, 2);
        assert_eq!(result.nodes.len(), 3);
        assert_eq!(result.edges.len(), 2);
        assert_eq!(result.rollback_strategy, RollbackStrategy::ReverseOrder);
    }

    #[test]
    fn synthesizer_empty_graph_errors() {
        let g = PlanGraph::new();
        let plan = make_plan();
        let synth = WorkflowSynthesizer::new();
        assert!(synth.synthesize(&plan, &g).is_err());
    }

    #[test]
    fn synthesizer_with_custom_settings() {
        let (g, _, _, _) = make_graph();
        let plan = make_plan();
        let synth = WorkflowSynthesizer::new()
            .with_rollback_strategy(RollbackStrategy::ToLastCheckpoint)
            .with_checkpoint_planner(CheckpointPlanner::new().with_max_nodes(1));
        let result = synth.synthesize(&plan, &g).unwrap();
        assert_eq!(result.rollback_strategy, RollbackStrategy::ToLastCheckpoint);
        assert!(!result.checkpoints.is_empty());
    }

    #[test]
    fn synthesizer_synthesize_graph() {
        let (g, _, _, _) = make_graph();
        let plan_id = PlanId::new();
        let synth = WorkflowSynthesizer::new();
        let result = synth.synthesize_graph(plan_id, &g).unwrap();
        assert_eq!(result.plan_id, plan_id);
        assert_eq!(result.node_count, 3);
    }

    #[test]
    fn synthesizer_empty_graph_synthesize_graph_errors() {
        let g = PlanGraph::new();
        let synth = WorkflowSynthesizer::new();
        assert!(synth.synthesize_graph(PlanId::new(), &g).is_err());
    }

    #[test]
    fn synthesizer_total_cost_and_duration() {
        let (g, _, _, _) = make_graph();
        let plan = make_plan();
        let synth = WorkflowSynthesizer::new();
        let result = synth.synthesize(&plan, &g).unwrap();
        assert!(result.total_estimated_cost >= 0.0);
        assert!(result.total_estimated_duration_secs >= 0);
    }

    #[test]
    fn synthesizer_node_type_mapping() {
        let mut g = PlanGraph::new();
        let ids: Vec<PlanningNodeId> = (0..3).map(|_| PlanningNodeId::new()).collect();
        g.add_node(PlanningNode::new(ids[0], "start", PlanningNodeType::Start));
        g.add_node(PlanningNode::new(ids[1], "task", PlanningNodeType::Task));
        g.add_node(PlanningNode::new(
            ids[2],
            "decision",
            PlanningNodeType::Decision,
        ));
        g.add_edge(PlanningEdge::new(PlanningEdgeId::new(), ids[0], ids[1]))
            .unwrap();
        g.add_edge(PlanningEdge::new(PlanningEdgeId::new(), ids[1], ids[2]))
            .unwrap();

        let plan = make_plan();
        let synth = WorkflowSynthesizer::new();
        let result = synth.synthesize(&plan, &g).unwrap();
        let types: Vec<&WorkflowNodeType> = result.nodes.iter().map(|n| &n.node_type).collect();
        assert!(types.contains(&&WorkflowNodeType::Event));
        assert!(types.contains(&&WorkflowNodeType::Task));
        assert!(types.contains(&&WorkflowNodeType::Decision));
    }

    #[test]
    fn synthesizer_custom_rollback() {
        let (g, _, _, _) = make_graph();
        let plan = make_plan();
        let synth = WorkflowSynthesizer::new().with_rollback_strategy(RollbackStrategy::NoRollback);
        let result = synth.synthesize(&plan, &g).unwrap();
        assert_eq!(result.rollback_strategy, RollbackStrategy::NoRollback);
    }

    // Serialization tests

    #[test]
    fn result_serialization_roundtrip() {
        let (g, _, _, _) = make_graph();
        let plan = make_plan();
        let synth = WorkflowSynthesizer::new();
        let result = synth.synthesize(&plan, &g).unwrap();
        let json = serde_json::to_string(&result).unwrap();
        let back: WorkflowSynthesisResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.node_count, 3);
        assert_eq!(back.plan_id, plan.id);
    }

    #[test]
    fn node_def_serialization_roundtrip() {
        let n = WorkflowNodeDef::new(PlanningNodeId::new(), "x", WorkflowNodeType::Task)
            .with_cost(10.0);
        let json = serde_json::to_string(&n).unwrap();
        let back: WorkflowNodeDef = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "x");
    }

    #[test]
    fn edge_def_serialization_roundtrip() {
        let e = WorkflowEdgeDef::new(PlanningNodeId::new(), PlanningNodeId::new()).with_label("a");
        let json = serde_json::to_string(&e).unwrap();
        let back: WorkflowEdgeDef = serde_json::from_str(&json).unwrap();
        assert_eq!(back.label.as_deref(), Some("a"));
    }

    #[test]
    fn checkpoint_planner_serialization_roundtrip() {
        let p = CheckpointPlanner::new().with_max_nodes(7);
        let json = serde_json::to_string(&p).unwrap();
        let back: CheckpointPlanner = serde_json::from_str(&json).unwrap();
        assert_eq!(back.max_nodes_between_checkpoints, 7);
    }
}
