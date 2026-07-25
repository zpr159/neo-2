use std::path::PathBuf;

use crate::types::WorldSnapshot;

/// Persists world model state to disk.
pub struct PersistenceManager {
    path: Option<PathBuf>,
    enabled: bool,
}

impl PersistenceManager {
    pub fn new(path: Option<String>) -> Self {
        let enabled = path.is_some();
        Self {
            path: path.map(PathBuf::from),
            enabled,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn save_snapshot(&self, snapshot: &WorldSnapshot) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }
        let path = self.path.as_ref().ok_or("no persistence path")?;
        std::fs::create_dir_all(path).map_err(|e| e.to_string())?;
        let filename = format!("snapshot_v{}.json", snapshot.version.0);
        let filepath = path.join(filename);
        let json = serde_json::to_string_pretty(snapshot).map_err(|e| e.to_string())?;
        std::fs::write(&filepath, json).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn load_latest_snapshot(&self) -> Result<Option<WorldSnapshot>, String> {
        if !self.enabled {
            return Ok(None);
        }
        let path = self.path.as_ref().ok_or("no persistence path")?;
        if !path.exists() {
            return Ok(None);
        }

        let mut files: Vec<PathBuf> = std::fs::read_dir(path)
            .map_err(|e| e.to_string())?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .map_or(false, |n| n.starts_with("snapshot_v") && n.ends_with(".json"))
            })
            .collect();

        files.sort();

        if let Some(latest) = files.last() {
            let content = std::fs::read_to_string(latest).map_err(|e| e.to_string())?;
            let snapshot: WorldSnapshot = serde_json::from_str(&content).map_err(|e| e.to_string())?;
            Ok(Some(snapshot))
        } else {
            Ok(None)
        }
    }

    pub fn save_state(&self, state: &serde_json::Value, label: &str) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }
        let path = self.path.as_ref().ok_or("no persistence path")?;
        std::fs::create_dir_all(path).map_err(|e| e.to_string())?;
        let filepath = path.join(format!("{label}.json"));
        let json = serde_json::to_string_pretty(state).map_err(|e| e.to_string())?;
        std::fs::write(&filepath, json).map_err(|e| e.to_string())?;
        Ok(())
    }
}

impl Default for PersistenceManager {
    fn default() -> Self {
        Self::new(None)
    }
}
