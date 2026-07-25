use crate::core::WorkflowState;
use crate::error::{WorkflowError, WorkflowResult};

/// State machine for workflow lifecycle transitions.
///
/// Tracks valid transitions and enforces state invariants.
#[derive(Debug, Clone)]
pub struct WorkflowStateMachine {
    current: WorkflowState,
    history: Vec<StateTransition>,
}

/// Record of a single state transition.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StateTransition {
    pub from: WorkflowState,
    pub to: WorkflowState,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub reason: String,
}

impl WorkflowStateMachine {
    /// Create a new state machine starting at `Created`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            current: WorkflowState::Created,
            history: Vec::new(),
        }
    }

    /// Create a state machine starting at the given state.
    #[must_use]
    pub fn with_state(state: WorkflowState) -> Self {
        Self {
            current: state,
            history: Vec::new(),
        }
    }

    /// Get the current state.
    #[must_use]
    pub fn current(&self) -> WorkflowState {
        self.current
    }

    /// Attempt a transition, recording it in the history.
    pub fn transition(
        &mut self,
        target: WorkflowState,
        reason: impl Into<String>,
    ) -> WorkflowResult<WorkflowState> {
        let new_state = self.current.try_transition(target)?;
        self.history.push(StateTransition {
            from: self.current,
            to: new_state,
            timestamp: chrono::Utc::now(),
            reason: reason.into(),
        });
        self.current = new_state;
        Ok(self.current)
    }

    /// Force a transition (skip validation). Use only for recovery.
    pub fn force_transition(&mut self, target: WorkflowState, reason: impl Into<String>) {
        self.history.push(StateTransition {
            from: self.current,
            to: target,
            timestamp: chrono::Utc::now(),
            reason: reason.into(),
        });
        self.current = target;
    }

    /// Get the transition history.
    #[must_use]
    pub fn history(&self) -> &[StateTransition] {
        &self.history
    }

    /// Check if a transition to the target is valid without performing it.
    #[must_use]
    pub fn can_transition(&self, target: WorkflowState) -> bool {
        self.current.valid_transitions().contains(&target)
    }

    /// Check if the workflow is in a terminal state.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        self.current.is_terminal()
    }

    /// Reset to Created state (for re-execution).
    pub fn reset(&mut self) {
        self.force_transition(WorkflowState::Created, "reset for re-execution");
    }
}

impl Default for WorkflowStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state() {
        let sm = WorkflowStateMachine::new();
        assert_eq!(sm.current(), WorkflowState::Created);
        assert!(!sm.is_terminal());
    }

    #[test]
    fn valid_transitions() {
        let mut sm = WorkflowStateMachine::new();
        sm.transition(WorkflowState::Queued, "queued").unwrap();
        assert_eq!(sm.current(), WorkflowState::Queued);
        sm.transition(WorkflowState::Running, "start").unwrap();
        assert_eq!(sm.current(), WorkflowState::Running);
        sm.transition(WorkflowState::Completed, "done").unwrap();
        assert_eq!(sm.current(), WorkflowState::Completed);
        assert!(sm.is_terminal());
    }

    #[test]
    fn invalid_transition() {
        let mut sm = WorkflowStateMachine::new();
        let result = sm.transition(WorkflowState::Completed, "skip");
        assert!(result.is_err());
    }

    #[test]
    fn transition_history() {
        let mut sm = WorkflowStateMachine::new();
        sm.transition(WorkflowState::Queued, "q").unwrap();
        sm.transition(WorkflowState::Running, "r").unwrap();
        assert_eq!(sm.history().len(), 2);
        assert_eq!(sm.history()[0].from, WorkflowState::Created);
        assert_eq!(sm.history()[0].to, WorkflowState::Queued);
    }

    #[test]
    fn force_transition() {
        let mut sm = WorkflowStateMachine::new();
        sm.force_transition(WorkflowState::Completed, "force");
        assert_eq!(sm.current(), WorkflowState::Completed);
        assert_eq!(sm.history().len(), 1);
    }

    #[test]
    fn can_transition_check() {
        let sm = WorkflowStateMachine::new();
        assert!(sm.can_transition(WorkflowState::Queued));
        assert!(!sm.can_transition(WorkflowState::Running));
    }

    #[test]
    fn reset() {
        let mut sm = WorkflowStateMachine::new();
        sm.transition(WorkflowState::Queued, "q").unwrap();
        sm.transition(WorkflowState::Running, "r").unwrap();
        sm.reset();
        assert_eq!(sm.current(), WorkflowState::Created);
    }

    #[test]
    fn with_state_constructor() {
        let sm = WorkflowStateMachine::with_state(WorkflowState::Running);
        assert_eq!(sm.current(), WorkflowState::Running);
    }

    #[test]
    fn pause_resume_flow() {
        let mut sm = WorkflowStateMachine::new();
        sm.transition(WorkflowState::Queued, "q").unwrap();
        sm.transition(WorkflowState::Running, "r").unwrap();
        sm.transition(WorkflowState::Paused, "pause").unwrap();
        assert_eq!(sm.current(), WorkflowState::Paused);
        sm.transition(WorkflowState::Running, "resume").unwrap();
        assert_eq!(sm.current(), WorkflowState::Running);
    }

    #[test]
    fn cancel_from_running() {
        let mut sm = WorkflowStateMachine::new();
        sm.transition(WorkflowState::Queued, "q").unwrap();
        sm.transition(WorkflowState::Running, "r").unwrap();
        sm.transition(WorkflowState::Cancelled, "user cancel")
            .unwrap();
        assert!(sm.is_terminal());
    }

    #[test]
    fn rollback_flow() {
        let mut sm = WorkflowStateMachine::new();
        sm.transition(WorkflowState::Queued, "q").unwrap();
        sm.transition(WorkflowState::Running, "r").unwrap();
        sm.transition(WorkflowState::RollingBack, "failure")
            .unwrap();
        assert_eq!(sm.current(), WorkflowState::RollingBack);
        sm.transition(WorkflowState::Failed, "rollback done")
            .unwrap();
        assert!(sm.is_terminal());
    }
}
