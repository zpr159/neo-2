//! # Conversation Subsystem
//!
//! Orchestrates Neo's cognitive systems into a coherent response generation pipeline.
//!
//! The Conversation subsystem is not responsible for cognition. Its responsibility
//! is to orchestrate existing cognitive systems into a coherent pipeline.
//!
//! ## Cognitive Pipeline
//!
//! ```text
//! User
//!   ↓
//! ConversationManager
//!   ↓
//! ConversationPipeline
//!   ↓
//! Executive → Planning → Reasoning → Memory → Knowledge Graph
//!   ↓
//! World Model → Workflow Engine → Agent Framework
//!   ↓
//! RetrievalCoordinator → ContextAssembler → PromptBuilder
//!   ↓
//! Language Engine → Response Validation
//!   ↓
//! Tool Execution (if required) → Memory Consolidation
//!   ↓
//! User
//! ```
//!
//! ## Architecture
//!
//! Every stage is observable, measurable, and independently testable.
//! All cognitive bridges are traits, allowing mock implementations for testing
//! and real implementations when subsystems come online.
//!
//! No subsystem operates independently. Every response is synthesized from
//! multiple cognitive sources.

pub mod config;
pub mod error;
pub mod types;
pub mod evidence;
pub mod executive_bridge;
pub mod planning_bridge;
pub mod reasoning_bridge;
pub mod memory_bridge;
pub mod knowledge_bridge;
pub mod world_model_bridge;
pub mod workflow_bridge;
pub mod agent_bridge;
pub mod context_ranker;
pub mod context_merger;
pub mod retrieval_coordinator;
pub mod tool_coordinator;
pub mod prompt_builder;
pub mod response_validator;
pub mod pipeline;
pub mod manager;

pub use config::{
    ConversationConfig, RankingConfig, ToolConfig, DistributedConversationConfig,
};
pub use error::{ConversationError, ConversationErrorCode, ConversationResult};
pub use types::{
    ConversationContext, ConversationId, ConversationMessage, ConversationMetrics,
    ConversationResponse, Intent, ExecutionPolicy, RequestClassification, ResponseFormat,
    ReasoningDepth, Urgency, SessionId,
};
pub use evidence::{
    Evidence, EvidenceCollection, EvidenceSource, Provenance, ProvenanceStep,
};
pub use executive_bridge::{
    ExecutiveConversationBridge, ExecutiveDecision, MockExecutiveBridge,
};
pub use planning_bridge::{
    PlanningConversationBridge, PlanningContext, MockPlanningBridge,
};
pub use reasoning_bridge::{
    ReasoningConversationBridge, ReasoningResult, MockReasoningBridge,
};
pub use memory_bridge::{
    MemoryConversationBridge, MemoryQuery, MemoryRetrievalResult, MemoryType,
    RetrievalMethod, MockMemoryBridge,
};
pub use knowledge_bridge::{
    KnowledgeConversationBridge, KnowledgeResult, MockKnowledgeBridge,
};
pub use world_model_bridge::{
    WorldModelConversationBridge, WorldState, MockWorldModelBridge,
};
pub use workflow_bridge::{
    WorkflowConversationBridge, WorkflowInfo, WorkflowStatus, MockWorkflowBridge,
};
pub use agent_bridge::{
    AgentConversationBridge, AgentInfo, AgentStatus, MockAgentBridge,
};
pub use context_ranker::{ContextRanker, RankedEvidence};
pub use context_merger::{ContextMerger, UnifiedContext};
pub use retrieval_coordinator::{RetrievalCoordinator, CognitiveContext};
pub use tool_coordinator::{
    ToolCoordinator, ToolCapability, ToolChain, ToolChainResult,
    ToolExecutionRequest, ToolExecutionResult, ToolExecutionStatus,
};
pub use prompt_builder::{PromptBuilder, BuiltPrompt};
pub use response_validator::{ResponseValidator, ValidatedResponse};
pub use pipeline::{ConversationPipeline, PipelineEvent};
pub use manager::ConversationManager;
