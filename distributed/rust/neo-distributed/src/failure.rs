//! Failure detection and recovery — crash detection, network partition
//! handling, automatic recovery, workload migration, retry, and rollback.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::error::{NeoResult, RecoveryAction};
use crate::node::NodeManager;
use crate::types::{NodeHealth, NodeId, NodeState};

// ---------------------------------------------------------------------------
// FailureType
// ---------------------------------------------------------------------------

/// Classification of detected failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FailureType {
    /// Node crash (no heartbeat).
    Crash,
    /// Network partition (partial connectivity).
    NetworkPartition,
    /// Transient failure (temporary unavailability).
    Transient,
    /// Permanent failure (persistent hardware/software issue).
    Permanent,
    /// Performance degradation (slow responses).
    Degradation,
}

impl std::fmt::Display for FailureType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Crash => write!(f, "crash"),
            Self::NetworkPartition => write!(f, "network_partition"),
            Self::Transient => write!(f, "transient"),
            Self::Permanent => write!(f, "permanent"),
            Self::Degradation => write!(f, "degradation"),
        }
    }
}

// ---------------------------------------------------------------------------
// FailureRecord
// ---------------------------------------------------------------------------

/// A recorded failure event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureRecord {
    /// Unique failure ID.
    pub id: uuid::Uuid,
    /// Node that failed.
    pub node_id: NodeId,
    /// Type of failure.
    pub failure_type: FailureType,
    /// Error description.
    pub description: String,
    /// When the failure was detected.
    pub detected_at: DateTime<Utc>,
    /// Whether recovery was attempted.
    pub recovery_attempted: bool,
    /// Recovery action taken.
    pub recovery_action: Option<RecoveryAction>,
    /// Whether recovery succeeded.
    pub recovery_succeeded: Option<bool>,
}

// ---------------------------------------------------------------------------
// RecoveryStrategy
// ---------------------------------------------------------------------------

/// Strategy for recovering from failures.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryStrategy {
    /// Maximum number of retries.
    pub max_retries: u32,
    /// Base delay between retries.
    pub retry_base_delay: Duration,
    /// Maximum delay between retries.
    pub retry_max_delay: Duration,
    /// Whether to migrate workloads on failure.
    pub migrate_workloads: bool,
    /// Whether to attempt rollback on failure.
    pub attempt_rollback: bool,
    /// Timeout for recovery operations.
    pub recovery_timeout: Duration,
}

impl Default for RecoveryStrategy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_base_delay: Duration::from_millis(100),
            retry_max_delay: Duration::from_secs(10),
            migrate_workloads: true,
            attempt_rollback: false,
            recovery_timeout: Duration::from_secs(60),
        }
    }
}

// ---------------------------------------------------------------------------
// FailureHistory
// ---------------------------------------------------------------------------

/// Tracks failure history for analysis and pattern detection.
pub struct FailureHistory {
    /// Recent failure records (bounded queue).
    records: RwLock<VecDeque<FailureRecord>>,
    /// Maximum history size.
    max_size: usize,
    /// Failure counts by node.
    node_failures: RwLock<HashMap<NodeId, u32>>,
    /// Failure counts by type.
    type_failures: RwLock<HashMap<FailureType, u32>>,
}

impl FailureHistory {
    pub fn new(max_size: usize) -> Self {
        Self {
            records: RwLock::new(VecDeque::with_capacity(max_size)),
            max_size,
            node_failures: RwLock::new(HashMap::new()),
            type_failures: RwLock::new(HashMap::new()),
        }
    }

    /// Record a failure.
    pub fn record(&self, failure: FailureRecord) {
        let mut records = self.records.write();
        if records.len() >= self.max_size {
            records.pop_front();
        }
        records.push_back(failure.clone());

        *self
            .node_failures
            .write()
            .entry(failure.node_id)
            .or_insert(0) += 1;
        *self
            .type_failures
            .write()
            .entry(failure.failure_type)
            .or_insert(0) += 1;
    }

    /// Get recent failures.
    pub fn recent(&self, count: usize) -> Vec<FailureRecord> {
        self.records.read().iter().rev().take(count).cloned().collect()
    }

    /// Get failure count for a node.
    pub fn node_failure_count(&self, node_id: NodeId) -> u32 {
        self.node_failures.read().get(&node_id).copied().unwrap_or(0)
    }

    /// Get failure count by type.
    pub fn type_failure_count(&self, failure_type: FailureType) -> u32 {
        self.type_failures
            .read()
            .get(&failure_type)
            .copied()
            .unwrap_or(0)
    }

    /// Total failures recorded.
    pub fn total(&self) -> usize {
        self.records.read().len()
    }

    /// Check if a node has excessive failures (potential permanent failure).
    pub fn is_permanently_failed(&self, node_id: NodeId, threshold: u32) -> bool {
        self.node_failure_count(node_id) >= threshold
    }

