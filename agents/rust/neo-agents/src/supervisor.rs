use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::agent::Agent;
use crate::types::{AgentHealth, AgentId, AgentMetrics, AgentStatus};

// ---------------------------------------------------------------------------
// HealthCheck
// ---------------------------------------------------------------------------

/// Result of a health check on a single agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    /// The agent that was checked.
    pub agent_id: AgentId,
    /// The reported health.
    pub health: AgentHealth,
    /// When the check was performed.
    pub checked_at: DateTime<Utc>,
    /// Any issues detected.
    pub issues: Vec<String>,
    /// Latency of the check in milliseconds.
    pub latency_ms: f64,
}

// ---------------------------------------------------------------------------
// HealthManager
// ---------------------------------------------------------------------------

/// Monitors the health of all agents in the system.
pub struct HealthManager {
    /// Health check history per agent.
    health_history: DashMap<AgentId, Vec<HealthCheck>>,
    /// Current health status per agent.
    current_health: DashMap<AgentId, AgentHealth>,
    /// Maximum history entries per agent.
    max_history: usize,
    /// Health check interval in seconds.
    check_interval_secs: u64,
}

impl HealthManager {
    /// Create a new health manager.
    #[must_use]
    pub fn new(check_interval_secs: u64, max_history: usize) -> Self {
        Self {
            health_history: DashMap::new(),
            current_health: DashMap::new(),
            max_history,
            check_interval_secs,
        }
    }

    /// Perform a health check on an agent.
    pub fn check_health(&self, agent: &Agent) -> HealthCheck {
        let start = Utc::now();
        let mut issues = Vec::new();

        // Determine health based on agent state
        let health = if agent.status().is_terminal() || agent.status() == AgentStatus::Failed {
            AgentHealth::Unhealthy
        } else if agent.heartbeat_expired() {
            issues.push("heartbeat expired".to_string());
            AgentHealth::Degraded
        } else if agent.metrics().error_count > 10 {
            issues.push(format!("high error count: {}", agent.metrics().error_count));
            AgentHealth::Degraded
        } else if agent.recovery_attempts() > 0 {
            issues.push(format!("has recovered {} times", agent.recovery_attempts()));
            AgentHealth::Degraded
        } else {
            AgentHealth::Healthy
        };

        let latency = Utc::now().signed_duration_since(start).num_milliseconds() as f64;

        let check = HealthCheck {
            agent_id: agent.id(),
            health,
            checked_at: Utc::now(),
            issues,
            latency_ms: latency,
        };

        // Update current health
        self.current_health.insert(agent.id(), health);

        // Add to history
        let mut history = self.health_history.entry(agent.id()).or_default();
        history.push(check.clone());
        if history.len() > self.max_history {
            history.remove(0);
        }

        check
    }

    /// Get the current health of an agent.
    #[must_use]
    pub fn get_health(&self, agent_id: &AgentId) -> AgentHealth {
        self.current_health
            .get(agent_id)
            .map(|h| *h)
            .unwrap_or(AgentHealth::Unknown)
    }

    /// Get health history for an agent.
    pub fn get_history(&self, agent_id: &AgentId) -> Vec<HealthCheck> {
        self.health_history
            .get(agent_id)
            .map(|h| h.clone())
            .unwrap_or_default()
    }

    /// Get all unhealthy agents.
    #[must_use]
    pub fn unhealthy_agents(&self) -> Vec<AgentId> {
        self.current_health
            .iter()
            .filter(|entry| *entry.value() == AgentHealth::Unhealthy)
            .map(|entry| *entry.key())
            .collect()
    }

    /// Get all degraded agents.
    #[must_use]
    pub fn degraded_agents(&self) -> Vec<AgentId> {
        self.current_health
            .iter()
            .filter(|entry| *entry.value() == AgentHealth::Degraded)
            .map(|entry| *entry.key())
            .collect()
    }

