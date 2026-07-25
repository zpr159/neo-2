use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A piece of evidence supporting or contradicting a knowledge claim.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceRecord {
    /// Evidence id.
    pub id: String,
    /// Target entity or relation id.
    pub target_id: String,
    /// Whether the evidence supports (true) or contradicts (false) the claim.
    pub supports: bool,
    /// Description of the evidence.
    pub description: String,
    /// Source of the evidence.
    pub source: String,
    /// Confidence in the evidence (0.0 - 1.0).
    pub confidence: f32,
    /// When recorded.
    pub recorded_at: DateTime<Utc>,
}

/// Tracks evidence for and against knowledge claims.
pub struct EvidenceTracker {
    records: parking_lot::RwLock<Vec<EvidenceRecord>>,
}

impl EvidenceTracker {
    /// Create a new evidence tracker.
    #[must_use]
    pub fn new() -> Self {
        Self {
            records: parking_lot::RwLock::new(Vec::new()),
        }
    }

    /// Add a supporting evidence record.
    pub fn add_supporting(
        &self,
        target_id: impl Into<String>,
        description: impl Into<String>,
        source: impl Into<String>,
        confidence: f32,
    ) {
        self.add_evidence(target_id, true, description, source, confidence);
    }

    /// Add a contradicting evidence record.
    pub fn add_contradicting(
        &self,
        target_id: impl Into<String>,
        description: impl Into<String>,
        source: impl Into<String>,
        confidence: f32,
    ) {
        self.add_evidence(target_id, false, description, source, confidence);
    }

    fn add_evidence(
        &self,
        target_id: impl Into<String>,
        supports: bool,
        description: impl Into<String>,
        source: impl Into<String>,
        confidence: f32,
    ) {
        let record = EvidenceRecord {
            id: format!("ev-{}", chrono::Utc::now().timestamp_millis()),
            target_id: target_id.into(),
            supports,
            description: description.into(),
            source: source.into(),
            confidence: confidence.clamp(0.0, 1.0),
            recorded_at: Utc::now(),
        };
        self.records.write().push(record);
    }

    /// Get all evidence for a target.
    #[must_use]
    pub fn get_evidence(&self, target_id: &str) -> Vec<EvidenceRecord> {
        self.records
            .read()
            .iter()
            .filter(|r| r.target_id == target_id)
            .cloned()
            .collect()
    }

    /// Count supporting evidence for a target.
    #[must_use]
    pub fn support_count(&self, target_id: &str) -> usize {
        self.records
            .read()
            .iter()
            .filter(|r| r.target_id == target_id && r.supports)
            .count()
    }

    /// Count contradicting evidence for a target.
    #[must_use]
    pub fn contradiction_count(&self, target_id: &str) -> usize {
        self.records
            .read()
            .iter()
            .filter(|r| r.target_id == target_id && !r.supports)
            .count()
    }

    /// Compute net evidence score (support - contradiction weighted by confidence).
    #[must_use]
    pub fn net_score(&self, target_id: &str) -> f32 {
        let records = self.records.read();
        let relevant: Vec<&EvidenceRecord> = records
            .iter()
            .filter(|r| r.target_id == target_id)
            .collect();

        if relevant.is_empty() {
            return 0.0;
        }

        let total: f32 = relevant
            .iter()
            .map(|r| {
                if r.supports {
                    r.confidence
                } else {
                    -r.confidence
                }
            })
            .sum();

        (total / relevant.len() as f32).clamp(-1.0, 1.0)
    }

    /// Total evidence records.
    #[must_use]
    pub fn total_records(&self) -> usize {
        self.records.read().len()
    }
}

impl Default for EvidenceTracker {
    fn default() -> Self {
        Self::new()
    }
}