    /// Detect failure patterns (e.g., repeated failures in short time).
    pub fn detect_patterns(&self) -> Vec<FailurePattern> {
        let records = self.records.read();
        let mut patterns = Vec::new();

        // Check for repeated failures on the same node within 5 minutes.
        let now = Utc::now();
        let window = chrono::Duration::minutes(5);
        let mut node_counts: HashMap<NodeId, Vec<FailureRecord>> = HashMap::new();

        for record in records.iter() {
            if now.signed_duration_since(record.detected_at) <= window {
                node_counts
                    .entry(record.node_id)
                    .or_default()
                    .push(record.clone());
            }
        }

        for (node_id, failures) in node_counts {
            if failures.len() >= 3 {
                patterns.push(FailurePattern {
                    node_id,
                    pattern_type: FailurePatternType::RepeatedFailure,
                    count: failures.len() as u32,
                    window: window,
                });
            }
        }

        patterns
    }
}

impl Default for FailureHistory {
    fn default() -> Self {
        Self::new(1000)
    }
}

// ---------------------------------------------------------------------------
// FailurePattern
// ---------------------------------------------------------------------------

/// Detected failure pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailurePattern {
    pub node_id: NodeId,
    pub pattern_type: FailurePatternType,
    pub count: u32,
    pub window: chrono::Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FailurePatternType {
    RepeatedFailure,
    CascadingFailure,
    NetworkPartition,
}

// ---------------------------------------------------------------------------
// RecoveryCoordinator
// ---------------------------------------------------------------------------

/// Coordinates recovery actions across the cluster.
pub struct RecoveryCoordinator {
    strategy: RwLock<RecoveryStrategy>,
    history: Arc<FailureHistory>,
    /// Active recovery attempts.
    active_recoveries: RwLock<HashMap<NodeId, RecoveryAttempt>>,
}

impl RecoveryCoordinator {
    pub fn new(strategy: RecoveryStrategy) -> Self {
        Self {
            strategy: RwLock::new(strategy),
            history: Arc::new(FailureHistory::new(1000)),
            active_recoveries: RwLock::new(HashMap::new()),
        }
    }

    /// Get the failure history.
    pub fn history(&self) -> &Arc<FailureHistory> {
        &self.history
    }

    /// Handle a detected failure.
    pub fn handle_failure(
        &self,
        node_id: NodeId,
        failure_type: FailureType,
        description: String,
    ) -> NeoResult<RecoveryAction> {
        let strategy = self.strategy.read();

        // Record the failure.
        let record = FailureRecord {
            id: uuid::Uuid::new_v4(),
            node_id,
            failure_type,
            description,
            detected_at: Utc::now(),
            recovery_attempted: false,
            recovery_action: None,
            recovery_succeeded: None,
        };
        self.history.record(record);

        // Determine recovery action based on failure type and history.
        let action = self.determine_recovery_action(node_id, failure_type, &strategy);

        tracing::warn!(
            node_id = %node_id,
            failure_type = %failure_type,
            action = ?action,
            "failure detected, recovery planned"
        );

        // Create recovery attempt.
        let attempt = RecoveryAttempt {
            node_id,
            action,
            started_at: Instant::now(),
            attempt_number: 0,
            max_attempts: strategy.max_retries,
        };
        self.active_recoveries.write().insert(node_id, attempt);

        Ok(action)
    }

    /// Determine the appropriate recovery action.
    fn determine_recovery_action(
        &self,
        node_id: NodeId,
        failure_type: FailureType,
        strategy: &RecoveryStrategy,
    ) -> RecoveryAction {
        // Check if node has failed too many times.
        if self.history.is_permanently_failed(node_id, strategy.max_retries + 1) {
            return if strategy.migrate_workloads {
                RecoveryAction::Migrate
            } else {
                RecoveryAction::Abort
            };
        }

        match failure_type {
            FailureType::Crash => {
                if strategy.migrate_workloads {
                    RecoveryAction::Failover
                } else {
                    RecoveryAction::Retry
                }
            }
            FailureType::NetworkPartition => RecoveryAction::Retry,
            FailureType::Transient => RecoveryAction::Retry,
            FailureType::Permanent => {
                if strategy.migrate_workloads {
                    RecoveryAction::Migrate
                } else {
                    RecoveryAction::Abort
                }
            }
            FailureType::Degradation => RecoveryAction::Migrate,
        }
    }

    /// Mark a recovery attempt as completed.
    pub fn complete_recovery(&self, node_id: NodeId, succeeded: bool) {
        if let Some(mut attempt) = self.active_recoveries.write().remove(&node_id) {
            tracing::info!(
                node_id = %node_id,
                succeeded = succeeded,
                attempt_number = attempt.attempt_number,
                "recovery attempt completed"
            );
        }
    }

    /// Get active recoveries.
    pub fn active_recoveries(&self) -> Vec<NodeId> {
        self.active_recoveries.read().keys().copied().collect()
    }

    /// Check if a node is currently being recovered.
    pub fn is_recovering(&self, node_id: NodeId) -> bool {
        self.active_recoveries.read().contains_key(&node_id)
    }
}

// ---------------------------------------------------------------------------
// RecoveryAttempt
// ---------------------------------------------------------------------------

/// Tracks an in-progress recovery attempt.
#[derive(Debug, Clone)]
struct RecoveryAttempt {
    node_id: NodeId,
    action: RecoveryAction,
    started_at: Instant,
    attempt_number: u32,
    max_attempts: u32,
}

