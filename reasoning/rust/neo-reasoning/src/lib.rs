//! # Neo Reasoning Engine
//!
//! Multi-paradigm reasoning engine for the Neo AGI Operating System.
//!
//! Provides deductive, inductive, abductive, analogical, probabilistic,
//! causal, counterfactual, constraint-based, and rule-based reasoning strategies.
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────┐
//! │                  Reasoning Orchestrator                       │
//! │  (session management, pipeline, state machine, execution)    │
//! ├──────────────┬──────────────┬──────────────┬────────────────┤
//! │   Planning   │  Hypothesis  │  Reflection  │   Decision     │
//! │   Engine     │  Engine      │  Engine      │   Engine       │
//! │  (goals,     │  (generate,  │  (self-eval, │  (score,       │
//! │   tasks,     │   rank,      │   verify,    │   risk,        │
//! │   plans)     │   discard)   │   consistent)│   utility)     │
//! ├──────────────┴──────────────┴──────────────┴────────────────┤
//! │            Chain of Thought (Internal Representation)        │
//! │  (hidden reasoning graph, intermediate state, checkpoints)   │
//! ├──────────────────────────────────────────────────────────────┤
//! │  Strategy Registry  │  Knowledge Integration  │  Cache       │
//! │  (9 strategies)     │  (memory, KG, context)  │  (reuse)     │
//! ├─────────────────────┴─────────────────────────┴─────────────┤
//! │  Tool Reasoning  │  Multi-Model  │  Explanation  │ Analytics │
//! └─────────────────┴───────────────┴──────────────┴───────────┘
//! ```

#![allow(missing_docs)]

pub mod error;
pub mod types;
pub mod strategy;
pub mod chain;
pub mod planning;
pub mod reflection;
pub mod hypothesis;
pub mod decision;
pub mod knowledge_integration;
pub mod cache;
pub mod tool_reasoning;
pub mod multi_model;
pub mod explanation;
pub mod analytics;
pub mod orchestrator;
pub mod api;

pub use error::{ReasoningError, ReasoningResult, ReasoningErrorCode};
pub use types::{
    ReasoningSession, SessionState, ReasoningPhase, ExecutionGraph,
    ExecutionNode, NodeStatus, NodePriority, ReasoningConfig, PhaseTransition,
};
pub use strategy::{
    ReasoningStrategy, StrategyContext, StrategyResult,
    StrategyRegistry, ReasoningStrategyExecutor,
    DeductiveStrategy, InductiveStrategy, AbductiveStrategy,
    AnalogicalStrategy, ProbabilisticStrategy, CausalStrategy,
    CounterfactualStrategy, ConstraintBasedStrategy, RuleBasedStrategy,
};
pub use chain::{InternalChain, InternalStep, InternalReasoningState, StepType};
pub use planning::{
    Goal, GoalPriority, Plan, PlanTask, TaskStatus, PlanningEngine,
};
pub use reflection::{ReflectionEngine, ReflectionResult, ReflectionType, ReflectionEntry};
pub use hypothesis::{
    Hypothesis, HypothesisStatus, HypothesisEngine, HypothesisRanking, Evidence,
};
pub use decision::{
    DecisionEngine, DecisionResult, DecisionOption,
    ObjectiveWeight, ScoredOption,
};
pub use knowledge_integration::{
    KnowledgeIntegrator, IntegratedContext, RetrievedContext,
    ContextSource, RankedContext,
};
pub use cache::{ReasoningCache, CachedReasoningResult, CacheStats};
pub use tool_reasoning::{
    ToolReasoner, ToolDescriptor, ToolPlan, ToolPlanStep,
    ToolOutput, ToolPlanResult, ToolType,
};
pub use multi_model::{
    MultiModelReasoner, MultiModelResult, ConsensusResult,
    ConsensusMethod, ModelBackend, ModelResponse, ModelRole,
};
pub use explanation::{ExplanationEngine, Explanation, EvidenceRef};
pub use analytics::{ReasoningAnalytics, ReasoningAnalyticsSnapshot};
pub use orchestrator::{
    ReasoningOrchestrator, ReasoningRequest, ReasoningResponse,
    SessionInfo, SessionSummary, ChainSummary,
};
pub use api::ReasoningApi;
