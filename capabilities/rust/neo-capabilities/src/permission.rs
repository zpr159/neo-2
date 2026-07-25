use std::collections::{HashMap, HashSet};
use std::fmt;
#[allow(unused_imports)]
use std::sync::Arc;

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[allow(unused_imports)]
use crate::core::{CapabilityId, CapabilityMetadata};
use crate::error::{CapabilityError, CapabilityResult};

// ────────────────────────────────────────────────────────────────────────────
// Role
// ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Role {
    Admin,
    Operator,
    Viewer,
    Guest,
    Custom(String),
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Role::Admin => write!(f, "admin"),
            Role::Operator => write!(f, "operator"),
            Role::Viewer => write!(f, "viewer"),
            Role::Guest => write!(f, "guest"),
            Role::Custom(name) => write!(f, "custom:{}", name),
        }
    }
}

impl Role {
    pub fn default_permissions(&self) -> Vec<String> {
        match self {
            Role::Admin => vec![
                "execute".into(),
                "read".into(),
                "write".into(),
                "delete".into(),
                "register".into(),
                "unregister".into(),
                "enable".into(),
                "disable".into(),
                "approve".into(),
                "revoke".into(),
                "admin".into(),
            ],
            Role::Operator => vec![
                "execute".into(),
                "read".into(),
                "write".into(),
                "enable".into(),
                "disable".into(),
            ],
            Role::Viewer => vec!["read".into()],
            Role::Guest => vec!["read".into()],
            Role::Custom(_) => vec![],
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// RolePermissions
// ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RolePermissions {
    pub role: Role,
    pub allowed_capabilities: HashSet<CapabilityId>,
    pub denied_capabilities: HashSet<CapabilityId>,
    pub permissions: HashSet<String>,
}

impl RolePermissions {
    pub fn new(role: Role, permissions: Vec<String>) -> Self {
        let mut perm_set = HashSet::new();
        for p in permissions {
            perm_set.insert(p);
        }
        Self {
            role,
            allowed_capabilities: HashSet::new(),
            denied_capabilities: HashSet::new(),
            permissions: perm_set,
        }
    }

    pub fn can_access(&self, capability_id: &CapabilityId) -> bool {
        if self.denied_capabilities.contains(capability_id) {
            return false;
        }
        if self.allowed_capabilities.is_empty() {
            return true;
        }
        self.allowed_capabilities.contains(capability_id)
    }

    pub fn grant_access(&mut self, capability_id: CapabilityId) {
        self.denied_capabilities.remove(&capability_id);
        self.allowed_capabilities.insert(capability_id);
    }

    pub fn revoke_access(&mut self, capability_id: CapabilityId) {
        self.allowed_capabilities.remove(&capability_id);
        self.denied_capabilities.insert(capability_id);
    }

    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(permission) || self.permissions.contains("admin")
    }
}

// ────────────────────────────────────────────────────────────────────────────
// AllowDenyList
// ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AllowDenyList {
    pub allow_list: HashSet<String>,
    pub deny_list: HashSet<String>,
}

impl AllowDenyList {
    pub fn new() -> Self {
        Self {
            allow_list: HashSet::new(),
            deny_list: HashSet::new(),
        }
    }

    pub fn allow(&mut self, pattern: impl Into<String>) {
        self.allow_list.insert(pattern.into());
    }

    pub fn deny(&mut self, pattern: impl Into<String>) {
        self.deny_list.insert(pattern.into());
    }

    pub fn is_allowed(&self, identifier: &str) -> bool {
        if self.deny_list.contains(identifier) {
            return false;
        }
        if self.allow_list.is_empty() {
            return true;
        }
        self.allow_list.contains(identifier)
    }

    pub fn from_config(config: &serde_json::Value) -> Self {
        let mut list = Self::new();
        if let Some(allow) = config.get("allow").and_then(|v| v.as_array()) {
            for entry in allow {
                if let Some(s) = entry.as_str() {
                    list.allow_list.insert(s.to_string());
                }
            }
        }
        if let Some(deny) = config.get("deny").and_then(|v| v.as_array()) {
            for entry in deny {
                if let Some(s) = entry.as_str() {
                    list.deny_list.insert(s.to_string());
                }
            }
        }
        list
    }
}

// ────────────────────────────────────────────────────────────────────────────
// SandboxConfig
// ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub enabled: bool,
    pub max_memory_bytes: u64,
    pub max_cpu_time_ms: u64,
    pub max_inference_tokens: u32,
    pub network_access: bool,
    pub filesystem_access: bool,
    pub allowed_namespaces: Vec<String>,
}

impl SandboxConfig {
    pub fn new() -> Self {
        Self {
            enabled: true,
            max_memory_bytes: 256 * 1024 * 1024,
            max_cpu_time_ms: 30_000,
            max_inference_tokens: 4096,
            network_access: false,
            filesystem_access: false,
            allowed_namespaces: Vec::new(),
        }
    }

    pub fn default_restricted() -> Self {
        Self {
            enabled: true,
            max_memory_bytes: 64 * 1024 * 1024,
            max_cpu_time_ms: 5_000,
            max_inference_tokens: 1024,
            network_access: false,
            filesystem_access: false,
            allowed_namespaces: Vec::new(),
        }
    }

    pub fn default_permissive() -> Self {
        Self {
            enabled: false,
            max_memory_bytes: u64::MAX,
            max_cpu_time_ms: u64::MAX,
            max_inference_tokens: u32::MAX,
            network_access: true,
            filesystem_access: true,
            allowed_namespaces: vec![
                "neo.core".into(),
                "neo.inference".into(),
                "neo.reasoning".into(),
                "neo.memory".into(),
                "neo.knowledge".into(),
                "neo.developer".into(),
                "neo.communication".into(),
            ],
        }
    }

    pub fn validate_resource(&self, resource: &str, amount: u64) -> bool {
        if !self.enabled {
            return true;
        }
        match resource {
            "memory_bytes" => amount <= self.max_memory_bytes,
            "cpu_time_ms" => amount <= self.max_cpu_time_ms,
            "inference_tokens" => amount <= self.max_inference_tokens as u64,
            _ => true,
        }
    }
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self::new()
    }
}

// ────────────────────────────────────────────────────────────────────────────
// ResourceUsage / Sandbox
// ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_time_ms: u64,
    pub memory_bytes: u64,
    pub inference_tokens: u32,
}

pub struct Sandbox {
    pub config: SandboxConfig,
    pub active_resources: RwLock<ResourceUsage>,
}

impl Sandbox {
    pub fn new(config: SandboxConfig) -> Self {
        Self {
            config,
            active_resources: RwLock::new(ResourceUsage::default()),
        }
    }

    pub fn check_and_consume(&self, resource: &str, amount: u64) -> CapabilityResult<()> {
        if !self.config.enabled {
            return Ok(());
        }
        let mut usage = self.active_resources.write();
        match resource {
            "cpu_time_ms" => {
                let new_total = usage.cpu_time_ms.saturating_add(amount);
                if new_total > self.config.max_cpu_time_ms {
                    return Err(CapabilityError::sandbox_violation(format!(
                        "cpu_time_ms limit exceeded: requested {} would reach {} (max {})",
                        amount, new_total, self.config.max_cpu_time_ms
                    )));
                }
                usage.cpu_time_ms = new_total;
            }
            "memory_bytes" => {
                let new_total = usage.memory_bytes.saturating_add(amount);
                if new_total > self.config.max_memory_bytes {
                    return Err(CapabilityError::sandbox_violation(format!(
                        "memory_bytes limit exceeded: requested {} would reach {} (max {})",
                        amount, new_total, self.config.max_memory_bytes
                    )));
                }
                usage.memory_bytes = new_total;
            }
            "inference_tokens" => {
                let new_total = usage.inference_tokens.saturating_add(amount as u32);
                if new_total > self.config.max_inference_tokens {
                    return Err(CapabilityError::sandbox_violation(format!(
                        "inference_tokens limit exceeded: requested {} would reach {} (max {})",
                        amount, new_total, self.config.max_inference_tokens
                    )));
                }
                usage.inference_tokens = new_total;
            }
            _ => {}
        }
        Ok(())
    }

