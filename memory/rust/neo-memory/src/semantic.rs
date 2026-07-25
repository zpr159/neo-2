use std::collections::HashSet;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{MemoryError, MemoryResult};
use crate::types::{MemoryEntry, MemoryId};

/// A semantic fact with relationships, confidence, and version history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticFact {
    /// Unique identifier.
    pub id: Uuid,
    /// The subject entity.
    pub subject: String,
    /// The predicate (relationship) connecting subject to object.
    pub predicate: String,
    /// The object (target value or entity).
    pub object: serde_json::Value,
    /// Confidence in this fact, ranging from 0.0 to 1.0.
    pub confidence: f32,
    /// Source that provided this fact.
    pub source: Option<String>,
    /// When this fact was created.
    pub created_at: DateTime<Utc>,
    /// When this fact was last modified.
    pub last_modified: DateTime<Utc>,
    /// Version number for optimistic concurrency.
    pub version: u64,
    /// Tags for categorization.
    pub tags: HashSet<String>,
    /// Supporting evidence (list of memory ids that support this fact).
    pub supporting_evidence: Vec<MemoryId>,
    /// Counter-evidence (list of memory ids that contradict this fact).
    pub counter_evidence: Vec<MemoryId>,
}

impl SemanticFact {
    /// Create a new semantic fact.
    pub fn new(
        subject: impl Into<String>,
        predicate: impl Into<String>,
        object: serde_json::Value,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            subject: subject.into(),
            predicate: predicate.into(),
            object,
            confidence: 0.5,
            source: None,
            created_at: now,
            last_modified: now,
            version: 1,
            tags: HashSet::new(),
            supporting_evidence: Vec::new(),
            counter_evidence: Vec::new(),
        }
    }

    /// Set confidence.
    #[must_use]
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Set source.
    #[must_use]
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Add supporting evidence.
    pub fn add_support(&mut self, memory_id: MemoryId) {
        self.supporting_evidence.push(memory_id);
        self.recalculate_confidence();
    }

    /// Add counter-evidence.
    pub fn add_counter_evidence(&mut self, memory_id: MemoryId) {
        self.counter_evidence.push(memory_id);
        self.recalculate_confidence();
    }

    /// Recalculate confidence based on evidence balance.
    pub fn recalculate_confidence(&mut self) {
        let support = self.supporting_evidence.len() as f32;
        let counter = self.counter_evidence.len() as f32;
        let total = support + counter;
        if total > 0.0 {
            self.confidence = support / total;
        }
        self.last_modified = Utc::now();
        self.version += 1;
    }
}

/// A semantic concept with definition and relationships.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticConcept {
    /// Unique identifier.
    pub id: Uuid,
    /// Concept name.
    pub name: String,
    /// Definition of the concept.
    pub definition: String,
    /// Related concept ids.
    pub related_concepts: HashSet<Uuid>,
    /// Parent concept id (for hierarchy).
    pub parent_concept: Option<Uuid>,
    /// Child concept ids.
    pub child_concepts: HashSet<Uuid>,
    /// Confidence in this concept.
    pub confidence: f32,
    /// Source attribution.
    pub source: Option<String>,
    /// When this concept was created.
    pub created_at: DateTime<Utc>,
    /// Version number.
    pub version: u64,
    /// Usage count.
    pub usage_count: u64,
}

impl SemanticConcept {
    /// Create a new semantic concept.
    pub fn new(name: impl Into<String>, definition: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            definition: definition.into(),
            related_concepts: HashSet::new(),
            parent_concept: None,
            child_concepts: HashSet::new(),
            confidence: 0.5,
            source: None,
            created_at: Utc::now(),
            version: 1,
            usage_count: 0,
        }
    }

    /// Link to another concept bidirectionally.
    pub fn link_to(&mut self, other_id: Uuid) {
        self.related_concepts.insert(other_id);
    }

    /// Set parent concept.
    pub fn set_parent(&mut self, parent_id: Uuid) {
        self.parent_concept = Some(parent_id);
        self.related_concepts.insert(parent_id);
    }

    /// Add child concept.
    pub fn add_child(&mut self, child_id: Uuid) {
        self.child_concepts.insert(child_id);
        self.related_concepts.insert(child_id);
    }
}

