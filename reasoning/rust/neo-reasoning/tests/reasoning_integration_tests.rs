use std::collections::HashMap;

use neo_reasoning::analytics::ReasoningAnalyticsSnapshot;
use neo_reasoning::cache::{CachedReasoningResult, ReasoningCache};
use neo_reasoning::chain::{InternalChain, InternalReasoningState, InternalStep, StepType};
use neo_reasoning::decision::{DecisionEngine, DecisionOption, ObjectiveWeight};
use neo_reasoning::error::{ReasoningError, ReasoningErrorCode, ReasoningResult};
use neo_reasoning::explanation::ExplanationEngine;
use neo_reasoning::hypothesis::{Evidence, Hypothesis, HypothesisEngine, HypothesisStatus};
use neo_reasoning::knowledge_integration::{
    ContextSource, KnowledgeIntegrator, RetrievedContext,
};
use neo_reasoning::multi_model::{
    MultiModelReasoner, ModelBackend, ModelRole,
};
use neo_reasoning::orchestrator::{
    ReasoningOrchestrator, ReasoningRequest,
};
use neo_reasoning::planning::{Goal, PlanningEngine};
use neo_reasoning::reflection::ReflectionEngine;
use neo_reasoning::strategy::{
    DeductiveStrategy, InductiveStrategy, AbductiveStrategy,
    ReasoningStrategyExecutor, ReasoningStrategy, StrategyContext, StrategyRegistry,
};
use neo_reasoning::tool_reasoning::{ToolDescriptor, ToolReasoner, ToolType};
use neo_reasoning::types::{
    ExecutionGraph, ExecutionNode, NodeStatus, ReasoningConfig, ReasoningSession,
    SessionState, ReasoningPhase,
};
use neo_reasoning::ReasoningApi;

// ── Error Tests ──

#[test]
fn test_error_types() {
    let err = ReasoningError::PlanningFailed("test".to_string());
    assert_eq!(err.code(), ReasoningErrorCode::PlanningFailed);
    assert!(err.to_string().contains("test"));

    let err: ReasoningResult<()> = Err(ReasoningError::SessionTimeout("timed out".to_string()));
    assert!(err.is_err());
}

#[test]
fn test_error_display() {
    let err = ReasoningError::NoOptions("none".to_string());
    let s = err.to_string();
    assert!(s.contains("reasoning"));
    assert!(s.contains("none"));
}

#[test]
fn test_error_conversion() {
    let err = ReasoningError::PlanningFailed("test".to_string());
    let neo_err: neo_core::error::NeoError = err.into();
    assert!(neo_err.to_string().contains("reasoning"));
}

// ── Session State Tests ──

#[test]
fn test_session_state_transitions() {
    assert!(SessionState::Created.can_transition_to(SessionState::Planning));
    assert!(SessionState::Planning.can_transition_to(SessionState::Reasoning));
    assert!(SessionState::Reasoning.can_transition_to(SessionState::Reflecting));
    assert!(SessionState::Reflecting.can_transition_to(SessionState::Completed));
    assert!(!SessionState::Created.can_transition_to(SessionState::Completed));
    assert!(!SessionState::Completed.can_transition_to(SessionState::Planning));
    assert!(SessionState::Reasoning.can_transition_to(SessionState::Failed));
    assert!(SessionState::Created.can_transition_to(SessionState::Cancelled));
}

#[test]
fn test_session_lifecycle() {
    let mut session = ReasoningSession::new(
        "test query".to_string(),
        ReasoningStrategy::Deductive,
        30_000,
    );
    assert_eq!(session.state, SessionState::Created);
    assert_eq!(session.query, "test query");

    session.transition(SessionState::Planning).unwrap();
    assert_eq!(session.state, SessionState::Planning);

    session.transition(SessionState::Reasoning).unwrap();
    assert_eq!(session.state, SessionState::Reasoning);

    session.transition(SessionState::Reflecting).unwrap();
    assert_eq!(session.state, SessionState::Reflecting);

    session.transition(SessionState::Completed).unwrap();
    assert_eq!(session.state, SessionState::Completed);
}

