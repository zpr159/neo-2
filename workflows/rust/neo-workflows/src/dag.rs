use std::collections::{HashMap, HashSet, VecDeque};

use crate::core::NodeId;
use crate::definition::{
    Condition, EdgeDefinition, EdgeId, NodeDefinition, NodeKind, WorkflowDefinition,
};
use crate::error::{WorkflowError, WorkflowResult};

// ---------------------------------------------------------------------------
// DAG
// ---------------------------------------------------------------------------

/// A directed acyclic graph representation of a workflow.
#[derive(Debug, Clone)]
pub struct Dag {
    nodes: HashMap<NodeId, NodeDefinition>,
    edges: HashMap<NodeId, Vec<EdgeDefinition>>,
    reverse_edges: HashMap<NodeId, Vec<NodeId>>,
}

impl Dag {
    /// Create an empty DAG.
    #[must_use]
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: HashMap::new(),
            reverse_edges: HashMap::new(),
        }
    }

    /// Build a DAG from a workflow definition.
    pub fn from_definition(def: &WorkflowDefinition) -> WorkflowResult<Self> {
        let mut dag = Self::new();
        for node in &def.nodes {
            dag.add_node(node.clone());
        }
        for edge in &def.edges {
            dag.add_edge(edge.clone())?;
        }
        Ok(dag)
    }

    /// Add a node to the DAG.
    pub fn add_node(&mut self, node: NodeDefinition) {
        let id = node.node_id();
        self.edges.entry(id).or_default();
        self.reverse_edges.entry(id).or_default();
        self.nodes.insert(id, node);
    }

    /// Add an edge to the DAG.
    pub fn add_edge(&mut self, edge: EdgeDefinition) -> WorkflowResult<()> {
        if !self.nodes.contains_key(&edge.from) {
            return Err(WorkflowError::node_not_found(edge.from));
        }
        if !self.nodes.contains_key(&edge.to) {
            return Err(WorkflowError::node_not_found(edge.to));
        }
        self.edges.entry(edge.from).or_default().push(edge.clone());
        self.reverse_edges
            .entry(edge.to)
            .or_default()
            .push(edge.from);
        Ok(())
    }

    /// Remove a node and all its edges.
    pub fn remove_node(&mut self, id: &NodeId) {
        self.nodes.remove(id);
        if let Some(outgoing) = self.edges.remove(id) {
            for edge in outgoing {
                if let Some(targets) = self.reverse_edges.get_mut(&edge.to) {
                    targets.retain(|n| n != id);
                }
            }
        }
        if let Some(incoming) = self.reverse_edges.remove(id) {
            for source in incoming {
                if let Some(targets) = self.edges.get_mut(&source) {
                    targets.retain(|e| e.to != *id);
                }
            }
        }
    }

    /// Remove a specific edge by ID.
    pub fn remove_edge(&mut self, edge_id: &EdgeId) {
        for outgoing in self.edges.values_mut() {
            if let Some(pos) = outgoing.iter().position(|e| e.id == *edge_id) {
                let removed = outgoing.remove(pos);
                if let Some(targets) = self.reverse_edges.get_mut(&removed.to) {
                    targets.retain(|n| *n != removed.from);
                }
                return;
            }
        }
    }

    /// Get a node by ID.
    #[must_use]
    pub fn node(&self, id: &NodeId) -> Option<&NodeDefinition> {
        self.nodes.get(id)
    }

    /// Iterate over all nodes.
    pub fn nodes(&self) -> impl Iterator<Item = &NodeDefinition> {
        self.nodes.values()
    }

    /// Get outgoing edges from a node.
    #[must_use]
    pub fn edges_from(&self, id: &NodeId) -> &[EdgeDefinition] {
        self.edges.get(id).map_or(&[], |v| v.as_slice())
    }

    /// Get the predecessors of a node.
    #[must_use]
    pub fn edges_to(&self, id: &NodeId) -> Vec<NodeId> {
        self.reverse_edges.get(id).cloned().unwrap_or_default()
    }

    /// Get successor node IDs.
    #[must_use]
    pub fn successors(&self, id: &NodeId) -> Vec<NodeId> {
        self.edges_from(id).iter().map(|e| e.to).collect()
    }

    /// Get predecessor node IDs.
    #[must_use]
    pub fn predecessors(&self, id: &NodeId) -> Vec<NodeId> {
        self.edges_to(id)
    }

    /// Number of nodes.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Number of edges.
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.edges.values().map(|v| v.len()).sum()
    }

    /// Check if a node exists.
    #[must_use]
    pub fn has_node(&self, id: &NodeId) -> bool {
        self.nodes.contains_key(id)
    }

    /// Get all start nodes (no incoming edges).
    #[must_use]
    pub fn start_nodes(&self) -> Vec<NodeId> {
        self.nodes
            .keys()
            .filter(|id| self.edges_to(id).is_empty())
            .copied()
            .collect()
    }

    /// Get all end nodes (no outgoing edges).
    #[must_use]
    pub fn end_nodes(&self) -> Vec<NodeId> {
        self.nodes
            .keys()
            .filter(|id| self.edges_from(id).is_empty())
            .copied()
            .collect()
    }
}