/// Configuration for semantic memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticMemoryConfig {
    /// Maximum number of facts.
    pub max_facts: usize,
    /// Maximum number of concepts.
    pub max_concepts: usize,
    /// Minimum confidence threshold for fact retrieval.
    pub min_confidence_threshold: f32,
    /// Whether to persist to sled DB.
    pub persistence_enabled: bool,
    /// Path for sled DB persistence.
    pub persistence_path: Option<String>,
}

impl Default for SemanticMemoryConfig {
    fn default() -> Self {
        Self {
            max_facts: 100_000,
            max_concepts: 50_000,
            min_confidence_threshold: 0.1,
            persistence_enabled: false,
            persistence_path: None,
        }
    }
}

/// Semantic memory store for facts, concepts, and relationships.
#[derive(Debug)]
pub struct SemanticMemory {
    facts: DashMap<Uuid, SemanticFact>,
    concepts: DashMap<Uuid, SemanticConcept>,
    facts_by_subject: DashMap<String, HashSet<Uuid>>,
    facts_by_predicate: DashMap<String, HashSet<Uuid>>,
    concepts_by_name: DashMap<String, Uuid>,
    entries: DashMap<MemoryId, MemoryEntry>,
    db: Option<sled::Db>,
    config: SemanticMemoryConfig,
    fact_count: RwLock<usize>,
    concept_count: RwLock<usize>,
}

impl SemanticMemory {
    /// Create a new semantic memory store.
    pub fn new(config: SemanticMemoryConfig) -> MemoryResult<Self> {
        let db = if config.persistence_enabled {
            let path = config
                .persistence_path
                .as_deref()
                .unwrap_or("/tmp/neo-semantic");
            Some(
                sled::open(path)
                    .map_err(|e| MemoryError::PersistenceError(e.to_string()))?,
            )
        } else {
            None
        };
        Ok(Self {
            facts: DashMap::new(),
            concepts: DashMap::new(),
            facts_by_subject: DashMap::new(),
            facts_by_predicate: DashMap::new(),
            concepts_by_name: DashMap::new(),
            entries: DashMap::new(),
            db,
            config,
            fact_count: RwLock::new(0),
            concept_count: RwLock::new(0),
        })
    }

    /// Add a new semantic fact and return its id.
    pub fn add_fact(&self, mut fact: SemanticFact) -> MemoryResult<Uuid> {
        let count = *self.fact_count.read();
        if count >= self.config.max_facts {
            return Err(MemoryError::CapacityExceeded(
                "Semantic fact capacity reached".to_string(),
            ));
        }

        let id = fact.id;

        // Persist to sled DB.
        if let Some(ref db) = self.db {
            let key = format!("fact:{id}");
            let value = serde_json::to_vec(&fact)
                .map_err(|e| MemoryError::SerializationError(e.to_string()))?;
            db.insert(key.as_bytes(), value)
                .map_err(|e| MemoryError::PersistenceError(e.to_string()))?;
        }

        // Update indexes.
        self.facts_by_subject
            .entry(fact.subject.clone())
            .or_default()
            .insert(id);
        self.facts_by_predicate
            .entry(fact.predicate.clone())
            .or_default()
            .insert(id);

        self.facts.insert(id, fact);
        *self.fact_count.write() += 1;

        Ok(id)
    }

    /// Update an existing fact's object and confidence.
    pub fn update_fact(
        &self,
        id: Uuid,
        object: serde_json::Value,
        confidence: f32,
    ) -> MemoryResult<()> {
        let mut fact = self
            .facts
            .get_mut(&id)
            .ok_or_else(|| MemoryError::NotFound(format!("Fact {id} not found")))?;

        fact.object = object;
        fact.confidence = confidence.clamp(0.0, 1.0);
        fact.last_modified = Utc::now();
        fact.version += 1;

        // Persist update.
        if let Some(ref db) = self.db {
            let key = format!("fact:{id}");
            let value = serde_json::to_vec(fact.value())
                .map_err(|e| MemoryError::SerializationError(e.to_string()))?;
            db.insert(key.as_bytes(), value)
                .map_err(|e| MemoryError::PersistenceError(e.to_string()))?;
        }

        Ok(())
    }

