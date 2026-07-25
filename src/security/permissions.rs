/// Permission management for the Neo security layer.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Individual permission grants within the system.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Permission {
    /// Read access to user resources.
    Read,
    /// Write access to user resources.
    Write,
    /// Execute access to user resources.
    Execute,
    /// Administrative access.
    Admin,
    /// Read access to system-level resources.
    SystemRead,
    /// Write access to system-level resources.
    SystemWrite,
    /// Ability to invoke tools.
    ToolUse,
    /// Ability to execute workflows.
    WorkflowExecute,
    /// Ability to control agents.
    AgentControl,
    /// Access to memory subsystems.
    MemoryAccess,
    /// Access to knowledge stores.
    KnowledgeAccess,
    /// Access to the world model.
    WorldModelAccess,
}

impl std::fmt::Display for Permission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Execute => "execute",
            Self::Admin => "admin",
            Self::SystemRead => "system_read",
            Self::SystemWrite => "system_write",
            Self::ToolUse => "tool_use",
            Self::WorkflowExecute => "workflow_execute",
            Self::AgentControl => "agent_control",
            Self::MemoryAccess => "memory_access",
            Self::KnowledgeAccess => "knowledge_access",
            Self::WorldModelAccess => "world_model_access",
        };
        write!(f, "{label}")
    }
}

/// A set of permissions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionSet {
    /// The permissions in this set.
    pub permissions: Vec<Permission>,
}

impl PermissionSet {
    /// Create an empty permission set.
    pub fn new() -> Self {
        Self {
            permissions: Vec::new(),
        }
    }

    /// Check whether the set contains a permission.
    pub fn has(&self, permission: &Permission) -> bool {
        self.permissions.contains(permission)
    }

    /// Add a permission to the set.
    pub fn add(&mut self, permission: Permission) {
        if !self.has(&permission) {
            self.permissions.push(permission);
        }
    }

    /// Remove a permission from the set.
    pub fn remove(&mut self, permission: &Permission) {
        self.permissions.retain(|p| p != permission);
    }

    /// Check whether the set includes the Admin permission.
    pub fn is_admin(&self) -> bool {
        self.has(&Permission::Admin)
    }
}

impl Default for PermissionSet {
    fn default() -> Self {
        Self::new()
    }
}

/// Manages per-user permission grants.
#[derive(Debug)]
pub struct PermissionManager {
    /// user_id -> PermissionSet
    grants: RwLock<HashMap<String, PermissionSet>>,
}

impl PermissionManager {
    /// Create a new, empty PermissionManager.
    pub fn new() -> Self {
        Self {
            grants: RwLock::new(HashMap::new()),
        }
    }

    /// Grant a permission to a user.
    pub async fn grant(&self, user_id: &str, permission: Permission) {
        let mut grants = self.grants.write().await;
        grants
            .entry(user_id.to_string())
            .or_insert_with(PermissionSet::new)
            .add(permission);
        tracing::info!(user = user_id, "permission granted");
    }

    /// Revoke a permission from a user.
    pub async fn revoke(&self, user_id: &str, permission: &Permission) {
        let mut grants = self.grants.write().await;
        if let Some(set) = grants.get_mut(user_id) {
            set.remove(permission);
            tracing::info!(user = user_id, "permission revoked");
        }
    }

    /// Check whether a user holds a specific permission.
    pub async fn check(&self, user_id: &str, permission: &Permission) -> bool {
        let grants = self.grants.read().await;
        match grants.get(user_id) {
            Some(set) => set.is_admin() || set.has(permission),
            None => false,
        }
    }

    /// List all permissions currently granted to a user.
    pub async fn list_permissions(&self, user_id: &str) -> Vec<Permission> {
        let grants = self.grants.read().await;
        match grants.get(user_id) {
            Some(set) => set.permissions.clone(),
            None => Vec::new(),
        }
    }
}

impl Default for PermissionManager {
    fn default() -> Self {
        Self::new()
    }
}
