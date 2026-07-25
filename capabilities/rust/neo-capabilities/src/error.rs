use std::fmt;

/// Error codes specific to capability operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum CapabilityErrorCode {
    NotFound = 4001,
    AlreadyRegistered = 4002,
    InvalidState = 4003,
    ExecutionFailed = 4004,
    ValidationFailed = 4005,
    PermissionDenied = 4006,
    Timeout = 4007,
    Cancelled = 4008,
    DependencyMissing = 4009,
    ConflictDetected = 4010,
    ResourceExhausted = 4011,
    CompositionFailed = 4012,
    MarketplaceError = 4013,
    SigningError = 4014,
    VersionIncompatible = 4015,
    DiscoveryFailed = 4016,
    HotReloadFailed = 4017,
    SandboxViolation = 4018,
    ApprovalRequired = 4019,
    QuotaExceeded = 4020,
}

impl fmt::Display for CapabilityErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", *self as u16)
    }
}

/// Unified error type for capability framework operations.
#[derive(Debug)]
pub enum CapabilityError {
    NotFound(String),
    AlreadyRegistered(String),
    InvalidState(String),
    ExecutionFailed(String),
    ValidationFailed(String),
    PermissionDenied(String),
    Timeout(String),
    Cancelled(String),
    DependencyMissing(String),
    ConflictDetected(String),
    ResourceExhausted(String),
    CompositionFailed(String),
    MarketplaceError(String),
    SigningError(String),
    VersionIncompatible(String),
    DiscoveryFailed(String),
    HotReloadFailed(String),
    SandboxViolation(String),
    ApprovalRequired(String),
    QuotaExceeded(String),
}

impl CapabilityError {
    /// Returns the error code for this error variant.
    pub fn code(&self) -> CapabilityErrorCode {
        match self {
            Self::NotFound(_) => CapabilityErrorCode::NotFound,
            Self::AlreadyRegistered(_) => CapabilityErrorCode::AlreadyRegistered,
            Self::InvalidState(_) => CapabilityErrorCode::InvalidState,
            Self::ExecutionFailed(_) => CapabilityErrorCode::ExecutionFailed,
            Self::ValidationFailed(_) => CapabilityErrorCode::ValidationFailed,
            Self::PermissionDenied(_) => CapabilityErrorCode::PermissionDenied,
            Self::Timeout(_) => CapabilityErrorCode::Timeout,
            Self::Cancelled(_) => CapabilityErrorCode::Cancelled,
            Self::DependencyMissing(_) => CapabilityErrorCode::DependencyMissing,
            Self::ConflictDetected(_) => CapabilityErrorCode::ConflictDetected,
            Self::ResourceExhausted(_) => CapabilityErrorCode::ResourceExhausted,
            Self::CompositionFailed(_) => CapabilityErrorCode::CompositionFailed,
            Self::MarketplaceError(_) => CapabilityErrorCode::MarketplaceError,
            Self::SigningError(_) => CapabilityErrorCode::SigningError,
            Self::VersionIncompatible(_) => CapabilityErrorCode::VersionIncompatible,
            Self::DiscoveryFailed(_) => CapabilityErrorCode::DiscoveryFailed,
            Self::HotReloadFailed(_) => CapabilityErrorCode::HotReloadFailed,
            Self::SandboxViolation(_) => CapabilityErrorCode::SandboxViolation,
            Self::ApprovalRequired(_) => CapabilityErrorCode::ApprovalRequired,
            Self::QuotaExceeded(_) => CapabilityErrorCode::QuotaExceeded,
        }
    }

