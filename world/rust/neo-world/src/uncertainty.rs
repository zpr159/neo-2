use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::confidence::{ConfidenceAccumulator, Evidence};
use crate::types::Confidence;

/// An uncertainty record tracking what we don't know.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Uncertainty {
    pub description: String,
    pub category: UncertaintyCategory,
    pub confidence: Confidence,
    pub evidence_count: usize,
    pub created_at: DateTime<Utc>,
    pub resolved: bool,
    pub resolution: Option<String>,
}

/// Categories of uncertainty.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UncertaintyCategory {
    MissingData,
    ConflictingEvidence,
    LowReliability,
    PredictionUncertainty,
    CausalAmbiguity,
    TemporalAmbiguity,
    SpatialAmbiguity,
    ModelUncertainty,
    SensorNoise,
    Unknown,
}

impl std::fmt::Display for UncertaintyCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingData => write!(f, "missing_data"),
            Self::ConflictingEvidence => write!(f, "conflicting_evidence"),
            Self::LowReliability => write!(f, "low_reliability"),
            Self::PredictionUncertainty => write!(f, "prediction_uncertainty"),
            Self::CausalAmbiguity => write!(f, "causal_ambiguity"),
            Self::TemporalAmbiguity => write!(f, "temporal_ambiguity"),
            Self::SpatialAmbiguity => write!(f, "spatial_ambiguity"),
            Self::ModelUncertainty => write!(f, "model_uncertainty"),
            Self::SensorNoise => write!(f, "sensor_noise"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

/// Tracks uncertainty across the world model.
pub struct UncertaintyTracker {
    active: dashmap::DashMap<String, Uncertainty>,
    resolved_history: Vec<Uncertainty>,
    evidence_store: dashmap::DashMap<String, ConfidenceAccumulator>,
}

impl UncertaintyTracker {
    #[must_use]
    pub fn new() -> Self {
        Self {
            active: dashmap::DashMap::new(),
            resolved_history: Vec::new(),
            evidence_store: dashmap::DashMap::new(),
        }
    }

    /// Register a new uncertainty.
    pub fn register(&self, key: impl Into<String>, description: impl Into<String>, category: UncertaintyCategory) {
        let key = key.into();
        let uncertainty = Uncertainty {
            description: description.into(),
            category,
            confidence: Confidence::UNKNOWN,
            evidence_count: 0,
            created_at: Utc::now(),
            resolved: false,
            resolution: None,
        };
        self.active.insert(key, uncertainty);
    }

    /// Add evidence to an uncertainty, updating confidence.
    pub fn add_evidence(&self, key: &str, evidence: Evidence) {
        let mut acc = self
            .evidence_store
            .entry(key.to_string())
            .or_insert_with(|| ConfidenceAccumulator::new(0.5));
        acc.add_evidence(evidence);

        if let Some(mut u) = self.active.get_mut(key) {
            u.confidence = acc.posterior();
            u.evidence_count = acc.evidence_count();
        }
    }

    /// Resolve an uncertainty.
    pub fn resolve(&mut self, key: &str, resolution: impl Into<String>) -> bool {
        if let Some((_, mut u)) = self.active.remove(key) {
            u.resolved = true;
            u.resolution = Some(resolution.into());
            self.resolved_history.push(u);
            true
        } else {
            false
        }
    }

    /// Get all active uncertainties.
    pub fn active_uncertainties(&self) -> Vec<Uncertainty> {
        self.active.iter().map(|u| u.value().clone()).collect()
    }

    /// Get active uncertainties by category.
    pub fn by_category(&self, category: &UncertaintyCategory) -> Vec<Uncertainty> {
        self.active
            .iter()
            .filter(|u| &u.value().category == category)
            .map(|u| u.value().clone())
            .collect()
    }

    /// Get unresolved count.
    #[must_use]
    pub fn unresolved_count(&self) -> usize {
        self.active.len()
    }

    /// Get average confidence of active uncertainties.
    #[must_use]
    pub fn average_confidence(&self) -> f32 {
        let entries: Vec<f32> = self.active.iter().map(|u| u.value().confidence.value()).collect();
        if entries.is_empty() {
            return 0.0;
        }
        entries.iter().sum::<f32>() / entries.len() as f32
    }
}

impl Default for UncertaintyTracker {
    fn default() -> Self {
        Self::new()
    }
}
