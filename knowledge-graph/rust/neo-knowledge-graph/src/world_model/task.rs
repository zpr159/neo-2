use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::core::entity::{Entity, EntityId};

/// Status of a task.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

/// A task in the world model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEntity {
    pub id: EntityId,
    pub name: String,
    pub description: String,
    pub status: TaskStatus,
    pub priority: u32,
    pub assignee_id: Option<EntityId>,
    pub project_id: Option<EntityId>,
    pub deadline: Option<DateTime<Utc>>,
    pub properties: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

impl TaskEntity {
    #[must_use]
    pub fn from_entity(entity: &Entity) -> Self {
        let status_str = entity
            .get_property("status")
            .and_then(|v| v.as_str())
            .unwrap_or("pending");
        let status = match status_str {
            "in_progress" => TaskStatus::InProgress,
            "completed" => TaskStatus::Completed,
            "failed" => TaskStatus::Failed,
            "cancelled" => TaskStatus::Cancelled,
            _ => TaskStatus::Pending,
        };

        Self {
            id: entity.id,
            name: entity.label.clone(),
            description: entity.description.clone(),
            status,
            priority: entity
                .get_property("priority")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32,
            assignee_id: entity
                .get_property("assignee_id")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
                .map(EntityId),
            project_id: entity
                .get_property("project_id")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
                .map(EntityId),
            deadline: entity
                .get_property("deadline")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok()),
            properties: entity.properties.clone(),
            created_at: entity.created_at,
        }
    }
}
