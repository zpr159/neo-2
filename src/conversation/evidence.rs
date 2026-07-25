use serde::{Deserialize, Serialize};

use crate::time::Timestamp;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceSource {
    Memory,
    KnowledgeGraph,
    WorldModel,
    Reasoning,
    Planning,
    Executive,
    Agent,
    Workflow,
    Tool,
    UserInput,
    ConversationHistory,
    External,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub id: uuid::Uuid,
    pub source: EvidenceSource,
    pub confidence: f32,
    pub timestamp: Timestamp,
    pub retrieval_method: String,
    pub supporting_references: Vec<String>,
    pub explanation: Option<String>,
    pub relevance_score: f32,
    pub content: String,
    pub provenance: Provenance,
}

impl Evidence {
    pub fn new(
        source: EvidenceSource,
        confidence: f32,
        content: impl Into<String>,
        retrieval_method: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            source,
            confidence,
            timestamp: Timestamp::now(),
            retrieval_method: retrieval_method.into(),
            supporting_references: Vec::new(),
            explanation: None,
            relevance_score: 0.0,
            content: content.into(),
            provenance: Provenance::default(),
        }
    }

    pub fn with_relevance(mut self, score: f32) -> Self {
        self.relevance_score = score;
        self
    }

    pub fn with_explanation(mut self, explanation: impl Into<String>) -> Self {
        self.explanation = Some(explanation.into());
        self
    }

    pub fn with_reference(mut self, reference: impl Into<String>) -> Self {
        self.supporting_references.push(reference.into());
        self
    }

    pub fn with_provenance(mut self, provenance: Provenance) -> Self {
        self.provenance = provenance;
        self
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Provenance {
    pub chain: Vec<ProvenanceStep>,
    pub root_source: Option<String>,
    pub derivation_method: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceStep {
    pub source: EvidenceSource,
    pub operation: String,
    pub timestamp: Timestamp,
    pub confidence: f32,
}

impl Provenance {
    pub fn step(
        mut self,
        source: EvidenceSource,
        operation: impl Into<String>,
        confidence: f32,
    ) -> Self {
        self.chain.push(ProvenanceStep {
            source,
            operation: operation.into(),
            timestamp: Timestamp::now(),
            confidence,
        });
        self
    }

    pub fn root(mut self, source: impl Into<String>) -> Self {
        self.root_source = Some(source.into());
        self
    }

    pub fn derivation(mut self, method: impl Into<String>) -> Self {
        self.derivation_method = Some(method.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceCollection {
    pub items: Vec<Evidence>,
    pub total_confidence: f32,
    pub source_distribution: std::collections::HashMap<EvidenceSource, usize>,
}

impl EvidenceCollection {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            total_confidence: 0.0,
            source_distribution: std::collections::HashMap::new(),
        }
    }

    pub fn push(&mut self, evidence: Evidence) {
        self.total_confidence += evidence.confidence;
        *self
            .source_distribution
            .entry(evidence.source.clone())
            .or_insert(0) += 1;
        self.items.push(evidence);
    }

    pub fn average_confidence(&self) -> f32 {
        if self.items.is_empty() {
            0.0
        } else {
            self.total_confidence / self.items.len() as f32
        }
    }

    pub fn by_source(&self, source: &EvidenceSource) -> Vec<&Evidence> {
        self.items
            .iter()
            .filter(|e| e.source == *source)
            .collect()
    }

    pub fn sorted_by_confidence(&self) -> Vec<&Evidence> {
        let mut items: Vec<&Evidence> = self.items.iter().collect();
        items.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
        items
    }

    pub fn sorted_by_relevance(&self) -> Vec<&Evidence> {
        let mut items: Vec<&Evidence> = self.items.iter().collect();
        items.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap_or(std::cmp::Ordering::Equal));
        items
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl Default for EvidenceCollection {
    fn default() -> Self {
        Self::new()
    }
}
