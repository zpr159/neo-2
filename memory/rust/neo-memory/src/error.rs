use std::fmt;

/// Error codes specific to memory operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum MemoryErrorCode {
    /// Memory entry was not found.
    NotFound = 100,
    /// Memory capacity exceeded.
    CapacityExceeded = 101,
    /// Memory has expired.
    Expired = 102,
    /// Access denied due to permissions.
    AccessDenied = 103,
    /// Invalid namespace.
    InvalidNamespace = 104,
    /// Encryption or decryption failure.
    EncryptionError = 105,
    /// Persistence layer failure.
    PersistenceError = 106,
    /// Index corruption or failure.
    IndexError = 107,
    /// Embedding generation failed.
    EmbeddingError = 108,
    /// Consolidation failure.
    ConsolidationError = 109,
    /// Serialization error.
    SerializationError = 110,
    /// Invalid input or configuration.
    InvalidInput = 111,
    /// Memory already exists.
    AlreadyExists = 112,
    /// Operation not permitted in current state.
    StateError = 113,
    /// Internal error.
    Internal = 999,
}

impl fmt::Display for MemoryErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "NOT_FOUND"),
            Self::CapacityExceeded => write!(f, "CAPACITY_EXCEEDED"),
            Self::Expired => write!(f, "EXPIRED"),
            Self::AccessDenied => write!(f, "ACCESS_DENIED"),
            Self::InvalidNamespace => write!(f, "INVALID_NAMESPACE"),
            Self::EncryptionError => write!(f, "ENCRYPTION_ERROR"),
            Self::PersistenceError => write!(f, "PERSISTENCE_ERROR"),
            Self::IndexError => write!(f, "INDEX_ERROR"),
            Self::EmbeddingError => write!(f, "EMBEDDING_ERROR"),
            Self::ConsolidationError => write!(f, "CONSOLIDATION_ERROR"),
            Self::SerializationError => write!(f, "SERIALIZATION_ERROR"),
            Self::InvalidInput => write!(f, "INVALID_INPUT"),
            Self::AlreadyExists => write!(f, "ALREADY_EXISTS"),
            Self::StateError => write!(f, "STATE_ERROR"),
            Self::Internal => write!(f, "INTERNAL"),
        }
    }
}

/// Unified error type for all memory subsystem operations.
#[derive(Debug)]
pub enum MemoryError {
    /// Memory entry not found.
    NotFound(String),
    /// Memory capacity exceeded.
    CapacityExceeded(String),
    /// Memory has expired.
    Expired(String),
    /// Access denied.
    AccessDenied(String),
    /// Invalid namespace.
    InvalidNamespace(String),
    /// Encryption/decryption error.
    EncryptionError(String),
    /// Persistence failure.
    PersistenceError(String),
    /// Index error.
    IndexError(String),
    /// Embedding generation error.
    EmbeddingError(String),
    /// Consolidation error.
    ConsolidationError(String),
    /// Serialization error.
    SerializationError(String),
    /// Invalid input.
    InvalidInput(String),
    /// Already exists.
    AlreadyExists(String),
    /// State error.
    StateError(String),
    /// Internal error.
    Internal(String),
    /// Not implemented.
    NotImplemented(String),
}

impl MemoryError {
    /// Get the error code.
    #[must_use]
    pub fn code(&self) -> MemoryErrorCode {
        match self {
            Self::NotFound(_) => MemoryErrorCode::NotFound,
            Self::CapacityExceeded(_) => MemoryErrorCode::CapacityExceeded,
            Self::Expired(_) => MemoryErrorCode::Expired,
            Self::AccessDenied(_) => MemoryErrorCode::AccessDenied,
            Self::InvalidNamespace(_) => MemoryErrorCode::InvalidNamespace,
            Self::EncryptionError(_) => MemoryErrorCode::EncryptionError,
            Self::PersistenceError(_) => MemoryErrorCode::PersistenceError,
            Self::IndexError(_) => MemoryErrorCode::IndexError,
            Self::EmbeddingError(_) => MemoryErrorCode::EmbeddingError,
            Self::ConsolidationError(_) => MemoryErrorCode::ConsolidationError,
            Self::SerializationError(_) => MemoryErrorCode::SerializationError,
            Self::InvalidInput(_) => MemoryErrorCode::InvalidInput,
            Self::AlreadyExists(_) => MemoryErrorCode::AlreadyExists,
            Self::StateError(_) => MemoryErrorCode::StateError,
            Self::Internal(_) => MemoryErrorCode::Internal,
            Self::NotImplemented(_) => MemoryErrorCode::Internal,
        }
    }

