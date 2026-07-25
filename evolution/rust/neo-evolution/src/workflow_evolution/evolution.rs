use std::collections::HashMap;
use std::sync::Arc;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::config::EvolutionConfiguration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowAnalysisResult {
    pub workflow_id: String,
    pub redundant_steps: Vec<String>,
    pub unnecessary_synchronization: Vec<String>,
    pub better_ordering: Option<Vec<String>>,
    pub reusable_fragments: Vec<String>,
    pub efficiency_score: f64,
}

pub struct WorkflowEvolution {
    analyses: DashMap<String, WorkflowAnalysisResult>,
    config: EvolutionConfiguration,
}

impl WorkflowEvolution {
    pub fn new(config: EvolutionConfiguration) -> Arc<Self> {
        Arc::new(Self {
            analyses: DashMap::new(),
            config,
        })
    }

    pub fn analyze_workflow(
        &self,
        workflow_id: impl Into<String>,
        steps: &[String],
    ) -> WorkflowAnalysisResult {
        let wf_id = workflow_id.into();
        let mut redundant = Vec::new();
        let mut sync = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for step in steps {
            if !seen.insert(step.clone()) {
                redundant.push(step.clone());
            }
            if step.contains("sync") || step.contains("wait") {
                sync.push(step.clone());
            }
        }

        let mut ordering = steps.to_vec();
        ordering.sort();
        let better_ordering = if ordering != steps.to_vec() {
            Some(ordering)
        } else {
            None
        };

        let total = steps.len() as f64;
        let issues = redundant.len() + sync.len() as usize;
        let efficiency = if total > 0.0 {
            1.0 - (issues as f64 / total)
        } else {
            1.0
        };

        let result = WorkflowAnalysisResult {
            workflow_id: wf_id.clone(),
            redundant_steps: redundant,
            unnecessary_synchronization: sync,
            better_ordering,
            reusable_fragments: Vec::new(),
            efficiency_score: efficiency,
        };

        self.analyses.insert(wf_id, result.clone());
        result
    }

    pub fn discover_redundancies(&self, workflow_id: &str) -> Vec<String> {
        self.analyses
            .get(workflow_id)
            .map(|r| r.redundant_steps.clone())
            .unwrap_or_default()
    }

    pub fn find_reusable_fragments(&self) -> Vec<String> {
        let mut fragments = Vec::new();
        for entry in self.analyses.iter() {
            fragments.extend(entry.reusable_fragments.clone());
        }
        fragments
    }

    pub fn suggest_ordering(&self, workflow_id: &str) -> Option<Vec<String>> {
        self.analyses
            .get(workflow_id)
            .and_then(|r| r.better_ordering.clone())
    }

    pub fn get_all_analyses(&self) -> Vec<WorkflowAnalysisResult> {
        self.analyses.iter().map(|r| r.value().clone()).collect()
    }
}
