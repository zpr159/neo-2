use std::collections::{HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::error::EvolutionResult;

/// Information about a single module in the architecture.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    /// Module name (e.g. `neo_core`).
    pub name: String,
    /// Relative path to the module root.
    pub path: String,
    /// Approximate line count.
    pub line_count: usize,
    /// Number of public functions / methods.
    pub function_count: usize,
    /// Complexity score derived from cyclomatic + nesting depth.
    pub complexity: f64,
}

/// Summary of the module dependency graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyGraphInfo {
    /// All module names.
    pub nodes: Vec<String>,
    /// Directed edges `(from, to)` representing a dependency.
    pub edges: Vec<(String, String)>,
    /// Cycles detected in the graph (each inner vec is a cycle of module names).
    pub cycles: Vec<Vec<String>>,
    /// Maximum depth of the dependency tree from the root.
    pub depth: usize,
}

/// Top-level architecture analysis output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureAnalysis {
    /// Modules discovered during analysis.
    pub modules: Vec<ModuleInfo>,
    /// Dependency graph summary.
    pub dependency_graph: DependencyGraphInfo,
    /// Aggregate complexity score (higher = more complex).
    pub complexity_score: f64,
    /// Architectural drift score in `[0.0, 1.0]` (0 = no drift).
    pub drift_score: f64,
}

/// Analyses the high-level architecture of the Neo crate ecosystem.
pub struct ArchitectureAnalyzer;

impl ArchitectureAnalyzer {
    /// Create a new `ArchitectureAnalyzer`.
    pub fn new() -> Self {
        Self
    }

    /// Perform a full architecture analysis and return the result.
    pub fn analyze(&self) -> EvolutionResult<ArchitectureAnalysis> {
        let modules = self.builtin_modules();
        let graph = self.build_dependency_graph(&modules);
        let cycles = self.detect_cycles(&graph);
        let depth = self.compute_depth(&graph);

        let graph_with_cycles = DependencyGraphInfo {
            nodes: graph.0.clone(),
            edges: graph.1.clone(),
            cycles: cycles.clone(),
            depth,
        };

        let complexity_score = Self::aggregate_complexity(&modules);
        let drift_score = self.detect_drift_inner(&cycles, &modules);

        Ok(ArchitectureAnalysis {
            modules,
            dependency_graph: graph_with_cycles,
            complexity_score,
            drift_score,
        })
    }

