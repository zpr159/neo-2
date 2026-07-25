use std::sync::Arc;
use std::time::Instant;

use crate::context::ContextManager;
use crate::error::ConversationResult;
use crate::language::{FinishReason, GenerateRequest, LanguageEngine};
use crate::metrics::ConversationMetrics;
use crate::session::ConversationSession;
use crate::stream::ResponseStreamer;
use crate::tools::ToolBridge;
use crate::types::{
    CognitiveContext, StreamChunk, TokenUsage, ToolCall,
};

/// The conversation pipeline orchestrates the full cognitive-to-language flow:
///
/// ```text
/// User Input
///     ↓
/// Executive (task scheduling)
///     ↓
/// Planning (task decomposition)
///     ↓
/// Reasoning (chain-of-thought)
///     ↓
/// Memory (relevant recall)
///     ↓
/// Knowledge Graph (fact retrieval)
///     ↓
/// World Model (environmental context)
///     ↓
/// Workflow Engine (automation)
///     ↓
/// Agent Framework (autonomous agents)
///     ↓
/// Context Builder (merge & rank)
///     ↓
/// Prompt Builder (construct prompt)
///     ↓
/// Language Engine (LLM generation)
///     ↓
/// Response Validator (quality checks)
///     ↓
/// Streaming (delivery)
/// ```
pub struct ConversationPipeline {
    context_manager: Arc<ContextManager>,
    tool_bridge: Arc<ToolBridge>,
    streamer: Arc<ResponseStreamer>,
    metrics: ConversationMetrics,
}

impl ConversationPipeline {
    #[must_use]
    pub fn new(config: &crate::types::SessionConfig) -> Self {
        Self {
            context_manager: Arc::new(ContextManager::new(config)),
            tool_bridge: Arc::new(ToolBridge::new()),
            streamer: Arc::new(ResponseStreamer::new()),
            metrics: ConversationMetrics::new(),
        }
    }

    pub fn with_metrics(config: &crate::types::SessionConfig, metrics: ConversationMetrics) -> Self {
        Self {
            context_manager: Arc::new(ContextManager::new(config)),
            tool_bridge: Arc::new(ToolBridge::new()),
            streamer: Arc::new(ResponseStreamer::new()),
            metrics,
        }
    }

    /// Process a complete conversation turn (non-streaming).
    pub async fn process_turn(
        &self,
        session: &mut ConversationSession,
        user_input: &str,
        engine: &dyn LanguageEngine,
    ) -> ConversationResult<ConversationResponse> {
        // 1. Record user message.
        session.add_user_message(user_input);
        self.metrics.message_sent();

        // 2. Gather cognitive context (in production, calls all subsystems).
        let context_start = Instant::now();
        let cognitive_context = self.gather_cognitive_context(session, user_input).await?;
        session.set_cognitive_context(cognitive_context);
        self.metrics.record_context_assembly_time(context_start.elapsed());

        // 3. Build prompt from session + cognitive context.
        let mut messages = self.context_manager.build_prompt(session);
        self.context_manager
            .truncate_to_budget(&mut messages, session.config.max_context_tokens);

        // 4. Generate response.
        let request = GenerateRequest {
            messages,
            max_tokens: session.config.max_generation_tokens,
            temperature: session.config.temperature,
            top_p: session.config.top_p,
            stop: session.config.stop_sequences.clone(),
            stream: false,
        };

        let provider_start = Instant::now();
        let response = engine.generate(&request).await?;
        self.metrics.record_provider_latency(provider_start.elapsed());

        // 5. Record tokens.
        session.record_tokens(response.usage.prompt_tokens, response.usage.completion_tokens);
        self.metrics.tokens_used(response.usage.total_tokens);

        // 6. Check for tool calls.
        let tool_calls = self.tool_bridge.extract_tool_calls(&response.text);

        if !tool_calls.is_empty() {
            return self
                .handle_tool_calls(session, tool_calls, engine)
                .await;
        }

        // 7. Record assistant response.
        let assistant_text = response.text.clone();
        session.add_assistant_message(&assistant_text);

        Ok(ConversationResponse {
            text: assistant_text,
            usage: response.usage,
            tool_calls,
            finish_reason: response.finish_reason,
        })
    }

