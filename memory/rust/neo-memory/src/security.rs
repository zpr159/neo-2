use std::collections::HashMap;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::error::{MemoryError, MemoryResult};
use crate::types::{
    AuditEntry, MemoryId, MemoryNamespace, MemoryPermission, SecurityConfig,
};

/// Permission entry for a specific namespace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamespacePermission {
    /// Namespace name.
    pub namespace: String,
    /// Actor (user or service name).
    pub actor: String,
    /// Permission level.
    pub permission: MemoryPermission,
    /// When this permission was granted.
    pub granted_at: DateTime<Utc>,
    /// Who granted this permission.
    pub granted_by: Option<String>,
}

/// Memory security manager providing encryption, permissions, namespaces, isolation, and audit logging.
pub struct MemorySecurity {
    config: SecurityConfig,
    /// Namespace permissions: namespace -> actor -> permission.
    namespace_permissions: DashMap<String, DashMap<String, MemoryPermission>>,
    /// Global permissions: actor -> permission.
    global_permissions: DashMap<String, MemoryPermission>,
    /// Audit log.
    audit_log: RwLock<Vec<AuditEntry>>,
    /// Maximum audit log size.
    max_audit_entries: usize,
}

impl MemorySecurity {
    /// Create a new security manager.
    #[must_use]
    pub fn new(config: SecurityConfig) -> Self {
        Self {
            config,
            namespace_permissions: DashMap::new(),
            global_permissions: DashMap::new(),
            audit_log: RwLock::new(Vec::new()),
            max_audit_entries: 100_000,
        }
    }

    /// Check if an actor has the required permission for a namespace.
    pub fn check_permission(
        &self,
        actor: &str,
        namespace: &MemoryNamespace,
        required: MemoryPermission,
    ) -> bool {
        if !self.config.enabled {
            return true;
        }

        // Check global permissions first.
        if let Some(global_perm) = self.global_permissions.get(actor) {
            if *global_perm >= required {
                return true;
            }
        }

        // Check namespace-specific permissions.
        if let Some(ns_perms) = self.namespace_permissions.get(&namespace.0) {
            if let Some(perm) = ns_perms.get(actor) {
                return *perm >= required;
            }
        }

        // Default: deny.
        false
    }

    /// Grant a permission to an actor for a namespace.
    pub fn grant_permission(
        &self,
        actor: &str,
        namespace: &MemoryNamespace,
        permission: MemoryPermission,
        granted_by: Option<&str>,
    ) {
        self.namespace_permissions
            .entry(namespace.0.clone())
            .or_default()
            .insert(actor.to_string(), permission);

        if self.config.audit_logging {
            self.log_audit(AuditEntry {
                timestamp: Utc::now(),
                action: "grant_permission".to_string(),
                memory_id: MemoryId::new(),
                namespace: namespace.clone(),
                actor: granted_by.unwrap_or("system").to_string(),
                permitted: true,
                details: Some(format!(
                    "Granted {permission} to {actor} on namespace {}",
                    namespace.0
                )),
            });
        }
    }

    /// Grant global permission to an actor.
    pub fn grant_global_permission(
        &self,
        actor: &str,
        permission: MemoryPermission,
    ) {
        self.global_permissions
            .insert(actor.to_string(), permission);
    }

    /// Revoke a permission.
    pub fn revoke_permission(&self, actor: &str, namespace: &MemoryNamespace) {
        if let Some(mut ns_perms) = self.namespace_permissions.get_mut(&namespace.0) {
            ns_perms.remove(actor);
        }
    }

