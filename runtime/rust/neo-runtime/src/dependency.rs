//! Dependency resolver with directed acyclic graph, topological sorting,
//! circular dependency detection, optional dependencies, and version constraints.

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::{DependencyError, DependencyErrorKind};
use crate::lifecycle::ServiceId;

/// Version constraint using semantic versioning rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VersionConstraint {
    /// Exact version match (`==x.y.z`).
    Exact {
        major: u32,
        minor: u32,
        patch: u32,
    },
    /// Minimum version (`>=x.y.z`).
    AtLeast {
        major: u32,
        minor: u32,
        patch: u32,
    },
    /// Compatible range (`^x.y.z` — allows patch and minor bumps).
    Compatible {
        major: u32,
        minor: u32,
        patch: u32,
    },
    /// Range `[min, max]`.
    Range {
        min_major: u32,
        min_minor: u32,
        min_patch: u32,
        max_major: u32,
        max_minor: u32,
        max_patch: u32,
    },
    /// Any version is acceptable.
    Any,
}

impl VersionConstraint {
    /// Check whether the given version satisfies this constraint.
    pub fn matches(&self, major: u32, minor: u32, patch: u32) -> bool {
        match self {
            Self::Exact {
                major: m,
                minor: n,
                patch: p,
            } => major == *m && minor == *n && patch == *p,
            Self::AtLeast {
                major: m,
                minor: n,
                patch: p,
            } => {
                (major, minor, patch) >= (*m, *n, *p)
            }
            Self::Compatible {
                major: m,
                minor: n,
                patch: p,
            } => {
                major == *m
                    && ((minor == *n && patch >= *p) || minor > *n)
            }
            Self::Range {
                min_major,
                min_minor,
                min_patch,
                max_major,
                max_minor,
                max_patch,
            } => {
                let ver = (major, minor, patch);
                ver >= (*min_major, *min_minor, *min_patch)
                    && ver <= (*max_major, *max_minor, *max_patch)
            }
            Self::Any => true,
        }
    }
}

impl fmt::Display for VersionConstraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exact {
                major,
                minor,
                patch,
            } => write!(f, "=={}.{}.{}", major, minor, patch),
            Self::AtLeast {
                major,
                minor,
                patch,
            } => write!(f, ">={}.{}.{}", major, minor, patch),
            Self::Compatible {
                major,
                minor,
                patch,
            } => write!(f, "^{}.{}.{}", major, minor, patch),
            Self::Range {
                min_major,
                min_minor,
                min_patch,
                max_major,
                max_minor,
                max_patch,
            } => write!(
                f,
                "[{}.{}.{} .. {}.{}.{}]",
                min_major, min_minor, min_patch, max_major, max_minor, max_patch
            ),
            Self::Any => write!(f, "*"),
        }
    }
}

/// A single dependency edge in the graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    /// The service that is required.
    pub service_id: ServiceId,
    /// The service name (for human-readable resolution).
    pub service_name: String,
    /// Version constraint on the dependency.
    pub version_constraint: VersionConstraint,
    /// Whether the dependency is optional.
    pub optional: bool,
}

/// A node in the dependency graph.
#[derive(Debug, Clone)]
pub struct DependencyNode {
    pub id: ServiceId,
    pub name: String,
    pub version: (u32, u32, u32),
    pub dependencies: Vec<Dependency>,
}

/// Directed acyclic graph for service dependencies.
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    nodes: HashMap<ServiceId, DependencyNode>,
    name_index: HashMap<String, ServiceId>,
}