    /// Get the health check interval.
    #[must_use]
    pub fn check_interval_secs(&self) -> u64 {
        self.check_interval_secs
    }

    /// Remove health data for an agent.
    pub fn remove_agent(&self, agent_id: &AgentId) {
        self.health_history.remove(agent_id);
        self.current_health.remove(agent_id);
    }
}

impl Default for HealthManager {
    fn default() -> Self {
        Self::new(10, 100)
    }
}

// ---------------------------------------------------------------------------
// FailureDetector
// ---------------------------------------------------------------------------

/// Detects agent failures based on various signals.
pub struct FailureDetector {
    /// Heartbeat timeout multiplier. Agent is considered dead if no heartbeat
    /// for `heartbeat_interval * multiplier` seconds.
    heartbeat_timeout_multiplier: u64,
    /// Maximum consecutive failures before marking as dead.
    max_consecutive_failures: u32,
    /// Consecutive failure counts per agent.
    consecutive_failures: DashMap<AgentId, u32>,
    /// Whether detection is enabled.
    enabled: Arc<AtomicBool>,
}

impl FailureDetector {
    /// Create a new failure detector.
    #[must_use]
    pub fn new(heartbeat_timeout_multiplier: u64, max_consecutive_failures: u32) -> Self {
        Self {
            heartbeat_timeout_multiplier,
            max_consecutive_failures,
            consecutive_failures: DashMap::new(),
            enabled: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Record a successful heartbeat for an agent.
    pub fn record_heartbeat(&self, agent_id: &AgentId) {
        self.consecutive_failures.insert(*agent_id, 0);
    }

    /// Record a failure for an agent.
    pub fn record_failure(&self, agent_id: &AgentId) -> bool {
        let count = self
            .consecutive_failures
            .entry(*agent_id)
            .and_modify(|c| *c += 1)
            .or_insert(1);

        *count >= self.max_consecutive_failures
    }

    /// Check if an agent is considered dead based on heartbeat.
    #[must_use]
    pub fn is_agent_dead(&self, agent: &Agent, heartbeat_interval_secs: u64) -> bool {
        if !self.enabled.load(Ordering::SeqCst) {
            return false;
        }
        let timeout = heartbeat_interval_secs * self.heartbeat_timeout_multiplier;
        let heartbeat_ts = agent.last_heartbeat().timestamp();
        let now_ts = Utc::now().timestamp();
        agent.heartbeat_expired() || heartbeat_ts + (timeout as i64) < now_ts
    }

    /// Check if an agent has too many consecutive failures.
    #[must_use]
    pub fn is_failure_threshold_exceeded(&self, agent_id: &AgentId) -> bool {
        self.consecutive_failures
            .get(agent_id)
            .map(|c| *c >= self.max_consecutive_failures)
            .unwrap_or(false)
    }

    /// Enable or disable failure detection.
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::SeqCst);
    }

    /// Get the consecutive failure count for an agent.
    #[must_use]
    pub fn consecutive_failures(&self, agent_id: &AgentId) -> u32 {
        self.consecutive_failures
            .get(agent_id)
            .map(|c| *c)
            .unwrap_or(0)
    }
}

impl Default for FailureDetector {
    fn default() -> Self {
        Self::new(3, 5)
    }
}

// ---------------------------------------------------------------------------
// RecoveryManager
// ---------------------------------------------------------------------------

/// Manages agent recovery after failures.
pub struct RecoveryManager {
    /// Recovery strategies per agent.
    strategies: DashMap<AgentId, RecoveryStrategy>,
    /// Recovery history.
    history: DashMap<AgentId, Vec<RecoveryEvent>>,
    /// Maximum recovery history per agent.
    max_history: usize,
}

