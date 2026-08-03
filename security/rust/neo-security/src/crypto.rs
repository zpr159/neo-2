use ed25519_dalek::{Signer as _, SigningKey, Verifier as _, VerifyingKey, SecretKey as DalekSecretKey, Signature};
use rand::rngs::OsRng;
use rand::RngCore;
use sha2::{Digest, Sha256};
use serde::{Serialize, Deserialize};

pub type NeoResult<T> = Result<T, CryptoError>;

#[derive(Debug, Clone, thiserror::Error)]
pub enum CryptoError {
    #[error("encryption failed: {0}")]
    EncryptionFailed(String),
    #[error("decryption failed: {0}")]
    DecryptionFailed(String),
    #[error("invalid signature")]
    InvalidSignature,
    #[error("invalid key length")]
    InvalidKeyLength,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    pub algorithm: String,
    pub key_size: u32,
    pub mode: String,
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            algorithm: "AES-256-GCM".to_string(),
            key_size: 256,
            mode: "GCM".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretKey(pub Vec<u8>);

impl SecretKey {
    pub fn as_bytes(&self) -> &[u8] { &self.0 }
    pub fn len(&self) -> usize { self.0.len() }
    pub fn is_empty(&self) -> bool { self.0.is_empty() }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicKey(pub Vec<u8>);

impl PublicKey {
    pub fn as_bytes(&self) -> &[u8] { &self.0 }
    pub fn len(&self) -> usize { self.0.len() }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyPair {
    pub public: PublicKey,
    pub secret: SecretKey,
}

impl KeyPair {
    pub fn generate() -> Self {
        let mut csprng = OsRng;
        let mut secret_bytes = [0u8; 32];
        csprng.fill_bytes(&mut secret_bytes);
        let signing_key = SigningKey::from_bytes(&secret_bytes);
        let verifying_key = signing_key.verifying_key();

        Self {
            public: PublicKey(verifying_key.as_bytes().to_vec()),
            secret: SecretKey(signing_key.to_bytes().to_vec()),
        }
    }

    pub fn from_bytes(public: Vec<u8>, secret: Vec<u8>) -> Self {
        Self {
            public: PublicKey(public),
            secret: SecretKey(secret),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Encryption {
    pub config: EncryptionConfig,
}

impl Encryption {
    pub fn new(config: EncryptionConfig) -> Self {
        Self { config }
    }
}

#[derive(Debug, Clone)]
pub struct Decryption {
    pub config: EncryptionConfig,
}

impl Decryption {
    pub fn new(config: EncryptionConfig) -> Self {
        Self { config }
    }
}
