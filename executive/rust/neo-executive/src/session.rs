use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::goal::{Goal, GoalId, GoalPriority, GoalState};
use crate::task::{Task, TaskId, TaskState};
use crate::error::{ExecutiveError, ExecutiveResult};

/// Unique identifier for an executive session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub Uuid);

impl SessionId {
    /// Create a new session identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// State of an executive session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SessionState {
    Created,
    Active,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

impl SessionState {
    /// Check if the session is in a terminal state.
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            SessionState::Completed | SessionState::Failed | SessionState::Cancelled
        )
    }
}

/// A snapshot of session state for persistence and inspection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSnapshot {
    pub session_id: SessionId,
    pub state: SessionState,
    pub goal_ids: Vec<GoalId>,
    pub task_ids: Vec<TaskId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// An executive session groups related goals and tasks under a single context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub state: SessionState,
    pub goal_ids: Vec<GoalId>,
    pub task_ids: Vec<TaskId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Session {
    /// Create a new session.
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            id: SessionId::new(),
            state: SessionState::Created,
            goal_ids: Vec::new(),
            task_ids: Vec::new(),
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
        }
    }

    /// Activate the session.
    pub fn activate(&mut self) -> ExecutiveResult<()> {
        if self.state != SessionState::Created && self.state != SessionState::Paused {
            return Err(ExecutiveError::internal(format!(
                "cannot activate session in state {:?}",
                self.state
            )));
        }
        self.state = SessionState::Active;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Pause the session.
    pub fn pause(&mut self) -> ExecutiveResult<()> {
        if self.state != SessionState::Active {
            return Err(ExecutiveError::internal(format!(
                "cannot pause session in state {:?}",
                self.state
            )));
        }
        self.state = SessionState::Paused;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Complete the session.
    pub fn complete(&mut self) -> ExecutiveResult<()> {
        if self.state.is_terminal() {
            return Err(ExecutiveError::internal(format!(
                "cannot complete session in terminal state {:?}",
                self.state
            )));
        }
        self.state = SessionState::Completed;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Fail the session.
    pub fn fail(&mut self) -> ExecutiveResult<()> {
        if self.state.is_terminal() {
            return Err(ExecutiveError::internal(format!(
                "cannot fail session in terminal state {:?}",
                self.state
            )));
        }
        self.state = SessionState::Failed;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Cancel the session.
    pub fn cancel(&mut self) -> ExecutiveResult<()> {
        if self.state.is_terminal() {
            return Err(ExecutiveError::internal(format!(
                "cannot cancel session in terminal state {:?}",
                self.state
            )));
        }
        self.state = SessionState::Cancelled;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Add a goal to the session.
    pub fn add_goal(&mut self, goal_id: GoalId) {
        if !self.goal_ids.contains(&goal_id) {
            self.goal_ids.push(goal_id);
            self.updated_at = Utc::now();
        }
    }

    /// Remove a goal from the session.
    pub fn remove_goal(&mut self, goal_id: GoalId) {
        self.goal_ids.retain(|id| *id != goal_id);
        self.updated_at = Utc::now();
    }

    /// Add a task to the session.
    pub fn add_task(&mut self, task_id: TaskId) {
        if !self.task_ids.contains(&task_id) {
            self.task_ids.push(task_id);
            self.updated_at = Utc::now();
        }
    }

    /// Remove a task from the session.
    pub fn remove_task(&mut self, task_id: TaskId) {
        self.task_ids.retain(|id| *id != task_id);
        self.updated_at = Utc::now();
    }

    /// Set metadata.
    pub fn set_metadata(&mut self, key: String, value: serde_json::Value) {
        self.metadata.insert(key, value);
        self.updated_at = Utc::now();
    }

    /// Get a snapshot of this session.
    pub fn snapshot(&self) -> SessionSnapshot {
        SessionSnapshot {
            session_id: self.id,
            state: self.state,
            goal_ids: self.goal_ids.clone(),
            task_ids: self.task_ids.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
            metadata: self.metadata.clone(),
        }
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe session manager that tracks all executive sessions.
#[derive(Clone)]
pub struct SessionManager {
    inner: Arc<SessionManagerInner>,
}

struct SessionManagerInner {
    sessions: RwLock<HashMap<SessionId, Session>>,
}

impl SessionManager {
    /// Create a new session manager.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(SessionManagerInner {
                sessions: RwLock::new(HashMap::new()),
            }),
        }
    }

    /// Create and register a new session.
    pub fn create_session(&self) -> Session {
        let session = Session::new();
        let id = session.id;
        self.inner.sessions.write().insert(id, session.clone());
        session
    }

    /// Get a session by ID.
    pub fn get_session(&self, id: SessionId) -> Option<Session> {
        self.inner.sessions.read().get(&id).cloned()
    }

    /// Update a session.
    pub fn update_session(&self, session: Session) {
        self.inner.sessions.write().insert(session.id, session);
    }

    /// Remove a session.
    pub fn remove_session(&self, id: SessionId) -> Option<Session> {
        self.inner.sessions.write().remove(&id)
    }

    /// List all sessions.
    pub fn list_sessions(&self) -> Vec<Session> {
        self.inner.sessions.read().values().cloned().collect()
    }

    /// List active sessions.
    pub fn active_sessions(&self) -> Vec<Session> {
        self.inner
            .sessions
            .read()
            .values()
            .filter(|s| s.state == SessionState::Active)
            .cloned()
            .collect()
    }

    /// Get the count of sessions.
    pub fn session_count(&self) -> usize {
        self.inner.sessions.read().len()
    }

    /// Get session count by state.
    pub fn sessions_by_state(&self) -> HashMap<SessionState, usize> {
        let mut counts = HashMap::new();
        for session in self.inner.sessions.read().values() {
            *counts.entry(session.state).or_insert(0) += 1;
        }
        counts
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_creation() {
        let mgr = SessionManager::new();
        let session = mgr.create_session();
        assert_eq!(session.state, SessionState::Created);
        assert_eq!(mgr.session_count(), 1);
    }

    #[test]
    fn session_lifecycle() {
        let mut session = Session::new();
        session.activate().unwrap();
        assert_eq!(session.state, SessionState::Active);

        session.pause().unwrap();
        assert_eq!(session.state, SessionState::Paused);

        session.activate().unwrap();
        session.complete().unwrap();
        assert!(session.state.is_terminal());
    }

    #[test]
    fn session_goal_tracking() {
        let mut session = Session::new();
        let gid = GoalId::new();
        session.add_goal(gid);
        assert_eq!(session.goal_ids.len(), 1);
        session.remove_goal(gid);
        assert!(session.goal_ids.is_empty());
    }

    #[test]
    fn session_task_tracking() {
        let mut session = Session::new();
        let tid = TaskId::new();
        session.add_task(tid);
        assert_eq!(session.task_ids.len(), 1);
        session.remove_task(tid);
        assert!(session.task_ids.is_empty());
    }

    #[test]
    fn session_snapshot() {
        let session = Session::new();
        let snap = session.snapshot();
        assert_eq!(snap.session_id, session.id);
        assert_eq!(snap.state, SessionState::Created);
    }

    #[test]
    fn session_manager_list_active() {
        let mgr = SessionManager::new();
        let mut s1 = mgr.create_session();
        s1.activate().unwrap();
        mgr.update_session(s1);

        let _s2 = mgr.create_session();
        assert_eq!(mgr.active_sessions().len(), 1);
    }
}
