use sha2::{Sha256, Digest};

/// Provides simple hash-based integrity checks for graph data.
/// (Full AES-GCM encryption is available via the neo-security crate.)
pub struct GraphEncryption;

impl GraphEncryption {
    /// Create a new encryption utility.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Compute a SHA-256 hash of the given data.
    #[must_use]
    pub fn hash(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        hex::encode(result)
    }

    /// Compute a checksum for a string.
    #[must_use]
    pub fn checksum(data: &str) -> String {
        Self::hash(data.as_bytes())
    }
}

impl Default for GraphEncryption {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple hex encoding helper (avoids adding hex crate dependency).
mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        bytes
            .as_ref()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect()
    }
}
