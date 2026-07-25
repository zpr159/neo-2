#!\[forbid(unsafe_code)\]
#![deny(
    missing_docs,
    warnings,
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces,
    unused_extern_crates
)]

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};
use async_trait::async_trait;
use tracing::info;

/// # Core Types Module
/// 
/// Defines the fundamental data structures for the learning system:
/// - LearningEngine: Main orchestrator
/// - LearningSession: Individual learning session
/// - LearningPolicy: Learning strategy and constraints
/// - Experience, Episode, Reflection: Core learning artifacts
/// - Metrics and Analytics: Performance tracking

pub mod engine;
pub mod session;
pub mod policy;
pub mod configuration;
pub mod statistics;
pub mod metrics;
pub mod snapshot;
pub mod checkpoint;
pub mod repository;
pub mod result;
pub mod objective;

/// Re-export essential types for external use
pub use engine::LearningEngine;
pub use session::LearningSession;
pub use policy::LearningPolicy;
pub use configuration::LearningConfiguration;
pub use statistics::LearningStatistics;
pub use metrics::LearningMetrics;
pub use snapshot::LearningSnapshot;
pub use checkpoint::LearningCheckpoint;
pub use repository::LearningRepository;
pub use result::LearningResult;
pub use objective::LearningObjective;