    /// Process a streaming conversation turn.
    pub async fn process_turn_streaming(
        &self,
        session: &mut ConversationSession,
        user_input: &str,
        engine: &dyn LanguageEngine,
    ) -> ConversationResult<tokio::sync::mpsc::Receiver<ConversationResult<StreamChunk>>> {
        // 1. Record user message.
        session.add_user_message(user_input);
        self.metrics.message_sent();

        // 2. Gather cognitive context.
        let cognitive_context = self.gather_cognitive_context(session, user_input).await?;
        session.set_cognitive_context(cognitive_context);

        // 3. Build prompt.
        let mut messages = self.context_manager.build_prompt(session);
        self.context_manager
            .truncate_to_budget(&mut messages, session.config.max_context_tokens);

        // 4. Generate streaming response.
        let request = GenerateRequest {
            messages,
            max_tokens: session.config.max_generation_tokens,
            temperature: session.config.temperature,
            top_p: session.config.top_p,
            stop: session.config.stop_sequences.clone(),
            stream: true,
        };

        let rx = engine.generate_stream(&request, session.id.clone()).await?;

        // 5. Wrap with response streamer to accumulate and record.
        let session_id = session.id.clone();
        let (wrapped_tx, wrapped_rx) = tokio::sync::mpsc::channel(256);
        let streamer = self.streamer.clone();
        let metrics = self.metrics.clone();

        tokio::spawn(async move {
            let mut accumulated = String::new();
            let mut rx = rx;

            while let Some(chunk) = rx.recv().await {
                match chunk {
                    Ok(chunk) => {
                        accumulated.push_str(&chunk.text);
                        metrics.stream_chunk_sent();
                        let _ = wrapped_tx.send(Ok(chunk)).await;
                    }
                    Err(e) => {
                        let _ = wrapped_tx.send(Err(e)).await;
                        return;
                    }
                }
            }

            streamer
                .record_completed(&session_id, &accumulated)
                .await;
        });

        Ok(wrapped_rx)
    }

    /// Gather cognitive context from all subsystems.
    async fn gather_cognitive_context(
        &self,
        session: &ConversationSession,
        query: &str,
    ) -> ConversationResult<CognitiveContext> {
        let _ = (session, query);
        // In production, this calls each cognitive subsystem:
        // - Memory: retrieve relevant memories
        // - Knowledge: query knowledge graph
        // - Reasoning: chain-of-thought analysis
        // - World Model: current observations
        // - Planning: active plan context
        // - Executive: recent decisions
        // - Agents: agent outputs
        // - Workflows: workflow outputs
        Ok(CognitiveContext::empty())
    }

    /// Handle tool calls within a conversation turn.
    async fn handle_tool_calls(
        &self,
        session: &mut ConversationSession,
        tool_calls: Vec<ToolCall>,
        engine: &dyn LanguageEngine,
    ) -> ConversationResult<ConversationResponse> {
        let tool_results = self.tool_bridge.execute_tools(&tool_calls).await;
        self.metrics.tool_call_recorded();

        for result in &tool_results {
            let content = if result.success {
                format!("Tool '{}' result: {}", result.name, result.result)
            } else {
                format!(
                    "Tool '{}' failed: {}",
                    result.name,
                    result.error.as_deref().unwrap_or("unknown error")
                )
            };
            session.add_tool_result(content);
        }

        session.set_cognitive_context(CognitiveContext {
            tool_results,
            ..CognitiveContext::empty()
        });

        let mut followup_messages = self.context_manager.build_prompt(session);
        self.context_manager
            .truncate_to_budget(&mut followup_messages, session.config.max_context_tokens);

        let followup_request = GenerateRequest {
            messages: followup_messages,
            max_tokens: session.config.max_generation_tokens,
            temperature: session.config.temperature,
            top_p: session.config.top_p,
            stop: session.config.stop_sequences.clone(),
            stream: false,
        };

        let provider_start = Instant::now();
        let followup_response = engine.generate(&followup_request).await?;
        self.metrics.record_provider_latency(provider_start.elapsed());

        session.record_tokens(
            followup_response.usage.prompt_tokens,
            followup_response.usage.completion_tokens,
        );
        self.metrics
            .tokens_used(followup_response.usage.total_tokens);

        let assistant_text = followup_response.text.clone();
        session.add_assistant_message(&assistant_text);

        Ok(ConversationResponse {
            text: assistant_text,
            usage: followup_response.usage,
            tool_calls,
            finish_reason: followup_response.finish_reason,
        })
    }

    #[must_use]
    pub fn context_manager(&self) -> &ContextManager {
        &self.context_manager
    }

    #[must_use]
    pub fn tool_bridge(&self) -> &ToolBridge {
        &self.tool_bridge
    }

    #[must_use]
    pub fn metrics(&self) -> &ConversationMetrics {
        &self.metrics
    }
}

/// Response from a conversation turn.
#[derive(Debug)]
pub struct ConversationResponse {
    pub text: String,
    pub usage: TokenUsage,
    pub tool_calls: Vec<ToolCall>,
    pub finish_reason: FinishReason,
}