    /// Remove a fact by id.
    pub fn remove_fact(&self, id: Uuid) -> MemoryResult<bool> {
        if let Some((_, fact)) = self.facts.remove(&id) {
            if let Some(mut ids) = self.facts_by_subject.get_mut(&fact.subject) {
                ids.remove(&id);
            }
            if let Some(mut ids) = self.facts_by_predicate.get_mut(&fact.predicate) {
                ids.remove(&id);
            }

            if let Some(ref db) = self.db {
                let key = format!("fact:{id}");
                let _ = db.remove(key.as_bytes());
            }

            *self.fact_count.write() -= 1;
            return Ok(true);
        }
        Ok(false)
    }

    /// Query facts by subject.
    #[must_use]
    pub fn query_subject(&self, subject: &str) -> Vec<SemanticFact> {
        self.facts_by_subject
            .get(subject)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.facts.get(id).map(|f| f.value().clone()))
                    .filter(|f| f.confidence >= self.config.min_confidence_threshold)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Query facts by predicate.
    #[must_use]
    pub fn query_predicate(&self, predicate: &str) -> Vec<SemanticFact> {
        self.facts_by_predicate
            .get(predicate)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.facts.get(id).map(|f| f.value().clone()))
                    .filter(|f| f.confidence >= self.config.min_confidence_threshold)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Query facts by both subject and predicate.
    #[must_use]
    pub fn query_relationship(
        &self,
        subject: &str,
        predicate: &str,
    ) -> Vec<SemanticFact> {
        let subject_ids = self
            .facts_by_subject
            .get(subject)
            .map(|ids| ids.iter().copied().collect::<HashSet<_>>())
            .unwrap_or_default();
        let predicate_ids = self
            .facts_by_predicate
            .get(predicate)
            .map(|ids| ids.iter().copied().collect::<HashSet<_>>())
            .unwrap_or_default();

        let matching: Vec<Uuid> = subject_ids.intersection(&predicate_ids).copied().collect();

        matching
            .iter()
            .filter_map(|id| self.facts.get(id).map(|f| f.value().clone()))
            .filter(|f| f.confidence >= self.config.min_confidence_threshold)
            .collect()
    }

    /// Search facts by keyword in subject, predicate, or tags.
    #[must_use]
    pub fn search_facts(&self, keyword: &str) -> Vec<SemanticFact> {
        let lower = keyword.to_lowercase();
        self.facts
            .iter()
            .filter(|f| {
                f.value().subject.to_lowercase().contains(&lower)
                    || f.value().predicate.to_lowercase().contains(&lower)
                    || f.value().tags.iter().any(|t| t.to_lowercase().contains(&lower))
                    || f.value()
                        .object
                        .to_string()
                        .to_lowercase()
                        .contains(&lower)
            })
            .map(|f| f.value().clone())
            .filter(|f| f.confidence >= self.config.min_confidence_threshold)
            .collect()
    }

    /// Get all facts related to a specific fact by shared subjects or predicates.
    #[must_use]
    pub fn related_facts(&self, fact_id: Uuid) -> Vec<SemanticFact> {
        let fact = match self.facts.get(&fact_id) {
            Some(f) => f.value().clone(),
            None => return Vec::new(),
        };

        let mut related = Vec::new();
        let mut seen = HashSet::new();
        seen.insert(fact_id);

        // Find facts sharing the same subject.
        if let Some(ids) = self.facts_by_subject.get(&fact.subject) {
            for &id in ids.iter() {
                if !seen.contains(&id) {
                    if let Some(f) = self.facts.get(&id) {
                        related.push(f.value().clone());
                        seen.insert(id);
                    }
                }
            }
        }

        // Find facts sharing the same predicate.
        if let Some(ids) = self.facts_by_predicate.get(&fact.predicate) {
            for &id in ids.iter() {
                if !seen.contains(&id) {
                    if let Some(f) = self.facts.get(&id) {
                        related.push(f.value().clone());
                        seen.insert(id);
                    }
                }
            }
        }

        related
    }

