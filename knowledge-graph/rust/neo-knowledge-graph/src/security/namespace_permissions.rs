use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Permission level for namespace access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PermissionLevel {
    Read = 0,
    Write = 1,
    Admin = 2,
}

/// Permissions for a specific namespace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamespacePermission {
    /// Namespace name.
    pub namespace: String,
    /// Allowed permission level.
    pub level: PermissionLevel,
}

/// Manages namespace-level permissions.
pub struct NamespacePermissions {
    permissions: parking_lot::RwLock<HashMap<String, PermissionLevel>>,
}

impl NamespacePermissions {
    /// Create a new permissions manager with default permissions.
    #[must_use]
    pub fn new() -> Self {
        let mut permissions = HashMap::new();
        permissions.insert("default".to_string(), PermissionLevel::Admin);
        Self {
            permissions: parking_lot::RwLock::new(permissions),
        }
    }

    /// Set permissions for a namespace.
    pub fn set(&self, namespace: impl Into<String>, level: PermissionLevel) {
        self.permissions.write().insert(namespace.into(), level);
    }

    /// Get the permission level for a namespace.
    #[must_use]
    pub fn get(&self, namespace: &str) -> PermissionLevel {
        self.permissions
            .read()
            .get(namespace)
            .copied()
            .unwrap_or(PermissionLevel::Read)
    }

    /// Check if a given level is allowed for a namespace.
    #[must_use]
    pub fn check(&self, namespace: &str, required: PermissionLevel) -> bool {
        self.get(namespace) >= required
    }

    /// Remove permissions for a namespace.
    pub fn remove(&self, namespace: &str) -> Option<PermissionLevel> {
        self.permissions.write().remove(namespace)
    }
}

impl Default for NamespacePermissions {
    fn default() -> Self {
        Self::new()
    }
}
