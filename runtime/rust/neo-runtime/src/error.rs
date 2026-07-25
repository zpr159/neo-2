//! Runtime error types with context, backtrace, logging, and recovery support.

use std::backtrace::Backtrace;
use std::fmt;

use serde::{Deserialize, Serialize};
use tracing::error;

/// Suggested recovery action for an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RecoveryAction {
    Retry,
    RestartService,
    Failover,
    Restart,
    Ignore,
    Abort,
}

impl fmt::Display for RecoveryAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Retry => write!(f, "retry"),
            Self::RestartService => write!(f, "restart_service"),
            Self::Failover => write!(f, "failover"),
            Self::Restart => write!(f, "restart"),
            Self::Ignore => write!(f, "ignore"),
            Self::Abort => write!(f, "abort"),
        }
    }
}

/// Classification of runtime errors.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuntimeErrorKind {
    Lifecycle(LifecycleErrorKind),
    Scheduler(SchedulerErrorKind),
    Dependency(DependencyErrorKind),
    Plugin(PluginErrorKind),
    Resource(ResourceErrorKind),
    Timeout(TimeoutErrorKind),
    Service(String),
    Config(String),
    Shutdown(String),
    Unknown,
}

impl fmt::Display for RuntimeErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lifecycle(e) => write!(f, "lifecycle: {}", e),
            Self::Scheduler(e) => write!(f, "scheduler: {}", e),
            Self::Dependency(e) => write!(f, "dependency: {}", e),
            Self::Plugin(e) => write!(f, "plugin: {}", e),
            Self::Resource(e) => write!(f, "resource: {}", e),
            Self::Timeout(e) => write!(f, "timeout: {}", e),
            Self::Service(msg) => write!(f, "service: {}", msg),
            Self::Config(msg) => write!(f, "config: {}", msg),
            Self::Shutdown(msg) => write!(f, "shutdown: {}", msg),
            Self::Unknown => write!(f, "unknown runtime error"),
        }
    }
}

/// The primary runtime error type.
///
/// Carries error classification, human-readable message, optional source error,
/// chain of context strings, a backtrace, and a suggested recovery action.
pub struct RuntimeError {
    kind: RuntimeErrorKind,
    message: String,
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
    context: Vec<String>,
    backtrace: Backtrace,
    recovery: RecoveryAction,
}

impl RuntimeError {
    /// Create a new error with the given kind and message.
    pub fn new(kind: RuntimeErrorKind, message: impl Into<String>) -> Self {
        let msg = message.into();
        error!(kind = ?kind, message = %msg, "runtime error");
        Self {
            kind,
            message: msg,
            source: None,
            context: Vec::new(),
            backtrace: Backtrace::capture(),
            recovery: RecoveryAction::Abort,
        }
    }

    /// Wrap a source error.
    #[must_use]
    pub fn with_source(mut self, source: impl std::error::Error + Send + Sync + 'static) -> Self {
        self.source = Some(Box::new(source));
        self
    }

    /// Add a context string to the error chain.
    #[must_use]
    pub fn with_context(mut self, ctx: impl Into<String>) -> Self {
        self.context.push(ctx.into());
        self
    }

    /// Set the suggested recovery action.
    #[must_use]
    pub fn with_recovery(mut self, recovery: RecoveryAction) -> Self {
        self.recovery = recovery;
        self
    }

    /// Create a lifecycle error.
    pub fn lifecycle(message: impl Into<String>) -> Self {
        Self::new(
            RuntimeErrorKind::Lifecycle(LifecycleErrorKind::InvalidTransition),
            message,
        )
    }

    /// Create a scheduler error.
    pub fn scheduler(message: impl Into<String>) -> Self {
        Self::new(
            RuntimeErrorKind::Scheduler(SchedulerErrorKind::TaskRejected),
            message,
        )
    }

    /// Create a dependency error.
    pub fn dependency(message: impl Into<String>) -> Self {
        Self::new(
            RuntimeErrorKind::Dependency(DependencyErrorKind::CircularDependency),
            message,
        )
    }

