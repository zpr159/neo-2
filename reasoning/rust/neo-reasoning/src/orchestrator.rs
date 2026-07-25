use std::collections::HashMap;
use std::time::Instant;

use dashmap::DashMap;
use parking_lot::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

use crate::analytics::{ReasoningAnalytics, ReasoningAnalyticsSnapshot};
use crate::cache::{CachedReasoningResult, ReasoningCache};
use crate::chain::{InternalReasoningState, InternalStep, StepType};
use crate::decision::{DecisionEngine, DecisionOption};
use crate::error::{ReasoningError, ReasoningResult};
use crate::explanation::ExplanationEngine;
use crate::hypothesis::HypothesisEngine;
use crate::knowledge_integration::KnowledgeIntegrator;
use crate::multi_model::MultiModelReasoner;
use crate::planning::{Goal, PlanningEngine};
use crate::reflection::{ReflectionEngine, ReflectionResult};
use crate::strategy::{StrategyContext, StrategyRegistry};
use crate::tool_reasoning::ToolReasoner;
use crate::types::{
    ReasoningConfig, ReasoningSession,
    ReasoningStrategy, SessionState,
};

#[derive(Debug, Clone)]
pub struct ReasoningRequest {
    pub query: String,
    pub strategy: Option<ReasoningStrategy>,
    pub context: HashMap<String, serde_json::Value>,
    pub max_depth: Option<u32>,
    pub timeout_ms: Option<u64>,
    pub enable_reflection: bool,
    pub enable_planning: bool,
    pub enable_hypotheses: bool,
    pub enable_decision: bool,
    pub enable_explanation: bool,
}

impl ReasoningRequest {
    pub fn new(query: String) -> Self {
        Self {
            query,
            strategy: None,
            context: HashMap::new(),
            max_depth: None,
            timeout_ms: None,
            enable_reflection: true,
            enable_planning: true,
            enable_hypotheses: false,
            enable_decision: false,
            enable_explanation: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReasoningResponse {
    pub session_id: Uuid,
    pub conclusion: String,
    pub confidence: f32,
    pub explanation: Option<String>,
    pub alternative_count: usize,
    pub reasoning_depth: usize,
    pub strategy_used: String,
    pub latency_ms: u64,
    pub cache_hit: bool,
    pub metadata: HashMap<String, serde_json::Value>,
}

pub struct ReasoningOrchestrator {
    config: ReasoningConfig,
    strategy_registry: StrategyRegistry,
    planning_engine: PlanningEngine,
    reflection_engine: ReflectionEngine,
    hypothesis_engine: RwLock<HypothesisEngine>,
    decision_engine: DecisionEngine,
    _knowledge_integrator: KnowledgeIntegrator,
    reasoning_cache: ReasoningCache,
    _tool_reasoner: RwLock<ToolReasoner>,
    _multi_model_reasoner: RwLock<MultiModelReasoner>,
    explanation_engine: ExplanationEngine,
    analytics: ReasoningAnalytics,
    sessions: DashMap<Uuid, ReasoningSession>,
    reasoning_states: DashMap<Uuid, InternalReasoningState>,
}

impl std::fmt::Debug for ReasoningOrchestrator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReasoningOrchestrator")
            .field("config", &self.config)
            .field("sessions_count", &self.sessions.len())
            .field("states_count", &self.reasoning_states.len())
            .finish()
    }
}

impl ReasoningOrchestrator {
    pub fn new(config: ReasoningConfig) -> Self {
        let cache = ReasoningCache::new(
            config.max_cache_entries,
            config.cache_ttl_secs,
        );

        Self {
            strategy_registry: StrategyRegistry::new(),
            planning_engine: PlanningEngine::new(),
            reflection_engine: ReflectionEngine::new(),
            hypothesis_engine: RwLock::new(
                HypothesisEngine::new().with_max_hypotheses(config.max_hypotheses),
            ),
            decision_engine: DecisionEngine::new(),
            _knowledge_integrator: KnowledgeIntegrator::new(),
            reasoning_cache: cache,
            _tool_reasoner: RwLock::new(ToolReasoner::new()),
            _multi_model_reasoner: RwLock::new(MultiModelReasoner::new()),
            explanation_engine: ExplanationEngine::new(),
            analytics: ReasoningAnalytics::new(),
            sessions: DashMap::new(),
            reasoning_states: DashMap::new(),
            config,
        }
    }

