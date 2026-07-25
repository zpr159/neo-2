use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{ExecutiveError, ExecutiveResult};
use crate::task::{TaskId, TaskState};

/// Unique identifier for a checkpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CheckpointId(pub Uuid);

impl CheckpointId {
    /// Create a new checkpoint identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for CheckpointId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for CheckpointId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A snapshot of execution state at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: CheckpointId,
    pub task_id: TaskId,
    pub timestamp: DateTime<Utc>,
    pub state_snapshot: serde_json::Value,
    pub step_index: u32,
    pub description: String,
}

/// Strategy for handling failures.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FallbackStrategy {
    Retry,
    Skip,
    UseAlternative,
    DegradeGracefully,
    FailFast,
}

/// A fallback configuration for a specific failure type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackConfig {
    pub strategy: FallbackStrategy,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
    pub alternative_description: Option<String>,
}

impl Default for FallbackConfig {
    fn default() -> Self {
        Self {
            strategy: FallbackStrategy::Retry,
            max_retries: 3,
            retry_delay_ms: 1000,
            alternative_description: None,
        }
    }
}

/// Recovery attempt record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryAttempt {
    pub timestamp: DateTime<Utc>,
    pub task_id: TaskId,
    pub strategy: FallbackStrategy,
    pub success: bool,
    pub error: Option<String>,
    pub duration_ms: u64,
}

/// Checkpoint and recovery state for a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecoveryState {
    pub task_id: TaskId,
    pub checkpoints: Vec<Checkpoint>,
    pub recovery_attempts: Vec<RecoveryAttempt>,
    pub current_strategy: FallbackStrategy,
    pub total_retries: u32,
    pub degraded: bool,
}

/// Failure recovery manages retries, fallback strategies, checkpoint resume, and graceful degradation.
#[derive(Clone)]
pub struct FailureRecovery {
    inner: Arc<FailureRecoveryInner>,
}

struct FailureRecoveryInner {
    recovery_states: RwLock<HashMap<TaskId, TaskRecoveryState>>,
    fallback_configs: RwLock<HashMap<String, FallbackConfig>>,
    global_max_retries: RwLock<u32>,
    degradation_level: RwLock<DegradationLevel>,
    recovery_log: RwLock<Vec<RecoveryAttempt>>,
}

/// System degradation level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DegradationLevel {
    None,
    Minor,
    Moderate,
    Severe,
    Critical,
}

