use serde::{Deserialize, Serialize};

use crate::core::entity::Entity;
use crate::core::relation::Relation;

/// A single conflict between two pieces of knowledge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeConflict {
    /// First entity/relation involved.
    pub first_id: String,
    /// Second entity/relation involved.
    pub second_id: String,
    /// Description of the conflict.
    pub description: String,
    /// Severity (0.0 - 1.0).
    pub severity: f32,
}

/// A report from confidence estimation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceReport {
    /// Overall confidence score.
    pub overall_confidence: f32,
    /// Per-entity confidence scores.
    pub entity_confidences: std::collections::HashMap<String, f32>,
    /// Per-relation confidence scores.
    pub relation_confidences: std::collections::HashMap<String, f32>,
    /// Number of sources backing the knowledge.
    pub source_count: usize,
}

/// Result of conflict detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictDetection {
    /// Whether conflicts were found.
    pub has_conflicts: bool,
    /// List of conflicts.
    pub conflicts: Vec<KnowledgeConflict>,
    /// Suggested resolution strategies.
    pub suggestions: Vec<String>,
}

/// Estimates confidence and detects conflicts in knowledge.
pub struct ConfidenceEstimator;

impl ConfidenceEstimator {
    /// Create a new estimator.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Estimate confidence for a set of entities based on source count and agreement.
    #[must_use]
    pub fn estimate_entity_confidence(&self, entity: &Entity, all_sources: &[String]) -> f32 {
        let source_score = if all_sources.is_empty() {
            0.5
        } else {
            (all_sources.len() as f32 / (all_sources.len() as f32 + 2.0)).min(1.0)
        };
        let base_score = entity.confidence;
        (base_score * 0.6 + source_score * 0.4).clamp(0.0, 1.0)
    }

    /// Estimate confidence for a relation.
    #[must_use]
    pub fn estimate_relation_confidence(&self, relation: &Relation) -> f32 {
        let weight_score = relation.weight;
        let source_score = if relation.sources.is_empty() {
            0.5
        } else {
            (relation.sources.len() as f32 / (relation.sources.len() as f32 + 2.0)).min(1.0)
        };
        (relation.confidence * 0.5 + weight_score * 0.25 + source_score * 0.25).clamp(0.0, 1.0)
    }

    /// Generate a confidence report for the full graph.
    #[must_use]
    pub fn generate_report(
        &self,
        entities: &[Entity],
        relations: &[Relation],
    ) -> ConfidenceReport {
        let mut entity_confidences = std::collections::HashMap::new();
        let mut relation_confidences = std::collections::HashMap::new();

        for entity in entities {
            let conf = self.estimate_entity_confidence(entity, &entity.sources);
            entity_confidences.insert(entity.id.to_string(), conf);
        }

        for relation in relations {
            let conf = self.estimate_relation_confidence(relation);
            relation_confidences.insert(relation.id.to_string(), conf);
        }

        let overall = if entity_confidences.is_empty() && relation_confidences.is_empty() {
            0.0
        } else {
            let total: f32 = entity_confidences.values().chain(relation_confidences.values()).sum();
            let count = entity_confidences.len() + relation_confidences.len();
            if count > 0 { total / count as f32 } else { 0.0 }
        };

        let all_source_count: usize = entities
            .iter()
            .map(|e| e.sources.len())
            .sum();

        ConfidenceReport {
            overall_confidence: overall,
            entity_confidences,
            relation_confidences,
            source_count: all_source_count,
        }
    }

    /// Detect conflicts between entities with the same label but different types or properties.
    #[must_use]
    pub fn detect_conflicts(&self, entities: &[Entity], relations: &[Relation]) -> ConflictDetection {
        let mut conflicts = Vec::new();
        let mut suggestions = Vec::new();

        // Entity type conflicts
        let mut by_label: std::collections::HashMap<String, Vec<&Entity>> = std::collections::HashMap::new();
        for entity in entities {
            by_label
                .entry(entity.label.to_lowercase())
                .or_default()
                .push(entity);
        }

        for (label, group) in &by_label {
            if group.len() > 1 {
                let types: std::collections::HashSet<_> =
                    group.iter().map(|e| &e.entity_type).collect();
                if types.len() > 1 {
                    conflicts.push(KnowledgeConflict {
                        first_id: group[0].id.to_string(),
                        second_id: group[1].id.to_string(),
                        description: format!(
                            "Entity '{}' has conflicting types: {:?}",
                            label,
                            types.iter().map(|t| t.to_string()).collect::<Vec<_>>()
                        ),
                        severity: 0.7,
                    });
                    suggestions.push(format!(
                        "Consider merging or disambiguating entities labeled '{}'",
                        label
                    ));
                }
            }
        }

        // Contradiction relations
        for relation in relations {
            if let crate::core::relation::RelationType::Contradicts = relation.relation_type {
                conflicts.push(KnowledgeConflict {
                    first_id: relation.source.to_string(),
                    second_id: relation.target.to_string(),
                    description: format!(
                        "Explicit contradiction between {} and {}",
                        relation.source, relation.target
                    ),
                    severity: relation.weight,
                });
                suggestions.push(
                    "Resolve contradiction by updating one of the contradicting entities".to_string(),
                );
            }
        }

        ConflictDetection {
            has_conflicts: !conflicts.is_empty(),
            conflicts,
            suggestions,
        }
    }
}

impl Default for ConfidenceEstimator {
    fn default() -> Self {
        Self::new()
    }
}