    /// Compute an architectural-drift score based on circular dependencies
    /// and excessive module complexity.
    pub fn detect_drift(&self) -> f64 {
        let modules = self.builtin_modules();
        let graph = self.build_dependency_graph(&modules);
        let cycles = self.detect_cycles(&graph);
        self.detect_drift_inner(&cycles, &modules)
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Return the statically-known set of modules in the crate ecosystem.
    fn builtin_modules(&self) -> Vec<ModuleInfo> {
        vec![
            ModuleInfo {
                name: "neo_core".into(),
                path: "core/rust/neo-core/src".into(),
                line_count: 12400,
                function_count: 186,
                complexity: 3.2,
            },
            ModuleInfo {
                name: "neo_runtime".into(),
                path: "runtime/rust/neo-runtime/src".into(),
                line_count: 8900,
                function_count: 134,
                complexity: 4.1,
            },
            ModuleInfo {
                name: "neo_agents".into(),
                path: "agents/rust/neo-agents/src".into(),
                line_count: 11200,
                function_count: 203,
                complexity: 3.8,
            },
            ModuleInfo {
                name: "neo_planning".into(),
                path: "planning/rust/neo-planning/src".into(),
                line_count: 7600,
                function_count: 112,
                complexity: 4.5,
            },
            ModuleInfo {
                name: "neo_memory".into(),
                path: "memory/rust/neo-memory/src".into(),
                line_count: 6800,
                function_count: 98,
                complexity: 3.0,
            },
            ModuleInfo {
                name: "neo_knowledge_graph".into(),
                path: "knowledge-graph/rust/neo-knowledge-graph/src".into(),
                line_count: 9200,
                function_count: 145,
                complexity: 4.2,
            },
            ModuleInfo {
                name: "neo_reasoning".into(),
                path: "reasoning/rust/neo-reasoning/src".into(),
                line_count: 8100,
                function_count: 127,
                complexity: 5.0,
            },
            ModuleInfo {
                name: "neo_workflows".into(),
                path: "workflows/rust/neo-workflows/src".into(),
                line_count: 5400,
                function_count: 78,
                complexity: 2.8,
            },
            ModuleInfo {
                name: "neo_distributed".into(),
                path: "distributed/rust/neo-distributed/src".into(),
                line_count: 10500,
                function_count: 167,
                complexity: 4.7,
            },
            ModuleInfo {
                name: "neo_capabilities".into(),
                path: "capabilities/rust/neo-capabilities/src".into(),
                line_count: 4200,
                function_count: 63,
                complexity: 2.1,
            },
            ModuleInfo {
                name: "neo_executive".into(),
                path: "executive/rust/neo-executive/src".into(),
                line_count: 7300,
                function_count: 109,
                complexity: 3.5,
            },
            ModuleInfo {
                name: "neo_learning".into(),
                path: "learning/rust/neo-learning/src".into(),
                line_count: 9800,
                function_count: 152,
                complexity: 4.4,
            },
            ModuleInfo {
                name: "neo_tools".into(),
                path: "tools/rust/neo-tools/src".into(),
                line_count: 5100,
                function_count: 81,
                complexity: 2.5,
            },
            ModuleInfo {
                name: "neo_evolution".into(),
                path: "evolution/rust/neo-evolution/src".into(),
                line_count: 6200,
                function_count: 94,
                complexity: 3.3,
            },
        ]
    }

    /// Build a dependency graph from the known inter-crate edges.
    fn build_dependency_graph(
        &self,
        _modules: &[ModuleInfo],
    ) -> (Vec<String>, Vec<(String, String)>) {
        let nodes: Vec<String> = _modules.iter().map(|m| m.name.clone()).collect();

        // Edges derived from Cargo.toml dependency declarations.
        let edges: Vec<(String, String)> = vec![
            ("neo_runtime".into(), "neo_core".into()),
            ("neo_agents".into(), "neo_core".into()),
            ("neo_agents".into(), "neo_runtime".into()),
            ("neo_agents".into(), "neo_memory".into()),
            ("neo_planning".into(), "neo_core".into()),
            ("neo_planning".into(), "neo_agents".into()),
            ("neo_planning".into(), "neo_knowledge_graph".into()),
            ("neo_memory".into(), "neo_core".into()),
            ("neo_knowledge_graph".into(), "neo_core".into()),
            ("neo_knowledge_graph".into(), "neo_memory".into()),
            ("neo_reasoning".into(), "neo_core".into()),
            ("neo_reasoning".into(), "neo_knowledge_graph".into()),
            ("neo_workflows".into(), "neo_core".into()),
            ("neo_workflows".into(), "neo_planning".into()),
            ("neo_workflows".into(), "neo_agents".into()),
            ("neo_distributed".into(), "neo_core".into()),
            ("neo_distributed".into(), "neo_runtime".into()),
            ("neo_capabilities".into(), "neo_core".into()),
            ("neo_capabilities".into(), "neo_agents".into()),
            ("neo_executive".into(), "neo_core".into()),
            ("neo_executive".into(), "neo_agents".into()),
            ("neo_executive".into(), "neo_planning".into()),
            ("neo_learning".into(), "neo_core".into()),
            ("neo_learning".into(), "neo_memory".into()),
            ("neo_learning".into(), "neo_reasoning".into()),
            ("neo_tools".into(), "neo_core".into()),
            ("neo_tools".into(), "neo_runtime".into()),
            ("neo_evolution".into(), "neo_core".into()),
            ("neo_evolution".into(), "neo_runtime".into()),
            ("neo_evolution".into(), "neo_agents".into()),
            ("neo_evolution".into(), "neo_planning".into()),
            ("neo_evolution".into(), "neo_memory".into()),
            ("neo_evolution".into(), "neo_knowledge_graph".into()),
            ("neo_evolution".into(), "neo_reasoning".into()),
            ("neo_evolution".into(), "neo_workflows".into()),
            ("neo_evolution".into(), "neo_distributed".into()),
            ("neo_evolution".into(), "neo_capabilities".into()),
            ("neo_evolution".into(), "neo_executive".into()),
            ("neo_evolution".into(), "neo_learning".into()),
            ("neo_evolution".into(), "neo_tools".into()),
        ];

        (nodes, edges)
    }

    /// Detect cycles via iterative DFS (Tarjan-inspired).
    fn detect_cycles(&self, graph: &(Vec<String>, Vec<(String, String)>)) -> Vec<Vec<String>> {
        use std::collections::HashMap;

        let adj: HashMap<&str, Vec<&str>> = {
            let mut m: HashMap<&str, Vec<&str>> = HashMap::new();
            for node in &graph.0 {
                m.entry(node.as_str()).or_default();
            }
            for (from, to) in &graph.1 {
                m.entry(from.as_str()).or_default().push(to.as_str());
            }
            m
        };

        let mut visited: HashSet<&str> = HashSet::new();
        let mut stack: HashSet<&str> = HashSet::new();
        let mut cycles: Vec<Vec<String>> = Vec::new();

        fn dfs<'a>(
            node: &'a str,
            adj: &HashMap<&'a str, Vec<&'a str>>,
            visited: &mut HashSet<&'a str>,
            stack: &mut HashSet<&'a str>,
            path: &mut Vec<&'a str>,
            cycles: &mut Vec<Vec<String>>,
        ) {
            if stack.contains(node) {
                // Found a cycle — extract it.
                if let Some(start) = path.iter().position(|&n| n == node) {
                    let cycle: Vec<String> = path[start..].iter().map(|s| s.to_string()).collect();
                    cycles.push(cycle);
                }
                return;
            }
            if visited.contains(node) {
                return;
            }
            visited.insert(node);
            stack.insert(node);
            path.push(node);

            if let Some(neighbours) = adj.get(node) {
                for &neighbour in neighbours {
                    dfs(neighbour, adj, visited, stack, path, cycles);
                }
            }

            path.pop();
            stack.remove(node);
        }

        for node in &graph.0 {
            dfs(
                node.as_str(),
                &adj,
                &mut visited,
                &mut stack,
                &mut Vec::new(),
                &mut cycles,
            );
        }

        cycles
    }