impl FailureRecovery {
    /// Create a new failure recovery manager.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(FailureRecoveryInner {
                recovery_states: RwLock::new(HashMap::new()),
                fallback_configs: RwLock::new(HashMap::new()),
                global_max_retries: RwLock::new(3),
                degradation_level: RwLock::new(DegradationLevel::None),
                recovery_log: RwLock::new(Vec::new()),
            }),
        }
    }

    /// Create a checkpoint for a task.
    pub fn create_checkpoint(
        &self,
        task_id: TaskId,
        state_snapshot: serde_json::Value,
        step_index: u32,
        description: String,
    ) -> Checkpoint {
        let checkpoint = Checkpoint {
            id: CheckpointId::new(),
            task_id,
            timestamp: Utc::now(),
            state_snapshot,
            step_index,
            description,
        };

        let mut states = self.inner.recovery_states.write();
        let state = states.entry(task_id).or_insert_with(|| TaskRecoveryState {
            task_id,
            checkpoints: Vec::new(),
            recovery_attempts: Vec::new(),
            current_strategy: FallbackStrategy::Retry,
            total_retries: 0,
            degraded: false,
        });

        state.checkpoints.push(checkpoint.clone());
        tracing::info!(task_id = %task_id, checkpoint_id = %checkpoint.id, "checkpoint created");
        checkpoint
    }

    /// Resume from the latest checkpoint.
    pub fn resume_from_checkpoint(
        &self,
        task_id: TaskId,
    ) -> ExecutiveResult<Option<Checkpoint>> {
        let states = self.inner.recovery_states.read();
        let state = states
            .get(&task_id)
            .ok_or_else(|| ExecutiveError::task_not_found(&task_id.as_str()))?;

        let latest = state.checkpoints.last().cloned();
        if let Some(ref cp) = latest {
            tracing::info!(
                task_id = %task_id,
                checkpoint_id = %cp.id,
                step = cp.step_index,
                "resuming from checkpoint"
            );
        }

        Ok(latest)
    }

    /// Get the latest checkpoint for a task.
    pub fn latest_checkpoint(&self, task_id: TaskId) -> Option<Checkpoint> {
        self.inner
            .recovery_states
            .read()
            .get(&task_id)
            .and_then(|s| s.checkpoints.last().cloned())
    }

    /// Get all checkpoints for a task.
    pub fn checkpoints(&self, task_id: TaskId) -> Vec<Checkpoint> {
        self.inner
            .recovery_states
            .read()
            .get(&task_id)
            .map_or_else(Vec::new, |s| s.checkpoints.clone())
    }

    /// Record a recovery attempt.
    pub fn record_recovery_attempt(
        &self,
        task_id: TaskId,
        strategy: FallbackStrategy,
        success: bool,
        error: Option<String>,
        duration_ms: u64,
    ) {
        let attempt = RecoveryAttempt {
            timestamp: Utc::now(),
            task_id,
            strategy: strategy.clone(),
            success,
            error,
            duration_ms,
        };

        self.inner.recovery_log.write().push(attempt.clone());

        let should_adjust = {
            let mut states = self.inner.recovery_states.write();
            let state = states.entry(task_id).or_insert_with(|| TaskRecoveryState {
                task_id,
                checkpoints: Vec::new(),
                recovery_attempts: Vec::new(),
                current_strategy: FallbackStrategy::Retry,
                total_retries: 0,
                degraded: false,
            });

            state.recovery_attempts.push(attempt);
            state.total_retries += 1;

            !success && state.total_retries >= *self.inner.global_max_retries.read()
        };

        if should_adjust {
            if let Some(state) = self.inner.recovery_states.write().get_mut(&task_id) {
                state.current_strategy = FallbackStrategy::DegradeGracefully;
                state.degraded = true;
            }
            self.adjust_degradation_level();
        }
    }

    /// Determine the fallback strategy for a failed task.
    pub fn determine_strategy(&self, task_id: TaskId, error: &str) -> FallbackStrategy {
        let states = self.inner.recovery_states.read();
        let state = states.get(&task_id);

        if let Some(state) = state {
            if state.total_retries >= *self.inner.global_max_retries.read() {
                let custom_config = self
                    .inner
                    .fallback_configs
                    .read()
                    .get(error)
                    .cloned();

                if let Some(config) = custom_config {
                    return config.strategy;
                }

                return match *self.inner.degradation_level.read() {
                    DegradationLevel::None | DegradationLevel::Minor => {
                        FallbackStrategy::DegradeGracefully
                    }
                    DegradationLevel::Moderate => FallbackStrategy::Skip,
                    DegradationLevel::Severe | DegradationLevel::Critical => {
                        FallbackStrategy::FailFast
                    }
                };
            }
        }

        FallbackStrategy::Retry
    }

    /// Register a fallback configuration for a specific error type.
    pub fn register_fallback(&self, error_type: String, config: FallbackConfig) {
        self.inner.fallback_configs.write().insert(error_type, config);
    }

    /// Set the global maximum retries.
    pub fn set_global_max_retries(&self, max: u32) {
        *self.inner.global_max_retries.write() = max;
    }

    /// Get the global maximum retries.
    pub fn global_max_retries(&self) -> u32 {
        *self.inner.global_max_retries.read()
    }

    /// Get the current degradation level.
    pub fn degradation_level(&self) -> DegradationLevel {
        *self.inner.degradation_level.read()
    }

    /// Adjust degradation level based on failure patterns.
    fn adjust_degradation_level(&self) {
        let states = self.inner.recovery_states.read();
        let degraded_count = states.values().filter(|s| s.degraded).count();
        let total = states.len();

        let level = if total == 0 {
            DegradationLevel::None
        } else {
            let ratio = degraded_count as f64 / total as f64;
            if ratio < 0.05 {
                DegradationLevel::Minor
            } else if ratio < 0.15 {
                DegradationLevel::Moderate
            } else if ratio < 0.35 {
                DegradationLevel::Severe
            } else {
                DegradationLevel::Critical
            }
        };

        *self.inner.degradation_level.write() = level;
    }

    /// Manually set the degradation level.
    pub fn set_degradation_level(&self, level: DegradationLevel) {
        *self.inner.degradation_level.write() = level;
    }

    /// Get recovery state for a task.
    pub fn recovery_state(&self, task_id: TaskId) -> Option<TaskRecoveryState> {
        self.inner.recovery_states.read().get(&task_id).cloned()
    }

    /// Get the recovery log.
    pub fn recovery_log(&self) -> Vec<RecoveryAttempt> {
        self.inner.recovery_log.read().clone()
    }

    /// Get total recovery attempts.
    pub fn total_recovery_attempts(&self) -> usize {
        self.inner.recovery_log.read().len()
    }

    /// Get total successful recoveries.
    pub fn successful_recoveries(&self) -> usize {
        self.inner
            .recovery_log
            .read()
            .iter()
            .filter(|a| a.success)
            .count()
    }

    /// Clear recovery state for a task.
    pub fn clear_task_recovery(&self, task_id: TaskId) {
        self.inner.recovery_states.write().remove(&task_id);
    }

    /// Get all degraded tasks.
    pub fn degraded_tasks(&self) -> Vec<TaskId> {
        self.inner
            .recovery_states
            .read()
            .values()
            .filter(|s| s.degraded)
            .map(|s| s.task_id)
            .collect()
    }
}

