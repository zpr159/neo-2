use std::sync::Arc;

use neo_core::conversation::config::*;
use neo_core::conversation::context_ranker::*;
use neo_core::conversation::context_merger::*;
use neo_core::conversation::evidence::{Evidence, EvidenceCollection, EvidenceSource, Provenance};
use neo_core::conversation::executive_bridge::*;
use neo_core::conversation::memory_bridge::*;
use neo_core::conversation::knowledge_bridge::*;
use neo_core::conversation::planning_bridge::*;
use neo_core::conversation::reasoning_bridge::*;
use neo_core::conversation::retrieval_coordinator::*;
use neo_core::conversation::tool_coordinator::*;
use neo_core::conversation::world_model_bridge::*;
use neo_core::conversation::workflow_bridge::*;
use neo_core::conversation::agent_bridge::*;
use neo_core::conversation::prompt_builder::*;
use neo_core::conversation::response_validator::*;
use neo_core::conversation::pipeline::*;
use neo_core::conversation::manager::*;
use neo_core::conversation::types::*;
use neo_core::conversation::error::*;
use neo_core::component::Component;
use neo_core::id::AgentId;

fn test_context() -> ConversationContext {
    ConversationContext::new(uuid::Uuid::new_v4(), uuid::Uuid::new_v4())
}

fn test_evidence(source: EvidenceSource, content: &str, confidence: f32) -> Evidence {
    Evidence::new(source, confidence, content, "test_retrieval")
}

fn test_evidence_with_relevance(source: EvidenceSource, content: &str, confidence: f32, relevance: f32) -> Evidence {
    Evidence::new(source, confidence, content, "test_retrieval").with_relevance(relevance)
}

// Executive Bridge Tests

#[tokio::test]
async fn test_intent_classification() {
    let bridge = MockExecutiveBridge;
    let context = test_context();
    let intent = bridge.classify_intent(&context, "What is the capital of France?").await.unwrap();
    assert_eq!(intent, Intent::Conversation);
}

#[tokio::test]
async fn test_intent_display() {
    assert_eq!(Intent::Question.to_string(), "question");
    assert_eq!(Intent::Coding.to_string(), "coding");
    assert_eq!(Intent::Custom("test".to_string()).to_string(), "custom:test");
}

#[tokio::test]
async fn test_executive_decision() {
    let bridge = MockExecutiveBridge;
    let context = test_context();
    let decision = bridge.process_objective(&context, "Help me with my code").await.unwrap();
    assert_eq!(decision.intent, Intent::Conversation);
    assert_eq!(decision.urgency, Urgency::Normal);
    assert_eq!(decision.execution_policy, ExecutionPolicy::Immediate);
}

#[tokio::test]
async fn test_executive_authorize_tool() {
    let bridge = MockExecutiveBridge;
    let context = test_context();
    let args = serde_json::json!({"file": "test.rs"});
    let auth = bridge.authorize_tool(&context, "read_file", &args).await.unwrap();
    assert_eq!(auth, ToolAuthorization::Auto);
}

#[tokio::test]
async fn test_executive_approve_workflow() {
    let bridge = MockExecutiveBridge;
    let context = test_context();
    let approved = bridge.approve_workflow(&context, "workflow-1").await.unwrap();
    assert!(approved);
}

#[tokio::test]
async fn test_executive_approve_agent_delegation() {
    let bridge = MockExecutiveBridge;
    let context = test_context();
    let approved = bridge.approve_agent_delegation(&context, "agent-1", "Do something").await.unwrap();
    assert!(approved);
}

// Planning Bridge Tests

#[tokio::test]
async fn test_planning_decompose() {
    let bridge = MockPlanningBridge;
    let context = test_context();
    let plan = bridge.decompose(&context, "Build a web application").await.unwrap();
    assert_eq!(plan.subtasks.len(), 1);
    assert!(!plan.clarification_needed);
    assert!(!plan.subtasks[0].id.is_empty());
}

#[tokio::test]
async fn test_planning_estimate_cost() {
    let bridge = MockPlanningBridge;
    let context = test_context();
    let cost = bridge.estimate_cost(&context, "Analyze data").await.unwrap();
    assert!(cost >= 0.0);
}

