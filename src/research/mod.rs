//! # Research Subsystem
//!
//! Autonomous research capability for Neo: acquiring, validating, organizing,
//! and synthesizing knowledge with traceable evidence.
//!
//! ## Research Pipeline
//!
//! ```text
//! Objective
//!   ↓
//! Planning
//!   ↓
//! Search
//!   ↓
//! Retrieval (Fetching)
//!   ↓
//! Extraction
//!   ↓
//! Validation
//!   ↓
//! Ranking
//!   ↓
//! Evidence Assembly
//!   ↓
//! Synthesis
//!   ↓
//! Knowledge Graph Update
//!   ↓
//! World Model Update
//!   ↓
//! Memory Update
//!   ↓
//! Final Report
//! ```
//!
//! ## Architecture
//!
//! The research subsystem is composed of the following modules:
//!
//! - **config**: Configuration for all research subsystems
//! - **error**: Research-specific error types
//! - **api**: Core types (tasks, findings, citations, evidence)
//! - **search**: Search provider traits and implementations
//! - **crawler**: Multi-provider search coordination
//! - **fetcher**: Asynchronous content retrieval (HTML, PDF, JSON, XML, etc.)
//! - **extractor**: Entity, relationship, event, date, location, citation, fact extraction
//! - **validator**: Fact validation with provenance and confidence scoring
//! - **ranking**: Composite scoring and diversity-aware ranking
//! - **deduplication**: Exact, fuzzy, and semantic deduplication
//! - **citation**: Citation generation, formatting, and management
//! - **synthesis**: Finding merging, contradiction detection, summary generation
//! - **evidence**: Research evidence collection and management
//! - **knowledge_update**: Knowledge Graph update proposals with governance
//! - **world_update**: World Model update proposals with governance
//! - **memory_update**: Memory update proposals with governance
//! - **planner**: Research plan generation from requests
//! - **workflow**: Full pipeline execution engine
//! - **manager**: Top-level ResearchManager (Component implementation)
//! - **metrics**: Research metrics collection and snapshots
//! - **integration**: Bridges to Conversation, Executive, Planning, Reasoning,
//!   Knowledge Graph, World Model, and Memory subsystems

pub mod config;
pub mod error;
pub mod api;
pub mod search;
pub mod crawler;
pub mod fetcher;
pub mod extractor;
pub mod validator;
pub mod ranking;
pub mod deduplication;
pub mod citation;
pub mod synthesis;
pub mod evidence;
pub mod knowledge_update;
pub mod world_update;
pub mod memory_update;
pub mod planner;
pub mod workflow;
pub mod manager;
pub mod metrics;
pub mod integration;

pub use config::ResearchConfig;
pub use error::{ResearchError, ResearchErrorCode, ResearchResult};
pub use api::{
    Citation, Finding, RankedFinding, ResearchContradiction, ResearchEvidence,
    ResearchProvenance, ResearchRequest, ResearchOutput,
    ResearchTask, ResearchTaskId, ResearchTaskMetrics, ResearchTaskStatus,
    ValidatedFact,
};
pub use manager::{ResearchManager, ResearchSubsystemMetrics};
pub use metrics::{MetricsSnapshot, ResearchMetrics, SharedResearchMetrics};
pub use integration::{
    MockResearchBridge, NeoResearchBridge, ResearchConversationBridge,
    ResearchIntegration,
};
