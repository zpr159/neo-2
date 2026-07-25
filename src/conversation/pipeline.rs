use std::sync::Arc;
use tokio::sync::mpsc;

use crate::conversation::config::ConversationConfig;
use crate::conversation::error::{ConversationError, ConversationResult};
use crate::conversation::executive_bridge::{ExecutiveConversationBridge, ExecutiveDecision};
use crate::conversation::planning_bridge::PlanningConversationBridge;
use crate::conversation::reasoning_bridge::ReasoningConversationBridge;
use crate::conversation::memory_bridge::{MemoryConversationBridge, MemoryConsolidationItem};
use crate::conversation::knowledge_bridge::KnowledgeConversationBridge;
use crate::conversation::world_model_bridge::WorldModelConversationBridge;
use crate::conversation::workflow_bridge::WorkflowConversationBridge;
use crate::conversation::agent_bridge::AgentConversationBridge;
use crate::conversation::retrieval_coordinator::RetrievalCoordinator;
use crate::conversation::prompt_builder::PromptBuilder;
use crate::conversation::response_validator::ResponseValidator;
use crate::conversation::tool_coordinator::{
    ToolCoordinator, ToolExecutionRequest, ToolExecutionResult, ToolExecutionStatus,
};
use crate::conversation::types::*;
use crate::language::engine::LanguageEngine;
use crate::language::types::{GenerationConfig, GenerationResponse, Message, ToolCall};

/// Events emitted by the pipeline during processing.
#[derive(Debug, Clone)]
pub enum PipelineEvent {
    IntentClassified(Intent),
    ExecutiveDecisionMade(ExecutiveDecision),
    PlanningComplete(usize),
    ReasoningComplete(String),
    MemoryRetrieved(usize),
    KnowledgeRetrieved(usize),
    WorldModelQueried(usize),
    ContextAssembled(usize),
    PromptBuilt,
    Generating,
    ToolExecutionStarted(String),
    ToolExecutionComplete(String, bool),
    ResponseValidated,
    MemoryConsolidated,
    Error(String),
}

/// The ConversationPipeline orchestrates the complete cognitive execution graph.
///
/// ```text
/// User → ConversationManager → ConversationPipeline →
///   Executive → Planning → Reasoning → Memory → Knowledge →
///   World Model → Workflow → Agent → RetrievalCoordinator →
///   ContextAssembler → PromptBuilder → LanguageEngine →
///   ResponseValidator → ToolExecution → MemoryConsolidation → User
/// ```
pub struct ConversationPipeline {
    config: ConversationConfig,
    executive: Arc<dyn ExecutiveConversationBridge>,
    planning: Arc<dyn PlanningConversationBridge>,
    reasoning: Arc<dyn ReasoningConversationBridge>,
    memory: Arc<dyn MemoryConversationBridge>,
    knowledge: Arc<dyn KnowledgeConversationBridge>,
    world_model: Arc<dyn WorldModelConversationBridge>,
    workflow: Arc<dyn WorkflowConversationBridge>,
    agent: Arc<dyn AgentConversationBridge>,
    language_engine: Arc<dyn LanguageEngine>,
    tool_coordinator: Arc<ToolCoordinator>,
    retrieval_coordinator: Arc<RetrievalCoordinator>,
    prompt_builder: Arc<PromptBuilder>,
    response_validator: Arc<ResponseValidator>,
}

