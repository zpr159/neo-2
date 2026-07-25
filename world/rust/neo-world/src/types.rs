use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

// ── Identity Types ──────────────────────────────────────────────────────────

macro_rules! define_world_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(pub String);

        impl $name {
            pub fn new(id: impl Into<String>) -> Self {
                Self(id.into())
            }

            pub fn random() -> Self {
                Self(uuid::Uuid::new_v4().to_string())
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(s.to_string())
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self(s)
            }
        }
    };
}

define_world_id!(
    /// Unique identifier for a world entity.
    EntityId
);

define_world_id!(
    /// Unique identifier for a location.
    LocationId
);

define_world_id!(
    /// Unique identifier for a temporal event.
    EventId
);

define_world_id!(
    /// Unique identifier for a causal link.
    CausalLinkId
);

define_world_id!(
    /// Unique identifier for a relationship.
    RelationshipId
);

define_world_id!(
    /// Unique identifier for an observation.
    ObservationId
);

define_world_id!(
    /// Unique identifier for a perception.
    PerceptionId
);

define_world_id!(
    /// Unique identifier for an environment.
    EnvironmentId
);

define_world_id!(
    /// Unique identifier for a world snapshot.
    SnapshotId
);

define_world_id!(
    /// Unique identifier for a simulation run.
    SimulationId
);

define_world_id!(
    /// Unique identifier for a prediction.
    PredictionId
);

define_world_id!(
    /// Unique identifier for an evidence record.
    EvidenceId
);

define_world_id!(
    /// Unique identifier for a history entry.
    HistoryEntryId
);

// ── Entity Types ────────────────────────────────────────────────────────────

/// Classification of world entity types. Extensible via Custom variant.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityType {
    Human,
    User,
    Agent,
    Tool,
    Capability,
    Workflow,
    Task,
    Location,
    Object,
    File,
    Document,
    Image,
    Audio,
    Video,
    Conversation,
    Goal,
    Memory,
    Knowledge,
    Environment,
    Vehicle,
    Device,
    Container,
    Sensor,
    Service,
    Organization,
    Concept,
    System,
    Custom(String),
}

impl EntityType {
    /// Returns a human-readable label for the entity type.
    #[must_use]
    pub fn label(&self) -> &str {
        match self {
            Self::Human => "Human",
            Self::User => "User",
            Self::Agent => "Agent",
            Self::Tool => "Tool",
            Self::Capability => "Capability",
            Self::Workflow => "Workflow",
            Self::Task => "Task",
            Self::Location => "Location",
            Self::Object => "Object",
            Self::File => "File",
            Self::Document => "Document",
            Self::Image => "Image",
            Self::Audio => "Audio",
            Self::Video => "Video",
            Self::Conversation => "Conversation",
            Self::Goal => "Goal",
            Self::Memory => "Memory",
            Self::Knowledge => "Knowledge",
            Self::Environment => "Environment",
            Self::Vehicle => "Vehicle",
            Self::Device => "Device",
            Self::Container => "Container",
            Self::Sensor => "Sensor",
            Self::Service => "Service",
            Self::Organization => "Organization",
            Self::Concept => "Concept",
            Self::System => "System",
            Self::Custom(name) => name,
        }
    }

    /// Parse an entity type from a string.
    pub fn from_str_loose(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "human" => Self::Human,
            "user" => Self::User,
            "agent" => Self::Agent,
            "tool" => Self::Tool,
            "capability" => Self::Capability,
            "workflow" => Self::Workflow,
            "task" => Self::Task,
            "location" => Self::Location,
            "object" => Self::Object,
            "file" => Self::File,
            "document" => Self::Document,
            "image" => Self::Image,
            "audio" => Self::Audio,
            "video" => Self::Video,
            "conversation" => Self::Conversation,
            "goal" => Self::Goal,
            "memory" => Self::Memory,
            "knowledge" => Self::Knowledge,
            "environment" => Self::Environment,
            "vehicle" => Self::Vehicle,
            "device" => Self::Device,
            "container" => Self::Container,
            "sensor" => Self::Sensor,
            "service" => Self::Service,
            "organization" => Self::Organization,
            "concept" => Self::Concept,
            "system" => Self::System,
            other => Self::Custom(other.to_string()),
        }
    }
}

