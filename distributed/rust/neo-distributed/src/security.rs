//! Cluster security — mutual TLS, node authentication, authorization,
//! certificate management, key rotation, and signed messages.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::config::SecurityConfiguration;
use crate::error::{DistributedError, NeoResult};
use crate::types::NodeId;

// ---------------------------------------------------------------------------
// NodeCertificate
// ---------------------------------------------------------------------------

/// A node's TLS certificate and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCertificate {
    /// Node this certificate belongs to.
    pub node_id: NodeId,
    /// PEM-encoded certificate.
    pub cert_pem: String,
    /// SHA-256 fingerprint.
    pub fingerprint: String,
    /// When the certificate was issued.
    pub issued_at: DateTime<Utc>,
    /// When the certificate expires.
    pub expires_at: DateTime<Utc>,
    /// Issuer (CA) identifier.
    pub issuer: String,
    /// Serial number.
    pub serial: String,
}

impl NodeCertificate {
    /// Check if the certificate is still valid.
    pub fn is_valid(&self) -> bool {
        let now = Utc::now();
        now >= self.issued_at && now <= self.expires_at
    }

    /// Check if the certificate will expire within the given duration.
    pub fn is_expiring_soon(&self, within: Duration) -> bool {
        let remaining = self.expires_at.signed_duration_since(Utc::now());
        remaining.to_std().unwrap_or(Duration::ZERO) < within
    }

    /// Compute SHA-256 fingerprint of the PEM data.
    pub fn compute_fingerprint(pem: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(pem.as_bytes());
        let result = hasher.finalize();
        result
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<_>>()
            .join(":")
    }
}

// ---------------------------------------------------------------------------
// CertificateManager
// ---------------------------------------------------------------------------

/// Manages TLS certificates for the cluster.
pub struct CertificateManager {
    /// Our own certificate.
    own_cert: RwLock<Option<NodeCertificate>>,
    /// Trusted CA certificates (fingerprint → cert).
    trusted_cas: RwLock<HashMap<String, NodeCertificate>>,
    /// Node certificates cache.
    node_certs: RwLock<HashMap<NodeId, NodeCertificate>>,
    /// Revoked certificate fingerprints.
    revoked: RwLock<Vec<String>>,
    /// Key rotation interval.
    rotation_interval: Duration,
    /// Last rotation time.
    last_rotation: RwLock<Option<Instant>>,
}

impl CertificateManager {
    pub fn new(rotation_interval: Duration) -> Self {
        tracing::info!(
            rotation_interval_secs = rotation_interval.as_secs(),
            "certificate manager created"
        );
        Self {
            own_cert: RwLock::new(None),
            trusted_cas: RwLock::new(HashMap::new()),
            node_certs: RwLock::new(HashMap::new()),
            revoked: RwLock::new(Vec::new()),
            rotation_interval,
            last_rotation: RwLock::new(None),
        }
    }

    /// Set our own certificate.
    pub fn set_own_certificate(&self, cert: NodeCertificate) {
        tracing::info!(node_id = %cert.node_id, "own certificate set");
        *self.own_cert.write() = Some(cert);
    }

    /// Get our own certificate.
    pub fn own_certificate(&self) -> Option<NodeCertificate> {
        self.own_cert.read().clone()
    }

    /// Register a trusted CA certificate.
    pub fn add_trusted_ca(&self, cert: NodeCertificate) {
        self.trusted_cas
            .write()
            .insert(cert.fingerprint.clone(), cert);
    }

    /// Cache a node's certificate.
    pub fn cache_node_certificate(&self, node_id: NodeId, cert: NodeCertificate) {
        self.node_certs.write().insert(node_id, cert);
    }

    /// Get a node's cached certificate.
    pub fn get_node_certificate(&self, node_id: NodeId) -> Option<NodeCertificate> {
        self.node_certs.read().get(&node_id).cloned()
    }

