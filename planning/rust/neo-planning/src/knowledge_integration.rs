//! Knowledge integration for the planning system.
//!
//! Provides a graph-based knowledge representation that the planner can
//! use to store and query domain knowledge, relationships between concepts,
//! and learned facts that inform planning decisions.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{PlanningError, PlanningResult};

// ---------------------------------------------------------------------------
// KnowledgeNodeType
// ---------------------------------------------------------------------------

/// Type of a knowledge node.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KnowledgeNodeType {
    Concept,
    Entity,
    Relation,
    Rule,
    Fact,
    Goal,
    Strategy,
    Custom(String),
}

impl std::fmt::Display for KnowledgeNodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Concept => write!(f, "concept"),
            Self::Entity => write!(f, "entity"),
            Self::Relation => write!(f, "relation"),
            Self::Rule => write!(f, "rule"),
            Self::Fact => write!(f, "fact"),
            Self::Goal => write!(f, "goal"),
            Self::Strategy => write!(f, "strategy"),
            Self::Custom(name) => write!(f, "custom({})", name),
        }
    }
}

// ---------------------------------------------------------------------------
// KnowledgeNode
// ---------------------------------------------------------------------------

/// A single node in the planning knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeNode {
    pub id: String,
    pub name: String,
    pub node_type: KnowledgeNodeType,
    pub description: String,
    pub confidence: f64,
    pub source: String,
    pub properties: HashMap<String, serde_json::Value>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl KnowledgeNode {
    /// Create a new knowledge node.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        node_type: KnowledgeNodeType,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            name: name.into(),
            node_type,
            description: String::new(),
            confidence: 1.0,
            source: String::new(),
            properties: HashMap::new(),
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Set the description.
    #[must_use]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set the confidence.
    #[must_use]
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Set the source.
    #[must_use]
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = source.into();
        self
    }

    /// Add a property.
    #[must_use]
    pub fn with_property(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.properties.insert(key.into(), value);
        self
    }

    /// Add a tag.
    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }
}

// ---------------------------------------------------------------------------
// KnowledgeEdge
// ---------------------------------------------------------------------------

/// A directed edge in the planning knowledge graph representing a
/// relationship between two knowledge nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEdge {
    pub from: String,
    pub to: String,
    pub relationship: String,
    pub weight: f64,
    pub properties: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

impl KnowledgeEdge {
    /// Create a new edge.
    pub fn new(
        from: impl Into<String>,
        to: impl Into<String>,
        relationship: impl Into<String>,
    ) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            relationship: relationship.into(),
            weight: 1.0,
            properties: HashMap::new(),
            created_at: Utc::now(),
        }
    }

    /// Set the weight.
    #[must_use]
    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }

    /// Add a property.
    #[must_use]
    pub fn with_property(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.properties.insert(key.into(), value);
        self
    }
}

// ---------------------------------------------------------------------------
// PlanningKnowledgeGraph
// ---------------------------------------------------------------------------

/// A knowledge graph for the planning system, storing domain knowledge
/// as nodes and relationships.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningKnowledgeGraph {
    nodes: HashMap<String, KnowledgeNode>,
    edges: Vec<KnowledgeEdge>,
    adjacency: HashMap<String, Vec<usize>>,
    reverse_adjacency: HashMap<String, Vec<usize>>,
}