#[tokio::test]
async fn test_planning_generate_plan() {
    let bridge = MockPlanningBridge;
    let context = test_context();
    let plan = bridge.generate_plan(&context, "Complex task").await.unwrap();
    assert!(!plan.subtasks.is_empty());
    assert!(!plan.execution_graph.layers.is_empty());
}

// Reasoning Bridge Tests

#[tokio::test]
async fn test_reasoning_logical_inference() {
    let bridge = MockReasoningBridge;
    let context = test_context();
    let props = vec!["A implies B".to_string(), "A is true".to_string()];
    let result = bridge.logical_inference(&context, &props).await.unwrap();
    assert!(result.confidence > 0.0);
    assert!(!result.conclusion.is_empty());
}

#[tokio::test]
async fn test_reasoning_consistency_check() {
    let bridge = MockReasoningBridge;
    let context = test_context();
    let statements = vec!["X is 5".to_string(), "Y is 10".to_string()];
    let result = bridge.consistency_check(&context, &statements).await.unwrap();
    assert!(result.contradictions.is_empty());
}

#[tokio::test]
async fn test_reasoning_detect_contradictions() {
    let bridge = MockReasoningBridge;
    let context = test_context();
    let statements = vec!["It is raining".to_string(), "It is not raining".to_string()];
    let contradictions = bridge.detect_contradictions(&context, &statements).await.unwrap();
    assert!(contradictions.is_empty());
}

#[tokio::test]
async fn test_reasoning_estimate_confidence() {
    let bridge = MockReasoningBridge;
    let context = test_context();
    let evidence = vec![];
    let confidence = bridge.estimate_confidence(&context, "Conclusion A", &evidence).await.unwrap();
    assert!(confidence > 0.0);
}

// Memory Bridge Tests

#[tokio::test]
async fn test_memory_retrieve() {
    let bridge = MockMemoryBridge;
    let context = test_context();
    let query = MemoryQuery {
        text: "test query".to_string(),
        memory_types: vec![MemoryType::Working],
        retrieval_methods: vec![RetrievalMethod::SimilaritySearch],
        limit: 10,
        ..Default::default()
    };
    let result = bridge.retrieve(&context, &query).await.unwrap();
    assert_eq!(result.total_retrieved, 0);
    assert!(!result.memory_types_used.is_empty());
}

