//! Permission management for tool access control.

use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::error::{ToolError, ToolResult};

// ---------------------------------------------------------------------------
// PermissionScope
// ---------------------------------------------------------------------------

/// Scope of a permission grant.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PermissionScope {
    Filesystem,
    Network,
    Shell,
    Process,
    Database,
    Cloud,
    Browser,
    ToolSpecific(String),
}

impl std::fmt::Display for PermissionScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Filesystem => write!(f, "filesystem"),
            Self::Network => write!(f, "network"),
            Self::Shell => write!(f, "shell"),
            Self::Process => write!(f, "process"),
            Self::Database => write!(f, "database"),
            Self::Cloud => write!(f, "cloud"),
            Self::Browser => write!(f, "browser"),
            Self::ToolSpecific(s) => write!(f, "tool:{}", s),
        }
    }
}

// ---------------------------------------------------------------------------
// PermissionPolicy
// ---------------------------------------------------------------------------

/// Policy determining how permissions are evaluated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionPolicy {
    /// Allow all operations within a scope.
    AllowAll,
    /// Deny all operations within a scope.
    DenyAll,
    /// Allow specific operations.
    AllowList(Vec<String>),
    /// Deny specific operations.
    DenyList(Vec<String>),
}

impl PermissionPolicy {
    pub fn is_allowed(&self, operation: &str) -> bool {
        match self {
            Self::AllowAll => true,
            Self::DenyAll => false,
            Self::AllowList(ops) => ops.contains(&operation.to_string()),
            Self::DenyList(ops) => !ops.contains(&operation.to_string()),
        }
    }
}

// ---------------------------------------------------------------------------
// ToolPermission
// ---------------------------------------------------------------------------

/// Permission entry for a specific caller accessing a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPermission {
    pub grant_id: String,
    pub tool_name: String,
    pub caller_id: String,
    pub scope: PermissionScope,
    pub policy: PermissionPolicy,
    pub rate_limit_per_minute: Option<u32>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl ToolPermission {
    pub fn new(
        tool_name: impl Into<String>,
        caller_id: impl Into<String>,
        scope: PermissionScope,
        policy: PermissionPolicy,
    ) -> Self {
        Self {
            grant_id: uuid::Uuid::new_v4().to_string(),
            tool_name: tool_name.into(),
            caller_id: caller_id.into(),
            scope,
            policy,
            rate_limit_per_minute: None,
            expires_at: None,
        }
    }

    pub fn with_rate_limit(mut self, per_minute: u32) -> Self {
        self.rate_limit_per_minute = Some(per_minute);
        self
    }

    pub fn with_expiry(mut self, expires_at: chrono::DateTime<chrono::Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|exp| chrono::Utc::now() > exp)
            .unwrap_or(false)
    }

    pub fn is_allowed(&self, operation: &str) -> bool {
        if self.is_expired() {
            return false;
        }
        self.policy.is_allowed(operation)
    }
}

// ---------------------------------------------------------------------------
// RateLimiter
// ---------------------------------------------------------------------------

/// Simple sliding-window rate limiter.
struct RateLimiter {
    limit: u32,
    window_ms: u64,
    timestamps: Vec<u64>,
}

impl RateLimiter {
    fn new(limit: u32, window_ms: u64) -> Self {
        Self {
            limit,
            window_ms,
            timestamps: Vec::new(),
        }
    }

    fn check(&mut self, now_ms: u64) -> bool {
        self.timestamps
            .retain(|&ts| now_ms.saturating_sub(ts) < self.window_ms);
        if self.timestamps.len() < self.limit as usize {
            self.timestamps.push(now_ms);
            true
        } else {
            false
        }
    }
}

// ---------------------------------------------------------------------------
// PermissionManager
// ---------------------------------------------------------------------------

/// Central permission manager that evaluates access control for tool executions.
pub struct PermissionManager {
    permissions: DashMap<String, Vec<ToolPermission>>,
    rate_limiters: DashMap<String, RateLimiter>,
}

impl std::fmt::Debug for PermissionManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PermissionManager")
            .field("permission_entries", &self.permissions.len())
            .field("rate_limiter_count", &self.rate_limiters.len())
            .finish()
    }
}

impl PermissionManager {
    pub fn new() -> Self {
        Self {
            permissions: DashMap::new(),
            rate_limiters: DashMap::new(),
        }
    }

    /// Grant a permission.
    pub fn grant(&self, perm: ToolPermission) {
        let key = format!("{}:{}", perm.tool_name, perm.caller_id);
        self.permissions.entry(key).or_default().push(perm);
    }

    /// Revoke a specific permission by grant ID.
    pub fn revoke(&self, tool_name: &str, caller_id: &str, grant_id: &str) -> bool {
        let key = format!("{}:{}", tool_name, caller_id);
        if let Some(mut perms) = self.permissions.get_mut(&key) {
            let len_before = perms.len();
            perms.retain(|p| p.grant_id != grant_id);
            perms.len() < len_before
        } else {
            false
        }
    }