impl fmt::Display for EntityType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

// ── Event Types ─────────────────────────────────────────────────────────────

/// Types of temporal events.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventType {
    Creation,
    Update,
    Deletion,
    Movement,
    Conversation,
    Reasoning,
    Learning,
    ToolExecution,
    WorkflowExecution,
    AgentCommunication,
    ClusterEvent,
    PlanningEvent,
    MemoryUpdate,
    KnowledgeUpdate,
    Observation,
    Action,
    StateChange,
    Decision,
    Arrival,
    Departure,
    Error,
    System,
    Query,
    Response,
    Custom(String),
}

impl EventType {
    pub fn label(&self) -> &str {
        match self {
            Self::Creation => "creation",
            Self::Update => "update",
            Self::Deletion => "deletion",
            Self::Movement => "movement",
            Self::Conversation => "conversation",
            Self::Reasoning => "reasoning",
            Self::Learning => "learning",
            Self::ToolExecution => "tool_execution",
            Self::WorkflowExecution => "workflow_execution",
            Self::AgentCommunication => "agent_communication",
            Self::ClusterEvent => "cluster_event",
            Self::PlanningEvent => "planning_event",
            Self::MemoryUpdate => "memory_update",
            Self::KnowledgeUpdate => "knowledge_update",
            Self::Observation => "observation",
            Self::Action => "action",
            Self::StateChange => "state_change",
            Self::Decision => "decision",
            Self::Arrival => "arrival",
            Self::Departure => "departure",
            Self::Error => "error",
            Self::System => "system",
            Self::Query => "query",
            Self::Response => "response",
            Self::Custom(name) => name,
        }
    }
}

impl fmt::Display for EventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

// ── Entity State ────────────────────────────────────────────────────────────

/// Lifecycle state of an entity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityState {
    Created,
    Active,
    Suspended,
    Updating,
    Migrating,
    Archived,
    Deleted,
    Unknown,
}

impl EntityState {
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Deleted | Self::Archived)
    }

    #[must_use]
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Active | Self::Updating)
    }
}

impl fmt::Display for EntityState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Created => write!(f, "created"),
            Self::Active => write!(f, "active"),
            Self::Suspended => write!(f, "suspended"),
            Self::Updating => write!(f, "updating"),
            Self::Migrating => write!(f, "migrating"),
            Self::Archived => write!(f, "archived"),
            Self::Deleted => write!(f, "deleted"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

// ── Entity Attribute Value ──────────────────────────────────────────────────

/// Typed property value for entity attributes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AttributeValue {
    Boolean(bool),
    Integer(i64),
    Float(f64),
    Text(String),
    Bytes(Vec<u8>),
    Vector(Vec<f32>),
    Json(serde_json::Value),
    Nested(HashMap<String, AttributeValue>),
    List(Vec<AttributeValue>),
    Map(HashMap<String, AttributeValue>),
    EntityRef(EntityId),
    Timestamp(String),
}

impl fmt::Display for AttributeValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Boolean(v) => write!(f, "{v}"),
            Self::Integer(v) => write!(f, "{v}"),
            Self::Float(v) => write!(f, "{v}"),
            Self::Text(v) => write!(f, "{v}"),
            Self::Bytes(v) => write!(f, "[{} bytes]", v.len()),
            Self::Vector(v) => write!(f, "[{} dims]", v.len()),
            Self::Json(v) => write!(f, "{v}"),
            Self::Nested(m) => write!(f, "{{{} keys}}", m.len()),
            Self::List(l) => write!(f, "[{} items]", l.len()),
            Self::Map(m) => write!(f, "{{{} entries}}", m.len()),
            Self::EntityRef(id) => write!(f, "entity:{id}"),
            Self::Timestamp(t) => write!(f, "{t}"),
        }
    }
}