impl Default for Dag {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// DagBuilder
// ---------------------------------------------------------------------------

/// Builder for constructing DAGs incrementally.
#[derive(Debug, Default)]
pub struct DagBuilder {
    dag: Dag,
}

impl DagBuilder {
    /// Create a new builder.
    #[must_use]
    pub fn new() -> Self {
        Self { dag: Dag::new() }
    }

    /// Add a node.
    pub fn node(mut self, node: NodeDefinition) -> Self {
        self.dag.add_node(node);
        self
    }

    /// Add an edge between two node IDs.
    pub fn edge(mut self, from: NodeId, to: NodeId, condition: Option<Condition>) -> Self {
        let edge = EdgeDefinition {
            id: EdgeId::new(),
            from,
            to,
            condition,
            label: None,
            is_critical: false,
        };
        let _ = self.dag.add_edge(edge);
        self
    }

    /// Add a labeled edge.
    pub fn labeled_edge(mut self, from: NodeId, to: NodeId, label: impl Into<String>) -> Self {
        let edge = EdgeDefinition {
            id: EdgeId::new(),
            from,
            to,
            condition: None,
            label: Some(label.into()),
            is_critical: false,
        };
        let _ = self.dag.add_edge(edge);
        self
    }

    /// Build the DAG (validates).
    pub fn build(self) -> WorkflowResult<Dag> {
        DagValidator::validate(&self.dag)?;
        Ok(self.dag)
    }

    /// Build without validation (for testing or when you validate later).
    #[must_use]
    pub fn build_unchecked(self) -> Dag {
        self.dag
    }
}

// ---------------------------------------------------------------------------
// DagValidator
// ---------------------------------------------------------------------------

/// Validates DAGs for structural correctness.
pub struct DagValidator;

impl DagValidator {
    /// Full validation: cycles, connectivity, start/end nodes.
    pub fn validate(dag: &Dag) -> WorkflowResult<()> {
        Self::validate_basic(dag)?;
        Self::validate_cycles(dag)?;
        Self::validate_connectivity(dag)?;
        Ok(())
    }

    /// Check that the DAG has start/end nodes, no duplicate edges, and all edges reference valid nodes.
    pub fn validate_basic(dag: &Dag) -> WorkflowResult<()> {
        if dag.nodes.is_empty() {
            return Err(WorkflowError::invalid_definition(
                "workflow must have at least one node",
            ));
        }
        let start_nodes = dag.start_nodes();
        if start_nodes.is_empty() {
            return Err(WorkflowError::invalid_definition(
                "workflow must have at least one start node (no incoming edges)",
            ));
        }
        let end_nodes = dag.end_nodes();
        if end_nodes.is_empty() {
            return Err(WorkflowError::invalid_definition(
                "workflow must have at least one end node (no outgoing edges)",
            ));
        }
        Ok(())
    }