impl PlanningKnowledgeGraph {
    /// Create an empty knowledge graph.
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            adjacency: HashMap::new(),
            reverse_adjacency: HashMap::new(),
        }
    }

    /// Add a node. Returns `true` if the node was newly inserted.
    pub fn add_node(&mut self, node: KnowledgeNode) -> bool {
        if self.nodes.contains_key(&node.id) {
            return false;
        }
        self.nodes.insert(node.id.clone(), node);
        true
    }

    /// Add an edge. Both endpoint nodes must already exist.
    ///
    /// Returns `Ok(true)` if the edge was newly inserted, `Ok(false)` if
    /// it already existed, or `Err` if a node is missing.
    pub fn add_edge(&mut self, edge: KnowledgeEdge) -> PlanningResult<bool> {
        if !self.nodes.contains_key(&edge.from) {
            return Err(PlanningError::new(
                crate::error::PlanningErrorCode::PlanValidationFailed,
                format!("source node '{}' not found", edge.from),
            ));
        }
        if !self.nodes.contains_key(&edge.to) {
            return Err(PlanningError::new(
                crate::error::PlanningErrorCode::PlanValidationFailed,
                format!("target node '{}' not found", edge.to),
            ));
        }

        let already_exists = self
            .edges
            .iter()
            .any(|e| e.from == edge.from && e.to == edge.to && e.relationship == edge.relationship);
        if already_exists {
            return Ok(false);
        }

        let idx = self.edges.len();
        self.adjacency
            .entry(edge.from.clone())
            .or_default()
            .push(idx);
        self.reverse_adjacency
            .entry(edge.to.clone())
            .or_default()
            .push(idx);
        self.edges.push(edge);
        Ok(true)
    }

    /// Get a node by id.
    pub fn node(&self, id: &str) -> Option<&KnowledgeNode> {
        self.nodes.get(id)
    }

    /// Get a mutable reference to a node.
    pub fn node_mut(&mut self, id: &str) -> Option<&mut KnowledgeNode> {
        self.nodes.get_mut(id)
    }

    /// Iterate over all nodes.
    pub fn nodes(&self) -> impl Iterator<Item = &KnowledgeNode> {
        self.nodes.values()
    }

    /// Get all outgoing edges from a node.
    pub fn edges_from(&self, id: &str) -> Vec<&KnowledgeEdge> {
        self.adjacency
            .get(id)
            .map(|idxs| idxs.iter().map(|&i| &self.edges[i]).collect())
            .unwrap_or_default()
    }

    /// Get all incoming edges to a node.
    pub fn edges_to(&self, id: &str) -> Vec<&KnowledgeEdge> {
        self.reverse_adjacency
            .get(id)
            .map(|idxs| idxs.iter().map(|&i| &self.edges[i]).collect())
            .unwrap_or_default()
    }

    /// Get successor node ids (nodes reachable via one outgoing edge).
    pub fn successors(&self, id: &str) -> Vec<&str> {
        self.edges_from(id).iter().map(|e| e.to.as_str()).collect()
    }

    /// Get predecessor node ids (nodes with edges pointing to id).
    pub fn predecessors(&self, id: &str) -> Vec<&str> {
        self.edges_to(id).iter().map(|e| e.from.as_str()).collect()
    }

    /// Remove a node and all incident edges.
    pub fn remove_node(&mut self, id: &str) -> bool {
        if self.nodes.remove(id).is_none() {
            return false;
        }
        // Remove edges referencing this node
        let mut to_remove: Vec<usize> = Vec::new();
        for (i, edge) in self.edges.iter().enumerate() {
            if edge.from == id || edge.to == id {
                to_remove.push(i);
            }
        }
        for &i in to_remove.iter().rev() {
            self.edges.remove(i);
        }
        self.adjacency.remove(id);
        self.reverse_adjacency.remove(id);
        // Rebuild adjacency indices (simplified: just rebuild everything)
        self.adjacency.clear();
        self.reverse_adjacency.clear();
        for (i, edge) in self.edges.iter().enumerate() {
            self.adjacency.entry(edge.from.clone()).or_default().push(i);
            self.reverse_adjacency
                .entry(edge.to.clone())
                .or_default()
                .push(i);
        }
        true
    }

    /// Total number of nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Total number of edges.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Check whether a node exists.
    pub fn has_node(&self, id: &str) -> bool {
        self.nodes.contains_key(id)
    }

    /// Check if the graph is empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Find nodes whose name contains the query (case-insensitive).
    pub fn search_by_name(&self, query: &str) -> Vec<&KnowledgeNode> {
        let q = query.to_lowercase();
        self.nodes
            .values()
            .filter(|n| n.name.to_lowercase().contains(&q))
            .collect()
    }

    /// Find nodes with a given type.
    pub fn nodes_by_type(&self, node_type: &KnowledgeNodeType) -> Vec<&KnowledgeNode> {
        self.nodes
            .values()
            .filter(|n| n.node_type == *node_type)
            .collect()
    }

    /// Find nodes with a given tag.
    pub fn nodes_with_tag(&self, tag: &str) -> Vec<&KnowledgeNode> {
        self.nodes
            .values()
            .filter(|n| n.tags.contains(&tag.to_string()))
            .collect()
    }

    /// Get all edges.
    pub fn edges(&self) -> &[KnowledgeEdge] {
        &self.edges
    }

    /// Find edges with a given relationship.
    pub fn edges_with_relationship(&self, relationship: &str) -> Vec<&KnowledgeEdge> {
        self.edges
            .iter()
            .filter(|e| e.relationship == relationship)
            .collect()
    }
}