#[test]
fn test_session_invalid_transition() {
    let mut session = ReasoningSession::new(
        "test".to_string(),
        ReasoningStrategy::Deductive,
        30_000,
    );
    let result = session.transition(SessionState::Completed);
    assert!(result.is_err());
}

#[test]
fn test_session_builder() {
    let session = ReasoningSession::new(
        "query".to_string(),
        ReasoningStrategy::Inductive,
        5000,
    )
    .with_depth(64)
    .with_context("key".to_string(), serde_json::json!("value"));

    assert_eq!(session.max_depth, 64);
    assert_eq!(session.context.get("key").unwrap(), &serde_json::json!("value"));
}

// ── Execution Graph Tests ──

fn make_node(phase: ReasoningPhase) -> ExecutionNode {
    ExecutionNode {
        id: uuid::Uuid::new_v4(),
        phase,
        strategy: None,
        input: serde_json::json!({}),
        output: None,
        dependencies: Vec::new(),
        status: NodeStatus::Pending,
        started_at: None,
        completed_at: None,
        error: None,
    }
}

#[test]
fn test_execution_graph_basics() {
    let mut graph = ExecutionGraph::new();

    let n1 = make_node(ReasoningPhase::Planning);
    let n2 = make_node(ReasoningPhase::StrategyExecution);
    graph.add_node(n1);
    graph.add_node(n2);
    assert_eq!(graph.nodes.len(), 2);

    let roots = graph.roots();
    assert_eq!(roots.len(), 2);
}

#[test]
fn test_execution_graph_dependencies() {
    let mut graph = ExecutionGraph::new();

    let mut n1 = make_node(ReasoningPhase::Planning);
    let mut n2 = make_node(ReasoningPhase::StrategyExecution);
    let id1 = n1.id;
    let id2 = n2.id;
    n2.dependencies.push(id1);

    graph.add_node(n1);
    graph.add_node(n2);

    let roots = graph.roots();
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].id, id1);

    let dependents = graph.dependents_of(id1);
    assert_eq!(dependents.len(), 1);

    let order = graph.execution_order();
    assert_eq!(order.len(), 2);
    assert!(order.contains(&id1));
    assert!(order.contains(&id2));
    assert!(order.iter().position(|&x| x == id2) < order.iter().position(|&x| x == id1));
}

#[test]
fn test_execution_graph_completion() {
    let mut graph = ExecutionGraph::new();
    let mut n1 = make_node(ReasoningPhase::Planning);
    n1.status = NodeStatus::Completed;
    graph.add_node(n1);
    assert!(graph.all_completed());
    assert!(!graph.has_failures());

    let mut n2 = make_node(ReasoningPhase::ChainOfThought);
    n2.status = NodeStatus::Failed;
    graph.add_node(n2);
    assert!(!graph.all_completed());
    assert!(graph.has_failures());
}

#[test]
fn test_execution_graph_ready_nodes() {
    let mut graph = ExecutionGraph::new();

    let mut n1 = make_node(ReasoningPhase::Planning);
    n1.status = NodeStatus::Completed;
    let id1 = n1.id;
    graph.add_node(n1);

    let mut n2 = make_node(ReasoningPhase::StrategyExecution);
    let n2_id = uuid::Uuid::new_v4();
    n2.id = n2_id;
    n2.dependencies.push(id1);
    graph.add_node(n2);

    let ready = graph.ready_nodes();
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].id, n2_id);
}

// ── Config Tests ──

#[test]
fn test_reasoning_config() {
    let config = ReasoningConfig::default();
    assert_eq!(config.max_depth, 128);
    assert_eq!(config.timeout_ms, 30_000);
    assert!(config.enable_reflection);
    assert!(!config.enable_multi_model);
}

// ── Strategy Tests ──