    pub fn get_usage(&self) -> ResourceUsage {
        self.active_resources.read().clone()
    }

    pub fn reset_usage(&self) {
        let mut usage = self.active_resources.write();
        *usage = ResourceUsage::default();
    }
}

// ────────────────────────────────────────────────────────────────────────────
// ApprovalStatus / ApprovalRequest
// ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Denied,
    Expired,
}

impl fmt::Display for ApprovalStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApprovalStatus::Pending => write!(f, "pending"),
            ApprovalStatus::Approved => write!(f, "approved"),
            ApprovalStatus::Denied => write!(f, "denied"),
            ApprovalStatus::Expired => write!(f, "expired"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub id: Uuid,
    pub capability_id: CapabilityId,
    pub requester: String,
    pub reason: String,
    pub status: ApprovalStatus,
    pub created_at: DateTime<Utc>,
    pub approved_at: Option<DateTime<Utc>>,
    pub approver: Option<String>,
}

impl ApprovalRequest {
    pub fn new(
        capability_id: CapabilityId,
        requester: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            capability_id,
            requester: requester.into(),
            reason: reason.into(),
            status: ApprovalStatus::Pending,
            created_at: Utc::now(),
            approved_at: None,
            approver: None,
        }
    }

    pub fn approve(&mut self, approver: impl Into<String>) {
        self.status = ApprovalStatus::Approved;
        self.approved_at = Some(Utc::now());
        self.approver = Some(approver.into());
    }

    pub fn deny(&mut self, approver: impl Into<String>) {
        self.status = ApprovalStatus::Denied;
        self.approved_at = Some(Utc::now());
        self.approver = Some(approver.into());
    }

    pub fn is_pending(&self) -> bool {
        self.status == ApprovalStatus::Pending
    }
}

// ────────────────────────────────────────────────────────────────────────────
// ApprovalManager
// ────────────────────────────────────────────────────────────────────────────

pub struct ApprovalManager {
    pub pending: RwLock<HashMap<Uuid, ApprovalRequest>>,
    pub history: RwLock<Vec<ApprovalRequest>>,
}

impl ApprovalManager {
    pub fn new() -> Self {
        Self {
            pending: RwLock::new(HashMap::new()),
            history: RwLock::new(Vec::new()),
        }
    }

    pub fn request_approval(
        &self,
        capability_id: CapabilityId,
        requester: impl Into<String>,
        reason: impl Into<String>,
    ) -> Uuid {
        let request = ApprovalRequest::new(capability_id, requester, reason);
        let id = request.id;
        self.pending.write().insert(id, request);
        id
    }

    pub fn approve(
        &self,
        request_id: Uuid,
        approver: impl Into<String>,
    ) -> CapabilityResult<()> {
        let mut pending = self.pending.write();
        let mut request = pending
            .remove(&request_id)
            .ok_or_else(|| CapabilityError::not_found(format!("approval request {}", request_id)))?;
        request.approve(approver);
        self.history.write().push(request);
        Ok(())
    }

    pub fn deny(
        &self,
        request_id: Uuid,
        approver: impl Into<String>,
    ) -> CapabilityResult<()> {
        let mut pending = self.pending.write();
        let mut request = pending
            .remove(&request_id)
            .ok_or_else(|| CapabilityError::not_found(format!("approval request {}", request_id)))?;
        request.deny(approver);
        self.history.write().push(request);
        Ok(())
    }

    pub fn get_request(&self, request_id: &Uuid) -> Option<ApprovalRequest> {
        self.pending
            .read()
            .get(request_id)
            .cloned()
            .or_else(|| self.history.read().iter().find(|r| r.id == *request_id).cloned())
    }

    pub fn pending_requests(&self) -> Vec<ApprovalRequest> {
        self.pending.read().values().cloned().collect()
    }

    pub fn has_pending_approval(&self, capability_id: &CapabilityId) -> bool {
        self.pending
            .read()
            .values()
            .any(|r| r.capability_id == *capability_id && r.is_pending())
    }
}

impl Default for ApprovalManager {
    fn default() -> Self {
        Self::new()
    }
}

// ────────────────────────────────────────────────────────────────────────────
// AuditAction / AuditEntry
// ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AuditAction {
    Execute,
    Register,
    Unregister,
    Enable,
    Disable,
    Revoke,
    PermissionChange,
    ApprovalRequest,
    ApprovalDecision,
}

impl fmt::Display for AuditAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuditAction::Execute => write!(f, "execute"),
            AuditAction::Register => write!(f, "register"),
            AuditAction::Unregister => write!(f, "unregister"),
            AuditAction::Enable => write!(f, "enable"),
            AuditAction::Disable => write!(f, "disable"),
            AuditAction::Revoke => write!(f, "revoke"),
            AuditAction::PermissionChange => write!(f, "permission_change"),
            AuditAction::ApprovalRequest => write!(f, "approval_request"),
            AuditAction::ApprovalDecision => write!(f, "approval_decision"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: Uuid,
    pub capability_id: CapabilityId,
    pub action: AuditAction,
    pub actor: String,
    pub timestamp: DateTime<Utc>,
    pub details: HashMap<String, serde_json::Value>,
    pub success: bool,
}

// ────────────────────────────────────────────────────────────────────────────
// AuditLog
// ────────────────────────────────────────────────────────────────────────────

pub struct AuditLog {
    pub entries: RwLock<Vec<AuditEntry>>,
}

impl AuditLog {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(Vec::new()),
        }
    }

    pub fn record(
        &self,
        capability_id: CapabilityId,
        action: AuditAction,
        actor: impl Into<String>,
        details: HashMap<String, serde_json::Value>,
        success: bool,
    ) -> Uuid {
        let entry = AuditEntry {
            id: Uuid::new_v4(),
            capability_id,
            action,
            actor: actor.into(),
            timestamp: Utc::now(),
            details,
            success,
        };
        let entry_id = entry.id;
        self.entries.write().push(entry);
        entry_id
    }

    pub fn query_by_capability(&self, capability_id: &CapabilityId) -> Vec<AuditEntry> {
        self.entries
            .read()
            .iter()
            .filter(|e| e.capability_id == *capability_id)
            .cloned()
            .collect()
    }

    pub fn query_by_actor(&self, actor: &str) -> Vec<AuditEntry> {
        self.entries
            .read()
            .iter()
            .filter(|e| e.actor == actor)
            .cloned()
            .collect()
    }

    pub fn query_by_action(&self, action: &AuditAction) -> Vec<AuditEntry> {
        self.entries
            .read()
            .iter()
            .filter(|e| e.action == *action)
            .cloned()
            .collect()
    }

    pub fn query_by_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<AuditEntry> {
        self.entries
            .read()
            .iter()
            .filter(|e| e.timestamp >= start && e.timestamp <= end)
            .cloned()
            .collect()
    }

    pub fn recent(&self, n: usize) -> Vec<AuditEntry> {
        let entries = self.entries.read();
        let len = entries.len();
        if n >= len {
            entries.clone()
        } else {
            entries[len - n..].to_vec()
        }
    }

    pub fn total_entries(&self) -> usize {
        self.entries.read().len()
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}