/// Recovery strategy for an agent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RecoveryStrategy {
    /// Restart the agent with the same configuration.
    Restart,
    /// Restart with a fresh state.
    FreshRestart,
    /// Migrate to a different agent.
    Migrate,
    /// Skip the failed task and continue.
    SkipAndContinue,
    /// Escalate to a supervisor agent.
    Escalate,
    /// Custom recovery strategy.
    Custom(String),
}

/// Record of a recovery event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryEvent {
    /// The agent that was recovered.
    pub agent_id: AgentId,
    /// The strategy used.
    pub strategy: RecoveryStrategy,
    /// When the recovery was attempted.
    pub attempted_at: DateTime<Utc>,
    /// Whether the recovery succeeded.
    pub success: bool,
    /// Error message if recovery failed.
    pub error: Option<String>,
}

impl RecoveryManager {
    /// Create a new recovery manager.
    #[must_use]
    pub fn new(max_history: usize) -> Self {
        Self {
            strategies: DashMap::new(),
            history: DashMap::new(),
            max_history,
        }
    }

    /// Set the recovery strategy for an agent.
    pub fn set_strategy(&self, agent_id: AgentId, strategy: RecoveryStrategy) {
        self.strategies.insert(agent_id, strategy);
    }

    /// Get the recovery strategy for an agent.
    #[must_use]
    pub fn get_strategy(&self, agent_id: &AgentId) -> RecoveryStrategy {
        self.strategies
            .get(agent_id)
            .map(|s| s.clone())
            .unwrap_or(RecoveryStrategy::Restart)
    }

    /// Record a recovery attempt.
    pub fn record_recovery(&self, event: RecoveryEvent) {
        let mut history = self.history.entry(event.agent_id).or_default();
        history.push(event);
        if history.len() > self.max_history {
            history.remove(0);
        }
    }

    /// Get recovery history for an agent.
    pub fn get_history(&self, agent_id: &AgentId) -> Vec<RecoveryEvent> {
        self.history
            .get(agent_id)
            .map(|h| h.clone())
            .unwrap_or_default()
    }

    /// Remove recovery data for an agent.
    pub fn remove_agent(&self, agent_id: &AgentId) {
        self.strategies.remove(agent_id);
        self.history.remove(agent_id);
    }
}

impl Default for RecoveryManager {
    fn default() -> Self {
        Self::new(50)
    }
}

// ---------------------------------------------------------------------------
// LoadBalancer
// ---------------------------------------------------------------------------

/// Distributes tasks across agents based on load.
pub struct LoadBalancer {
    /// Agent loads: agent_id -> load factor (0.0 - 1.0).
    agent_loads: DashMap<AgentId, f64>,
    /// Maximum load factor before an agent is considered saturated.
    saturation_threshold: f64,
}

impl LoadBalancer {
    /// Create a new load balancer.
    #[must_use]
    pub fn new(saturation_threshold: f64) -> Self {
        Self {
            agent_loads: DashMap::new(),
            saturation_threshold,
        }
    }

    /// Update the load factor for an agent.
    pub fn update_load(&self, agent_id: AgentId, load: f64) {
        self.agent_loads.insert(agent_id, load.clamp(0.0, 1.0));
    }

    /// Get the load factor for an agent.
    #[must_use]
    pub fn get_load(&self, agent_id: &AgentId) -> f64 {
        self.agent_loads.get(agent_id).map(|l| *l).unwrap_or(0.0)
    }

