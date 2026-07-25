use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A provenance record for a piece of knowledge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceRecord {
    /// Entity or relation id.
    pub target_id: String,
    /// Source description (e.g., "memory:abc123", "user_input", "inference_engine").
    pub source: String,
    /// When the source was recorded.
    pub recorded_at: DateTime<Utc>,
    /// Confidence in this source.
    pub confidence: f32,
    /// Additional metadata.
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

/// Tracks the provenance of knowledge elements.
pub struct SourceTracker {
    records: parking_lot::RwLock<Vec<ProvenanceRecord>>,
}

impl SourceTracker {
    /// Create a new source tracker.
    #[must_use]
    pub fn new() -> Self {
        Self {
            records: parking_lot::RwLock::new(Vec::new()),
        }
    }

    /// Record a source for a knowledge element.
    pub fn record_source(
        &self,
        target_id: impl Into<String>,
        source: impl Into<String>,
        confidence: f32,
    ) {
        let record = ProvenanceRecord {
            target_id: target_id.into(),
            source: source.into(),
            recorded_at: Utc::now(),
            confidence: confidence.clamp(0.0, 1.0),
            metadata: std::collections::HashMap::new(),
        };
        self.records.write().push(record);
    }

    /// Get all provenance records for a target.
    #[must_use]
    pub fn get_sources(&self, target_id: &str) -> Vec<ProvenanceRecord> {
        self.records
            .read()
            .iter()
            .filter(|r| r.target_id == target_id)
            .cloned()
            .collect()
    }

    /// Get the number of distinct sources for a target.
    #[must_use]
    pub fn source_count(&self, target_id: &str) -> usize {
        self.records
            .read()
            .iter()
            .filter(|r| r.target_id == target_id)
            .map(|r| r.source.as_str())
            .collect::<std::collections::HashSet<_>>()
            .len()
    }

    /// Get total records.
    #[must_use]
    pub fn total_records(&self) -> usize {
        self.records.read().len()
    }

    /// Get all unique sources.
    #[must_use]
    pub fn all_sources(&self) -> Vec<String> {
        self.records
            .read()
            .iter()
            .map(|r| r.source.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect()
    }
}

impl Default for SourceTracker {
    fn default() -> Self {
        Self::new()
    }
}