#[tokio::test]
async fn test_memory_store() {
    let bridge = MockMemoryBridge;
    let context = test_context();
    let item = MemoryConsolidationItem {
        content: "Important fact".to_string(),
        memory_type: MemoryType::Episodic,
        importance: 0.8,
        context: std::collections::HashMap::new(),
        source_conversation_id: uuid::Uuid::new_v4(),
        timestamp: neo_core::time::Timestamp::now(),
    };
    let result = bridge.store(&context, &item).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_memory_consolidate() {
    let bridge = MockMemoryBridge;
    let context = test_context();
    let items = vec![MemoryConsolidationItem {
        content: "Fact to remember".to_string(),
        memory_type: MemoryType::Semantic,
        importance: 0.7,
        context: std::collections::HashMap::new(),
        source_conversation_id: uuid::Uuid::new_v4(),
        timestamp: neo_core::time::Timestamp::now(),
    }];
    let result = bridge.consolidate(&context, &items).await;
    assert!(result.is_ok());
}

// Knowledge Bridge Tests

#[tokio::test]
async fn test_knowledge_entity_lookup() {
    let bridge = MockKnowledgeBridge;
    let context = test_context();
    let entities = bridge.entity_lookup(&context, "Rust").await.unwrap();
    assert!(entities.is_empty());
}

#[tokio::test]
async fn test_knowledge_graph_search() {
    let bridge = MockKnowledgeBridge;
    let context = test_context();
    let result = bridge.graph_search(&context, "programming languages", 10).await.unwrap();
    assert_eq!(result.confidence, 0.0);
}

#[tokio::test]
async fn test_knowledge_verify_fact() {
    let bridge = MockKnowledgeBridge;
    let context = test_context();
    let (verified, confidence) = bridge.verify_fact(&context, "Rust", "is", "a language").await.unwrap();
    assert!(!verified);
    assert_eq!(confidence, 0.0);
}

// World Model Bridge Tests

#[tokio::test]
async fn test_world_model_get_state() {
    let bridge = MockWorldModelBridge;
    let context = test_context();
    let state = bridge.get_world_state(&context).await.unwrap();
    assert_eq!(state.version, 0);
    assert!(state.entities.is_empty());
}

#[tokio::test]
async fn test_world_model_simulate() {
    let bridge = MockWorldModelBridge;
    let context = test_context();
    let request = SimulationRequest {
        scenario: "test scenario".to_string(),
        initial_state: std::collections::HashMap::new(),
        steps: 10,
        parameters: std::collections::HashMap::new(),
    };
    let result = bridge.simulate(&context, &request).await.unwrap();
    assert_eq!(result.confidence, 0.0);
}

// Workflow Bridge Tests

#[tokio::test]
async fn test_workflow_discover() {
    let bridge = MockWorkflowBridge;
    let context = test_context();
    let workflows = bridge.discover_workflows(&context, "deploy").await.unwrap();
    assert!(workflows.is_empty());
}

#[tokio::test]
async fn test_workflow_execute() {
    let bridge = MockWorkflowBridge;
    let context = test_context();
    let params = std::collections::HashMap::new();
    let info = bridge.execute_workflow(&context, "workflow-1", &params).await.unwrap();
    assert_eq!(info.status, WorkflowStatus::Completed);
}

// Agent Bridge Tests

#[tokio::test]
async fn test_agent_discover() {
    let bridge = MockAgentBridge;
    let context = test_context();
    let caps = vec!["coding".to_string()];
    let agents = bridge.discover_agents(&context, &caps).await.unwrap();
    assert!(agents.is_empty());
}

#[tokio::test]
async fn test_agent_assign() {
    let bridge = MockAgentBridge;
    let context = test_context();
    let agent_id = AgentId::new();
    let result = bridge.assign_objective(&context, agent_id, "Write tests").await.unwrap();
    assert_eq!(result.status, AgentStatus::Available);
}

// Context Ranker Tests

#[test]
fn test_context_ranker_basic() {
    let config = RankingConfig::default();
    let ranker = ContextRanker::new(config);
    let evidence = vec![
        test_evidence_with_relevance(EvidenceSource::Memory, "Memory fact", 0.9, 0.8),
        test_evidence_with_relevance(EvidenceSource::KnowledgeGraph, "KG fact", 0.7, 0.6),
        test_evidence_with_relevance(EvidenceSource::Reasoning, "Reasoning conclusion", 0.85, 0.9),
    ];
    let ranked = ranker.rank(evidence, &["fact".to_string()], "test query");
    assert_eq!(ranked.len(), 3);
    for i in 1..ranked.len() {
        assert!(ranked[i - 1].final_score >= ranked[i].final_score);
    }
}

#[test]
fn test_context_ranker_empty() {
    let config = RankingConfig::default();
    let ranker = ContextRanker::new(config);
    let ranked = ranker.rank(vec![], &[], "test");
    assert!(ranked.is_empty());
}

#[test]
fn test_ranked_evidence_computation() {
    let config = RankingConfig::default();
    let mut re = RankedEvidence::new(test_evidence(EvidenceSource::Memory, "test", 0.8));
    re.semantic_similarity = 0.5;
    re.recency_score = 0.9;
    re.importance_score = 0.7;
    re.confidence_score = 0.8;
    re.source_reliability = 0.7;
    re.user_relevance = 0.6;
    re.task_relevance = 0.4;
    re.compute_final_score(&config);
    assert!(re.final_score > 0.0);
    assert!(re.final_score <= 1.0);
}

// Context Merger Tests

#[test]
fn test_context_merger_merge_empty() {
    let merger = ContextMerger::new();
    let ctx = merger.merge(vec![]);
    assert_eq!(ctx.total_items, 0);
}

#[test]
fn test_context_merger_dedup() {
    let merger = ContextMerger::new();
    let evidence = vec![
        test_evidence(EvidenceSource::Memory, "same fact", 0.9),
        test_evidence(EvidenceSource::Memory, "same fact", 0.7),
    ];
    let ctx = merger.merge(evidence);
    assert_eq!(ctx.evidence.len(), 1);
    assert!(ctx.evidence[0].confidence >= 0.7);
}

#[test]
fn test_context_merger_different_sources_not_deduped() {
    let merger = ContextMerger::new();
    let evidence = vec![
        test_evidence(EvidenceSource::Memory, "same fact", 0.9),
        test_evidence(EvidenceSource::KnowledgeGraph, "same fact", 0.7),
    ];
    let ctx = merger.merge(evidence);
    assert_eq!(ctx.evidence.len(), 2);
}

#[test]
fn test_context_merger_contradiction_detection() {
    let mut ctx = UnifiedContext::default();
    ctx.evidence.push(test_evidence(EvidenceSource::Memory, "It is raining", 0.9));
    ctx.evidence.push(test_evidence(EvidenceSource::KnowledgeGraph, "It is not raining", 0.8));
    ContextMerger::detect_contradictions(&mut ctx);
    assert!(ctx.contradictions_detected > 0);
}

#[test]
fn test_context_merger_deterministic_sort() {
    let mut ctx = UnifiedContext::default();
    ctx.evidence.push(test_evidence(EvidenceSource::Reasoning, "C", 0.7));
    ctx.evidence.push(test_evidence(EvidenceSource::Memory, "A", 0.9));
    ctx.evidence.push(test_evidence(EvidenceSource::KnowledgeGraph, "B", 0.8));
    ContextMerger::sort_deterministic(&mut ctx);
    for i in 1..ctx.evidence.len() {
        assert!(ctx.evidence[i - 1].id <= ctx.evidence[i].id);
    }
}

#[test]
fn test_context_merger_average_confidence() {
    let merger = ContextMerger::new();
    let evidence = vec![
        test_evidence(EvidenceSource::Memory, "A", 0.8),
        test_evidence(EvidenceSource::Memory, "B", 0.6),
    ];
    let ctx = merger.merge(evidence);
    assert!((ctx.average_confidence - 0.7).abs() < 0.01);
}

// Evidence Tests

#[test]
fn test_evidence_creation() {
    let e = test_evidence(EvidenceSource::Memory, "test content", 0.85);
    assert_eq!(e.source, EvidenceSource::Memory);
    assert_eq!(e.confidence, 0.85);
}

#[test]
fn test_evidence_chaining() {
    let e = Evidence::new(EvidenceSource::Reasoning, 0.9, "fact", "method")
        .with_relevance(0.8)
        .with_explanation("Because X")
        .with_reference("ref://1");
    assert_eq!(e.relevance_score, 0.8);
    assert_eq!(e.supporting_references.len(), 1);
}

#[test]
fn test_evidence_collection() {
    let mut collection = EvidenceCollection::new();
    assert!(collection.is_empty());
    collection.push(test_evidence(EvidenceSource::Memory, "A", 0.8));
    collection.push(test_evidence(EvidenceSource::KnowledgeGraph, "B", 0.6));
    collection.push(test_evidence(EvidenceSource::Memory, "C", 0.9));
    assert_eq!(collection.len(), 3);
    assert_eq!(collection.source_distribution[&EvidenceSource::Memory], 2);
}

#[test]
fn test_provenance_chain() {
    let prov = Provenance::default()
        .root("user_input")
        .derivation("deduction")
        .step(EvidenceSource::Memory, "retrieved", 0.9);
    assert_eq!(prov.root_source, Some("user_input".to_string()));
    assert_eq!(prov.chain.len(), 1);
}

// Tool Coordinator Tests

struct MockToolExecutor;

#[async_trait::async_trait]
impl ToolExecutor for MockToolExecutor {
    async fn execute(&self, tool_name: &str, _arguments: &serde_json::Value, _timeout_ms: u64) -> ConversationResult<ToolExecutionResult> {
        Ok(ToolExecutionResult {
            tool_name: tool_name.to_string(),
            status: ToolExecutionStatus::Success,
            output: serde_json::json!({"result": "success"}),
            execution_time_ms: 10,
            logs: vec![],
            warnings: vec![],
            confidence: 0.9,
            errors: vec![],
        })
    }
}

async fn create_test_tool_coordinator() -> ToolCoordinator {
    let config = ToolConfig::default();
    let executor = Arc::new(MockToolExecutor);
    let coordinator = ToolCoordinator::new(config, executor);
    coordinator.register_tool(ToolDefinitionFull {
        capability: ToolCapability {
            name: "read_file".to_string(),
            description: "Read a file".to_string(),
            category: "file".to_string(),
            safe: true,
            requires_approval: false,
            estimated_cost: 0.0,
            tags: vec!["file".to_string()],
        },
        schema: serde_json::json!({"type": "object"}),
        version: "1.0.0".to_string(),
        source: ToolSource::Local,
    }).await;
    coordinator
}

#[tokio::test]
async fn test_tool_discovery() {
    let coordinator = create_test_tool_coordinator().await;
    let tools = coordinator.discover_tools("file").await;
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "read_file");
}