// ────────────────────────────────────────────────────────────────────────────
// CapabilityPermissionManager
// ────────────────────────────────────────────────────────────────────────────

pub struct CapabilityPermissionManager {
    pub roles: RwLock<HashMap<String, RolePermissions>>,
    pub allow_deny: AllowDenyList,
    pub sandbox: Sandbox,
    pub approval_manager: ApprovalManager,
    pub audit_log: AuditLog,
}

impl CapabilityPermissionManager {
    pub fn new(allow_deny: AllowDenyList, sandbox_config: SandboxConfig) -> Self {
        Self {
            roles: RwLock::new(HashMap::new()),
            allow_deny,
            sandbox: Sandbox::new(sandbox_config),
            approval_manager: ApprovalManager::new(),
            audit_log: AuditLog::new(),
        }
    }

    pub fn register_role(&self, name: impl Into<String>, role_permissions: RolePermissions) {
        self.roles.write().insert(name.into(), role_permissions);
    }

    pub fn check_permission(
        &self,
        role: &str,
        capability_id: &CapabilityId,
    ) -> CapabilityResult<bool> {
        let roles = self.roles.read();
        let role_perms = roles
            .get(role)
            .ok_or_else(|| CapabilityError::permission_denied(format!("unknown role '{}'", role)))?;
        Ok(role_perms.can_access(capability_id))
    }

    pub fn require_permission(
        &self,
        role: &str,
        capability_id: &CapabilityId,
    ) -> CapabilityResult<()> {
        if !self.check_permission(role, capability_id)? {
            return Err(CapabilityError::permission_denied(format!(
                "role '{}' cannot access capability {}",
                role, capability_id
            )));
        }
        Ok(())
    }

    pub fn enforce_sandbox(
        &self,
        _capability_id: &CapabilityId,
        resource: &str,
        amount: u64,
    ) -> CapabilityResult<()> {
        self.sandbox.check_and_consume(resource, amount)
    }

    pub fn request_approval(
        &self,
        capability_id: CapabilityId,
        requester: impl Into<String>,
        reason: impl Into<String>,
    ) -> Uuid {
        let request_id = self
            .approval_manager
            .request_approval(capability_id, requester, reason);
        let mut details = HashMap::new();
        details.insert(
            "request_id".into(),
            serde_json::Value::String(request_id.to_string()),
        );
        self.audit_log.record(
            capability_id,
            AuditAction::ApprovalRequest,
            "system",
            details,
            true,
        );
        request_id
    }

    pub fn approve_request(
        &self,
        request_id: Uuid,
        approver: impl Into<String>,
    ) -> CapabilityResult<()> {
        let approver_str = approver.into();
        self.approval_manager.approve(request_id, &approver_str)?;
        if let Some(request) = self.approval_manager.get_request(&request_id) {
            let mut details = HashMap::new();
            details.insert(
                "request_id".into(),
                serde_json::Value::String(request_id.to_string()),
            );
            details.insert(
                "decision".into(),
                serde_json::Value::String("approved".into()),
            );
            details.insert(
                "capability_id".into(),
                serde_json::Value::String(request.capability_id.to_string()),
            );
            self.audit_log.record(
                request.capability_id,
                AuditAction::ApprovalDecision,
                &approver_str,
                details,
                true,
            );
        }
        Ok(())
    }

    pub fn deny_request(
        &self,
        request_id: Uuid,
        approver: impl Into<String>,
    ) -> CapabilityResult<()> {
        let approver_str = approver.into();
        self.approval_manager.deny(request_id, &approver_str)?;
        if let Some(request) = self.approval_manager.get_request(&request_id) {
            let mut details = HashMap::new();
            details.insert(
                "request_id".into(),
                serde_json::Value::String(request_id.to_string()),
            );
            details.insert(
                "decision".into(),
                serde_json::Value::String("denied".into()),
            );
            details.insert(
                "capability_id".into(),
                serde_json::Value::String(request.capability_id.to_string()),
            );
            self.audit_log.record(
                request.capability_id,
                AuditAction::ApprovalDecision,
                &approver_str,
                details,
                true,
            );
        }
        Ok(())
    }

    pub fn log_action(
        &self,
        capability_id: CapabilityId,
        action: AuditAction,
        actor: impl Into<String>,
        details: HashMap<String, serde_json::Value>,
        success: bool,
    ) -> Uuid {
        self.audit_log
            .record(capability_id, action, actor, details, success)
    }

    pub fn get_audit_log(&self) -> &AuditLog {
        &self.audit_log
    }