    /// Create a plugin error.
    pub fn plugin(message: impl Into<String>) -> Self {
        Self::new(
            RuntimeErrorKind::Plugin(PluginErrorKind::LoadFailed),
            message,
        )
    }

    /// Create a resource error.
    pub fn resource(message: impl Into<String>) -> Self {
        Self::new(
            RuntimeErrorKind::Resource(ResourceErrorKind::Exhausted),
            message,
        )
    }

    /// Create a timeout error.
    pub fn timeout(message: impl Into<String>) -> Self {
        Self::new(
            RuntimeErrorKind::Timeout(TimeoutErrorKind::OperationTimeout),
            message,
        )
    }

    /// Access the error kind.
    pub fn kind(&self) -> &RuntimeErrorKind {
        &self.kind
    }

    /// Access the error message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Access the context chain.
    pub fn context(&self) -> &[String] {
        &self.context
    }

    /// Access the backtrace.
    pub fn backtrace(&self) -> &Backtrace {
        &self.backtrace
    }

    /// Access the recovery action.
    pub fn recovery(&self) -> RecoveryAction {
        self.recovery
    }
}

impl fmt::Debug for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RuntimeError")
            .field("kind", &self.kind)
            .field("message", &self.message)
            .field("context", &self.context)
            .field("recovery", &self.recovery)
            .finish_non_exhaustive()
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.kind, self.message)?;
        for ctx in &self.context {
            write!(f, "\n  -> {}", ctx)?;
        }
        Ok(())
    }
}

impl std::error::Error for RuntimeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_ref()
            .map(|e| e.as_ref() as &(dyn std::error::Error + 'static))
    }
}

impl From<std::io::Error> for RuntimeError {
    fn from(e: std::io::Error) -> Self {
        Self::new(
            RuntimeErrorKind::Unknown,
            format!("io error: {}", e),
        )
        .with_source(e)
    }
}

impl From<serde_json::Error> for RuntimeError {
    fn from(e: serde_json::Error) -> Self {
        Self::new(
            RuntimeErrorKind::Unknown,
            format!("serialization error: {}", e),
        )
        .with_source(e)
    }
}

// ---------------------------------------------------------------------------
// Lifecycle errors
// ---------------------------------------------------------------------------

/// Specific lifecycle failure modes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LifecycleErrorKind {
    InvalidTransition,
    ServiceNotFound,
    AlreadyInitialized,
    NotInitialized,
    StartupFailed,
    ShutdownFailed,
    HealthCheckFailed,
}

impl fmt::Display for LifecycleErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTransition => write!(f, "invalid state transition"),
            Self::ServiceNotFound => write!(f, "service not found"),
            Self::AlreadyInitialized => write!(f, "already initialized"),
            Self::NotInitialized => write!(f, "not initialized"),
            Self::StartupFailed => write!(f, "startup failed"),
            Self::ShutdownFailed => write!(f, "shutdown failed"),
            Self::HealthCheckFailed => write!(f, "health check failed"),
        }
    }
}

/// Error originating from lifecycle management.
#[derive(Debug)]
pub struct LifecycleError {
    pub kind: LifecycleErrorKind,
    pub message: String,
    pub context: Vec<String>,
    pub backtrace: Backtrace,
}

impl LifecycleError {
    pub fn new(kind: LifecycleErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            context: Vec::new(),
            backtrace: Backtrace::capture(),
        }
    }

    #[must_use]
    pub fn with_context(mut self, ctx: impl Into<String>) -> Self {
        self.context.push(ctx.into());
        self
    }
}

impl fmt::Display for LifecycleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[lifecycle] {}: {}", self.kind, self.message)
    }
}

impl std::error::Error for LifecycleError {}

impl From<LifecycleError> for RuntimeError {
    fn from(e: LifecycleError) -> Self {
        let msg = e.message.clone();
        Self::new(RuntimeErrorKind::Lifecycle(e.kind), msg)
            .with_context(e.context.join(" -> "))
    }
}

// ---------------------------------------------------------------------------
// Scheduler errors
// ---------------------------------------------------------------------------

