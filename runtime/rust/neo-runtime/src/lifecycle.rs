//! Lifecycle manager with validated state transitions.
//!
//! Every service in the Neo runtime passes through a strict lifecycle.
//! Transitions are validated against a whitelist of allowed edges, and
//! the full history of state changes is retained for auditing.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{LifecycleError, LifecycleErrorKind};

/// Unique service identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ServiceId(pub Uuid);

impl ServiceId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ServiceId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ServiceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Lifecycle states for a managed service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ServiceState {
    /// Service has been created but not yet registered.
    Created,
    /// Service is registered in the registry.
    Registered,
    /// Service has been initialized (resources acquired).
    Initialized,
    /// Service is in the process of starting.
    Starting,
    /// Service is running and processing requests.
    Running,
    /// Service is paused (suspended but can resume).
    Paused,
    /// Service is in the process of stopping.
    Stopping,
    /// Service has stopped gracefully.
    Stopped,
    /// Service encountered a failure.
    Failed,
    /// Service is being restarted after a failure.
    Restarting,
    /// Service has been permanently destroyed.
    Destroyed,
}

impl ServiceState {
    /// Return the set of states this state can transition to.
    pub fn valid_targets(self) -> &'static [ServiceState] {
        use ServiceState::*;
        match self {
            Created => &[Registered],
            Registered => &[Initialized, Failed],
            Initialized => &[Starting, Failed, Destroyed],
            Starting => &[Running, Failed],
            Running => &[Paused, Stopping, Failed],
            Paused => &[Running, Stopping, Failed],
            Stopping => &[Stopped, Failed],
            Stopped => &[Destroyed, Restarting],
            Failed => &[Restarting, Destroyed],
            Restarting => &[Initialized, Starting, Running, Failed],
            Destroyed => &[],
        }
    }

    /// Check whether a transition from `self` to `target` is valid.
    pub fn can_transition_to(self, target: ServiceState) -> bool {
        self.valid_targets().contains(&target)
    }
}

impl fmt::Display for ServiceState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Created => write!(f, "created"),
            Self::Registered => write!(f, "registered"),
            Self::Initialized => write!(f, "initialized"),
            Self::Starting => write!(f, "starting"),
            Self::Running => write!(f, "running"),
            Self::Paused => write!(f, "paused"),
            Self::Stopping => write!(f, "stopping"),
            Self::Stopped => write!(f, "stopped"),
            Self::Failed => write!(f, "failed"),
            Self::Restarting => write!(f, "restarting"),
            Self::Destroyed => write!(f, "destroyed"),
        }
    }
}

/// Record of a single state transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    pub service_id: ServiceId,
    pub from: ServiceState,
    pub to: ServiceState,
    pub timestamp: DateTime<Utc>,
    pub reason: String,
}

/// A service tracked by the lifecycle manager.
#[derive(Debug, Clone)]
pub struct LifecycleService {
    pub id: ServiceId,
    pub name: String,
    pub state: ServiceState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub error_message: Option<String>,
}

impl LifecycleService {
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: ServiceId::new(),
            name: name.into(),
            state: ServiceState::Created,
            created_at: now,
            updated_at: now,
            error_message: None,
        }
    }
}

/// Thread-safe lifecycle manager.
///
/// Maintains service states, validates transitions, and records history.
#[derive(Clone)]
pub struct LifecycleManager {
    inner: Arc<LifecycleManagerInner>,
}

struct LifecycleManagerInner {
    services: RwLock<HashMap<ServiceId, LifecycleService>>,
    history: RwLock<Vec<StateTransition>>,
}

