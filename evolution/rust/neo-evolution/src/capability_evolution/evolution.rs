use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::config::EvolutionConfiguration;
use crate::error::{EvolutionError, EvolutionResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityVersion {
    pub capability_id: String,
    pub version: String,
    pub changes: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MigrationStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    RolledBack,
}

impl std::fmt::Display for MigrationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::InProgress => write!(f, "in_progress"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::RolledBack => write!(f, "rolled_back"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityMigration {
    pub capability_id: String,
    pub from_version: String,
    pub to_version: String,
    pub status: MigrationStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

pub struct CapabilityEvolution {
    versions: DashMap<String, Vec<CapabilityVersion>>,
    migrations: DashMap<String, Vec<CapabilityMigration>>,
    config: EvolutionConfiguration,
}

impl CapabilityEvolution {
    pub fn new(config: EvolutionConfiguration) -> Arc<Self> {
        Arc::new(Self {
            versions: DashMap::new(),
            migrations: DashMap::new(),
            config,
        })
    }

    pub fn create_version(
        &self,
        capability_id: impl Into<String>,
        version: impl Into<String>,
        changes: Vec<String>,
    ) -> CapabilityVersion {
        let cap_id = capability_id.into();
        let ver = CapabilityVersion {
            capability_id: cap_id.clone(),
            version: version.into(),
            changes,
            created_at: Utc::now(),
            active: true,
        };
        self.versions.entry(cap_id).or_default().push(ver.clone());
        ver
    }

    pub fn migrate(
        &self,
        capability_id: &str,
        to_version: &str,
    ) -> EvolutionResult<CapabilityMigration> {
        let versions = self
            .versions
            .get(capability_id)
            .ok_or_else(|| EvolutionError::NotFound(format!("capability {capability_id}")))?;

        let from_version = versions
            .iter()
            .find(|v| v.active)
            .map(|v| v.version.clone())
            .unwrap_or_else(|| "0.0.0".into());

        let migration = CapabilityMigration {
            capability_id: capability_id.to_string(),
            from_version,
            to_version: to_version.to_string(),
            status: MigrationStatus::InProgress,
            started_at: Some(Utc::now()),
            completed_at: None,
        };

        drop(versions);
        self.migrations
            .entry(capability_id.to_string())
            .or_default()
            .push(migration.clone());
        Ok(migration)
    }

    pub fn rollback_migration(&self, capability_id: &str) -> EvolutionResult<()> {
        if let Some(mut migrations) = self.migrations.get_mut(capability_id) {
            if let Some(last) = migrations.last_mut() {
                last.status = MigrationStatus::RolledBack;
                last.completed_at = Some(Utc::now());
            }
        }
        Ok(())
    }

    pub fn get_versions(&self, capability_id: &str) -> Vec<CapabilityVersion> {
        self.versions
            .get(capability_id)
            .map(|v| v.value().clone())
            .unwrap_or_default()
    }

    pub fn get_active_version(&self, capability_id: &str) -> Option<CapabilityVersion> {
        self.versions
            .get(capability_id)?
            .iter()
            .find(|v| v.active)
            .cloned()
    }

    pub fn optimize(&self, capability_id: &str) -> EvolutionResult<String> {
        let versions = self.get_versions(capability_id);
        if versions.is_empty() {
            return Err(EvolutionError::NotFound(format!(
                "no versions for {capability_id}"
            )));
        }
        Ok(format!(
            "capability {capability_id}: {} versions, latest = {}",
            versions.len(),
            versions.last().map_or("none", |v| &v.version)
        ))
    }
}
