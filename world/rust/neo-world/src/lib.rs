//! # Neo World Model
//!
//! Internal world model for the Neo AGI Operating System.
//!
//! Maintains a continuously evolving internal representation of reality,
//! integrating memory, perception, reasoning, planning, knowledge, conversation,
//! distributed observations, and tool outputs into one coherent representation.
//!
//! ## Predictive, Not Archival
//!
//! The World Model answers:
//! - What currently exists?
//! - What changed? Why?
//! - What will probably happen next?
//! - How should planning adapt?
//!
//! ## Subsystems
//!
//! - **EntityTracker** — Persistent entity tracking with attributes, lifecycle, and versioning
//! - **RelationshipManager** — Directional typed relationships with history and strength
//! - **SpatialModel** — Location tracking, spatial queries, proximity
//! - **TemporalModel** — Event recording, timelines, temporal queries
//! - **CausalModel** — Causal reasoning, root cause discovery, counterfactuals
//! - **EnvironmentManager** — Environment modeling and tracking
//! - **PerceptionProcessor** — Converting observations into world model updates
//! - **PredictionEngine** — Future state prediction
//! - **SimulationEngine** — What-if analysis on isolated state copies
//! - **UncertaintyTracker** — Evidence-based uncertainty management
//! - **HistoryManager** — Complete change history and replay
//! - **WorldStateManager** — Versioned snapshots and state diffs

pub mod api;
pub mod builders;
pub mod causal;
pub mod confidence;
pub mod config;
pub mod distributed;
pub mod entity;
pub mod environment;
pub mod error;
pub mod history;
pub mod integration;
pub mod lifecycle;
pub mod metrics;
pub mod observation;
pub mod ontology;
pub mod persistence;
pub mod perception;
pub mod prediction;
pub mod relationships;
pub mod simulation;
pub mod spatial;
pub mod state;
pub mod synchronization;
pub mod temporal;
pub mod types;
pub mod uncertainty;

pub use api::WorldModel;
pub use builders::{EntityBuilder, RelationshipBuilder};
pub use causal::{CausalChain, CausalLink, CausalModel, CausalStrength};
pub use confidence::{apply_decay, merge_confidences, ConfidenceAccumulator, Evidence, SourceReliability};
pub use config::WorldConfig;
pub use distributed::{DistributedManager, DistributedNode};
pub use entity::{EntityTracker, WorldEntity};
pub use environment::{Environment, EnvironmentManager};
pub use error::{WorldError, WorldResult};
pub use history::{HistoryEntry, HistoryManager};
pub use integration::IntegrationLayer;
pub use lifecycle::{is_valid_transition, LifecycleEvent, LifecycleManager};
pub use metrics::{MetricsSnapshot, WorldMetrics};
pub use observation::{Observation, ObservationPipeline, ObservationType};
pub use ontology::{EntityTypeEntry, EntityTypeRegistry};
pub use persistence::PersistenceManager;
pub use perception::{
    Perception, PerceptionBuffer, PerceptionFusion, PerceptionProcessor, PerceivedRelationship,
};
pub use prediction::{Prediction, PredictionEngine};
pub use relationships::{Relationship, RelationshipManager, RelationshipStrength, RelationshipType};
pub use simulation::{SimulationEngine, SimulationResult, SimulationScenario};
pub use spatial::{Coordinates, Location, SpatialModel, SpatialRegion, SpatialRelationType};
pub use state::WorldStateManager;
pub use synchronization::SynchronizationManager;
pub use temporal::{TemporalEvent, TemporalModel, TimeWindow, Timeline, TimelineEntry};
pub use types::{
    AttributeValue, AttributeSource, Confidence, ConfidenceLevel, CausalLinkId, EntityAttribute,
    EntityId, EntityQuery, EntityState, EntityType, EntityVersion, EnvironmentId, EnvironmentType,
    EventId, EventQuery, EventType, HistoryEntryId, HistoryEntryType, LocationId, ObservationId,
    ObservationSource, PerceptionId, PredictionId, PredictionType, QueryResult, ReferenceFrame,
    RelationshipId, SimulationId, SimulationState, SnapshotId, WorldContext, WorldSnapshot,
    WorldVersion,
};
pub use uncertainty::{Uncertainty, UncertaintyCategory, UncertaintyTracker};

#[cfg(test)]
mod tests;