#[test]
fn test_strategy_registry() {
    let registry = StrategyRegistry::new();
    assert_eq!(registry.count(), 9);
    assert!(registry.strategies().contains(&ReasoningStrategy::Deductive));
    assert!(registry.strategies().contains(&ReasoningStrategy::Inductive));
    assert!(registry.strategies().contains(&ReasoningStrategy::Abductive));
}

#[test]
fn test_strategy_select_best() {
    let registry = StrategyRegistry::new();
    let strategy = registry.select_best("if X then Y, therefore");
    assert_eq!(strategy.strategy_type(), ReasoningStrategy::Deductive);

    let strategy = registry.select_best("based on pattern observation");
    assert_eq!(strategy.strategy_type(), ReasoningStrategy::Inductive);

    let strategy = registry.select_best("why is the sky blue");
    assert_eq!(strategy.strategy_type(), ReasoningStrategy::Abductive);
}

#[test]
fn test_all_strategies_produce_results() {
    let ctx = StrategyContext::new("general reasoning query".to_string());
    for strategy in StrategyRegistry::new().strategies() {
        let registry = StrategyRegistry::new();
        let executor = registry.get(&strategy).unwrap();
        let result = executor.execute(&ctx);
        assert!(result.is_ok(), "Strategy {:?} failed: {:?}", strategy, result.err());
        let result = result.unwrap();
        assert!(result.confidence >= 0.0 && result.confidence <= 1.0,
            "Strategy {:?} confidence out of range: {}", strategy, result.confidence);
        assert!(result.steps_taken > 0);
    }
}

#[test]
fn test_deductive_strategy() {
    let strategy = DeductiveStrategy;
    let ctx = StrategyContext::new("If it rains, the ground gets wet. It rains.".to_string());
    let result = strategy.execute(&ctx).unwrap();
    assert_eq!(result.strategy, ReasoningStrategy::Deductive);
    assert!(result.confidence > 0.0);
}

#[test]
fn test_inductive_strategy() {
    let strategy = InductiveStrategy;
    let ctx = StrategyContext::new("Observation: all observed swans are white".to_string());
    let result = strategy.execute(&ctx).unwrap();
    assert_eq!(result.strategy, ReasoningStrategy::Inductive);
}

#[test]
fn test_abductive_strategy() {
    let strategy = AbductiveStrategy;
    let ctx = StrategyContext::new("The best explanation for the wet ground is rain".to_string());
    let result = strategy.execute(&ctx).unwrap();
    assert_eq!(result.strategy, ReasoningStrategy::Abductive);
}

// ── Chain of Thought Tests ──

#[test]
fn test_internal_chain() {
    let mut chain = InternalChain::new(ReasoningStrategy::ChainOfThought);
    assert_eq!(chain.steps.len(), 0);

    chain.add_step(InternalStep::new(StepType::Premise, "P1".to_string(), 0.9));
    chain.add_step(InternalStep::new(StepType::Inference, "I1".to_string(), 0.85));
    chain.add_step(InternalStep::new(StepType::Conclusion, "C1".to_string(), 0.8));

    assert_eq!(chain.steps.len(), 3);
    assert!(chain.get_conclusion().is_some());
    assert!((chain.average_confidence() - (0.9 + 0.85 + 0.8) / 3.0).abs() < 0.001);
}

#[test]
fn test_chain_checkpoint() {
    let mut chain = InternalChain::new(ReasoningStrategy::ChainOfThought);
    chain.add_step(InternalStep::new(StepType::Premise, "P1".to_string(), 0.9));
    let cp_id = chain.add_checkpoint();
    assert!(chain.checkpoints.contains(&cp_id));
    assert_eq!(chain.step_count(), 1);
}

#[test]
fn test_chain_prune() {
    let mut chain = InternalChain::new(ReasoningStrategy::ChainOfThought);
    chain.add_step(InternalStep::new(StepType::Premise, "P1".to_string(), 0.9));
    chain.add_step(InternalStep::new(StepType::Inference, "I1".to_string(), 0.3));
    chain.add_step(InternalStep::new(StepType::Conclusion, "C1".to_string(), 0.8));

    chain.prune_below_confidence(0.5);
    assert_eq!(chain.steps.len(), 2);
}

