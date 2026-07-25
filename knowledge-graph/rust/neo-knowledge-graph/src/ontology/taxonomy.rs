use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A node in the taxonomy tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxonomyNode {
    /// Name of this type.
    pub name: String,
    /// Parent type name.
    pub parent: Option<String>,
    /// Child type names.
    pub children: Vec<String>,
    /// Depth in the tree.
    pub depth: usize,
    /// Description.
    pub description: String,
}

/// A path from root to a specific type in the taxonomy.
pub type TaxonomyPath = Vec<String>;

/// Hierarchical taxonomy tree for type inheritance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxonomyTree {
    nodes: HashMap<String, TaxonomyNode>,
}

impl TaxonomyTree {
    /// Create a new empty taxonomy tree.
    #[must_use]
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    /// Add a type to the taxonomy.
    pub fn add_type(&mut self, name: String, parent: Option<String>, description: String) {
        let depth = parent.as_ref().and_then(|p| self.nodes.get(p)).map_or(0, |n| n.depth + 1);

        let node = TaxonomyNode {
            name: name.clone(),
            parent: parent.clone(),
            children: Vec::new(),
            depth,
            description,
        };

        if let Some(ref p) = parent {
            if let Some(parent_node) = self.nodes.get_mut(p) {
                parent_node.children.push(name.clone());
            }
        }

        self.nodes.insert(name, node);
    }

    /// Remove a type from the taxonomy.
    pub fn remove_type(&mut self, name: &str) -> Option<TaxonomyNode> {
        if let Some(node) = self.nodes.remove(name) {
            if let Some(ref parent_name) = node.parent {
                if let Some(parent) = self.nodes.get_mut(parent_name) {
                    parent.children.retain(|c| c != name);
                }
            }
            for child in &node.children {
                if let Some(child_node) = self.nodes.get_mut(child) {
                    child_node.parent = node.parent.clone();
                }
            }
            Some(node)
        } else {
            None
        }
    }

    /// Get a node by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&TaxonomyNode> {
        self.nodes.get(name)
    }

    /// Get the path from root to a type.
    #[must_use]
    pub fn path_to_root(&self, name: &str) -> TaxonomyPath {
        let mut path = Vec::new();
        let mut current = name;
        while let Some(node) = self.nodes.get(current) {
            path.push(node.name.clone());
            if let Some(ref parent) = node.parent {
                current = parent;
            } else {
                break;
            }
        }
        path.reverse();
        path
    }

    /// Check if one type is an ancestor of another.
    #[must_use]
    pub fn is_ancestor(&self, potential_ancestor: &str, descendant: &str) -> bool {
        let path = self.path_to_root(descendant);
        path.iter().any(|n| n == potential_ancestor)
    }

    /// Get all descendants of a type.
    #[must_use]
    pub fn descendants(&self, name: &str) -> Vec<String> {
        let mut result = Vec::new();
        if let Some(node) = self.nodes.get(name) {
            for child in &node.children {
                result.push(child.clone());
                result.extend(self.descendants(child));
            }
        }
        result
    }

    /// Get all ancestors of a type (excluding itself).
    #[must_use]
    pub fn ancestors(&self, name: &str) -> Vec<String> {
        let mut path = self.path_to_root(name);
        if let Some(pos) = path.iter().position(|n| n == name) {
            path.drain(pos..);
        }
        path
    }

    /// Get all root types (no parent).
    #[must_use]
    pub fn roots(&self) -> Vec<&TaxonomyNode> {
        self.nodes.values().filter(|n| n.parent.is_none()).collect()
    }

    /// Count all types in the taxonomy.
    #[must_use]
    pub fn count(&self) -> usize {
        self.nodes.len()
    }

    /// Get the depth of a type.
    #[must_use]
    pub fn depth_of(&self, name: &str) -> Option<usize> {
        self.nodes.get(name).map(|n| n.depth)
    }

    /// Get the maximum depth of the taxonomy.
    #[must_use]
    pub fn max_depth(&self) -> usize {
        self.nodes.values().map(|n| n.depth).max().unwrap_or(0)
    }
}

impl Default for TaxonomyTree {
    fn default() -> Self {
        Self::new()
    }
}