impl LifecycleManager {
    /// Create a new empty lifecycle manager.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(LifecycleManagerInner {
                services: RwLock::new(HashMap::new()),
                history: RwLock::new(Vec::new()),
            }),
        }
    }

    /// Register a new service in the `Created` state.
    pub fn register_service(&self, name: impl Into<String>) -> ServiceId {
        let service = LifecycleService::new(name);
        let id = service.id;
        self.inner.services.write().insert(id, service);
        id
    }

    /// Attempt a state transition for the given service.
    pub fn transition(
        &self,
        id: ServiceId,
        target: ServiceState,
        reason: impl Into<String>,
    ) -> Result<(), LifecycleError> {
        let reason = reason.into();
        let mut services = self.inner.services.write();
        let service = services
            .get_mut(&id)
            .ok_or_else(|| LifecycleError::new(LifecycleErrorKind::ServiceNotFound, "service not found"))?;

        let from = service.state;
        if !from.can_transition_to(target) {
            return Err(LifecycleError::new(
                LifecycleErrorKind::InvalidTransition,
                format!(
                    "cannot transition service '{}' from {} to {}",
                    service.name, from, target
                ),
            ));
        }

        let now = Utc::now();
        service.state = target;
        service.updated_at = now;

        if target == ServiceState::Failed {
            service.error_message = Some(reason.clone());
        }

        let transition = StateTransition {
            service_id: id,
            from,
            to: target,
            timestamp: now,
            reason,
        };

        drop(services);
        self.inner.history.write().push(transition);

        Ok(())
    }

    /// Get the current state of a service.
    pub fn state(&self, id: ServiceId) -> Result<ServiceState, LifecycleError> {
        self.inner
            .services
            .read()
            .get(&id)
            .map(|s| s.state)
            .ok_or_else(|| LifecycleError::new(LifecycleErrorKind::ServiceNotFound, "service not found"))
    }

    /// Get a snapshot of a service.
    pub fn service(&self, id: ServiceId) -> Option<LifecycleService> {
        self.inner.services.read().get(&id).cloned()
    }

    /// List all service identifiers and their current states.
    pub fn list_services(&self) -> Vec<(ServiceId, String, ServiceState)> {
        self.inner
            .services
            .read()
            .iter()
            .map(|(id, s)| (*id, s.name.clone(), s.state))
            .collect()
    }

    /// Get the transition history for a service.
    pub fn history(&self, id: ServiceId) -> Vec<StateTransition> {
        self.inner
            .history
            .read()
            .iter()
            .filter(|t| t.service_id == id)
            .cloned()
            .collect()
    }

    /// Get the full transition history.
    pub fn full_history(&self) -> Vec<StateTransition> {
        self.inner.history.read().clone()
    }

    /// Remove a service that is in the `Destroyed` state.
    pub fn remove_service(&self, id: ServiceId) -> Result<(), LifecycleError> {
        let mut services = self.inner.services.write();
        let service = services
            .get(&id)
            .ok_or_else(|| LifecycleError::new(LifecycleErrorKind::ServiceNotFound, "service not found"))?;

        if service.state != ServiceState::Destroyed {
            return Err(LifecycleError::new(
                LifecycleErrorKind::InvalidTransition,
                format!(
                    "cannot remove service '{}' in state {} (must be destroyed)",
                    service.name, service.state
                ),
            ));
        }

        services.remove(&id);
        Ok(())
    }

    /// Check whether a service exists.
    pub fn has_service(&self, id: ServiceId) -> bool {
        self.inner.services.read().contains_key(&id)
    }

    /// Get the count of services in each state.
    pub fn state_summary(&self) -> HashMap<ServiceState, usize> {
        let mut summary = HashMap::new();
        for service in self.inner.services.read().values() {
            *summary.entry(service.state).or_insert(0) += 1;
        }
        summary
    }
}