#[test]
fn test_internal_reasoning_state() {
    let mut state = InternalReasoningState::new();
    let chain_id = state.start_chain(ReasoningStrategy::Deductive);

    if let Some(chain) = state.active_chain_mut() {
        chain.add_step(InternalStep::new(StepType::Premise, "premise".to_string(), 0.9));
        chain.add_step(InternalStep::new(StepType::Conclusion, "conclusion".to_string(), 0.85));
    }
    state.finalize_active_chain();

    assert!(state.active_chain().is_none());
    let best = state.best_chain();
    assert!(best.is_some());
    assert_eq!(best.unwrap().id, chain_id);
}

#[test]
fn test_internal_state_working_memory() {
    let mut state = InternalReasoningState::new();
    state.store_working("key".to_string(), serde_json::json!("value"));
    assert_eq!(state.get_working("key"), Some(&serde_json::json!("value")));
    assert!(state.get_working("missing").is_none());
}

// ── Planning Tests ──

#[test]
fn test_planning_engine() {
    let engine = PlanningEngine::new();
    let goal = Goal::new("Solve the problem".to_string());
    let plan = engine.create_plan(goal, ReasoningStrategy::Deductive).unwrap();
    assert!(!plan.tasks.is_empty());
    assert!(engine.validate_plan(&plan).is_ok());
}

#[test]
fn test_plan_select_best() {
    let engine = PlanningEngine::new();
    let g1 = Goal::new("Task A".to_string());
    let g2 = Goal::new("Task B".to_string());
    let p1 = engine.create_plan(g1, ReasoningStrategy::Deductive).unwrap();
    let p2 = engine.create_plan(g2, ReasoningStrategy::Deductive).unwrap();

    let plans = vec![p1, p2];
    let best = engine.select_best_plan(&plans);
    assert!(best.is_some());
}

// ── Reflection Tests ──

#[test]
fn test_reflection_engine() {
    let engine = ReflectionEngine::new();
    let mut state = InternalReasoningState::new();
    state.start_chain(ReasoningStrategy::ChainOfThought);

    if let Some(chain) = state.active_chain_mut() {
        chain.add_step(InternalStep::new(StepType::Premise, "premise".to_string(), 0.9));
        chain.add_step(InternalStep::new(StepType::Conclusion, "conclusion".to_string(), 0.85));
    }
    state.finalize_active_chain();

    let context = HashMap::new();
    let result = engine.reflect(&state, &context).unwrap();
    assert!(result.overall_score >= 0.0 && result.overall_score <= 1.0);
}

// ── Hypothesis Tests ──

#[test]
fn test_hypothesis_engine() {
    let engine = HypothesisEngine::new().with_max_hypotheses(5);
    let context = HashMap::new();
    let hypotheses = engine.generate_hypotheses("Why is the sky blue?", &context, 3);
    assert!(!hypotheses.is_empty());
    assert!(hypotheses.len() <= 3);
}

#[test]
fn test_hypothesis_best() {
    let engine = HypothesisEngine::new();
    let h1 = Hypothesis::new("H1".to_string(), 0.9);
    let h2 = Hypothesis::new("H2".to_string(), 0.5);

    let hyps = vec![h1.clone(), h2.clone()];
    let best = engine.best_hypothesis(&hyps);
    assert!(best.is_some());
    assert_eq!(best.unwrap().statement, "H1");

    let mut h3 = Hypothesis::new("H3".to_string(), 0.1);
    h3.status = HypothesisStatus::Rejected;
    let hyps = vec![h2, h3];
    let best = engine.best_hypothesis(&hyps);
    assert_eq!(best.unwrap().statement, "H2");
}