    pub fn default_config() -> Self {
        Self::new(ReasoningConfig::default())
    }

    pub async fn start_session(&self, request: ReasoningRequest) -> ReasoningResult<Uuid> {
        let strategy = request
            .strategy
            .unwrap_or_else(|| ReasoningStrategy::classify_query(&request.query));

        let timeout = request
            .timeout_ms
            .unwrap_or(self.config.timeout_ms);

        let mut session = ReasoningSession::new(request.query.clone(), strategy.clone(), timeout);
        session.max_depth = request
            .max_depth
            .unwrap_or(self.config.max_depth);
        session.context = request.context;

        let session_id = session.id;
        self.sessions.insert(session_id, session);
        self.reasoning_states
            .insert(session_id, InternalReasoningState::new());

        self.analytics.record_session_start();

        info!(
            session_id = %session_id,
            strategy = %strategy,
            "Reasoning session started"
        );

        Ok(session_id)
    }

    pub async fn execute_session(
        &self,
        session_id: Uuid,
        request: ReasoningRequest,
    ) -> ReasoningResult<ReasoningResponse> {
        let start = Instant::now();

        let session = self
            .sessions
            .get(&session_id)
            .ok_or_else(|| ReasoningError::SessionNotFound(session_id.to_string()))?
            .clone();

        let strategy = session.strategy.clone();
        let query = session.query.clone();

        if let Some(cached) = self.reasoning_cache.get(&query) {
            self.analytics.record_cache_hit();
            let latency = start.elapsed().as_millis() as u64;
            return Ok(self.build_response_from_cache(
                session_id,
                cached,
                latency,
                strategy.to_string(),
            ));
        }
        self.analytics.record_cache_miss();

        if let Err(e) = self
            .execute_reasoning_pipeline(
                session_id,
                &query,
                &strategy,
                &request,
            )
            .await
        {
            self.analytics.record_session_failed();
            if let Some(mut s) = self.sessions.get_mut(&session_id) {
                let _ = s.transition(SessionState::Failed);
            }
            return Err(e);
        }

        let result = self
            .finalize_session(session_id)
            .await?;

        let latency = start.elapsed().as_millis() as u64;

        self.analytics.record_session_complete(
            latency as f64,
            result.reasoning_depth as u32,
            result.confidence,
            &result.strategy_used,
        );

        Ok(result)
    }

    async fn execute_reasoning_pipeline(
        &self,
        session_id: Uuid,
        query: &str,
        strategy: &ReasoningStrategy,
        request: &ReasoningRequest,
    ) -> ReasoningResult<()> {
        if let Some(mut s) = self.sessions.get_mut(&session_id) {
            s.transition(SessionState::Planning)?;
        }

        if request.enable_planning {
            self.execute_planning_phase(session_id, query, strategy).await?;
        }

        if let Some(mut s) = self.sessions.get_mut(&session_id) {
            s.transition(SessionState::Reasoning)?;
        }

        self.execute_strategy_phase(session_id, query, strategy).await?;

        if request.enable_hypotheses {
            self.execute_hypothesis_phase(session_id, query).await?;
        }

        if self.config.enable_reflection && request.enable_reflection {
            self.execute_reflection_phase(session_id).await?;
        }

        if request.enable_decision {
            self.execute_decision_phase(session_id, query).await?;
        }

        if let Some(mut s) = self.sessions.get_mut(&session_id) {
            s.transition(SessionState::Verifying)?;
        }

        self.execute_verification_phase(session_id).await?;

        Ok(())
    }

