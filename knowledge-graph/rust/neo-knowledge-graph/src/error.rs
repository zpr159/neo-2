use std::fmt;

/// Error codes for the knowledge system (4000-4099).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum KnowledgeErrorCode {
    EntityNotFound = 4000,
    RelationNotFound = 4001,
    ConceptNotFound = 4002,
    InvalidEntity = 4003,
    InvalidRelation = 4004,
    DuplicateEntity = 4005,
    DuplicateRelation = 4006,
    OntologyViolation = 4007,
    SchemaViolation = 4008,
    ExtractionFailed = 4009,
    StorageError = 4010,
    SerializationError = 4011,
    DeserializationError = 4012,
    SearchError = 4013,
    TraversalError = 4014,
    ValidationError = 4015,
    ContradictionDetected = 4016,
    ConflictUnresolvable = 4017,
    NamespaceError = 4018,
    PermissionDenied = 4019,
    InferenceError = 4020,
    WorldModelError = 4021,
    AnalyticsError = 4022,
    SnapshotError = 4023,
    RecoveryError = 4024,
    CompressionError = 4025,
    PruningError = 4026,
    EvolutionError = 4027,
    ConfigError = 4028,
    InternalError = 4029,
}

impl fmt::Display for KnowledgeErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Error type for the Neo Knowledge System.
#[derive(Debug)]
pub enum KnowledgeError {
    EntityNotFound(String),
    RelationNotFound(String),
    ConceptNotFound(String),
    InvalidEntity(String),
    InvalidRelation(String),
    DuplicateEntity(String),
    DuplicateRelation(String),
    OntologyViolation(String),
    SchemaViolation(String),
    ExtractionFailed(String),
    StorageError(String),
    SerializationError(String),
    DeserializationError(String),
    SearchError(String),
    TraversalError(String),
    ValidationError(String),
    ContradictionDetected(String),
    ConflictUnresolvable(String),
    NamespaceError(String),
    PermissionDenied(String),
    InferenceError(String),
    WorldModelError(String),
    AnalyticsError(String),
    SnapshotError(String),
    RecoveryError(String),
    CompressionError(String),
    PruningError(String),
    EvolutionError(String),
    ConfigError(String),
    InternalError(String),
}

impl KnowledgeError {
    /// Returns the error code for this error.
    #[must_use]
    pub fn code(&self) -> KnowledgeErrorCode {
        match self {
            Self::EntityNotFound(_) => KnowledgeErrorCode::EntityNotFound,
            Self::RelationNotFound(_) => KnowledgeErrorCode::RelationNotFound,
            Self::ConceptNotFound(_) => KnowledgeErrorCode::ConceptNotFound,
            Self::InvalidEntity(_) => KnowledgeErrorCode::InvalidEntity,
            Self::InvalidRelation(_) => KnowledgeErrorCode::InvalidRelation,
            Self::DuplicateEntity(_) => KnowledgeErrorCode::DuplicateEntity,
            Self::DuplicateRelation(_) => KnowledgeErrorCode::DuplicateRelation,
            Self::OntologyViolation(_) => KnowledgeErrorCode::OntologyViolation,
            Self::SchemaViolation(_) => KnowledgeErrorCode::SchemaViolation,
            Self::ExtractionFailed(_) => KnowledgeErrorCode::ExtractionFailed,
            Self::StorageError(_) => KnowledgeErrorCode::StorageError,
            Self::SerializationError(_) => KnowledgeErrorCode::SerializationError,
            Self::DeserializationError(_) => KnowledgeErrorCode::DeserializationError,
            Self::SearchError(_) => KnowledgeErrorCode::SearchError,
            Self::TraversalError(_) => KnowledgeErrorCode::TraversalError,
            Self::ValidationError(_) => KnowledgeErrorCode::ValidationError,
            Self::ContradictionDetected(_) => KnowledgeErrorCode::ContradictionDetected,
            Self::ConflictUnresolvable(_) => KnowledgeErrorCode::ConflictUnresolvable,
            Self::NamespaceError(_) => KnowledgeErrorCode::NamespaceError,
            Self::PermissionDenied(_) => KnowledgeErrorCode::PermissionDenied,
            Self::InferenceError(_) => KnowledgeErrorCode::InferenceError,
            Self::WorldModelError(_) => KnowledgeErrorCode::WorldModelError,
            Self::AnalyticsError(_) => KnowledgeErrorCode::AnalyticsError,
            Self::SnapshotError(_) => KnowledgeErrorCode::SnapshotError,
            Self::RecoveryError(_) => KnowledgeErrorCode::RecoveryError,
            Self::CompressionError(_) => KnowledgeErrorCode::CompressionError,
            Self::PruningError(_) => KnowledgeErrorCode::PruningError,
            Self::EvolutionError(_) => KnowledgeErrorCode::EvolutionError,
            Self::ConfigError(_) => KnowledgeErrorCode::ConfigError,
            Self::InternalError(_) => KnowledgeErrorCode::InternalError,
        }
    }
}