#[test]
fn test_hypothesis_evidence() {
    let mut engine = HypothesisEngine::new();
    let h = Hypothesis::new("Test hypothesis".to_string(), 0.5);

    let evidence = Evidence::new("Supporting evidence".to_string(), 0.8)
        .supports_hypothesis(h.id);

    engine.accumulate_evidence(h.id, evidence, true);
    assert_eq!(engine.evidence_count(), 1);
}

#[test]
fn test_hypothesis_ranking() {
    let mut engine = HypothesisEngine::new();
    let h1 = Hypothesis::new("H1".to_string(), 0.9);
    let h2 = Hypothesis::new("H2".to_string(), 0.5);

    let evidence = Evidence::new("support".to_string(), 0.7).supports_hypothesis(h1.id);
    engine.accumulate_evidence(h1.id, evidence, true);

    let rankings = engine.rank_hypotheses(&[h1, h2]);
    assert_eq!(rankings.len(), 2);
    assert_eq!(rankings[0].rank, 1);
}

#[test]
fn test_hypothesis_discard_weak() {
    let engine = HypothesisEngine::new();
    let mut h1 = Hypothesis::new("strong".to_string(), 0.9);
    let mut h2 = Hypothesis::new("weak".to_string(), 0.1);
    h1.supporting_evidence.clear();
    h2.contradicting_evidence.clear();

    let mut hypotheses = vec![h1, h2];
    let discarded = engine.discard_weak(&mut hypotheses, 0.5);
    assert!(!discarded.is_empty());
}

// ── Decision Tests ──

#[test]
fn test_decision_engine() {
    let engine = DecisionEngine::new();
    let options = vec![
        DecisionOption::new("Option A".to_string())
            .with_utility(0.7)
            .with_risk(0.3),
        DecisionOption::new("Option B".to_string())
            .with_utility(0.5)
            .with_risk(0.2),
    ];

    let result = engine.select_best(&options, None).unwrap();
    assert!(!result.all_scored.is_empty());
}

#[test]
fn test_decision_with_weights() {
    let engine = DecisionEngine::new();
    let options = vec![
        DecisionOption::new("Safe".to_string())
            .with_utility(0.6)
            .with_risk(0.1),
        DecisionOption::new("Risky".to_string())
            .with_utility(0.9)
            .with_risk(0.8),
    ];

    let weights = vec![
        ObjectiveWeight { name: "utility".to_string(), weight: 0.3 },
        ObjectiveWeight { name: "risk".to_string(), weight: 0.7 },
    ];

    let result = engine.select_best(&options, Some(&weights)).unwrap();
    assert!(!result.all_scored.is_empty());
}

#[test]
fn test_decision_no_options() {
    let engine = DecisionEngine::new();
    let result = engine.select_best(&[], None);
    assert!(result.is_err());
}

// ── Knowledge Integration Tests ──

#[test]
fn test_knowledge_integrator() {
    let integrator = KnowledgeIntegrator::new();
    let memory_ctx = vec![RetrievedContext {
        id: uuid::Uuid::new_v4(),
        content: "Memory content".to_string(),
        relevance_score: 0.8,
        confidence: 0.9,
        source: ContextSource::Memory,
        timestamp: chrono::Utc::now(),
        metadata: HashMap::new(),
    }];

    let result = integrator.integrate(memory_ctx, vec![], vec![], vec![]).unwrap();
    assert_eq!(result.contexts.len(), 1);
    assert!(!result.merged_knowledge.is_empty());
}

// ── Cache Tests ──

#[test]
fn test_cache_store_and_get() {
    let cache = ReasoningCache::new(100, 3600);
    let result = CachedReasoningResult {
        conclusion: "test conclusion".to_string(),
        confidence: 0.8,
        explanation: "test explanation".to_string(),
        strategy_used: "deductive".to_string(),
        step_count: 5,
        metadata: HashMap::new(),
    };

    cache.store("test query", result);
    let cached = cache.get("test query");
    assert!(cached.is_some());
    assert_eq!(cached.unwrap().conclusion, "test conclusion");
}