impl Default for PlanningKnowledgeGraph {
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

    // KnowledgeNodeType tests

    #[test]
    fn knowledge_node_type_display() {
        assert_eq!(KnowledgeNodeType::Concept.to_string(), "concept");
        assert_eq!(KnowledgeNodeType::Entity.to_string(), "entity");
        assert_eq!(KnowledgeNodeType::Fact.to_string(), "fact");
        assert_eq!(
            KnowledgeNodeType::Custom("x".to_string()).to_string(),
            "custom(x)"
        );
    }

    // KnowledgeNode tests

    #[test]
    fn node_creation() {
        let n = KnowledgeNode::new("n1", "node1", KnowledgeNodeType::Concept);
        assert_eq!(n.id, "n1");
        assert_eq!(n.name, "node1");
        assert_eq!(n.node_type, KnowledgeNodeType::Concept);
        assert_eq!(n.confidence, 1.0);
    }

    #[test]
    fn node_builder() {
        let n = KnowledgeNode::new("n", "name", KnowledgeNodeType::Entity)
            .with_description("desc")
            .with_confidence(0.8)
            .with_source("user")
            .with_property("key", serde_json::json!("value"))
            .with_tag("important");
        assert_eq!(n.description, "desc");
        assert!((n.confidence - 0.8).abs() < f64::EPSILON);
        assert_eq!(n.source, "user");
        assert_eq!(n.properties.get("key").unwrap(), "value");
        assert!(n.tags.contains(&"important".to_string()));
    }

    #[test]
    fn node_confidence_clamped() {
        let n = KnowledgeNode::new("n", "n", KnowledgeNodeType::Concept).with_confidence(5.0);
        assert!((n.confidence - 1.0).abs() < f64::EPSILON);
    }

    // KnowledgeEdge tests

