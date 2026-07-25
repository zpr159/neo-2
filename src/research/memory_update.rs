use super::api::{Finding, MemoryUpdateProposal};
use super::config::MemoryUpdateConfig;
use super::error::{ResearchError, ResearchResult};

/// Manages validated memory updates proposed by research.
pub struct MemoryUpdateManager {
    config: MemoryUpdateConfig,
}

impl MemoryUpdateManager {
    pub fn new(config: MemoryUpdateConfig) -> Self {
        Self { config }
    }

    /// Check if a memory update is allowed.
    pub fn can_update(&self, importance: f32) -> bool {
        self.config.enabled && importance >= self.config.importance_threshold
    }

    /// Filter proposals to those that pass governance.
    pub fn filter_approved(
        &self,
        proposals: Vec<MemoryUpdateProposal>,
    ) -> Vec<MemoryUpdateProposal> {
        if !self.config.enabled {
            return Vec::new();
        }

        proposals
            .into_iter()
            .filter(|p| p.importance >= self.config.importance_threshold)
            .take(self.config.max_memory_items_per_task)
            .collect()
    }

    /// Create a memory update proposal from findings.
    pub fn propose_update(
        &self,
        findings: &[Finding],
        objective: &str,
    ) -> ResearchResult<MemoryUpdateProposal> {
        if findings.is_empty() {
            return Err(ResearchError::MemoryUpdateFailed(
                "no findings to propose as memory".to_string(),
            ));
        }

        let content: String = findings
            .iter()
            .map(|f| f.statement.as_str())
            .collect::<Vec<_>>()
            .join("; ");

        let avg_confidence: f32 =
            findings.iter().map(|f| f.confidence).sum::<f32>() / findings.len() as f32;

        let importance = avg_confidence;

        if !self.can_update(importance) {
            return Err(ResearchError::GovernanceRejected(format!(
                "importance {} below threshold {}",
                importance, self.config.importance_threshold
            )));
        }

        let mut context = std::collections::HashMap::new();
        context.insert("research_objective".to_string(), objective.to_string());
        context.insert("source_count".to_string(), findings.len().to_string());
        context.insert(
            "average_confidence".to_string(),
            format!("{:.2}", avg_confidence),
        );

        Ok(MemoryUpdateProposal {
            content,
            memory_type: "semantic".to_string(),
            importance,
            context,
            source_citations: Vec::new(),
        })
    }
}
