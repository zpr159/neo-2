use std::collections::HashMap;

use crate::decision::{DecisionEngine, DecisionResult, DecisionOption, ObjectiveWeight};
use crate::error::ReasoningResult;
use crate::hypothesis::{Hypothesis, HypothesisEngine};
use crate::orchestrator::{
    ReasoningOrchestrator, ReasoningRequest, ReasoningResponse, SessionInfo, SessionSummary,
};
use crate::types::ReasoningConfig;

pub struct ReasoningApi {
    orchestrator: ReasoningOrchestrator,
}

impl std::fmt::Debug for ReasoningApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReasoningApi").finish()
    }
}

impl ReasoningApi {
    pub fn new(config: ReasoningConfig) -> Self {
        Self {
            orchestrator: ReasoningOrchestrator::new(config),
        }
    }

    pub fn default() -> Self {
        Self::new(ReasoningConfig::default())
    }

    pub async fn reason(&self, query: String) -> ReasoningResult<ReasoningResponse> {
        let request = ReasoningRequest::new(query);
        let session_id = self.orchestrator.start_session(request.clone()).await?;
        self.orchestrator.execute_session(session_id, request).await
    }

    pub async fn reason_with_strategy(
        &self,
        query: String,
        strategy: crate::types::ReasoningStrategy,
    ) -> ReasoningResult<ReasoningResponse> {
        let mut request = ReasoningRequest::new(query);
        request.strategy = Some(strategy);
        let session_id = self.orchestrator.start_session(request.clone()).await?;
        self.orchestrator.execute_session(session_id, request).await
    }

    pub async fn reason_with_context(
        &self,
        query: String,
        context: HashMap<String, serde_json::Value>,
    ) -> ReasoningResult<ReasoningResponse> {
        let mut request = ReasoningRequest::new(query);
        request.context = context;
        let session_id = self.orchestrator.start_session(request.clone()).await?;
        self.orchestrator.execute_session(session_id, request).await
    }

    pub async fn start_reasoning(
        &self,
        request: ReasoningRequest,
    ) -> ReasoningResult<uuid::Uuid> {
        self.orchestrator.start_session(request).await
    }

    pub async fn resume_reasoning(
        &self,
        session_id: uuid::Uuid,
        context: HashMap<String, serde_json::Value>,
    ) -> ReasoningResult<ReasoningResponse> {
        self.orchestrator.resume_session(session_id, context).await
    }

    pub async fn cancel_reasoning(
        &self,
        session_id: uuid::Uuid,
    ) -> ReasoningResult<()> {
        self.orchestrator.cancel_session(session_id).await
    }

    pub fn inspect_reasoning(
        &self,
        session_id: uuid::Uuid,
    ) -> ReasoningResult<SessionInfo> {
        self.orchestrator.inspect_session(session_id)
    }

    pub fn export_summary(
        &self,
        session_id: uuid::Uuid,
    ) -> ReasoningResult<SessionSummary> {
        self.orchestrator.export_summary(session_id)
    }

    pub fn available_strategies(&self) -> Vec<crate::types::ReasoningStrategy> {
        self.orchestrator.strategies()
    }

    pub fn analytics(&self) -> crate::analytics::ReasoningAnalyticsSnapshot {
        self.orchestrator.analytics()
    }

    pub fn cache_stats(&self) -> crate::cache::CacheStats {
        self.orchestrator.cache_stats()
    }

    pub fn decide(
        &self,
        descriptions: Vec<String>,
        weights: Option<Vec<ObjectiveWeight>>,
    ) -> ReasoningResult<DecisionResult> {
        let options: Vec<DecisionOption> = descriptions
            .into_iter()
            .map(|d| {
                DecisionOption::new(d)
                    .with_utility(0.5)
                    .with_risk(0.3)
            })
            .collect();

        let engine = DecisionEngine::new();
        engine.select_best(&options, weights.as_deref())
    }

    pub fn generate_hypotheses(
        &self,
        query: String,
        count: usize,
    ) -> Vec<Hypothesis> {
        let engine = HypothesisEngine::new().with_max_hypotheses(count);
        let context = HashMap::new();
        engine.generate_hypotheses(&query, &context, count)
    }
}