    /// Get the error message.
    #[must_use]
    pub fn message(&self) -> &str {
        match self {
            Self::NotFound(msg)
            | Self::CapacityExceeded(msg)
            | Self::Expired(msg)
            | Self::AccessDenied(msg)
            | Self::InvalidNamespace(msg)
            | Self::EncryptionError(msg)
            | Self::PersistenceError(msg)
            | Self::IndexError(msg)
            | Self::EmbeddingError(msg)
            | Self::ConsolidationError(msg)
            | Self::SerializationError(msg)
            | Self::InvalidInput(msg)
            | Self::AlreadyExists(msg)
            | Self::StateError(msg)
            | Self::Internal(msg)
            | Self::NotImplemented(msg) => msg,
        }
    }
}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(msg) => write!(f, "[{}] {}", MemoryErrorCode::NotFound, msg),
            Self::CapacityExceeded(msg) => write!(f, "[{}] {}", MemoryErrorCode::CapacityExceeded, msg),
            Self::Expired(msg) => write!(f, "[{}] {}", MemoryErrorCode::Expired, msg),
            Self::AccessDenied(msg) => write!(f, "[{}] {}", MemoryErrorCode::AccessDenied, msg),
            Self::InvalidNamespace(msg) => write!(f, "[{}] {}", MemoryErrorCode::InvalidNamespace, msg),
            Self::EncryptionError(msg) => write!(f, "[{}] {}", MemoryErrorCode::EncryptionError, msg),
            Self::PersistenceError(msg) => write!(f, "[{}] {}", MemoryErrorCode::PersistenceError, msg),
            Self::IndexError(msg) => write!(f, "[{}] {}", MemoryErrorCode::IndexError, msg),
            Self::EmbeddingError(msg) => write!(f, "[{}] {}", MemoryErrorCode::EmbeddingError, msg),
            Self::ConsolidationError(msg) => write!(f, "[{}] {}", MemoryErrorCode::ConsolidationError, msg),
            Self::SerializationError(msg) => write!(f, "[{}] {}", MemoryErrorCode::SerializationError, msg),
            Self::InvalidInput(msg) => write!(f, "[{}] {}", MemoryErrorCode::InvalidInput, msg),
            Self::AlreadyExists(msg) => write!(f, "[{}] {}", MemoryErrorCode::AlreadyExists, msg),
            Self::StateError(msg) => write!(f, "[{}] {}", MemoryErrorCode::StateError, msg),
            Self::Internal(msg) => write!(f, "[{}] {}", MemoryErrorCode::Internal, msg),
            Self::NotImplemented(msg) => write!(f, "[NOT_IMPLEMENTED] {}", msg),
        }
    }
}

impl std::error::Error for MemoryError {}

impl From<MemoryError> for neo_core::error::NeoError {
    fn from(e: MemoryError) -> Self {
        neo_core::error::NeoError::Internal(e.to_string())
    }
}

impl From<serde_json::Error> for MemoryError {
    fn from(e: serde_json::Error) -> Self {
        MemoryError::SerializationError(e.to_string())
    }
}

impl From<std::io::Error> for MemoryError {
    fn from(e: std::io::Error) -> Self {
        MemoryError::PersistenceError(e.to_string())
    }
}

impl From<rusqlite::Error> for MemoryError {
    fn from(e: rusqlite::Error) -> Self {
        MemoryError::PersistenceError(e.to_string())
    }
}

/// Convenience result alias for memory operations.
pub type MemoryResult<T> = Result<T, MemoryError>;
