use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::core::entity::EntityType;
use crate::core::relation::RelationType;

/// Definition of an entity type in the ontology.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityTypeDefinition {
    /// The entity type.
    pub entity_type: EntityType,
    /// Human-readable description.
    pub description: String,
    /// Parent type for inheritance (None for root types).
    pub parent_type: Option<EntityType>,
    /// Required property names.
    pub required_properties: Vec<String>,
    /// Allowed property names with their types.
    pub allowed_properties: HashMap<String, String>,
    /// Whether instances of this type can be created.
    pub instantiable: bool,
}

/// Definition of a relation type in the ontology.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationTypeDefinition {
    /// The relation type.
    pub relation_type: RelationType,
    /// Human-readable description.
    pub description: String,
    /// Parent type for inheritance.
    pub parent_type: Option<RelationType>,
    /// If set, only these source entity types are valid.
    pub valid_source_types: Option<Vec<EntityType>>,
    /// If set, only these target entity types are valid.
    pub valid_target_types: Option<Vec<EntityType>>,
    /// Required property names.
    pub required_properties: Vec<String>,
    /// Whether the relation is symmetric.
    pub symmetric: bool,
    /// Whether the relation is transitive.
    pub transitive: bool,
    /// Maximum weight.
    pub max_weight: f32,
}

/// Definition of a property in the ontology.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyDefinition {
    /// Property name.
    pub name: String,
    /// Expected type (string, integer, float, boolean, datetime, json).
    pub property_type: String,
    /// Whether the property is required.
    pub required: bool,
    /// Default value.
    pub default: Option<serde_json::Value>,
    /// Description.
    pub description: String,
}

/// The ontology defines the schema for the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ontology {
    /// Ontology name.
    pub name: String,
    /// Ontology version.
    pub version: String,
    /// Entity type definitions.
    pub entity_types: HashMap<String, EntityTypeDefinition>,
    /// Relation type definitions.
    pub relation_types: HashMap<String, RelationTypeDefinition>,
    /// Global property definitions.
    pub properties: HashMap<String, PropertyDefinition>,
    /// Description.
    pub description: String,
}