/// Specific scheduler failure modes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SchedulerErrorKind {
    TaskRejected,
    QueueFull,
    WorkerPanic,
    DeadlineExceeded,
    DeadlockDetected,
    TaskCancelled,
}

impl fmt::Display for SchedulerErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TaskRejected => write!(f, "task rejected"),
            Self::QueueFull => write!(f, "queue full"),
            Self::WorkerPanic => write!(f, "worker panicked"),
            Self::DeadlineExceeded => write!(f, "deadline exceeded"),
            Self::DeadlockDetected => write!(f, "deadlock detected"),
            Self::TaskCancelled => write!(f, "task cancelled"),
        }
    }
}

/// Error originating from task scheduling.
#[derive(Debug)]
pub struct SchedulerError {
    pub kind: SchedulerErrorKind,
    pub message: String,
    pub context: Vec<String>,
    pub backtrace: Backtrace,
}

impl SchedulerError {
    pub fn new(kind: SchedulerErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            context: Vec::new(),
            backtrace: Backtrace::capture(),
        }
    }

    #[must_use]
    pub fn with_context(mut self, ctx: impl Into<String>) -> Self {
        self.context.push(ctx.into());
        self
    }
}

impl fmt::Display for SchedulerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[scheduler] {}: {}", self.kind, self.message)
    }
}

impl std::error::Error for SchedulerError {}

impl From<SchedulerError> for RuntimeError {
    fn from(e: SchedulerError) -> Self {
        let msg = e.message.clone();
        Self::new(RuntimeErrorKind::Scheduler(e.kind), msg)
            .with_context(e.context.join(" -> "))
    }
}

// ---------------------------------------------------------------------------
// Dependency errors
// ---------------------------------------------------------------------------

/// Specific dependency resolution failure modes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DependencyErrorKind {
    CircularDependency,
    MissingDependency,
    VersionMismatch,
    ResolutionFailed,
    OptionalDependencyFailed,
}

impl fmt::Display for DependencyErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CircularDependency => write!(f, "circular dependency"),
            Self::MissingDependency => write!(f, "missing dependency"),
            Self::VersionMismatch => write!(f, "version mismatch"),
            Self::ResolutionFailed => write!(f, "resolution failed"),
            Self::OptionalDependencyFailed => write!(f, "optional dependency failed"),
        }
    }
}

/// Error originating from dependency resolution.
#[derive(Debug)]
pub struct DependencyError {
    pub kind: DependencyErrorKind,
    pub message: String,
    pub context: Vec<String>,
    pub backtrace: Backtrace,
}

impl DependencyError {
    pub fn new(kind: DependencyErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            context: Vec::new(),
            backtrace: Backtrace::capture(),
        }
    }

    #[must_use]
    pub fn with_context(mut self, ctx: impl Into<String>) -> Self {
        self.context.push(ctx.into());
        self
    }
}

impl fmt::Display for DependencyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[dependency] {}: {}", self.kind, self.message)
    }
}

impl std::error::Error for DependencyError {}

impl From<DependencyError> for RuntimeError {
    fn from(e: DependencyError) -> Self {
        let msg = e.message.clone();
        Self::new(RuntimeErrorKind::Dependency(e.kind), msg)
            .with_context(e.context.join(" -> "))
    }
}

// ---------------------------------------------------------------------------
// Plugin errors
// ---------------------------------------------------------------------------

/// Specific plugin failure modes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PluginErrorKind {
    LoadFailed,
    SymbolNotFound,
    VerificationFailed,
    SandboxViolation,
    HotReloadFailed,
    UnloadFailed,
    InitializationFailed,
}

impl fmt::Display for PluginErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LoadFailed => write!(f, "load failed"),
            Self::SymbolNotFound => write!(f, "symbol not found"),
            Self::VerificationFailed => write!(f, "verification failed"),
            Self::SandboxViolation => write!(f, "sandbox violation"),
            Self::HotReloadFailed => write!(f, "hot reload failed"),
            Self::UnloadFailed => write!(f, "unload failed"),
            Self::InitializationFailed => write!(f, "initialization failed"),
        }
    }
}

