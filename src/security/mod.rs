/// Security module for the Neo AGI Operating System
///
/// Provides authentication, authorization, credential management,
/// policy enforcement, audit logging, and encryption at rest.

pub mod auth;
pub mod audit;
pub mod credentials;
pub mod encryption;
pub mod permissions;
pub mod policy;

pub use auth::{AuthResult, AuthToken, Authenticator};
pub use audit::{AuditEvent, AuditLogger, AuditOutcome};
pub use credentials::{Credential, CredentialStore, CredentialType, InMemoryCredentialStore};
pub use encryption::{EncryptionAlgorithm, EncryptionKey, EncryptionManager};
pub use permissions::{Permission, PermissionManager, PermissionSet};
pub use policy::{Policy, PolicyEffect, PolicyEngine, PolicyRule};

use serde::{Deserialize, Serialize};
use std::fmt;
use tokio::sync::RwLock;

/// Global security configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Whether authentication is enforced.
    pub auth_enabled: bool,
    /// Whether encryption at rest is enabled.
    pub encryption_enabled: bool,
    /// Whether audit logging is enabled.
    pub audit_enabled: bool,
    /// Path to the policy file.
    pub policy_file: Option<String>,
    /// Path to the credential store.
    pub credential_store_path: Option<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            auth_enabled: true,
            encryption_enabled: false,
            audit_enabled: true,
            policy_file: None,
            credential_store_path: None,
        }
    }
}

/// Central security manager that ties together all security subsystems.
pub struct SecurityManager {
    config: SecurityConfig,
    permission_manager: PermissionManager,
    policy_engine: PolicyEngine,
    audit_logger: AuditLogger,
    credential_store: RwLock<InMemoryCredentialStore>,
    encryption_manager: EncryptionManager,
}

impl SecurityManager {
    /// Create a new SecurityManager with the given configuration.
    pub fn new(config: SecurityConfig) -> Self {
        let credential_store =
            InMemoryCredentialStore::new(config.credential_store_path.clone().unwrap_or_default());
        Self {
            config,
            permission_manager: PermissionManager::new(),
            policy_engine: PolicyEngine::new(),
            audit_logger: AuditLogger::new(),
            credential_store: RwLock::new(credential_store),
            encryption_manager: EncryptionManager::new(),
        }
    }

    /// Authenticate a user and return an auth result.
    pub async fn authenticate(
        &self,
        _authenticator: &dyn Authenticator,
        credentials: &str,
    ) -> AuthResult {
        if !self.config.auth_enabled {
            tracing::warn!("Authentication is disabled; allowing access");
            return AuthResult {
                authenticated: true,
                token: Some(AuthToken {
                    token_id: "disabled-mode".into(),
                    user_id: "anonymous".into(),
                    roles: vec![],
                    expires_at: String::new(),
                    issued_at: String::new(),
                    metadata: Default::default(),
                }),
                error: None,
            };
        }
        _authenticator.authenticate(credentials).await
    }

    /// Authorize a token against a policy.
    pub async fn authorize(
        &self,
        token: &AuthToken,
        resource: &str,
        action: &str,
    ) -> bool {
        self.policy_engine
            .check_access(&token.user_id, &token.roles, resource, action)
            .await
    }

    /// Check whether a user holds a specific permission.
    pub async fn check_permission(
        &self,
        user_id: &str,
        permission: &Permission,
    ) -> bool {
        self.permission_manager.check(user_id, permission).await
    }

    /// Write an audit log entry.
    pub async fn audit_log(
        &self,
        user_id: &str,
        action: &str,
        resource: &str,
        outcome: AuditOutcome,
        details: Option<String>,
    ) {
        if self.config.audit_enabled {
            self.audit_logger
                .log_event(user_id, action, resource, outcome, details)
                .await;
        }
    }

    /// Store a credential.
    pub async fn store_credential(&self, credential: Credential) -> Result<String, String> {
        let mut store = self.credential_store.write().await;
        let id = credential.id.clone();
        store.store(credential).await;
        Ok(id)
    }

    /// Validate access against the policy engine.
    pub async fn validate_policy(
        &self,
        user_id: &str,
        resource: &str,
        action: &str,
    ) -> bool {
        self.policy_engine.check_access(user_id, &[], resource, action).await
    }

    /// Return a reference to the permission manager.
    pub fn permission_manager(&self) -> &PermissionManager {
        &self.permission_manager
    }

    /// Return a reference to the policy engine.
    pub fn policy_engine(&self) -> &PolicyEngine {
        &self.policy_engine
    }

    /// Return a reference to the audit logger.
    pub fn audit_logger(&self) -> &AuditLogger {
        &self.audit_logger
    }

    /// Return a reference to the encryption manager.
    pub fn encryption_manager(&self) -> &EncryptionManager {
        &self.encryption_manager
    }

    /// Return the current security configuration.
    pub fn config(&self) -> &SecurityConfig {
        &self.config
    }
}

impl fmt::Debug for SecurityManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SecurityManager")
            .field("config", &self.config)
            .finish()
    }
}
