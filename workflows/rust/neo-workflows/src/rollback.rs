use chrono::{DateTime, Utc};
use tracing::info;

use crate::core::{NodeId, WorkflowContext};
use crate::error::{WorkflowError, WorkflowResult};
use crate::execution::NodeExecutor;

use crate::definition::NodeDefinition;

/// Records compensation/rollback actions for a workflow.
#[derive(Debug, Clone)]
pub struct RollbackRecord {
    pub node_id: NodeId,
    pub action: String,
    pub timestamp: DateTime<Utc>,
    pub success: bool,
    pub error: Option<String>,
}

/// Manages workflow rollback (compensation) operations.
#[derive(Debug)]
pub struct RollbackManager {
    records: Vec<RollbackRecord>,
    enabled: bool,
}

impl RollbackManager {
    #[must_use]
    pub fn new(enabled: bool) -> Self {
        Self {
            records: Vec::new(),
            enabled,
        }
    }

    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    #[must_use]
    pub fn records(&self) -> &[RollbackRecord] {
        &self.records
    }

    #[must_use]
    pub fn completed_count(&self) -> usize {
        self.records.iter().filter(|r| r.success).count()
    }

    /// Execute rollback for a list of nodes in reverse order.
    pub async fn rollback_nodes(
        &mut self,
        nodes: Vec<NodeId>,
        executor: &dyn NodeExecutor,
        node_defs: &std::collections::HashMap<NodeId, NodeDefinition>,
        context: &WorkflowContext,
    ) {
        if !self.enabled {
            info!("Rollback disabled, skipping");
            return;
        }

        for node_id in nodes.into_iter().rev() {
            if let Some(node) = node_defs.get(&node_id) {
                if executor.can_compensate(node) {
                    let action = format!("compensate:{node_id}");
                    match executor.compensate(node, context).await {
                        Ok(()) => {
                            self.records.push(RollbackRecord {
                                node_id,
                                action,
                                timestamp: Utc::now(),
                                success: true,
                                error: None,
                            });
                        }
                        Err(e) => {
                            self.records.push(RollbackRecord {
                                node_id,
                                action,
                                timestamp: Utc::now(),
                                success: false,
                                error: Some(e.to_string()),
                            });
                        }
                    }
                }
            }
        }
    }

    /// Check if all compensations succeeded.
    #[must_use]
    pub fn all_succeeded(&self) -> bool {
        self.records.iter().all(|r| r.success)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rollback_disabled() {
        let mut rm = RollbackManager::new(false);
        assert!(!rm.is_enabled());
        assert_eq!(rm.records().len(), 0);
    }

    #[test]
    fn rollback_enabled_records() {
        let mut rm = RollbackManager::new(true);
        assert!(rm.is_enabled());
        rm.records.push(RollbackRecord {
            node_id: NodeId::new(),
            action: "test".into(),
            timestamp: Utc::now(),
            success: true,
            error: None,
        });
        assert_eq!(rm.completed_count(), 1);
        assert!(rm.all_succeeded());
    }
}