// ---------------------------------------------------------------------------
// FailureDetector
// ---------------------------------------------------------------------------

/// High-level failure detector that monitors node health and triggers recovery.
pub struct FailureDetector {
    /// Node manager reference.
    node_manager: Arc<NodeManager>,
    /// Recovery coordinator.
    recovery: Arc<RecoveryCoordinator>,
    /// Failure detection interval.
    check_interval: Duration,
    /// Consecutive failures before marking suspect.
    suspect_threshold: u32,
    /// Consecutive failures before marking failed.
    failure_threshold: u32,
    /// Per-node consecutive failure counts.
    consecutive_failures: RwLock<HashMap<NodeId, u32>>,
}

impl FailureDetector {
    pub fn new(
        node_manager: Arc<NodeManager>,
        recovery: Arc<RecoveryCoordinator>,
        check_interval: Duration,
    ) -> Self {
        Self {
            node_manager,
            recovery,
            check_interval,
            suspect_threshold: 3,
            failure_threshold: 5,
            consecutive_failures: RwLock::new(HashMap::new()),
        }
    }

    /// Get the recovery coordinator.
    pub fn recovery(&self) -> &Arc<RecoveryCoordinator> {
        &self.recovery
    }

    /// Check all nodes for failures.
    pub async fn check_nodes(&self) -> NeoResult<Vec<NodeId>> {
        let nodes = self.node_manager.nodes();
        let mut failed = Vec::new();

        for entry in &nodes {
            if !entry.is_reachable() {
                let count = {
                    let mut counts = self.consecutive_failures.write();
                    let c = counts.entry(entry.id).or_insert(0);
                    *c += 1;
                    *c
                };

                if count >= self.failure_threshold {
                    // Mark as failed.
                    self.node_manager.mark_failed(entry.id)?;
                    failed.push(entry.id);

                    self.recovery.handle_failure(
                        entry.id,
                        crate::failure::FailureType::Crash,
                        format!(
                            "node unreachable after {} consecutive missed heartbeats",
                            count
                        ),
                    )?;
                } else if count >= self.suspect_threshold {
                    // Mark as suspect.
                    self.node_manager.mark_suspect(entry.id)?;
                    tracing::warn!(
                        node_id = %entry.id,
                        consecutive_failures = count,
                        "node suspect"
                    );
                }
            } else {
                // Reset failure count.
                self.consecutive_failures.write().remove(&entry.id);
            }
        }

        Ok(failed)
    }

    /// Check interval.
    pub fn check_interval(&self) -> Duration {
        self.check_interval
    }

    /// Get failure statistics.
    pub fn stats(&self) -> FailureStats {
        let total_failures = self.recovery.history().total();
        let active_recoveries = self.recovery.active_recoveries().len();
        let suspect_count = self.consecutive_failures.read().len();

        FailureStats {
            total_failures,
            active_recoveries,
            suspect_nodes: suspect_count,
        }
    }
}

// ---------------------------------------------------------------------------
// FailureStats
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureStats {
    pub total_failures: usize,
    pub active_recoveries: usize,
    pub suspect_nodes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failure_history_record() {
        let history = FailureHistory::new(10);
        let record = FailureRecord {
            id: uuid::Uuid::new_v4(),
            node_id: NodeId::new(),
            failure_type: FailureType::Crash,
            description: "test".to_string(),
            detected_at: Utc::now(),
            recovery_attempted: false,
            recovery_action: None,
            recovery_succeeded: None,
        };
        history.record(record);
        assert_eq!(history.total(), 1);
    }

    #[test]
    fn failure_history_by_node() {
        let history = FailureHistory::new(10);
        let node_id = NodeId::new();
        for _ in 0..3 {
            let record = FailureRecord {
                id: uuid::Uuid::new_v4(),
                node_id,
                failure_type: FailureType::Crash,
                description: "test".to_string(),
                detected_at: Utc::now(),
                recovery_attempted: false,
                recovery_action: None,
                recovery_succeeded: None,
            };
            history.record(record);
        }
        assert_eq!(history.node_failure_count(node_id), 3);
    }

    #[test]
    fn recovery_coordinator() {
        let coord = RecoveryCoordinator::new(RecoveryStrategy::default());
        let action = coord
            .handle_failure(
                NodeId::new(),
                FailureType::Crash,
                "test crash".to_string(),
            )
            .unwrap();
        assert!(matches!(
            action,
            RecoveryAction::Failover | RecoveryAction::Retry
        ));
    }

    #[test]
    fn failure_detector_stats() {
        let node_manager = Arc::new(NodeManager::new());
        let recovery = Arc::new(RecoveryCoordinator::new(RecoveryStrategy::default()));
        let detector = FailureDetector::new(
            node_manager,
            recovery,
            Duration::from_secs(1),
        );
        let stats = detector.stats();
        assert_eq!(stats.total_failures, 0);
    }

    #[test]
    fn strategy_default() {
        let strategy = RecoveryStrategy::default();
        assert_eq!(strategy.max_retries, 3);
        assert!(strategy.migrate_workloads);
    }
}
