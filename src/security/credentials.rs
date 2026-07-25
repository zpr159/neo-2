/// Credential storage subsystem for the Neo security layer.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;

/// The type of credential being stored.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CredentialType {
    /// An API key.
    ApiKey,
    /// A password.
    Password,
    /// A bearer or access token.
    Token,
    /// A TLS certificate.
    Certificate,
    /// An OAuth2 client / access token.
    OAuth2,
}

impl std::fmt::Display for CredentialType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::ApiKey => "api_key",
            Self::Password => "password",
            Self::Token => "token",
            Self::Certificate => "certificate",
            Self::OAuth2 => "oauth2",
        };
        write!(f, "{label}")
    }
}

/// A stored credential.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credential {
    /// Unique identifier for the credential.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// The type of credential.
    pub credential_type: CredentialType,
    /// The secret value (stored as-is; real encryption happens at the deployment layer).
    pub value: String,
    /// Arbitrary metadata.
    pub metadata: HashMap<String, String>,
    /// Creation timestamp (ISO-8601).
    pub created_at: String,
    /// Optional expiration timestamp (ISO-8601).
    pub expires_at: Option<String>,
}

/// Async trait for credential storage backends.
#[async_trait]
pub trait CredentialStore: Send + Sync {
    /// Store a credential.
    async fn store(&mut self, credential: Credential);

    /// Retrieve a credential by id.
    async fn retrieve(&self, id: &str) -> Option<Credential>;

    /// Delete a credential by id.
    async fn delete(&mut self, id: &str) -> Result<(), String>;

    /// List all stored credentials.
    async fn list(&self) -> Vec<Credential>;

    /// Rotate a credential's value.
    async fn rotate(&mut self, id: &str, new_value: String) -> Result<(), String>;
}

/// In-memory credential store. Suitable for testing.
#[derive(Debug)]
pub struct InMemoryCredentialStore {
    credentials: RwLock<HashMap<String, Credential>>,
    /// Optional path hint for debugging / diagnostics.
    _store_path: String,
}

impl InMemoryCredentialStore {
    /// Create a new in-memory credential store.
    pub fn new(store_path: String) -> Self {
        Self {
            credentials: RwLock::new(HashMap::new()),
            _store_path: store_path,
        }
    }
}

#[async_trait]
impl CredentialStore for InMemoryCredentialStore {
    async fn store(&mut self, credential: Credential) {
        tracing::info!(id = %credential.id, name = %credential.name, "storing credential");
        let mut creds = self.credentials.write().await;
        creds.insert(credential.id.clone(), credential);
    }

    async fn retrieve(&self, id: &str) -> Option<Credential> {
        let creds = self.credentials.read().await;
        creds.get(id).cloned()
    }

    async fn delete(&mut self, id: &str) -> Result<(), String> {
        let mut creds = self.credentials.write().await;
        if creds.remove(id).is_some() {
            tracing::info!(id = %id, "credential deleted");
            Ok(())
        } else {
            Err(format!("credential {id} not found"))
        }
    }

    async fn list(&self) -> Vec<Credential> {
        let creds = self.credentials.read().await;
        creds.values().cloned().collect()
    }

    async fn rotate(&mut self, id: &str, new_value: String) -> Result<(), String> {
        let mut creds = self.credentials.write().await;
        match creds.get_mut(id) {
            Some(cred) => {
                cred.value = new_value;
                tracing::info!(id = %id, "credential rotated");
                Ok(())
            }
            None => Err(format!("credential {id} not found")),
        }
    }
}
