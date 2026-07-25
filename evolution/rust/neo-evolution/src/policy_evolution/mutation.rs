use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::policy_evolution::policy::Policy;
use crate::types::EvolutionId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PolicyMutationType {
    AddRule,
    RemoveRule,
    ModifyWeight,
    ModifyCondition,
    ModifyAction,
    Crossover,
}

impl std::fmt::Display for PolicyMutationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AddRule => write!(f, "add_rule"),
            Self::RemoveRule => write!(f, "remove_rule"),
            Self::ModifyWeight => write!(f, "modify_weight"),
            Self::ModifyCondition => write!(f, "modify_condition"),
            Self::ModifyAction => write!(f, "modify_action"),
            Self::Crossover => write!(f, "crossover"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyMutation {
    pub id: EvolutionId,
    pub policy_id: EvolutionId,
    pub mutation_type: PolicyMutationType,
    pub original: Policy,
    pub mutated: Policy,
    pub timestamp: DateTime<Utc>,
}

pub struct PolicyMutationEngine;

impl PolicyMutationEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn mutate_policy(
        &self,
        policy: &Policy,
        mutation_type: PolicyMutationType,
        intensity: f64,
    ) -> PolicyMutation {
        let mutated = match mutation_type {
            PolicyMutationType::ModifyWeight => policy.mutate(intensity),
            PolicyMutationType::AddRule => {
                let mut p = policy.clone();
                p.add_rule(crate::policy_evolution::policy::PolicyRule {
                    condition: "auto_generated".into(),
                    action: "default".into(),
                    weight: 1.0,
                    enabled: true,
                });
                p
            }
            PolicyMutationType::RemoveRule => {
                let mut p = policy.clone();
                if !p.rules.is_empty() {
                    p.rules.remove(p.rules.len() - 1);
                    p.version += 1;
                }
                p
            }
            PolicyMutationType::ModifyCondition => {
                let mut p = policy.clone();
                for rule in &mut p.rules {
                    rule.condition = format!("{}_mutated", rule.condition);
                }
                p.version += 1;
                p
            }
            PolicyMutationType::ModifyAction => {
                let mut p = policy.clone();
                for rule in &mut p.rules {
                    rule.action = format!("{}_mutated", rule.action);
                }
                p.version += 1;
                p
            }
            PolicyMutationType::Crossover => policy.mutate(intensity * 0.5),
        };

        PolicyMutation {
            id: uuid::Uuid::new_v4(),
            policy_id: policy.id,
            mutation_type,
            original: policy.clone(),
            mutated,
            timestamp: Utc::now(),
        }
    }

    pub fn apply_mutation(&self, mutation: &PolicyMutation) -> Policy {
        mutation.mutated.clone()
    }

    pub fn rollback_mutation(&self, mutation: &PolicyMutation) -> Policy {
        mutation.original.clone()
    }
}

impl Default for PolicyMutationEngine {
    fn default() -> Self {
        Self::new()
    }
}
