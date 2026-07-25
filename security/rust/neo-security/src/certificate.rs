use chrono::{DateTime, Utc};
use dashmap::DashMap;
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub type NeoResult<T> = Result<T, CertificateError>;

#[derive(Debug, Clone, thiserror::Error)]
pub enum CertificateError {
    #[error("certificate not found: {0}")]
    NotFound(String),
    #[error("certificate expired: subject={subject}")]
    Expired { subject: String },
    #[error("certificate already revoked: {0}")]
    AlreadyRevoked(String),
}

#[derive(Debug, Clone)]
pub struct Certificate {
    pub id: Uuid,
    pub subject: String,
    pub issuer: String,
    pub not_before: DateTime<Utc>,
    pub not_after: DateTime<Utc>,
    pub serial_number: String,
    pub fingerprint: String,
}

impl Certificate {
    pub fn new(subject: &str, issuer: &str, valid_days: u32) -> Self {
        let now = Utc::now();
        let not_after = now + chrono::Duration::days(valid_days as i64);

        let fingerprint_input = format!("{}:{}:{}", subject, issuer, now.timestamp());
        let mut hasher = Sha256::new();
        hasher.update(fingerprint_input.as_bytes());
        let fingerprint = format!("{:x}", hasher.finalize());

        let serial_number = format!("{:032x}", Uuid::new_v4().as_u128());

        tracing::debug!(
            subject = subject,
            issuer = issuer,
            valid_days = valid_days,
            "certificate created"
        );

        Self {
            id: Uuid::new_v4(),
            subject: subject.to_string(),
            issuer: issuer.to_string(),
            not_before: now,
            not_after,
            serial_number,
            fingerprint,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CertificateChain {
    pub certificates: Vec<Certificate>,
    pub root_ca: Certificate,
}

impl CertificateChain {
    pub fn new(root_ca: Certificate) -> Self {
        Self {
            certificates: Vec::new(),
            root_ca,
        }
    }

    pub fn add_certificate(&mut self, cert: Certificate) {
        self.certificates.push(cert);
    }

    pub fn length(&self) -> usize {
        self.certificates.len() + 1
    }
}

#[derive(Debug)]
pub struct CertificateManager {
    certificates: DashMap<Uuid, Certificate>,
    chains: DashMap<Uuid, CertificateChain>,
}

impl CertificateManager {
    pub fn new() -> Self {
        tracing::info!("certificate manager initialized");
        Self {
            certificates: DashMap::new(),
            chains: DashMap::new(),
        }
    }

    pub fn generate_self_signed(
        &self,
        subject: &str,
        valid_days: u32,
    ) -> NeoResult<Certificate> {
        let cert = Certificate::new(subject, subject, valid_days);
        let cert_id = cert.id;
        tracing::info!(
            subject = subject,
            cert_id = %cert_id,
            valid_days = valid_days,
            "self-signed certificate generated"
        );
        self.certificates.insert(cert_id, cert.clone());
        Ok(cert)
    }

    pub fn validate(&self, cert: &Certificate) -> bool {
        let now = Utc::now();
        let valid = now >= cert.not_before && now <= cert.not_after;
        if !valid {
            tracing::warn!(
                subject = %cert.subject,
                "certificate validation failed: expired or not yet valid"
            );
        }
        valid
    }

    pub fn revoke(&self, cert_id: Uuid) -> NeoResult<()> {
        self.certificates
            .remove(&cert_id)
            .ok_or_else(|| CertificateError::NotFound(cert_id.to_string()))?;
        tracing::info!(cert_id = %cert_id, "certificate revoked");
        Ok(())
    }

    pub fn list_valid(&self) -> Vec<Certificate> {
        self.certificates
            .iter()
            .filter(|entry| {
                let now = Utc::now();
                now >= entry.value().not_before && now <= entry.value().not_after
            })
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn chain_for(&self, cert_id: Uuid) -> Option<CertificateChain> {
        self.chains.get(&cert_id).map(|entry| entry.value().clone())
    }

    pub fn total_certificates(&self) -> usize {
        self.certificates.len()
    }
}

impl Default for CertificateManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_certificate_generation() {
        let mgr = CertificateManager::new();
        let cert = mgr
            .generate_self_signed("test.example.com", 365)
            .unwrap();
        assert_eq!(cert.subject, "test.example.com");
        assert_eq!(cert.issuer, "test.example.com");
        assert!(!cert.fingerprint.is_empty());
    }

    #[test]
    fn test_certificate_validation() {
        let cert = Certificate::new("test.com", "ca.com", 365);
        let mgr = CertificateManager::new();
        assert!(mgr.validate(&cert));
    }

    #[test]
    fn test_revocation() {
        let mgr = CertificateManager::new();
        let cert = mgr.generate_self_signed("test.com", 365).unwrap();
        assert_eq!(mgr.total_certificates(), 1);

        mgr.revoke(cert.id).unwrap();
        assert_eq!(mgr.total_certificates(), 0);
    }

    #[test]
    fn test_list_valid() {
        let mgr = CertificateManager::new();
        mgr.generate_self_signed("a.com", 365).unwrap();
        mgr.generate_self_signed("b.com", 365).unwrap();

        let valid = mgr.list_valid();
        assert_eq!(valid.len(), 2);
    }
}
