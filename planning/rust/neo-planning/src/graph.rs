use crate::types::*;
use crate::goal::*;
use serde::{Deserialize, Serialize};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::algo::{is_cyclic_directed, topological_sort};
use std::collections::HashMap;
use neo_core::error::{Result, NeoError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanGraph {
    pub nodes: HashMap<PlanningNodeId, PlanningNode>,
    pub edges: Vec<PlanningEdge>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlanningNodeId(uuid::Uuid);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningNode {
    pub id: PlanningNodeId,
    pub goal_id: GoalId,
    pub action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningEdge {
    pub from: PlanningNodeId,
    pub to: PlanningNodeId,
    pub edge_type: PlanningEdgeType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanningEdgeType {
    Dependency,
    Sequence,
    Choice,
}

pub struct DependencyGraph {
    graph: DiGraph<PlanningNodeId, PlanningEdgeType>,
    node_map: HashMap<PlanningNodeId, NodeIndex>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_map: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, node_id: PlanningNodeId) {
        if !self.node_map.contains_key(&node_id) {
            let idx = self.graph.add_node(node_id);
            self.node_map.insert(node_id, idx);
        }
    }

    pub fn add_dependency(&mut self, from: PlanningNodeId, to: PlanningNodeId, edge_type: PlanningEdgeType) {
        let from_idx = *self.node_map.get(&from).expect("Node not found");
        let to_idx = *self.node_map.get(&to).expect("Node not found");
        self.graph.add_edge(from_idx, to_idx, edge_type);
    }

    pub fn validate(&self) -> Result<()> {
        if is_cyclic_directed(&self.graph) {
            return Err(NeoError::Validation("Cycle detected in dependency graph".to_string()));
        }
        Ok(())
    }

    pub fn get_topological_order(&self) -> Result<Vec<PlanningNodeId>> {
        let sorted = topological_sort(&self.graph, None)
            .map_err(|_| NeoError::Validation("Cycle detected in dependency graph".to_string()))?;
        
        Ok(sorted.into_iter().map(|idx| self.graph[idx]).collect())
    }
}

pub struct ExecutionGraph {
    // Similar to DependencyGraph but for execution orchestration
}