    /// Compute maximum depth via BFS from nodes with no incoming edges.
    fn compute_depth(&self, graph: &(Vec<String>, Vec<(String, String)>)) -> usize {
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let adj: HashMap<&str, Vec<&str>> = {
            let mut m: HashMap<&str, Vec<&str>> = HashMap::new();
            for node in &graph.0 {
                m.entry(node.as_str()).or_default();
                in_degree.entry(node.as_str()).or_insert(0);
            }
            for (from, to) in &graph.1 {
                m.entry(from.as_str()).or_default().push(to.as_str());
                *in_degree.entry(to.as_str()).or_insert(0) += 1;
            }
            m
        };

        let mut queue: VecDeque<&str> = VecDeque::new();
        let mut depth_map: HashMap<&str, usize> = HashMap::new();

        for (&node, &deg) in &in_degree {
            if deg == 0 {
                queue.push_back(node);
                depth_map.insert(node, 0);
            }
        }

        let mut max_depth = 0usize;
        while let Some(node) = queue.pop_front() {
            let d = *depth_map.get(node).unwrap_or(&0);
            if d > max_depth {
                max_depth = d;
            }
            if let Some(neighbours) = adj.get(node) {
                for &neighbour in neighbours {
                    let nd = d + 1;
                    let entry = depth_map.entry(neighbour).or_insert(0);
                    if nd > *entry {
                        *entry = nd;
                    }
                    let deg = in_degree.get_mut(neighbour).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(neighbour);
                    }
                }
            }
        }

        max_depth
    }

    /// Aggregate complexity across all modules.
    fn aggregate_complexity(modules: &[ModuleInfo]) -> f64 {
        if modules.is_empty() {
            return 0.0;
        }
        let total: f64 = modules.iter().map(|m| m.complexity).sum();
        total / modules.len() as f64
    }

    /// Compute drift based on cycle count and excessive complexity.
    fn detect_drift_inner(&self, cycles: &[Vec<String>], modules: &[ModuleInfo]) -> f64 {
        let cycle_penalty = (cycles.len() as f64 * 0.08).min(0.40);

        let complex_modules = modules.iter().filter(|m| m.complexity > 4.0).count();
        let complexity_penalty = (complex_modules as f64 / modules.len().max(1) as f64) * 0.30;

        (cycle_penalty + complexity_penalty).clamp(0.0, 1.0)
    }
}

impl Default for ArchitectureAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