    pub fn can_execute(
        &self,
        role: &str,
        capability_id: &CapabilityId,
    ) -> CapabilityResult<bool> {
        let roles = self.roles.read();
        let role_perms = roles
            .get(role)
            .ok_or_else(|| CapabilityError::permission_denied(format!("unknown role '{}'", role)))?;

        if !role_perms.can_access(capability_id) {
            self.audit_log.record(
                *capability_id,
                AuditAction::Execute,
                role,
                HashMap::new(),
                false,
            );
            return Ok(false);
        }

        let cap_id_str = capability_id.to_string();
        if !self.allow_deny.is_allowed(&cap_id_str) {
            self.audit_log.record(
                *capability_id,
                AuditAction::Execute,
                role,
                HashMap::new(),
                false,
            );
            return Ok(false);
        }

        if self.approval_manager.has_pending_approval(capability_id) {
            self.audit_log.record(
                *capability_id,
                AuditAction::Execute,
                role,
                HashMap::new(),
                false,
            );
            return Ok(false);
        }

        let mut details = HashMap::new();
        details.insert(
            "role".into(),
            serde_json::Value::String(role.to_string()),
        );
        self.audit_log.record(
            *capability_id,
            AuditAction::Execute,
            role,
            details,
            true,
        );
        Ok(true)
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_capability_id() -> CapabilityId {
        CapabilityId::new()
    }

    // ── Role ──────────────────────────────────────────────────────────────

    #[test]
    fn role_display() {
        assert_eq!(Role::Admin.to_string(), "admin");
        assert_eq!(Role::Operator.to_string(), "operator");
        assert_eq!(Role::Viewer.to_string(), "viewer");
        assert_eq!(Role::Guest.to_string(), "guest");
        assert_eq!(
            Role::Custom("analyst".into()).to_string(),
            "custom:analyst"
        );
    }

    #[test]
    fn role_default_permissions_admin_has_all() {
        let admin = Role::Admin;
        let perms = admin.default_permissions();
        assert!(perms.contains(&"execute".into()));
        assert!(perms.contains(&"admin".into()));
        assert!(perms.contains(&"approve".into()));
        assert!(perms.contains(&"revoke".into()));
        assert!(perms.len() >= 10);
    }

    #[test]
    fn role_default_permissions_operator() {
        let perms = Role::Operator.default_permissions();
        assert!(perms.contains(&"execute".into()));
        assert!(perms.contains(&"read".into()));
        assert!(perms.contains(&"write".into()));
        assert!(!perms.contains(&"admin".into()));
        assert!(!perms.contains(&"approve".into()));
    }

    #[test]
    fn role_default_permissions_viewer() {
        let perms = Role::Viewer.default_permissions();
        assert_eq!(perms, vec!["read".to_string()]);
    }

    #[test]
    fn role_default_permissions_guest() {
        let perms = Role::Guest.default_permissions();
        assert_eq!(perms, vec!["read".to_string()]);
    }

    #[test]
    fn role_default_permissions_custom_empty() {
        let perms = Role::Custom("anything".into()).default_permissions();
        assert!(perms.is_empty());
    }

    #[test]
    fn role_equality() {
        assert_eq!(Role::Admin, Role::Admin);
        assert_ne!(Role::Admin, Role::Operator);
        assert_ne!(
            Role::Custom("a".into()),
            Role::Custom("b".into())
        );
    }

    #[test]
    fn role_in_hashset() {
        let mut set = HashSet::new();
        set.insert(Role::Admin);
        set.insert(Role::Admin);
        assert_eq!(set.len(), 1);
        set.insert(Role::Viewer);
        assert_eq!(set.len(), 2);
    }

    // ── RolePermissions ───────────────────────────────────────────────────

    #[test]
    fn role_permissions_new() {
        let rp = RolePermissions::new(Role::Operator, vec!["read".into(), "write".into()]);
        assert_eq!(rp.role, Role::Operator);
        assert!(rp.permissions.contains("read"));
        assert!(rp.permissions.contains("write"));
        assert!(rp.allowed_capabilities.is_empty());
        assert!(rp.denied_capabilities.is_empty());
    }

    #[test]
    fn role_permissions_can_access_no_restrictions() {
        let rp = RolePermissions::new(Role::Admin, vec![]);
        let cid = test_capability_id();
        assert!(rp.can_access(&cid));
    }

    #[test]
    fn role_permissions_grant_and_revoke() {
        let cid = test_capability_id();
        let mut rp = RolePermissions::new(Role::Operator, vec![]);

        rp.grant_access(cid);
        assert!(rp.can_access(&cid));

        rp.revoke_access(cid);
        assert!(!rp.can_access(&cid));
    }

    #[test]
    fn role_permissions_deny_overrides_allow() {
        let cid = test_capability_id();
        let mut rp = RolePermissions::new(Role::Operator, vec![]);

        rp.grant_access(cid);
        assert!(rp.can_access(&cid));

        rp.revoke_access(cid);
        assert!(!rp.can_access(&cid));

        rp.grant_access(cid);
        assert!(rp.can_access(&cid));
    }

    #[test]
    fn role_permissions_has_permission() {
        let rp = RolePermissions::new(Role::Viewer, vec!["read".into()]);
        assert!(rp.has_permission("read"));
        assert!(!rp.has_permission("write"));
        assert!(!rp.has_permission("admin"));
    }

    #[test]
    fn role_permissions_admin_permission_bypasses() {
        let rp = RolePermissions::new(Role::Admin, vec!["admin".into()]);
        assert!(rp.has_permission("read"));
        assert!(rp.has_permission("execute"));
        assert!(rp.has_permission("anything_at_all"));
    }

    #[test]
    fn role_permissions_multiple_capabilities() {
        let cid1 = test_capability_id();
        let cid2 = test_capability_id();
        let cid3 = test_capability_id();
        let mut rp = RolePermissions::new(Role::Operator, vec![]);

        rp.grant_access(cid1);
        rp.grant_access(cid2);
        rp.revoke_access(cid3);

        assert!(rp.can_access(&cid1));
        assert!(rp.can_access(&cid2));
        assert!(!rp.can_access(&cid3));
    }

    // ── AllowDenyList ─────────────────────────────────────────────────────

    #[test]
    fn allow_deny_list_empty_allows_all() {
        let list = AllowDenyList::new();
        assert!(list.is_allowed("anything"));
        assert!(list.is_allowed("some.capability"));
    }

    #[test]
    fn allow_deny_list_allow_only() {
        let mut list = AllowDenyList::new();
        list.allow("cap_a");
        list.allow("cap_b");

        assert!(list.is_allowed("cap_a"));
        assert!(list.is_allowed("cap_b"));
        assert!(!list.is_allowed("cap_c"));
    }

    #[test]
    fn allow_deny_list_deny_overrides_allow() {
        let mut list = AllowDenyList::new();
        list.allow("cap_a");
        list.deny("cap_a");

        assert!(!list.is_allowed("cap_a"));
    }

    #[test]
    fn allow_deny_list_deny_only() {
        let mut list = AllowDenyList::new();
        list.deny("blocked");

        assert!(!list.is_allowed("blocked"));
        assert!(list.is_allowed("allowed"));
    }

    #[test]
    fn allow_deny_list_mixed() {
        let mut list = AllowDenyList::new();
        list.allow("alpha");
        list.allow("beta");
        list.deny("beta");

        assert!(list.is_allowed("alpha"));
        assert!(!list.is_allowed("beta"));
        assert!(!list.is_allowed("gamma"));
    }

    #[test]
    fn allow_deny_list_from_config() {
        let config = serde_json::json!({
            "allow": ["cap_a", "cap_b"],
            "deny": ["cap_b"]
        });
        let list = AllowDenyList::from_config(&config);

        assert!(list.is_allowed("cap_a"));
        assert!(!list.is_allowed("cap_b"));
        assert!(!list.is_allowed("cap_c"));
    }

    #[test]
    fn allow_deny_list_from_config_empty() {
        let config = serde_json::json!({});
        let list = AllowDenyList::from_config(&config);
        assert!(list.is_allowed("anything"));
    }

    #[test]
    fn allow_deny_list_from_config_deny_only() {
        let config = serde_json::json!({
            "deny": ["evil"]
        });
        let list = AllowDenyList::from_config(&config);
        assert!(!list.is_allowed("evil"));
        assert!(list.is_allowed("good"));
    }

    #[test]
    fn allow_deny_list_default_is_empty() {
        let list = AllowDenyList::default();
        assert!(list.allow_list.is_empty());
        assert!(list.deny_list.is_empty());
        assert!(list.is_allowed("anything"));
    }

    // ── SandboxConfig ─────────────────────────────────────────────────────

    #[test]
    fn sandbox_config_new_defaults() {
        let cfg = SandboxConfig::new();
        assert!(cfg.enabled);
        assert_eq!(cfg.max_memory_bytes, 256 * 1024 * 1024);
        assert_eq!(cfg.max_cpu_time_ms, 30_000);
        assert_eq!(cfg.max_inference_tokens, 4096);
        assert!(!cfg.network_access);
        assert!(!cfg.filesystem_access);
    }

    #[test]
    fn sandbox_config_default_restricted() {
        let cfg = SandboxConfig::default_restricted();
        assert!(cfg.enabled);
        assert_eq!(cfg.max_memory_bytes, 64 * 1024 * 1024);
        assert_eq!(cfg.max_cpu_time_ms, 5_000);
        assert_eq!(cfg.max_inference_tokens, 1024);
        assert!(!cfg.network_access);
    }

    #[test]
    fn sandbox_config_default_permissive() {
        let cfg = SandboxConfig::default_permissive();
        assert!(!cfg.enabled);
        assert_eq!(cfg.max_memory_bytes, u64::MAX);
        assert_eq!(cfg.max_cpu_time_ms, u64::MAX);
        assert_eq!(cfg.max_inference_tokens, u32::MAX);
        assert!(cfg.network_access);
        assert!(cfg.filesystem_access);
        assert!(!cfg.allowed_namespaces.is_empty());
    }

    #[test]
    fn sandbox_config_validate_resource_within_limits() {
        let cfg = SandboxConfig::new();
        assert!(cfg.validate_resource("memory_bytes", 1024));
        assert!(cfg.validate_resource("cpu_time_ms", 100));
        assert!(cfg.validate_resource("inference_tokens", 500));
    }

    #[test]
    fn sandbox_config_validate_resource_exceeds_limits() {
        let cfg = SandboxConfig::new();
        assert!(!cfg.validate_resource("memory_bytes", 256 * 1024 * 1024 + 1));
        assert!(!cfg.validate_resource("cpu_time_ms", 30_001));
        assert!(!cfg.validate_resource("inference_tokens", 4097));
    }

    #[test]
    fn sandbox_config_validate_resource_unknown_always_ok() {
        let cfg = SandboxConfig::new();
        assert!(cfg.validate_resource("disk_bytes", u64::MAX));
    }

    #[test]
    fn sandbox_config_validate_resource_disabled_allows_all() {
        let mut cfg = SandboxConfig::new();
        cfg.enabled = false;
        assert!(cfg.validate_resource("memory_bytes", u64::MAX));
        assert!(cfg.validate_resource("cpu_time_ms", u64::MAX));
    }

    // ── Sandbox ───────────────────────────────────────────────────────────

    #[test]
    fn sandbox_new() {
        let sandbox = Sandbox::new(SandboxConfig::new());
        let usage = sandbox.get_usage();
        assert_eq!(usage.cpu_time_ms, 0);
        assert_eq!(usage.memory_bytes, 0);
        assert_eq!(usage.inference_tokens, 0);
    }

    #[test]
    fn sandbox_check_and_consume_within_limits() {
        let sandbox = Sandbox::new(SandboxConfig::new());
        assert!(sandbox.check_and_consume("cpu_time_ms", 1000).is_ok());
        assert!(sandbox.check_and_consume("memory_bytes", 1024).is_ok());
        assert!(sandbox.check_and_consume("inference_tokens", 100).is_ok());

        let usage = sandbox.get_usage();
        assert_eq!(usage.cpu_time_ms, 1000);
        assert_eq!(usage.memory_bytes, 1024);
        assert_eq!(usage.inference_tokens, 100);
    }

    #[test]
    fn sandbox_check_and_consume_exceeds_limit() {
        let cfg = SandboxConfig {
            max_cpu_time_ms: 100,
            ..SandboxConfig::new()
        };
        let sandbox = Sandbox::new(cfg);

        assert!(sandbox.check_and_consume("cpu_time_ms", 80).is_ok());
        let err = sandbox.check_and_consume("cpu_time_ms", 30);
        assert!(err.is_err());
        let usage = sandbox.get_usage();
        assert_eq!(usage.cpu_time_ms, 80);
    }

    #[test]
    fn sandbox_check_and_consume_memory_limit() {
        let cfg = SandboxConfig {
            max_memory_bytes: 1024,
            ..SandboxConfig::new()
        };
        let sandbox = Sandbox::new(cfg);

        assert!(sandbox.check_and_consume("memory_bytes", 512).is_ok());
        assert!(sandbox.check_and_consume("memory_bytes", 513).is_err());
    }

    #[test]
    fn sandbox_check_and_consume_inference_limit() {
        let cfg = SandboxConfig {
            max_inference_tokens: 500,
            ..SandboxConfig::new()
        };
        let sandbox = Sandbox::new(cfg);

        assert!(sandbox.check_and_consume("inference_tokens", 499).is_ok());
        assert!(sandbox.check_and_consume("inference_tokens", 2).is_err());
    }

    #[test]
    fn sandbox_check_and_consume_disabled_allows_all() {
        let cfg = SandboxConfig {
            enabled: false,
            ..SandboxConfig::new()
        };
        let sandbox = Sandbox::new(cfg);

        assert!(sandbox.check_and_consume("cpu_time_ms", u64::MAX).is_ok());
        assert!(sandbox.check_and_consume("memory_bytes", u64::MAX).is_ok());
        assert!(sandbox
            .check_and_consume("inference_tokens", u64::MAX)
            .is_ok());
    }

    #[test]
    fn sandbox_check_and_consume_unknown_resource() {
        let sandbox = Sandbox::new(SandboxConfig::new());
        assert!(sandbox.check_and_consume("unknown_resource", 999).is_ok());
    }

    #[test]
    fn sandbox_reset_usage() {
        let sandbox = Sandbox::new(SandboxConfig::new());
        sandbox.check_and_consume("cpu_time_ms", 500).unwrap();
        sandbox.check_and_consume("memory_bytes", 256).unwrap();

        sandbox.reset_usage();
        let usage = sandbox.get_usage();
        assert_eq!(usage.cpu_time_ms, 0);
        assert_eq!(usage.memory_bytes, 0);
    }

    #[test]
    fn sandbox_cumulative_consumption() {
        let cfg = SandboxConfig {
            max_cpu_time_ms: 100,
            ..SandboxConfig::new()
        };
        let sandbox = Sandbox::new(cfg);

        sandbox.check_and_consume("cpu_time_ms", 30).unwrap();
        sandbox.check_and_consume("cpu_time_ms", 30).unwrap();
        sandbox.check_and_consume("cpu_time_ms", 30).unwrap();
        assert!(sandbox.check_and_consume("cpu_time_ms", 30).is_err());

        let usage = sandbox.get_usage();
        assert_eq!(usage.cpu_time_ms, 90);
    }

    // ── ApprovalRequest ───────────────────────────────────────────────────

    #[test]
    fn approval_request_new() {
        let cid = test_capability_id();
        let req = ApprovalRequest::new(cid, "alice", "need access for testing");
        assert_eq!(req.capability_id, cid);
        assert_eq!(req.requester, "alice");
        assert_eq!(req.reason, "need access for testing");
        assert_eq!(req.status, ApprovalStatus::Pending);
        assert!(req.approved_at.is_none());
        assert!(req.approver.is_none());
        assert!(req.is_pending());
    }

    #[test]
    fn approval_request_approve() {
        let cid = test_capability_id();
        let mut req = ApprovalRequest::new(cid, "alice", "reason");
        req.approve("admin_user");

        assert_eq!(req.status, ApprovalStatus::Approved);
        assert!(req.approved_at.is_some());
        assert_eq!(req.approver.as_deref(), Some("admin_user"));
        assert!(!req.is_pending());
    }

    #[test]
    fn approval_request_deny() {
        let cid = test_capability_id();
        let mut req = ApprovalRequest::new(cid, "alice", "reason");
        req.deny("admin_user");

        assert_eq!(req.status, ApprovalStatus::Denied);
        assert!(req.approved_at.is_some());
        assert_eq!(req.approver.as_deref(), Some("admin_user"));
        assert!(!req.is_pending());
    }

    #[test]
    fn approval_status_display() {
        assert_eq!(ApprovalStatus::Pending.to_string(), "pending");
        assert_eq!(ApprovalStatus::Approved.to_string(), "approved");
        assert_eq!(ApprovalStatus::Denied.to_string(), "denied");
        assert_eq!(ApprovalStatus::Expired.to_string(), "expired");
    }

    // ── ApprovalManager ───────────────────────────────────────────────────

    #[test]
    fn approval_manager_new() {
        let mgr = ApprovalManager::new();
        assert!(mgr.pending_requests().is_empty());
    }

    #[test]
    fn approval_manager_request_approve() {
        let mgr = ApprovalManager::new();
        let cid = test_capability_id();
        let req_id = mgr.request_approval(cid, "alice", "need it");

        assert!(mgr.has_pending_approval(&cid));
        assert_eq!(mgr.pending_requests().len(), 1);

        mgr.approve(req_id, "admin").unwrap();

        assert!(!mgr.has_pending_approval(&cid));
        assert!(mgr.pending_requests().is_empty());

        let req = mgr.get_request(&req_id).unwrap();
        assert_eq!(req.status, ApprovalStatus::Approved);
        assert_eq!(req.approver.as_deref(), Some("admin"));
    }

    #[test]
    fn approval_manager_request_deny() {
        let mgr = ApprovalManager::new();
        let cid = test_capability_id();
        let req_id = mgr.request_approval(cid, "bob", "access needed");

        mgr.deny(req_id, "admin").unwrap();

        assert!(!mgr.has_pending_approval(&cid));

        let req = mgr.get_request(&req_id).unwrap();
        assert_eq!(req.status, ApprovalStatus::Denied);
    }

    #[test]
    fn approval_manager_nonexistent_request() {
        let mgr = ApprovalManager::new();
        let fake_id = Uuid::new_v4();
        assert!(mgr.approve(fake_id, "admin").is_err());
        assert!(mgr.deny(fake_id, "admin").is_err());
    }

    #[test]
    fn approval_manager_get_request_from_history() {
        let mgr = ApprovalManager::new();
        let cid = test_capability_id();
        let req_id = mgr.request_approval(cid, "alice", "reason");
        mgr.approve(req_id, "admin").unwrap();

        let req = mgr.get_request(&req_id).unwrap();
        assert_eq!(req.status, ApprovalStatus::Approved);
    }

    #[test]
    fn approval_manager_get_request_not_found() {
        let mgr = ApprovalManager::new();
        assert!(mgr.get_request(&Uuid::new_v4()).is_none());
    }

    #[test]
    fn approval_manager_has_pending_false_after_decision() {
        let mgr = ApprovalManager::new();
        let cid = test_capability_id();
        let req_id = mgr.request_approval(cid, "alice", "r");

        assert!(mgr.has_pending_approval(&cid));
        mgr.deny(req_id, "admin").unwrap();
        assert!(!mgr.has_pending_approval(&cid));
    }

    #[test]
    fn approval_manager_multiple_requests_different_capabilities() {
        let mgr = ApprovalManager::new();
        let cid1 = test_capability_id();
        let cid2 = test_capability_id();

        let id1 = mgr.request_approval(cid1, "alice", "r1");
        let id2 = mgr.request_approval(cid2, "bob", "r2");

        assert!(mgr.has_pending_approval(&cid1));
        assert!(mgr.has_pending_approval(&cid2));
        assert_eq!(mgr.pending_requests().len(), 2);

        mgr.approve(id1, "admin").unwrap();
        assert!(!mgr.has_pending_approval(&cid1));
        assert!(mgr.has_pending_approval(&cid2));
    }

    #[test]
    fn approval_manager_default() {
        let mgr = ApprovalManager::default();
        assert!(mgr.pending_requests().is_empty());
    }

    // ── AuditAction ───────────────────────────────────────────────────────

    #[test]
    fn audit_action_display() {
        assert_eq!(AuditAction::Execute.to_string(), "execute");
        assert_eq!(AuditAction::Register.to_string(), "register");
        assert_eq!(AuditAction::Unregister.to_string(), "unregister");
        assert_eq!(AuditAction::Enable.to_string(), "enable");
        assert_eq!(AuditAction::Disable.to_string(), "disable");
        assert_eq!(AuditAction::Revoke.to_string(), "revoke");
        assert_eq!(AuditAction::PermissionChange.to_string(), "permission_change");
        assert_eq!(
            AuditAction::ApprovalRequest.to_string(),
            "approval_request"
        );
        assert_eq!(
            AuditAction::ApprovalDecision.to_string(),
            "approval_decision"
        );
    }

    #[test]
    fn audit_action_hashset() {
        let mut set = HashSet::new();
        set.insert(AuditAction::Execute);
        set.insert(AuditAction::Execute);
        assert_eq!(set.len(), 1);
        set.insert(AuditAction::Register);
        assert_eq!(set.len(), 2);
    }

    // ── AuditLog ──────────────────────────────────────────────────────────

    #[test]
    fn audit_log_new() {
        let log = AuditLog::new();
        assert_eq!(log.total_entries(), 0);
    }

    #[test]
    fn audit_log_record() {
        let log = AuditLog::new();
        let cid = test_capability_id();
        let entry_id = log.record(
            cid,
            AuditAction::Execute,
            "alice",
            HashMap::new(),
            true,
        );
        assert_eq!(log.total_entries(), 1);
        let entries = log.query_by_capability(&cid);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, entry_id);
        assert!(entries[0].success);
    }