    /// Validate a certificate.
    pub fn validate_certificate(&self, cert: &NodeCertificate) -> NeoResult<()> {
        // Check expiry.
        if !cert.is_valid() {
            return Err(DistributedError::security(format!(
                "certificate expired: {}",
                cert.fingerprint
            )));
        }

        // Check revocation.
        if self.revoked.read().contains(&cert.fingerprint) {
            return Err(DistributedError::security(format!(
                "certificate revoked: {}",
                cert.fingerprint
            )));
        }

        Ok(())
    }

    /// Revoke a certificate.
    pub fn revoke(&self, fingerprint: String) {
        tracing::warn!(fingerprint = %fingerprint, "certificate revoked");
        self.revoked.write().push(fingerprint);
    }

    /// Check if a certificate is revoked.
    pub fn is_revoked(&self, fingerprint: &str) -> bool {
        self.revoked.read().iter().any(|f| f == fingerprint)
    }

    /// Check if key rotation is needed.
    pub fn needs_rotation(&self) -> bool {
        match *self.last_rotation.read() {
            Some(last) => last.elapsed() >= self.rotation_interval,
            None => true,
        }
    }

    /// Perform key rotation (generates new certificate).
    pub fn rotate(&self, new_cert: NodeCertificate) -> NeoResult<()> {
        self.set_own_certificate(new_cert);
        *self.last_rotation.write() = Some(Instant::now());
        tracing::info!("key rotation completed");
        Ok(())
    }

    /// Get certificates expiring soon.
    pub fn expiring_soon(&self, within: Duration) -> Vec<NodeCertificate> {
        self.node_certs
            .read()
            .values()
            .filter(|c| c.is_expiring_soon(within))
            .cloned()
            .collect()
    }

    /// Number of cached certificates.
    pub fn cached_count(&self) -> usize {
        self.node_certs.read().len()
    }

    /// Number of revoked certificates.
    pub fn revoked_count(&self) -> usize {
        self.revoked.read().len()
    }
}

// ---------------------------------------------------------------------------
// ClusterSecurity
// ---------------------------------------------------------------------------

/// High-level cluster security manager.
pub struct ClusterSecurity {
    config: RwLock<SecurityConfiguration>,
    cert_manager: Arc<CertificateManager>,
    /// Signed message nonces (to prevent replay).
    nonces: RwLock<HashMap<String, Instant>>,
    /// Message signing keys (node_id → key bytes).
    signing_keys: RwLock<HashMap<NodeId, Vec<u8>>>,
}

impl ClusterSecurity {
    pub fn new(config: SecurityConfiguration) -> Self {
        let rotation_interval = config.key_rotation_interval;
        let cert_manager = Arc::new(CertificateManager::new(rotation_interval));
        tracing::info!(
            enabled = config.enabled,
            mtls = config.mtls_enabled,
            "cluster security created"
        );
        Self {
            config: RwLock::new(config),
            cert_manager,
            nonces: RwLock::new(HashMap::new()),
            signing_keys: RwLock::new(HashMap::new()),
        }
    }

    /// Get the certificate manager.
    pub fn cert_manager(&self) -> &Arc<CertificateManager> {
        &self.cert_manager
    }

