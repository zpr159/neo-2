use std::collections::{HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::analysis::architecture_analyzer::DependencyGraphInfo;
use crate::error::EvolutionResult;

/// Full dependency analysis output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyAnalysis {
    /// The full dependency graph.
    pub graph: DependencyGraphInfo,
    /// Modules that no other module depends on.
    pub orphans: Vec<String>,
    /// Modules on the critical (longest) path.
    pub critical_path: Vec<String>,
    /// Detected circular dependencies.
    pub circular_deps: Vec<Vec<String>>,
}

/// Analyses inter-module dependencies for orphans, critical paths,
/// and circular references.
pub struct DependencyAnalyzer {
    nodes: Vec<String>,
    edges: Vec<(String, String)>,
}

impl DependencyAnalyzer {
    /// Create a new `DependencyAnalyzer` seeded with the known crate graph.
    pub fn new() -> Self {
        let (nodes, edges) = default_graph();
        Self { nodes, edges }
    }

    /// Run a full dependency analysis.
    pub fn analyze(&self) -> EvolutionResult<DependencyAnalysis> {
        let orphans = self.find_orphans();
        let critical_path = self.find_critical_path();
        let circular_deps = self.detect_circular_dependencies();

        let graph = DependencyGraphInfo {
            nodes: self.nodes.clone(),
            edges: self.edges.clone(),
            cycles: circular_deps.clone(),
            depth: critical_path.len(),
        };

        Ok(DependencyAnalysis {
            graph,
            orphans,
            critical_path,
            circular_deps,
        })
    }

    /// Identify modules that nothing else depends on (leaf nodes).
    pub fn find_orphans(&self) -> Vec<String> {
        let mut dependents: HashSet<&str> = HashSet::new();
        for (_from, to) in &self.edges {
            dependents.insert(to.as_str());
        }
        self.nodes
            .iter()
            .filter(|n| !dependents.contains(n.as_str()))
            .cloned()
            .collect()
    }

    /// Find the longest dependency chain (critical path) via BFS depth.
    pub fn find_critical_path(&self) -> Vec<String> {
        let adj = build_adjacency(&self.nodes, &self.edges);

        // BFS from each node with no incoming edges to find the longest path.
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        for n in &self.nodes {
            in_degree.entry(n.as_str()).or_insert(0);
        }
        for (_from, to) in &self.edges {
            *in_degree.entry(to.as_str()).or_insert(0) += 1;
        }

        let mut best_path: Vec<String> = Vec::new();

        for (&start, &deg) in &in_degree {
            if deg != 0 {
                continue;
            }
            // BFS to compute depth map.
            let mut depth: HashMap<&str, usize> = HashMap::new();
            let mut predecessor: HashMap<&str, &str> = HashMap::new();
            let mut queue: VecDeque<&str> = VecDeque::new();
            queue.push_back(start);
            depth.insert(start, 0);

            let mut furthest = start;
            let mut max_d = 0usize;

            while let Some(node) = queue.pop_front() {
                let d = *depth.get(node).unwrap_or(&0);
                if d > max_d {
                    max_d = d;
                    furthest = node;
                }
                if let Some(neighbours) = adj.get(node) {
                    for &nb in neighbours {
                        let nd = d + 1;
                        if nd > *depth.get(nb).unwrap_or(&0) {
                            depth.insert(nb, nd);
                            predecessor.insert(nb, node);
                            queue.push_back(nb);
                        }
                    }
                }
            }

            // Reconstruct path.
            let mut path = Vec::new();
            let mut cur = furthest;
            loop {
                path.push(cur.to_string());
                if let Some(&prev) = predecessor.get(cur) {
                    cur = prev;
                } else {
                    break;
                }
            }
            path.reverse();
            if path.len() > best_path.len() {
                best_path = path;
            }
        }

        best_path
    }

