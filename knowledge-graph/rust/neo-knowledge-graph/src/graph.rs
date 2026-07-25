use std::collections::HashSet;

use dashmap::DashMap;
use tracing::info;

use crate::edge::{Edge, EdgeId};
use crate::node::{Node, NodeId};
use crate::schema::GraphSchema;

/// In-memory knowledge graph with optional sled persistence.
pub struct KnowledgeGraph {
    nodes: DashMap<NodeId, Node>,
    edges: DashMap<EdgeId, Edge>,
    adjacency: DashMap<NodeId, HashSet<EdgeId>>,
    reverse_adjacency: DashMap<NodeId, HashSet<EdgeId>>,
    schema: GraphSchema,
    #[allow(dead_code)]
    db: Option<sled::Db>,
}

impl std::fmt::Debug for KnowledgeGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KnowledgeGraph")
            .field("node_count", &self.nodes.len())
            .field("edge_count", &self.edges.len())
            .finish()
    }
}

impl KnowledgeGraph {
    /// Create a new knowledge graph with the given schema.
    #[must_use]
    pub fn new(schema: GraphSchema) -> Self {
        Self {
            nodes: DashMap::new(),
            edges: DashMap::new(),
            adjacency: DashMap::new(),
            reverse_adjacency: DashMap::new(),
            schema,
            db: None,
        }
    }

    /// Insert a node and return its id.
    pub fn add_node(&self, node: Node) -> NodeId {
        let id = node.id;
        self.adjacency.entry(id).or_default();
        self.reverse_adjacency.entry(id).or_default();
        self.nodes.insert(id, node);
        info!(node_id = %id.0, "Added node to knowledge graph");
        id
    }

    /// Remove a node and all its connected edges. Returns true if it existed.
    pub fn remove_node(&self, id: NodeId) -> bool {
        if self.nodes.remove(&id).is_none() {
            return false;
        }
        if let Some((_, out_edges)) = self.adjacency.remove(&id) {
            for edge_id in out_edges {
                if let Some((_, edge)) = self.edges.remove(&edge_id) {
                    if let Some(mut rev) = self.reverse_adjacency.get_mut(&edge.target) {
                        rev.remove(&edge_id);
                    }
                }
            }
        }
        if let Some((_, in_edges)) = self.reverse_adjacency.remove(&id) {
            for edge_id in in_edges {
                if let Some((_, edge)) = self.edges.remove(&edge_id) {
                    if let Some(mut fwd) = self.adjacency.get_mut(&edge.source) {
                        fwd.remove(&edge_id);
                    }
                }
            }
        }
        true
    }

    /// Insert an edge and return its id.
    pub fn add_edge(&self, edge: Edge) -> EdgeId {
        let id = edge.id;
        self.adjacency
            .entry(edge.source)
            .or_default()
            .insert(id);
        self.reverse_adjacency
            .entry(edge.target)
            .or_default()
            .insert(id);
        self.edges.insert(id, edge);
        id
    }

    /// Remove an edge by id. Returns true if it existed.
    pub fn remove_edge(&self, id: EdgeId) -> bool {
        if let Some((_, edge)) = self.edges.remove(&id) {
            if let Some(mut fwd) = self.adjacency.get_mut(&edge.source) {
                fwd.remove(&id);
            }
            if let Some(mut rev) = self.reverse_adjacency.get_mut(&edge.target) {
                rev.remove(&id);
            }
            true
        } else {
            false
        }
    }

    /// Get a copy of a node by id.
    #[must_use]
    pub fn get_node(&self, id: NodeId) -> Option<Node> {
        self.nodes.get(&id).map(|n| n.value().clone())
    }

    /// Get a copy of an edge by id.
    #[must_use]
    pub fn get_edge(&self, id: EdgeId) -> Option<Edge> {
        self.edges.get(&id).map(|e| e.value().clone())
    }

    /// Return all neighbor node ids (both incoming and outgoing) of a node.
    #[must_use]
    pub fn neighbors(&self, id: NodeId) -> Vec<NodeId> {
        let mut result = Vec::new();
        if let Some(out) = self.adjacency.get(&id) {
            for edge_id in out.iter() {
                if let Some(edge) = self.edges.get(edge_id) {
                    result.push(edge.target);
                }
            }
        }
        if let Some(in_) = self.reverse_adjacency.get(&id) {
            for edge_id in in_.iter() {
                if let Some(edge) = self.edges.get(edge_id) {
                    result.push(edge.source);
                }
            }
        }
        result
    }

    /// Return node ids with edges pointing into the given node.
    #[must_use]
    pub fn in_neighbors(&self, id: NodeId) -> Vec<NodeId> {
        self.reverse_adjacency
            .get(&id)
            .map(|in_edges| {
                in_edges
                    .iter()
                    .filter_map(|eid| self.edges.get(eid).map(|e| e.source))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Return node ids that the given node has edges pointing to.
    #[must_use]
    pub fn out_neighbors(&self, id: NodeId) -> Vec<NodeId> {
        self.adjacency
            .get(&id)
            .map(|out_edges| {
                out_edges
                    .iter()
                    .filter_map(|eid| self.edges.get(eid).map(|e| e.target))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Total number of nodes.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Total number of edges.
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Access the schema.
    #[must_use]
    pub fn schema(&self) -> &GraphSchema {
        &self.schema
    }

    /// Collect all node ids.
    pub fn all_node_ids(&self) -> Vec<NodeId> {
        self.nodes.iter().map(|r| *r.key()).collect()
    }

    /// Collect all edge ids.
    pub fn all_edge_ids(&self) -> Vec<EdgeId> {
        self.edges.iter().map(|r| *r.key()).collect()
    }

    /// Collect all nodes into a Vec.
    pub fn collect_nodes(&self) -> Vec<Node> {
        self.nodes.iter().map(|r| r.value().clone()).collect()
    }

    /// Collect all edges into a Vec.
    pub fn collect_edges(&self) -> Vec<Edge> {
        self.edges.iter().map(|r| r.value().clone()).collect()
    }
}
