use std::sync::Arc;

use crate::config::EvolutionConfiguration;
use crate::error::{EvolutionError, EvolutionResult};
use crate::policy_evolution::evaluation::PolicyEvaluator;
use crate::policy_evolution::mutation::{PolicyMutation, PolicyMutationEngine};
use crate::policy_evolution::policy::{Policy, PolicyType};
use crate::policy_evolution::repository::PolicyRepository;
use crate::types::EvolutionId;

pub struct PolicyEvolutionEngine {
    repository: PolicyRepository,
    mutation_engine: PolicyMutationEngine,
    evaluator: PolicyEvaluator,
    config: EvolutionConfiguration,
}

impl PolicyEvolutionEngine {
    pub fn new(config: EvolutionConfiguration) -> Arc<Self> {
        Arc::new(Self {
            repository: PolicyRepository::new(),
            mutation_engine: PolicyMutationEngine::new(),
            evaluator: PolicyEvaluator::new(),
            config,
        })
    }

    pub fn evolve_policy(&self, policy_type: PolicyType) -> EvolutionResult<PolicyMutation> {
        let policy = self
            .repository
            .get_active(policy_type)
            .or_else(|| {
                let policies = self.repository.list_by_type(policy_type);
                policies.into_iter().next()
            })
            .ok_or_else(|| EvolutionError::NotFound(format!("no policy for {policy_type}")))?;

        let mutation = self.mutation_engine.mutate_policy(
            &policy,
            crate::policy_evolution::mutation::PolicyMutationType::ModifyWeight,
            0.5,
        );
        Ok(mutation)
    }

    pub fn auto_evolve(
        &self,
        policy_type: PolicyType,
        iterations: usize,
    ) -> EvolutionResult<Policy> {
        let mut best_policy = self
            .repository
            .get_active(policy_type)
            .ok_or_else(|| EvolutionError::NotFound(format!("no policy for {policy_type}")))?;
        let mut best_eval = self.evaluator.evaluate_policy(&best_policy);

        for _ in 0..iterations {
            let mutation = self.mutation_engine.mutate_policy(
                &best_policy,
                crate::policy_evolution::mutation::PolicyMutationType::ModifyWeight,
                0.3,
            );
            let new_policy = self.mutation_engine.apply_mutation(&mutation);
            let new_eval = self.evaluator.evaluate_policy(&new_policy);
            if new_eval.score > best_eval.score {
                best_policy = new_policy;
                best_eval = new_eval;
            }
        }

        self.repository.save(best_policy.clone());
        self.repository.set_active(&best_policy);
        Ok(best_policy)
    }

    pub fn get_best_policy(&self, policy_type: PolicyType) -> Option<Policy> {
        let mut policies = self.repository.list_by_type(policy_type);
        self.evaluator.rank_policies(&mut policies);
        policies.into_iter().next()
    }

    pub fn rollback_policy(&self, policy_id: EvolutionId) -> EvolutionResult<()> {
        let mut policy = self
            .repository
            .load(policy_id)
            .ok_or_else(|| EvolutionError::NotFound(format!("policy {policy_id}")))?;
        if policy.version > 1 {
            policy.version -= 1;
            self.repository.save(policy);
        }
        Ok(())
    }
}
