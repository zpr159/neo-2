use serde::{Deserialize, Serialize};

/// Supported optimizer algorithms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OptimizerType {
    SGD,
    Adam,
    AdamW,
    Adagrad,
    RMSProp,
    LAMB,
    LARS,
}

impl std::fmt::Display for OptimizerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OptimizerType::SGD => write!(f, "SGD"),
            OptimizerType::Adam => write!(f, "Adam"),
            OptimizerType::AdamW => write!(f, "AdamW"),
            OptimizerType::Adagrad => write!(f, "Adagrad"),
            OptimizerType::RMSProp => write!(f, "RMSProp"),
            OptimizerType::LAMB => write!(f, "LAMB"),
            OptimizerType::LARS => write!(f, "LARS"),
        }
    }
}

/// Configuration for an optimizer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizerConfig {
    pub optimizer_type: OptimizerType,
    pub learning_rate: f64,
    pub weight_decay: f64,
    pub beta1: Option<f64>,
    pub beta2: Option<f64>,
    pub epsilon: Option<f64>,
}

impl Default for OptimizerConfig {
    fn default() -> Self {
        Self {
            optimizer_type: OptimizerType::Adam,
            learning_rate: 0.001,
            weight_decay: 0.0,
            beta1: Some(0.9),
            beta2: Some(0.999),
            epsilon: Some(1e-8),
        }
    }
}

/// An optimizer instance wrapping its configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Optimizer {
    config: OptimizerConfig,
}

impl Optimizer {
    /// Creates a new optimizer with the given configuration.
    pub fn new(config: OptimizerConfig) -> Self {
        Self { config }
    }

    /// Returns a reference to the optimizer configuration.
    pub fn config(&self) -> &OptimizerConfig {
        &self.config
    }

    /// Returns the optimizer type.
    pub fn optimizer_type(&self) -> OptimizerType {
        self.config.optimizer_type
    }

    /// Returns the current learning rate.
    pub fn learning_rate(&self) -> f64 {
        self.config.learning_rate
    }
}