impl Default for LifecycleManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_lookup() {
        let mgr = LifecycleManager::new();
        let id = mgr.register_service("test-service");
        let state = mgr.state(id).unwrap();
        assert_eq!(state, ServiceState::Created);
        assert!(mgr.has_service(id));
    }

    #[test]
    fn valid_transition_chain() {
        let mgr = LifecycleManager::new();
        let id = mgr.register_service("svc");

        mgr.transition(id, ServiceState::Registered, "registered")
            .unwrap();
        mgr.transition(id, ServiceState::Initialized, "initialized")
            .unwrap();
        mgr.transition(id, ServiceState::Starting, "starting")
            .unwrap();
        mgr.transition(id, ServiceState::Running, "running")
            .unwrap();

        assert_eq!(mgr.state(id).unwrap(), ServiceState::Running);
    }

    #[test]
    fn invalid_transition_rejected() {
        let mgr = LifecycleManager::new();
        let id = mgr.register_service("svc");

        let result = mgr.transition(id, ServiceState::Running, "skip");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind, LifecycleErrorKind::InvalidTransition);
    }

    #[test]
    fn pause_resume() {
        let mgr = LifecycleManager::new();
        let id = mgr.register_service("svc");
        mgr.transition(id, ServiceState::Registered, "").unwrap();
        mgr.transition(id, ServiceState::Initialized, "").unwrap();
        mgr.transition(id, ServiceState::Starting, "").unwrap();
        mgr.transition(id, ServiceState::Running, "").unwrap();

        mgr.transition(id, ServiceState::Paused, "pause").unwrap();
        assert_eq!(mgr.state(id).unwrap(), ServiceState::Paused);

        mgr.transition(id, ServiceState::Running, "resume").unwrap();
        assert_eq!(mgr.state(id).unwrap(), ServiceState::Running);
    }

    #[test]
    fn failure_from_any_running_state() {
        let mgr = LifecycleManager::new();
        let id = mgr.register_service("svc");
        mgr.transition(id, ServiceState::Registered, "").unwrap();
        mgr.transition(id, ServiceState::Initialized, "").unwrap();
        mgr.transition(id, ServiceState::Starting, "").unwrap();
        mgr.transition(id, ServiceState::Running, "").unwrap();

        mgr.transition(id, ServiceState::Failed, "crash").unwrap();
        assert_eq!(mgr.state(id).unwrap(), ServiceState::Failed);
    }

    #[test]
    fn restart_from_failure() {
        let mgr = LifecycleManager::new();
        let id = mgr.register_service("svc");
        mgr.transition(id, ServiceState::Registered, "").unwrap();
        mgr.transition(id, ServiceState::Initialized, "").unwrap();
        mgr.transition(id, ServiceState::Starting, "").unwrap();
        mgr.transition(id, ServiceState::Running, "").unwrap();
        mgr.transition(id, ServiceState::Failed, "crash").unwrap();

        mgr.transition(id, ServiceState::Restarting, "retry").unwrap();
        mgr.transition(id, ServiceState::Initialized, "reinit").unwrap();
        assert_eq!(mgr.state(id).unwrap(), ServiceState::Initialized);
    }

    #[test]
    fn destroy_after_stop() {
        let mgr = LifecycleManager::new();
        let id = mgr.register_service("svc");
        mgr.transition(id, ServiceState::Registered, "").unwrap();
        mgr.transition(id, ServiceState::Initialized, "").unwrap();
        mgr.transition(id, ServiceState::Starting, "").unwrap();
        mgr.transition(id, ServiceState::Running, "").unwrap();
        mgr.transition(id, ServiceState::Stopping, "shutting down")
            .unwrap();
        mgr.transition(id, ServiceState::Stopped, "stopped").unwrap();
        mgr.transition(id, ServiceState::Destroyed, "done").unwrap();

        assert_eq!(mgr.state(id).unwrap(), ServiceState::Destroyed);
    }

    #[test]
    fn remove_destroyed_service() {
        let mgr = LifecycleManager::new();
        let id = mgr.register_service("svc");
        mgr.transition(id, ServiceState::Registered, "").unwrap();
        mgr.transition(id, ServiceState::Initialized, "").unwrap();
        mgr.transition(id, ServiceState::Starting, "").unwrap();
        mgr.transition(id, ServiceState::Running, "").unwrap();
        mgr.transition(id, ServiceState::Stopping, "").unwrap();
        mgr.transition(id, ServiceState::Stopped, "").unwrap();
        mgr.transition(id, ServiceState::Destroyed, "").unwrap();

        mgr.remove_service(id).unwrap();
        assert!(!mgr.has_service(id));
    }

    #[test]
    fn cannot_remove_non_destroyed() {
        let mgr = LifecycleManager::new();
        let id = mgr.register_service("svc");
        let result = mgr.remove_service(id);
        assert!(result.is_err());
    }

    #[test]
    fn history_tracking() {
        let mgr = LifecycleManager::new();
        let id = mgr.register_service("svc");
        mgr.transition(id, ServiceState::Registered, "reg").unwrap();
        mgr.transition(id, ServiceState::Initialized, "init").unwrap();

        let history = mgr.history(id);
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].from, ServiceState::Created);
        assert_eq!(history[0].to, ServiceState::Registered);
        assert_eq!(history[1].from, ServiceState::Registered);
        assert_eq!(history[1].to, ServiceState::Initialized);
    }

    #[test]
    fn state_summary() {
        let mgr = LifecycleManager::new();
        let id1 = mgr.register_service("svc1");
        let id2 = mgr.register_service("svc2");
        mgr.transition(id1, ServiceState::Registered, "").unwrap();

        let summary = mgr.state_summary();
        assert_eq!(summary[&ServiceState::Created], 1);
        assert_eq!(summary[&ServiceState::Registered], 1);
    }

    #[test]
    fn service_not_found() {
        let mgr = LifecycleManager::new();
        let fake = ServiceId::new();
        let result = mgr.state(fake);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind, LifecycleErrorKind::ServiceNotFound);
    }

    #[test]
    fn list_services() {
        let mgr = LifecycleManager::new();
        mgr.register_service("a");
        mgr.register_service("b");
        let list = mgr.list_services();
        assert_eq!(list.len(), 2);
    }
}
