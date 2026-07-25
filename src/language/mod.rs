//! # Language Engine
//!
//! Universal language engine for the Neo AGI Operating System.
//!
//! This module provides a provider-independent interface for language model
//! inference. It handles generation, streaming, token accounting, model
//! lifecycle, provider failover, and load balancing.
//!
//! ## Architecture
//!
//! The LanguageEngine trait is the core abstraction. All providers implement
//! this trait. No subsystem above this trait knows which provider is being used.
//!
//! ```text
//! Neo Cognitive Systems
//!        ↓
//! Conversation Pipeline
//!        ↓
//!    Prompt Builder
//!        ↓
//!   LanguageEngine Trait
//!        ↓
//!  Provider Implementation
//!        ↓
//!    Language Model
//!        ↓
//!  Generated Tokens
//!        ↓
//!  Response Validator
//!        ↓
//!    Streaming
//!        ↓
//!      User
//! ```

pub mod config;
pub mod engine;
pub mod error;
pub mod failover;
pub mod loadbalancer;
pub mod metrics;
pub mod model;
pub mod providers;
pub mod registry;
pub mod token;
pub mod types;

pub use config::{LanguageEngineConfig, ProviderConfig, ProviderType};
pub use engine::{LanguageEngine, ProviderCapabilities};
pub use error::{LanguageError, LanguageErrorCode, LanguageResult};
pub use failover::FailoverManager;
pub use loadbalancer::LoadBalancer;
pub use metrics::{LanguageEngineMetrics, MetricsCollector};
pub use model::{ModelManager, ModelState};
pub use registry::{ProviderDescriptor, ProviderRegistry};
pub use token::{TokenCounter, TokenEstimator, TokenizerAdapter};
pub use types::{
    CancellationToken, FinishReason, GenerationConfig, GenerationId, GenerationResponse,
    Message, MessageRole, ModelCapabilities, ModelInfo, ProviderHealth, ProviderMetrics,
    StreamChunk, StreamId, TokenUsage, ToolCall, ToolDefinition,
};