impl ConversationPipeline {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config: ConversationConfig,
        executive: Arc<dyn ExecutiveConversationBridge>,
        planning: Arc<dyn PlanningConversationBridge>,
        reasoning: Arc<dyn ReasoningConversationBridge>,
        memory: Arc<dyn MemoryConversationBridge>,
        knowledge: Arc<dyn KnowledgeConversationBridge>,
        world_model: Arc<dyn WorldModelConversationBridge>,
        workflow: Arc<dyn WorkflowConversationBridge>,
        agent: Arc<dyn AgentConversationBridge>,
        language_engine: Arc<dyn LanguageEngine>,
        tool_coordinator: Arc<ToolCoordinator>,
    ) -> Self {
        let retrieval_coordinator = Arc::new(RetrievalCoordinator::new(
            config.ranking_config.clone(),
        ));
        let prompt_builder = Arc::new(PromptBuilder::new());
        let response_validator = Arc::new(ResponseValidator::new());

        Self {
            config,
            executive,
            planning,
            reasoning,
            memory,
            knowledge,
            world_model,
            workflow,
            agent,
            language_engine,
            tool_coordinator,
            retrieval_coordinator,
            prompt_builder,
            response_validator,
        }
    }

    /// Process a complete user interaction through the full cognitive pipeline.
    pub async fn process(
        &self,
        context: &mut ConversationContext,
        user_message: &str,
        event_tx: Option<mpsc::Sender<PipelineEvent>>,
    ) -> ConversationResult<ConversationResponse> {
        let emit = |event: PipelineEvent| {
            if let Some(ref tx) = event_tx {
                let tx = tx.clone();
                tokio::spawn(async move {
                    let _ = tx.send(event).await;
                });
            }
        };

        // 1. Intent Classification via Executive
        emit(PipelineEvent::Generating);
        let objective = user_message.to_string();

        let decision = self
            .executive
            .process_objective(context, &objective)
            .await
            .map_err(|e| ConversationError::IntentClassificationFailed(e.to_string()))?;

        context.intent = Some(decision.intent.clone());
        context.urgency = decision.urgency;
        context.classification = Some(decision.classification.clone());
        context.execution_policy = Some(decision.execution_policy);
        context.reasoning_depth = decision.reasoning_depth;
        context.tool_authorizations = decision.tool_authorizations.clone();

        emit(PipelineEvent::IntentClassified(decision.intent.clone()));
        emit(PipelineEvent::ExecutiveDecisionMade(decision.clone()));

        // 2. Planning (if required)
        if matches!(
            decision.classification,
            RequestClassification::ComplexQuery | RequestClassification::MultiStepTask
        ) {
            let plan = self
                .planning
                .generate_plan(context, &objective)
                .await
                .map_err(|e| ConversationError::PlanningFailed(e.to_string()))?;

            emit(PipelineEvent::PlanningComplete(plan.subtasks.len()));
        }

        // 3. Reasoning (if depth > None)
        if !matches!(decision.reasoning_depth, ReasoningDepth::None) {
            let propositions: Vec<String> = context
                .messages
                .iter()
                .filter(|m| m.role == crate::language::types::MessageRole::User)
                .map(|m| m.content.clone())
                .collect();

            let _reasoning_result = self
                .reasoning
                .consistency_check(context, &propositions)
                .await
                .map_err(|e| ConversationError::ReasoningFailed(e.to_string()))?;

            emit(PipelineEvent::ReasoningComplete("consistency checked".to_string()));
        }

        // 4. Memory Retrieval
        if self.config.enable_memory_retrieval {
            let query = crate::conversation::memory_bridge::MemoryQuery {
                text: objective.clone(),
                limit: 20,
                confidence_threshold: self.config.confidence_threshold,
                ..Default::default()
            };
            match self.memory.retrieve(context, &query).await {
                Ok(result) => {
                    emit(PipelineEvent::MemoryRetrieved(result.total_retrieved));
                }
                Err(e) => {
                    emit(PipelineEvent::Error(format!("memory retrieval: {}", e)));
                }
            }
        }

        // 5. Knowledge Graph Retrieval
        if self.config.enable_knowledge_retrieval {
            match self.knowledge.retrieve_evidence(context, &objective, 10).await {
                Ok(evidence) => {
                    emit(PipelineEvent::KnowledgeRetrieved(evidence.len()));
                }
                Err(e) => {
                    emit(PipelineEvent::Error(format!("knowledge retrieval: {}", e)));
                }
            }
        }

        // 6. World Model Query
        if self.config.enable_world_model {
            match self.world_model.query_evidence(context, &objective).await {
                Ok(evidence) => {
                    emit(PipelineEvent::WorldModelQueried(evidence.len()));
                }
                Err(e) => {
                    emit(PipelineEvent::Error(format!("world model: {}", e)));
                }
            }
        }

        // 7. Retrieve and assemble unified cognitive context
        let cognitive = self
            .retrieval_coordinator
            .retrieve(
                context,
                &objective,
                self.memory.as_ref(),
                self.knowledge.as_ref(),
                self.world_model.as_ref(),
                self.planning.as_ref(),
                self.reasoning.as_ref(),
                self.executive.as_ref(),
                self.agent.as_ref(),
                self.workflow.as_ref(),
            )
            .await
            .map_err(|e| ConversationError::ContextAssemblyFailed(e.to_string()))?;

        emit(PipelineEvent::ContextAssembled(cognitive.ranked_evidence.len()));

        // 8. Build prompt
        let tool_defs = self.tool_coordinator.to_tool_definitions().await;
        let built_prompt = self
            .prompt_builder
            .build(context, &cognitive, &tool_defs, &[])
            .map_err(|e| ConversationError::PromptBuildFailed(e.to_string()))?;

        emit(PipelineEvent::PromptBuilt);

        // 9. Generate response via language engine
        let mut messages = vec![built_prompt.system_message.clone()];
        messages.extend(built_prompt.context_messages.clone());
        messages.push(Message::user(user_message));

        let gen_config = GenerationConfig {
            messages,
            max_tokens: self.config.max_tokens_per_response,
            tools: if tool_defs.is_empty() {
                None
            } else {
                Some(tool_defs)
            },
            ..Default::default()
        };

        let response = self
            .language_engine
            .generate(&gen_config)
            .await
            .map_err(|e| ConversationError::ProviderError(e.to_string()))?;

        // 10. Handle tool calls if present
        if let Some(ref tool_calls) = response.tool_calls {
            let mut tool_results = Vec::new();
            for tc in tool_calls {
                emit(PipelineEvent::ToolExecutionStarted(tc.function.name.clone()));

                let args: serde_json::Value = serde_json::from_str(&tc.function.arguments)
                    .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

                let request = ToolExecutionRequest {
                    tool_name: tc.function.name.clone(),
                    arguments: args,
                    timeout_ms: Some(self.tool_config().default_timeout_ms),
                    retries: None,
                    chain_id: None,
                };

                match self
                    .tool_coordinator
                    .execute_tool(context, &request, self.executive.as_ref())
                    .await
                {
                    Ok(result) => {
                        let success = result.status == ToolExecutionStatus::Success;
                        emit(PipelineEvent::ToolExecutionComplete(
                            tc.function.name.clone(),
                            success,
                        ));
                        tool_results.push(result);
                    }
                    Err(e) => {
                        emit(PipelineEvent::Error(format!("tool execution: {}", e)));
                    }
                }
            }

            // If tools were executed, do a follow-up generation with results
            if !tool_results.is_empty() {
                let tool_messages: Vec<Message> = tool_results
                    .iter()
                    .map(|r| Message::tool(
                        serde_json::to_string(&r.output).unwrap_or_default(),
                        &r.tool_name,
                    ))
                    .collect();

                let follow_up_config = GenerationConfig {
                    messages: {
                        let mut msgs = vec![built_prompt.system_message];
                        msgs.extend(built_prompt.context_messages);
                        msgs.push(Message::user(user_message));
                        msgs.extend(tool_messages);
                        msgs
                    },
                    max_tokens: self.config.max_tokens_per_response,
                    ..Default::default()
                };

                match self.language_engine.generate(&follow_up_config).await {
                    Ok(follow_up_response) => {
                        return self.finalize_response(
                            context,
                            follow_up_response,
                            Some(tool_results),
                            emit,
                        )
                        .await;
                    }
                    Err(e) => {
                        return Err(ConversationError::ProviderError(e.to_string()));
                    }
                }
            }
        }

        // 11. Validate and refine response
        self.finalize_response(context, response, None, emit).await
    }

    async fn finalize_response(
        &self,
        context: &mut ConversationContext,
        response: GenerationResponse,
        tool_results: Option<Vec<ToolExecutionResult>>,
        emit: impl Fn(PipelineEvent),
    ) -> ConversationResult<ConversationResponse> {
        // Validate
        let validated = self
            .response_validator
            .validate(&response, context, None)
            .map_err(|e| ConversationError::ResponseValidationFailed(e.to_string()))?;

        emit(PipelineEvent::ResponseValidated);

        // Build response message
        let mut response_msg = ConversationMessage::assistant(validated.text);
        response_msg.tool_calls = response.tool_calls;
        response_msg
            .metadata
            .insert("validated".to_string(), "true".to_string());
        if !validated.warnings.is_empty() {
            response_msg.metadata.insert(
                "warnings".to_string(),
                validated.warnings.join("; "),
            );
        }

        context.push_message(response_msg.clone());

        // Memory consolidation (async, non-blocking)
        if self.config.auto_consolidate_memory {
            let memory = self.memory.clone();
            let ctx_clone = context.clone();
            let user_msg = user_message_for_consolidation(context);

            tokio::spawn(async move {
                let items = vec![MemoryConsolidationItem {
                    content: user_msg,
                    memory_type: crate::conversation::memory_bridge::MemoryType::Episodic,
                    importance: 0.5,
                    context: std::collections::HashMap::new(),
                    source_conversation_id: ctx_clone.conversation_id,
                    timestamp: crate::time::Timestamp::now(),
                }];

                let _ = memory.consolidate(&ctx_clone, &items).await;
            });

            emit(PipelineEvent::MemoryConsolidated);
        }

        Ok(ConversationResponse {
            conversation_id: context.conversation_id,
            message: response_msg,
            tool_calls: tool_results.map(|results| {
                results
                    .iter()
                    .map(|r| ToolCall {
                        id: r.tool_name.clone(),
                        call_type: "function".to_string(),
                        function: crate::language::types::FunctionCall {
                            name: r.tool_name.clone(),
                            arguments: serde_json::to_string(&r.output)
                                .unwrap_or_default(),
                        },
                    })
                    .collect()
            }),
            requires_continuation: false,
            metadata: std::collections::HashMap::new(),
        })
    }

    pub fn config(&self) -> &ConversationConfig {
        &self.config
    }

    pub fn tool_config(&self) -> &crate::conversation::config::ToolConfig {
        &self.config.tool_config
    }
}

fn user_message_for_consolidation(context: &ConversationContext) -> String {
    context
        .messages
        .iter()
        .rev()
        .find(|m| m.role == crate::language::types::MessageRole::User)
        .map(|m| m.content.clone())
        .unwrap_or_default()
}
