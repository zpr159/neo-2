use serde::{Deserialize, Serialize};

use crate::conversation::evidence::Evidence;
use crate::conversation::memory_bridge::MemoryConversationBridge;
use crate::conversation::knowledge_bridge::KnowledgeConversationBridge;
use crate::conversation::world_model_bridge::WorldModelConversationBridge;
use crate::conversation::planning_bridge::PlanningConversationBridge;
use crate::conversation::reasoning_bridge::ReasoningConversationBridge;
use crate::conversation::executive_bridge::ExecutiveConversationBridge;
use crate::conversation::agent_bridge::AgentConversationBridge;
use crate::conversation::workflow_bridge::WorkflowConversationBridge;
use crate::conversation::error::ConversationResult;
use crate::conversation::types::ConversationContext;
use crate::conversation::memory_bridge::MemoryQuery;
use crate::conversation::config::RankingConfig;

/// The unified cognitive context produced by the RetrievalCoordinator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveContext {
    pub ranked_evidence: Vec<super::context_ranker::RankedEvidence>,
    pub unified: UnifiedContextForSerialization,
    pub executive_context: Option<ExecutiveContextSnapshot>,
    pub planning_context: Option<PlanningContextSnapshot>,
    pub reasoning_context: Option<ReasoningContextSnapshot>,
    pub memory_context: Option<MemoryContextSnapshot>,
    pub knowledge_context: Option<KnowledgeContextSnapshot>,
    pub world_model_context: Option<WorldModelContextSnapshot>,
    pub agent_context: Option<AgentContextSnapshot>,
    pub workflow_context: Option<WorkflowContextSnapshot>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UnifiedContextForSerialization {
    pub evidence_count: usize,
    pub average_confidence: f32,
    pub source_coverage: std::collections::HashMap<String, usize>,
    pub contradictions_detected: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutiveContextSnapshot {
    pub intent: crate::conversation::types::Intent,
    pub urgency: crate::conversation::types::Urgency,
    pub classification: crate::conversation::types::RequestClassification,
    pub reasoning_depth: crate::conversation::types::ReasoningDepth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningContextSnapshot {
    pub subtask_count: usize,
    pub estimated_cost: f64,
    pub clarification_needed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningContextSnapshot {
    pub conclusion: Option<String>,
    pub confidence: f32,
    pub contradictions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryContextSnapshot {
    pub total_retrieved: usize,
    pub average_confidence: f32,
    pub memory_types_used: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeContextSnapshot {
    pub entities_found: usize,
    pub facts_found: usize,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldModelContextSnapshot {
    pub entities_found: usize,
    pub events_found: usize,
    pub predictions_found: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContextSnapshot {
    pub agents_found: usize,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowContextSnapshot {
    pub workflows_found: usize,
    pub workflow_names: Vec<String>,
}

/// Collects information from all cognitive sources, normalizes, deduplicates,
/// scores relevance, merges contexts, and produces a unified cognitive context.
pub struct RetrievalCoordinator {
    ranking_config: RankingConfig,
}

impl RetrievalCoordinator {
    pub fn new(ranking_config: RankingConfig) -> Self {
        Self { ranking_config }
    }

    /// Retrieve and merge context from all available cognitive sources.
    pub async fn retrieve(
        &self,
        context: &ConversationContext,
        objective: &str,
        memory: &dyn MemoryConversationBridge,
        knowledge: &dyn KnowledgeConversationBridge,
        world_model: &dyn WorldModelConversationBridge,
        _planning: &dyn PlanningConversationBridge,
        _reasoning: &dyn ReasoningConversationBridge,
        _executive: &dyn ExecutiveConversationBridge,
        _agents: &dyn AgentConversationBridge,
        _workflows: &dyn WorkflowConversationBridge,
    ) -> ConversationResult<CognitiveContext> {
        let mut all_evidence: Vec<Evidence> = Vec::new();
        let executive_snapshot = None;
        let planning_snapshot = None;
        let reasoning_snapshot = None;
        let mut memory_snapshot = None;
        let mut knowledge_snapshot = None;
        let mut world_snapshot = None;
        let agent_snapshot = None;
        let workflow_snapshot = None;

        // Memory retrieval
        let memory_query = MemoryQuery {
            text: objective.to_string(),
            limit: 20,
            confidence_threshold: 0.3,
            ..Default::default()
        };
        if let Ok(result) = memory.retrieve(context, &memory_query).await {
            memory_snapshot = Some(MemoryContextSnapshot {
                total_retrieved: result.total_retrieved,
                average_confidence: result.average_confidence,
                memory_types_used: result
                    .memory_types_used
                    .iter()
                    .map(|t| format!("{:?}", t))
                    .collect(),
            });
            all_evidence.extend(result.evidence);
        }

        // Knowledge graph retrieval
        if let Ok(result) = knowledge.retrieve_evidence(context, objective, 10).await {
            knowledge_snapshot = Some(KnowledgeContextSnapshot {
                entities_found: result.len(),
                facts_found: result.len(),
                confidence: result.iter().map(|e| e.confidence).sum::<f32>()
                    / result.len().max(1) as f32,
            });
            all_evidence.extend(result);
        }

        // World model retrieval
        if let Ok(result) = world_model.query_evidence(context, objective).await {
            world_snapshot = Some(WorldModelContextSnapshot {
                entities_found: result.len(),
                events_found: 0,
                predictions_found: 0,
            });
            all_evidence.extend(result);
        }

        // Merge and deduplicate
        let mut unified = crate::conversation::context_merger::ContextMerger::new()
            .merge(all_evidence);
        crate::conversation::context_merger::ContextMerger::detect_contradictions(&mut unified);
        crate::conversation::context_merger::ContextMerger::sort_deterministic(&mut unified);

        // Rank
        let keywords: Vec<String> = objective
            .split_whitespace()
            .map(|w| w.to_lowercase())
            .filter(|w| w.len() > 3)
            .collect();
        let ranker = crate::conversation::context_ranker::ContextRanker::new(self.ranking_config.clone());
        let ranked = ranker.rank(unified.evidence.clone(), &keywords, &objective);

        let overall_confidence = if ranked.is_empty() {
            0.0
        } else {
            ranked.iter().map(|r| r.final_score).sum::<f32>() / ranked.len() as f32
        };

        Ok(CognitiveContext {
            ranked_evidence: ranked,
            unified: UnifiedContextForSerialization {
                evidence_count: unified.evidence.len(),
                average_confidence: unified.average_confidence,
                source_coverage: unified
                    .source_coverage
                    .iter()
                    .map(|(k, v)| (format!("{:?}", k), *v))
                    .collect(),
                contradictions_detected: unified.contradictions_detected,
            },
            executive_context: executive_snapshot,
            planning_context: planning_snapshot,
            reasoning_context: reasoning_snapshot,
            memory_context: memory_snapshot,
            knowledge_context: knowledge_snapshot,
            world_model_context: world_snapshot,
            agent_context: agent_snapshot,
            workflow_context: workflow_snapshot,
            confidence: overall_confidence,
        })
    }
}
