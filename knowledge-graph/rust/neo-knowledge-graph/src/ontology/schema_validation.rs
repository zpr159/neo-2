use crate::core::entity::Entity;
use crate::core::relation::Relation;
use crate::error::{KnowledgeError, KnowledgeResult};
use crate::ontology::types::Ontology;

/// A single validation violation.
#[derive(Debug, Clone)]
pub struct ValidationViolation {
    /// The kind of violation.
    pub kind: ViolationKind,
    /// Human-readable description.
    pub message: String,
    /// Entity or relation id affected.
    pub target_id: Option<String>,
}

/// Type of violation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViolationKind {
    UnknownEntityType,
    UnknownRelationType,
    MissingRequiredProperty,
    InvalidPropertyType,
    InvalidSourceEntityType,
    InvalidTargetEntityType,
    CyclicInheritance,
}

/// Result of validating entities/relations against the ontology.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether validation passed.
    pub valid: bool,
    /// List of violations found.
    pub violations: Vec<ValidationViolation>,
}

impl ValidationResult {
    /// Create a passing result.
    #[must_use]
    pub fn pass() -> Self {
        Self {
            valid: true,
            violations: Vec::new(),
        }
    }

    /// Create a failing result.
    #[must_use]
    pub fn fail(violations: Vec<ValidationViolation>) -> Self {
        Self {
            valid: violations.is_empty(),
            violations,
        }
    }

    /// Add a violation.
    pub fn add_violation(&mut self, violation: ValidationViolation) {
        self.violations.push(violation);
        self.valid = false;
    }
}

/// Validates entities and relations against the ontology.
pub struct OntologyValidator<'a> {
    ontology: &'a Ontology,
}

impl<'a> OntologyValidator<'a> {
    /// Create a new validator with the given ontology.
    #[must_use]
    pub fn new(ontology: &'a Ontology) -> Self {
        Self { ontology }
    }

    /// Validate an entity against the ontology.
    #[must_use]
    pub fn validate_entity(&self, entity: &Entity) -> ValidationResult {
        let mut result = ValidationResult::pass();

        let type_name = entity.entity_type.as_str();
        if !self.ontology.has_entity_type(type_name) {
            result.add_violation(ValidationViolation {
                kind: ViolationKind::UnknownEntityType,
                message: format!("Unknown entity type: {}", type_name),
                target_id: Some(entity.id.to_string()),
            });
            return result;
        }

        if let Some(def) = self.ontology.get_entity_type(type_name) {
            for required_prop in &def.required_properties {
                if !entity.properties.contains_key(required_prop) {
                    result.add_violation(ValidationViolation {
                        kind: ViolationKind::MissingRequiredProperty,
                        message: format!(
                            "Entity '{}' missing required property '{}'",
                            entity.label, required_prop
                        ),
                        target_id: Some(entity.id.to_string()),
                    });
                }
            }
        }

        result
    }

    /// Validate a relation against the ontology.
    #[must_use]
    pub fn validate_relation(
        &self,
        relation: &Relation,
        source: &Entity,
        target: &Entity,
    ) -> ValidationResult {
        let mut result = ValidationResult::pass();

        let type_name = relation.relation_type.as_str();
        if !self.ontology.has_relation_type(type_name) {
            result.add_violation(ValidationViolation {
                kind: ViolationKind::UnknownRelationType,
                message: format!("Unknown relation type: {}", type_name),
                target_id: Some(relation.id.to_string()),
            });
            return result;
        }

        if let Some(def) = self.ontology.get_relation_type(type_name) {
            if let Some(ref valid_sources) = def.valid_source_types {
                let source_type = &source.entity_type;
                if !valid_sources.contains(source_type) {
                    result.add_violation(ValidationViolation {
                        kind: ViolationKind::InvalidSourceEntityType,
                        message: format!(
                            "Source entity type '{}' not valid for relation type '{}'",
                            source_type, type_name
                        ),
                        target_id: Some(relation.id.to_string()),
                    });
                }
            }

            if let Some(ref valid_targets) = def.valid_target_types {
                let target_type = &target.entity_type;
                if !valid_targets.contains(target_type) {
                    result.add_violation(ValidationViolation {
                        kind: ViolationKind::InvalidTargetEntityType,
                        message: format!(
                            "Target entity type '{}' not valid for relation type '{}'",
                            target_type, type_name
                        ),
                        target_id: Some(relation.id.to_string()),
                    });
                }
            }

            for required_prop in &def.required_properties {
                if !relation.properties.contains_key(required_prop) {
                    result.add_violation(ValidationViolation {
                        kind: ViolationKind::MissingRequiredProperty,
                        message: format!(
                            "Relation '{}' missing required property '{}'",
                            relation.label, required_prop
                        ),
                        target_id: Some(relation.id.to_string()),
                    });
                }
            }
        }

        result
    }
}
