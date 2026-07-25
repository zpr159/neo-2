use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::core::entity::EntityType;
use crate::error::{KnowledgeError, KnowledgeResult};

/// A concept extracted from text or memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedConcept {
    /// The concept label.
    pub label: String,
    /// Concept category/type.
    pub concept_type: EntityType,
    /// Confidence in the extraction (0.0 - 1.0).
    pub confidence: f32,
    /// Context in which the concept was found.
    pub context: String,
    /// Source text or memory id.
    pub source: String,
    /// Extraction timestamp.
    pub extracted_at: DateTime<Utc>,
    /// Additional properties.
    pub properties: HashMap<String, serde_json::Value>,
}

/// Extracts concepts from text and memory entries.
pub struct ConceptExtractor;

impl ConceptExtractor {
    /// Create a new concept extractor.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Extract concepts from text content.
    #[must_use]
    pub fn extract_from_text(&self, text: &str, source: &str) -> Vec<ExtractedConcept> {
        let mut concepts = Vec::new();
        let words: Vec<&str> = text.split_whitespace().collect();

        // Extract capitalized terms as potential concepts
        let mut current_chunk = Vec::new();
        for word in &words {
            let trimmed = word.trim_matches(|c: char| c.is_ascii_punctuation());
            if !trimmed.is_empty()
                && trimmed
                    .chars()
                    .next()
                    .map_or(false, |c| c.is_uppercase())
            {
                current_chunk.push(trimmed.to_string());
            } else if !current_chunk.is_empty() {
                let label = current_chunk.join(" ");
                let confidence = self.concept_confidence(&label, text);
                concepts.push(ExtractedConcept {
                    label: label.clone(),
                    concept_type: EntityType::Concept,
                    confidence,
                    context: self.extract_context(text, &label),
                    source: source.to_string(),
                    extracted_at: Utc::now(),
                    properties: HashMap::new(),
                });
                current_chunk.clear();
            }
        }
        if !current_chunk.is_empty() {
            let label = current_chunk.join(" ");
            let confidence = self.concept_confidence(&label, text);
            concepts.push(ExtractedConcept {
                label,
                concept_type: EntityType::Concept,
                confidence,
                context: self.extract_context(text, &current_chunk.join(" ")),
                source: source.to_string(),
                extracted_at: Utc::now(),
                properties: HashMap::new(),
            });
        }

        concepts
    }

    /// Extract concepts from a list of keywords.
    #[must_use]
    pub fn extract_from_keywords(
        &self,
        keywords: &[String],
        source: &str,
    ) -> Vec<ExtractedConcept> {
        keywords
            .iter()
            .map(|kw| ExtractedConcept {
                label: kw.clone(),
                concept_type: EntityType::Concept,
                confidence: 0.8,
                context: String::new(),
                source: source.to_string(),
                extracted_at: Utc::now(),
                properties: HashMap::new(),
            })
            .collect()
    }

    fn concept_confidence(&self, label: &str, text: &str) -> f32 {
        let word_count = label.split_whitespace().count();
        let frequency = text.matches(label).count() as f32;
        let length_score = if word_count >= 2 { 0.9 } else { 0.6 };
        let freq_score = (frequency / (frequency + 5.0)).min(1.0);
        (length_score * 0.6 + freq_score * 0.4).clamp(0.0, 1.0)
    }

    fn extract_context(&self, text: &str, label: &str) -> String {
        if let Some(pos) = text.find(label) {
            let start = pos.saturating_sub(50);
            let end = (pos + label.len() + 50).min(text.len());
            text[start..end].to_string()
        } else {
            text.chars().take(100).collect()
        }
    }
}

impl Default for ConceptExtractor {
    fn default() -> Self {
        Self::new()
    }
}
