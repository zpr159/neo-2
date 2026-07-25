use std::fmt;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use neo_core::error::{NeoError, NeoResult};

use crate::layer::LayerConfig;
use crate::loss::LossType;
use crate::optimizer::OptimizerConfig;

/// Metadata describing a model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub created_at: DateTime<Utc>,
    pub parameters_count: u64,
}

/// Full configuration for constructing a model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub layers: Vec<LayerConfig>,
    pub optimizer: OptimizerConfig,
    pub loss: LossType,
    pub metadata: ModelMetadata,
}

/// Lifecycle state of a model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModelState {
    Untrained,
    Training,
    Trained,
    Evaluating,
}

impl fmt::Display for ModelState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelState::Untrained => write!(f, "Untrained"),
            ModelState::Training => write!(f, "Training"),
            ModelState::Trained => write!(f, "Trained"),
            ModelState::Evaluating => write!(f, "Evaluating"),
        }
    }
}

/// A neural network model with configuration and state tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    id: Uuid,
    config: ModelConfig,
    state: ModelState,
    version: u32,
}

impl Model {
    /// Creates a new model in the Untrained state.
    pub fn new(config: ModelConfig) -> Self {
        Self {
            id: Uuid::new_v4(),
            config,
            state: ModelState::Untrained,
            version: 1,
        }
    }

    /// Returns the model's unique identifier.
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Returns the current lifecycle state.
    pub fn state(&self) -> ModelState {
        self.state
    }

    /// Returns the total number of parameters across all layers.
    pub fn parameter_count(&self) -> u64 {
        self.config
            .layers
            .iter()
            .map(|l| {
                l.input_size.unwrap_or(0) as u64 * l.output_size.unwrap_or(0) as u64
            })
            .sum()
    }

    /// Returns a reference to the model configuration.
    pub fn config(&self) -> &ModelConfig {
        &self.config
    }

    /// Returns the model version number.
    pub fn version(&self) -> u32 {
        self.version
    }

    /// Saves the model to disk (stub).
    pub async fn save(&self, path: &str) -> NeoResult<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(NeoError::Serialization)?;
        tokio::fs::write(path, json).await?;
        Ok(())
    }

    /// Loads a model from disk (stub).
    pub async fn load(path: &str) -> NeoResult<Self> {
        let data = tokio::fs::read_to_string(path).await?;
        let model: Self =
            serde_json::from_str(&data).map_err(NeoError::Serialization)?;
        Ok(model)
    }
}

impl fmt::Display for Model {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Model(name={}, state={}, params={}, v{})",
            self.config.metadata.name,
            self.state,
            self.parameter_count(),
            self.version,
        )
    }
}