/// Error originating from plugin operations.
#[derive(Debug)]
pub struct PluginError {
    pub kind: PluginErrorKind,
    pub message: String,
    pub context: Vec<String>,
    pub backtrace: Backtrace,
}

impl PluginError {
    pub fn new(kind: PluginErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            context: Vec::new(),
            backtrace: Backtrace::capture(),
        }
    }

    #[must_use]
    pub fn with_context(mut self, ctx: impl Into<String>) -> Self {
        self.context.push(ctx.into());
        self
    }
}

impl fmt::Display for PluginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[plugin] {}: {}", self.kind, self.message)
    }
}

impl std::error::Error for PluginError {}

impl From<PluginError> for RuntimeError {
    fn from(e: PluginError) -> Self {
        let msg = e.message.clone();
        Self::new(RuntimeErrorKind::Plugin(e.kind), msg)
            .with_context(e.context.join(" -> "))
    }
}

// ---------------------------------------------------------------------------
// Resource errors
// ---------------------------------------------------------------------------

/// Specific resource management failure modes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceErrorKind {
    Exhausted,
    NotFound,
    QuotaExceeded,
    AllocationFailed,
    DeallocationFailed,
}

impl fmt::Display for ResourceErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exhausted => write!(f, "exhausted"),
            Self::NotFound => write!(f, "not found"),
            Self::QuotaExceeded => write!(f, "quota exceeded"),
            Self::AllocationFailed => write!(f, "allocation failed"),
            Self::DeallocationFailed => write!(f, "deallocation failed"),
        }
    }
}

/// Error originating from resource management.
#[derive(Debug)]
pub struct ResourceError {
    pub kind: ResourceErrorKind,
    pub message: String,
    pub context: Vec<String>,
    pub backtrace: Backtrace,
}

impl ResourceError {
    pub fn new(kind: ResourceErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            context: Vec::new(),
            backtrace: Backtrace::capture(),
        }
    }

    #[must_use]
    pub fn with_context(mut self, ctx: impl Into<String>) -> Self {
        self.context.push(ctx.into());
        self
    }
}

impl fmt::Display for ResourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[resource] {}: {}", self.kind, self.message)
    }
}

impl std::error::Error for ResourceError {}

impl From<ResourceError> for RuntimeError {
    fn from(e: ResourceError) -> Self {
        let msg = e.message.clone();
        Self::new(RuntimeErrorKind::Resource(e.kind), msg)
            .with_context(e.context.join(" -> "))
    }
}

// ---------------------------------------------------------------------------
// Timeout errors
// ---------------------------------------------------------------------------

/// Specific timeout failure modes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TimeoutErrorKind {
    OperationTimeout,
    StartupTimeout,
    ShutdownTimeout,
    HealthCheckTimeout,
    SchedulerTimeout,
}

impl fmt::Display for TimeoutErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OperationTimeout => write!(f, "operation timeout"),
            Self::StartupTimeout => write!(f, "startup timeout"),
            Self::ShutdownTimeout => write!(f, "shutdown timeout"),
            Self::HealthCheckTimeout => write!(f, "health check timeout"),
            Self::SchedulerTimeout => write!(f, "scheduler timeout"),
        }
    }
}

/// Error originating from a timeout.
#[derive(Debug)]
pub struct TimeoutError {
    pub kind: TimeoutErrorKind,
    pub message: String,
    pub duration_ms: u64,
    pub context: Vec<String>,
    pub backtrace: Backtrace,
}

impl TimeoutError {
    pub fn new(kind: TimeoutErrorKind, message: impl Into<String>, duration_ms: u64) -> Self {
        Self {
            kind,
            message: message.into(),
            duration_ms,
            context: Vec::new(),
            backtrace: Backtrace::capture(),
        }
    }

    #[must_use]
    pub fn with_context(mut self, ctx: impl Into<String>) -> Self {
        self.context.push(ctx.into());
        self
    }
}

impl fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[timeout] {} after {}ms: {}",
            self.kind, self.duration_ms, self.message
        )
    }
}

impl std::error::Error for TimeoutError {}