    /// Add a semantic concept and return its id.
    pub fn add_concept(&self, concept: SemanticConcept) -> MemoryResult<Uuid> {
        let count = *self.concept_count.read();
        if count >= self.config.max_concepts {
            return Err(MemoryError::CapacityExceeded(
                "Semantic concept capacity reached".to_string(),
            ));
        }

        let id = concept.id;

        // Persist to sled DB.
        if let Some(ref db) = self.db {
            let key = format!("concept:{id}");
            let value = serde_json::to_vec(&concept)
                .map_err(|e| MemoryError::SerializationError(e.to_string()))?;
            db.insert(key.as_bytes(), value)
                .map_err(|e| MemoryError::PersistenceError(e.to_string()))?;
        }

        self.concepts_by_name
            .entry(concept.name.clone())
            .or_insert(id);
        self.concepts.insert(id, concept);
        *self.concept_count.write() += 1;

        Ok(id)
    }

    /// Retrieve a concept by id.
    pub fn get_concept(&self, id: Uuid) -> Option<SemanticConcept> {
        self.concepts.get(&id).map(|c| c.value().clone())
    }

    /// Retrieve a concept by name.
    pub fn get_concept_by_name(&self, name: &str) -> Option<SemanticConcept> {
        let id = *self.concepts_by_name.get(name)?;
        self.concepts.get(&id).map(|c| c.value().clone())
    }

    /// Search concepts by keyword in name or definition.
    #[must_use]
    pub fn search_concepts(&self, keyword: &str) -> Vec<SemanticConcept> {
        let lower = keyword.to_lowercase();
        self.concepts
            .iter()
            .filter(|c| {
                c.value().name.to_lowercase().contains(&lower)
                    || c.value().definition.to_lowercase().contains(&lower)
            })
            .map(|c| c.value().clone())
            .collect()
    }

    /// Get concept hierarchy (parent and ancestors).
    #[must_use]
    pub fn concept_hierarchy(&self, id: Uuid) -> Vec<SemanticConcept> {
        let mut hierarchy = Vec::new();
        let mut current_id = Some(id);
        let mut visited = HashSet::new();

        while let Some(cid) = current_id {
            if visited.contains(&cid) {
                break; // Prevent infinite loops.
            }
            visited.insert(cid);

            if let Some(concept) = self.concepts.get(&cid) {
                current_id = concept.value().parent_concept;
                hierarchy.push(concept.value().clone());
            } else {
                break;
            }
        }

        hierarchy
    }

    /// Link two concepts.
    pub fn link_concepts(&self, a_id: Uuid, b_id: Uuid) -> MemoryResult<()> {
        if let Some(mut a) = self.concepts.get_mut(&a_id) {
            a.value_mut().link_to(b_id);
        } else {
            return Err(MemoryError::NotFound(format!("Concept {a_id} not found")));
        }

        if let Some(mut b) = self.concepts.get_mut(&b_id) {
            b.value_mut().link_to(a_id);
        } else {
            return Err(MemoryError::NotFound(format!("Concept {b_id} not found")));
        }

        Ok(())
    }

    /// Set parent-child relationship between concepts.
    pub fn set_concept_parent(
        &self,
        child_id: Uuid,
        parent_id: Uuid,
    ) -> MemoryResult<()> {
        if let Some(mut child) = self.concepts.get_mut(&child_id) {
            child.value_mut().set_parent(parent_id);
        } else {
            return Err(MemoryError::NotFound(format!("Child concept {child_id} not found")));
        }

        if let Some(mut parent) = self.concepts.get_mut(&parent_id) {
            parent.value_mut().add_child(child_id);
        } else {
            return Err(MemoryError::NotFound(format!("Parent concept {parent_id} not found")));
        }

        Ok(())
    }