/// Source attribution for an attribute value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeSource {
    pub source_type: String,
    pub source_id: Option<String>,
    pub confidence: f32,
    pub recorded_at: DateTime<Utc>,
}

impl Default for AttributeSource {
    fn default() -> Self {
        Self {
            source_type: "unknown".into(),
            source_id: None,
            confidence: 0.5,
            recorded_at: Utc::now(),
        }
    }
}

/// An attribute with value, metadata, and source attribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityAttribute {
    pub key: String,
    pub value: AttributeValue,
    pub source: AttributeSource,
    pub confidence: f32,
    pub updated_at: DateTime<Utc>,
}

// ── Entity Version ──────────────────────────────────────────────────────────

/// Version of an entity at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityVersion {
    pub version: u64,
    pub timestamp: DateTime<Utc>,
    pub snapshot: serde_json::Value,
    pub change_description: String,
}

// ── World Version ───────────────────────────────────────────────────────────

/// Monotonically increasing version of the world state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WorldVersion(pub u64);

impl WorldVersion {
    pub fn initial() -> Self {
        Self(0)
    }

    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }

    #[must_use]
    pub fn number(self) -> u64 {
        self.0
    }
}

impl Default for WorldVersion {
    fn default() -> Self {
        Self::initial()
    }
}

impl fmt::Display for WorldVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}", self.0)
    }
}

// ── Confidence ──────────────────────────────────────────────────────────────

/// Confidence level for world model beliefs.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Confidence(pub f32);

impl Confidence {
    pub const CERTAIN: Self = Self(1.0);
    pub const HIGH: Self = Self(0.8);
    pub const MEDIUM: Self = Self(0.5);
    pub const LOW: Self = Self(0.3);
    pub const SPECULATIVE: Self = Self(0.1);
    pub const UNKNOWN: Self = Self(0.0);

    #[must_use]
    pub fn level(&self) -> ConfidenceLevel {
        if self.0 >= 0.95 {
            ConfidenceLevel::Certain
        } else if self.0 >= 0.7 {
            ConfidenceLevel::High
        } else if self.0 >= 0.4 {
            ConfidenceLevel::Medium
        } else if self.0 >= 0.2 {
            ConfidenceLevel::Low
        } else if self.0 > 0.0 {
            ConfidenceLevel::Speculative
        } else {
            ConfidenceLevel::Unknown
        }
    }

    #[must_use]
    pub fn value(self) -> f32 {
        self.0
    }

    #[must_use]
    pub fn is_high_enough(self, threshold: f32) -> bool {
        self.0 >= threshold
    }
}

impl Default for Confidence {
    fn default() -> Self {
        Self::MEDIUM
    }
}

impl fmt::Display for Confidence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.level())
    }
}

impl From<f32> for Confidence {
    fn from(v: f32) -> Self {
        Self(v.clamp(0.0, 1.0))
    }
}

/// Named confidence levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ConfidenceLevel {
    Certain,
    High,
    Medium,
    Low,
    Speculative,
    Unknown,
}

impl fmt::Display for ConfidenceLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Certain => write!(f, "certain"),
            Self::High => write!(f, "high"),
            Self::Medium => write!(f, "medium"),
            Self::Low => write!(f, "low"),
            Self::Speculative => write!(f, "speculative"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

// ── Environment Type ────────────────────────────────────────────────────────

/// Classification of environment types.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EnvironmentType {
    Physical,
    Digital,
    Virtual,
    Hybrid,
    Abstract,
}

impl fmt::Display for EnvironmentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Physical => write!(f, "physical"),
            Self::Digital => write!(f, "digital"),
            Self::Virtual => write!(f, "virtual"),
            Self::Hybrid => write!(f, "hybrid"),
            Self::Abstract => write!(f, "abstract"),
        }
    }
}

// ── World Snapshot ──────────────────────────────────────────────────────────

