use std::fmt;

/// Error type for the world model subsystem.
#[derive(Debug)]
pub enum WorldError {
    EntityNotFound(String),
    RelationshipNotFound(String),
    LocationNotFound(String),
    EventNotFound(String),
    EnvironmentNotFound(String),
    SnapshotNotFound(String),
    SpatialError(String),
    TemporalError(String),
    CausalError(String),
    StateError(String),
    PerceptionError(String),
    ObservationError(String),
    PredictionError(String),
    SimulationError(String),
    PersistenceError(String),
    SynchronizationError(String),
    DistributedError(String),
    ValidationError(String),
    Serialization(String),
    InvalidInput(String),
    Internal(String),
    NotInitialized,
    Configuration(String),
    AlreadyExists(String),
    Conflict(String),
    VersionMismatch { expected: u64, actual: u64 },
}

impl fmt::Display for WorldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EntityNotFound(id) => write!(f, "[entity not found] {id}"),
            Self::RelationshipNotFound(id) => write!(f, "[relationship not found] {id}"),
            Self::LocationNotFound(id) => write!(f, "[location not found] {id}"),
            Self::EventNotFound(id) => write!(f, "[event not found] {id}"),
            Self::EnvironmentNotFound(id) => write!(f, "[environment not found] {id}"),
            Self::SnapshotNotFound(id) => write!(f, "[snapshot not found] {id}"),
            Self::SpatialError(msg) => write!(f, "[spatial] {msg}"),
            Self::TemporalError(msg) => write!(f, "[temporal] {msg}"),
            Self::CausalError(msg) => write!(f, "[causal] {msg}"),
            Self::StateError(msg) => write!(f, "[state] {msg}"),
            Self::PerceptionError(msg) => write!(f, "[perception] {msg}"),
            Self::ObservationError(msg) => write!(f, "[observation] {msg}"),
            Self::PredictionError(msg) => write!(f, "[prediction] {msg}"),
            Self::SimulationError(msg) => write!(f, "[simulation] {msg}"),
            Self::PersistenceError(msg) => write!(f, "[persistence] {msg}"),
            Self::SynchronizationError(msg) => write!(f, "[synchronization] {msg}"),
            Self::DistributedError(msg) => write!(f, "[distributed] {msg}"),
            Self::ValidationError(msg) => write!(f, "[validation] {msg}"),
            Self::Serialization(msg) => write!(f, "[serialization] {msg}"),
            Self::InvalidInput(msg) => write!(f, "[invalid input] {msg}"),
            Self::Internal(msg) => write!(f, "[internal] {msg}"),
            Self::NotInitialized => write!(f, "[not initialized] world model not initialized"),
            Self::Configuration(msg) => write!(f, "[configuration] {msg}"),
            Self::AlreadyExists(msg) => write!(f, "[already exists] {msg}"),
            Self::Conflict(msg) => write!(f, "[conflict] {msg}"),
            Self::VersionMismatch { expected, actual } => {
                write!(f, "[version mismatch] expected v{expected}, got v{actual}")
            }
        }
    }
}

impl std::error::Error for WorldError {}

impl From<WorldError> for neo_core::NeoError {
    fn from(e: WorldError) -> Self {
        neo_core::NeoError::Internal(e.to_string())
    }
}

impl From<serde_json::Error> for WorldError {
    fn from(e: serde_json::Error) -> Self {
        Self::Serialization(e.to_string())
    }
}

/// Result type for world model operations.
pub type WorldResult<T> = Result<T, WorldError>;
