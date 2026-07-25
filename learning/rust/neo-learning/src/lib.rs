#!\[forbid(unsafe_code)\]
#![deny(
    missing_docs,
    warnings,
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces,
    unused_extern_crates
)]

/// Neo Learning System — autonomous learning, knowledge consolidation, and
/// continuous improvement for Neo AGI OS.
//!
/// This module provides the complete production-grade learning system for Neo AGI OS,
/// enabling the system to learn from experience, evaluate outcomes, refine heuristics,
/// consolidate knowledge, and improve future planning and execution.
//!
/// The learning system operates within explicit safety and governance constraints
/// while continuously improving decision-making through experience-based adaptation.
//!
/// ## Architecture Overview
///
/// The Learning System consists of several interconnected components:
/// - **Experience System**: Captures and stores interactions with the world
/// - **Episodic Memory**: Organizes experiences into coherent episodes
/// - **Reflection Engine**: Analyzes experiences and produces insights
/// - **Knowledge Consolidation**: Extracts reusable knowledge from reflections
/// - **Pattern Discovery**: Identifies recurring patterns and relationships
/// - **Skill Library**: Manages learned skills and capabilities
/// - **Strategy Refinement**: Improves planning and execution heuristics
/// - **Performance Optimization**: Identifies and corrects performance bottlenecks
/// - **Failure Analysis**: Analyzes failures to prevent recurrence
/// - **Learning Policies**: Governs when and how learning occurs
///
/// The system integrates with all other Neo components, feeding learned insights
/// back to the Planning, Runtime, and Executive systems while maintaining
/// strict security and governance controls.

pub mod core;
pub mod experience;
pub mod memory;
pub mod reflection;
pub mod knowledge;
pub mod patterns;
pub mod skills;
pub mod strategy;
pub mod performance;
pub mod failure;
pub mod policies;
pub mod events;
pub mod analytics;
pub mod persistence;
pub mod integration;
pub mod api;
pub mod cli;
pub mod security;

/// Library-level result alias for consistency with other Neo modules.
pub type Result<T> = std::result::Result<T, error::LearningError>;

/// Convenient re-exports for common types and traits used throughout the learning system.
pub mod prelude {
    pub use super::core::{LearningEngine, LearningSession, LearningConfiguration};
    pub use super::experience::{Experience, ExperienceBuilder};
    pub use super::memory::{Episode, EpisodeStore};
    pub use super::reflection::{ReflectionResult, ReflectionRecommendation};
    pub use super::knowledge::{KnowledgeConsolidator, ConceptMerger};
    pub use super::patterns::{PatternMiner, Pattern};
    pub use super::skills::{SkillLibrary, Skill};
    pub use super::strategy::{StrategyRefiner, HeuristicRepository};
    pub use super::performance::PerformanceOptimizer;
    pub use super::failure::{FailureAnalyzer, RootCauseAnalyzer};
    pub use super::policies::{LearningPolicy, LearningPolicyType};
    pub use super::events::{LearningEvent, LearningEventType};
    pub use super::analytics::LearningAnalytics;
    pub use super::persistence::LearningRepository;
}