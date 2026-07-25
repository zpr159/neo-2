use std::collections::HashMap;
use serde::{Deserialize, Serialize};

pub use crate::model::QuantizationType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantizationConfig {
    pub target_type: QuantizationType,
    pub group_size: usize,
    pub symmetric: bool,
    pub calibration_samples: usize,
    pub dynamic: bool,
    pub mixed_precision: bool,
    pub layer_wise: bool,
    pub sparsity_threshold: f64,
    pub clip_ratio: f64,
}

impl Default for QuantizationConfig {
    fn default() -> Self {
        Self {
            target_type: QuantizationType::Int8,
            group_size: 128,
            symmetric: true,
            calibration_samples: 256,
            dynamic: false,
            mixed_precision: false,
            layer_wise: false,
            sparsity_threshold: 0.0,
            clip_ratio: 0.9999,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantizedWeight {
    pub shape: Vec<usize>,
    pub quant_type: QuantizationType,
    pub data: Vec<u8>,
    pub scale: Vec<f32>,
    pub zero_point: Option<Vec<i32>>,
    pub group_size: usize,
    pub original_dtype: String,
}

impl QuantizedWeight {
    #[must_use]
    pub fn compression_ratio(&self) -> f64 {
        let original_size: usize = self.shape.iter().product::<usize>() * 4;
        let quantized_size = self.data.len();
        if quantized_size > 0 {
            original_size as f64 / quantized_size as f64
        } else {
            1.0
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantizationResult {
    pub quant_type: QuantizationType,
    pub weights: HashMap<String, QuantizedWeight>,
    pub compression_ratio: f64,
    pub original_size: u64,
    pub quantized_size: u64,
    pub calibration_metrics: Option<QuantizationMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantizationMetrics {
    pub mean_squared_error: f64,
    pub max_error: f64,
    pub signal_to_noise_ratio: f64,
    pub perplexity_delta: Option<f64>,
    pub accuracy_delta: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QuantizationBackend {
    Cpu,
    Cuda,
    Rocm,
}

impl std::fmt::Display for QuantizationBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cpu => write!(f, "cpu"),
            Self::Cuda => write!(f, "cuda"),
            Self::Rocm => write!(f, "rocm"),
        }
    }
}

pub mod engine;