    async fn execute_planning_phase(
        &self,
        session_id: Uuid,
        query: &str,
        strategy: &ReasoningStrategy,
    ) -> ReasoningResult<()> {
        let goal = Goal::new(query.to_string());
        let plan = self.planning_engine.create_plan(goal, strategy.clone())?;
        self.planning_engine.validate_plan(&plan)?;

        if let Some(mut state) = self.reasoning_states.get_mut(&session_id) {
            state.store_working(
                "current_plan".to_string(),
                serde_json::to_value(&plan)
                    .map_err(|e| ReasoningError::PlanningFailed(e.to_string()))?,
            );
        }

        info!(
            session_id = %session_id,
            task_count = plan.task_count(),
            "Planning phase completed"
        );

        Ok(())
    }

    async fn execute_strategy_phase(
        &self,
        session_id: Uuid,
        query: &str,
        strategy: &ReasoningStrategy,
    ) -> ReasoningResult<()> {
        let strategy_ctx = self.build_strategy_context(session_id, query);

        let executor = self
            .strategy_registry
            .get(strategy)
            .ok_or_else(|| {
                ReasoningError::StrategyNotFound(format!(
                    "strategy {:?} not registered",
                    strategy
                ))
            })?;

        let result = executor.execute(&strategy_ctx)?;

        if let Some(mut state) = self.reasoning_states.get_mut(&session_id) {
            let _chain_id = state.start_chain(strategy.clone());

            let step = InternalStep::new(
                StepType::Premise,
                query.to_string(),
                0.5,
            )
            .with_reasoning(format!("Initial query for {:?} strategy", strategy));

            if let Some(chain) = state.active_chain_mut() {
                chain.add_step(step);
            }

            for intermediate in &result.intermediate_states {
                let inf_step = InternalStep::new(
                    StepType::Inference,
                    intermediate.clone(),
                    result.confidence,
                );
                if let Some(chain) = state.active_chain_mut() {
                    chain.add_step(inf_step);
                }
            }

            let conclusion = InternalStep::new(
                StepType::Conclusion,
                result.output.clone(),
                result.confidence,
            )
            .with_reasoning(format!("Conclusion from {:?} strategy", strategy));

            if let Some(chain) = state.active_chain_mut() {
                chain.add_step(conclusion);
                chain.add_checkpoint();
            }

            state.finalize_active_chain();
            state.add_evidence(result.output);
        }

        info!(
            session_id = %session_id,
            strategy = %strategy,
            confidence = result.confidence,
            "Strategy phase completed"
        );

        Ok(())
    }

    async fn execute_hypothesis_phase(
        &self,
        session_id: Uuid,
        query: &str,
    ) -> ReasoningResult<()> {
        let context = self.get_session_context(session_id);
        let engine = self.hypothesis_engine.write();

        let hypotheses = engine.generate_hypotheses(query, &context, self.config.max_hypotheses);

        for _h in &hypotheses {
            self.analytics.record_hypothesis();
        }

        let rankings = engine.rank_hypotheses(&hypotheses);

        if let Some(mut state) = self.reasoning_states.get_mut(&session_id) {
            for h in &hypotheses {
                state.add_evidence(format!("Hypothesis: {}", h.statement));
            }
            state.store_working(
                "hypotheses".to_string(),
                serde_json::json!({
                    "count": hypotheses.len(),
                    "rankings": rankings.len(),
                }),
            );
        }

        info!(
            session_id = %session_id,
            hypothesis_count = hypotheses.len(),
            "Hypothesis phase completed"
        );

        Ok(())
    }

