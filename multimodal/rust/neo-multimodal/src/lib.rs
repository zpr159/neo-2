#!\[forbid(unsafe_code)\]
#![deny(
    missing_docs,
    warnings,
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces,
    unused_extern_crates
)]

//! Neo Multimodal Intelligence System — unified perception, understanding,
//! and reasoning across text, images, audio, video, and documents.
//!
//! This module provides Neo AGI OS with the ability to process and understand
//! multiple data modalities through a consistent API. The system enables Neo
//! to:
//! - Perceive the world through visual, auditory, and textual channels
//! - Extract meaningful information from diverse content types
//! - Create unified representations for cross-modal reasoning
//! - Retrieve relevant information across modalities
//! - Generate insights and summaries
//!
//! The Multimodal Intelligence System serves as Neo's perceptual layer,
//! working in concert with the Executive, Reasoning, Memory, and other
//! subsystems to provide comprehensive understanding of the environment
//! and user interactions.

pub mod core;
pub mod processors;
pub mod engines;
pub mod embedding;
pub mod pipeline;
pub mod storage;
pub mod analytics;
pub mod events;
pub mod integration;
pub mod security;
pub mod rest;
pub mod cli;
pub mod sdk;

/// Library-level result alias for consistency with other Neo modules.
pub type Result<T> = std::result::Result<T, error::MultimodalError>;

/// Convenient re-exports for common types and traits used throughout
/// the multimodal system.
pub mod prelude {
    pub use super::core::{MultimodalEngine, MultimodalSession, MultimodalContext};
    pub use super::processors::{TextProcessor, ImageProcessor, AudioProcessor, VideoProcessor, DocumentProcessor, OCRProcessor};
    pub use super::embedding::{EmbeddingEngine, EmbeddingStore};
    pub use super::engines::{MultimodalRouter, InferenceManager};
    pub use super::pipeline::{MediaPipeline, ProcessingStep};
    pub use super::analytics::MultimodalAnalytics;
    pub use super::events::{MediaEvent, MediaEventType};
    pub use super::security::{MediaSecurity, ContentValidator};
    pub use super::integration::{PlanningIntegration, ReasoningIntegration, LearningIntegration};
    pub use super::core::{MediaAsset, MediaMetadata, Modality, MediaFormat};
}