    /// Create a not-found error.
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }

    /// Create an already-registered error.
    pub fn already_registered(msg: impl Into<String>) -> Self {
        Self::AlreadyRegistered(msg.into())
    }

    /// Create an invalid-state error.
    pub fn invalid_state(msg: impl Into<String>) -> Self {
        Self::InvalidState(msg.into())
    }

    /// Create an execution-failed error.
    pub fn execution_failed(msg: impl Into<String>) -> Self {
        Self::ExecutionFailed(msg.into())
    }

    /// Create a validation-failed error.
    pub fn validation_failed(msg: impl Into<String>) -> Self {
        Self::ValidationFailed(msg.into())
    }

    /// Create a permission-denied error.
    pub fn permission_denied(msg: impl Into<String>) -> Self {
        Self::PermissionDenied(msg.into())
    }

    /// Create a timeout error.
    pub fn timeout(msg: impl Into<String>) -> Self {
        Self::Timeout(msg.into())
    }

    /// Create a cancelled error.
    pub fn cancelled(msg: impl Into<String>) -> Self {
        Self::Cancelled(msg.into())
    }

    /// Create a dependency-missing error.
    pub fn dependency_missing(msg: impl Into<String>) -> Self {
        Self::DependencyMissing(msg.into())
    }

    /// Create a conflict-detected error.
    pub fn conflict_detected(msg: impl Into<String>) -> Self {
        Self::ConflictDetected(msg.into())
    }

    /// Create a resource-exhausted error.
    pub fn resource_exhausted(msg: impl Into<String>) -> Self {
        Self::ResourceExhausted(msg.into())
    }

    /// Create a composition-failed error.
    pub fn composition_failed(msg: impl Into<String>) -> Self {
        Self::CompositionFailed(msg.into())
    }

    /// Create a marketplace error.
    pub fn marketplace(msg: impl Into<String>) -> Self {
        Self::MarketplaceError(msg.into())
    }

    /// Create a signing error.
    pub fn signing(msg: impl Into<String>) -> Self {
        Self::SigningError(msg.into())
    }

    /// Create a version-incompatible error.
    pub fn version_incompatible(msg: impl Into<String>) -> Self {
        Self::VersionIncompatible(msg.into())
    }

    /// Create a discovery-failed error.
    pub fn discovery_failed(msg: impl Into<String>) -> Self {
        Self::DiscoveryFailed(msg.into())
    }

    /// Create a hot-reload-failed error.
    pub fn hot_reload_failed(msg: impl Into<String>) -> Self {
        Self::HotReloadFailed(msg.into())
    }

    /// Create a sandbox-violation error.
    pub fn sandbox_violation(msg: impl Into<String>) -> Self {
        Self::SandboxViolation(msg.into())
    }

    /// Create an approval-required error.
    pub fn approval_required(msg: impl Into<String>) -> Self {
        Self::ApprovalRequired(msg.into())
    }

    /// Create a quota-exceeded error.
    pub fn quota_exceeded(msg: impl Into<String>) -> Self {
        Self::QuotaExceeded(msg.into())
    }
}

impl fmt::Display for CapabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(msg) => write!(f, "[capability not found] {}", msg),
            Self::AlreadyRegistered(msg) => write!(f, "[already registered] {}", msg),
            Self::InvalidState(msg) => write!(f, "[invalid state] {}", msg),
            Self::ExecutionFailed(msg) => write!(f, "[execution failed] {}", msg),
            Self::ValidationFailed(msg) => write!(f, "[validation failed] {}", msg),
            Self::PermissionDenied(msg) => write!(f, "[permission denied] {}", msg),
            Self::Timeout(msg) => write!(f, "[timeout] {}", msg),
            Self::Cancelled(msg) => write!(f, "[cancelled] {}", msg),
            Self::DependencyMissing(msg) => write!(f, "[dependency missing] {}", msg),
            Self::ConflictDetected(msg) => write!(f, "[conflict detected] {}", msg),
            Self::ResourceExhausted(msg) => write!(f, "[resource exhausted] {}", msg),
            Self::CompositionFailed(msg) => write!(f, "[composition failed] {}", msg),
            Self::MarketplaceError(msg) => write!(f, "[marketplace error] {}", msg),
            Self::SigningError(msg) => write!(f, "[signing error] {}", msg),
            Self::VersionIncompatible(msg) => write!(f, "[version incompatible] {}", msg),
            Self::DiscoveryFailed(msg) => write!(f, "[discovery failed] {}", msg),
            Self::HotReloadFailed(msg) => write!(f, "[hot reload failed] {}", msg),
            Self::SandboxViolation(msg) => write!(f, "[sandbox violation] {}", msg),
            Self::ApprovalRequired(msg) => write!(f, "[approval required] {}", msg),
            Self::QuotaExceeded(msg) => write!(f, "[quota exceeded] {}", msg),
        }
    }
}

