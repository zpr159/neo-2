use neo_core::error::{NeoError, NeoResult};
use serde::{Deserialize, Serialize};

use crate::edge::Edge;
use crate::edge::EdgeType;
use crate::node::Node;
use crate::node::NodeType;

/// Definition of a single property within a schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaProperty {
    /// Name of the property.
    pub name: String,
    /// Expected JSON type name (e.g. "string", "number", "boolean").
    pub property_type: String,
    /// Whether this property is required.
    pub required: bool,
    /// Optional default value.
    pub default: Option<serde_json::Value>,
}

/// Schema definition for a specific node type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaNodeDefinition {
    /// The node type this definition applies to.
    pub node_type: NodeType,
    /// Expected properties for this node type.
    pub properties: Vec<SchemaProperty>,
}

/// Schema definition for a specific edge type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaEdgeDefinition {
    /// The edge type this definition applies to.
    pub edge_type: EdgeType,
    /// Expected properties for this edge type.
    pub properties: Vec<SchemaProperty>,
    /// If set, only these source node types are allowed.
    pub allowed_source_types: Option<Vec<NodeType>>,
    /// If set, only these target node types are allowed.
    pub allowed_target_types: Option<Vec<NodeType>>,
}

/// Top-level schema for a knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphSchema {
    /// Schema name.
    pub name: String,
    /// Schema version.
    pub version: u32,
    /// Node type definitions.
    pub node_definitions: Vec<SchemaNodeDefinition>,
    /// Edge type definitions.
    pub edge_definitions: Vec<SchemaEdgeDefinition>,
}

impl GraphSchema {
    /// Create a new schema with the given name.
    #[must_use]
    pub fn new(name: String) -> Self {
        Self {
            name,
            version: 1,
            node_definitions: Vec::new(),
            edge_definitions: Vec::new(),
        }
    }

    /// Add a node type definition.
    pub fn add_node_definition(&mut self, def: SchemaNodeDefinition) {
        self.node_definitions.push(def);
        self.version += 1;
    }

    /// Add an edge type definition.
    pub fn add_edge_definition(&mut self, def: SchemaEdgeDefinition) {
        self.edge_definitions.push(def);
        self.version += 1;
    }

    /// Validate a node against the schema.
    pub fn validate_node(&self, node: &Node) -> NeoResult<()> {
        let def = self
            .node_definitions
            .iter()
            .find(|d| d.node_type == node.node_type)
            .ok_or_else(|| {
                NeoError::InvalidInput(format!(
                    "No schema definition for node type {:?}",
                    node.node_type
                ))
            })?;

        for prop in &def.properties {
            if prop.required && !node.properties.contains_key(&prop.name) {
                return Err(NeoError::InvalidInput(format!(
                    "Missing required property '{}' on node '{}'",
                    prop.name, node.label
                )));
            }
        }
        Ok(())
    }

    /// Validate an edge against the schema, checking type constraints.
    pub fn validate_edge(
        &self,
        edge: &Edge,
        source: &Node,
        target: &Node,
    ) -> NeoResult<()> {
        let def = self
            .edge_definitions
            .iter()
            .find(|d| d.edge_type == edge.edge_type)
            .ok_or_else(|| {
                NeoError::InvalidInput(format!(
                    "No schema definition for edge type {:?}",
                    edge.edge_type
                ))
            })?;

        if let Some(ref allowed) = def.allowed_source_types {
            if !allowed.contains(&source.node_type) {
                return Err(NeoError::InvalidInput(format!(
                    "Source node type {:?} not allowed for edge type {:?}",
                    source.node_type, edge.edge_type
                )));
            }
        }

        if let Some(ref allowed) = def.allowed_target_types {
            if !allowed.contains(&target.node_type) {
                return Err(NeoError::InvalidInput(format!(
                    "Target node type {:?} not allowed for edge type {:?}",
                    target.node_type, edge.edge_type
                )));
            }
        }

        for prop in &def.properties {
            if prop.required && !edge.properties.contains_key(&prop.name) {
                return Err(NeoError::InvalidInput(format!(
                    "Missing required property '{}' on edge",
                    prop.name
                )));
            }
        }
        Ok(())
    }
}