    /// Check if security is enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.read().enabled
    }

    /// Check if mTLS is enabled.
    pub fn is_mtls_enabled(&self) -> bool {
        self.config.read().mtls_enabled
    }

    /// Check if signed messages are enabled.
    pub fn is_signed_messages_enabled(&self) -> bool {
        self.config.read().signed_messages
    }

    /// Sign a message.
    pub fn sign_message(&self, node_id: NodeId, data: &[u8]) -> NeoResult<String> {
        let keys = self.signing_keys.read();
        let key = keys
            .get(&node_id)
            .ok_or_else(|| DistributedError::security("no signing key for node"))?;

        let mut hasher = Sha256::new();
        hasher.update(key);
        hasher.update(data);
        let signature = hasher.finalize();
        Ok(signature.iter().map(|b| format!("{b:02x}")).collect())
    }

    /// Verify a message signature.
    pub fn verify_signature(
        &self,
        node_id: NodeId,
        data: &[u8],
        signature: &str,
    ) -> NeoResult<bool> {
        let computed = self.sign_message(node_id, data)?;
        Ok(computed == signature)
    }

    /// Register a signing key for a node.
    pub fn register_signing_key(&self, node_id: NodeId, key: Vec<u8>) {
        self.signing_keys.write().insert(node_id, key);
    }

    /// Check for message replay using nonce.
    pub fn check_nonce(&self, nonce: &str) -> NeoResult<()> {
        let mut nonces = self.nonces.write();
        if nonces.contains_key(nonce) {
            return Err(DistributedError::security("duplicate nonce detected"));
        }
        nonces.insert(nonce.to_string(), Instant::now());
        Ok(())
    }

    /// Clean up old nonces.
    pub fn cleanup_nonces(&self, max_age: Duration) {
        let now = Instant::now();
        self.nonces.write().retain(|_, time| {
            now.duration_since(*time) < max_age
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn certificate_validity() {
        let cert = NodeCertificate {
            node_id: NodeId::new(),
            cert_pem: "test-pem".to_string(),
            fingerprint: "abc123".to_string(),
            issued_at: Utc::now() - chrono::Duration::hours(1),
            expires_at: Utc::now() + chrono::Duration::hours(24),
            issuer: "test-ca".to_string(),
            serial: "001".to_string(),
        };
        assert!(cert.is_valid());
    }

    #[test]
    fn certificate_fingerprint() {
        let fp = NodeCertificate::compute_fingerprint("test-data");
        assert_eq!(fp.len(), 63); // 32 bytes * 2 chars + 31 colons
    }

    #[test]
    fn certificate_manager() {
        let mgr = CertificateManager::new(Duration::from_secs(3600));
        let cert = NodeCertificate {
            node_id: NodeId::new(),
            cert_pem: "test".to_string(),
            fingerprint: "fp123".to_string(),
            issued_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::hours(24),
            issuer: "ca".to_string(),
            serial: "1".to_string(),
        };
        mgr.set_own_certificate(cert.clone());
        assert!(mgr.own_certificate().is_some());
    }

    #[test]
    fn certificate_validation() {
        let mgr = CertificateManager::new(Duration::from_secs(3600));
        let cert = NodeCertificate {
            node_id: NodeId::new(),
            cert_pem: "test".to_string(),
            fingerprint: "fp".to_string(),
            issued_at: Utc::now() - chrono::Duration::hours(1),
            expires_at: Utc::now() + chrono::Duration::hours(24),
            issuer: "ca".to_string(),
            serial: "1".to_string(),
        };
        assert!(mgr.validate_certificate(&cert).is_ok());
    }

    #[test]
    fn revoked_certificate() {
        let mgr = CertificateManager::new(Duration::from_secs(3600));
        mgr.revoke("revoked-fp".to_string());
        assert!(mgr.is_revoked("revoked-fp"));
        assert!(!mgr.is_revoked("other-fp"));
    }

    #[test]
    fn cluster_security() {
        let config = SecurityConfiguration {
            enabled: true,
            mtls_enabled: true,
            signed_messages: true,
            ..Default::default()
        };
        let sec = ClusterSecurity::new(config);
        assert!(sec.is_enabled());
        assert!(sec.is_mtls_enabled());
    }

    #[test]
    fn message_signing() {
        let config = SecurityConfiguration {
            enabled: true,
            signed_messages: true,
            ..Default::default()
        };
        let sec = ClusterSecurity::new(config);
        let node_id = NodeId::new();
        sec.register_signing_key(node_id, vec![1, 2, 3, 4]);
        let sig = sec.sign_message(node_id, b"test data").unwrap();
        assert!(sec.verify_signature(node_id, b"test data", &sig).unwrap());
        assert!(!sec.verify_signature(node_id, b"other data", &sig).unwrap());
    }

    #[test]
    fn nonce_check() {
        let config = SecurityConfiguration::default();
        let sec = ClusterSecurity::new(config);
        assert!(sec.check_nonce("nonce-1").is_ok());
        assert!(sec.check_nonce("nonce-1").is_err()); // Replay
        assert!(sec.check_nonce("nonce-2").is_ok());
    }
}