impl std::error::Error for CapabilityError {}

impl From<CapabilityError> for neo_core::error::NeoError {
    fn from(e: CapabilityError) -> Self {
        match e {
            CapabilityError::NotFound(msg) => neo_core::error::NeoError::NotFound(msg),
            CapabilityError::PermissionDenied(msg) => {
                neo_core::error::NeoError::PermissionDenied(msg)
            }
            CapabilityError::AlreadyRegistered(msg) => {
                neo_core::error::NeoError::AlreadyExists(msg)
            }
            CapabilityError::Timeout(msg) => neo_core::error::NeoError::Timeout(msg),
            CapabilityError::Cancelled(msg) => neo_core::error::NeoError::Cancelled(msg),
            CapabilityError::ResourceExhausted(msg) => {
                neo_core::error::NeoError::ResourceExhausted(msg)
            }
            _ => neo_core::error::NeoError::Internal(e.to_string()),
        }
    }
}

/// Result type alias for capability operations.
pub type CapabilityResult<T> = Result<T, CapabilityError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes() {
        let err = CapabilityError::not_found("test");
        assert_eq!(err.code(), CapabilityErrorCode::NotFound);

        let err = CapabilityError::permission_denied("test");
        assert_eq!(err.code(), CapabilityErrorCode::PermissionDenied);

        let err = CapabilityError::timeout("test");
        assert_eq!(err.code(), CapabilityErrorCode::Timeout);
    }

    #[test]
    fn error_display() {
        let err = CapabilityError::not_found("my-cap");
        let msg = format!("{}", err);
        assert!(msg.contains("not found"));
        assert!(msg.contains("my-cap"));
    }

    #[test]
    fn error_into_neo_error() {
        let err = CapabilityError::not_found("x");
        let neo_err: neo_core::error::NeoError = err.into();
        assert_eq!(neo_err.code(), neo_core::error::ErrorCode::NotFound);

        let err = CapabilityError::timeout("y");
        let neo_err: neo_core::error::NeoError = err.into();
        assert_eq!(neo_err.code(), neo_core::error::ErrorCode::Timeout);
    }

    #[test]
    fn all_codes_display() {
        let codes = [
            CapabilityErrorCode::NotFound,
            CapabilityErrorCode::AlreadyRegistered,
            CapabilityErrorCode::InvalidState,
            CapabilityErrorCode::ExecutionFailed,
            CapabilityErrorCode::ValidationFailed,
            CapabilityErrorCode::PermissionDenied,
            CapabilityErrorCode::Timeout,
            CapabilityErrorCode::Cancelled,
            CapabilityErrorCode::DependencyMissing,
            CapabilityErrorCode::ConflictDetected,
            CapabilityErrorCode::ResourceExhausted,
            CapabilityErrorCode::CompositionFailed,
            CapabilityErrorCode::MarketplaceError,
            CapabilityErrorCode::SigningError,
            CapabilityErrorCode::VersionIncompatible,
            CapabilityErrorCode::DiscoveryFailed,
            CapabilityErrorCode::HotReloadFailed,
            CapabilityErrorCode::SandboxViolation,
            CapabilityErrorCode::ApprovalRequired,
            CapabilityErrorCode::QuotaExceeded,
        ];
        for code in codes {
            let s = format!("{}", code);
            assert!(!s.is_empty());
        }
    }
}
