//! Tool lifecycle state machine with validated transitions.

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::error::{ToolError, ToolResult};

/// Lifecycle states for a tool within the registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolLifecycleState {
    /// Tool manifest has been registered but not loaded.
    Registered,
    /// Tool source/binary is being loaded.
    Loading,
    /// Tool is loaded into memory.
    Loaded,
    /// Tool is performing initialization.
    Initializing,
    /// Tool is ready to accept executions.
    Ready,
    /// Tool is actively executing a request.
    Running,
    /// Tool is executing but marked busy (concurrent limit).
    Busy,
    /// Tool is temporarily paused.
    Paused,
    /// Tool is being updated.
    Updating,
    /// Tool is shutting down.
    Stopping,
    /// Tool has been stopped.
    Stopped,
    /// Tool has encountered a fatal error.
    Failed,
    /// Tool has been disabled by an administrator.
    Disabled,
    /// Tool is being unloaded from memory.
    Unloading,
}

impl ToolLifecycleState {
    /// Whether the tool can accept new executions.
    #[must_use]
    pub fn can_execute(&self) -> bool {
        matches!(self, Self::Ready)
    }

    /// Whether the tool is in a terminal state (cannot transition back).
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Stopped | Self::Failed | Self::Disabled)
    }

    /// Whether the tool is actively doing work.
    #[must_use]
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Running | Self::Busy)
    }

    /// Valid transitions from this state.
    #[must_use]
    pub fn valid_transitions(&self) -> &'static [ToolLifecycleState] {
        match self {
            Self::Registered => &[Self::Loading, Self::Disabled, Self::Failed],
            Self::Loading => &[Self::Loaded, Self::Failed, Self::Stopped],
            Self::Loaded => &[Self::Initializing, Self::Unloading, Self::Disabled],
            Self::Initializing => &[Self::Ready, Self::Failed, Self::Stopping],
            Self::Ready => &[
                Self::Running,
                Self::Paused,
                Self::Updating,
                Self::Stopping,
                Self::Disabled,
            ],
            Self::Running => &[Self::Ready, Self::Busy, Self::Stopping, Self::Failed],
            Self::Busy => &[Self::Ready, Self::Running, Self::Stopping, Self::Failed],
            Self::Paused => &[Self::Ready, Self::Stopping, Self::Disabled],
            Self::Updating => &[Self::Ready, Self::Loaded, Self::Failed],
            Self::Stopping => &[Self::Stopped, Self::Failed],
            Self::Stopped => &[Self::Loading, Self::Registered],
            Self::Failed => &[Self::Loading, Self::Registered, Self::Disabled],
            Self::Disabled => &[Self::Registered, Self::Stopped],
            Self::Unloading => &[Self::Stopped, Self::Registered],
        }
    }

    /// Attempt a transition to the target state, returning an error if invalid.
    pub fn transition_to(self, target: Self) -> ToolResult<Self> {
        if self.valid_transitions().contains(&target) {
            Ok(target)
        } else {
            Err(ToolError::lifecycle_violation(format!(
                "cannot transition from {:?} to {:?}",
                self, target
            )))
        }
    }
}

impl fmt::Display for ToolLifecycleState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Registered => "registered",
            Self::Loading => "loading",
            Self::Loaded => "loaded",
            Self::Initializing => "initializing",
            Self::Ready => "ready",
            Self::Running => "running",
            Self::Busy => "busy",
            Self::Paused => "paused",
            Self::Updating => "updating",
            Self::Stopping => "stopping",
            Self::Stopped => "stopped",
            Self::Failed => "failed",
            Self::Disabled => "disabled",
            Self::Unloading => "unloading",
        };
        write!(f, "{}", label)
    }
}

/// Tracks lifecycle state and transition history for a tool.
#[derive(Debug, Clone)]
pub struct LifecycleTracker {
    current: ToolLifecycleState,
    history: Vec<(ToolLifecycleState, chrono::DateTime<chrono::Utc>)>,
}

impl LifecycleTracker {
    pub fn new(initial: ToolLifecycleState) -> Self {
        Self {
            current: initial,
            history: vec![(initial, chrono::Utc::now())],
        }
    }

    pub fn current(&self) -> ToolLifecycleState {
        self.current
    }

    pub fn history(&self) -> &[(ToolLifecycleState, chrono::DateTime<chrono::Utc>)] {
        &self.history
    }

    pub fn transition(&mut self, target: ToolLifecycleState) -> ToolResult<()> {
        self.current = self.current.transition_to(target)?;
        self.history.push((target, chrono::Utc::now()));
        Ok(())
    }

    pub fn force_transition(&mut self, target: ToolLifecycleState) {
        self.current = target;
        self.history.push((target, chrono::Utc::now()));
    }
}

impl Default for LifecycleTracker {
    fn default() -> Self {
        Self::new(ToolLifecycleState::Registered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_transitions() {
        let result = ToolLifecycleState::Registered.transition_to(ToolLifecycleState::Loading);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ToolLifecycleState::Loading);
    }

    #[test]
    fn test_invalid_transition() {
        let result = ToolLifecycleState::Registered.transition_to(ToolLifecycleState::Running);
        assert!(result.is_err());
    }

    #[test]
    fn test_can_execute() {
        assert!(ToolLifecycleState::Ready.can_execute());
        assert!(!ToolLifecycleState::Running.can_execute());
        assert!(!ToolLifecycleState::Failed.can_execute());
    }

    #[test]
    fn test_is_terminal() {
        assert!(ToolLifecycleState::Stopped.is_terminal());
        assert!(ToolLifecycleState::Failed.is_terminal());
        assert!(!ToolLifecycleState::Ready.is_terminal());
    }

    #[test]
    fn test_lifecycle_tracker() {
        let mut tracker = LifecycleTracker::new(ToolLifecycleState::Registered);
        assert_eq!(tracker.current(), ToolLifecycleState::Registered);

        tracker.transition(ToolLifecycleState::Loading).unwrap();
        assert_eq!(tracker.current(), ToolLifecycleState::Loading);

        tracker.transition(ToolLifecycleState::Loaded).unwrap();
        assert_eq!(tracker.current(), ToolLifecycleState::Loaded);

        assert_eq!(tracker.history().len(), 3);
    }
}