    #[test]
    fn audit_log_query_by_capability() {
        let log = AuditLog::new();
        let cid1 = test_capability_id();
        let cid2 = test_capability_id();

        log.record(cid1, AuditAction::Execute, "alice", HashMap::new(), true);
        log.record(cid1, AuditAction::Disable, "bob", HashMap::new(), true);
        log.record(cid2, AuditAction::Execute, "alice", HashMap::new(), true);

        let results = log.query_by_capability(&cid1);
        assert_eq!(results.len(), 2);

        let results = log.query_by_capability(&cid2);
        assert_eq!(results.len(), 1);

        let cid3 = test_capability_id();
        let results = log.query_by_capability(&cid3);
        assert!(results.is_empty());
    }

    #[test]
    fn audit_log_query_by_actor() {
        let log = AuditLog::new();
        let cid = test_capability_id();

        log.record(cid, AuditAction::Execute, "alice", HashMap::new(), true);
        log.record(cid, AuditAction::Disable, "bob", HashMap::new(), true);
        log.record(cid, AuditAction::Enable, "alice", HashMap::new(), true);

        let results = log.query_by_actor("alice");
        assert_eq!(results.len(), 2);

        let results = log.query_by_actor("bob");
        assert_eq!(results.len(), 1);

        let results = log.query_by_actor("charlie");
        assert!(results.is_empty());
    }