impl fmt::Display for KnowledgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EntityNotFound(msg) => write!(f, "[entity not found] {}", msg),
            Self::RelationNotFound(msg) => write!(f, "[relation not found] {}", msg),
            Self::ConceptNotFound(msg) => write!(f, "[concept not found] {}", msg),
            Self::InvalidEntity(msg) => write!(f, "[invalid entity] {}", msg),
            Self::InvalidRelation(msg) => write!(f, "[invalid relation] {}", msg),
            Self::DuplicateEntity(msg) => write!(f, "[duplicate entity] {}", msg),
            Self::DuplicateRelation(msg) => write!(f, "[duplicate relation] {}", msg),
            Self::OntologyViolation(msg) => write!(f, "[ontology violation] {}", msg),
            Self::SchemaViolation(msg) => write!(f, "[schema violation] {}", msg),
            Self::ExtractionFailed(msg) => write!(f, "[extraction failed] {}", msg),
            Self::StorageError(msg) => write!(f, "[storage error] {}", msg),
            Self::SerializationError(msg) => write!(f, "[serialization error] {}", msg),
            Self::DeserializationError(msg) => write!(f, "[deserialization error] {}", msg),
            Self::SearchError(msg) => write!(f, "[search error] {}", msg),
            Self::TraversalError(msg) => write!(f, "[traversal error] {}", msg),
            Self::ValidationError(msg) => write!(f, "[validation error] {}", msg),
            Self::ContradictionDetected(msg) => write!(f, "[contradiction detected] {}", msg),
            Self::ConflictUnresolvable(msg) => write!(f, "[conflict unresolvable] {}", msg),
            Self::NamespaceError(msg) => write!(f, "[namespace error] {}", msg),
            Self::PermissionDenied(msg) => write!(f, "[permission denied] {}", msg),
            Self::InferenceError(msg) => write!(f, "[inference error] {}", msg),
            Self::WorldModelError(msg) => write!(f, "[world model error] {}", msg),
            Self::AnalyticsError(msg) => write!(f, "[analytics error] {}", msg),
            Self::SnapshotError(msg) => write!(f, "[snapshot error] {}", msg),
            Self::RecoveryError(msg) => write!(f, "[recovery error] {}", msg),
            Self::CompressionError(msg) => write!(f, "[compression error] {}", msg),
            Self::PruningError(msg) => write!(f, "[pruning error] {}", msg),
            Self::EvolutionError(msg) => write!(f, "[evolution error] {}", msg),
            Self::ConfigError(msg) => write!(f, "[config error] {}", msg),
            Self::InternalError(msg) => write!(f, "[internal error] {}", msg),
        }
    }
}

impl std::error::Error for KnowledgeError {}

impl From<serde_json::Error> for KnowledgeError {
    fn from(e: serde_json::Error) -> Self {
        Self::SerializationError(e.to_string())
    }
}

impl From<std::io::Error> for KnowledgeError {
    fn from(e: std::io::Error) -> Self {
        Self::StorageError(e.to_string())
    }
}

impl From<neo_core::error::NeoError> for KnowledgeError {
    fn from(e: neo_core::error::NeoError) -> Self {
        Self::InternalError(e.to_string())
    }
}

/// Convenience result alias for knowledge operations.
pub type KnowledgeResult<T> = Result<T, KnowledgeError>;
