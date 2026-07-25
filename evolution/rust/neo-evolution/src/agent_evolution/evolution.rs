use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::config::EvolutionConfiguration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleEvolution {
    pub agent_id: String,
    pub old_role: String,
    pub new_role: String,
    pub reason: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorEvolution {
    pub agent_id: String,
    pub behavior_type: String,
    pub change_description: String,
    pub impact_score: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationEvolution {
    pub agent_id: String,
    pub pattern: String,
    pub improvement: String,
    pub timestamp: DateTime<Utc>,
}

pub struct AgentEvolution {
    role_evolutions: DashMap<String, Vec<RoleEvolution>>,
    behavior_evolutions: DashMap<String, Vec<BehaviorEvolution>>,
    communication_evolutions: DashMap<String, Vec<CommunicationEvolution>>,
    #[allow(dead_code)]
    config: EvolutionConfiguration,
}

impl AgentEvolution {
    pub fn new(config: EvolutionConfiguration) -> Arc<Self> {
        Arc::new(Self {
            role_evolutions: DashMap::new(),
            behavior_evolutions: DashMap::new(),
            communication_evolutions: DashMap::new(),
            config,
        })
    }

    pub fn evolve_role(
        &self,
        agent_id: impl Into<String>,
        old_role: impl Into<String>,
        new_role: impl Into<String>,
        reason: impl Into<String>,
    ) -> RoleEvolution {
        let evo = RoleEvolution {
            agent_id: agent_id.into(),
            old_role: old_role.into(),
            new_role: new_role.into(),
            reason: reason.into(),
            timestamp: Utc::now(),
        };
        self.role_evolutions
            .entry(evo.agent_id.clone())
            .or_default()
            .push(evo.clone());
        evo
    }

    pub fn evolve_behavior(
        &self,
        agent_id: impl Into<String>,
        behavior_type: impl Into<String>,
        change: impl Into<String>,
        impact: f64,
    ) -> BehaviorEvolution {
        let evo = BehaviorEvolution {
            agent_id: agent_id.into(),
            behavior_type: behavior_type.into(),
            change_description: change.into(),
            impact_score: impact,
            timestamp: Utc::now(),
        };
        self.behavior_evolutions
            .entry(evo.agent_id.clone())
            .or_default()
            .push(evo.clone());
        evo
    }

    pub fn evolve_communication(
        &self,
        agent_id: impl Into<String>,
        pattern: impl Into<String>,
        improvement: impl Into<String>,
    ) -> CommunicationEvolution {
        let evo = CommunicationEvolution {
            agent_id: agent_id.into(),
            pattern: pattern.into(),
            improvement: improvement.into(),
            timestamp: Utc::now(),
        };
        self.communication_evolutions
            .entry(evo.agent_id.clone())
            .or_default()
            .push(evo.clone());
        evo
    }

    pub fn get_role_history(&self, agent_id: &str) -> Vec<RoleEvolution> {
        self.role_evolutions
            .get(agent_id)
            .map(|v| v.value().clone())
            .unwrap_or_default()
    }

    pub fn get_behavior_history(&self, agent_id: &str) -> Vec<BehaviorEvolution> {
        self.behavior_evolutions
            .get(agent_id)
            .map(|v| v.value().clone())
            .unwrap_or_default()
    }

    pub fn get_communication_history(&self, agent_id: &str) -> Vec<CommunicationEvolution> {
        self.communication_evolutions
            .get(agent_id)
            .map(|v| v.value().clone())
            .unwrap_or_default()
    }
}
