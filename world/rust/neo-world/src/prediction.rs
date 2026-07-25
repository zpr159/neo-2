use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::{Confidence, EntityId, PredictionId, PredictionType};

/// Prediction about future world state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prediction {
    pub id: PredictionId,
    pub description: String,
    pub prediction_type: PredictionType,
    pub confidence: Confidence,
    pub predicted_at: DateTime<Utc>,
    pub predicted_for: Option<DateTime<Utc>>,
    pub context_entity_ids: Vec<EntityId>,
    pub reasoning: String,
    pub actual_outcome: Option<String>,
    pub was_correct: Option<bool>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Engine that generates predictions.
pub struct PredictionEngine {
    predictions: dashmap::DashMap<PredictionId, Prediction>,
    prediction_count: std::sync::atomic::AtomicU64,
}

impl PredictionEngine {
    #[must_use]
    pub fn new() -> Self {
        Self {
            predictions: dashmap::DashMap::new(),
            prediction_count: std::sync::atomic::AtomicU64::new(0),
        }
    }

    pub fn predict(
        &self,
        description: impl Into<String>,
        prediction_type: PredictionType,
        confidence: Confidence,
        reasoning: impl Into<String>,
    ) -> PredictionId {
        let pred = Prediction {
            id: PredictionId::random(),
            description: description.into(),
            prediction_type,
            confidence,
            predicted_at: Utc::now(),
            predicted_for: None,
            context_entity_ids: Vec::new(),
            reasoning: reasoning.into(),
            actual_outcome: None,
            was_correct: None,
            metadata: HashMap::new(),
        };
        let id = pred.id.clone();
        self.predictions.insert(id.clone(), pred);
        self.prediction_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        id
    }

    pub fn get(&self, id: &PredictionId) -> Option<Prediction> {
        self.predictions.get(id).map(|p| p.value().clone())
    }

    pub fn record_outcome(&self, id: &PredictionId, outcome: &str, correct: bool) -> bool {
        if let Some(mut pred) = self.predictions.get_mut(id) {
            pred.actual_outcome = Some(outcome.to_string());
            pred.was_correct = Some(correct);
            true
        } else {
            false
        }
    }

    pub fn accuracy(&self) -> f32 {
        let evaluated: Vec<bool> = self
            .predictions
            .iter()
            .filter_map(|p| p.value().was_correct)
            .collect();
        if evaluated.is_empty() {
            return 0.0;
        }
        let correct = evaluated.iter().filter(|&&c| c).count();
        correct as f32 / evaluated.len() as f32
    }

    pub fn total_predictions(&self) -> u64 {
        self.prediction_count.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn recent(&self, count: usize) -> Vec<Prediction> {
        let mut preds: Vec<Prediction> = self.predictions.iter().map(|p| p.value().clone()).collect();
        preds.sort_by(|a, b| b.predicted_at.cmp(&a.predicted_at));
        preds.into_iter().take(count).collect()
    }

    pub fn by_type(&self, pred_type: &PredictionType) -> Vec<Prediction> {
        self.predictions
            .iter()
            .filter(|p| &p.value().prediction_type == pred_type)
            .map(|p| p.value().clone())
            .collect()
    }

    pub fn unresolved(&self) -> Vec<Prediction> {
        self.predictions
            .iter()
            .filter(|p| p.value().was_correct.is_none())
            .map(|p| p.value().clone())
            .collect()
    }
}

impl Default for PredictionEngine {
    fn default() -> Self {
        Self::new()
    }
}
