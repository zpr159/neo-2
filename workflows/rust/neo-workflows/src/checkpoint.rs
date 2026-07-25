use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::core::{CheckpointId, ExecutionId, NodeId, WorkflowContext, WorkflowId, WorkflowState};
use crate::error::WorkflowResult;

/// Snapshot of a single node's execution state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSnapshot {
    pub node_id: NodeId,
    pub state: crate::core::NodeState,
    pub output: Option<serde_json::Value>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub retries: u32,
}

/// A complete checkpoint capturing workflow execution state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowCheckpoint {
    pub id: CheckpointId,
    pub execution_id: ExecutionId,
    pub workflow_id: WorkflowId,
    pub state: WorkflowState,
    pub variables: HashMap<String, serde_json::Value>,
    pub node_snapshots: HashMap<NodeId, NodeSnapshot>,
    pub created_at: DateTime<Utc>,
    pub checksum: String,
}

impl WorkflowCheckpoint {
    /// Compute a SHA-256 checksum of the checkpoint contents.
    fn compute_checksum(
        execution_id: &ExecutionId,
        workflow_id: &WorkflowId,
        state: &WorkflowState,
        variables: &HashMap<String, serde_json::Value>,
        node_snapshots: &HashMap<NodeId, NodeSnapshot>,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(execution_id.to_string().as_bytes());
        hasher.update(workflow_id.to_string().as_bytes());
        hasher.update(format!("{state:?}").as_bytes());

        let mut keys: Vec<&String> = variables.keys().collect();
        keys.sort();
        for k in keys {
            hasher.update(k.as_bytes());
            if let Ok(v) = serde_json::to_string(&variables[k]) {
                hasher.update(v.as_bytes());
            }
        }

        let mut node_ids: Vec<&NodeId> = node_snapshots.keys().collect();
        node_ids.sort_by(|a, b| a.to_string().cmp(&b.to_string()));
        for nid in node_ids {
            hasher.update(nid.to_string().as_bytes());
            if let Ok(v) = serde_json::to_string(&node_snapshots[nid]) {
                hasher.update(v.as_bytes());
            }
        }

        format!("{:x}", hasher.finalize())
    }
}

/// Manages workflow checkpoints.
#[derive(Debug)]
pub struct CheckpointManager {
    checkpoints: Vec<WorkflowCheckpoint>,
    max_checkpoints: usize,
}

impl CheckpointManager {
    #[must_use]
    pub fn new(max_checkpoints: usize) -> Self {
        Self {
            checkpoints: Vec::new(),
            max_checkpoints,
        }
    }

    /// Create a checkpoint from current execution state.
    pub fn create_checkpoint(
        &mut self,
        execution_id: ExecutionId,
        workflow_id: WorkflowId,
        state: WorkflowState,
        context: &WorkflowContext,
        node_snapshots: HashMap<NodeId, NodeSnapshot>,
    ) -> WorkflowCheckpoint {
        let variables = context.snapshot_variables();

        let checksum = WorkflowCheckpoint::compute_checksum(
            &execution_id,
            &workflow_id,
            &state,
            &variables,
            &node_snapshots,
        );

        let checkpoint = WorkflowCheckpoint {
            id: CheckpointId::new(),
            execution_id,
            workflow_id,
            state,
            variables,
            node_snapshots,
            created_at: Utc::now(),
            checksum,
        };

        self.checkpoints.push(checkpoint.clone());

        // Trim oldest if over limit
        if self.checkpoints.len() > self.max_checkpoints {
            let excess = self.checkpoints.len() - self.max_checkpoints;
            self.checkpoints.drain(0..excess);
        }

        checkpoint
    }

    /// Get the latest checkpoint for a given execution.
    #[must_use]
    pub fn latest(&self, execution_id: &ExecutionId) -> Option<&WorkflowCheckpoint> {
        self.checkpoints
            .iter()
            .rev()
            .find(|c| c.execution_id == *execution_id)
    }

    /// Get all checkpoints for an execution.
    #[must_use]
    pub fn for_execution(&self, execution_id: &ExecutionId) -> Vec<&WorkflowCheckpoint> {
        self.checkpoints
            .iter()
            .filter(|c| c.execution_id == *execution_id)
            .collect()
    }

    /// Verify checkpoint integrity.
    #[must_use]
    pub fn verify(checkpoint: &WorkflowCheckpoint) -> bool {
        let expected = WorkflowCheckpoint::compute_checksum(
            &checkpoint.execution_id,
            &checkpoint.workflow_id,
            &checkpoint.state,
            &checkpoint.variables,
            &checkpoint.node_snapshots,
        );
        checkpoint.checksum == expected
    }

    /// Delete all checkpoints for an execution.
    pub fn delete_execution_checkpoints(&mut self, execution_id: &ExecutionId) {
        self.checkpoints.retain(|c| c.execution_id != *execution_id);
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.checkpoints.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::NodeState;

    #[test]
    fn create_and_retrieve_checkpoint() {
        let mut mgr = CheckpointManager::new(10);
        let exec_id = ExecutionId::new();
        let wf_id = WorkflowId::new();
        let ctx = WorkflowContext::new();

        let cp =
            mgr.create_checkpoint(exec_id, wf_id, WorkflowState::Running, &ctx, HashMap::new());

        assert!(CheckpointManager::verify(&cp));
        assert_eq!(mgr.count(), 1);
        assert!(mgr.latest(&exec_id).is_some());
    }

    #[test]
    fn trim_excess() {
        let mut mgr = CheckpointManager::new(3);
        let wf_id = WorkflowId::new();
        let ctx = WorkflowContext::new();

        for _ in 0..5 {
            mgr.create_checkpoint(
                ExecutionId::new(),
                wf_id,
                WorkflowState::Running,
                &ctx,
                HashMap::new(),
            );
        }

        assert_eq!(mgr.count(), 3);
    }

    #[test]
    fn verify_integrity() {
        let mut mgr = CheckpointManager::new(10);
        let cp = mgr.create_checkpoint(
            ExecutionId::new(),
            WorkflowId::new(),
            WorkflowState::Running,
            &WorkflowContext::new(),
            HashMap::new(),
        );
        assert!(CheckpointManager::verify(&cp));
    }
}
