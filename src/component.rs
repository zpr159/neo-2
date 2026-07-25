use crate::error::NeoResult;
use std::fmt;

/// Lifecycle state of a Neo component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ComponentState {
    Created,
    Initializing,
    Running,
    Suspended,
    Stopping,
    Stopped,
    Failed,
}

impl fmt::Display for ComponentState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ComponentState::Created => write!(f, "created"),
            ComponentState::Initializing => write!(f, "initializing"),
            ComponentState::Running => write!(f, "running"),
            ComponentState::Suspended => write!(f, "suspended"),
            ComponentState::Stopping => write!(f, "stopping"),
            ComponentState::Stopped => write!(f, "stopped"),
            ComponentState::Failed => write!(f, "failed"),
        }
    }
}

/// Trait that all Neo components must implement.
///
/// Provides a uniform lifecycle interface for initialization, execution,
/// and shutdown of components within the Neo runtime.
pub trait Component: Send + Sync {
    /// Returns the human-readable name of this component.
    fn name(&self) -> &str;

    /// Returns the current lifecycle state.
    fn state(&self) -> ComponentState;

    /// Initialize the component. Called once after creation.
    async fn initialize(&mut self) -> NeoResult<()>;

    /// Start the component. Transitions from Initialized to Running.
    async fn start(&mut self) -> NeoResult<()>;

    /// Stop the component gracefully. Transitions to Stopped.
    async fn stop(&mut self) -> NeoResult<()>;

    /// Downcast helper for dynamic dispatch.
    fn as_any(&self) -> &dyn std::any::Any;
}
