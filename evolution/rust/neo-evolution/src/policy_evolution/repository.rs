use dashmap::DashMap;

use crate::error::{EvolutionError, EvolutionResult};
use crate::policy_evolution::policy::{Policy, PolicyType};
use crate::types::EvolutionId;

pub struct PolicyRepository {
    store: DashMap<EvolutionId, Policy>,
    active_policies: DashMap<PolicyType, EvolutionId>,
}

impl PolicyRepository {
    pub fn new() -> Self {
        Self {
            store: DashMap::new(),
            active_policies: DashMap::new(),
        }
    }

    pub fn save(&self, policy: Policy) {
        let id = policy.id;
        self.store.insert(id, policy);
    }

    pub fn load(&self, id: EvolutionId) -> Option<Policy> {
        self.store.get(&id).map(|p| p.value().clone())
    }

    pub fn list_all(&self) -> Vec<Policy> {
        self.store.iter().map(|p| p.value().clone()).collect()
    }

    pub fn list_by_type(&self, policy_type: PolicyType) -> Vec<Policy> {
        self.store
            .iter()
            .filter(|p| p.policy_type == policy_type)
            .map(|p| p.value().clone())
            .collect()
    }

    pub fn get_active(&self, policy_type: PolicyType) -> Option<Policy> {
        self.active_policies
            .get(&policy_type)
            .and_then(|entry| self.store.get(entry.value()).map(|p| p.value().clone()))
    }

    pub fn set_active(&self, policy: &Policy) {
        self.active_policies.insert(policy.policy_type, policy.id);
    }

    pub fn deactivate(&self, policy_type: PolicyType) {
        self.active_policies.remove(&policy_type);
    }
}

impl Default for PolicyRepository {
    fn default() -> Self {
        Self::new()
    }
}
