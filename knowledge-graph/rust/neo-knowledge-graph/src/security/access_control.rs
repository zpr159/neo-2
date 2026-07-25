use crate::security::namespace_permissions::{NamespacePermissions, PermissionLevel};

/// Controls access to the knowledge graph based on namespace permissions.
pub struct AccessController {
    permissions: NamespacePermissions,
}

impl AccessController {
    /// Create a new access controller.
    #[must_use]
    pub fn new() -> Self {
        Self {
            permissions: NamespacePermissions::new(),
        }
    }

    /// Check if read access is allowed for a namespace.
    #[must_use]
    pub fn can_read(&self, namespace: &str) -> bool {
        self.permissions.check(namespace, PermissionLevel::Read)
    }

    /// Check if write access is allowed for a namespace.
    #[must_use]
    pub fn can_write(&self, namespace: &str) -> bool {
        self.permissions.check(namespace, PermissionLevel::Write)
    }

    /// Check if admin access is allowed for a namespace.
    #[must_use]
    pub fn can_admin(&self, namespace: &str) -> bool {
        self.permissions.check(namespace, PermissionLevel::Admin)
    }

    /// Set the permission level for a namespace.
    pub fn set_permission(&self, namespace: impl Into<String>, level: PermissionLevel) {
        self.permissions.set(namespace, level);
    }

    /// Get the underlying permissions manager.
    #[must_use]
    pub fn permissions(&self) -> &NamespacePermissions {
        &self.permissions
    }
}

impl Default for AccessController {
    fn default() -> Self {
        Self::new()
    }
}
