/// Authentication subsystem for the Neo security layer.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

/// An authentication token issued after successful authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken {
    /// Unique identifier for this token.
    pub token_id: String,
    /// ID of the user that owns the token.
    pub user_id: String,
    /// Roles granted to the user.
    pub roles: Vec<String>,
    /// Expiration timestamp (ISO-8601 string).
    pub expires_at: String,
    /// Issuance timestamp (ISO-8601 string).
    pub issued_at: String,
    /// Arbitrary metadata attached to the token.
    pub metadata: std::collections::HashMap<String, String>,
}

/// Result returned from an authentication attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResult {
    /// Whether authentication succeeded.
    pub authenticated: bool,
    /// The issued token, if authentication succeeded.
    pub token: Option<AuthToken>,
    /// An error message, if authentication failed.
    pub error: Option<String>,
}

impl fmt::Display for AuthResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.authenticated {
            write!(f, "authenticated")
        } else {
            match &self.error {
                Some(e) => write!(f, "authentication failed: {e}"),
                None => write!(f, "authentication failed"),
            }
        }
    }
}

/// Async trait for implementing authentication backends.
#[async_trait]
pub trait Authenticator: Send + Sync {
    /// Authenticate using raw credential data.
    async fn authenticate(&self, credentials: &str) -> AuthResult;

    /// Validate an existing token.
    async fn validate_token(&self, token: &AuthToken) -> bool;

    /// Revoke a token.
    async fn revoke_token(&self, token_id: &str) -> Result<(), String>;

    /// Refresh an existing token, returning a new one.
    async fn refresh_token(&self, token: &AuthToken) -> AuthResult;
}

/// A mock authenticator that always succeeds. Useful for testing.
#[derive(Debug, Clone)]
pub struct MockAuthenticator;

impl MockAuthenticator {
    /// Create a new MockAuthenticator.
    pub fn new() -> Self {
        Self
    }
}

impl Default for MockAuthenticator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Authenticator for MockAuthenticator {
    async fn authenticate(&self, _credentials: &str) -> AuthResult {
        tracing::info!("MockAuthenticator: allowing all credentials");
        AuthResult {
            authenticated: true,
            token: Some(AuthToken {
                token_id: "mock-token-0".into(),
                user_id: "mock-user".into(),
                roles: vec!["user".into()],
                expires_at: "2099-12-31T23:59:59Z".into(),
                issued_at: "2026-01-01T00:00:00Z".into(),
                metadata: Default::default(),
            }),
            error: None,
        }
    }

    async fn validate_token(&self, _token: &AuthToken) -> bool {
        tracing::info!("MockAuthenticator: validating token");
        true
    }

    async fn revoke_token(&self, _token_id: &str) -> Result<(), String> {
        tracing::info!("MockAuthenticator: revoking token");
        Ok(())
    }

    async fn refresh_token(&self, token: &AuthToken) -> AuthResult {
        tracing::info!("MockAuthenticator: refreshing token");
        AuthResult {
            authenticated: true,
            token: Some(AuthToken {
                token_id: format!("{}-refreshed", token.token_id),
                user_id: token.user_id.clone(),
                roles: token.roles.clone(),
                expires_at: "2099-12-31T23:59:59Z".into(),
                issued_at: "2026-01-01T00:00:00Z".into(),
                metadata: token.metadata.clone(),
            }),
            error: None,
        }
    }
}
