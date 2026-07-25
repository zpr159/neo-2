use std::collections::HashMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::autograd::ADTensor;
use crate::error::NnResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointMetadata {
    pub epoch: u32,
    pub step: u64,
    pub loss: f64,
    pub learning_rate: f64,
    pub timestamp: DateTime<Utc>,
    pub version: u32,
    pub custom: HashMap<String, String>,
}

impl Default for CheckpointMetadata {
    fn default() -> Self {
        Self {
            epoch: 0, step: 0, loss: 0.0, learning_rate: 0.0,
            timestamp: Utc::now(), version: 1, custom: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub metadata: CheckpointMetadata,
    pub parameters: HashMap<String, Vec<f64>>,
    pub parameter_shapes: HashMap<String, Vec<usize>>,
    pub optimizer_state: HashMap<String, f64>,
}

impl Checkpoint {
    pub fn new(metadata: CheckpointMetadata) -> Self {
        Self {
            metadata,
            parameters: HashMap::new(),
            parameter_shapes: HashMap::new(),
            optimizer_state: HashMap::new(),
        }
    }

    pub fn from_parameters(
        metadata: CheckpointMetadata,
        params: &HashMap<String, &ADTensor>,
        optimizer_state: &HashMap<String, f64>,
    ) -> Self {
        let mut parameters = HashMap::new();
        let mut parameter_shapes = HashMap::new();
        for (name, tensor) in params {
            if let Ok(vals) = tensor.to_vec_f64() {
                parameters.insert(name.clone(), vals);
                parameter_shapes.insert(name.clone(), tensor.shape().dims().to_vec());
            }
        }
        Self { metadata, parameters, parameter_shapes, optimizer_state: optimizer_state.clone() }
    }

    pub fn to_parameters(&self) -> NnResult<HashMap<String, ADTensor>> {
        let mut params = HashMap::new();
        for (name, vals) in &self.parameters {
            if let Some(shape) = self.parameter_shapes.get(name) {
                let tensor = neo_neural_engine::tensor::Tensor::from_vec_f64(vals, neo_neural_engine::shape::Shape::new(shape.clone()));
                params.insert(name.clone(), ADTensor::new(tensor, true));
            }
        }
        Ok(params)
    }
}

#[derive(Debug)]
pub struct CheckpointManager {
    save_dir: PathBuf,
    max_checkpoints: usize,
    checkpoints: Vec<CheckpointMetadata>,
}

impl CheckpointManager {
    pub fn new(save_dir: impl Into<PathBuf>, max_checkpoints: usize) -> Self {
        Self { save_dir: save_dir.into(), max_checkpoints, checkpoints: Vec::new() }
    }

    pub fn save(&mut self, checkpoint: &Checkpoint) -> NnResult<PathBuf> {
        let filename = format!("checkpoint_epoch_{}_step_{}.bin", checkpoint.metadata.epoch, checkpoint.metadata.step);
        let path = self.save_dir.join(&filename);
        std::fs::create_dir_all(&self.save_dir)?;
        let data = bincode::serialize(checkpoint)?;
        std::fs::write(&path, data)?;
        self.checkpoints.push(checkpoint.metadata.clone());
        self.cleanup_old()?;
        Ok(path)
    }

    pub fn load(&self, path: &Path) -> NnResult<Checkpoint> {
        let data = std::fs::read(path)?;
        let checkpoint: Checkpoint = bincode::deserialize(&data)?;
        Ok(checkpoint)
    }

    pub fn load_latest(&self) -> NnResult<Option<Checkpoint>> {
        let mut entries: Vec<_> = std::fs::read_dir(&self.save_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "bin"))
            .collect();
        entries.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
        if let Some(entry) = entries.first() {
            Ok(Some(self.load(&entry.path())?))
        } else {
            Ok(None)
        }
    }

    pub fn list_checkpoints(&self) -> &[CheckpointMetadata] {
        &self.checkpoints
    }

    fn cleanup_old(&mut self) -> NnResult<()> {
        if self.checkpoints.len() > self.max_checkpoints {
            let to_remove = self.checkpoints.len() - self.max_checkpoints;
            for _ in 0..to_remove {
                if let Some(removed) = self.checkpoints.first() {
                    let filename = format!("checkpoint_epoch_{}_step_{}.bin", removed.epoch, removed.step);
                    let path = self.save_dir.join(&filename);
                    let _ = std::fs::remove_file(path);
                    self.checkpoints.remove(0);
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct IncrementalCheckpoint {
    base_path: Option<PathBuf>,
    delta: HashMap<String, Vec<f64>>,
    delta_shapes: HashMap<String, Vec<usize>>,
}

impl IncrementalCheckpoint {
    pub fn new() -> Self {
        Self { base_path: None, delta: HashMap::new(), delta_shapes: HashMap::new() }
    }

    pub fn compute_delta(&mut self, current: &HashMap<String, &ADTensor>, previous: &HashMap<String, Vec<f64>>) {
        self.delta.clear();
        self.delta_shapes.clear();
        for (name, tensor) in current {
            if let Ok(vals) = tensor.to_vec_f64() {
                if let Some(prev_vals) = previous.get(name) {
                    if vals.len() == prev_vals.len() {
                        let diff: Vec<f64> = vals.iter().zip(prev_vals.iter()).map(|(a, b)| a - b).collect();
                        let dominated = diff.iter().all(|&d| d.abs() < 1e-10);
                        if !dominated {
                            self.delta.insert(name.clone(), diff);
                            self.delta_shapes.insert(name.clone(), tensor.shape().dims().to_vec());
                        }
                    } else {
                        self.delta.insert(name.clone(), vals);
                        self.delta_shapes.insert(name.clone(), tensor.shape().dims().to_vec());
                    }
                } else {
                    self.delta.insert(name.clone(), vals);
                    self.delta_shapes.insert(name.clone(), tensor.shape().dims().to_vec());
                }
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.delta.is_empty()
    }
}

impl Default for IncrementalCheckpoint {
    fn default() -> Self {
        Self::new()
    }
}