    /// Store a memory entry alongside a semantic fact.
    pub fn store_with_entry(
        &self,
        entry: MemoryEntry,
        fact: SemanticFact,
    ) -> MemoryResult<MemoryId> {
        let memory_id = entry.id;
        self.entries.insert(memory_id, entry);
        self.add_fact(fact)?;
        Ok(memory_id)
    }

    /// Return the total number of facts.
    #[must_use]
    pub fn fact_count(&self) -> usize {
        self.facts.len()
    }

    /// Return the total number of concepts.
    #[must_use]
    pub fn concept_count(&self) -> usize {
        self.concepts.len()
    }

    /// Get facts by confidence threshold.
    #[must_use]
    pub fn high_confidence_facts(&self, min_confidence: f32) -> Vec<SemanticFact> {
        self.facts
            .iter()
            .filter(|f| f.value().confidence >= min_confidence)
            .map(|f| f.value().clone())
            .collect()
    }

    /// Get all unique subjects.
    #[must_use]
    pub fn all_subjects(&self) -> Vec<String> {
        self.facts_by_subject
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// Get all unique predicates.
    #[must_use]
    pub fn all_predicates(&self) -> Vec<String> {
        self.facts_by_predicate
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn add_and_query_fact() {
        let mem = SemanticMemory::new(SemanticMemoryConfig::default()).unwrap();
        let fact = SemanticFact::new("Neo", "is_a", json!("AGI System"))
            .with_confidence(0.95)
            .with_source("architecture_doc");

        let id = mem.add_fact(fact).unwrap();

        let results = mem.query_subject("Neo");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].subject, "Neo");
        assert_eq!(results[0].confidence, 0.95);
    }

    #[test]
    fn query_relationship() {
        let mem = SemanticMemory::new(SemanticMemoryConfig::default()).unwrap();
        mem.add_fact(SemanticFact::new("Runtime", "manages", json!("Services")))
            .unwrap();
        mem.add_fact(SemanticFact::new("Runtime", "uses", json!("Scheduler")))
            .unwrap();

        let results = mem.query_relationship("Runtime", "manages");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].predicate, "manages");
    }

    #[test]
    fn add_and_search_concept() {
        let mem = SemanticMemory::new(SemanticMemoryConfig::default()).unwrap();
        let concept = SemanticConcept::new("Neural Engine", "GPU-accelerated computation");
        let id = mem.add_concept(concept).unwrap();

        let found = mem.get_concept(id);
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Neural Engine");

        let results = mem.search_concepts("GPU");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn concept_hierarchy() {
        let mem = SemanticMemory::new(SemanticMemoryConfig::default()).unwrap();
        let parent = SemanticConcept::new("AI", "Artificial Intelligence");
        let parent_id = mem.add_concept(parent).unwrap();

        let child = SemanticConcept::new("ML", "Machine Learning");
        let child_id = mem.add_concept(child).unwrap();

        mem.set_concept_parent(child_id, parent_id).unwrap();

        let hierarchy = mem.concept_hierarchy(child_id);
        assert_eq!(hierarchy.len(), 2);
        assert_eq!(hierarchy[0].name, "ML");
        assert_eq!(hierarchy[1].name, "AI");
    }

    #[test]
    fn fact_confidence_updates() {
        let mut fact = SemanticFact::new("X", "rel", json!("Y"));
        fact.add_support(MemoryId::new());
        fact.add_support(MemoryId::new());
        fact.add_counter_evidence(MemoryId::new());
        assert!((fact.confidence - 0.666).abs() < 0.01);
    }

    #[test]
    fn capacity_limit() {
        let config = SemanticMemoryConfig {
            max_facts: 2,
            ..SemanticMemoryConfig::default()
        };
        let mem = SemanticMemory::new(config).unwrap();
        mem.add_fact(SemanticFact::new("A", "b", json!("C"))).unwrap();
        mem.add_fact(SemanticFact::new("D", "e", json!("F"))).unwrap();
        let result = mem.add_fact(SemanticFact::new("G", "h", json!("I")));
        assert!(result.is_err());
    }
}
