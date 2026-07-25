use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::core::entity::{Entity, EntityType};
use crate::error::KnowledgeResult;

/// An entity extracted from text or memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntity {
    /// Entity label.
    pub label: String,
    /// Entity type.
    pub entity_type: EntityType,
    /// Confidence (0.0 - 1.0).
    pub confidence: f32,
    /// Description.
    pub description: String,
    /// Properties.
    pub properties: HashMap<String, serde_json::Value>,
    /// Source text.
    pub source: String,
    /// Extraction timestamp.
    pub extracted_at: DateTime<Utc>,
}

/// Extracts entities from text content using pattern-based extraction.
pub struct EntityExtractor;

impl EntityExtractor {
    /// Create a new entity extractor.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Extract entities from text.
    #[must_use]
    pub fn extract(&self, text: &str, source: &str) -> Vec<ExtractedEntity> {
        let mut entities = Vec::new();
        let mut seen_labels: std::collections::HashSet<String> = std::collections::HashSet::new();

        // Person detection: "Mr.", "Mrs.", "Dr.", or capitalized names after "I am" / "called"
        for line in text.lines() {
            for pattern in &[
                "Dr.",
                "Mr.",
                "Mrs.",
                "Prof.",
                "Sir",
                "Ms.",
            ] {
                if let Some(start) = line.find(pattern) {
                    let after_pattern = &line[start + pattern.len()..];
                    let name: String = after_pattern
                        .split_whitespace()
                        .take(2)
                        .collect::<Vec<_>>()
                        .join(" ");
                    if !name.is_empty() {
                        let label = format!("{} {}", pattern.trim_end_matches('.'), name.trim());
                        if seen_labels.insert(label.clone()) {
                            entities.push(ExtractedEntity {
                                label,
                                entity_type: EntityType::Person,
                                confidence: 0.85,
                                description: String::new(),
                                properties: HashMap::new(),
                                source: source.to_string(),
                                extracted_at: Utc::now(),
                            });
                        }
                    }
                }
            }
        }

        // Capitalized word detection for proper nouns (names, places, concepts)
        let skip_words: std::collections::HashSet<&str> = [
            "A", "An", "The", "I", "She", "He", "It", "We", "They",
            "In", "At", "On", "By", "For", "To", "Of", "Is", "Are",
            "Was", "Were", "Be", "Been", "Being", "Have", "Has", "Had",
            "Do", "Does", "Did", "Will", "Would", "Could", "Should",
            "May", "Might", "Can", "Must", "Shall", "Not", "And", "But",
            "Or", "So", "Yet", "Both", "Either", "Neither", "Each", "Every",
            "This", "That", "These", "Those", "My", "Your", "His", "Her",
            "Its", "Our", "Their", "What", "Which", "Who", "Whom", "When",
            "Where", "How", "Why", "If", "Then", "Else", "Because", "Since",
            "While", "Although", "Though", "After", "Before", "Until",
            "Above", "Below", "Between", "Under", "Over", "With", "Without",
            "From", "Into", "About", "Like", "As", "Than", "Also", "Very",
            "Just", "Only", "Even", "Still", "Already", "Here", "There",
            "Now", "Then", "Soon", "Always", "Never", "Sometimes", "Often",
            "She", "depends", "engineer",
        ].iter().copied().collect();

        for word in text.split_whitespace() {
            let clean = word.trim_matches(|c: char| c.is_ascii_punctuation());
            if clean.len() < 2 {
                continue;
            }
            // Must start with uppercase and rest lowercase (proper noun pattern)
            let first_char = clean.chars().next().unwrap();
            if first_char.is_uppercase()
                && clean.chars().skip(1).all(|c| c.is_lowercase() || c == '\'')
                && !skip_words.contains(clean)
            {
                let label = clean.to_string();
                if seen_labels.insert(label.clone()) {
                    entities.push(ExtractedEntity {
                        label,
                        entity_type: EntityType::Concept,
                        confidence: 0.5,
                        description: String::new(),
                        properties: HashMap::new(),
                        source: source.to_string(),
                        extracted_at: Utc::now(),
                    });
                }
            }
        }

        // Quoted text as potential entity
        let mut chars = text.char_indices();
        while let Some((start, c)) = chars.next() {
            if c == '"' || c == '\'' {
                let quote_char = c;
                let content_start = start + 1;
                for (end, ec) in chars.by_ref() {
                    if ec == quote_char {
                        let content = &text[content_start..end];
                        if content.len() > 3 && content.len() < 100 {
                            let is_concept = content
                                .chars()
                                .next()
                                .map_or(false, |c| c.is_uppercase());
                            if is_concept && seen_labels.insert(content.to_string()) {
                                entities.push(ExtractedEntity {
                                    label: content.to_string(),
                                    entity_type: EntityType::Concept,
                                    confidence: 0.6,
                                    description: String::new(),
                                    properties: HashMap::new(),
                                    source: source.to_string(),
                                    extracted_at: Utc::now(),
                                });
                            }
                        }
                        break;
                    }
                }
            }
        }

        // URL detection
        for word in text.split_whitespace() {
            if word.starts_with("http://") || word.starts_with("https://") {
                if seen_labels.insert(word.to_string()) {
                    entities.push(ExtractedEntity {
                        label: word.to_string(),
                        entity_type: EntityType::Document,
                        confidence: 0.95,
                        description: "URL".to_string(),
                        properties: HashMap::new(),
                        source: source.to_string(),
                        extracted_at: Utc::now(),
                    });
                }
            }
        }

        entities
    }

    /// Convert extracted entities into proper Entity objects.
    #[must_use]
    pub fn to_entities(extracted: &[ExtractedEntity]) -> Vec<Entity> {
        extracted
            .iter()
            .map(|e| {
                Entity::builder(e.entity_type.clone(), e.label.clone())
                    .description(e.description.clone())
                    .confidence(e.confidence)
                    .source(e.source.clone())
                    .build()
            })
            .collect()
    }
}

impl Default for EntityExtractor {
    fn default() -> Self {
        Self::new()
    }
}