/// A snapshot of the world state at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSnapshot {
    pub id: SnapshotId,
    pub version: WorldVersion,
    pub entity_count: usize,
    pub relationship_count: usize,
    pub location_count: usize,
    pub event_count: usize,
    pub causal_link_count: usize,
    pub environment_count: usize,
    pub summary: String,
    pub created_at: DateTime<Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl WorldSnapshot {
    #[must_use]
    pub fn new(version: WorldVersion, summary: impl Into<String>) -> Self {
        Self {
            id: SnapshotId::random(),
            version,
            entity_count: 0,
            relationship_count: 0,
            location_count: 0,
            event_count: 0,
            causal_link_count: 0,
            environment_count: 0,
            summary: summary.into(),
            created_at: Utc::now(),
            metadata: HashMap::new(),
        }
    }
}

impl fmt::Display for WorldSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Snapshot({}, entities={}, relationships={}, locations={}, events={})",
            self.version, self.entity_count, self.relationship_count,
            self.location_count, self.event_count,
        )
    }
}

// ── World Context ───────────────────────────────────────────────────────────

/// Contextual information about the current world state provided to subsystems.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldContext {
    pub version: WorldVersion,
    pub entity_count: usize,
    pub active_entity_count: usize,
    pub recent_events_count: usize,
    pub environment_summary: String,
    pub key_entities: Vec<EntityId>,
    pub active_goals: Vec<EntityId>,
    pub pending_predictions: usize,
}

impl Default for WorldContext {
    fn default() -> Self {
        Self {
            version: WorldVersion::initial(),
            entity_count: 0,
            active_entity_count: 0,
            recent_events_count: 0,
            environment_summary: String::new(),
            key_entities: Vec::new(),
            active_goals: Vec::new(),
            pending_predictions: 0,
        }
    }
}

// ── Query Types ─────────────────────────────────────────────────────────────

/// Criteria for finding entities.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EntityQuery {
    pub name: Option<String>,
    pub entity_type: Option<EntityType>,
    pub state: Option<EntityState>,
    pub tag: Option<String>,
    pub min_confidence: Option<f32>,
    pub location_id: Option<LocationId>,
    pub related_to: Option<EntityId>,
    pub limit: Option<usize>,
}

/// Criteria for finding events.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventQuery {
    pub event_type: Option<String>,
    pub entity_id: Option<EntityId>,
    pub after: Option<DateTime<Utc>>,
    pub before: Option<DateTime<Utc>>,
    pub min_confidence: Option<f32>,
    pub limit: Option<usize>,
}

/// Result of a world model query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult<T> {
    pub items: Vec<T>,
    pub total_count: usize,
    pub version: WorldVersion,
    pub query_time_ms: u64,
}

// ── Prediction Types ────────────────────────────────────────────────────────

/// A prediction about the future state of the world.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prediction {
    pub id: PredictionId,
    pub description: String,
    pub prediction_type: PredictionType,
    pub confidence: Confidence,
    pub predicted_at: DateTime<Utc>,
    pub predicted_for: Option<DateTime<Utc>>,
    pub supporting_evidence: Vec<EvidenceId>,
    pub context: HashMap<String, serde_json::Value>,
    pub actual_outcome: Option<String>,
    pub was_correct: Option<bool>,
}

/// Types of predictions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PredictionType {
    NextAction,
    FutureState,
    WorkflowOutcome,
    AgentBehavior,
    ResourceUtilization,
    ClusterHealth,
    TaskCompletion,
    PlanningSuccess,
    ConversationTopic,
    SystemFailure,
    Custom(String),
}

impl fmt::Display for PredictionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NextAction => write!(f, "next_action"),
            Self::FutureState => write!(f, "future_state"),
            Self::WorkflowOutcome => write!(f, "workflow_outcome"),
            Self::AgentBehavior => write!(f, "agent_behavior"),
            Self::ResourceUtilization => write!(f, "resource_utilization"),
            Self::ClusterHealth => write!(f, "cluster_health"),
            Self::TaskCompletion => write!(f, "task_completion"),
            Self::PlanningSuccess => write!(f, "planning_success"),
            Self::ConversationTopic => write!(f, "conversation_topic"),
            Self::SystemFailure => write!(f, "system_failure"),
            Self::Custom(name) => write!(f, "{name}"),
        }
    }
}