#[tokio::test]
async fn test_tool_definitions_conversion() {
    let coordinator = create_test_tool_coordinator().await;
    let defs = coordinator.to_tool_definitions().await;
    assert_eq!(defs.len(), 1);
}

#[tokio::test]
async fn test_tool_execution() {
    let coordinator = create_test_tool_coordinator().await;
    let context = test_context();
    let executive = MockExecutiveBridge;
    let request = ToolExecutionRequest {
        tool_name: "read_file".to_string(),
        arguments: serde_json::json!({"path": "/test/file.rs"}),
        timeout_ms: None,
        retries: None,
        chain_id: None,
    };
    let result = coordinator.execute_tool(&context, &request, &executive).await.unwrap();
    assert_eq!(result.status, ToolExecutionStatus::Success);
    assert_eq!(result.tool_name, "read_file");
}

#[tokio::test]
async fn test_tool_chain_execution() {
    let coordinator = create_test_tool_coordinator().await;
    let context = test_context();
    let executive = MockExecutiveBridge;
    let chain = ToolChain {
        id: "chain-1".to_string(),
        name: "test chain".to_string(),
        description: "A test chain".to_string(),
        steps: vec![ToolChainStep {
            step_index: 0,
            tool_name: "read_file".to_string(),
            input_mapping: std::collections::HashMap::from([("path".to_string(), "$previous_output".to_string())]),
            timeout_ms: None,
        }],
    };
    let initial_input = serde_json::json!({"path": "/test/file.rs"});
    let result = coordinator.execute_chain(&context, &chain, &initial_input, &executive).await.unwrap();
    assert!(result.success);
    assert_eq!(result.step_results.len(), 1);
}

