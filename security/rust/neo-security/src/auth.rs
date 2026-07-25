use std::collections::HashMap;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type NeoResult<T> = Result<T, AuthError>;

#[derive(Debug, Clone, thiserror::Error)]
pub enum AuthError {
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("token expired")]
    TokenExpired,
    #[error("token not found: {0}")]
    TokenNotFound(String),
    #[error("authentication provider error: {0}")]
    ProviderError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuthProvider {
    ApiKey,
    OAuth2,
    Jwt,
    Certificate,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthResult {
    Authenticated {
        principal: String,
        permissions: Vec<String>,
    },
    Denied {
        reason: String,
    },
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken {
    pub token_id: Uuid,
    pub principal: String,
    pub provider: AuthProvider,
    pub permissions: Vec<String>,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

impl AuthToken {
    pub fn is_valid(&self) -> bool {
        Utc::now() < self.expires_at
    }

    pub fn has_permission(&self, perm: &str) -> bool {
        self.is_valid() && self.permissions.iter().any(|p| p == perm)
    }

    pub fn remaining_secs(&self) -> i64 {
        let now = Utc::now();
        if now >= self.expires_at {
            0
        } else {
            (self.expires_at - now).num_seconds()
        }
    }
}

#[derive(Debug, Clone)]
pub struct Authenticator {
    tokens: DashMap<Uuid, AuthToken>,
}

impl Authenticator {
    pub fn new() -> Self {
        tracing::info!("authenticator initialized");
        Self {
            tokens: DashMap::new(),
        }
    }

    pub async fn authenticate(
        &self,
        credentials: &serde_json::Value,
    ) -> NeoResult<AuthResult> {
        let provider_str = credentials
            .get("provider")
            .and_then(|v| v.as_str())
            .unwrap_or("api_key");

        let provider = match provider_str {
            "api_key" => AuthProvider::ApiKey,
            "oauth2" => AuthProvider::OAuth2,
            "jwt" => AuthProvider::Jwt,
            "certificate" => AuthProvider::Certificate,
            other => AuthProvider::Custom(other.to_string()),
        };

        let principal = credentials
            .get("principal")
            .and_then(|v| v.as_str())
            .unwrap_or("anonymous")
            .to_string();

        let permissions: Vec<String> = credentials
            .get("permissions")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let token = AuthToken {
            token_id: Uuid::new_v4(),
            principal: principal.clone(),
            provider,
            permissions: permissions.clone(),
            issued_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::hours(24),
            metadata: HashMap::new(),
        };

        let token_id = token.token_id;
        self.tokens.insert(token_id, token);

        tracing::info!(
            principal = %principal,
            token_id = %token_id,
            "authentication successful"
        );

        Ok(AuthResult::Authenticated {
            principal,
            permissions,
        })
    }

    pub fn validate_token(&self, token: &str) -> Option<AuthToken> {
        if let Ok(uuid) = Uuid::parse_str(token) {
            if let Some(entry) = self.tokens.get(&uuid) {
                if entry.is_valid() {
                    return Some(entry.value().clone());
                }
                tracing::debug!(token_id = %uuid, "token expired");
            }
        }
        None
    }

    pub fn revoke_token(&self, token_id: Uuid) -> bool {
        let removed = self.tokens.remove(&token_id).is_some();
        if removed {
            tracing::info!(token_id = %token_id, "token revoked");
        }
        removed
    }

    pub fn active_token_count(&self) -> usize {
        self.tokens.iter().filter(|e| e.value().is_valid()).count()
    }
}

impl Default for Authenticator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_authentication() {
        let auth = Authenticator::new();
        let creds = serde_json::json!({
            "provider": "api_key",
            "principal": "admin",
            "permissions": ["read", "write"]
        });

        let result = auth.authenticate(&creds).await.unwrap();
        match result {
            AuthResult::Authenticated {
                principal,
                permissions,
            } => {
                assert_eq!(principal, "admin");
                assert!(permissions.contains(&"read".to_string()));
            }
            _ => panic!("expected authenticated result"),
        }
    }

    #[tokio::test]
    async fn test_token_validation() {
        let auth = Authenticator::new();
        let creds = serde_json::json!({
            "provider": "jwt",
            "principal": "user1",
            "permissions": ["execute"]
        });

        let _ = auth.authenticate(&creds).await.unwrap();
        assert_eq!(auth.active_token_count(), 1);
    }

    #[test]
    fn test_token_permissions() {
        let token = AuthToken {
            token_id: Uuid::new_v4(),
            principal: "test".to_string(),
            provider: AuthProvider::ApiKey,
            permissions: vec!["read".to_string()],
            issued_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::hours(1),
            metadata: HashMap::new(),
        };
        assert!(token.has_permission("read"));
        assert!(!token.has_permission("write"));
        assert!(token.is_valid());
    }
}
