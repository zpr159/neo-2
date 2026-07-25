use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{KnowledgeError, KnowledgeResult};

/// Status of a recovery operation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RecoveryStatus {
    NotStarted,
    InProgress,
    Completed,
    Failed,
}

/// A recovery plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryPlan {
    /// Plan id.
    pub id: String,
    /// When the plan was created.
    pub created_at: DateTime<Utc>,
    /// Steps in the recovery plan.
    pub steps: Vec<RecoveryStep>,
    /// Current status.
    pub status: RecoveryStatus,
    /// Description.
    pub description: String,
}

/// A single step in a recovery plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryStep {
    /// Step description.
    pub description: String,
    /// Whether this step has been completed.
    pub completed: bool,
    /// Whether this step is optional.
    pub optional: bool,
}

/// Manages graph recovery from snapshots or logs.
pub struct RecoveryManager {
    plans: parking_lot::RwLock<Vec<RecoveryPlan>>,
}

impl RecoveryManager {
    /// Create a new recovery manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            plans: parking_lot::RwLock::new(Vec::new()),
        }
    }

    /// Create a recovery plan.
    pub fn create_plan(&self, description: impl Into<String>) -> RecoveryPlan {
        let plan = RecoveryPlan {
            id: format!("recovery-{}", chrono::Utc::now().timestamp_millis()),
            created_at: Utc::now(),
            steps: Vec::new(),
            status: RecoveryStatus::NotStarted,
            description: description.into(),
        };
        self.plans.write().push(plan.clone());
        plan
    }

    /// Add a step to a recovery plan.
    pub fn add_step(
        &self,
        plan_id: &str,
        description: impl Into<String>,
        optional: bool,
    ) -> KnowledgeResult<()> {
        let mut plans = self.plans.write();
        let plan = plans
            .iter_mut()
            .find(|p| p.id == plan_id)
            .ok_or_else(|| KnowledgeError::RecoveryError(format!("Plan '{}' not found", plan_id)))?;

        plan.steps.push(RecoveryStep {
            description: description.into(),
            completed: false,
            optional,
        });
        Ok(())
    }

    /// Mark a step as completed.
    pub fn complete_step(&self, plan_id: &str, step_index: usize) -> KnowledgeResult<()> {
        let mut plans = self.plans.write();
        let plan = plans
            .iter_mut()
            .find(|p| p.id == plan_id)
            .ok_or_else(|| KnowledgeError::RecoveryError(format!("Plan '{}' not found", plan_id)))?;

        if step_index >= plan.steps.len() {
            return Err(KnowledgeError::RecoveryError(format!(
                "Step index {} out of range",
                step_index
            )));
        }

        plan.steps[step_index].completed = true;

        // Check if all non-optional steps are complete
        let all_done = plan
            .steps
            .iter()
            .filter(|s| !s.optional)
            .all(|s| s.completed);

        if all_done {
            plan.status = RecoveryStatus::Completed;
        } else {
            plan.status = RecoveryStatus::InProgress;
        }

        Ok(())
    }

    /// Get a plan by id.
    #[must_use]
    pub fn get_plan(&self, plan_id: &str) -> Option<RecoveryPlan> {
        self.plans.read().iter().find(|p| p.id == plan_id).cloned()
    }

    /// List all plans.
    #[must_use]
    pub fn list_plans(&self) -> Vec<RecoveryPlan> {
        self.plans.read().clone()
    }
}

impl Default for RecoveryManager {
    fn default() -> Self {
        Self::new()
    }
}