// ── Simulation Types ────────────────────────────────────────────────────────

/// State of a simulation run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimulationState {
    Created,
    Running,
    Paused,
    Completed,
    Failed,
}

impl fmt::Display for SimulationState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Created => write!(f, "created"),
            Self::Running => write!(f, "running"),
            Self::Paused => write!(f, "paused"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

// ── Observation Types ───────────────────────────────────────────────────────

/// Source of an observation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ObservationSource {
    Conversation,
    Memory,
    KnowledgeGraph,
    ToolResult,
    Sensor,
    Filesystem,
    Network,
    DistributedNode,
    ExternalApi,
    Robotics,
    AgentCommunication,
    Workflow,
    Planning,
    Reasoning,
    Custom(String),
}

impl fmt::Display for ObservationSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Conversation => write!(f, "conversation"),
            Self::Memory => write!(f, "memory"),
            Self::KnowledgeGraph => write!(f, "knowledge_graph"),
            Self::ToolResult => write!(f, "tool_result"),
            Self::Sensor => write!(f, "sensor"),
            Self::Filesystem => write!(f, "filesystem"),
            Self::Network => write!(f, "network"),
            Self::DistributedNode => write!(f, "distributed_node"),
            Self::ExternalApi => write!(f, "external_api"),
            Self::Robotics => write!(f, "robotics"),
            Self::AgentCommunication => write!(f, "agent_communication"),
            Self::Workflow => write!(f, "workflow"),
            Self::Planning => write!(f, "planning"),
            Self::Reasoning => write!(f, "reasoning"),
            Self::Custom(name) => write!(f, "{name}"),
        }
    }
}

// ── History Types ───────────────────────────────────────────────────────────

/// A type of history entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HistoryEntryType {
    EntityCreated,
    EntityUpdated,
    EntityDeleted,
    RelationshipCreated,
    RelationshipUpdated,
    RelationshipDeleted,
    EventRecorded,
    StateChanged,
    PredictionMade,
    SimulationRun,
    SnapshotTaken,
    ObservationProcessed,
    Custom(String),
}

impl fmt::Display for HistoryEntryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EntityCreated => write!(f, "entity_created"),
            Self::EntityUpdated => write!(f, "entity_updated"),
            Self::EntityDeleted => write!(f, "entity_deleted"),
            Self::RelationshipCreated => write!(f, "relationship_created"),
            Self::RelationshipUpdated => write!(f, "relationship_updated"),
            Self::RelationshipDeleted => write!(f, "relationship_deleted"),
            Self::EventRecorded => write!(f, "event_recorded"),
            Self::StateChanged => write!(f, "state_changed"),
            Self::PredictionMade => write!(f, "prediction_made"),
            Self::SimulationRun => write!(f, "simulation_run"),
            Self::SnapshotTaken => write!(f, "snapshot_taken"),
            Self::ObservationProcessed => write!(f, "observation_processed"),
            Self::Custom(name) => write!(f, "{name}"),
        }
    }
}

// ── Reference Frame ─────────────────────────────────────────────────────────

/// Coordinate reference frame for spatial queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReferenceFrame {
    Global,
    Local(LocationId),
    Entity(EntityId),
    Grid { cell_size: f64 },
    Custom(String),
}

impl fmt::Display for ReferenceFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Global => write!(f, "global"),
            Self::Local(loc) => write!(f, "local:{loc}"),
            Self::Entity(eid) => write!(f, "entity:{eid}"),
            Self::Grid { cell_size } => write!(f, "grid:{cell_size}"),
            Self::Custom(name) => write!(f, "custom:{name}"),
        }
    }
}