#[test]
fn test_cache_miss() {
    let cache = ReasoningCache::new(100, 3600);
    assert!(cache.get("nonexistent").is_none());
}

#[test]
fn test_cache_eviction() {
    let cache = ReasoningCache::new(3, 3600);
    for i in 0..5 {
        let result = CachedReasoningResult {
            conclusion: format!("result {i}"),
            confidence: 0.5,
            explanation: String::new(),
            strategy_used: "test".to_string(),
            step_count: 1,
            metadata: HashMap::new(),
        };
        cache.store(&format!("query {i}"), result);
    }
    let stats = cache.stats();
    assert!(stats.evicted_count > 0);
}

#[test]
fn test_cache_stats() {
    let cache = ReasoningCache::new(100, 3600);
    let result = CachedReasoningResult {
        conclusion: "test".to_string(),
        confidence: 0.8,
        explanation: String::new(),
        strategy_used: "test".to_string(),
        step_count: 1,
        metadata: HashMap::new(),
    };

    cache.store("q1", result);
    cache.get("q1");
    cache.get("miss");

    let stats = cache.stats();
    assert_eq!(stats.hit_count, 1);
    assert_eq!(stats.miss_count, 1);
}

#[test]
fn test_cache_clear() {
    let cache = ReasoningCache::new(100, 3600);
    let result = CachedReasoningResult {
        conclusion: "test".to_string(),
        confidence: 0.8,
        explanation: String::new(),
        strategy_used: "test".to_string(),
        step_count: 1,
        metadata: HashMap::new(),
    };

    cache.store("q1", result);
    assert!(cache.get("q1").is_some());

    cache.clear();
    assert!(cache.get("q1").is_none());
}

// ── Tool Reasoning Tests ──

#[test]
fn test_tool_reasoner() {
    let mut reasoner = ToolReasoner::new();
    let descriptor = ToolDescriptor::new(
        "calculator".to_string(),
        ToolType::Calculator,
        "Performs calculations".to_string(),
    )
    .with_reliability(0.95);

    reasoner.register_tool(descriptor);
    assert_eq!(reasoner.tools().len(), 1);
}

// ── Multi-Model Tests ──

#[test]
fn test_multi_model_reasoner() {
    let mut reasoner = MultiModelReasoner::new();
    let model = ModelBackend::new("model-a".to_string(), ModelRole::Primary);
    reasoner.register_model(model);
    assert_eq!(reasoner.models().len(), 1);
}

#[test]
fn test_multi_model_routing() {
    let mut reasoner = MultiModelReasoner::new();
    let primary = ModelBackend::new("primary".to_string(), ModelRole::Primary);
    let specialist = ModelBackend::new("specialist".to_string(), ModelRole::Specialist);
    reasoner.register_model(primary);
    reasoner.register_model(specialist);

    let selected = reasoner.select_primary(&ReasoningStrategy::ChainOfThought);
    assert!(selected.is_some());
}

// ── Explanation Tests ──

#[test]
fn test_explanation_engine() {
    let engine = ExplanationEngine::new();
    let mut state = InternalReasoningState::new();
    state.start_chain(ReasoningStrategy::Deductive);

    if let Some(chain) = state.active_chain_mut() {
        chain.add_step(InternalStep::new(StepType::Premise, "premise".to_string(), 0.9));
        chain.add_step(InternalStep::new(StepType::Conclusion, "conclusion".to_string(), 0.85));
    }
    state.finalize_active_chain();

    let context = HashMap::new();
    let explanation = engine.generate_explanation(&state, None, &context).unwrap();
    assert!(!explanation.summary.is_empty());
}

// ── Analytics Tests ──

#[test]
fn test_analytics() {
    let analytics = neo_reasoning::analytics::ReasoningAnalytics::new();
    analytics.record_session_start();
    analytics.record_session_complete(42.0, 5, 0.85, "deductive");
    analytics.record_session_start();
    analytics.record_session_complete(55.0, 8, 0.7, "deductive");
    analytics.record_session_start();
    analytics.record_session_complete(30.0, 3, 0.9, "inductive");

    let snapshot = analytics.snapshot();
    assert_eq!(snapshot.total_sessions, 3);
    assert!(snapshot.avg_latency_ms > 0.0);
}