impl From<TimeoutError> for RuntimeError {
    fn from(e: TimeoutError) -> Self {
        let msg = e.message.clone();
        Self::new(RuntimeErrorKind::Timeout(e.kind), msg)
            .with_context(e.context.join(" -> "))
    }
}

/// Convenience result alias for runtime operations.
pub type RuntimeResult<T> = Result<T, RuntimeError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_creation_and_display() {
        let err = RuntimeError::new(RuntimeErrorKind::Unknown, "something broke");
        assert_eq!(err.message(), "something broke");
        assert!(format!("{}", err).contains("something broke"));
    }

    #[test]
    fn error_with_context() {
        let err = RuntimeError::scheduler("task failed")
            .with_context("while processing batch")
            .with_context("in worker 3");
        assert_eq!(err.context().len(), 2);
        assert_eq!(err.context()[0], "while processing batch");
        assert_eq!(err.context()[1], "in worker 3");
    }

    #[test]
    fn error_with_recovery() {
        let err = RuntimeError::timeout("op took too long").with_recovery(RecoveryAction::Retry);
        assert_eq!(err.recovery(), RecoveryAction::Retry);
    }

    #[test]
    fn error_with_source() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let err = RuntimeError::new(RuntimeErrorKind::Unknown, "io failure").with_source(io_err);
        assert!(err.source().is_some());
    }

    #[test]
    fn lifecycle_error_conversion() {
        let err = LifecycleError::new(LifecycleErrorKind::InvalidTransition, "bad transition");
        let runtime_err: RuntimeError = err.into();
        assert!(matches!(
            runtime_err.kind(),
            RuntimeErrorKind::Lifecycle(LifecycleErrorKind::InvalidTransition)
        ));
    }

    #[test]
    fn scheduler_error_conversion() {
        let err = SchedulerError::new(SchedulerErrorKind::QueueFull, "queue overflow");
        let runtime_err: RuntimeError = err.into();
        assert!(matches!(
            runtime_err.kind(),
            RuntimeErrorKind::Scheduler(SchedulerErrorKind::QueueFull)
        ));
    }

    #[test]
    fn dependency_error_conversion() {
        let err =
            DependencyError::new(DependencyErrorKind::CircularDependency, "A->B->A");
        let runtime_err: RuntimeError = err.into();
        assert!(matches!(
            runtime_err.kind(),
            RuntimeErrorKind::Dependency(DependencyErrorKind::CircularDependency)
        ));
    }

    #[test]
    fn plugin_error_conversion() {
        let err = PluginError::new(PluginErrorKind::LoadFailed, "dlopen failed");
        let runtime_err: RuntimeError = err.into();
        assert!(matches!(
            runtime_err.kind(),
            RuntimeErrorKind::Plugin(PluginErrorKind::LoadFailed)
        ));
    }

    #[test]
    fn resource_error_conversion() {
        let err = ResourceError::new(ResourceErrorKind::Exhausted, "no memory");
        let runtime_err: RuntimeError = err.into();
        assert!(matches!(
            runtime_err.kind(),
            RuntimeErrorKind::Resource(ResourceErrorKind::Exhausted)
        ));
    }

    #[test]
    fn timeout_error_conversion() {
        let err =
            TimeoutError::new(TimeoutErrorKind::OperationTimeout, "deadline hit", 5000);
        let runtime_err: RuntimeError = err.into();
        assert!(matches!(
            runtime_err.kind(),
            RuntimeErrorKind::Timeout(TimeoutErrorKind::OperationTimeout)
        ));
    }

    #[test]
    fn error_backtrace_is_captured() {
        let err = RuntimeError::new(RuntimeErrorKind::Unknown, "test");
        let bt = err.backtrace();
        assert!(!format!("{:?}", bt).is_empty());
    }

    #[test]
    fn recovery_action_display() {
        assert_eq!(RecoveryAction::Retry.to_string(), "retry");
        assert_eq!(RecoveryAction::Abort.to_string(), "abort");
        assert_eq!(RecoveryAction::RestartService.to_string(), "restart_service");
    }
}