// Prompt Builder Tests

#[test]
fn test_prompt_builder_basic() {
    let builder = PromptBuilder::new();
    let context = test_context();
    let cognitive = CognitiveContext {
        ranked_evidence: vec![],
        unified: UnifiedContextForSerialization::default(),
        executive_context: None,
        planning_context: None,
        reasoning_context: None,
        memory_context: None,
        knowledge_context: None,
        world_model_context: None,
        agent_context: None,
        workflow_context: None,
        confidence: 0.0,
    };
    let prompt = builder.build(&context, &cognitive, &[], &[]).unwrap();
    assert!(!prompt.system_message.content.is_empty());
    assert!(prompt.metadata.contains_key("conversation_id"));
}

// Response Validator Tests

#[test]
fn test_response_validator_basic() {
    let validator = ResponseValidator::new();
    let context = test_context();
    let response = neo_core::language::types::GenerationResponse {
        id: uuid::Uuid::new_v4(),
        text: "This is a valid response about the topic.".to_string(),
        finish_reason: neo_core::language::types::FinishReason::Stop,
        usage: neo_core::language::types::TokenUsage::default(),
        latency: std::time::Duration::from_millis(100),
        provider: "mock".to_string(),
        model: "mock-model".to_string(),
        confidence: Some(0.9),
        warnings: vec![],
        tool_calls: None,
        metadata: std::collections::HashMap::new(),
        created_at: neo_core::time::Timestamp::now(),
    };
    let validated = validator.validate(&response, &context, None).unwrap();
    assert!(validated.safety_passed);
}