    async fn execute_reflection_phase(
        &self,
        session_id: Uuid,
    ) -> ReasoningResult<()> {
        let state = self
            .reasoning_states
            .get(&session_id)
            .ok_or_else(|| ReasoningError::SessionNotFound(session_id.to_string()))?;

        let context = self.get_session_context(session_id);
        let reflection = self.reflection_engine.reflect(&state, &context)?;

        self.analytics.record_reflection();

        if !reflection.passed(self.config.min_confidence) {
            warn!(
                session_id = %session_id,
                score = reflection.overall_score,
                "Reflection did not pass threshold"
            );
        }

        drop(state);

        if let Some(mut state) = self.reasoning_states.get_mut(&session_id) {
            state.store_working(
                "reflection".to_string(),
                serde_json::to_value(&reflection)
                    .unwrap_or(serde_json::json!({"error": "serialization failed"})),
            );
        }

        info!(
            session_id = %session_id,
            score = reflection.overall_score,
            consistent = reflection.is_consistent,
            "Reflection phase completed"
        );

        Ok(())
    }

    async fn execute_decision_phase(
        &self,
        session_id: Uuid,
        query: &str,
    ) -> ReasoningResult<()> {
        let state = self
            .reasoning_states
            .get(&session_id)
            .ok_or_else(|| ReasoningError::SessionNotFound(session_id.to_string()))?;

        let mut options = Vec::new();

        for chain in state.all_finalized_chains() {
            if let Some(conclusion) = chain.get_conclusion() {
                let opt = DecisionOption::new(conclusion.content.clone())
                    .with_utility(conclusion.confidence as f64)
                    .with_risk(1.0 - conclusion.confidence as f64);
                options.push(opt);
            }
        }

        if options.is_empty() {
            options.push(
                DecisionOption::new(query.to_string())
                    .with_utility(0.5)
                    .with_risk(0.5),
            );
        }

        drop(state);

        let decision = self.decision_engine.select_best(&options, None)?;

        self.analytics.record_decision();

        if let Some(mut state) = self.reasoning_states.get_mut(&session_id) {
            state.store_working(
                "decision".to_string(),
                serde_json::to_value(&decision)
                    .unwrap_or(serde_json::json!({"error": "serialization failed"})),
            );
        }

        info!(
            session_id = %session_id,
            selected = decision.selected_description,
            score = decision.composite_score,
            "Decision phase completed"
        );

        Ok(())
    }

    async fn execute_verification_phase(
        &self,
        session_id: Uuid,
    ) -> ReasoningResult<()> {
        let state = self
            .reasoning_states
            .get(&session_id)
            .ok_or_else(|| ReasoningError::SessionNotFound(session_id.to_string()))?;

        let best = state.best_chain();

        match best {
            Some(chain) => {
                let conf = chain.average_confidence();
                if conf < self.config.min_confidence {
                    return Err(ReasoningError::InconsistentResult(format!(
                        "verification failed: confidence {:.2} below threshold {:.2}",
                        conf, self.config.min_confidence
                    )));
                }
            }
            None => {
                return Err(ReasoningError::InconsistentResult(
                    "no finalized reasoning chain for verification".to_string(),
                ));
            }
        }

        if let Some(mut s) = self.sessions.get_mut(&session_id) {
            s.transition(SessionState::Completed)?;
        }

        info!(session_id = %session_id, "Verification phase completed");

        Ok(())
    }