    #[test]
    fn edge_creation() {
        let e = KnowledgeEdge::new("a", "b", "related_to");
        assert_eq!(e.from, "a");
        assert_eq!(e.to, "b");
        assert_eq!(e.relationship, "related_to");
        assert!((e.weight - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn edge_builder() {
        let e = KnowledgeEdge::new("a", "b", "causes")
            .with_weight(0.7)
            .with_property("strength", serde_json::json!("strong"));
        assert!((e.weight - 0.7).abs() < f64::EPSILON);
        assert_eq!(e.properties.get("strength").unwrap(), "strong");
    }

    // PlanningKnowledgeGraph tests

    #[test]
    fn graph_new_is_empty() {
        let g = PlanningKnowledgeGraph::new();
        assert!(g.is_empty());
        assert_eq!(g.node_count(), 0);
        assert_eq!(g.edge_count(), 0);
    }

    #[test]
    fn graph_add_node() {
        let mut g = PlanningKnowledgeGraph::new();
        assert!(g.add_node(KnowledgeNode::new("a", "A", KnowledgeNodeType::Concept)));
        assert!(!g.add_node(KnowledgeNode::new("a", "A2", KnowledgeNodeType::Concept)));
        assert!(g.has_node("a"));
        assert_eq!(g.node_count(), 1);
    }

    #[test]
    fn graph_add_edge_basic() {
        let mut g = PlanningKnowledgeGraph::new();
        g.add_node(KnowledgeNode::new("a", "A", KnowledgeNodeType::Concept));
        g.add_node(KnowledgeNode::new("b", "B", KnowledgeNodeType::Concept));
        assert!(g
            .add_edge(KnowledgeEdge::new("a", "b", "related_to"))
            .unwrap());
        assert!(!g
            .add_edge(KnowledgeEdge::new("a", "b", "related_to"))
            .unwrap());
        assert_eq!(g.edge_count(), 1);
    }

    #[test]
    fn graph_add_edge_missing_node() {
        let mut g = PlanningKnowledgeGraph::new();
        g.add_node(KnowledgeNode::new("a", "A", KnowledgeNodeType::Concept));
        let result = g.add_edge(KnowledgeEdge::new("a", "b", "rel"));
        assert!(result.is_err());
    }

    #[test]
    fn graph_remove_node() {
        let mut g = PlanningKnowledgeGraph::new();
        g.add_node(KnowledgeNode::new("a", "A", KnowledgeNodeType::Concept));
        g.add_node(KnowledgeNode::new("b", "B", KnowledgeNodeType::Concept));
        g.add_edge(KnowledgeEdge::new("a", "b", "rel")).unwrap();
        assert!(g.remove_node("a"));
        assert!(!g.has_node("a"));
        assert_eq!(g.node_count(), 1);
        assert_eq!(g.edge_count(), 0);
    }

    #[test]
    fn graph_node_lookup() {
        let mut g = PlanningKnowledgeGraph::new();
        g.add_node(KnowledgeNode::new("n1", "Node1", KnowledgeNodeType::Fact));
        assert!(g.node("n1").is_some());
        assert_eq!(g.node("n1").unwrap().name, "Node1");
        assert!(g.node("missing").is_none());
    }

    #[test]
    fn graph_node_mut() {
        let mut g = PlanningKnowledgeGraph::new();
        g.add_node(KnowledgeNode::new("n1", "Node1", KnowledgeNodeType::Fact));
        g.node_mut("n1").unwrap().name = "Changed".to_string();
        assert_eq!(g.node("n1").unwrap().name, "Changed");
    }

    #[test]
    fn graph_nodes_iter() {
        let mut g = PlanningKnowledgeGraph::new();
        g.add_node(KnowledgeNode::new("a", "A", KnowledgeNodeType::Concept));
        g.add_node(KnowledgeNode::new("b", "B", KnowledgeNodeType::Entity));
        assert_eq!(g.nodes().count(), 2);
    }

    #[test]
    fn graph_edges_from_and_to() {
        let mut g = PlanningKnowledgeGraph::new();
        g.add_node(KnowledgeNode::new("a", "A", KnowledgeNodeType::Concept));
        g.add_node(KnowledgeNode::new("b", "B", KnowledgeNodeType::Concept));
        g.add_node(KnowledgeNode::new("c", "C", KnowledgeNodeType::Concept));
        g.add_edge(KnowledgeEdge::new("a", "b", "rel1")).unwrap();
        g.add_edge(KnowledgeEdge::new("c", "b", "rel2")).unwrap();
        assert_eq!(g.edges_from("a").len(), 1);
        assert_eq!(g.edges_to("b").len(), 2);
    }

    #[test]
    fn graph_successors_and_predecessors() {
        let mut g = PlanningKnowledgeGraph::new();
        g.add_node(KnowledgeNode::new("a", "A", KnowledgeNodeType::Concept));
        g.add_node(KnowledgeNode::new("b", "B", KnowledgeNodeType::Concept));
        g.add_edge(KnowledgeEdge::new("a", "b", "rel")).unwrap();
        let mut succ = g.successors("a");
        assert_eq!(succ.len(), 1);
        assert_eq!(succ.pop().unwrap(), "b");
        let mut pred = g.predecessors("b");
        assert_eq!(pred.len(), 1);
        assert_eq!(pred.pop().unwrap(), "a");
    }

    #[test]
    fn graph_search_by_name() {
        let mut g = PlanningKnowledgeGraph::new();
        g.add_node(KnowledgeNode::new(
            "a",
            "Fast Algorithm",
            KnowledgeNodeType::Concept,
        ));
        g.add_node(KnowledgeNode::new(
            "b",
            "Slow Process",
            KnowledgeNodeType::Concept,
        ));
        let results = g.search_by_name("fast");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "a");
    }

    #[test]
    fn graph_nodes_by_type() {
        let mut g = PlanningKnowledgeGraph::new();
        g.add_node(KnowledgeNode::new("a", "A", KnowledgeNodeType::Concept));
        g.add_node(KnowledgeNode::new("b", "B", KnowledgeNodeType::Entity));
        g.add_node(KnowledgeNode::new("c", "C", KnowledgeNodeType::Concept));
        assert_eq!(g.nodes_by_type(&KnowledgeNodeType::Concept).len(), 2);
    }

    #[test]
    fn graph_nodes_with_tag() {
        let mut g = PlanningKnowledgeGraph::new();
        g.add_node(KnowledgeNode::new("a", "A", KnowledgeNodeType::Concept).with_tag("important"));
        g.add_node(KnowledgeNode::new("b", "B", KnowledgeNodeType::Concept));
        assert_eq!(g.nodes_with_tag("important").len(), 1);
    }

    #[test]
    fn graph_edges_with_relationship() {
        let mut g = PlanningKnowledgeGraph::new();
        g.add_node(KnowledgeNode::new("a", "A", KnowledgeNodeType::Concept));
        g.add_node(KnowledgeNode::new("b", "B", KnowledgeNodeType::Concept));
        g.add_node(KnowledgeNode::new("c", "C", KnowledgeNodeType::Concept));
        g.add_edge(KnowledgeEdge::new("a", "b", "causes")).unwrap();
        g.add_edge(KnowledgeEdge::new("b", "c", "related_to"))
            .unwrap();
        assert_eq!(g.edges_with_relationship("causes").len(), 1);
    }

    #[test]
    fn graph_default() {
        let g = PlanningKnowledgeGraph::default();
        assert!(g.is_empty());
    }

    // Serialization tests

    #[test]
    fn node_serialization_roundtrip() {
        let n = KnowledgeNode::new("n1", "Node1", KnowledgeNodeType::Fact).with_confidence(0.8);
        let json = serde_json::to_string(&n).unwrap();
        let back: KnowledgeNode = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "n1");
        assert!((back.confidence - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn edge_serialization_roundtrip() {
        let e = KnowledgeEdge::new("a", "b", "rel").with_weight(0.5);
        let json = serde_json::to_string(&e).unwrap();
        let back: KnowledgeEdge = serde_json::from_str(&json).unwrap();
        assert_eq!(back.from, "a");
        assert!((back.weight - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn graph_serialization_roundtrip() {
        let mut g = PlanningKnowledgeGraph::new();
        g.add_node(KnowledgeNode::new("a", "A", KnowledgeNodeType::Concept));
        g.add_node(KnowledgeNode::new("b", "B", KnowledgeNodeType::Entity));
        g.add_edge(KnowledgeEdge::new("a", "b", "rel")).unwrap();
        let json = serde_json::to_string(&g).unwrap();
        let back: PlanningKnowledgeGraph = serde_json::from_str(&json).unwrap();
        assert_eq!(back.node_count(), 2);
        assert_eq!(back.edge_count(), 1);
    }
}
