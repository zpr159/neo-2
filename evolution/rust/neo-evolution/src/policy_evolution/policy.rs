use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::EvolutionId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PolicyType {
    Planning,
    Scheduling,
    Learning,
    Reasoning,
    CapabilitySelection,
    WorkflowRouting,
}

impl std::fmt::Display for PolicyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Planning => write!(f, "planning"),
            Self::Scheduling => write!(f, "scheduling"),
            Self::Learning => write!(f, "learning"),
            Self::Reasoning => write!(f, "reasoning"),
            Self::CapabilitySelection => write!(f, "capability_selection"),
            Self::WorkflowRouting => write!(f, "workflow_routing"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub condition: String,
    pub action: String,
    pub weight: f64,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub id: EvolutionId,
    pub policy_type: PolicyType,
    pub name: String,
    pub rules: Vec<PolicyRule>,
    pub version: u32,
    pub created_at: DateTime<Utc>,
    pub active: bool,
}

impl Policy {
    pub fn new(name: impl Into<String>, policy_type: PolicyType) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            policy_type,
            name: name.into(),
            rules: Vec::new(),
            version: 1,
            created_at: Utc::now(),
            active: true,
        }
    }

    pub fn evaluate(&self, _context: &HashMap<String, serde_json::Value>) -> Option<&PolicyRule> {
        self.rules.iter().filter(|r| r.enabled).max_by(|a, b| {
            a.weight
                .partial_cmp(&b.weight)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    pub fn mutate(&self, intensity: f64) -> Self {
        let mut mutated = self.clone();
        for rule in &mut mutated.rules {
            let perturbation = (rand::random::<f64>() - 0.5) * intensity;
            rule.weight = (rule.weight + perturbation).clamp(0.0, 10.0);
        }
        mutated.version += 1;
        mutated
    }

    pub fn version_up(&mut self) {
        self.version += 1;
    }

    pub fn add_rule(&mut self, rule: PolicyRule) {
        self.rules.push(rule);
        self.version += 1;
    }
}
