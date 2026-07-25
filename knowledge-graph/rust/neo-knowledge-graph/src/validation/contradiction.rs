use serde::{Deserialize, Serialize};

use crate::core::entity::Entity;
use crate::core::relation::Relation;

/// A detected contradiction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedContradiction {
    /// First entity/relation id.
    pub first_id: String,
    /// Second entity/relation id.
    pub second_id: String,
    /// Type of contradiction.
    pub contradiction_type: ContradictionType,
    /// Description.
    pub description: String,
    /// Severity (0.0 - 1.0).
    pub severity: f32,
}

/// Type of contradiction.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContradictionType {
    TypeConflict,
    PropertyConflict,
    RelationConflict,
    TemporalConflict,
}

/// Detects contradictions between knowledge elements.
pub struct ContradictionDetector;

impl ContradictionDetector {
    /// Create a new detector.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Detect contradictions between entities (same label, different types).
    #[must_use]
    pub fn detect_entity_contradictions(&self, entities: &[Entity]) -> Vec<DetectedContradiction> {
        let mut contradictions = Vec::new();
        let mut by_label: std::collections::HashMap<String, Vec<&Entity>> = std::collections::HashMap::new();

        for entity in entities {
            if !entity.active {
                continue;
            }
            by_label
                .entry(entity.label.to_lowercase())
                .or_default()
                .push(entity);
        }

        for (label, group) in &by_label {
            if group.len() < 2 {
                continue;
            }

            let types: std::collections::HashSet<_> =
                group.iter().map(|e| &e.entity_type).collect();
            if types.len() > 1 {
                contradictions.push(DetectedContradiction {
                    first_id: group[0].id.to_string(),
                    second_id: group[1].id.to_string(),
                    contradiction_type: ContradictionType::TypeConflict,
                    description: format!(
                        "Entity '{}' has conflicting types: {:?}",
                        label,
                        types.iter().map(|t| t.to_string()).collect::<Vec<_>>()
                    ),
                    severity: 0.8,
                });
            }

            // Check property conflicts
            for i in 0..group.len() {
                for j in (i + 1)..group.len() {
                    for (key, val_a) in &group[i].properties {
                        if let Some(val_b) = group[j].properties.get(key) {
                            if val_a != val_b {
                                contradictions.push(DetectedContradiction {
                                    first_id: group[i].id.to_string(),
                                    second_id: group[j].id.to_string(),
                                    contradiction_type: ContradictionType::PropertyConflict,
                                    description: format!(
                                        "Entities '{}' and '{}' have conflicting values for property '{}'",
                                        group[i].label, group[j].label, key
                                    ),
                                    severity: 0.6,
                                });
                            }
                        }
                    }
                }
            }
        }

        contradictions
    }

    /// Detect contradiction relations.
    #[must_use]
    pub fn detect_relation_contradictions(&self, relations: &[Relation]) -> Vec<DetectedContradiction> {
        relations
            .iter()
            .filter(|r| {
                r.active
                    && matches!(r.relation_type, crate::core::relation::RelationType::Contradicts)
            })
            .map(|r| DetectedContradiction {
                first_id: r.source.to_string(),
                second_id: r.target.to_string(),
                contradiction_type: ContradictionType::RelationConflict,
                description: format!(
                    "Explicit contradiction: {} --[contradicts]--> {}",
                    r.source, r.target
                ),
                severity: r.weight,
            })
            .collect()
    }

    /// Detect all contradictions.
    #[must_use]
    pub fn detect_all(
        &self,
        entities: &[Entity],
        relations: &[Relation],
    ) -> Vec<DetectedContradiction> {
        let mut all = self.detect_entity_contradictions(entities);
        all.extend(self.detect_relation_contradictions(relations));
        all.sort_by(|a, b| b.severity.partial_cmp(&a.severity).unwrap_or(std::cmp::Ordering::Equal));
        all
    }
}

impl Default for ContradictionDetector {
    fn default() -> Self {
        Self::new()
    }
}
