use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::{EvolutionId, SubsystemTarget};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heuristic {
    pub id: EvolutionId,
    pub name: String,
    pub description: String,
    pub category: SubsystemTarget,
    pub parameters: HashMap<String, f64>,
    pub score: f64,
    pub usage_count: u64,
    pub created_at: DateTime<Utc>,
    pub retired: bool,
}

impl Heuristic {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        category: SubsystemTarget,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            name: name.into(),
            description: description.into(),
            category,
            parameters: HashMap::new(),
            score: 0.5,
            usage_count: 0,
            created_at: Utc::now(),
            retired: false,
        }
    }

    pub fn update_score(&mut self, new_score: f64) {
        self.score = new_score.clamp(0.0, 1.0);
    }

    pub fn increment_usage(&mut self) {
        self.usage_count += 1;
    }

    pub fn retire(&mut self) {
        self.retired = true;
    }
}
