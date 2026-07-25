use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ReasoningResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievedContext {
    pub id: Uuid,
    pub source: ContextSource,
    pub content: String,
    pub relevance_score: f32,
    pub confidence: f32,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContextSource {
    Memory,
    KnowledgeGraph,
    InferenceCache,
    SessionHistory,
    External,
}

impl std::fmt::Display for ContextSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Memory => write!(f, "memory"),
            Self::KnowledgeGraph => write!(f, "knowledge_graph"),
            Self::InferenceCache => write!(f, "inference_cache"),
            Self::SessionHistory => write!(f, "session_history"),
            Self::External => write!(f, "external"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedContext {
    pub context: RetrievedContext,
    pub rank: usize,
    pub merged_score: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IntegratedContext {
    pub contexts: Vec<RankedContext>,
    pub merged_knowledge: Vec<String>,
    pub deduplication_count: usize,
    pub total_retrieved: usize,
}

pub struct KnowledgeIntegrator {
    max_contexts: usize,
    redundancy_threshold: f32,
    rank_weights: ContextRankWeights,
}

impl std::fmt::Debug for KnowledgeIntegrator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KnowledgeIntegrator")
            .field("max_contexts", &self.max_contexts)
            .field("redundancy_threshold", &self.redundancy_threshold)
            .field("rank_weights", &self.rank_weights)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct ContextRankWeights {
    pub relevance: f32,
    pub recency: f32,
    pub confidence: f32,
}

impl Default for ContextRankWeights {
    fn default() -> Self {
        Self {
            relevance: 0.5,
            recency: 0.2,
            confidence: 0.3,
        }
    }
}

impl KnowledgeIntegrator {
    pub fn new() -> Self {
        Self {
            max_contexts: 20,
            redundancy_threshold: 0.85,
            rank_weights: ContextRankWeights::default(),
        }
    }

    pub fn with_max_contexts(mut self, max: usize) -> Self {
        self.max_contexts = max;
        self
    }

    pub fn integrate(
        &self,
        memory_contexts: Vec<RetrievedContext>,
        kg_contexts: Vec<RetrievedContext>,
        inference_contexts: Vec<RetrievedContext>,
        session_contexts: Vec<RetrievedContext>,
    ) -> ReasoningResult<IntegratedContext> {
        let mut all = Vec::new();
        all.extend(memory_contexts);
        all.extend(kg_contexts);
        all.extend(inference_contexts);
        all.extend(session_contexts);

        let total_retrieved = all.len();

        let deduplicated = self.remove_redundancy(all);
        let dedup_count = total_retrieved - deduplicated.len();

        let ranked = self.rank_contexts(deduplicated);

        let merged_knowledge: Vec<String> = ranked
            .iter()
            .map(|rc| rc.content.clone())
            .collect();

        let contexts: Vec<RankedContext> = ranked
            .into_iter()
            .enumerate()
            .map(|(i, rc)| RankedContext {
                context: rc,
                rank: i + 1,
                merged_score: 0.0,
            })
            .collect();

        Ok(IntegratedContext {
            contexts,
            merged_knowledge,
            deduplication_count: dedup_count,
            total_retrieved,
        })
    }

    fn remove_redundancy(&self, contexts: Vec<RetrievedContext>) -> Vec<RetrievedContext> {
        let mut kept = Vec::new();

        for ctx in contexts {
            let is_redundant = kept.iter().any(|existing: &RetrievedContext| {
                self.text_similarity(&existing.content, &ctx.content) > self.redundancy_threshold
            });

            if !is_redundant {
                kept.push(ctx);
            }

            if kept.len() >= self.max_contexts {
                break;
            }
        }

        kept
    }

    fn text_similarity(&self, a: &str, b: &str) -> f32 {
        if a == b {
            return 1.0;
        }

        let words_a: std::collections::HashSet<&str> = a.split_whitespace().collect();
        let words_b: std::collections::HashSet<&str> = b.split_whitespace().collect();

        if words_a.is_empty() || words_b.is_empty() {
            return 0.0;
        }

        let intersection = words_a.intersection(&words_b).count();
        let union = words_a.len() + words_b.len() - intersection;

        if union == 0 {
            return 0.0;
        }

        intersection as f32 / union as f32
    }

    fn rank_contexts(&self, contexts: Vec<RetrievedContext>) -> Vec<RetrievedContext> {
        let now = chrono::Utc::now();

        let mut scored: Vec<(RetrievedContext, f32)> = contexts
            .into_iter()
            .map(|ctx| {
                let age_hours = now
                    .signed_duration_since(ctx.timestamp)
                    .num_hours()
                    .max(0) as f32;
                let recency = 1.0 / (1.0 + age_hours * 0.01);

                let score = self.rank_weights.relevance * ctx.relevance_score
                    + self.rank_weights.recency * recency
                    + self.rank_weights.confidence * ctx.confidence;

                (ctx, score)
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        scored.into_iter().map(|(ctx, _)| ctx).collect()
    }

    pub fn retrieve_from_memory(
        &self,
        query: &str,
        _memory_manager: &dyn std::any::Any,
    ) -> ReasoningResult<Vec<RetrievedContext>> {
        let _ = query;
        Ok(Vec::new())
    }

    pub fn retrieve_from_knowledge_graph(
        &self,
        query: &str,
        _kg: &dyn std::any::Any,
    ) -> ReasoningResult<Vec<RetrievedContext>> {
        let _ = query;
        Ok(Vec::new())
    }

    pub fn build_reasoning_context(
        &self,
        query: &str,
        integrated: &IntegratedContext,
    ) -> String {
        let mut context_parts = Vec::new();

        context_parts.push(format!("Query: {query}"));
        context_parts.push("Retrieved context:".to_string());

        for rc in &integrated.contexts {
            context_parts.push(format!(
                "[{}:{}] {}",
                rc.context.source,
                rc.rank,
                rc.context.content
            ));
        }

        if integrated.deduplication_count > 0 {
            context_parts.push(format!(
                "({} redundant items removed)",
                integrated.deduplication_count
            ));
        }

        context_parts.join("\n")
    }
}

impl Default for KnowledgeIntegrator {
    fn default() -> Self {
        Self::new()
    }
}
