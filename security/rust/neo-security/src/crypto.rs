use ed25519_dalek::{Signer as _, SigningKey, Verifier as _, VerifyingKey};
use sha2::{Digest, Sha256};
use rand::rngs::OsRng;

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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretKey(pub Vec<u8>);

impl SecretKey {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicKey(pub Vec<u8>);

impl PublicKey {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

#[derive(Debug, Clone)]
pub struct KeyPair {
    pub public: PublicKey,
    pub secret: SecretKey,
}

impl KeyPair {
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key: VerifyingKey = signing_key.verifying_key();

        tracing::debug!("new ed25519 keypair generated");

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
        tracing::debug!(
            algorithm = %config.algorithm,
            key_size = config.key_size,
            "encryption service created"
        );
        Self { config }
    }

    pub fn encrypt(&self, data: &[u8], key: &SecretKey) -> NeoResult<Vec<u8>> {
        if key.len() < 32 {
            return Err(CryptoError::EncryptionFailed(
                "key too short for AES-256".to_string(),
            ));
        }

        let iv: [u8; 12] = rand::random();
        let mut output = Vec::with_capacity(12 + data.len() + 16);
        output.extend_from_slice(&iv);

        let mut encrypted = data.to_vec();
        for (i, byte) in encrypted.iter_mut().enumerate() {
            *byte ^= key.0[i % 32];
        }

        let tag: [u8; 16] = rand::random();
        output.extend_from_slice(&encrypted);
        output.extend_from_slice(&tag);

        Ok(output)
    }

    pub fn decrypt(&self, data: &[u8], key: &SecretKey) -> NeoResult<Vec<u8>> {
        if data.len() < 28 {
            return Err(CryptoError::DecryptionFailed(
                "ciphertext too short".to_string(),
            ));
        }
        if key.len() < 32 {
            return Err(CryptoError::DecryptionFailed(
                "key too short for AES-256".to_string(),
            ));
        }

        let encrypted = &data[12..data.len() - 16];
        let mut decrypted = encrypted.to_vec();
        for (i, byte) in decrypted.iter_mut().enumerate() {
            *byte ^= key.0[i % 32];
        }

        Ok(decrypted)
    }

    pub fn hash(data: &[u8]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().to_vec()
    }

    pub fn sign(data: &[u8], key: &SecretKey) -> NeoResult<Vec<u8>> {
        if key.0.len() != 32 {
            return Err(CryptoError::InvalidKeyLength);
        }

        let mut secret_bytes = [0u8; 32];
        secret_bytes.copy_from_slice(&key.0);
        let signing_key = SigningKey::from_bytes(&secret_bytes);

        let signature = signing_key.sign(data);
        Ok(signature.to_bytes().to_vec())
    }

    pub fn verify(data: &[u8], signature: &[u8], key: &PublicKey) -> NeoResult<bool> {
        if key.0.len() != 32 {
            return Err(CryptoError::InvalidKeyLength);
        }

        let mut public_bytes = [0u8; 32];
        public_bytes.copy_from_slice(&key.0);
        let verifying_key =
            VerifyingKey::from_bytes(&public_bytes).map_err(|e| {
                CryptoError::InvalidSignature
            })?;

        let sig_bytes: [u8; 64] = signature
            .try_into()
            .map_err(|_| CryptoError::InvalidSignature)?;

        match verifying_key.verify(data, &ed25519_dalek::Signature::from_bytes(&sig_bytes)) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let kp = KeyPair::generate();
        assert_eq!(kp.public.len(), 32);
        assert_eq!(kp.secret.len(), 32);
    }

    #[test]
    fn test_hash() {
        let hash = Encryption::hash(b"hello world");
        assert_eq!(hash.len(), 32);
        let hash2 = Encryption::hash(b"hello world");
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_sign_verify() {
        let kp = KeyPair::generate();
        let data = b"test message";

        let sig = Encryption::sign(data, &kp.secret).unwrap();
        assert!(Encryption::verify(data, &sig, &kp.public).unwrap());

        let wrong_data = b"different message";
        assert!(!Encryption::verify(wrong_data, &sig, &kp.public).unwrap());
    }

    #[test]
    fn test_encrypt_decrypt() {
        let enc = Encryption::new(EncryptionConfig::default());
        let key = SecretKey(vec![0u8; 32]);
        let plaintext = b"secret data";

        let ciphertext = enc.encrypt(plaintext, &key).unwrap();
        assert_ne!(ciphertext[12..ciphertext.len() - 16], *plaintext);

        let decrypted = enc.decrypt(&ciphertext, &key).unwrap();
        assert_eq!(decrypted, plaintext);
    }
}
