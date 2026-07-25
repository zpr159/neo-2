use super::api::ResearchEvidence;
use crate::time::Timestamp;

/// Research-specific evidence type with citation tracking.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResearchEvidenceItem {
    pub id: uuid::Uuid,
    pub finding_id: uuid::Uuid,
    pub content: String,
    pub source_url: Option<String>,
    pub source_name: String,
    pub confidence: f32,
    pub fetched_at: Timestamp,
    pub relevance_score: f32,
}

impl ResearchEvidenceItem {
    pub fn from_evidence(evidence: &ResearchEvidence, finding_id: uuid::Uuid) -> Self {
        Self {
            id: evidence.id,
            finding_id,
            content: evidence.content.clone(),
            source_url: evidence.source_url.clone(),
            source_name: evidence.source_name.clone(),
            confidence: evidence.confidence,
            fetched_at: evidence.extracted_at,
            relevance_score: evidence.relevance_score,
        }
    }
}

/// Collection of research evidence items with aggregation methods.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ResearchEvidenceCollection {
    pub items: Vec<ResearchEvidenceItem>,
}

impl ResearchEvidenceCollection {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
        }
    }

    pub fn push(&mut self, item: ResearchEvidenceItem) {
        self.items.push(item);
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn average_confidence(&self) -> f32 {
        if self.items.is_empty() {
            return 0.0;
        }
        self.items.iter().map(|i| i.confidence).sum::<f32>() / self.items.len() as f32
    }

    pub fn by_source(&self, source: &str) -> Vec<&ResearchEvidenceItem> {
        self.items
            .iter()
            .filter(|i| i.source_name == source)
            .collect()
    }

    pub fn sorted_by_confidence(&self) -> Vec<&ResearchEvidenceItem> {
        let mut items: Vec<&ResearchEvidenceItem> = self.items.iter().collect();
        items.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        items
    }

    pub fn sorted_by_relevance(&self) -> Vec<&ResearchEvidenceItem> {
        let mut items: Vec<&ResearchEvidenceItem> = self.items.iter().collect();
        items.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        items
    }

    pub fn unique_sources(&self) -> Vec<String> {
        let mut sources: Vec<String> = self
            .items
            .iter()
            .map(|i| i.source_name.clone())
            .collect();
        sources.sort();
        sources.dedup();
        sources
    }
}