    /// Revoke all permissions for a caller on a tool.
    pub fn revoke_all(&self, tool_name: &str, caller_id: &str) -> bool {
        let key = format!("{}:{}", tool_name, caller_id);
        self.permissions.remove(&key).is_some()
    }

    /// Check if a caller has permission to execute an operation on a tool.
    pub fn check(&self, tool_name: &str, caller_id: &str, operation: &str) -> ToolResult<()> {
        let key = format!("{}:{}", tool_name, caller_id);
        let perms = self.permissions.get(&key);

        match perms {
            Some(perms) => {
                let has_allowed = perms.iter().any(|p| p.is_allowed(operation));
                if has_allowed {
                    // Check rate limits
                    for perm in perms.iter() {
                        if let Some(limit) = perm.rate_limit_per_minute {
                            let limiter_key = format!("{}:{}", perm.grant_id, caller_id);
                            let now_ms = chrono::Utc::now().timestamp_millis() as u64;
                            let mut limiter = self
                                .rate_limiters
                                .entry(limiter_key)
                                .or_insert_with(|| RateLimiter::new(limit, 60_000));
                            if !limiter.check(now_ms) {
                                return Err(ToolError::rate_limited(format!(
                                    "rate limit of {limit}/min exceeded for tool '{tool_name}'"
                                )));
                            }
                        }
                    }
                    Ok(())
                } else {
                    Err(ToolError::permission_denied(format!(
                        "caller '{}' denied operation '{}' on tool '{}'",
                        caller_id, operation, tool_name
                    )))
                }
            }
            None => Err(ToolError::permission_denied(format!(
                "no permissions granted for caller '{}' on tool '{}'",
                caller_id, tool_name
            ))),
        }
    }

    /// Get all permissions for a caller.
    pub fn permissions_for(&self, caller_id: &str) -> Vec<ToolPermission> {
        self.permissions
            .iter()
            .filter(|entry| entry.value().iter().any(|p| p.caller_id == caller_id))
            .flat_map(|entry| entry.value().clone())
            .collect()
    }

    /// Get all permissions for a tool.
    pub fn permissions_for_tool(&self, tool_name: &str) -> Vec<ToolPermission> {
        self.permissions
            .iter()
            .filter(|entry| entry.key().starts_with(&format!("{tool_name}:")))
            .flat_map(|entry| entry.value().clone())
            .collect()
    }

    /// Remove expired permissions.
    pub fn cleanup_expired(&self) -> usize {
        let mut removed = 0;
        let keys: Vec<String> = self.permissions.iter().map(|e| e.key().clone()).collect();
        for key in keys {
            if let Some(mut entry) = self.permissions.get_mut(&key) {
                let len_before = entry.value().len();
                entry.value_mut().retain(|p| !p.is_expired());
                removed += len_before - entry.value().len();
            }
        }
        removed
    }
}

impl Default for PermissionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grant_and_check() {
        let pm = PermissionManager::new();
        let perm = ToolPermission::new(
            "file_tool",
            "agent_1",
            PermissionScope::Filesystem,
            PermissionPolicy::AllowAll,
        );
        pm.grant(perm);

        assert!(pm.check("file_tool", "agent_1", "read").is_ok());
    }

    #[test]
    fn test_deny() {
        let pm = PermissionManager::new();
        let perm = ToolPermission::new(
            "shell_tool",
            "agent_1",
            PermissionScope::Shell,
            PermissionPolicy::DenyAll,
        );
        pm.grant(perm);

        assert!(pm.check("shell_tool", "agent_1", "exec").is_err());
    }

    #[test]
    fn test_allow_list() {
        let pm = PermissionManager::new();
        let perm = ToolPermission::new(
            "file_tool",
            "agent_1",
            PermissionScope::Filesystem,
            PermissionPolicy::AllowList(vec!["read".into(), "write".into()]),
        );
        pm.grant(perm);

        assert!(pm.check("file_tool", "agent_1", "read").is_ok());
        assert!(pm.check("file_tool", "agent_1", "write").is_ok());
        assert!(pm.check("file_tool", "agent_1", "delete").is_err());
    }

    #[test]
    fn test_revoke() {
        let pm = PermissionManager::new();
        let perm = ToolPermission::new(
            "tool_a",
            "agent_1",
            PermissionScope::Network,
            PermissionPolicy::AllowAll,
        );
        let grant_id = perm.grant_id.clone();
        pm.grant(perm);

        assert!(pm.check("tool_a", "agent_1", "GET").is_ok());
        assert!(pm.revoke("tool_a", "agent_1", &grant_id));
        assert!(pm.check("tool_a", "agent_1", "GET").is_err());
    }

    #[test]
    fn test_expiry() {
        let pm = PermissionManager::new();
        let perm = ToolPermission::new(
            "tool_a",
            "agent_1",
            PermissionScope::Network,
            PermissionPolicy::AllowAll,
        )
        .with_expiry(chrono::Utc::now() - chrono::Duration::hours(1));

        pm.grant(perm);
        assert!(pm.check("tool_a", "agent_1", "GET").is_err());
    }
}