    #[test]
    fn audit_log_query_by_action() {
        let log = AuditLog::new();
        let cid = test_capability_id();

        log.record(
            cid,
            AuditAction::Execute,
            "alice",
            HashMap::new(),
            true,
        );
        log.record(
            cid,
            AuditAction::Execute,
            "bob",
            HashMap::new(),
            false,
        );
        log.record(
            cid,
            AuditAction::Register,
            "alice",
            HashMap::new(),
            true,
        );

        let results = log.query_by_action(&AuditAction::Execute);
        assert_eq!(results.len(), 2);

        let results = log.query_by_action(&AuditAction::Register);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn audit_log_query_by_time_range() {
        let log = AuditLog::new();
        let cid = test_capability_id();

        let t1 = Utc::now();
        log.record(cid, AuditAction::Execute, "alice", HashMap::new(), true);
        let t2 = Utc::now();
        log.record(cid, AuditAction::Enable, "bob", HashMap::new(), true);
        let t3 = Utc::now();

        let results = log.query_by_time_range(t1, t3);
        assert_eq!(results.len(), 2);

        let results = log.query_by_time_range(t2, t3);
        assert_eq!(results.len(), 1);

        let results = log.query_by_time_range(t3, t3);
        assert!(results.is_empty());
    }

    #[test]
    fn audit_log_recent() {
        let log = AuditLog::new();
        let cid = test_capability_id();

        for i in 0..5 {
            log.record(
                cid,
                AuditAction::Execute,
                format!("actor_{}", i),
                HashMap::new(),
                true,
            );
        }

        let all = log.recent(10);
        assert_eq!(all.len(), 5);

        let last3 = log.recent(3);
        assert_eq!(last3.len(), 3);
        assert_eq!(last3[0].actor, "actor_2");
        assert_eq!(last3[1].actor, "actor_3");
        assert_eq!(last3[2].actor, "actor_4");

        let exact = log.recent(5);
        assert_eq!(exact.len(), 5);
    }

    #[test]
    fn audit_log_recent_empty() {
        let log = AuditLog::new();
        let results = log.recent(10);
        assert!(results.is_empty());
    }

    #[test]
    fn audit_log_total_entries() {
        let log = AuditLog::new();
        let cid = test_capability_id();

        assert_eq!(log.total_entries(), 0);
        log.record(cid, AuditAction::Execute, "a", HashMap::new(), true);
        assert_eq!(log.total_entries(), 1);
        log.record(cid, AuditAction::Enable, "b", HashMap::new(), true);
        assert_eq!(log.total_entries(), 2);
    }

    #[test]
    fn audit_log_record_with_details() {
        let log = AuditLog::new();
        let cid = test_capability_id();
        let mut details = HashMap::new();
        details.insert("key".into(), serde_json::json!("value"));
        details.insert("count".into(), serde_json::json!(42));

        log.record(cid, AuditAction::Execute, "alice", details, true);

        let entries = log.query_by_capability(&cid);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].details["key"], "value");
        assert_eq!(entries[0].details["count"], 42);
    }

    #[test]
    fn audit_log_default() {
        let log = AuditLog::default();
        assert_eq!(log.total_entries(), 0);
    }

    // ── CapabilityPermissionManager ───────────────────────────────────────

    fn make_manager() -> CapabilityPermissionManager {
        let mgr =
            CapabilityPermissionManager::new(AllowDenyList::new(), SandboxConfig::default_permissive());

        let admin_perms = RolePermissions::new(Role::Admin, Role::Admin.default_permissions());
        mgr.register_role("admin", admin_perms);

        let operator_perms =
            RolePermissions::new(Role::Operator, Role::Operator.default_permissions());
        mgr.register_role("operator", operator_perms);

        let viewer_perms = RolePermissions::new(Role::Viewer, Role::Viewer.default_permissions());
        mgr.register_role("viewer", viewer_perms);

        mgr
    }

    #[test]
    fn manager_new() {
        let mgr = CapabilityPermissionManager::new(
            AllowDenyList::new(),
            SandboxConfig::new(),
        );
        assert!(mgr.roles.read().is_empty());
        assert_eq!(mgr.audit_log.total_entries(), 0);
    }

    #[test]
    fn manager_register_role() {
        let mgr = CapabilityPermissionManager::new(
            AllowDenyList::new(),
            SandboxConfig::new(),
        );
        let rp = RolePermissions::new(Role::Admin, vec!["read".into()]);
        mgr.register_role("admin", rp);

        assert!(mgr.roles.read().contains_key("admin"));
    }

    #[test]
    fn manager_check_permission_known_role() {
        let mgr = make_manager();
        let cid = test_capability_id();

        assert!(mgr.check_permission("admin", &cid).unwrap());
    }

    #[test]
    fn manager_check_permission_unknown_role() {
        let mgr = make_manager();
        let cid = test_capability_id();

        let result = mgr.check_permission("nonexistent", &cid);
        assert!(result.is_err());
    }

    #[test]
    fn manager_require_permission_ok() {
        let mgr = make_manager();
        let cid = test_capability_id();
        assert!(mgr.require_permission("admin", &cid).is_ok());
    }

    #[test]
    fn manager_require_permission_denied() {
        let mgr = CapabilityPermissionManager::new(
            AllowDenyList::new(),
            SandboxConfig::default_permissive(),
        );
        let cid = test_capability_id();
        let cid2 = test_capability_id();

        let mut viewer_rp =
            RolePermissions::new(Role::Viewer, Role::Viewer.default_permissions());
        viewer_rp.grant_access(cid);
        mgr.register_role("viewer", viewer_rp);

        assert!(mgr.require_permission("viewer", &cid).is_ok());
        let result = mgr.require_permission("viewer", &cid2);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().code(),
            crate::error::CapabilityErrorCode::PermissionDenied
        );
    }

    #[test]
    fn manager_enforce_sandbox() {
        let mgr = make_manager();
        let cid = test_capability_id();

        assert!(mgr.enforce_sandbox(&cid, "cpu_time_ms", 100).is_ok());
    }

    #[test]
    fn manager_request_approval_creates_pending() {
        let mgr = make_manager();
        let cid = test_capability_id();

        let req_id = mgr.request_approval(cid, "alice", "need access");
        assert!(mgr.approval_manager.has_pending_approval(&cid));
        assert_eq!(mgr.audit_log.total_entries(), 1);
    }

    #[test]
    fn manager_approve_request() {
        let mgr = make_manager();
        let cid = test_capability_id();

        let req_id = mgr.request_approval(cid, "alice", "reason");
        mgr.approve_request(req_id, "admin").unwrap();

        assert!(!mgr.approval_manager.has_pending_approval(&cid));
        let req = mgr.approval_manager.get_request(&req_id).unwrap();
        assert_eq!(req.status, ApprovalStatus::Approved);

        assert_eq!(mgr.audit_log.total_entries(), 2);
    }

    #[test]
    fn manager_deny_request() {
        let mgr = make_manager();
        let cid = test_capability_id();

        let req_id = mgr.request_approval(cid, "bob", "reason");
        mgr.deny_request(req_id, "admin").unwrap();

        assert!(!mgr.approval_manager.has_pending_approval(&cid));
        let req = mgr.approval_manager.get_request(&req_id).unwrap();
        assert_eq!(req.status, ApprovalStatus::Denied);
    }

    #[test]
    fn manager_log_action() {
        let mgr = make_manager();
        let cid = test_capability_id();

        let mut details = HashMap::new();
        details.insert("detail".into(), serde_json::json!("test"));
        let entry_id = mgr.log_action(cid, AuditAction::Enable, "admin", details, true);

        assert_eq!(mgr.audit_log.total_entries(), 1);
        let entry = &mgr.audit_log.entries.read()[0];
        assert_eq!(entry.id, entry_id);
        assert_eq!(entry.action, AuditAction::Enable);
        assert_eq!(entry.actor, "admin");
        assert!(entry.success);
    }

    #[test]
    fn manager_get_audit_log() {
        let mgr = make_manager();
        let log = mgr.get_audit_log();
        assert_eq!(log.total_entries(), 0);

        let cid = test_capability_id();
        mgr.log_action(cid, AuditAction::Execute, "user", HashMap::new(), true);
        assert_eq!(log.total_entries(), 1);
    }

    #[test]
    fn manager_can_execute_admin() {
        let mgr = make_manager();
        let cid = test_capability_id();
        assert!(mgr.can_execute("admin", &cid).unwrap());
    }

    #[test]
    fn manager_can_execute_unknown_role() {
        let mgr = make_manager();
        let cid = test_capability_id();
        let result = mgr.can_execute("ghost", &cid);
        assert!(result.is_err());
    }

    #[test]
    fn manager_can_execute_denied_capability() {
        let cid = CapabilityId(Uuid::new_v4());
        let cid_str = cid.to_string();

        let mut ad = AllowDenyList::new();
        ad.deny(&cid_str);
        let mgr = CapabilityPermissionManager::new(ad, SandboxConfig::default_permissive());

        let mut rp = RolePermissions::new(Role::Admin, Role::Admin.default_permissions());
        rp.grant_access(cid);
        mgr.register_role("admin", rp);

        assert!(!mgr.can_execute("admin", &cid).unwrap());
        assert_eq!(mgr.audit_log.total_entries(), 1);
    }

    #[test]
    fn manager_can_execute_pending_approval_blocks() {
        let mgr = make_manager();
        let cid = test_capability_id();

        mgr.request_approval(cid, "alice", "need it");
        assert!(!mgr.can_execute("admin", &cid).unwrap());

        let entries = mgr.audit_log.query_by_action(&AuditAction::Execute);
        assert_eq!(entries.len(), 1);
        assert!(!entries[0].success);
    }

    #[test]
    fn manager_can_execute_after_approval() {
        let mgr = make_manager();
        let cid = test_capability_id();

        let req_id = mgr.request_approval(cid, "alice", "need it");
        mgr.approve_request(req_id, "admin").unwrap();

        assert!(mgr.can_execute("admin", &cid).unwrap());
    }

    #[test]
    fn manager_can_execute_audits_success() {
        let mgr = make_manager();
        let cid = test_capability_id();

        mgr.can_execute("admin", &cid).unwrap();

        let entries = mgr.audit_log.query_by_action(&AuditAction::Execute);
        assert_eq!(entries.len(), 1);
        assert!(entries[0].success);
    }

    #[test]
    fn manager_can_execute_viewer_restricted() {
        let cid_allowed = CapabilityId(Uuid::new_v4());
        let cid_denied = CapabilityId(Uuid::new_v4());

        let mut allow_deny = AllowDenyList::new();
        allow_deny.allow(&cid_allowed.to_string());
        allow_deny.deny(&cid_denied.to_string());

        let mgr =
            CapabilityPermissionManager::new(allow_deny, SandboxConfig::default_permissive());

        let mut viewer_rp =
            RolePermissions::new(Role::Viewer, Role::Viewer.default_permissions());
        viewer_rp.grant_access(cid_allowed);
        viewer_rp.grant_access(cid_denied);
        mgr.register_role("viewer", viewer_rp);

        assert!(mgr.can_execute("viewer", &cid_allowed).unwrap());
        assert!(!mgr.can_execute("viewer", &cid_denied).unwrap());
    }

    #[test]
    fn manager_sandbox_enforce_on_manager() {
        let cfg = SandboxConfig {
            enabled: true,
            max_memory_bytes: 256,
            ..SandboxConfig::new()
        };
        let mgr = CapabilityPermissionManager::new(AllowDenyList::new(), cfg);
        let cid = test_capability_id();

        assert!(mgr.enforce_sandbox(&cid, "memory_bytes", 200).is_ok());
        assert!(mgr.enforce_sandbox(&cid, "memory_bytes", 100).is_err());
    }

    // ── Integration tests ─────────────────────────────────────────────────

    #[test]
    fn full_workflow_register_approve_execute_audit() {
        let cfg = SandboxConfig::default_permissive();

        let mgr = CapabilityPermissionManager::new(AllowDenyList::new(), cfg);

        let mut admin_rp =
            RolePermissions::new(Role::Admin, Role::Admin.default_permissions());
        let mut operator_rp =
            RolePermissions::new(Role::Operator, Role::Operator.default_permissions());

        let cid = CapabilityId::new();
        admin_rp.grant_access(cid);
        operator_rp.grant_access(cid);

        mgr.register_role("admin", admin_rp);
        mgr.register_role("operator", operator_rp);

        assert!(mgr.can_execute("admin", &cid).unwrap());

        assert!(mgr.can_execute("operator", &cid).unwrap());

        let req_id = mgr.request_approval(cid, "operator", "need elevated access");
        assert!(!mgr.can_execute("operator", &cid).unwrap());

        mgr.approve_request(req_id, "admin").unwrap();
        assert!(mgr.can_execute("operator", &cid).unwrap());

        let log = mgr.get_audit_log();
        let entries = log.query_by_capability(&cid);
        assert!(entries.len() >= 3);
    }

    #[test]
    fn multiple_roles_different_access() {
        let cfg = SandboxConfig::default_permissive();
        let mgr = CapabilityPermissionManager::new(AllowDenyList::new(), cfg);

        let cid1 = test_capability_id();
        let cid2 = test_capability_id();

        let mut admin_rp =
            RolePermissions::new(Role::Admin, Role::Admin.default_permissions());
        admin_rp.grant_access(cid1);
        admin_rp.grant_access(cid2);

        let mut limited_rp =
            RolePermissions::new(Role::Custom("limited".into()), vec!["read".into()]);
        limited_rp.grant_access(cid1);

        mgr.register_role("admin", admin_rp);
        mgr.register_role("limited", limited_rp);

        assert!(mgr.can_execute("admin", &cid1).unwrap());
        assert!(mgr.can_execute("admin", &cid2).unwrap());

        assert!(mgr.can_execute("limited", &cid1).unwrap());
        assert!(!mgr.can_execute("limited", &cid2).unwrap());
    }

    #[test]
    fn allow_deny_list_integration_with_manager() {
        let cap_good = CapabilityId(Uuid::new_v4());
        let cap_bad = CapabilityId(Uuid::new_v4());

        let mut allow_deny = AllowDenyList::new();
        allow_deny.allow(cap_good.to_string());
        allow_deny.deny(cap_bad.to_string());

        let mgr =
            CapabilityPermissionManager::new(allow_deny, SandboxConfig::default_permissive());

        let mut admin_rp =
            RolePermissions::new(Role::Admin, Role::Admin.default_permissions());

        admin_rp.grant_access(cap_good);
        admin_rp.grant_access(cap_bad);
        mgr.register_role("admin", admin_rp);

        assert!(mgr.can_execute("admin", &cap_good).unwrap());
        assert!(!mgr.can_execute("admin", &cap_bad).unwrap());
    }

    #[test]
    fn audit_log_full_query_coverage() {
        let log = AuditLog::new();
        let cid1 = test_capability_id();
        let cid2 = test_capability_id();

        let t0 = Utc::now();
        log.record(
            cid1,
            AuditAction::Register,
            "alice",
            HashMap::new(),
            true,
        );
        log.record(
            cid1,
            AuditAction::Enable,
            "alice",
            HashMap::new(),
            true,
        );
        log.record(
            cid2,
            AuditAction::Execute,
            "bob",
            HashMap::new(),
            false,
        );
        log.record(
            cid2,
            AuditAction::Disable,
            "alice",
            HashMap::new(),
            true,
        );
        let t1 = Utc::now();

        assert_eq!(log.total_entries(), 4);

        assert_eq!(log.query_by_capability(&cid1).len(), 2);
        assert_eq!(log.query_by_capability(&cid2).len(), 2);

        assert_eq!(log.query_by_actor("alice").len(), 3);
        assert_eq!(log.query_by_actor("bob").len(), 1);

        assert_eq!(log.query_by_action(&AuditAction::Execute).len(), 1);
        assert_eq!(log.query_by_action(&AuditAction::Register).len(), 1);

        assert_eq!(log.query_by_time_range(t0, t1).len(), 4);
        assert_eq!(log.recent(2).len(), 2);
    }
}