    /// Detect all strongly-connected components of size ≥ 2 (circular deps).
    pub fn detect_circular_dependencies(&self) -> Vec<Vec<String>> {
        // Kosaraju's algorithm.
        let n = self.nodes.len();
        let mut index_map: HashMap<&str, usize> = HashMap::new();
        for (i, node) in self.nodes.iter().enumerate() {
            index_map.insert(node.as_str(), i);
        }

        let adj = build_adjacency(&self.nodes, &self.edges);
        let radj = build_reverse_adjacency(&self.nodes, &self.edges);

        // First pass: fill order.
        let mut visited = vec![false; n];
        let mut order: Vec<usize> = Vec::new();

        fn dfs1(
            node: usize,
            adj: &HashMap<&str, Vec<&str>>,
            index_map: &HashMap<&str, usize>,
            nodes: &[String],
            visited: &mut [bool],
            order: &mut Vec<usize>,
        ) {
            visited[node] = true;
            let name = nodes[node].as_str();
            if let Some(neighbours) = adj.get(name) {
                for nb in neighbours {
                    if let Some(&idx) = index_map.get(nb) {
                        if !visited[idx] {
                            dfs1(idx, adj, index_map, nodes, visited, order);
                        }
                    }
                }
            }
            order.push(node);
        }

        for i in 0..n {
            if !visited[i] {
                dfs1(i, &adj, &index_map, &self.nodes, &mut visited, &mut order);
            }
        }

        // Second pass: assign components on reverse graph.
        visited = vec![false; n];
        let mut components: Vec<Vec<String>> = Vec::new();

        fn dfs2(
            node: usize,
            radj: &HashMap<&str, Vec<&str>>,
            index_map: &HashMap<&str, usize>,
            nodes: &[String],
            visited: &mut [bool],
            component: &mut Vec<String>,
        ) {
            visited[node] = true;
            component.push(nodes[node].clone());
            let name = nodes[node].as_str();
            if let Some(neighbours) = radj.get(name) {
                for nb in neighbours {
                    if let Some(&idx) = index_map.get(nb) {
                        if !visited[idx] {
                            dfs2(idx, radj, index_map, nodes, visited, component);
                        }
                    }
                }
            }
        }

        for &idx in order.iter().rev() {
            if !visited[idx] {
                let mut component = Vec::new();
                dfs2(
                    idx,
                    &radj,
                    &index_map,
                    &self.nodes,
                    &mut visited,
                    &mut component,
                );
                if component.len() > 1 {
                    component.sort();
                    components.push(component);
                }
            }
        }

        components
    }
}

impl Default for DependencyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn default_graph() -> (Vec<String>, Vec<(String, String)>) {
    let nodes = vec![
        "neo_core",
        "neo_runtime",
        "neo_agents",
        "neo_planning",
        "neo_memory",
        "neo_knowledge_graph",
        "neo_reasoning",
        "neo_workflows",
        "neo_distributed",
        "neo_capabilities",
        "neo_executive",
        "neo_learning",
        "neo_tools",
        "neo_evolution",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let edges: Vec<(String, String)> = vec![
        ("neo_runtime", "neo_core"),
        ("neo_agents", "neo_core"),
        ("neo_agents", "neo_runtime"),
        ("neo_agents", "neo_memory"),
        ("neo_planning", "neo_core"),
        ("neo_planning", "neo_agents"),
        ("neo_planning", "neo_knowledge_graph"),
        ("neo_memory", "neo_core"),
        ("neo_knowledge_graph", "neo_core"),
        ("neo_knowledge_graph", "neo_memory"),
        ("neo_reasoning", "neo_core"),
        ("neo_reasoning", "neo_knowledge_graph"),
        ("neo_workflows", "neo_core"),
        ("neo_workflows", "neo_planning"),
        ("neo_workflows", "neo_agents"),
        ("neo_distributed", "neo_core"),
        ("neo_distributed", "neo_runtime"),
        ("neo_capabilities", "neo_core"),
        ("neo_capabilities", "neo_agents"),
        ("neo_executive", "neo_core"),
        ("neo_executive", "neo_agents"),
        ("neo_executive", "neo_planning"),
        ("neo_learning", "neo_core"),
        ("neo_learning", "neo_memory"),
        ("neo_learning", "neo_reasoning"),
        ("neo_tools", "neo_core"),
        ("neo_tools", "neo_runtime"),
        ("neo_evolution", "neo_core"),
        ("neo_evolution", "neo_runtime"),
        ("neo_evolution", "neo_agents"),
        ("neo_evolution", "neo_planning"),
        ("neo_evolution", "neo_memory"),
        ("neo_evolution", "neo_knowledge_graph"),
        ("neo_evolution", "neo_reasoning"),
        ("neo_evolution", "neo_workflows"),
        ("neo_evolution", "neo_distributed"),
        ("neo_evolution", "neo_capabilities"),
        ("neo_evolution", "neo_executive"),
        ("neo_evolution", "neo_learning"),
        ("neo_evolution", "neo_tools"),
    ]
    .into_iter()
    .map(|(f, t)| (f.to_string(), t.to_string()))
    .collect();

    (nodes, edges)
}

fn build_adjacency<'a>(
    nodes: &'a [String],
    edges: &'a [(String, String)],
) -> HashMap<&'a str, Vec<&'a str>> {
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for node in nodes {
        adj.entry(node.as_str()).or_default();
    }
    for (from, to) in edges {
        adj.entry(from.as_str()).or_default().push(to.as_str());
    }
    adj
}

fn build_reverse_adjacency<'a>(
    nodes: &'a [String],
    edges: &'a [(String, String)],
) -> HashMap<&'a str, Vec<&'a str>> {
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for node in nodes {
        adj.entry(node.as_str()).or_default();
    }
    for (from, to) in edges {
        adj.entry(to.as_str()).or_default().push(from.as_str());
    }
    adj
}