    /// Select the least-loaded agent from a list of candidates.
    #[must_use]
    pub fn select_least_loaded(&self, candidates: &[AgentId]) -> Option<AgentId> {
        candidates
            .iter()
            .filter(|id| {
                let load = self.get_load(id);
                load < self.saturation_threshold
            })
            .min_by(|a, b| {
                let load_a = self.get_load(a);
                let load_b = self.get_load(b);
                load_a
                    .partial_cmp(&load_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .copied()
    }

    /// Get all agents below the saturation threshold.
    #[must_use]
    pub fn available_agents(&self) -> Vec<AgentId> {
        self.agent_loads
            .iter()
            .filter(|entry| *entry.value() < self.saturation_threshold)
            .map(|entry| *entry.key())
            .collect()
    }

    /// Calculate load based on metrics.
    #[must_use]
    pub fn calculate_load(metrics: &AgentMetrics, max_concurrent: usize) -> f64 {
        if max_concurrent == 0 {
            return 1.0;
        }
        let task_load = metrics.tasks_active as f64 / max_concurrent as f64;
        let error_load = if metrics.error_count > 0 {
            (metrics.error_count as f64
                / (metrics.tasks_completed + metrics.error_count).max(1) as f64)
                * 0.3
        } else {
            0.0
        };
        (task_load + error_load).clamp(0.0, 1.0)
    }
}

impl Default for LoadBalancer {
    fn default() -> Self {
        Self::new(0.9)
    }
}

// ---------------------------------------------------------------------------
// SupervisorAgent
// ---------------------------------------------------------------------------

/// A supervisor agent that monitors and manages other agents.
///
/// The supervisor is responsible for:
/// - Monitoring agent health
/// - Detecting and recovering from failures
/// - Rebalancing work across agents
/// - Detecting deadlocks, starvation, and livelocks
/// - Escalating critical issues
pub struct SupervisorAgent {
    /// The supervisor's own agent ID.
    pub id: AgentId,
    /// The health manager.
    pub health_manager: Arc<HealthManager>,
    /// The failure detector.
    pub failure_detector: Arc<FailureDetector>,
    /// The recovery manager.
    pub recovery_manager: Arc<RecoveryManager>,
    /// The load balancer.
    pub load_balancer: Arc<LoadBalancer>,
    /// Agents being supervised.
    supervised_agents: DashMap<AgentId, SupervisedAgentInfo>,
    /// Alert history.
    alerts: DashMap<uuid::Uuid, SupervisorAlert>,
    /// Whether the supervisor is running.
    is_running: Arc<AtomicBool>,
}

/// Information about a supervised agent.
#[derive(Debug, Clone)]
pub struct SupervisedAgentInfo {
    /// The agent ID.
    pub agent_id: AgentId,
    /// When supervision started.
    pub supervised_since: DateTime<Utc>,
    /// Last known status.
    pub last_status: AgentStatus,
    /// Number of failures observed.
    pub failure_count: u32,
    /// Number of recoveries performed.
    pub recovery_count: u32,
}

/// An alert raised by the supervisor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorAlert {
    /// Unique alert identifier.
    pub id: uuid::Uuid,
    /// The agent that triggered the alert.
    pub agent_id: AgentId,
    /// Alert severity.
    pub severity: AlertSeverity,
    /// Alert message.
    pub message: String,
    /// When the alert was raised.
    pub raised_at: DateTime<Utc>,
    /// Whether the alert has been acknowledged.
    pub acknowledged: bool,
}

/// Severity of a supervisor alert.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AlertSeverity {
    /// Informational.
    Info,
    /// Warning.
    Warning,
    /// Critical, immediate attention needed.
    Critical,
}

impl std::fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Info => write!(f, "info"),
            Self::Warning => write!(f, "warning"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

impl SupervisorAgent {
    /// Create a new supervisor agent.
    #[must_use]
    pub fn new() -> Self {
        Self {
            id: AgentId::new(),
            health_manager: Arc::new(HealthManager::default()),
            failure_detector: Arc::new(FailureDetector::default()),
            recovery_manager: Arc::new(RecoveryManager::default()),
            load_balancer: Arc::new(LoadBalancer::default()),
            supervised_agents: DashMap::new(),
            alerts: DashMap::new(),
            is_running: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Start supervising an agent.
    pub fn supervise(&self, agent_id: AgentId) {
        self.supervised_agents.insert(
            agent_id,
            SupervisedAgentInfo {
                agent_id,
                supervised_since: Utc::now(),
                last_status: AgentStatus::Created,
                failure_count: 0,
                recovery_count: 0,
            },
        );
        tracing::info!("Supervisor {} now supervising agent {}", self.id, agent_id);
    }

    /// Stop supervising an agent.
    pub fn unsupervise(&self, agent_id: &AgentId) {
        self.supervised_agents.remove(agent_id);
        self.health_manager.remove_agent(agent_id);
        self.recovery_manager.remove_agent(agent_id);
    }

    /// Check health of a supervised agent.
    pub fn check_agent_health(&self, agent: &Agent) -> HealthCheck {
        let check = self.health_manager.check_health(agent);

        if check.health == AgentHealth::Unhealthy {
            self.raise_alert(
                agent.id(),
                AlertSeverity::Critical,
                format!("agent is unhealthy: {}", check.issues.join(", ")),
            );
        } else if check.health == AgentHealth::Degraded {
            self.raise_alert(
                agent.id(),
                AlertSeverity::Warning,
                format!("agent is degraded: {}", check.issues.join(", ")),
            );
        }

        // Update supervised info
        if let Some(mut info) = self.supervised_agents.get_mut(&agent.id()) {
            info.last_status = agent.status();
        }

        // Record heartbeat in failure detector
        self.failure_detector.record_heartbeat(&agent.id());

        check
    }

    /// Handle a detected failure.
    pub fn handle_failure(&self, agent_id: AgentId, error: String) -> RecoveryStrategy {
        // Record failure
        let exceeded = self.failure_detector.record_failure(&agent_id);

        if let Some(mut info) = self.supervised_agents.get_mut(&agent_id) {
            info.failure_count += 1;
        }

        let strategy = self.recovery_manager.get_strategy(&agent_id);

        if exceeded {
            self.raise_alert(
                agent_id,
                AlertSeverity::Critical,
                format!("failure threshold exceeded: {error}"),
            );
        }

        strategy
    }

    /// Record a successful recovery.
    pub fn record_recovery(&self, agent_id: AgentId, strategy: RecoveryStrategy, success: bool) {
        let event = RecoveryEvent {
            agent_id,
            strategy,
            attempted_at: Utc::now(),
            success,
            error: None,
        };
        self.recovery_manager.record_recovery(event);

        if let Some(mut info) = self.supervised_agents.get_mut(&agent_id) {
            info.recovery_count += 1;
        }
    }

    /// Update load balancing for an agent.
    pub fn update_agent_load(
        &self,
        agent_id: AgentId,
        metrics: &AgentMetrics,
        max_concurrent: usize,
    ) {
        let load = LoadBalancer::calculate_load(metrics, max_concurrent);
        self.load_balancer.update_load(agent_id, load);
    }

    /// Select the best agent for a new task.
    #[must_use]
    pub fn select_agent(&self, candidates: &[AgentId]) -> Option<AgentId> {
        self.load_balancer.select_least_loaded(candidates)
    }

    /// Raise an alert.
    pub fn raise_alert(&self, agent_id: AgentId, severity: AlertSeverity, message: String) {
        let alert = SupervisorAlert {
            id: uuid::Uuid::new_v4(),
            agent_id,
            severity,
            message,
            raised_at: Utc::now(),
            acknowledged: false,
        };
        tracing::warn!(
            "Supervisor alert [{}]: agent {} - {}",
            alert.severity,
            agent_id,
            alert.message
        );
        self.alerts.insert(alert.id, alert);
    }

    /// Acknowledge an alert.
    pub fn acknowledge_alert(&self, alert_id: &uuid::Uuid) -> bool {
        if let Some(mut alert) = self.alerts.get_mut(alert_id) {
            alert.acknowledged = true;
            true
        } else {
            false
        }
    }

    /// Get all unacknowledged alerts.
    #[must_use]
    pub fn unacknowledged_alerts(&self) -> Vec<SupervisorAlert> {
        self.alerts
            .iter()
            .filter(|entry| !entry.value().acknowledged)
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Get the count of supervised agents.
    #[must_use]
    pub fn supervised_count(&self) -> usize {
        self.supervised_agents.len()
    }

    /// Check if the supervisor is running.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    /// Stop the supervisor.
    pub fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
    }

    /// Get supervised agent info.
    pub fn get_supervised_info(&self, agent_id: &AgentId) -> Option<SupervisedAgentInfo> {
        self.supervised_agents.get(agent_id).map(|i| i.clone())
    }

    /// List all supervised agent IDs.
    #[must_use]
    pub fn list_supervised(&self) -> Vec<AgentId> {
        self.supervised_agents
            .iter()
            .map(|entry| *entry.key())
            .collect()
    }
}

impl Default for SupervisorAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::Agent;
    use crate::types::AgentConfiguration;

    fn test_agent() -> Agent {
        Agent::new(AgentConfiguration::new("test"))
    }

    #[test]
    fn test_health_manager() {
        let hm = HealthManager::new(10, 100);
        let mut agent = test_agent();
        agent.initialize().unwrap();

        let check = hm.check_health(&agent);
        assert_eq!(check.health, AgentHealth::Healthy);
        assert!(check.issues.is_empty());

        assert_eq!(hm.get_health(&agent.id()), AgentHealth::Healthy);
    }

    #[test]
    fn test_failure_detector() {
        let fd = FailureDetector::new(3, 3);
        let id = AgentId::new();

        // Record failures
        assert!(!fd.record_failure(&id));
        assert!(!fd.record_failure(&id));
        assert!(fd.record_failure(&id));

        assert!(fd.is_failure_threshold_exceeded(&id));

        // Reset on heartbeat
        fd.record_heartbeat(&id);
        assert!(!fd.is_failure_threshold_exceeded(&id));
    }

    #[test]
    fn test_recovery_manager() {
        let rm = RecoveryManager::new(10);
        let id = AgentId::new();

        assert_eq!(rm.get_strategy(&id), RecoveryStrategy::Restart);

        rm.set_strategy(id, RecoveryStrategy::FreshRestart);
        assert_eq!(rm.get_strategy(&id), RecoveryStrategy::FreshRestart);

        rm.record_recovery(RecoveryEvent {
            agent_id: id,
            strategy: RecoveryStrategy::FreshRestart,
            attempted_at: Utc::now(),
            success: true,
            error: None,
        });

        assert_eq!(rm.get_history(&id).len(), 1);
    }

    #[test]
    fn test_load_balancer() {
        let lb = LoadBalancer::new(0.9);
        let a1 = AgentId::new();
        let a2 = AgentId::new();

        lb.update_load(a1, 0.3);
        lb.update_load(a2, 0.8);

        let best = lb.select_least_loaded(&[a1, a2]).unwrap();
        assert_eq!(best, a1);

        // Calculate load
        let metrics = AgentMetrics {
            tasks_active: 3,
            tasks_completed: 10,
            error_count: 2,
            ..AgentMetrics::default()
        };
        let load = LoadBalancer::calculate_load(&metrics, 4);
        assert!(load > 0.0 && load <= 1.0);
    }

    #[test]
    fn test_supervisor() {
        let sup = SupervisorAgent::new();
        let id = AgentId::new();
        sup.supervise(id);

        assert_eq!(sup.supervised_count(), 1);
        assert!(sup.get_supervised_info(&id).is_some());

        // Raise and acknowledge alert
        sup.raise_alert(id, AlertSeverity::Warning, "test".to_string());
        let alerts = sup.unacknowledged_alerts();
        assert_eq!(alerts.len(), 1);

        sup.acknowledge_alert(&alerts[0].id);
        assert!(sup.unacknowledged_alerts().is_empty());
    }
}
