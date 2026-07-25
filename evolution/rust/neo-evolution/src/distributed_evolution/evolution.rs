use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::config::EvolutionConfiguration;
use crate::types::EvolutionId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedResult {
    pub node_id: String,
    pub local_results: HashMap<String, f64>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterExperiment {
    pub experiment_id: EvolutionId,
    pub participating_nodes: Vec<String>,
    pub synchronized: bool,
    pub created_at: DateTime<Utc>,
}

pub struct DistributedEvolution {
    cluster_experiments: DashMap<EvolutionId, ClusterExperiment>,
    federated_results: DashMap<EvolutionId, Vec<FederatedResult>>,
    #[allow(dead_code)]
    config: EvolutionConfiguration,
}

impl DistributedEvolution {
    pub fn new(config: EvolutionConfiguration) -> Arc<Self> {
        Arc::new(Self {
            cluster_experiments: DashMap::new(),
            federated_results: DashMap::new(),
            config,
        })
    }

    pub fn create_cluster_experiment(
        &self,
        experiment_id: EvolutionId,
        nodes: Vec<String>,
    ) -> ClusterExperiment {
        let exp = ClusterExperiment {
            experiment_id,
            participating_nodes: nodes,
            synchronized: true,
            created_at: Utc::now(),
        };
        self.cluster_experiments.insert(experiment_id, exp.clone());
        exp
    }

    pub fn synchronize_nodes(&self, experiment_id: EvolutionId) -> bool {
        self.cluster_experiments
            .get(&experiment_id)
            .map_or(false, |e| e.synchronized)
    }

    pub fn collect_results(&self, experiment_id: EvolutionId, result: FederatedResult) {
        self.federated_results
            .entry(experiment_id)
            .or_default()
            .push(result);
    }

    pub fn federated_aggregate(&self, experiment_id: EvolutionId) -> HashMap<String, f64> {
        let results = self
            .federated_results
            .get(&experiment_id)
            .map(|r| r.value().clone())
            .unwrap_or_default();

        if results.is_empty() {
            return HashMap::new();
        }

        let mut aggregated: HashMap<String, Vec<f64>> = HashMap::new();
        for result in &results {
            for (k, v) in &result.local_results {
                aggregated.entry(k.clone()).or_default().push(*v);
            }
        }

        aggregated
            .into_iter()
            .map(|(k, vals)| {
                let avg = vals.iter().sum::<f64>() / vals.len() as f64;
                (k, avg)
            })
            .collect()
    }

    pub fn coordinated_rollback(&self, experiment_id: EvolutionId) -> bool {
        self.cluster_experiments.remove(&experiment_id).is_some()
    }

    pub fn get_active_experiments(&self) -> Vec<ClusterExperiment> {
        self.cluster_experiments
            .iter()
            .map(|e| e.value().clone())
            .collect()
    }
}
