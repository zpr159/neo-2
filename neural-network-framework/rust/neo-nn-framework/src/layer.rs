use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use neo_core::error::NeoResult;
use neo_neural_engine::tensor::TensorShape;

/// Supported layer types in the framework.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LayerType {
    Dense,
    Conv2d,
    Conv1d,
    MaxPool,
    AveragePool,
    BatchNorm,
    LayerNorm,
    Dropout,
    Attention,
    Embedding,
    LSTM,
    GRU,
    Transformer,
    Custom(String),
}

impl std::fmt::Display for LayerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LayerType::Dense => write!(f, "Dense"),
            LayerType::Conv2d => write!(f, "Conv2d"),
            LayerType::Conv1d => write!(f, "Conv1d"),
            LayerType::MaxPool => write!(f, "MaxPool"),
            LayerType::AveragePool => write!(f, "AveragePool"),
            LayerType::BatchNorm => write!(f, "BatchNorm"),
            LayerType::LayerNorm => write!(f, "LayerNorm"),
            LayerType::Dropout => write!(f, "Dropout"),
            LayerType::Attention => write!(f, "Attention"),
            LayerType::Embedding => write!(f, "Embedding"),
            LayerType::LSTM => write!(f, "LSTM"),
            LayerType::GRU => write!(f, "GRU"),
            LayerType::Transformer => write!(f, "Transformer"),
            LayerType::Custom(name) => write!(f, "Custom({})", name),
        }
    }
}

/// Configuration for a single layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerConfig {
    pub layer_type: LayerType,
    pub input_size: Option<usize>,
    pub output_size: Option<usize>,
    pub parameters: HashMap<String, serde_json::Value>,
}

/// A concrete layer instance within a model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    id: Uuid,
    config: LayerConfig,
    weight_shape: Option<TensorShape>,
}

impl Layer {
    /// Creates a new layer from its configuration.
    pub fn new(config: LayerConfig) -> Self {
        let weight_shape = match (&config.input_size, &config.output_size) {
            (Some(inp), Some(out)) => Some(vec![*inp, *out]),
            _ => None,
        };
        Self {
            id: Uuid::new_v4(),
            config,
            weight_shape,
        }
    }

    /// Returns the layer's unique identifier.
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Returns a rough estimate of the number of parameters.
    pub fn parameter_count(&self) -> u64 {
        match (&self.config.input_size, &self.config.output_size) {
            (Some(inp), Some(out)) => (*inp as u64) * (*out as u64),
            _ => 0,
        }
    }

    /// Computes the expected output shape given an input shape.
    pub fn output_shape(&self, input_shape: &TensorShape) -> Option<TensorShape> {
        match self.config.layer_type {
            LayerType::Dense => {
                self.config.output_size.map(|out| {
                    let mut shape = input_shape.clone();
                    if let Some(last) = shape.last_mut() {
                        *last = out;
                    } else {
                        shape.push(out);
                    }
                    shape
                })
            }
            LayerType::Conv2d => {
                // Simplified: assume same spatial dims with padding
                self.config.output_size.map(|c_out| {
                    let mut shape = input_shape.clone();
                    if shape.len() >= 3 {
                        shape[1] = c_out;
                    }
                    shape
                })
            }
            LayerType::MaxPool | LayerType::AveragePool => {
                // Halve spatial dimensions
                let mut shape = input_shape.clone();
                for dim in shape.iter_mut().skip(2) {
                    *dim /= 2;
                }
                Some(shape)
            }
            LayerType::Dropout | LayerType::BatchNorm | LayerType::LayerNorm => {
                Some(input_shape.clone())
            }
            _ => Some(input_shape.clone()),
        }
    }

    /// Returns a reference to the layer configuration.
    pub fn config(&self) -> &LayerConfig {
        &self.config
    }

    /// Returns the weight shape if known.
    pub fn weight_shape(&self) -> Option<&TensorShape> {
        self.weight_shape.as_ref()
    }
}