#[test]
fn test_response_validator_safety_check() {
    let validator = ResponseValidator::new();
    let response = neo_core::language::types::GenerationResponse {
        id: uuid::Uuid::new_v4(),
        text: "Run this command: rm -rf /".to_string(),
        finish_reason: neo_core::language::types::FinishReason::Stop,
        usage: neo_core::language::types::TokenUsage::default(),
        latency: std::time::Duration::ZERO,
        provider: "mock".to_string(),
        model: "mock".to_string(),
        confidence: None,
        warnings: vec![],
        tool_calls: None,
        metadata: std::collections::HashMap::new(),
        created_at: neo_core::time::Timestamp::now(),
    };
    let validated = validator.validate(&response, &test_context(), None).unwrap();
    assert!(!validated.safety_passed);
}

#[test]
fn test_response_validator_markdown_normalization() {
    let text = "Hello ```world";
    let normalized = ResponseValidator::normalize_markdown(text);
    assert!(normalized.ends_with("\n```"));
}

#[test]
fn test_response_validator_short_response_warning() {
    let validator = ResponseValidator::new();
    let response = neo_core::language::types::GenerationResponse {
        id: uuid::Uuid::new_v4(),
        text: "Hi".to_string(),
        finish_reason: neo_core::language::types::FinishReason::Stop,
        usage: neo_core::language::types::TokenUsage::default(),
        latency: std::time::Duration::ZERO,
        provider: "mock".to_string(),
        model: "mock".to_string(),
        confidence: None,
        warnings: vec![],
        tool_calls: None,
        metadata: std::collections::HashMap::new(),
        created_at: neo_core::time::Timestamp::now(),
    };
    let validated = validator.validate(&response, &test_context(), None).unwrap();
    assert!(validated.warnings.iter().any(|w| w.contains("short")));
}

// Config Tests

#[test]
fn test_conversation_config_default() {
    let config = ConversationConfig::default();
    assert_eq!(config.max_context_messages, 50);
    assert!(config.enable_memory_retrieval);
    assert!(config.auto_consolidate_memory);
}

#[test]
fn test_tool_config_default() {
    let config = ToolConfig::default();
    assert!(config.auto_approve_safe_tools);
    assert!(config.enable_tool_chains);
}

// ConversationManager Tests

fn create_test_pipeline() -> Arc<ConversationPipeline> {
    let config = ConversationConfig::default();
    let executive = Arc::new(MockExecutiveBridge);
    let planning = Arc::new(MockPlanningBridge);
    let reasoning = Arc::new(MockReasoningBridge);
    let memory = Arc::new(MockMemoryBridge);
    let knowledge = Arc::new(MockKnowledgeBridge);
    let world_model = Arc::new(MockWorldModelBridge);
    let workflow = Arc::new(MockWorkflowBridge);
    let agent = Arc::new(MockAgentBridge);
    let tool_config = ToolConfig::default();
    let executor = Arc::new(MockToolExecutor);
    let tool_coordinator = Arc::new(ToolCoordinator::new(tool_config, executor));
    let language_engine = Arc::new(MockLanguageEngine);
    Arc::new(ConversationPipeline::new(
        config, executive, planning, reasoning, memory,
        knowledge, world_model, workflow, agent, language_engine, tool_coordinator,
    ))
}

#[tokio::test]
async fn test_manager_create_session() {
    let pipeline = create_test_pipeline();
    let manager = ConversationManager::new(ConversationConfig::default(), pipeline);
    let _session_id = manager.create_session(Some("user-1".to_string())).await;
    assert_eq!(manager.active_session_count().await, 1);
}

#[tokio::test]
async fn test_manager_create_conversation() {
    let pipeline = create_test_pipeline();
    let manager = ConversationManager::new(ConversationConfig::default(), pipeline);
    let session_id = manager.create_session(None).await;
    let conv_id = manager.create_conversation(session_id).await.unwrap();
    let conversations = manager.list_conversations(session_id).await.unwrap();
    assert_eq!(conversations.len(), 1);
    assert_eq!(conversations[0], conv_id);
}