    async fn finalize_session(
        &self,
        session_id: Uuid,
    ) -> ReasoningResult<ReasoningResponse> {
        let state = self
            .reasoning_states
            .get(&session_id)
            .ok_or_else(|| ReasoningError::SessionNotFound(session_id.to_string()))?;

        let session = self
            .sessions
            .get(&session_id)
            .ok_or_else(|| ReasoningError::SessionNotFound(session_id.to_string()))?;

        let strategy_used = session.strategy.to_string();
        let (conclusion, confidence, depth) = match state.best_chain() {
            Some(chain) => {
                let concl = chain
                    .get_conclusion()
                    .map(|s| s.content.clone())
                    .unwrap_or_default();
                (concl, chain.average_confidence(), chain.step_count())
            }
            None => ("No conclusion reached".to_string(), 0.0, 0),
        };

        let alternative_count = state.all_finalized_chains().len().saturating_sub(1);

        let explanation = if self.config.enable_reflection {
            let context = self.get_session_context(session_id);
            let reflection_data = state
                .get_working("reflection")
                .and_then(|v| {
                    serde_json::from_value::<ReflectionResult>(v.clone()).ok()
                });

            let explanation = self.explanation_engine.generate_explanation(
                &state,
                reflection_data.as_ref(),
                &context,
            )?;

            Some(self.explanation_engine.generate_human_readable(&explanation))
        } else {
            None
        };

        let cached = CachedReasoningResult {
            conclusion: conclusion.clone(),
            confidence,
            explanation: explanation.clone().unwrap_or_default(),
            strategy_used: strategy_used.clone(),
            step_count: depth,
            metadata: HashMap::new(),
        };

        drop(state);
        drop(session);

        if let Some(session_data) = self.sessions.get(&session_id) {
            self.reasoning_cache
                .store(&session_data.query, cached);
        }

        let latency = self
            .sessions
            .get(&session_id)
            .map(|s| s.elapsed_ms())
            .unwrap_or(0);

        Ok(ReasoningResponse {
            session_id,
            conclusion,
            confidence,
            explanation,
            alternative_count,
            reasoning_depth: depth,
            strategy_used,
            latency_ms: latency,
            cache_hit: false,
            metadata: HashMap::new(),
        })
    }

    fn build_response_from_cache(
        &self,
        session_id: Uuid,
        cached: CachedReasoningResult,
        latency_ms: u64,
        strategy_used: String,
    ) -> ReasoningResponse {
        ReasoningResponse {
            session_id,
            conclusion: cached.conclusion,
            confidence: cached.confidence,
            explanation: if cached.explanation.is_empty() {
                None
            } else {
                Some(cached.explanation)
            },
            alternative_count: 0,
            reasoning_depth: cached.step_count,
            strategy_used,
            latency_ms,
            cache_hit: true,
            metadata: HashMap::new(),
        }
    }

    fn build_strategy_context(
        &self,
        session_id: Uuid,
        query: &str,
    ) -> StrategyContext {
        let mut ctx = StrategyContext::new(query.to_string());

        if let Some(session) = self.sessions.get(&session_id) {
            for (key, value) in &session.context {
                ctx.context_data.insert(key.clone(), value.clone());
                if let Some(s) = value.as_str() {
                    ctx.available_facts.push(s.to_string());
                }
            }
        }

        if let Some(state) = self.reasoning_states.get(&session_id) {
            for evidence in &state.accumulated_evidence {
                ctx.available_facts.push(evidence.clone());
            }
        }

        ctx.max_depth = self.config.max_depth;
        ctx
    }

    fn get_session_context(
        &self,
        session_id: Uuid,
    ) -> HashMap<String, serde_json::Value> {
        self.sessions
            .get(&session_id)
            .map(|s| s.context.clone())
            .unwrap_or_default()
    }

    pub async fn resume_session(
        &self,
        session_id: Uuid,
        additional_context: HashMap<String, serde_json::Value>,
    ) -> ReasoningResult<ReasoningResponse> {
        let session = self
            .sessions
            .get(&session_id)
            .ok_or_else(|| ReasoningError::SessionNotFound(session_id.to_string()))?
            .clone();

        if session.state == SessionState::Completed {
            return Err(ReasoningError::SessionCompleted(
                session_id.to_string(),
            ));
        }

        if session.state == SessionState::Cancelled {
            return Err(ReasoningError::SessionCancelled(
                session_id.to_string(),
            ));
        }

        drop(session);

        for (key, value) in additional_context {
            if let Some(mut session) = self.sessions.get_mut(&session_id) {
                session.context.insert(key, value);
            }
        }

        let request = ReasoningRequest {
            query: self
                .sessions
                .get(&session_id)
                .map(|s| s.query.clone())
                .unwrap_or_default(),
            strategy: Some(
                self.sessions
                    .get(&session_id)
                    .map(|s| s.strategy.clone())
                    .unwrap_or(ReasoningStrategy::ChainOfThought),
            ),
            context: self.get_session_context(session_id),
            max_depth: None,
            timeout_ms: None,
            enable_reflection: true,
            enable_planning: false,
            enable_hypotheses: false,
            enable_decision: false,
            enable_explanation: true,
        };

        self.execute_session(session_id, request).await
    }