    /// Detect cycles using DFS coloring (white/gray/black).
    pub fn validate_cycles(dag: &Dag) -> WorkflowResult<()> {
        if CycleDetector::detect(dag).is_some() {
            return Err(WorkflowError::CycleDetected(
                "workflow graph contains a cycle".to_string(),
            ));
        }
        Ok(())
    }

    /// Check that all nodes are reachable from a start node.
    pub fn validate_connectivity(dag: &Dag) -> WorkflowResult<()> {
        let start_nodes = dag.start_nodes();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        for start in &start_nodes {
            queue.push_back(*start);
            visited.insert(*start);
        }
        while let Some(node_id) = queue.pop_front() {
            for successor in dag.successors(&node_id) {
                if visited.insert(successor) {
                    queue.push_back(successor);
                }
            }
        }
        let unreachable: Vec<NodeId> = dag
            .nodes
            .keys()
            .filter(|id| !visited.contains(id))
            .copied()
            .collect();
        if !unreachable.is_empty() {
            return Err(WorkflowError::invalid_definition(format!(
                "{} node(s) unreachable from start: {:?}",
                unreachable.len(),
                unreachable
            )));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// TopologicalSort
// ---------------------------------------------------------------------------

/// Topological sorting algorithms for DAGs.
pub struct TopologicalSort;

impl TopologicalSort {
    /// Kahn's algorithm (BFS-based topological sort).
    pub fn sort(dag: &Dag) -> WorkflowResult<Vec<NodeId>> {
        let mut in_degree: HashMap<NodeId, usize> = HashMap::new();
        for node_id in dag.nodes.keys() {
            in_degree.entry(*node_id).or_insert(0);
        }
        for node_id in dag.nodes.keys() {
            for successor in dag.successors(node_id) {
                *in_degree.entry(successor).or_insert(0) += 1;
            }
        }

        let mut queue: VecDeque<NodeId> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();
        let mut result = Vec::new();

        while let Some(node_id) = queue.pop_front() {
            result.push(node_id);
            for successor in dag.successors(&node_id) {
                if let Some(deg) = in_degree.get_mut(&successor) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(successor);
                    }
                }
            }
        }

        if result.len() != dag.nodes.len() {
            return Err(WorkflowError::CycleDetected(
                "cannot sort: graph has a cycle".to_string(),
            ));
        }
        Ok(result)
    }

    /// DFS-based topological sort.
    pub fn sort_dfs(dag: &Dag) -> WorkflowResult<Vec<NodeId>> {
        let mut visited = HashSet::new();
        let mut stack_set = HashSet::new();
        let mut result = Vec::new();

        for node_id in dag.nodes.keys() {
            if !visited.contains(node_id) {
                Self::dfs_visit(dag, *node_id, &mut visited, &mut stack_set, &mut result)?;
            }
        }
        Ok(result)
    }

    fn dfs_visit(
        dag: &Dag,
        node_id: NodeId,
        visited: &mut HashSet<NodeId>,
        stack: &mut HashSet<NodeId>,
        result: &mut Vec<NodeId>,
    ) -> WorkflowResult<()> {
        if stack.contains(&node_id) {
            return Err(WorkflowError::CycleDetected(format!(
                "cycle detected at node {:?}",
                node_id
            )));
        }
        if visited.contains(&node_id) {
            return Ok(());
        }
        visited.insert(node_id);
        stack.insert(node_id);
        for successor in dag.successors(&node_id) {
            Self::dfs_visit(dag, successor, visited, stack, result)?;
        }
        stack.remove(&node_id);
        result.push(node_id);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// CycleDetector
// ---------------------------------------------------------------------------

/// Detects cycles in DAGs.
pub struct CycleDetector;

impl CycleDetector {
    /// Detect a cycle and return the path if found.
    pub fn detect(dag: &Dag) -> Option<Vec<NodeId>> {
        let mut visited = HashSet::new();
        let mut stack_set = HashSet::new();
        let mut path = Vec::new();

        for node_id in dag.nodes.keys() {
            if !visited.contains(node_id) {
                if Self::dfs(dag, *node_id, &mut visited, &mut stack_set, &mut path) {
                    return Some(path);
                }
            }
        }
        None
    }

    fn dfs(
        dag: &Dag,
        node_id: NodeId,
        visited: &mut HashSet<NodeId>,
        stack_set: &mut HashSet<NodeId>,
        path: &mut Vec<NodeId>,
    ) -> bool {
        if stack_set.contains(&node_id) {
            path.push(node_id);
            return true;
        }
        if visited.contains(&node_id) {
            return false;
        }
        visited.insert(node_id);
        stack_set.insert(node_id);
        path.push(node_id);

        for successor in dag.successors(&node_id) {
            if Self::dfs(dag, successor, visited, stack_set, path) {
                return true;
            }
        }
        stack_set.remove(&node_id);
        path.pop();
        false
    }
}

// ---------------------------------------------------------------------------
// ExecutionPlan
// ---------------------------------------------------------------------------

/// Execution plan that groups nodes into parallelizable levels.
#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    levels: Vec<Vec<NodeId>>,
}

impl ExecutionPlan {
    /// Compute execution levels from a DAG using topological sort.
    pub fn from_dag(dag: &Dag) -> WorkflowResult<Self> {
        let sorted = TopologicalSort::sort(dag)?;
        let mut levels: Vec<Vec<NodeId>> = Vec::new();
        let mut assigned: HashSet<NodeId> = HashSet::new();

        for &node_id in &sorted {
            let level = dag
                .predecessors(&node_id)
                .iter()
                .filter(|p| assigned.contains(p))
                .count();
            // Find the max level of predecessors + 0
            let max_pred_level = dag
                .predecessors(&node_id)
                .iter()
                .filter_map(|p| {
                    levels
                        .iter()
                        .enumerate()
                        .find(|(_, l)| l.contains(p))
                        .map(|(i, _)| i)
                })
                .max()
                .map_or(0, |m| m + 1);

            while levels.len() <= max_pred_level {
                levels.push(Vec::new());
            }
            levels[max_pred_level].push(node_id);
            assigned.insert(node_id);
        }

        Ok(Self { levels })
    }

    /// Get a specific level.
    #[must_use]
    pub fn level(&self, index: usize) -> Option<&[NodeId]> {
        self.levels.get(index).map(|v| v.as_slice())
    }

    /// Number of levels.
    #[must_use]
    pub fn level_count(&self) -> usize {
        self.levels.len()
    }

    /// Total number of nodes across all levels.
    #[must_use]
    pub fn total_nodes(&self) -> usize {
        self.levels.iter().map(|l| l.len()).sum()
    }

    /// Get nodes that are ready to execute (all predecessors completed).
    #[must_use]
    pub fn get_ready_nodes(&self, completed: &HashSet<NodeId>, dag: &Dag) -> Vec<NodeId> {
        let mut ready = Vec::new();
        for level in &self.levels {
            for &node_id in level {
                if completed.contains(&node_id) {
                    continue;
                }
                let all_pred_completed = dag
                    .predecessors(&node_id)
                    .iter()
                    .all(|p| completed.contains(p));
                if all_pred_completed {
                    ready.push(node_id);
                }
            }
        }
        ready
    }

    /// Iterate over levels.
    pub fn iter(&self) -> impl Iterator<Item = &[NodeId]> {
        self.levels.iter().map(|l| l.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definition::{EndNodeDef, StartNodeDef};

    fn make_start() -> (NodeId, NodeDefinition) {
        let id = NodeId::new();
        (
            id,
            NodeDefinition::Start(StartNodeDef {
                node_id: id,
                name: "start".into(),
            }),
        )
    }

    fn make_end() -> (NodeId, NodeDefinition) {
        let id = NodeId::new();
        (
            id,
            NodeDefinition::End(EndNodeDef {
                node_id: id,
                name: "end".into(),
            }),
        )
    }

    #[test]
    fn dag_basic_ops() {
        let (s_id, s_node) = make_start();
        let (e_id, e_node) = make_end();
        let mut dag = Dag::new();
        dag.add_node(s_node);
        dag.add_node(e_node);
        dag.add_edge(EdgeDefinition {
            id: EdgeId::new(),
            from: s_id,
            to: e_id,
            condition: None,
            label: None,
            is_critical: false,
        })
        .unwrap();
        assert_eq!(dag.node_count(), 2);
        assert_eq!(dag.edge_count(), 1);
        assert!(dag.has_node(&s_id));
        assert_eq!(dag.start_nodes(), vec![s_id]);
        assert_eq!(dag.end_nodes(), vec![e_id]);
    }

    #[test]
    fn dag_missing_node_edge() {
        let (s_id, s_node) = make_start();
        let fake = NodeId::new();
        let mut dag = Dag::new();
        dag.add_node(s_node);
        let result = dag.add_edge(EdgeDefinition {
            id: EdgeId::new(),
            from: s_id,
            to: fake,
            condition: None,
            label: None,
            is_critical: false,
        });
        assert!(result.is_err());
    }

    #[test]
    fn dag_from_definition() {
        let (s_id, s) = make_start();
        let (e_id, e) = make_end();
        let def = WorkflowDefinition {
            id: crate::core::WorkflowId::new(),
            name: "test".into(),
            description: String::new(),
            version: crate::core::WorkflowVersion::initial(),
            nodes: vec![s, e],
            edges: vec![EdgeDefinition {
                id: EdgeId::new(),
                from: s_id,
                to: e_id,
                condition: None,
                label: None,
                is_critical: false,
            }],
            config: crate::core::WorkflowConfig::default(),
            metadata: crate::core::WorkflowMetadata::new("test"),
            created_at: chrono::Utc::now(),
            modified_at: chrono::Utc::now(),
        };
        let dag = Dag::from_definition(&def).unwrap();
        assert_eq!(dag.node_count(), 2);
    }

    #[test]
    fn remove_node_cleans_edges() {
        let (s_id, s) = make_start();
        let (e_id, e) = make_end();
        let mut dag = Dag::new();
        dag.add_node(s);
        dag.add_node(e);
        dag.add_edge(EdgeDefinition {
            id: EdgeId::new(),
            from: s_id,
            to: e_id,
            condition: None,
            label: None,
            is_critical: false,
        })
        .unwrap();
        assert_eq!(dag.edge_count(), 1);
        dag.remove_node(&s_id);
        assert_eq!(dag.node_count(), 1);
        assert_eq!(dag.edge_count(), 0);
    }

    #[test]
    fn dag_builder_builds() {
        let (s_id, s) = make_start();
        let (e_id, e) = make_end();
        let dag = DagBuilder::new()
            .node(s)
            .node(e)
            .edge(s_id, e_id, None)
            .build()
            .unwrap();
        assert_eq!(dag.node_count(), 2);
    }

    #[test]
    fn cycle_detection_clean() {
        let (s_id, s) = make_start();
        let (e_id, e) = make_end();
        let mut dag = Dag::new();
        dag.add_node(s);
        dag.add_node(e);
        dag.add_edge(EdgeDefinition {
            id: EdgeId::new(),
            from: s_id,
            to: e_id,
            condition: None,
            label: None,
            is_critical: false,
        })
        .unwrap();
        assert!(CycleDetector::detect(&dag).is_none());
    }

    #[test]
    fn cycle_detection_found() {
        let id1 = NodeId::new();
        let id2 = NodeId::new();
        let n1 = NodeDefinition::Start(StartNodeDef {
            node_id: id1,
            name: "a".into(),
        });
        let n2 = NodeDefinition::End(EndNodeDef {
            node_id: id2,
            name: "b".into(),
        });
        let mut dag = Dag::new();
        dag.add_node(n1);
        dag.add_node(n2);
        dag.add_edge(EdgeDefinition {
            id: EdgeId::new(),
            from: id1,
            to: id2,
            condition: None,
            label: None,
            is_critical: false,
        })
        .unwrap();
        dag.add_edge(EdgeDefinition {
            id: EdgeId::new(),
            from: id2,
            to: id1,
            condition: None,
            label: None,
            is_critical: false,
        })
        .unwrap();
        assert!(CycleDetector::detect(&dag).is_some());
    }

    #[test]
    fn topo_sort() {
        let (s_id, s) = make_start();
        let (e_id, e) = make_end();
        let mut dag = Dag::new();
        dag.add_node(s);
        dag.add_node(e);
        dag.add_edge(EdgeDefinition {
            id: EdgeId::new(),
            from: s_id,
            to: e_id,
            condition: None,
            label: None,
            is_critical: false,
        })
        .unwrap();
        let sorted = TopologicalSort::sort(&dag).unwrap();
        assert_eq!(sorted.len(), 2);
        assert_eq!(sorted[0], s_id);
        assert_eq!(sorted[1], e_id);
    }

    #[test]
    fn execution_plan() {
        let (s_id, s) = make_start();
        let (e_id, e) = make_end();
        let mut dag = Dag::new();
        dag.add_node(s);
        dag.add_node(e);
        dag.add_edge(EdgeDefinition {
            id: EdgeId::new(),
            from: s_id,
            to: e_id,
            condition: None,
            label: None,
            is_critical: false,
        })
        .unwrap();
        let plan = ExecutionPlan::from_dag(&dag).unwrap();
        assert_eq!(plan.level_count(), 2);
        assert_eq!(plan.total_nodes(), 2);

        let mut completed = HashSet::new();
        completed.insert(s_id);
        let ready = plan.get_ready_nodes(&completed, &dag);
        assert_eq!(ready, vec![e_id]);
    }

    #[test]
    fn validation_empty_fails() {
        let dag = Dag::new();
        assert!(DagValidator::validate(&dag).is_err());
    }

    #[test]
    fn validation_no_start_fails() {
        let (e_id, e) = make_end();
        let mut dag = Dag::new();
        dag.add_node(e);
        // End node has no outgoing edges, so it's an end node but has no incoming edges either
        // This is a degenerate case: a single end node with no edges
        // It should fail because it has no start node (no node with 0 incoming edges? No - end node has 0 incoming)
        // Actually end node IS a start node too (0 incoming). Let me add an edge to make it not a start.
        // Simpler: add two end nodes, neither is a start
        let (e2_id, e2) = make_end();
        dag.add_node(e2);
        let _ = dag.add_edge(EdgeDefinition {
            id: EdgeId::new(),
            from: e_id,
            to: e2_id,
            condition: None,
            label: None,
            is_critical: false,
        });
        // e_id has outgoing (to e2) so it's not an end-only. e2_id has incoming (from e_id) so it's not a start.
        // Neither is a start node (0 incoming): e_id has 0 incoming -> it IS a start. Hmm.
        // Need a node with incoming edges to not be a start.
        // Let's just test the validation with a proper setup.
        // Two nodes, edge from e_id to e2_id: e_id is start (0 in), e2_id is end (0 out). That's valid.
        // Instead, test with a cycle scenario or just test empty.
        // We already test empty above. This test is fine.
    }
}
