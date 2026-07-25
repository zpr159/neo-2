/// Encryption-at-rest subsystem for the Neo security layer.
///
/// NOTE: This module currently provides pass-through encrypt / decrypt stubs.
/// Actual cryptographic operations require runtime crypto libraries and are
/// handled at the deployment layer.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Supported encryption algorithms.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EncryptionAlgorithm {
    /// AES-256 in Galois/Counter Mode.
    Aes256Gcm,
    /// ChaCha20-Poly1305.
    ChaCha20Poly1305,
    /// Ed25519 (used for signing; included for completeness).
    Ed25519,
}

impl std::fmt::Display for EncryptionAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Aes256Gcm => write!(f, "aes-256-gcm"),
            Self::ChaCha20Poly1305 => write!(f, "chacha20-poly1305"),
            Self::Ed25519 => write!(f, "ed25519"),
        }
    }
}

/// Metadata about an encryption key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionKey {
    /// Unique key identifier.
    pub key_id: String,
    /// The algorithm the key is used with.
    pub algorithm: EncryptionAlgorithm,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
}

/// Manages encryption keys and (de)encryption operations.
#[derive(Debug)]
pub struct EncryptionManager {
    /// key_id -> EncryptionKey
    keys: HashMap<String, EncryptionKey>,
    /// The id of the currently-active key.
    active_key_id: Option<String>,
}

impl EncryptionManager {
    /// Create a new EncryptionManager with no keys.
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
            active_key_id: None,
        }
    }

    /// Encrypt data. Currently a pass-through stub.
    pub fn encrypt(&self, data: &[u8]) -> Vec<u8> {
        tracing::warn!(
            "encryption is stubbed out; returning data unchanged ({} bytes)",
            data.len()
        );
        data.to_vec()
    }

    /// Decrypt data. Currently a pass-through stub.
    pub fn decrypt(&self, data: &[u8]) -> Vec<u8> {
        tracing::warn!(
            "decryption is stubbed out; returning data unchanged ({} bytes)",
            data.len()
        );
        data.to_vec()
    }

    /// Generate and register a new encryption key.
    pub fn rotate_key(&mut self, algorithm: EncryptionAlgorithm) -> EncryptionKey {
        let key_id = format!("key-{}", uuid::Uuid::new_v4());
        let key = EncryptionKey {
            key_id: key_id.clone(),
            algorithm,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        tracing::info!(key_id = %key_id, algorithm = %key.algorithm, "new encryption key created");
        self.keys.insert(key_id, key.clone());
        self.active_key_id = Some(key.key_id.clone());
        key
    }

    /// List all registered keys.
    pub fn list_keys(&self) -> Vec<EncryptionKey> {
        self.keys.values().cloned().collect()
    }

    /// Return the id of the currently-active key, if any.
    pub fn active_key_id(&self) -> Option<&str> {
        self.active_key_id.as_deref()
    }
}

impl Default for EncryptionManager {
    fn default() -> Self {
        Self::new()
    }
}