#[tokio::test]
async fn test_manager_send_message() {
    let pipeline = create_test_pipeline();
    let manager = ConversationManager::new(ConversationConfig::default(), pipeline);
    let session_id = manager.create_session(None).await;
    let conv_id = manager.create_conversation(session_id).await.unwrap();
    let response = manager.send_message(session_id, conv_id, "Hello").await.unwrap();
    assert_eq!(response.conversation_id, conv_id);
}

#[tokio::test]
async fn test_manager_get_history() {
    let pipeline = create_test_pipeline();
    let manager = ConversationManager::new(ConversationConfig::default(), pipeline);
    let session_id = manager.create_session(None).await;
    let conv_id = manager.create_conversation(session_id).await.unwrap();
    let _ = manager.send_message(session_id, conv_id, "Hello").await.unwrap();
    let history = manager.get_history(session_id, conv_id).await.unwrap();
    assert!(!history.is_empty());
}

#[tokio::test]
async fn test_manager_conversation_not_found() {
    let pipeline = create_test_pipeline();
    let manager = ConversationManager::new(ConversationConfig::default(), pipeline);
    let session_id = manager.create_session(None).await;
    let fake_id = uuid::Uuid::new_v4();
    let result = manager.send_message(session_id, fake_id, "Hello").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_manager_cancel_conversation() {
    let pipeline = create_test_pipeline();
    let manager = ConversationManager::new(ConversationConfig::default(), pipeline);
    let session_id = manager.create_session(None).await;
    let conv_id = manager.create_conversation(session_id).await.unwrap();
    manager.cancel_conversation(session_id, conv_id).await.unwrap();
    let conversations = manager.list_conversations(session_id).await.unwrap();
    assert!(conversations.is_empty());
}

#[tokio::test]
async fn test_manager_component_lifecycle() {
    let pipeline = create_test_pipeline();
    let mut manager = ConversationManager::new(ConversationConfig::default(), pipeline);
    assert_eq!(manager.name(), "ConversationManager");
    assert_eq!(manager.state(), neo_core::component::ComponentState::Created);
    manager.initialize().await.unwrap();
    assert_eq!(manager.state(), neo_core::component::ComponentState::Running);
    manager.stop().await.unwrap();
    assert_eq!(manager.state(), neo_core::component::ComponentState::Stopped);
}

#[tokio::test]
async fn test_concurrent_conversations() {
    let pipeline = create_test_pipeline();
    let manager = std::sync::Arc::new(ConversationManager::new(ConversationConfig::default(), pipeline));
    let session_id = manager.create_session(None).await;
    let mut handles = vec![];
    for i in 0..5 {
        let mgr = manager.clone();
        let sid = session_id;
        handles.push(tokio::spawn(async move {
            let conv_id = mgr.create_conversation(sid).await.unwrap();
            let msg = format!("Message {}", i);
            let response = mgr.send_message(sid, conv_id, &msg).await.unwrap();
            assert_eq!(response.conversation_id, conv_id);
        }));
    }
    for handle in handles {
        handle.await.unwrap();
    }
    assert_eq!(manager.active_conversation_count().await, 5);
}

// Error Tests

#[test]
fn test_error_codes() {
    let err = ConversationError::IntentClassificationFailed("test".to_string());
    assert_eq!(err.code(), ConversationErrorCode::IntentClassificationFailed);
    let err = ConversationError::ToolNotFound("test".to_string());
    assert_eq!(err.code(), ConversationErrorCode::ToolNotFound);
}

#[test]
fn test_error_display() {
    let err = ConversationError::PipelineError("something broke".to_string());
    assert_eq!(format!("{}", err), "[pipeline error] something broke");
}

#[test]
fn test_error_to_neo_error() {
    let err = ConversationError::InternalError("internal".to_string());
    let neo_err: neo_core::error::NeoError = err.into();
    assert!(format!("{}", neo_err).contains("conversation"));
}

// ConversationMessage Tests

#[test]
fn test_conversation_message_user() {
    let msg = ConversationMessage::user("Hello");
    assert_eq!(msg.content, "Hello");
    assert_eq!(msg.role, neo_core::language::types::MessageRole::User);
}

#[test]
fn test_conversation_message_to_language_message() {
    let msg = ConversationMessage::user("test");
    let lang_msg = msg.to_language_message();
    assert_eq!(lang_msg.content, "test");
    assert_eq!(lang_msg.role, neo_core::language::types::MessageRole::User);
}

#[test]
fn test_conversation_context() {
    let mut context = test_context();
    assert!(context.messages.is_empty());
    context.push_message(ConversationMessage::user("test"));
    assert_eq!(context.messages.len(), 1);
    assert!(context.last_user_message().is_some());
}

// Distributed Config Tests

#[test]
fn test_distributed_config_default() {
    let config = DistributedConversationConfig::default();
    assert!(!config.enabled);
    assert!(!config.node_failover_enabled);
}

// Pipeline Event Tests

#[test]
fn test_pipeline_event_clone() {
    let event = PipelineEvent::IntentClassified(Intent::Coding);
    match event {
        PipelineEvent::IntentClassified(intent) => assert_eq!(intent, Intent::Coding),
        _ => panic!("wrong variant"),
    }
}

// Mock LanguageEngine for pipeline tests

use async_trait::async_trait;
use neo_core::language::engine::{LanguageEngine, ProviderCapabilities};
use neo_core::language::config::ProviderConfig;
use neo_core::language::error::LanguageResult;
use neo_core::language::types::*;

struct MockLanguageEngine;

#[async_trait]
impl LanguageEngine for MockLanguageEngine {
    fn name(&self) -> &str { "mock" }
    fn config(&self) -> &ProviderConfig {
        Box::leak(Box::new(ProviderConfig::default()))
    }
    async fn load_model(&self, _name: &str) -> LanguageResult<()> { Ok(()) }
    async fn unload_model(&self, _name: &str) -> LanguageResult<()> { Ok(()) }
    async fn health_check(&self) -> LanguageResult<ProviderHealth> { Ok(ProviderHealth::healthy()) }
    async fn generate(&self, config: &GenerationConfig) -> LanguageResult<GenerationResponse> {
        let last_user = config.messages.iter().rev().find(|m| m.role == MessageRole::User);
        let text = last_user.map(|m| format!("Response to: {}", m.content)).unwrap_or_default();
        Ok(GenerationResponse {
            id: uuid::Uuid::new_v4(),
            text,
            finish_reason: FinishReason::Stop,
            usage: TokenUsage { prompt_tokens: 10, completion_tokens: 20, total_tokens: 30 },
            latency: std::time::Duration::from_millis(50),
            provider: "mock".to_string(),
            model: "mock-model".to_string(),
            confidence: Some(0.9),
            warnings: vec![],
            tool_calls: None,
            metadata: std::collections::HashMap::new(),
            created_at: neo_core::time::Timestamp::now(),
        })
    }
    async fn stream(&self, _config: &GenerationConfig, _cancel: CancellationToken) -> LanguageResult<tokio::sync::mpsc::Receiver<StreamChunk>> {
        let (_tx, rx) = tokio::sync::mpsc::channel(1);
        Ok(rx)
    }
    async fn count_tokens(&self, text: &str, _model: &str) -> LanguageResult<usize> { Ok(text.len() / 4) }
    async fn estimate_context_size(&self, messages: &[Message], _model: &str) -> LanguageResult<usize> { Ok(messages.len() * 100) }
    async fn capabilities(&self) -> LanguageResult<ProviderCapabilities> {
        Ok(ProviderCapabilities { streaming: true, ..Default::default() })
    }
    async fn cancel_generation(&self, _id: GenerationId) -> LanguageResult<()> { Ok(()) }
    async fn metrics(&self) -> LanguageResult<ProviderMetrics> { Ok(ProviderMetrics::default()) }
    async fn list_models(&self) -> LanguageResult<Vec<ModelInfo>> { Ok(vec![]) }
    async fn is_model_loaded(&self, _name: &str) -> LanguageResult<bool> { Ok(false) }
}
