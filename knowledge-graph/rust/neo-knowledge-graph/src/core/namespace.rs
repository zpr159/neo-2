use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Configuration for a namespace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamespaceConfig {
    /// Whether the namespace is read-only.
    pub read_only: bool,
    /// Maximum entities allowed in this namespace.
    pub max_entities: Option<usize>,
    /// Maximum relations allowed in this namespace.
    pub max_relations: Option<usize>,
    /// Whether encryption is enabled for this namespace.
    pub encrypted: bool,
    /// Allowed permissions for this namespace.
    pub allowed_permissions: Vec<String>,
}

impl Default for NamespaceConfig {
    fn default() -> Self {
        Self {
            read_only: false,
            max_entities: None,
            max_relations: None,
            encrypted: false,
            allowed_permissions: vec![
                "read".to_string(),
                "write".to_string(),
                "admin".to_string(),
            ],
        }
    }
}

/// A namespace in the knowledge graph for isolation and organization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeNamespace {
    /// Namespace name/identifier.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Configuration.
    pub config: NamespaceConfig,
    /// Number of entities in this namespace.
    pub entity_count: usize,
    /// Number of relations in this namespace.
    pub relation_count: usize,
}

impl KnowledgeNamespace {
    /// Create a new namespace.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            config: NamespaceConfig::default(),
            entity_count: 0,
            relation_count: 0,
        }
    }

    /// Create with a description.
    #[must_use]
    pub fn with_description(name: impl Into<String>, desc: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: desc.into(),
            config: NamespaceConfig::default(),
            entity_count: 0,
            relation_count: 0,
        }
    }
}

/// Registry for managing namespaces.
#[derive(Debug)]
pub struct NamespaceRegistry {
    namespaces: HashMap<String, KnowledgeNamespace>,
}

impl NamespaceRegistry {
    /// Create a new registry with a default namespace.
    #[must_use]
    pub fn new() -> Self {
        let mut namespaces = HashMap::new();
        namespaces.insert(
            "default".to_string(),
            KnowledgeNamespace::new("default"),
        );
        Self { namespaces }
    }

    /// Register a new namespace.
    pub fn register(&mut self, namespace: KnowledgeNamespace) -> Result<(), String> {
        if self.namespaces.contains_key(&namespace.name) {
            return Err(format!("namespace '{}' already exists", namespace.name));
        }
        self.namespaces.insert(namespace.name.clone(), namespace);
        Ok(())
    }

    /// Get a namespace by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&KnowledgeNamespace> {
        self.namespaces.get(name)
    }

    /// Get a mutable reference to a namespace.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut KnowledgeNamespace> {
        self.namespaces.get_mut(name)
    }

    /// Check if a namespace exists.
    #[must_use]
    pub fn exists(&self, name: &str) -> bool {
        self.namespaces.contains_key(name)
    }

    /// List all namespace names.
    #[must_use]
    pub fn list(&self) -> Vec<&str> {
        self.namespaces.keys().map(String::as_str).collect()
    }

    /// Remove a namespace.
    pub fn remove(&mut self, name: &str) -> Option<KnowledgeNamespace> {
        if name == "default" {
            return None;
        }
        self.namespaces.remove(name)
    }
}

impl Default for NamespaceRegistry {
    fn default() -> Self {
        Self::new()
    }
}