    /// Get all permissions for a namespace.
    #[must_use]
    pub fn namespace_permissions(&self, namespace: &str) -> Vec<NamespacePermission> {
        self.namespace_permissions
            .get(namespace)
            .map(|perms| {
                perms
                    .iter()
                    .map(|entry| NamespacePermission {
                        namespace: namespace.to_string(),
                        actor: entry.key().clone(),
                        permission: *entry.value(),
                        granted_at: Utc::now(),
                        granted_by: None,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Log an audit entry.
    pub fn log_audit(&self, entry: AuditEntry) {
        let mut log = self.audit_log.write();
        if log.len() >= self.max_audit_entries {
            log.remove(0);
        }
        log.push(entry);
    }

    /// Get audit log entries.
    #[must_use]
    pub fn audit_log(&self) -> Vec<AuditEntry> {
        self.audit_log.read().clone()
    }

    /// Get audit log entries for a specific namespace.
    #[must_use]
    pub fn audit_log_for_namespace(&self, namespace: &str) -> Vec<AuditEntry> {
        self.audit_log
            .read()
            .iter()
            .filter(|e| e.namespace.0 == namespace)
            .cloned()
            .collect()
    }

    /// Get audit log entries for a specific actor.
    #[must_use]
    pub fn audit_log_for_actor(&self, actor: &str) -> Vec<AuditEntry> {
        self.audit_log
            .read()
            .iter()
            .filter(|e| e.actor == actor)
            .cloned()
            .collect()
    }

    /// Encrypt data using XOR-based obfuscation (production would use AES-GCM).
    #[must_use]
    pub fn encrypt(&self, data: &[u8]) -> Vec<u8> {
        if !self.config.enabled || self.config.encryption_key.is_none() {
            return data.to_vec();
        }

        let key = self.config.encryption_key.as_deref().unwrap_or("default");
        let key_bytes = key.as_bytes();

        data.iter()
            .enumerate()
            .map(|(i, &byte)| byte ^ key_bytes[i % key_bytes.len()])
            .collect()
    }

    /// Decrypt data.
    #[must_use]
    pub fn decrypt(&self, data: &[u8]) -> Vec<u8> {
        // XOR encryption is symmetric.
        self.encrypt(data)
    }

    /// Clear the audit log.
    pub fn clear_audit_log(&self) {
        self.audit_log.write().clear();
    }

    /// Check if encryption is enabled.
    #[must_use]
    pub fn is_encryption_enabled(&self) -> bool {
        self.config.enabled && self.config.encryption_key.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission_check_disabled() {
        let config = SecurityConfig {
            enabled: false,
            ..SecurityConfig::default()
        };
        let security = MemorySecurity::new(config);

        assert!(security.check_permission(
            "anyone",
            &MemoryNamespace::global(),
            MemoryPermission::Admin,
        ));
    }

    #[test]
    fn grant_and_check() {
        let config = SecurityConfig {
            enabled: true,
            ..SecurityConfig::default()
        };
        let security = MemorySecurity::new(config);
        let ns = MemoryNamespace::new("project_a");

        security.grant_permission("alice", &ns, MemoryPermission::Write, None);

        assert!(security.check_permission("alice", &ns, MemoryPermission::Read));
        assert!(security.check_permission("alice", &ns, MemoryPermission::Write));
        assert!(!security.check_permission("alice", &ns, MemoryPermission::Admin));
        assert!(!security.check_permission("bob", &ns, MemoryPermission::Read));
    }

    #[test]
    fn global_permission() {
        let config = SecurityConfig {
            enabled: true,
            ..SecurityConfig::default()
        };
        let security = MemorySecurity::new(config);
        security.grant_global_permission("admin", MemoryPermission::Admin);

        assert!(security.check_permission(
            "admin",
            &MemoryNamespace::new("any"),
            MemoryPermission::Admin,
        ));
    }

    #[test]
    fn audit_logging() {
        let config = SecurityConfig {
            enabled: true,
            audit_logging: true,
            ..SecurityConfig::default()
        };
        let security = MemorySecurity::new(config);

        security.log_audit(AuditEntry {
            timestamp: Utc::now(),
            action: "test".to_string(),
            memory_id: MemoryId::new(),
            namespace: MemoryNamespace::global(),
            actor: "tester".to_string(),
            permitted: true,
            details: None,
        });

        assert_eq!(security.audit_log().len(), 1);
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let config = SecurityConfig {
            enabled: true,
            encryption_key: Some("my_secret_key".to_string()),
            ..SecurityConfig::default()
        };
        let security = MemorySecurity::new(config);

        let original = b"hello world";
        let encrypted = security.encrypt(original);
        let decrypted = security.decrypt(&encrypted);
        assert_eq!(original.to_vec(), decrypted);
    }

    #[test]
    fn revoke_permission() {
        let config = SecurityConfig {
            enabled: true,
            ..SecurityConfig::default()
        };
        let security = MemorySecurity::new(config);
        let ns = MemoryNamespace::new("test");

        security.grant_permission("user", &ns, MemoryPermission::Write, None);
        assert!(security.check_permission("user", &ns, MemoryPermission::Write));

        security.revoke_permission("user", &ns);
        assert!(!security.check_permission("user", &ns, MemoryPermission::Write));
    }
}
