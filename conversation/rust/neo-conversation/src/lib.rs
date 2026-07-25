//! # Neo Conversation
//!
//! Natural conversation system for the Neo AGI Operating System.
//!
//! This crate provides Neo with conversational capabilities, making Neo the
//! conversational front-end to all its cognitive subsystems (memory, reasoning,
//! knowledge graph, planning, executive, world model).
//!
//! ## Architecture
//!
//! The conversation system follows a cognitive pipeline architecture:
//!
//! ```text
//! User Input
//!     ↓
//! Executive (task scheduling)
//!     ↓
//! Planning (task decomposition)
//!     ↓
//! Reasoning (chain-of-thought)
//!     ↓
//! Memory (relevant recall)
//!     ↓
//! Knowledge Graph (fact retrieval)
//!     ↓
//! World Model (environmental context)
//!     ↓
//! Workflow Engine (automation)
//!     ↓
//! Agent Framework (autonomous agents)
//!     ↓
//! Context Builder (merge & rank)
//!     ↓
//! Prompt Builder (construct prompt)
//!     ↓
//! Language Engine (LLM generation)
//!     ↓
//! Response Validator (quality checks)
//!     ↓
//! Streaming (delivery)
//!     ↓
//! Human-readable Response
//! ```
//!
//! ## Language Engine Abstraction
//!
//! The `LanguageEngine` trait makes the underlying LLM completely replaceable.
//! Initially backed by Ollama, it can be swapped for any backend through
//! configuration alone:
//!
//! - `OllamaProvider` — Ollama REST API (initial default)
//! - `LlamaCppProvider` — llama.cpp HTTP server
//! - `OpenAiProvider` — OpenAI-compatible API
//! - `AnthropicProvider` — Anthropic Claude API
//! - `DeepSeekProvider` — DeepSeek API
//! - `NeoLmProvider` — Neo's own inference layer
//!
//! ## Key Design Principle
//!
//! The language model is the final translator in Neo's cognitive chain:
//!
//! ```text
//! Planning → Reasoning → Memory → Knowledge → Executive
//!     → World Model → Agents → Tools → Language Model → Human
//! ```
//!
//! Neo performs all higher-level cognition before invoking the model.

pub mod agents;
pub mod config;
pub mod context;
pub mod error;
pub mod executive;
pub mod history;
pub mod integration;
pub mod knowledge;
pub mod language;
pub mod manager;
pub mod memory;
pub mod metrics;
pub mod persistence;
pub mod pipeline;
pub mod planner;
pub mod prompt;
pub mod providers;
pub mod reasoning;
pub mod retrieval;
pub mod session;
pub mod stream;
pub mod tools;
pub mod types;
pub mod world;
pub mod workflow;

pub use config::ConversationConfig;
pub use context::{ContextAssembler, ContextManager};
pub use error::{ConversationError, ConversationResult};
pub use language::{
    FinishReason, GenerateRequest, GenerateResponse, LanguageBackendType, LanguageEngine,
    LanguageEngineConfig, LanguageEngineInfo, OllamaEngine,
};
pub use manager::ConversationManager;
pub use metrics::{ConversationMetrics, MetricsSnapshot, SessionMetrics};
pub use pipeline::{ConversationPipeline, ConversationResponse};
pub use prompt::{PromptBuilder, PromptTemplate};
pub use session::ConversationSession;
pub use stream::{ResponseStreamer, StreamAccumulator, StreamAccumulatorWrapper};
pub use tools::{ConversationTool, KeyValueTool, ToolBridge};
pub use types::{
    CognitiveContext, CognitiveSource, ConversationMessage, ConversationMode, LlmMessage,
    MessageId, MessageRole, SessionConfig, SessionId, StreamChunk, TokenUsage, ToolCall,
    ToolDefinition, ToolResult, UserId, UserModel,
};

pub use history::ConversationHistory;
pub use persistence::{InMemoryPersistence, PersistenceBackend, SessionData};
pub use planner::{DefaultPlanner, PlanResult, PlanningInterface};
pub use reasoning::{DefaultReasoner, ReasoningInterface, ReasoningResult};
pub use memory::{DefaultMemory, MemoryInterface, MemoryResult};
pub use knowledge::{DefaultKnowledge, KnowledgeInterface};
pub use world::{DefaultWorldModel, WorldModelInterface};
pub use executive::{DefaultExecutive, ExecutiveInterface};
pub use workflow::{DefaultWorkflow, WorkflowInterface};
pub use agents::{AgentInterface, DefaultAgentInterface};
pub use retrieval::RetrievalCoordinator;
pub use integration::ConversationIntegration;
pub use providers::{
    AnthropicProvider, DeepSeekProvider, LlamaCppProvider, NeoLmProvider, OpenAiProvider,
    OllamaProvider,
};
