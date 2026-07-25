use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::{Confidence, EvidenceId};

/// A single piece of evidence supporting a belief.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub id: EvidenceId,
    pub description: String,
    pub source: String,
    pub source_reliability: f32,
    pub weight: f32,
    pub timestamp: DateTime<Utc>,
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

impl Evidence {
    pub fn new(description: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            id: EvidenceId::random(),
            description: description.into(),
            source: source.into(),
            source_reliability: 0.5,
            weight: 1.0,
            timestamp: Utc::now(),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Effective weight = weight * source_reliability.
    #[must_use]
    pub fn effective_weight(&self) -> f32 {
        self.weight * self.source_reliability
    }
}

/// Accumulates evidence and computes Bayesian confidence.
#[derive(Debug, Clone)]
pub struct ConfidenceAccumulator {
    prior: f32,
    evidence: Vec<Evidence>,
}

impl ConfidenceAccumulator {
    pub fn new(prior: f32) -> Self {
        Self {
            prior: prior.clamp(0.001, 0.999),
            evidence: Vec::new(),
        }
    }

    /// Add a piece of evidence (likelihood ratio).
    pub fn add_evidence(&mut self, evidence: Evidence) {
        self.evidence.push(evidence);
    }

    /// Compute posterior confidence using naive Bayes fusion.
    #[must_use]
    pub fn posterior(&self) -> Confidence {
        if self.evidence.is_empty() {
            return Confidence(self.prior);
        }

        let mut log_odds = (self.prior / (1.0 - self.prior)).ln();
        for e in &self.evidence {
            let lr = e.effective_weight().max(0.01);
            log_odds += lr.ln();
        }
        let odds = log_odds.exp();
        let prob = odds / (1.0 + odds);
        Confidence(prob.clamp(0.0, 1.0))
    }

    /// Get the number of evidence records.
    #[must_use]
    pub fn evidence_count(&self) -> usize {
        self.evidence.len()
    }

    /// Get all evidence.
    pub fn evidence(&self) -> &[Evidence] {
        &self.evidence
    }
}

/// Apply confidence decay over time.
#[must_use]
pub fn apply_decay(confidence: f32, elapsed_secs: f64, half_life_secs: f64) -> f32 {
    if half_life_secs <= 0.0 {
        return confidence;
    }
    let decay_factor = (-elapsed_secs * std::f64::consts::LN_2 / half_life_secs).exp() as f32;
    (confidence * decay_factor).clamp(0.0, 1.0)
}

/// Merge multiple confidence scores using weighted average.
#[must_use]
pub fn merge_confidences(scores: &[(f32, f32)]) -> Confidence {
    if scores.is_empty() {
        return Confidence::UNKNOWN;
    }
    let mut total_weight = 0.0f32;
    let mut weighted_sum = 0.0f32;
    for &(score, weight) in scores {
        weighted_sum += score * weight;
        total_weight += weight;
    }
    if total_weight > 0.0 {
        Confidence(weighted_sum / total_weight)
    } else {
        Confidence::UNKNOWN
    }
}

/// Score source reliability based on historical accuracy.
#[derive(Debug, Clone)]
pub struct SourceReliability {
    correct: u64,
    total: u64,
}

impl SourceReliability {
    #[must_use]
    pub fn new() -> Self {
        Self { correct: 0, total: 0 }
    }

    pub fn record(&mut self, correct: bool) {
        self.total += 1;
        if correct {
            self.correct += 1;
        }
    }

    #[must_use]
    pub fn reliability(&self) -> f32 {
        if self.total == 0 {
            return 0.5;
        }
        self.correct as f32 / self.total as f32
    }

    #[must_use]
    pub fn confidence_interval(&self) -> (f32, f32) {
        if self.total == 0 {
            return (0.0, 1.0);
        }
        let p = self.reliability();
        let n = self.total as f32;
        let se = (p * (1.0 - p) / n).sqrt();
        let z = 1.96;
        ((p - z * se).max(0.0), (p + z * se).min(1.0))
    }
}

impl Default for SourceReliability {
    fn default() -> Self {
        Self::new()
    }
}