impl Default for FailureRecovery {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkpoint_creation() {
        let recovery = FailureRecovery::new();
        let task_id = TaskId::new();

        let cp = recovery.create_checkpoint(
            task_id,
            serde_json::json!({"step": 1}),
            0,
            "initial".to_string(),
        );

        assert_eq!(cp.task_id, task_id);
        assert_eq!(cp.step_index, 0);
    }

    #[test]
    fn checkpoint_resume() {
        let recovery = FailureRecovery::new();
        let task_id = TaskId::new();

        recovery.create_checkpoint(
            task_id,
            serde_json::json!({"data": "test"}),
            3,
            "step 3".to_string(),
        );

        let resumed = recovery.resume_from_checkpoint(task_id).unwrap();
        assert!(resumed.is_some());
        assert_eq!(resumed.unwrap().step_index, 3);
    }

    #[test]
    fn strategy_determination() {
        let recovery = FailureRecovery::new();
        let task_id = TaskId::new();

        let strategy = recovery.determine_strategy(task_id, "generic error");
        assert!(matches!(strategy, FallbackStrategy::Retry));
    }

    #[test]
    fn degradation_tracking() {
        let recovery = FailureRecovery::new();
        let task_id = TaskId::new();

        recovery.record_recovery_attempt(
            task_id,
            FallbackStrategy::Retry,
            false,
            Some("error".to_string()),
            100,
        );

        let state = recovery.recovery_state(task_id).unwrap();
        assert_eq!(state.total_retries, 1);
    }

    #[test]
    fn fallback_registration() {
        let recovery = FailureRecovery::new();
        recovery.register_fallback(
            "timeout".to_string(),
            FallbackConfig {
                strategy: FallbackStrategy::DegradeGracefully,
                max_retries: 1,
                retry_delay_ms: 500,
                alternative_description: None,
            },
        );
    }

    #[test]
    fn recovery_log() {
        let recovery = FailureRecovery::new();
        let task_id = TaskId::new();

        recovery.record_recovery_attempt(
            task_id,
            FallbackStrategy::Retry,
            true,
            None,
            50,
        );

        assert_eq!(recovery.total_recovery_attempts(), 1);
        assert_eq!(recovery.successful_recoveries(), 1);
    }
}
