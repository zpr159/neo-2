use std::collections::HashMap;

use crate::autograd::ADTensor;
use crate::error::NnResult;
use crate::module::{Module, TrainingMode};
use rand::Rng;

#[derive(Debug)]
pub struct Dropout {
    p: f64,
    mode: TrainingMode,
}

impl Dropout {
    pub fn new(p: f64) -> Self {
        Self { p, mode: TrainingMode::Train }
    }
}

impl Module for Dropout {
    fn forward(&self, input: &ADTensor) -> NnResult<ADTensor> {
        if self.mode == TrainingMode::Eval || self.p == 0.0 {
            return Ok(input.clone());
        }

        let ndim = input.ndim();
        let numel = input.numel();
        let input_data = input.to_vec_f64()?;
        let scale = 1.0 / (1.0 - self.p);
        let mut rng = rand::thread_rng();
        let mut result_data = Vec::with_capacity(numel);

        for i in 0..numel {
            if rng.gen::<f64>() >= self.p {
                result_data.push(input_data[i] * scale);
            } else {
                result_data.push(0.0);
            }
        }

        Ok(ADTensor::new(
            neo_neural_engine::tensor::Tensor::from_vec_f64(&result_data, input.shape().clone()),
            input.requires_grad(),
        ))
    }

    fn name(&self) -> &str {
        "Dropout"
    }

    fn set_mode(&mut self, mode: TrainingMode) {
        self.mode = mode;
    }

    fn mode(&self) -> TrainingMode {
        self.mode
    }
}

#[derive(Debug)]
pub struct AlphaDropout {
    p: f64,
    alpha: f64,
    scale: f64,
    mode: TrainingMode,
}

impl AlphaDropout {
    pub fn new(p: f64) -> Self {
        let alpha: f64 = 1.6732632423543772;
        let scale: f64 = 1.0507009873554805;
        let a_val = alpha * scale;
        let p_adj = p + (1.0 - p) * (1.0 - a_val).powi(2);
        let q = 1.0 - p_adj;
        let _ = (p_adj, q);
        Self {
            p,
            alpha: a_val,
            scale,
            mode: TrainingMode::Train,
        }
    }
}

impl Module for AlphaDropout {
    fn forward(&self, input: &ADTensor) -> NnResult<ADTensor> {
        if self.mode == TrainingMode::Eval || self.p == 0.0 {
            return Ok(input.clone());
        }

        let ndim = input.ndim();
        let numel = input.numel();
        let input_data = input.to_vec_f64()?;
        let mut rng = rand::thread_rng();
        let mut result_data = Vec::with_capacity(numel);

        let kept_prob = 1.0 - self.p;
        let alpha_sq = self.alpha * self.alpha;
        let a = (self.scale * kept_prob * (1.0 + self.alpha * self.alpha * kept_prob - 2.0 * self.alpha)).sqrt();
        let b = -a * self.alpha * kept_prob;

        for i in 0..numel {
            if rng.gen::<f64>() >= self.p {
                result_data.push(a * input_data[i] + b);
            } else {
                result_data.push(0.0);
            }
        }
        let _ = alpha_sq;

        Ok(ADTensor::new(
            neo_neural_engine::tensor::Tensor::from_vec_f64(&result_data, input.shape().clone()),
            input.requires_grad(),
        ))
    }

    fn name(&self) -> &str {
        "AlphaDropout"
    }

    fn set_mode(&mut self, mode: TrainingMode) {
        self.mode = mode;
    }

    fn mode(&self) -> TrainingMode {
        self.mode
    }
}
