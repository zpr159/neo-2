use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;


#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HypothesisStatus {
    Generated,
    Supported,
    Weakened,
    Rejected,
    Confirmed,
}

impl std::fmt::Display for HypothesisStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Generated => write!(f, "generated"),
            Self::Supported => write!(f, "supported"),
            Self::Weakened => write!(f, "weakened"),
            Self::Rejected => write!(f, "rejected"),
            Self::Confirmed => write!(f, "confirmed"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub id: Uuid,
    pub description: String,
    pub supports: Vec<Uuid>,
    pub contradicts: Vec<Uuid>,
    pub strength: f32,
    pub source: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl Evidence {
    pub fn new(description: String, strength: f32) -> Self {
        Self {
            id: Uuid::new_v4(),
            description,
            supports: Vec::new(),
            contradicts: Vec::new(),
            strength,
            source: None,
            timestamp: Utc::now(),
        }
    }

    pub fn supports_hypothesis(mut self, hypothesis_id: Uuid) -> Self {
        self.supports.push(hypothesis_id);
        self
    }

    pub fn contradicts_hypothesis(mut self, hypothesis_id: Uuid) -> Self {
        self.contradicts.push(hypothesis_id);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hypothesis {
    pub id: Uuid,
    pub statement: String,
    pub status: HypothesisStatus,
    pub confidence: f32,
    pub supporting_evidence: Vec<Uuid>,
    pub contradicting_evidence: Vec<Uuid>,
    pub generated_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Hypothesis {
    pub fn new(statement: String, confidence: f32) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            statement,
            status: HypothesisStatus::Generated,
            confidence,
            supporting_evidence: Vec::new(),
            contradicting_evidence: Vec::new(),
            generated_at: now,
            updated_at: now,
            metadata: HashMap::new(),
        }
    }

    pub fn net_support(&self, evidence: &[Evidence]) -> f32 {
        let supporting: f32 = evidence
            .iter()
            .filter(|e| self.supporting_evidence.contains(&e.id))
            .map(|e| e.strength)
            .sum();

        let contradicting: f32 = evidence
            .iter()
            .filter(|e| self.contradicting_evidence.contains(&e.id))
            .map(|e| e.strength)
            .sum();

        supporting - contradicting
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HypothesisRanking {
    pub hypothesis_id: Uuid,
    pub rank: usize,
    pub score: f32,
    pub reasoning: String,
}

#[derive(Debug)]
pub struct HypothesisEngine {
    max_hypotheses: usize,
    _min_confidence: f32,
    evidence_acc: Vec<Evidence>,
}

impl HypothesisEngine {
    pub fn new() -> Self {
        Self {
            max_hypotheses: 10,
            _min_confidence: 0.1,
            evidence_acc: Vec::new(),
        }
    }

    pub fn with_max_hypotheses(mut self, max: usize) -> Self {
        self.max_hypotheses = max;
        self
    }

    pub fn generate_hypotheses(
        &self,
        query: &str,
        context: &HashMap<String, serde_json::Value>,
        count: usize,
    ) -> Vec<Hypothesis> {
        let mut hypotheses = Vec::new();
        let limit = count.min(self.max_hypotheses);

        let context_facts: Vec<String> = context
            .values()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();

        for i in 0..limit {
            let confidence = 0.5 + (i as f32 * 0.03) - (i as f32 * 0.05);
            let statement = if context_facts.is_empty() {
                format!("Hypothesis {}: potential explanation for '{}'", i + 1, query)
            } else {
                let fact = &context_facts[i % context_facts.len()];
                format!("Hypothesis {}: '{}' may be related to {}", i + 1, query, fact)
            };

            hypotheses.push(Hypothesis::new(statement, confidence.clamp(0.1, 0.9)));
        }

        hypotheses
    }

    pub fn add_evidence(&mut self, evidence: Evidence) {
        self.evidence_acc.push(evidence);
    }

    pub fn rank_hypotheses(&self, hypotheses: &[Hypothesis]) -> Vec<HypothesisRanking> {
        let mut scored: Vec<(usize, f32, String)> = hypotheses
            .iter()
            .enumerate()
            .map(|(i, h)| {
                let net = h.net_support(&self.evidence_acc);
                let score = h.confidence + net;
                let reasoning = format!(
                    "confidence={:.2}, net_support={:.2}",
                    h.confidence, net
                );
                (i, score, reasoning)
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        scored
            .into_iter()
            .enumerate()
            .map(|(rank, (idx, score, reasoning))| HypothesisRanking {
                hypothesis_id: hypotheses[idx].id,
                rank: rank + 1,
                score,
                reasoning,
            })
            .collect()
    }

    pub fn discard_weak(
        &self,
        hypotheses: &mut Vec<Hypothesis>,
        threshold: f32,
    ) -> Vec<Hypothesis> {
        let mut discarded = Vec::new();
        hypotheses.retain(|h| {
            let net = h.net_support(&self.evidence_acc);
            let effective_conf = h.confidence + net;
            if effective_conf < threshold {
                discarded.push(h.clone());
                false
            } else {
                true
            }
        });
        discarded
    }

    pub fn accumulate_evidence(
        &mut self,
        hypothesis_id: Uuid,
        evidence: Evidence,
        supports: bool,
    ) {
        let mut ev = evidence;
        if supports {
            ev.supports.push(hypothesis_id);
        } else {
            ev.contradicts.push(hypothesis_id);
        }
        self.evidence_acc.push(ev);
    }

    pub fn update_statuses(&self, hypotheses: &mut [Hypothesis]) {
        for h in hypotheses.iter_mut() {
            let support_count = h.supporting_evidence.len();
            let contra_count = h.contradicting_evidence.len();

            h.status = if support_count > contra_count * 2 {
                HypothesisStatus::Confirmed
            } else if support_count > contra_count {
                HypothesisStatus::Supported
            } else if contra_count > support_count * 2 {
                HypothesisStatus::Rejected
            } else if contra_count > support_count {
                HypothesisStatus::Weakened
            } else {
                HypothesisStatus::Generated
            };

            h.updated_at = Utc::now();
        }
    }

    pub fn best_hypothesis<'a>(&self, hypotheses: &'a [Hypothesis]) -> Option<&'a Hypothesis> {
        hypotheses
            .iter()
            .filter(|h| h.status != HypothesisStatus::Rejected)
            .max_by(|a, b| {
                a.confidence
                    .partial_cmp(&b.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    pub fn evidence_count(&self) -> usize {
        self.evidence_acc.len()
    }

    pub fn clear_evidence(&mut self) {
        self.evidence_acc.clear();
    }
}

impl Default for HypothesisEngine {
    fn default() -> Self {
        Self::new()
    }
}