impl Ontology {
    /// Create a new ontology with default types.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        let mut ontology = Self {
            name: name.into(),
            version: "1.0.0".to_string(),
            entity_types: HashMap::new(),
            relation_types: HashMap::new(),
            properties: HashMap::new(),
            description: String::new(),
        };
        ontology.register_default_types();
        ontology
    }

    /// Register the default entity and relation types.
    fn register_default_types(&mut self) {
        // Entity types
        let default_entity_types = [
            (EntityType::Person, "A human person", None::<EntityType>),
            (EntityType::Place, "A physical or logical location", None),
            (EntityType::Organization, "An organization or group", None),
            (EntityType::Object, "A physical or digital object", None),
            (EntityType::Event, "An occurrence or happening", None),
            (EntityType::Concept, "An abstract concept or idea", None),
            (EntityType::Task, "A unit of work to be done", None),
            (EntityType::Goal, "An objective to be achieved", None),
            (EntityType::Skill, "A capability or expertise", None),
            (EntityType::Project, "A planned undertaking", None),
            (EntityType::Document, "A written or digital document", None),
            (EntityType::Idea, "A thought or suggestion", None),
            (EntityType::Rule, "A governing principle", None),
        ];

        for (et, desc, parent) in default_entity_types {
            let def = EntityTypeDefinition {
                entity_type: et.clone(),
                description: desc.to_string(),
                parent_type: parent.map(|p| p.clone()),
                required_properties: Vec::new(),
                allowed_properties: HashMap::new(),
                instantiable: true,
            };
            self.entity_types.insert(et.to_string(), def);
        }

        // Relation types
        let default_relation_types = [
            (RelationType::IsA, "Entity is a type of another", true, false),
            (RelationType::HasA, "Entity has a component", false, false),
            (RelationType::PartOf, "Entity is part of another", false, true),
            (RelationType::RelatedTo, "General relation", false, false),
            (RelationType::Causes, "First causes second", false, false),
            (RelationType::Enables, "First enables second", false, false),
            (RelationType::Prevents, "First prevents second", false, false),
            (RelationType::DependsOn, "First depends on second", false, false),
            (RelationType::LocatedAt, "Entity is located at", false, false),
            (RelationType::MemberOf, "Entity is member of", false, false),
            (RelationType::AuthorOf, "Entity authored target", false, false),
            (RelationType::CreatedBy, "Entity was created by", false, false),
            (RelationType::Uses, "Entity uses target", false, false),
            (RelationType::InheritsFrom, "Entity inherits from", false, false),
            (RelationType::Implements, "Entity implements target", false, false),
            (RelationType::Contradicts, "Entities contradict", true, false),
            (RelationType::Supports, "Entities support each other", true, false),
            (RelationType::TemporallyFollows, "First follows second in time", false, false),
            (RelationType::SpatiallyNear, "Entities are spatially near", true, false),
        ];

        for (rt, desc, symmetric, transitive) in default_relation_types {
            let def = RelationTypeDefinition {
                relation_type: rt.clone(),
                description: desc.to_string(),
                parent_type: None,
                valid_source_types: None,
                valid_target_types: None,
                required_properties: Vec::new(),
                symmetric,
                transitive,
                max_weight: 1.0,
            };
            self.relation_types.insert(rt.to_string(), def);
        }
    }

    /// Register a new entity type definition.
    pub fn register_entity_type(&mut self, def: EntityTypeDefinition) {
        self.entity_types.insert(def.entity_type.to_string().to_lowercase(), def);
    }

    /// Register a new relation type definition.
    pub fn register_relation_type(&mut self, def: RelationTypeDefinition) {
        self.relation_types.insert(def.relation_type.to_string(), def);
    }

    /// Get an entity type definition.
    #[must_use]
    pub fn get_entity_type(&self, name: &str) -> Option<&EntityTypeDefinition> {
        self.entity_types.get(name)
    }

    /// Get a relation type definition.
    #[must_use]
    pub fn get_relation_type(&self, name: &str) -> Option<&RelationTypeDefinition> {
        self.relation_types.get(name)
    }

    /// Check if an entity type exists.
    #[must_use]
    pub fn has_entity_type(&self, name: &str) -> bool {
        self.entity_types.contains_key(name)
    }

    /// Check if a relation type exists.
    #[must_use]
    pub fn has_relation_type(&self, name: &str) -> bool {
        self.relation_types.contains_key(name)
    }

    /// Get all entity type names.
    #[must_use]
    pub fn entity_type_names(&self) -> Vec<&str> {
        self.entity_types.keys().map(String::as_str).collect()
    }

    /// Get all relation type names.
    #[must_use]
    pub fn relation_type_names(&self) -> Vec<&str> {
        self.relation_types.keys().map(String::as_str).collect()
    }

    /// Get the parent type chain for an entity type.
    #[must_use]
    pub fn parent_chain(&self, entity_type: &str) -> Vec<String> {
        let mut chain = Vec::new();
        let mut current_opt: Option<String> = Some(entity_type.to_string());
        while let Some(ref current_name) = current_opt {
            if let Some(def) = self.entity_types.get(current_name) {
                if let Some(ref parent) = def.parent_type {
                    let parent_str = parent.to_string();
                    chain.push(parent_str.clone());
                    if self.entity_types.contains_key(&parent_str) {
                        current_opt = Some(parent_str);
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        chain
    }

    /// Check if a type is a subtype of another.
    #[must_use]
    pub fn is_subtype(&self, child: &str, parent: &str) -> bool {
        let chain = self.parent_chain(child);
        chain.iter().any(|p| p == parent)
    }
}

impl Default for Ontology {
    fn default() -> Self {
        Self::new("default")
    }
}
