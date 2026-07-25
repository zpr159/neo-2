use std::collections::HashMap;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::core::{CapabilityCategory, CapabilityVersion};
use crate::discovery::CapabilityManifest;
use crate::error::{CapabilityError, CapabilityResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceManifest {
    pub name: String,
    pub version: CapabilityVersion,
    pub description: String,
    pub author: String,
    pub license: String,
    pub repository_url: String,
    pub homepage: String,
    pub keywords: Vec<String>,
    pub category: CapabilityCategory,
    pub min_neo_version: CapabilityVersion,
    pub dependencies: Vec<MarketplaceDependency>,
    pub permissions: Vec<String>,
    pub checksum: String,
    pub signature: Option<String>,
    pub icon_url: String,
    pub changelog: String,
    pub maintainers: Vec<String>,
    pub downloads_count: u64,
    pub rating: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceDependency {
    pub name: String,
    pub version: CapabilityVersion,
    pub optional: bool,
}

impl MarketplaceManifest {
    pub fn new(
        name: impl Into<String>,
        version: CapabilityVersion,
        description: impl Into<String>,
        author: impl Into<String>,
        category: CapabilityCategory,
    ) -> Self {
        Self {
            name: name.into(),
            version,
            description: description.into(),
            author: author.into(),
            license: String::from("MIT"),
            repository_url: String::new(),
            homepage: String::new(),
            keywords: Vec::new(),
            category,
            min_neo_version: CapabilityVersion::new(1, 0, 0),
            dependencies: Vec::new(),
            permissions: Vec::new(),
            checksum: String::new(),
            signature: None,
            icon_url: String::new(),
            changelog: String::new(),
            maintainers: Vec::new(),
            downloads_count: 0,
            rating: 0.0,
        }
    }

    pub fn validate(&self) -> CapabilityResult<()> {
        if self.name.is_empty() {
            return Err(CapabilityError::validation_failed(
                "marketplace manifest name cannot be empty",
            ));
        }
        if self.description.is_empty() {
            return Err(CapabilityError::validation_failed(
                "marketplace manifest description cannot be empty",
            ));
        }
        if self.author.is_empty() {
            return Err(CapabilityError::validation_failed(
                "marketplace manifest author cannot be empty",
            ));
        }
        if self.rating < 0.0 || self.rating > 5.0 {
            return Err(CapabilityError::validation_failed(
                "rating must be between 0.0 and 5.0",
            ));
        }
        for dep in &self.dependencies {
            if dep.name.is_empty() {
                return Err(CapabilityError::validation_failed(
                    "dependency name cannot be empty",
                ));
            }
        }
        Ok(())
    }

    pub fn compute_checksum(&mut self) {
        let mut hasher = Sha256::new();
        hasher.update(self.name.as_bytes());
        hasher.update(self.version.to_string().as_bytes());
        hasher.update(self.description.as_bytes());
        hasher.update(self.author.as_bytes());
        let json = serde_json::to_string(&self.dependencies).unwrap_or_default();
        hasher.update(json.as_bytes());
        self.checksum = format!("{:x}", hasher.finalize());
    }

    pub fn to_manifest(&self) -> CapabilityManifest {
        CapabilityManifest {
            name: self.name.clone(),
            version: self.version.to_string(),
            description: self.description.clone(),
            category: format!("{}", self.category),
            namespace: String::new(),
            author: self.author.clone(),
            license: self.license.clone(),
            tags: self.keywords.clone(),
            aliases: Vec::new(),
            dependencies: self
                .dependencies
                .iter()
                .map(|d| crate::discovery::ManifestDependency {
                    name: d.name.clone(),
                    version_constraint: format!(">={}", d.version),
                    optional: d.optional,
                })
                .collect(),
            permissions: self.permissions.clone(),
            entry_point: None,
            min_neo_version: self.min_neo_version.to_string(),
            metadata: HashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// SigningInfo & CapabilitySignature
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SignatureAlgorithm {
    Ed25519,
    Sha256WithRsa,
    EcdsaP256,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigningInfo {
    pub signer_id: String,
    pub signature_algorithm: SignatureAlgorithm,
    pub public_key: String,
    pub signature: String,
    pub signed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitySignature {
    pub manifest_hash: String,
    pub signing_info: SigningInfo,
    pub verified: bool,
}

impl CapabilitySignature {
    pub fn new(manifest_hash: String, signing_info: SigningInfo) -> Self {
        Self {
            manifest_hash,
            signing_info,
            verified: false,
        }
    }

    pub fn verify(&mut self, expected_hash: &str) -> CapabilityResult<()> {
        if self.manifest_hash != expected_hash {
            self.verified = false;
            return Err(CapabilityError::signing(format!(
                "manifest hash mismatch: expected {}, got {}",
                expected_hash, self.manifest_hash
            )));
        }
        if self.signing_info.signature.is_empty() {
            self.verified = false;
            return Err(CapabilityError::signing("signature data is empty"));
        }
        if self.signing_info.public_key.is_empty() {
            self.verified = false;
            return Err(CapabilityError::signing("public key is empty"));
        }
        self.verified = true;
        Ok(())
    }

    pub fn is_valid(&self) -> bool {
        self.verified
    }
}

// ---------------------------------------------------------------------------
// VersionCompatibility
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionCompatibility {
    pub min_neo_version: CapabilityVersion,
    pub max_neo_version: Option<CapabilityVersion>,
    pub required_dependencies: Vec<(String, CapabilityVersion)>,
}

impl VersionCompatibility {
    pub fn new(
        min_neo_version: CapabilityVersion,
        max_neo_version: Option<CapabilityVersion>,
    ) -> Self {
        Self {
            min_neo_version,
            max_neo_version,
            required_dependencies: Vec::new(),
        }
    }

    pub fn check_compatibility(
        &self,
        neo_version: &CapabilityVersion,
        available_capabilities: &HashMap<String, CapabilityVersion>,
    ) -> CapabilityResult<()> {
        if *neo_version < self.min_neo_version {
            return Err(CapabilityError::version_incompatible(format!(
                "neo version {} is below minimum {}",
                neo_version, self.min_neo_version
            )));
        }
        if let Some(ref max) = self.max_neo_version {
            if *neo_version > *max {
                return Err(CapabilityError::version_incompatible(format!(
                    "neo version {} exceeds maximum {}",
                    neo_version, max
                )));
            }
        }
        for (dep_name, dep_version) in &self.required_dependencies {
            match available_capabilities.get(dep_name) {
                Some(available) => {
                    if !available.is_compatible_with(dep_version) {
                        return Err(CapabilityError::dependency_missing(format!(
                            "dependency '{}' requires {} but found {}",
                            dep_name, dep_version, available
                        )));
                    }
                }
                None => {
                    return Err(CapabilityError::dependency_missing(format!(
                        "required dependency '{}' not found",
                        dep_name
                    )));
                }
            }
        }
        Ok(())
    }

    pub fn is_compatible(
        &self,
        neo_version: &CapabilityVersion,
        available_capabilities: &HashMap<String, CapabilityVersion>,
    ) -> bool {
        self.check_compatibility(neo_version, available_capabilities)
            .is_ok()
    }
}

// ---------------------------------------------------------------------------
// InstallationRecord
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InstallationSource {
    Local,
    Registry(String),
    Git(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InstallationState {
    Installing,
    Installed,
    Updated,
    Failed,
    Removed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationRecord {
    pub id: Uuid,
    pub manifest: MarketplaceManifest,
    pub installed_at: DateTime<Utc>,
    pub source: InstallationSource,
    pub checksum: String,
    pub signature_verified: bool,
    pub state: InstallationState,
}

impl InstallationRecord {
    pub fn new(manifest: MarketplaceManifest, source: InstallationSource) -> Self {
        Self {
            id: Uuid::new_v4(),
            manifest,
            installed_at: Utc::now(),
            source,
            checksum: String::new(),
            signature_verified: false,
            state: InstallationState::Installing,
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.manifest.name.is_empty()
            && self.state != InstallationState::Failed
            && self.state != InstallationState::Removed
    }
}

// ---------------------------------------------------------------------------
// HookType & InstallationHook
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HookType {
    PreInstall,
    PostInstall,
    PreRemove,
    PostRemove,
    PreUpdate,
    PostUpdate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationHook {
    pub hook_type: HookType,
    pub script_path: Option<PathBuf>,
    pub command: String,
    pub timeout_ms: u64,
}

impl InstallationHook {
    pub fn new(hook_type: HookType, command: impl Into<String>, timeout_ms: u64) -> Self {
        Self {
            hook_type,
            script_path: None,
            command: command.into(),
            timeout_ms,
        }
    }

    pub fn with_script_path(mut self, path: PathBuf) -> Self {
        self.script_path = Some(path);
        self
    }

    pub fn execute(&self) -> CapabilityResult<()> {
        tracing::info!(
            hook_type = ?self.hook_type,
            command = %self.command,
            timeout_ms = self.timeout_ms,
            "executing installation hook"
        );
        if let Some(ref script_path) = self.script_path {
            tracing::info!(script = %script_path.display(), "hook has associated script");
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Marketplace
// ---------------------------------------------------------------------------

pub struct Marketplace {
    installed: RwLock<HashMap<Uuid, InstallationRecord>>,
    registry: RwLock<HashMap<String, MarketplaceManifest>>,
    hooks: RwLock<HashMap<HookType, Vec<InstallationHook>>>,
    signatures: RwLock<HashMap<Uuid, CapabilitySignature>>,
}

impl Marketplace {
    pub fn new() -> Self {
        Self {
            installed: RwLock::new(HashMap::new()),
            registry: RwLock::new(HashMap::new()),
            hooks: RwLock::new(HashMap::new()),
            signatures: RwLock::new(HashMap::new()),
        }
    }

    pub fn publish(&self, mut manifest: MarketplaceManifest) -> CapabilityResult<()> {
        manifest.validate()?;
        manifest.compute_checksum();
        let name = manifest.name.clone();
        self.registry.write().insert(name, manifest);
        Ok(())
    }

    pub fn install(
        &self,
        name: &str,
        version: &CapabilityVersion,
        source: InstallationSource,
        neo_version: &CapabilityVersion,
    ) -> CapabilityResult<Uuid> {
        let manifest = {
            let registry = self.registry.read();
            registry.get(name).cloned().ok_or_else(|| {
                CapabilityError::not_found(format!(
                    "capability '{}' not found in marketplace",
                    name
                ))
            })?
        };

        if manifest.version != *version {
            return Err(CapabilityError::version_incompatible(format!(
                "requested version {} but marketplace has {}",
                version, manifest.version
            )));
        }

        let compat = VersionCompatibility::new(manifest.min_neo_version.clone(), None);
        compat.check_compatibility(neo_version, &HashMap::new())?;

        self.run_hooks(&HookType::PreInstall)?;

        let mut record = InstallationRecord::new(manifest.clone(), source);
        record.checksum = manifest.checksum.clone();
        record.state = InstallationState::Installed;

        let id = record.id;
        self.installed.write().insert(id, record);

        self.run_hooks(&HookType::PostInstall)?;

        Ok(id)
    }

    pub fn uninstall(&self, id: &Uuid) -> CapabilityResult<()> {
        let mut record = {
            let installed = self.installed.read();
            installed.get(id).cloned().ok_or_else(|| {
                CapabilityError::not_found(format!("installation '{}' not found", id))
            })?
        };

        if record.state == InstallationState::Removed {
            return Err(CapabilityError::invalid_state(format!(
                "installation '{}' is already removed",
                id
            )));
        }

        self.run_hooks(&HookType::PreRemove)?;

        record.state = InstallationState::Removed;
        self.installed.write().insert(*id, record);

        self.run_hooks(&HookType::PostRemove)?;

        Ok(())
    }

    pub fn update(
        &self,
        id: &Uuid,
        new_manifest: MarketplaceManifest,
    ) -> CapabilityResult<()> {
        let mut record = {
            let installed = self.installed.read();
            installed.get(id).cloned().ok_or_else(|| {
                CapabilityError::not_found(format!("installation '{}' not found", id))
            })?
        };

        if record.state == InstallationState::Removed {
            return Err(CapabilityError::invalid_state(format!(
                "cannot update removed installation '{}'",
                id
            )));
        }

        new_manifest.validate()?;

        self.run_hooks(&HookType::PreUpdate)?;

        record.manifest = new_manifest;
        record.state = InstallationState::Updated;
        self.installed.write().insert(*id, record);

        self.run_hooks(&HookType::PostUpdate)?;

        Ok(())
    }

    pub fn search(&self, query: &str) -> Vec<MarketplaceManifest> {
        let q = query.to_lowercase();
        let registry = self.registry.read();
        registry
            .values()
            .filter(|m| {
                m.name.to_lowercase().contains(&q)
                    || m.description.to_lowercase().contains(&q)
                    || m.keywords.iter().any(|k| k.to_lowercase().contains(&q))
            })
            .cloned()
            .collect()
    }

    pub fn get_installed(&self) -> Vec<InstallationRecord> {
        self.installed.read().values().cloned().collect()
    }

    pub fn get_manifest(&self, name: &str) -> Option<MarketplaceManifest> {
        self.registry.read().get(name).cloned()
    }

    pub fn register_hook(&self, hook_type: HookType, hook: InstallationHook) {
        self.hooks
            .write()
            .entry(hook_type)
            .or_insert_with(Vec::new)
            .push(hook);
    }

    pub fn verify_signature(&self, id: &Uuid) -> CapabilityResult<bool> {
        let sig = {
            let signatures = self.signatures.read();
            signatures.get(id).cloned().ok_or_else(|| {
                CapabilityError::not_found(format!(
                    "no signature for installation '{}'",
                    id
                ))
            })?
        };

        let record = {
            let installed = self.installed.read();
            installed.get(id).cloned().ok_or_else(|| {
                CapabilityError::not_found(format!("installation '{}' not found", id))
            })?
        };

        let mut sig = sig;
        sig.verify(&record.checksum)?;
        self.signatures.write().insert(*id, sig.clone());
        Ok(sig.is_valid())
    }

    pub fn verify_integrity(&self, id: &Uuid) -> CapabilityResult<bool> {
        let record = {
            let installed = self.installed.read();
            installed.get(id).cloned().ok_or_else(|| {
                CapabilityError::not_found(format!("installation '{}' not found", id))
            })?
        };
        Ok(record.checksum == record.manifest.checksum && !record.checksum.is_empty())
    }

    pub fn export_manifest(&self, name: &str) -> CapabilityResult<String> {
        let manifest = {
            let registry = self.registry.read();
            registry.get(name).cloned().ok_or_else(|| {
                CapabilityError::not_found(format!("manifest '{}' not found", name))
            })?
        };
        serde_json::to_string_pretty(&manifest).map_err(|e| {
            CapabilityError::marketplace(format!("failed to serialize manifest: {}", e))
        })
    }

    pub fn import_manifest(&self, json: &str) -> CapabilityResult<()> {
        let manifest: MarketplaceManifest = serde_json::from_str(json).map_err(|e| {
            CapabilityError::marketplace(format!("failed to deserialize manifest: {}", e))
        })?;
        self.publish(manifest)
    }

    fn run_hooks(&self, hook_type: &HookType) -> CapabilityResult<()> {
        let hooks = self.hooks.read();
        if let Some(hook_list) = hooks.get(hook_type) {
            for hook in hook_list {
                hook.execute()?;
            }
        }
        Ok(())
    }
}

impl Default for Marketplace {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest(name: &str) -> MarketplaceManifest {
        MarketplaceManifest::new(
            name,
            CapabilityVersion::new(1, 2, 3),
            format!("A test capability called {}", name),
            "test-author",
            CapabilityCategory::Tool,
        )
    }

    fn sample_manifest_v2(name: &str) -> MarketplaceManifest {
        MarketplaceManifest::new(
            name,
            CapabilityVersion::new(2, 0, 0),
            format!("Updated capability {}", name),
            "test-author",
            CapabilityCategory::Inference,
        )
    }

    // -- MarketplaceManifest tests ------------------------------------------

    #[test]
    fn marketplace_manifest_creation() {
        let m = sample_manifest("my-cap");
        assert_eq!(m.name, "my-cap");
        assert_eq!(m.version, CapabilityVersion::new(1, 2, 3));
        assert_eq!(m.author, "test-author");
        assert_eq!(m.downloads_count, 0);
        assert_eq!(m.rating, 0.0);
        assert!(m.signature.is_none());
    }

    #[test]
    fn marketplace_manifest_validate_success() {
        let m = sample_manifest("valid-cap");
        assert!(m.validate().is_ok());
    }

    #[test]
    fn marketplace_manifest_validate_empty_name() {
        let mut m = sample_manifest("ok");
        m.name = String::new();
        assert!(m.validate().is_err());
    }

    #[test]
    fn marketplace_manifest_validate_empty_description() {
        let mut m = sample_manifest("ok");
        m.description = String::new();
        assert!(m.validate().is_err());
    }

    #[test]
    fn marketplace_manifest_validate_empty_author() {
        let mut m = sample_manifest("ok");
        m.author = String::new();
        assert!(m.validate().is_err());
    }

    #[test]
    fn marketplace_manifest_validate_bad_rating() {
        let mut m = sample_manifest("ok");
        m.rating = -1.0;
        assert!(m.validate().is_err());
        m.rating = 6.0;
        assert!(m.validate().is_err());
    }

    #[test]
    fn marketplace_manifest_validate_empty_dep_name() {
        let mut m = sample_manifest("ok");
        m.dependencies.push(MarketplaceDependency {
            name: String::new(),
            version: CapabilityVersion::initial(),
            optional: false,
        });
        assert!(m.validate().is_err());
    }

    #[test]
    fn marketplace_manifest_compute_checksum() {
        let mut m = sample_manifest("chk-cap");
        assert!(m.checksum.is_empty());
        m.compute_checksum();
        assert!(!m.checksum.is_empty());
        assert_eq!(m.checksum.len(), 64);
    }

    #[test]
    fn marketplace_manifest_checksum_deterministic() {
        let mut m1 = sample_manifest("same");
        let mut m2 = sample_manifest("same");
        m1.compute_checksum();
        m2.compute_checksum();
        assert_eq!(m1.checksum, m2.checksum);
    }

    #[test]
    fn marketplace_manifest_checksum_differs_for_different_data() {
        let mut m1 = sample_manifest("cap-a");
        let mut m2 = sample_manifest("cap-b");
        m1.compute_checksum();
        m2.compute_checksum();
        assert_ne!(m1.checksum, m2.checksum);
    }

    #[test]
    fn marketplace_manifest_to_manifest() {
        let mut m = sample_manifest("to-manifest");
        m.keywords.push("ai".to_string());
        m.keywords.push("tool".to_string());
        m.permissions.push("read".to_string());
        m.min_neo_version = CapabilityVersion::new(2, 0, 0);
        m.dependencies.push(MarketplaceDependency {
            name: "dep-a".to_string(),
            version: CapabilityVersion::new(1, 0, 0),
            optional: false,
        });

        let cap_manifest = m.to_manifest();
        assert_eq!(cap_manifest.name, "to-manifest");
        assert_eq!(cap_manifest.version, "1.2.3");
        assert_eq!(cap_manifest.author, "test-author");
        assert_eq!(cap_manifest.tags, vec!["ai", "tool"]);
        assert_eq!(cap_manifest.permissions, vec!["read"]);
        assert_eq!(cap_manifest.min_neo_version, "2.0.0");
        assert_eq!(cap_manifest.dependencies.len(), 1);
        assert_eq!(cap_manifest.dependencies[0].name, "dep-a");
        assert!(!cap_manifest.dependencies[0].optional);
    }

    #[test]
    fn marketplace_manifest_serialization_roundtrip() {
        let mut m = sample_manifest("serialize-test");
        m.repository_url = "https://github.com/test/repo".to_string();
        m.homepage = "https://example.com".to_string();
        m.keywords = vec!["test".to_string(), "serialization".to_string()];
        m.rating = 4.5;
        m.downloads_count = 1000;
        m.maintainers = vec!["alice".to_string(), "bob".to_string()];
        m.changelog = "v1.2.3: initial release".to_string();
        m.icon_url = "https://example.com/icon.png".to_string();

        let json = serde_json::to_string(&m).unwrap();
        let restored: MarketplaceManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.name, m.name);
        assert_eq!(restored.version, m.version);
        assert_eq!(restored.repository_url, m.repository_url);
        assert_eq!(restored.homepage, m.homepage);
        assert_eq!(restored.keywords, m.keywords);
        assert_eq!(restored.rating, m.rating);
        assert_eq!(restored.downloads_count, m.downloads_count);
        assert_eq!(restored.maintainers, m.maintainers);
        assert_eq!(restored.changelog, m.changelog);
        assert_eq!(restored.icon_url, m.icon_url);
    }

    // -- Signature tests ----------------------------------------------------

    #[test]
    fn signature_creation_and_verification() {
        let manifest_hash = "abc123def456".to_string();
        let signing_info = SigningInfo {
            signer_id: "signer-1".to_string(),
            signature_algorithm: SignatureAlgorithm::Ed25519,
            public_key: "pk_abcdef".to_string(),
            signature: "sig_abcdef".to_string(),
            signed_at: Utc::now(),
        };

        let mut sig = CapabilitySignature::new(manifest_hash.clone(), signing_info);
        assert!(!sig.is_valid());

        sig.verify(&manifest_hash).unwrap();
        assert!(sig.is_valid());
    }

    #[test]
    fn signature_verify_hash_mismatch() {
        let signing_info = SigningInfo {
            signer_id: "signer-1".to_string(),
            signature_algorithm: SignatureAlgorithm::Sha256WithRsa,
            public_key: "pk_rsa".to_string(),
            signature: "sig_rsa".to_string(),
            signed_at: Utc::now(),
        };

        let mut sig = CapabilitySignature::new("correct_hash".to_string(), signing_info);
        let result = sig.verify("wrong_hash");
        assert!(result.is_err());
        assert!(!sig.is_valid());
    }

    #[test]
    fn signature_verify_empty_signature() {
        let signing_info = SigningInfo {
            signer_id: "signer-1".to_string(),
            signature_algorithm: SignatureAlgorithm::EcdsaP256,
            public_key: "pk_ecdsa".to_string(),
            signature: String::new(),
            signed_at: Utc::now(),
        };

        let mut sig = CapabilitySignature::new("hash".to_string(), signing_info);
        let result = sig.verify("hash");
        assert!(result.is_err());
        assert!(!sig.is_valid());
    }

    #[test]
    fn signature_verify_empty_public_key() {
        let signing_info = SigningInfo {
            signer_id: "signer-1".to_string(),
            signature_algorithm: SignatureAlgorithm::Ed25519,
            public_key: String::new(),
            signature: "sig_data".to_string(),
            signed_at: Utc::now(),
        };

        let mut sig = CapabilitySignature::new("hash".to_string(), signing_info);
        let result = sig.verify("hash");
        assert!(result.is_err());
        assert!(!sig.is_valid());
    }

    #[test]
    fn signature_algorithm_variants() {
        let algs = [
            SignatureAlgorithm::Ed25519,
            SignatureAlgorithm::Sha256WithRsa,
            SignatureAlgorithm::EcdsaP256,
        ];
        for alg in &algs {
            let info = SigningInfo {
                signer_id: "s".to_string(),
                signature_algorithm: *alg,
                public_key: "pk".to_string(),
                signature: "sig".to_string(),
                signed_at: Utc::now(),
            };
            let mut sig = CapabilitySignature::new("h".to_string(), info);
            sig.verify("h").unwrap();
            assert!(sig.is_valid());
        }
    }

    // -- VersionCompatibility tests -----------------------------------------

    #[test]
    fn version_compatibility_compatible() {
        let compat = VersionCompatibility::new(
            CapabilityVersion::new(1, 0, 0),
            Some(CapabilityVersion::new(2, 0, 0)),
        );
        let neo = CapabilityVersion::new(1, 5, 0);
        assert!(compat.is_compatible(&neo, &HashMap::new()));
    }

    #[test]
    fn version_compatibility_below_minimum() {
        let compat = VersionCompatibility::new(CapabilityVersion::new(2, 0, 0), None);
        let neo = CapabilityVersion::new(1, 9, 9);
        assert!(!compat.is_compatible(&neo, &HashMap::new()));
    }

    #[test]
    fn version_compatibility_above_maximum() {
        let compat = VersionCompatibility::new(
            CapabilityVersion::new(1, 0, 0),
            Some(CapabilityVersion::new(1, 5, 0)),
        );
        let neo = CapabilityVersion::new(2, 0, 0);
        assert!(!compat.is_compatible(&neo, &HashMap::new()));
    }

    #[test]
    fn version_compatibility_with_deps_met() {
        let mut compat = VersionCompatibility::new(CapabilityVersion::new(1, 0, 0), None);
        compat
            .required_dependencies
            .push(("core-lib".to_string(), CapabilityVersion::new(1, 0, 0)));

        let mut available = HashMap::new();
        available.insert("core-lib".to_string(), CapabilityVersion::new(1, 2, 0));

        let neo = CapabilityVersion::new(1, 0, 0);
        assert!(compat.is_compatible(&neo, &available));
    }

    #[test]
    fn version_compatibility_with_deps_missing() {
        let mut compat = VersionCompatibility::new(CapabilityVersion::new(1, 0, 0), None);
        compat
            .required_dependencies
            .push(("missing-lib".to_string(), CapabilityVersion::new(1, 0, 0)));

        let neo = CapabilityVersion::new(1, 0, 0);
        let available = HashMap::new();
        assert!(!compat.is_compatible(&neo, &available));
    }

    #[test]
    fn version_compatibility_with_deps_version_mismatch() {
        let mut compat = VersionCompatibility::new(CapabilityVersion::new(1, 0, 0), None);
        compat
            .required_dependencies
            .push(("lib".to_string(), CapabilityVersion::new(2, 0, 0)));

        let mut available = HashMap::new();
        available.insert("lib".to_string(), CapabilityVersion::new(1, 0, 0));

        let neo = CapabilityVersion::new(1, 0, 0);
        assert!(!compat.is_compatible(&neo, &available));
    }

    #[test]
    fn version_compatibility_check_error_messages() {
        let compat = VersionCompatibility::new(CapabilityVersion::new(3, 0, 0), None);
        let neo = CapabilityVersion::new(1, 0, 0);
        let result = compat.check_compatibility(&neo, &HashMap::new());
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("below minimum"));
    }

    // -- InstallationRecord tests -------------------------------------------

    #[test]
    fn installation_record_creation() {
        let manifest = sample_manifest("inst-cap");
        let record = InstallationRecord::new(manifest.clone(), InstallationSource::Local);
        assert_eq!(record.manifest.name, "inst-cap");
        assert_eq!(record.state, InstallationState::Installing);
        assert!(!record.signature_verified);
        assert!(record.checksum.is_empty());
    }

    #[test]
    fn installation_record_is_valid() {
        let manifest = sample_manifest("valid");
        let mut record = InstallationRecord::new(manifest, InstallationSource::Local);
        record.state = InstallationState::Installing;
        assert!(record.is_valid());

        record.state = InstallationState::Installed;
        assert!(record.is_valid());

        record.state = InstallationState::Updated;
        assert!(record.is_valid());

        record.state = InstallationState::Failed;
        assert!(!record.is_valid());

        record.state = InstallationState::Removed;
        assert!(!record.is_valid());
    }

    #[test]
    fn installation_record_source_variants() {
        let manifest = sample_manifest("src");

        let local = InstallationRecord::new(manifest.clone(), InstallationSource::Local);
        assert!(matches!(local.source, InstallationSource::Local));

        let reg = InstallationRecord::new(
            manifest.clone(),
            InstallationSource::Registry("https://registry.neo.io".to_string()),
        );
        assert!(matches!(reg.source, InstallationSource::Registry(_)));

        let git = InstallationRecord::new(
            manifest,
            InstallationSource::Git("https://github.com/test/repo".to_string()),
        );
        assert!(matches!(git.source, InstallationSource::Git(_)));
    }

    #[test]
    fn installation_record_serialization() {
        let manifest = sample_manifest("ser-inst");
        let record = InstallationRecord::new(manifest, InstallationSource::Local);
        let json = serde_json::to_string(&record).unwrap();
        let restored: InstallationRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, record.id);
        assert_eq!(restored.manifest.name, "ser-inst");
    }

    // -- InstallationHook tests ---------------------------------------------

    #[test]
    fn hook_creation() {
        let hook = InstallationHook::new(HookType::PreInstall, "echo pre-install", 5000);
        assert_eq!(hook.hook_type, HookType::PreInstall);
        assert_eq!(hook.command, "echo pre-install");
        assert_eq!(hook.timeout_ms, 5000);
        assert!(hook.script_path.is_none());
    }

    #[test]
    fn hook_with_script_path() {
        let hook = InstallationHook::new(HookType::PostInstall, "run.sh", 10000)
            .with_script_path(PathBuf::from("/opt/scripts/run.sh"));
        assert!(hook.script_path.is_some());
        assert_eq!(
            hook.script_path.as_ref().unwrap(),
            &PathBuf::from("/opt/scripts/run.sh")
        );
    }

    #[test]
    fn hook_execute_success() {
        let hook = InstallationHook::new(HookType::PreInstall, "echo ok", 5000);
        assert!(hook.execute().is_ok());
    }

    #[test]
    fn hook_execute_with_script() {
        let hook = InstallationHook::new(HookType::PostRemove, "cleanup.sh", 3000)
            .with_script_path(PathBuf::from("/tmp/cleanup.sh"));
        assert!(hook.execute().is_ok());
    }

    #[test]
    fn hook_type_all_variants() {
        let types = [
            HookType::PreInstall,
            HookType::PostInstall,
            HookType::PreRemove,
            HookType::PostRemove,
            HookType::PreUpdate,
            HookType::PostUpdate,
        ];
        assert_eq!(types.len(), 6);
    }

    #[test]
    fn hook_type_equality() {
        assert_eq!(HookType::PreInstall, HookType::PreInstall);
        assert_ne!(HookType::PreInstall, HookType::PostInstall);
    }

    #[test]
    fn hook_serialization() {
        let hook = InstallationHook::new(HookType::PreUpdate, "test-cmd", 2000);
        let json = serde_json::to_string(&hook).unwrap();
        let restored: InstallationHook = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.hook_type, HookType::PreUpdate);
        assert_eq!(restored.command, "test-cmd");
        assert_eq!(restored.timeout_ms, 2000);
    }

    // -- Marketplace tests --------------------------------------------------

    #[test]
    fn marketplace_creation() {
        let mp = Marketplace::new();
        assert!(mp.get_installed().is_empty());
        assert!(mp.get_manifest("nonexistent").is_none());
    }

    #[test]
    fn marketplace_default() {
        let mp = Marketplace::default();
        assert!(mp.get_installed().is_empty());
    }

    #[test]
    fn marketplace_publish_and_get() {
        let mp = Marketplace::new();
        let manifest = sample_manifest("pub-cap");
        mp.publish(manifest).unwrap();

        let retrieved = mp.get_manifest("pub-cap");
        assert!(retrieved.is_some());
        let m = retrieved.unwrap();
        assert_eq!(m.name, "pub-cap");
        assert!(!m.checksum.is_empty());
    }

    #[test]
    fn marketplace_publish_validates() {
        let mp = Marketplace::new();
        let mut manifest = sample_manifest("bad");
        manifest.name = String::new();
        assert!(mp.publish(manifest).is_err());
    }

    #[test]
    fn marketplace_publish_overwrites() {
        let mp = Marketplace::new();
        let mut m1 = sample_manifest("overwrite");
        m1.version = CapabilityVersion::new(1, 0, 0);
        mp.publish(m1).unwrap();

        let mut m2 = sample_manifest("overwrite");
        m2.version = CapabilityVersion::new(2, 0, 0);
        mp.publish(m2).unwrap();

        let m = mp.get_manifest("overwrite").unwrap();
        assert_eq!(m.version, CapabilityVersion::new(2, 0, 0));
    }

    #[test]
    fn marketplace_install_success() {
        let mp = Marketplace::new();
        mp.publish(sample_manifest("install-me")).unwrap();

        let id = mp
            .install(
                "install-me",
                &CapabilityVersion::new(1, 2, 3),
                InstallationSource::Local,
                &CapabilityVersion::new(1, 0, 0),
            )
            .unwrap();

        let installed = mp.get_installed();
        assert_eq!(installed.len(), 1);
        assert_eq!(installed[0].id, id);
        assert_eq!(installed[0].state, InstallationState::Installed);
    }

    #[test]
    fn marketplace_install_not_found() {
        let mp = Marketplace::new();
        let result = mp.install(
            "nonexistent",
            &CapabilityVersion::new(1, 0, 0),
            InstallationSource::Local,
            &CapabilityVersion::new(1, 0, 0),
        );
        assert!(result.is_err());
    }

    #[test]
    fn marketplace_install_wrong_version() {
        let mp = Marketplace::new();
        mp.publish(sample_manifest("ver-cap")).unwrap();

        let result = mp.install(
            "ver-cap",
            &CapabilityVersion::new(9, 9, 9),
            InstallationSource::Local,
            &CapabilityVersion::new(1, 0, 0),
        );
        assert!(result.is_err());
    }

    #[test]
    fn marketplace_install_version_incompatible_with_neo() {
        let mp = Marketplace::new();
        let mut manifest = sample_manifest("neo-compat");
        manifest.min_neo_version = CapabilityVersion::new(5, 0, 0);
        mp.publish(manifest).unwrap();

        let result = mp.install(
            "neo-compat",
            &CapabilityVersion::new(1, 2, 3),
            InstallationSource::Local,
            &CapabilityVersion::new(1, 0, 0),
        );
        assert!(result.is_err());
    }

    #[test]
    fn marketplace_install_with_hooks() {
        let mp = Marketplace::new();
        mp.publish(sample_manifest("hooked-cap")).unwrap();

        mp.register_hook(
            HookType::PreInstall,
            InstallationHook::new(HookType::PreInstall, "pre-hook", 1000),
        );
        mp.register_hook(
            HookType::PostInstall,
            InstallationHook::new(HookType::PostInstall, "post-hook", 1000),
        );

        let result = mp.install(
            "hooked-cap",
            &CapabilityVersion::new(1, 2, 3),
            InstallationSource::Local,
            &CapabilityVersion::new(1, 0, 0),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn marketplace_install_from_registry() {
        let mp = Marketplace::new();
        mp.publish(sample_manifest("reg-cap")).unwrap();

        let id = mp
            .install(
                "reg-cap",
                &CapabilityVersion::new(1, 2, 3),
                InstallationSource::Registry("https://registry.neo.io".to_string()),
                &CapabilityVersion::new(1, 0, 0),
            )
            .unwrap();

        let record = mp.get_installed().into_iter().find(|r| r.id == id).unwrap();
        assert!(matches!(record.source, InstallationSource::Registry(_)));
    }

    #[test]
    fn marketplace_install_from_git() {
        let mp = Marketplace::new();
        mp.publish(sample_manifest("git-cap")).unwrap();

        let id = mp
            .install(
                "git-cap",
                &CapabilityVersion::new(1, 2, 3),
                InstallationSource::Git("https://github.com/test/repo".to_string()),
                &CapabilityVersion::new(1, 0, 0),
            )
            .unwrap();

        let record = mp.get_installed().into_iter().find(|r| r.id == id).unwrap();
        assert!(matches!(record.source, InstallationSource::Git(_)));
    }

    #[test]
    fn marketplace_uninstall_success() {
        let mp = Marketplace::new();
        mp.publish(sample_manifest("uninst")).unwrap();
        let id = mp
            .install(
                "uninst",
                &CapabilityVersion::new(1, 2, 3),
                InstallationSource::Local,
                &CapabilityVersion::new(1, 0, 0),
            )
            .unwrap();

        mp.uninstall(&id).unwrap();

        let record = mp
            .get_installed()
            .into_iter()
            .find(|r| r.id == id)
            .unwrap();
        assert_eq!(record.state, InstallationState::Removed);
    }

    #[test]
    fn marketplace_uninstall_not_found() {
        let mp = Marketplace::new();
        let fake_id = Uuid::new_v4();
        assert!(mp.uninstall(&fake_id).is_err());
    }

    #[test]
    fn marketplace_uninstall_already_removed() {
        let mp = Marketplace::new();
        mp.publish(sample_manifest("double-rm")).unwrap();
        let id = mp
            .install(
                "double-rm",
                &CapabilityVersion::new(1, 2, 3),
                InstallationSource::Local,
                &CapabilityVersion::new(1, 0, 0),
            )
            .unwrap();

        mp.uninstall(&id).unwrap();
        assert!(mp.uninstall(&id).is_err());
    }

    #[test]
    fn marketplace_uninstall_with_hooks() {
        let mp = Marketplace::new();
        mp.publish(sample_manifest("rm-hooks")).unwrap();

        mp.register_hook(
            HookType::PreRemove,
            InstallationHook::new(HookType::PreRemove, "pre-rm", 500),
        );
        mp.register_hook(
            HookType::PostRemove,
            InstallationHook::new(HookType::PostRemove, "post-rm", 500),
        );

        let id = mp
            .install(
                "rm-hooks",
                &CapabilityVersion::new(1, 2, 3),
                InstallationSource::Local,
                &CapabilityVersion::new(1, 0, 0),
            )
            .unwrap();

        assert!(mp.uninstall(&id).is_ok());
    }

    #[test]
    fn marketplace_update_success() {
        let mp = Marketplace::new();
        mp.publish(sample_manifest("upd-cap")).unwrap();
        let id = mp
            .install(
                "upd-cap",
                &CapabilityVersion::new(1, 2, 3),
                InstallationSource::Local,
                &CapabilityVersion::new(1, 0, 0),
            )
            .unwrap();

        let new_manifest = sample_manifest_v2("upd-cap");
        mp.update(&id, new_manifest).unwrap();

        let record = mp
            .get_installed()
            .into_iter()
            .find(|r| r.id == id)
            .unwrap();
        assert_eq!(record.state, InstallationState::Updated);
        assert_eq!(record.manifest.version, CapabilityVersion::new(2, 0, 0));
    }

    #[test]
    fn marketplace_update_not_found() {
        let mp = Marketplace::new();
        let fake_id = Uuid::new_v4();
        let manifest = sample_manifest("nope");
        assert!(mp.update(&fake_id, manifest).is_err());
    }

    #[test]
    fn marketplace_update_removed_record() {
        let mp = Marketplace::new();
        mp.publish(sample_manifest("upd-removed")).unwrap();
        let id = mp
            .install(
                "upd-removed",
                &CapabilityVersion::new(1, 2, 3),
                InstallationSource::Local,
                &CapabilityVersion::new(1, 0, 0),
            )
            .unwrap();

        mp.uninstall(&id).unwrap();

        let manifest = sample_manifest_v2("upd-removed");
        assert!(mp.update(&id, manifest).is_err());
    }

    #[test]
    fn marketplace_update_invalid_manifest() {
        let mp = Marketplace::new();
        mp.publish(sample_manifest("upd-bad")).unwrap();
        let id = mp
            .install(
                "upd-bad",
                &CapabilityVersion::new(1, 2, 3),
                InstallationSource::Local,
                &CapabilityVersion::new(1, 0, 0),
            )
            .unwrap();

        let mut bad = sample_manifest("upd-bad");
        bad.name = String::new();
        assert!(mp.update(&id, bad).is_err());
    }

    #[test]
    fn marketplace_update_with_hooks() {
        let mp = Marketplace::new();
        mp.publish(sample_manifest("upd-hooks")).unwrap();

        mp.register_hook(
            HookType::PreUpdate,
            InstallationHook::new(HookType::PreUpdate, "pre-upd", 500),
        );
        mp.register_hook(
            HookType::PostUpdate,
            InstallationHook::new(HookType::PostUpdate, "post-upd", 500),
        );

        let id = mp
            .install(
                "upd-hooks",
                &CapabilityVersion::new(1, 2, 3),
                InstallationSource::Local,
                &CapabilityVersion::new(1, 0, 0),
            )
            .unwrap();

        let new_manifest = sample_manifest_v2("upd-hooks");
        assert!(mp.update(&id, new_manifest).is_ok());
    }

    #[test]
    fn marketplace_search_by_name() {
        let mp = Marketplace::new();
        mp.publish(sample_manifest("alpha-search")).unwrap();
        mp.publish(sample_manifest("beta-search")).unwrap();
        mp.publish(sample_manifest("gamma")).unwrap();

        let results = mp.search("search");
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|m| m.name.contains("search")));
    }

    #[test]
    fn marketplace_search_by_keyword() {
        let mp = Marketplace::new();
        let mut m1 = sample_manifest("ai-tool");
        m1.keywords.push("artificial-intelligence".to_string());
        mp.publish(m1).unwrap();

        let mut m2 = sample_manifest("web-tool");
        m2.keywords.push("web-scraping".to_string());
        mp.publish(m2).unwrap();

        let results = mp.search("artificial");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "ai-tool");
    }

    #[test]
    fn marketplace_search_by_description() {
        let mp = Marketplace::new();
        let mut m = sample_manifest("desc-search");
        m.description = "A capability for natural language processing".to_string();
        mp.publish(m).unwrap();

        let results = mp.search("natural language");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn marketplace_search_case_insensitive() {
        let mp = Marketplace::new();
        mp.publish(sample_manifest("MyCap")).unwrap();

        let results = mp.search("mycap");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn marketplace_search_no_results() {
        let mp = Marketplace::new();
        mp.publish(sample_manifest("something")).unwrap();
        assert!(mp.search("zzz-nonexistent-zzz").is_empty());
    }

    #[test]
    fn marketplace_search_empty_query() {
        let mp = Marketplace::new();
        mp.publish(sample_manifest("a")).unwrap();
        mp.publish(sample_manifest("b")).unwrap();

        let results = mp.search("");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn marketplace_register_and_run_hooks() {
        let mp = Marketplace::new();
        let hook = InstallationHook::new(HookType::PreInstall, "echo hook", 5000);
        mp.register_hook(HookType::PreInstall, hook);

        let hooks = mp.hooks.read();
        assert_eq!(hooks.get(&HookType::PreInstall).unwrap().len(), 1);
        assert!(hooks.get(&HookType::PostInstall).is_none());
    }

    #[test]
    fn marketplace_multiple_hooks_per_type() {
        let mp = Marketplace::new();
        mp.register_hook(
            HookType::PreInstall,
            InstallationHook::new(HookType::PreInstall, "hook1", 1000),
        );
        mp.register_hook(
            HookType::PreInstall,
            InstallationHook::new(HookType::PreInstall, "hook2", 2000),
        );

        let hooks = mp.hooks.read();
        assert_eq!(hooks.get(&HookType::PreInstall).unwrap().len(), 2);
    }

    #[test]
    fn marketplace_verify_signature_success() {
        let mp = Marketplace::new();
        let mut manifest = sample_manifest("sig-cap");
        manifest.compute_checksum();
        mp.publish(manifest).unwrap();

        let id = mp
            .install(
                "sig-cap",
                &CapabilityVersion::new(1, 2, 3),
                InstallationSource::Local,
                &CapabilityVersion::new(1, 0, 0),
            )
            .unwrap();

        let record = mp.installed.read().get(&id).cloned().unwrap();
        let signing_info = SigningInfo {
            signer_id: "neo-signer".to_string(),
            signature_algorithm: SignatureAlgorithm::Ed25519,
            public_key: "pk_test_key".to_string(),
            signature: "sig_test_data".to_string(),
            signed_at: Utc::now(),
        };

        let sig = CapabilitySignature::new(record.checksum.clone(), signing_info);
        mp.signatures.write().insert(id, sig);

        let result = mp.verify_signature(&id);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn marketplace_verify_signature_no_signature() {
        let mp = Marketplace::new();
        mp.publish(sample_manifest("no-sig")).unwrap();
        let id = mp
            .install(
                "no-sig",
                &CapabilityVersion::new(1, 2, 3),
                InstallationSource::Local,
                &CapabilityVersion::new(1, 0, 0),
            )
            .unwrap();

        assert!(mp.verify_signature(&id).is_err());
    }

    #[test]
    fn marketplace_verify_integrity_valid() {
        let mp = Marketplace::new();
        let mut manifest = sample_manifest("integ");
        manifest.compute_checksum();
        let expected_checksum = manifest.checksum.clone();
        mp.publish(manifest).unwrap();

        let id = mp
            .install(
                "integ",
                &CapabilityVersion::new(1, 2, 3),
                InstallationSource::Local,
                &CapabilityVersion::new(1, 0, 0),
            )
            .unwrap();

        {
            let mut installed = mp.installed.write();
            let record = installed.get_mut(&id).unwrap();
            record.checksum = expected_checksum;
        }

        assert!(mp.verify_integrity(&id).unwrap());
    }

    #[test]
    fn marketplace_verify_integrity_mismatch() {
        let mp = Marketplace::new();
        let mut manifest = sample_manifest("integ-mismatch");
        manifest.compute_checksum();
        mp.publish(manifest).unwrap();

        let id = mp
            .install(
                "integ-mismatch",
                &CapabilityVersion::new(1, 2, 3),
                InstallationSource::Local,
                &CapabilityVersion::new(1, 0, 0),
            )
            .unwrap();

        {
            let mut installed = mp.installed.write();
            let record = installed.get_mut(&id).unwrap();
            record.checksum = "tampered_checksum".to_string();
        }

        assert!(!mp.verify_integrity(&id).unwrap());
    }

    #[test]
    fn marketplace_verify_integrity_not_found() {
        let mp = Marketplace::new();
        let fake_id = Uuid::new_v4();
        assert!(mp.verify_integrity(&fake_id).is_err());
    }

    #[test]
    fn marketplace_export_manifest() {
        let mp = Marketplace::new();
        mp.publish(sample_manifest("export-cap")).unwrap();

        let json = mp.export_manifest("export-cap").unwrap();
        let parsed: MarketplaceManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "export-cap");
    }

    #[test]
    fn marketplace_export_manifest_not_found() {
        let mp = Marketplace::new();
        assert!(mp.export_manifest("nonexistent").is_err());
    }

    #[test]
    fn marketplace_import_manifest() {
        let mp = Marketplace::new();
        let manifest = sample_manifest("import-cap");
        let json = serde_json::to_string(&manifest).unwrap();

        mp.import_manifest(&json).unwrap();
        assert!(mp.get_manifest("import-cap").is_some());
    }

    #[test]
    fn marketplace_import_manifest_invalid_json() {
        let mp = Marketplace::new();
        assert!(mp.import_manifest("not valid json").is_err());
    }

    #[test]
    fn marketplace_export_import_roundtrip() {
        let mp = Marketplace::new();
        let mut manifest = sample_manifest("roundtrip");
        manifest.keywords.push("test".to_string());
        manifest.repository_url = "https://github.com/test/repo".to_string();
        manifest.rating = 4.8;
        manifest.downloads_count = 9999;
        mp.publish(manifest).unwrap();

        let json = mp.export_manifest("roundtrip").unwrap();

        let mp2 = Marketplace::new();
        mp2.import_manifest(&json).unwrap();

        let restored = mp2.get_manifest("roundtrip").unwrap();
        assert_eq!(restored.name, "roundtrip");
        assert_eq!(restored.keywords, vec!["test"]);
        assert_eq!(restored.rating, 4.8);
        assert_eq!(restored.downloads_count, 9999);
    }

    #[test]
    fn marketplace_full_lifecycle() {
        let mp = Marketplace::new();

        mp.publish(sample_manifest("lifecycle")).unwrap();
        assert!(mp.get_manifest("lifecycle").is_some());

        let id = mp
            .install(
                "lifecycle",
                &CapabilityVersion::new(1, 2, 3),
                InstallationSource::Registry("https://registry.neo.io".to_string()),
                &CapabilityVersion::new(1, 0, 0),
            )
            .unwrap();
        assert_eq!(mp.get_installed().len(), 1);

        let new_manifest = sample_manifest_v2("lifecycle");
        mp.update(&id, new_manifest).unwrap();
        let record = mp
            .get_installed()
            .into_iter()
            .find(|r| r.id == id)
            .unwrap();
        assert_eq!(record.state, InstallationState::Updated);

        mp.uninstall(&id).unwrap();
        let record = mp
            .get_installed()
            .into_iter()
            .find(|r| r.id == id)
            .unwrap();
        assert_eq!(record.state, InstallationState::Removed);
    }

    #[test]
    fn marketplace_multiple_installs() {
        let mp = Marketplace::new();
        mp.publish(sample_manifest("multi-a")).unwrap();
        mp.publish(sample_manifest("multi-b")).unwrap();
        mp.publish(sample_manifest("multi-c")).unwrap();

        let _id_a = mp
            .install(
                "multi-a",
                &CapabilityVersion::new(1, 2, 3),
                InstallationSource::Local,
                &CapabilityVersion::new(1, 0, 0),
            )
            .unwrap();
        let _id_b = mp
            .install(
                "multi-b",
                &CapabilityVersion::new(1, 2, 3),
                InstallationSource::Local,
                &CapabilityVersion::new(1, 0, 0),
            )
            .unwrap();
        let _id_c = mp
            .install(
                "multi-c",
                &CapabilityVersion::new(1, 2, 3),
                InstallationSource::Local,
                &CapabilityVersion::new(1, 0, 0),
            )
            .unwrap();

        assert_eq!(mp.get_installed().len(), 3);

        let results = mp.search("multi");
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn marketplace_search_multiple_matches() {
        let mp = Marketplace::new();
        let mut m1 = sample_manifest("x-ai");
        m1.keywords.push("deep-learning".to_string());
        mp.publish(m1).unwrap();

        let mut m2 = sample_manifest("y-ai");
        m2.keywords.push("machine-learning".to_string());
        mp.publish(m2).unwrap();

        let mut m3 = sample_manifest("z-ml");
        m3.description = "Machine Learning Pipeline".to_string();
        mp.publish(m3).unwrap();

        let results = mp.search("learning");
        assert_eq!(results.len(), 3);
    }
}
