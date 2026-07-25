use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::core::entity::EntityId;
use crate::core::relation::RelationType;
use crate::error::KnowledgeResult;

/// A relation extracted from text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedRelation {
    /// Source entity label.
    pub source_label: String,
    /// Target entity label.
    pub target_label: String,
    /// Relation type.
    pub relation_type: RelationType,
    /// Confidence (0.0 - 1.0).
    pub confidence: f32,
    /// Context text.
    pub context: String,
    /// Source text.
    pub source: String,
    /// Extraction timestamp.
    pub extracted_at: DateTime<Utc>,
    /// Additional properties.
    pub properties: HashMap<String, serde_json::Value>,
}

/// Extracts relations from text using pattern-based extraction.
pub struct RelationExtractor;

impl RelationExtractor {
    /// Create a new relation extractor.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Extract relations from text between known entity labels.
    #[must_use]
    pub fn extract(
        &self,
        text: &str,
        entity_labels: &[String],
        source: &str,
    ) -> Vec<ExtractedRelation> {
        let mut relations = Vec::new();
        let lower_text = text.to_lowercase();

        // Pattern: "X is a Y" -> IsA
        let is_a_patterns = [" is a ", " is an ", " are a ", " are an "];
        for pattern in &is_a_patterns {
            for label_a in entity_labels {
                if let Some(pos_a) = lower_text.find(&format!("{}{}", label_a.to_lowercase(), pattern)) {
                    let after = &text[pos_a + label_a.len() + pattern.len()..];
                    let target: String = after
                        .split(|c: char| c == '.' || c == ',' || c == ';' || c.is_whitespace())
                        .take_while(|w| !w.is_empty())
                        .collect::<Vec<_>>()
                        .join(" ");
                    if !target.is_empty() && entity_labels.iter().any(|l| l.eq_ignore_ascii_case(&target)) {
                        relations.push(ExtractedRelation {
                            source_label: label_a.clone(),
                            target_label: target,
                            relation_type: RelationType::IsA,
                            confidence: 0.8,
                            context: self.get_context(text, pos_a),
                            source: source.to_string(),
                            extracted_at: Utc::now(),
                            properties: HashMap::new(),
                        });
                    }
                }
            }
        }

        // Pattern: "X has Y" / "X contains Y" -> HasA
        let has_patterns = [" has ", " contains ", " includes "];
        for pattern in &has_patterns {
            for label_a in entity_labels {
                if let Some(pos_a) = lower_text.find(&format!("{}{}", label_a.to_lowercase(), pattern)) {
                    let after = &text[pos_a + label_a.len() + pattern.len()..];
                    let target: String = after
                        .split(|c: char| c == '.' || c == ',' || c == ';')
                        .next()
                        .unwrap_or("")
                        .split_whitespace()
                        .take(3)
                        .collect::<Vec<_>>()
                        .join(" ");
                    if !target.is_empty() {
                        relations.push(ExtractedRelation {
                            source_label: label_a.clone(),
                            target_label: target,
                            relation_type: RelationType::HasA,
                            confidence: 0.7,
                            context: self.get_context(text, pos_a),
                            source: source.to_string(),
                            extracted_at: Utc::now(),
                            properties: HashMap::new(),
                        });
                    }
                }
            }
        }

        // Pattern: "X depends on Y" -> DependsOn
        let depends_patterns = [" depends on ", " relies on ", " requires "];
        for pattern in &depends_patterns {
            for label_a in entity_labels {
                if let Some(pos_a) = lower_text.find(&format!("{}{}", label_a.to_lowercase(), pattern)) {
                    let after = &text[pos_a + label_a.len() + pattern.len()..];
                    let target: String = after
                        .split(|c: char| c == '.' || c == ',' || c == ';')
                        .next()
                        .unwrap_or("")
                        .split_whitespace()
                        .take(3)
                        .collect::<Vec<_>>()
                        .join(" ");
                    if !target.is_empty() {
                        relations.push(ExtractedRelation {
                            source_label: label_a.clone(),
                            target_label: target,
                            relation_type: RelationType::DependsOn,
                            confidence: 0.75,
                            context: self.get_context(text, pos_a),
                            source: source.to_string(),
                            extracted_at: Utc::now(),
                            properties: HashMap::new(),
                        });
                    }
                }
            }
        }

        // Pattern: "X caused Y" / "X leads to Y" -> Causes
        let causes_patterns = [" caused ", " leads to ", " results in "];
        for pattern in &causes_patterns {
            for label_a in entity_labels {
                if let Some(pos_a) = lower_text.find(&format!("{}{}", label_a.to_lowercase(), pattern)) {
                    let after = &text[pos_a + label_a.len() + pattern.len()..];
                    let target: String = after
                        .split(|c: char| c == '.' || c == ',' || c == ';')
                        .next()
                        .unwrap_or("")
                        .split_whitespace()
                        .take(3)
                        .collect::<Vec<_>>()
                        .join(" ");
                    if !target.is_empty() {
                        relations.push(ExtractedRelation {
                            source_label: label_a.clone(),
                            target_label: target,
                            relation_type: RelationType::Causes,
                            confidence: 0.7,
                            context: self.get_context(text, pos_a),
                            source: source.to_string(),
                            extracted_at: Utc::now(),
                            properties: HashMap::new(),
                        });
                    }
                }
            }
        }

        relations
    }

    fn get_context(&self, text: &str, pos: usize) -> String {
        let start = pos.saturating_sub(50);
        let end = (pos + 100).min(text.len());
        text[start..end].to_string()
    }
}

impl Default for RelationExtractor {
    fn default() -> Self {
        Self::new()
    }
}