    pub async fn cancel_session(
        &self,
        session_id: Uuid,
    ) -> ReasoningResult<()> {
        let mut session = self
            .sessions
            .get_mut(&session_id)
            .ok_or_else(|| ReasoningError::SessionNotFound(session_id.to_string()))?;

        session.transition(SessionState::Cancelled)?;
        self.analytics.record_session_cancelled();

        info!(session_id = %session_id, "Session cancelled");

        Ok(())
    }

    pub fn inspect_session(
        &self,
        session_id: Uuid,
    ) -> ReasoningResult<SessionInfo> {
        let session = self
            .sessions
            .get(&session_id)
            .ok_or_else(|| ReasoningError::SessionNotFound(session_id.to_string()))?;

        let state = self.reasoning_states.get(&session_id);

        let chain_count = state.as_ref().map_or(0, |s| s.chains.len());
        let finalized_count = state
            .as_ref()
            .map_or(0, |s| s.all_finalized_chains().len());

        Ok(SessionInfo {
            id: session.id,
            query: session.query.clone(),
            state: session.state,
            strategy: session.strategy.clone(),
            created_at: session.created_at,
            elapsed_ms: session.elapsed_ms(),
            is_expired: session.is_expired(),
            chain_count,
            finalized_chain_count: finalized_count,
            phase_transitions: session.phase_history.len(),
        })
    }

    pub fn export_summary(
        &self,
        session_id: Uuid,
    ) -> ReasoningResult<SessionSummary> {
        let info = self.inspect_session(session_id)?;

        let state = self
            .reasoning_states
            .get(&session_id)
            .ok_or_else(|| ReasoningError::SessionNotFound(session_id.to_string()))?;

        let best = state.best_chain();
        let conclusion = best
            .and_then(|c| c.get_conclusion())
            .map(|s| s.content.clone())
            .unwrap_or_default();

        let confidence = best.map_or(0.0, |c| c.average_confidence());

        let chain_summaries: Vec<ChainSummary> = state
            .all_finalized_chains()
            .iter()
            .map(|c| ChainSummary {
                strategy: c.strategy.to_string(),
                step_count: c.step_count(),
                average_confidence: c.average_confidence(),
                has_conclusion: c.get_conclusion().is_some(),
            })
            .collect();

        Ok(SessionSummary {
            info,
            conclusion,
            confidence,
            chain_summaries,
            evidence_count: state.accumulated_evidence.len(),
            rejected_paths_count: state.rejected_paths.len(),
        })
    }

    pub fn analytics(&self) -> ReasoningAnalyticsSnapshot {
        self.analytics.snapshot()
    }

    pub fn cache_stats(&self) -> crate::cache::CacheStats {
        self.reasoning_cache.stats()
    }

    pub fn cleanup_cache(&self) -> usize {
        self.reasoning_cache.cleanup_expired()
    }

    pub fn strategies(&self) -> Vec<ReasoningStrategy> {
        self.strategy_registry.strategies()
    }
}

impl Default for ReasoningOrchestrator {
    fn default() -> Self {
        Self::default_config()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionInfo {
    pub id: Uuid,
    pub query: String,
    pub state: SessionState,
    pub strategy: ReasoningStrategy,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub elapsed_ms: u64,
    pub is_expired: bool,
    pub chain_count: usize,
    pub finalized_chain_count: usize,
    pub phase_transitions: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChainSummary {
    pub strategy: String,
    pub step_count: usize,
    pub average_confidence: f32,
    pub has_conclusion: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionSummary {
    pub info: SessionInfo,
    pub conclusion: String,
    pub confidence: f32,
    pub chain_summaries: Vec<ChainSummary>,
    pub evidence_count: usize,
    pub rejected_paths_count: usize,
}