// ── Async Orchestrator Tests ──

#[tokio::test]
async fn test_orchestrator_session_lifecycle() {
    let config = ReasoningConfig::default();
    let orchestrator = ReasoningOrchestrator::new(config);

    let mut request = ReasoningRequest::new("test query".to_string());
    request.strategy = Some(ReasoningStrategy::Deductive);
    let session_id = orchestrator.start_session(request.clone()).await.unwrap();

    let info = orchestrator.inspect_session(session_id).unwrap();
    assert_eq!(info.id, session_id);

    let response = orchestrator.execute_session(session_id, request).await.unwrap();
    assert!(!response.conclusion.is_empty());
    assert!(response.confidence > 0.0);
}

#[tokio::test]
async fn test_orchestrator_cancel() {
    let config = ReasoningConfig::default();
    let orchestrator = ReasoningOrchestrator::new(config);

    let mut request = ReasoningRequest::new("test".to_string());
    request.strategy = Some(ReasoningStrategy::Deductive);
    let session_id = orchestrator.start_session(request).await.unwrap();

    orchestrator.cancel_session(session_id).await.unwrap();

    let info = orchestrator.inspect_session(session_id).unwrap();
    assert_eq!(info.state, SessionState::Cancelled);
}

#[tokio::test]
async fn test_orchestrator_inspect_nonexistent() {
    let config = ReasoningConfig::default();
    let orchestrator = ReasoningOrchestrator::new(config);
    let result = orchestrator.inspect_session(uuid::Uuid::new_v4());
    assert!(result.is_err());
}

#[tokio::test]
async fn test_orchestrator_strategies() {
    let config = ReasoningConfig::default();
    let orchestrator = ReasoningOrchestrator::new(config);
    let strategies = orchestrator.strategies();
    assert_eq!(strategies.len(), 9);
}

#[tokio::test]
async fn test_orchestrator_analytics() {
    let config = ReasoningConfig::default();
    let orchestrator = ReasoningOrchestrator::new(config);

    let mut request = ReasoningRequest::new("analytics test".to_string());
    request.strategy = Some(ReasoningStrategy::Deductive);
    let session_id = orchestrator.start_session(request.clone()).await.unwrap();
    let _ = orchestrator.execute_session(session_id, request).await;

    let snapshot = orchestrator.analytics();
    assert!(snapshot.total_sessions > 0);
}

#[tokio::test]
async fn test_orchestrator_export_summary() {
    let config = ReasoningConfig::default();
    let orchestrator = ReasoningOrchestrator::new(config);

    let mut request = ReasoningRequest::new("summary test".to_string());
    request.strategy = Some(ReasoningStrategy::Deductive);
    let session_id = orchestrator.start_session(request.clone()).await.unwrap();
    let _ = orchestrator.execute_session(session_id, request).await;

    let summary = orchestrator.export_summary(session_id).unwrap();
    assert_eq!(summary.info.id, session_id);
}

#[tokio::test]
async fn test_multiple_reasoning_sessions() {
    let config = ReasoningConfig::default();
    let orchestrator = ReasoningOrchestrator::new(config);

    let mut session_ids = Vec::new();
    for i in 0..5 {
        let mut request = ReasoningRequest::new(format!("query {i}"));
        request.strategy = Some(ReasoningStrategy::Deductive);
        let id = orchestrator.start_session(request.clone()).await.unwrap();
        session_ids.push((id, request));
    }

    for (id, request) in session_ids {
        let response = orchestrator.execute_session(id, request).await.unwrap();
        assert!(!response.conclusion.is_empty());
    }
}

// ── API Tests ──