impl DependencyGraph {
    /// Create an empty dependency graph.
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            name_index: HashMap::new(),
        }
    }

    /// Register a service as a node in the graph.
    pub fn add_node(&mut self, id: ServiceId, name: impl Into<String>, version: (u32, u32, u32)) {
        let name = name.into();
        self.name_index.insert(name.clone(), id);
        self.nodes.insert(
            id,
            DependencyNode {
                id,
                name,
                version,
                dependencies: Vec::new(),
            },
        );
    }

    /// Add a dependency edge from `from_id` to the specified service.
    pub fn add_dependency(&mut self, from_id: ServiceId, dep: Dependency) {
        if let Some(node) = self.nodes.get_mut(&from_id) {
            node.dependencies.push(dep);
        }
    }

    /// Remove a node and all edges connected to it.
    pub fn remove_node(&mut self, id: ServiceId) {
        if let Some(node) = self.nodes.remove(&id) {
            self.name_index.remove(&node.name);
        }
        for node in self.nodes.values_mut() {
            node.dependencies.retain(|d| d.service_id != id);
        }
    }

    /// Look up a service ID by name.
    pub fn find_by_name(&self, name: &str) -> Option<ServiceId> {
        self.name_index.get(name).copied()
    }

    /// Get a node by ID.
    pub fn node(&self, id: ServiceId) -> Option<&DependencyNode> {
        self.nodes.get(&id)
    }

    /// Get all registered node IDs.
    pub fn node_ids(&self) -> Vec<ServiceId> {
        self.nodes.keys().copied().collect()
    }

    /// Get the direct dependencies of a service.
    pub fn dependencies_of(&self, id: ServiceId) -> Vec<&Dependency> {
        self.nodes
            .get(&id)
            .map(|n| n.dependencies.iter().collect())
            .unwrap_or_default()
    }

    /// Detect cycles using DFS. Returns the first cycle path found, if any.
    pub fn detect_cycle(&self) -> Option<Vec<ServiceId>> {
        let mut visited = HashSet::new();
        let mut stack = HashSet::new();
        let mut path = Vec::new();

        for &id in self.nodes.keys() {
            if !visited.contains(&id) {
                if self.dfs_cycle(id, &mut visited, &mut stack, &mut path) {
                    return Some(path);
                }
            }
        }
        None
    }

    fn dfs_cycle(
        &self,
        id: ServiceId,
        visited: &mut HashSet<ServiceId>,
        stack: &mut HashSet<ServiceId>,
        path: &mut Vec<ServiceId>,
    ) -> bool {
        visited.insert(id);
        stack.insert(id);
        path.push(id);

        if let Some(node) = self.nodes.get(&id) {
            for dep in &node.dependencies {
                if !dep.optional {
                    if !visited.contains(&dep.service_id) {
                        if self.dfs_cycle(dep.service_id, visited, stack, path) {
                            return true;
                        }
                    } else if stack.contains(&dep.service_id) {
                        path.push(dep.service_id);
                        return true;
                    }
                }
            }
        }

        stack.remove(&id);
        path.pop();
        false
    }

    /// Perform topological sort using Kahn's algorithm.
    ///
    /// Returns services in dependency order (dependencies first).
    /// Returns an error if a cycle is detected involving required dependencies.
    pub fn topological_sort(&self) -> Result<Vec<ServiceId>, DependencyError> {
        let mut in_degree: HashMap<ServiceId, usize> = HashMap::new();
        let mut adjacency: HashMap<ServiceId, Vec<ServiceId>> = HashMap::new();

        for &id in self.nodes.keys() {
            in_degree.entry(id).or_insert(0);
            adjacency.entry(id).or_default();
        }

        for node in self.nodes.values() {
            for dep in &node.dependencies {
                if !dep.optional {
                    adjacency
                        .entry(dep.service_id)
                        .or_default()
                        .push(node.id);
                    *in_degree.entry(node.id).or_insert(0) += 1;
                }
            }
        }

        let mut queue: VecDeque<ServiceId> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut sorted = Vec::new();

        while let Some(id) = queue.pop_front() {
            sorted.push(id);

            if let Some(children) = adjacency.get(&id) {
                for &child in children {
                    if let Some(deg) = in_degree.get_mut(&child) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(child);
                        }
                    }
                }
            }
        }

        if sorted.len() != self.nodes.len() {
            let cycle = self
                .detect_cycle()
                .map(|c| {
                    c.iter()
                        .map(|id| self.nodes.get(id).map_or("?".to_string(), |n| n.name.clone()))
                        .collect::<Vec<_>>()
                        .join(" -> ")
                })
                .unwrap_or_else(|| "unknown".to_string());

            return Err(DependencyError::new(
                DependencyErrorKind::CircularDependency,
                format!("circular dependency detected: {}", cycle),
            ));
        }

        Ok(sorted)
    }

    /// Resolve all dependencies, checking version constraints.
    ///
    /// Returns a list of (service_id, error) for any resolution failures.
    pub fn validate(&self) -> Vec<(ServiceId, DependencyError)> {
        let mut errors = Vec::new();

        for node in self.nodes.values() {
            for dep in &node.dependencies {
                if dep.optional {
                    continue;
                }

                if let Some(target) = self.nodes.get(&dep.service_id) {
                    if !dep
                        .version_constraint
                        .matches(target.version.0, target.version.1, target.version.2)
                    {
                        errors.push((
                            node.id,
                            DependencyError::new(
                                DependencyErrorKind::VersionMismatch,
                                format!(
                                    "service '{}' requires '{}' {} but found {}.{}.{}",
                                    node.name,
                                    dep.service_name,
                                    dep.version_constraint,
                                    target.version.0,
                                    target.version.1,
                                    target.version.2
                                ),
                            ),
                        ));
                    }
                } else {
                    errors.push((
                        node.id,
                        DependencyError::new(
                            DependencyErrorKind::MissingDependency,
                            format!(
                                "service '{}' depends on '{}' which is not registered",
                                node.name, dep.service_name
                            ),
                        ),
                    ));
                }
            }
        }

        errors
    }

    /// Get the total number of nodes in the graph.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Check whether the graph is empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_id() -> ServiceId {
        ServiceId::new()
    }

    #[test]
    fn empty_graph() {
        let graph = DependencyGraph::new();
        assert!(graph.is_empty());
        assert!(graph.detect_cycle().is_none());
        let sorted = graph.topological_sort().unwrap();
        assert!(sorted.is_empty());
    }

    #[test]
    fn linear_dependency() {
        let mut graph = DependencyGraph::new();
        let a = make_id();
        let b = make_id();
        let c = make_id();

        graph.add_node(a, "a", (1, 0, 0));
        graph.add_node(b, "b", (1, 0, 0));
        graph.add_node(c, "c", (1, 0, 0));

        graph.add_dependency(
            a,
            Dependency {
                service_id: b,
                service_name: "b".to_string(),
                version_constraint: VersionConstraint::Any,
                optional: false,
            },
        );
        graph.add_dependency(
            b,
            Dependency {
                service_id: c,
                service_name: "c".to_string(),
                version_constraint: VersionConstraint::Any,
                optional: false,
            },
        );

        let sorted = graph.topological_sort().unwrap();
        let pos_c = sorted.iter().position(|&id| id == c).unwrap();
        let pos_b = sorted.iter().position(|&id| id == b).unwrap();
        let pos_a = sorted.iter().position(|&id| id == a).unwrap();
        assert!(pos_c < pos_b);
        assert!(pos_b < pos_a);
    }

    #[test]
    fn cycle_detection() {
        let mut graph = DependencyGraph::new();
        let a = make_id();
        let b = make_id();

        graph.add_node(a, "a", (1, 0, 0));
        graph.add_node(b, "b", (1, 0, 0));

        graph.add_dependency(
            a,
            Dependency {
                service_id: b,
                service_name: "b".to_string(),
                version_constraint: VersionConstraint::Any,
                optional: false,
            },
        );
        graph.add_dependency(
            b,
            Dependency {
                service_id: a,
                service_name: "a".to_string(),
                version_constraint: VersionConstraint::Any,
                optional: false,
            },
        );

        assert!(graph.detect_cycle().is_some());
        assert!(graph.topological_sort().is_err());
    }

    #[test]
    fn optional_dependency_ignored_in_sort() {
        let mut graph = DependencyGraph::new();
        let a = make_id();
        let b = make_id();

        graph.add_node(a, "a", (1, 0, 0));
        graph.add_node(b, "b", (1, 0, 0));

        graph.add_dependency(
            a,
            Dependency {
                service_id: b,
                service_name: "b".to_string(),
                version_constraint: VersionConstraint::Any,
                optional: true,
            },
        );

        let sorted = graph.topological_sort().unwrap();
        assert_eq!(sorted.len(), 2);
    }

    #[test]
    fn version_constraint_exact() {
        let c = VersionConstraint::Exact {
            major: 1,
            minor: 2,
            patch: 3,
        };
        assert!(c.matches(1, 2, 3));
        assert!(!c.matches(1, 2, 4));
        assert!(!c.matches(1, 3, 0));
    }

    #[test]
    fn version_constraint_at_least() {
        let c = VersionConstraint::AtLeast {
            major: 1,
            minor: 2,
            patch: 3,
        };
        assert!(c.matches(1, 2, 3));
        assert!(c.matches(1, 2, 4));
        assert!(c.matches(1, 3, 0));
        assert!(c.matches(2, 0, 0));
        assert!(!c.matches(1, 2, 2));
        assert!(!c.matches(1, 1, 0));
    }

    #[test]
    fn version_constraint_compatible() {
        let c = VersionConstraint::Compatible {
            major: 1,
            minor: 2,
            patch: 3,
        };
        assert!(c.matches(1, 2, 3));
        assert!(c.matches(1, 2, 5));
        assert!(c.matches(1, 3, 0));
        assert!(!c.matches(1, 1, 0));
        assert!(!c.matches(2, 0, 0));
    }

    #[test]
    fn version_constraint_range() {
        let c = VersionConstraint::Range {
            min_major: 1,
            min_minor: 0,
            min_patch: 0,
            max_major: 2,
            max_minor: 0,
            max_patch: 0,
        };
        assert!(c.matches(1, 5, 3));
        assert!(c.matches(2, 0, 0));
        assert!(!c.matches(2, 0, 1));
        assert!(!c.matches(0, 9, 0));
    }

    #[test]
    fn version_constraint_any() {
        assert!(VersionConstraint::Any.matches(0, 0, 0));
        assert!(VersionConstraint::Any.matches(999, 999, 999));
    }

    #[test]
    fn validate_missing_dependency() {
        let mut graph = DependencyGraph::new();
        let a = make_id();
        let missing = make_id();

        graph.add_node(a, "a", (1, 0, 0));
        graph.add_dependency(
            a,
            Dependency {
                service_id: missing,
                service_name: "missing".to_string(),
                version_constraint: VersionConstraint::Any,
                optional: false,
            },
        );

        let errors = graph.validate();
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0].1.kind,
            DependencyErrorKind::MissingDependency
        ));
    }

    #[test]
    fn validate_version_mismatch() {
        let mut graph = DependencyGraph::new();
        let a = make_id();
        let b = make_id();

        graph.add_node(a, "a", (1, 0, 0));
        graph.add_node(b, "b", (2, 0, 0));

        graph.add_dependency(
            a,
            Dependency {
                service_id: b,
                service_name: "b".to_string(),
                version_constraint: VersionConstraint::Exact {
                    major: 1,
                    minor: 0,
                    patch: 0,
                },
                optional: false,
            },
        );

        let errors = graph.validate();
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0].1.kind,
            DependencyErrorKind::VersionMismatch
        ));
    }

    #[test]
    fn remove_node() {
        let mut graph = DependencyGraph::new();
        let a = make_id();
        let b = make_id();

        graph.add_node(a, "a", (1, 0, 0));
        graph.add_node(b, "b", (1, 0, 0));
        graph.add_dependency(
            a,
            Dependency {
                service_id: b,
                service_name: "b".to_string(),
                version_constraint: VersionConstraint::Any,
                optional: false,
            },
        );

        graph.remove_node(b);
        assert_eq!(graph.len(), 1);
        assert!(graph.find_by_name("b").is_none());
        assert!(graph.dependencies_of(a).is_empty());
    }

    #[test]
    fn find_by_name() {
        let mut graph = DependencyGraph::new();
        let id = make_id();
        graph.add_node(id, "my-service", (1, 0, 0));
        assert_eq!(graph.find_by_name("my-service"), Some(id));
        assert!(graph.find_by_name("nonexistent").is_none());
    }
}
