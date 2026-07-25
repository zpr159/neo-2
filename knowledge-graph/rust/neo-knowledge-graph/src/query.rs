use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Instant;

use uuid::Uuid;

use crate::edge::{Edge, EdgeId, EdgeType};
use crate::graph::KnowledgeGraph;
use crate::node::{Node, NodeId, NodeType};

/// Parameters for graph traversal queries.
#[derive(Debug, Clone, Default)]
pub struct TraversalPattern {
    /// Node to start the traversal from.
    pub start_node: Option<NodeId>,
    /// Filter traversal to these edge types.
    pub edge_types: Option<Vec<EdgeType>>,
    /// Filter results to these node types.
    pub node_types: Option<Vec<NodeType>>,
    /// Maximum depth of the traversal.
    pub max_depth: Option<u32>,
    /// Maximum number of results to return.
    pub max_results: Option<usize>,
}

/// Query types supported by the knowledge graph.
#[derive(Debug, Clone)]
pub enum GraphQuery {
    /// Find a node by its id.
    NodeById(NodeId),
    /// Find all nodes of a given type.
    NodesByType(NodeType),
    /// Find all edges of a given type.
    EdgesByType(EdgeType),
    /// Traverse the graph following a pattern.
    Traversal(TraversalPattern),
    /// Find nodes where a property equals a value.
    ByProperty(String, serde_json::Value),
    /// Find the shortest path between two nodes.
    ShortestPath(NodeId, NodeId),
}

/// Result of a graph query.
#[derive(Debug, Clone)]
pub struct QueryResult {
    /// Matching nodes.
    pub nodes: Vec<Node>,
    /// Matching edges.
    pub edges: Vec<Edge>,
    /// Paths found (e.g. for traversal or shortest-path queries).
    pub paths: Vec<Vec<NodeId>>,
    /// Query execution time in milliseconds.
    pub execution_time_ms: f64,
}

impl KnowledgeGraph {
    /// Execute a graph query and return results.
    #[must_use]
    pub fn query(&self, query: &GraphQuery) -> QueryResult {
        let start = Instant::now();
        let mut result = match query {
            GraphQuery::NodeById(id) => {
                let nodes = self.get_node(*id).into_iter().collect();
                QueryResult {
                    nodes,
                    edges: Vec::new(),
                    paths: Vec::new(),
                    execution_time_ms: 0.0,
                }
            }
            GraphQuery::NodesByType(node_type) => {
                let node_ids = self.all_node_ids();
                let nodes: Vec<Node> = node_ids
                    .into_iter()
                    .filter_map(|id| self.get_node(id))
                    .filter(|n| n.node_type == *node_type)
                    .collect();
                QueryResult {
                    nodes,
                    edges: Vec::new(),
                    paths: Vec::new(),
                    execution_time_ms: 0.0,
                }
            }
            GraphQuery::EdgesByType(edge_type) => {
                let edge_ids = self.all_edge_ids();
                let edges: Vec<Edge> = edge_ids
                    .into_iter()
                    .filter_map(|id| self.get_edge(id))
                    .filter(|e| e.edge_type == *edge_type)
                    .collect();
                QueryResult {
                    nodes: Vec::new(),
                    edges,
                    paths: Vec::new(),
                    execution_time_ms: 0.0,
                }
            }
            GraphQuery::Traversal(pattern) => self.execute_traversal(pattern),
            GraphQuery::ByProperty(key, value) => {
                let node_ids = self.all_node_ids();
                let nodes: Vec<Node> = node_ids
                    .into_iter()
                    .filter_map(|id| self.get_node(id))
                    .filter(|n| n.get_property(key).map_or(false, |v| v == value))
                    .collect();
                QueryResult {
                    nodes,
                    edges: Vec::new(),
                    paths: Vec::new(),
                    execution_time_ms: 0.0,
                }
            }
            GraphQuery::ShortestPath(from, to) => {
                let path = self.bfs_shortest_path(*from, *to);
                let nodes = path
                    .iter()
                    .filter_map(|id| self.get_node(*id))
                    .collect();
                QueryResult {
                    nodes,
                    edges: Vec::new(),
                    paths: if path.is_empty() {
                        Vec::new()
                    } else {
                        vec![path]
                    },
                    execution_time_ms: 0.0,
                }
            }
        };
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        result.execution_time_ms = elapsed;
        result
    }

    fn execute_traversal(&self, pattern: &TraversalPattern) -> QueryResult {
        let start = pattern.start_node.unwrap_or(NodeId(Uuid::nil()));
        let max_depth = pattern.max_depth.unwrap_or(5);
        let max_results = pattern.max_results.unwrap_or(100);

        let mut visited = HashSet::new();
        let mut queue: VecDeque<(NodeId, u32)> = VecDeque::new();
        let mut found_nodes = Vec::new();

        queue.push_back((start, 0));
        visited.insert(start);

        while let Some((current, depth)) = queue.pop_front() {
            if depth > max_depth || found_nodes.len() >= max_results {
                continue;
            }
            if let Some(node) = self.get_node(current) {
                let matches_type = pattern
                    .node_types
                    .as_ref()
                    .map_or(true, |types| types.contains(&node.node_type));
                if matches_type {
                    found_nodes.push(node);
                }
            }

            for neighbor_id in self.out_neighbors(current) {
                if visited.insert(neighbor_id) {
                    queue.push_back((neighbor_id, depth + 1));
                }
            }
        }

        QueryResult {
            nodes: found_nodes,
            edges: Vec::new(),
            paths: Vec::new(),
            execution_time_ms: 0.0,
        }
    }

    fn bfs_shortest_path(&self, from: NodeId, to: NodeId) -> Vec<NodeId> {
        if from == to {
            return vec![from];
        }
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut parent: HashMap<NodeId, NodeId> = HashMap::new();

        queue.push_back(from);
        visited.insert(from);

        while let Some(current) = queue.pop_front() {
            for neighbor in self.out_neighbors(current) {
                if visited.insert(neighbor) {
                    parent.insert(neighbor, current);
                    if neighbor == to {
                        let mut path = vec![to];
                        let mut cur = to;
                        while let Some(&p) = parent.get(&cur) {
                            path.push(p);
                            cur = p;
                        }
                        path.reverse();
                        return path;
                    }
                    queue.push_back(neighbor);
                }
            }
        }
        Vec::new()
    }
}