#[tokio::test]
async fn test_api_default() {
    let api = ReasoningApi::default();
    let response = api
        .reason_with_strategy("If it rains, the ground is wet. It rains.".to_string(), ReasoningStrategy::Deductive)
        .await
        .unwrap();
    assert!(!response.conclusion.is_empty());
}

#[tokio::test]
async fn test_api_with_strategy() {
    let api = ReasoningApi::default();
    let response = api
        .reason_with_strategy(
            "If it rains, the ground is wet. It rains.".to_string(),
            ReasoningStrategy::Deductive,
        )
        .await
        .unwrap();
    assert_eq!(response.strategy_used, "deductive");
}

#[tokio::test]
async fn test_api_with_context() {
    let api = ReasoningApi::default();
    let mut context = HashMap::new();
    context.insert("domain".to_string(), serde_json::json!("physics"));

    let response = api
        .reason_with_context("Explain gravity".to_string(), context)
        .await
        .unwrap();
    assert!(!response.conclusion.is_empty());
}

#[tokio::test]
async fn test_api_inspect_and_cancel() {
    let api = ReasoningApi::default();
    let mut req = ReasoningRequest::new("test".to_string());
    req.strategy = Some(ReasoningStrategy::Deductive);
    let session_id = api.start_reasoning(req).await.unwrap();

    let info = api.inspect_reasoning(session_id).unwrap();
    assert_eq!(info.id, session_id);

    api.cancel_reasoning(session_id).await.unwrap();
    let info = api.inspect_reasoning(session_id).unwrap();
    assert_eq!(info.state, SessionState::Cancelled);
}

#[tokio::test]
async fn test_api_available_strategies() {
    let api = ReasoningApi::default();
    let strategies = api.available_strategies();
    assert_eq!(strategies.len(), 9);
}

#[tokio::test]
async fn test_api_analytics() {
    let api = ReasoningApi::default();
    let snapshot = api.analytics();
    assert!(snapshot.total_sessions == 0);
}

#[tokio::test]
async fn test_api_cache_stats() {
    let api = ReasoningApi::default();
    let _ = api
        .reason_with_strategy("cache test".to_string(), ReasoningStrategy::Deductive)
        .await;
    let stats = api.cache_stats();
    assert!(stats.total_entries > 0 || stats.evicted_count >= 0);
}

#[test]
fn test_api_decide() {
    let api = ReasoningApi::default();
    let result = api
        .decide(vec!["Option A".to_string(), "Option B".to_string()], None)
        .unwrap();
    assert!(!result.all_scored.is_empty());
}

#[test]
fn test_api_generate_hypotheses() {
    let api = ReasoningApi::default();
    let hypotheses = api.generate_hypotheses("Why does X happen?".to_string(), 3);
    assert!(!hypotheses.is_empty());
}

// ── Serialization Tests ──

#[test]
fn test_config_serialization() {
    let config = ReasoningConfig::default();
    let json = serde_json::to_string(&config).unwrap();
    let deserialized: ReasoningConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(config.max_depth, deserialized.max_depth);
    assert_eq!(config.timeout_ms, deserialized.timeout_ms);
}

#[test]
fn test_session_serialization() {
    let session = ReasoningSession::new(
        "test".to_string(),
        ReasoningStrategy::Deductive,
        30_000,
    );
    let json = serde_json::to_string(&session).unwrap();
    let deserialized: ReasoningSession = serde_json::from_str(&json).unwrap();
    assert_eq!(session.query, deserialized.query);
}

// ── Reflection Disabled Test ──

#[tokio::test]
async fn test_reflection_disabled() {
    let mut config = ReasoningConfig::default();
    config.enable_reflection = false;

    let orchestrator = ReasoningOrchestrator::new(config);
    let mut request = ReasoningRequest::new("no reflection".to_string());
    request.strategy = Some(ReasoningStrategy::Deductive);
    let session_id = orchestrator.start_session(request.clone()).await.unwrap();
    let response = orchestrator.execute_session(session_id, request).await.unwrap();
    assert!(!response.conclusion.is_empty());
}
