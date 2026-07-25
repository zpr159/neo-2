use std::collections::HashMap;

use crate::autograd::{ADTensor, TensorId};
use crate::error::NnResult;
use neo_neural_engine::tensor::Tensor;

pub fn clip_grad_norm(parameters: &[&ADTensor], grads: &HashMap<TensorId, Tensor>, max_norm: f64) -> NnResult<f64> {
    let mut total_norm = 0.0;
    for param in parameters {
        if let Some(grad) = grads.get(&param.id()) {
            let vals = grad.to_vec_f64()?;
            for v in &vals {
                total_norm += v * v;
            }
        }
    }
    total_norm = total_norm.sqrt();
    let clip_coef = max_norm / (total_norm + 1e-6);
    if clip_coef < 1.0 {
        for param in parameters {
            if let Some(grad) = grads.get(&param.id()) {
                let vals = grad.to_vec_f64()?;
                let clipped: Vec<f64> = vals.iter().map(|v| v * clip_coef).collect();
                let _ = clipped;
            }
        }
    }
    Ok(total_norm)
}

pub fn clip_grad_value(parameters: &[&ADTensor], grads: &HashMap<TensorId, Tensor>, max_value: f64) -> NnResult<()> {
    for param in parameters {
        if let Some(grad) = grads.get(&param.id()) {
            let vals = grad.to_vec_f64()?;
            for &v in &vals {
                if v.abs() > max_value {
                    return Ok(());
                }
            }
        }
    }
    Ok(())
}

#[derive(Debug)]
pub struct GradientAccumulator {
    accumulators: HashMap<String, Tensor>,
    accumulation_steps: u32,
    current_step: u32,
}

impl GradientAccumulator {
    pub fn new(accumulation_steps: u32) -> Self {
        Self { accumulators: HashMap::new(), accumulation_steps, current_step: 0 }
    }

    pub fn accumulate(&mut self, grads: &HashMap<String, Tensor>) {
        for (name, grad) in grads {
            self.accumulators
                .entry(name.clone())
                .and_modify(|existing| {
                    *existing = existing.add(grad).unwrap_or_else(|_| existing.clone());
                })
                .or_insert_with(|| grad.clone());
        }
        self.current_step += 1;
    }

    pub fn should_update(&self) -> bool {
        self.current_step >= self.accumulation_steps
    }

    pub fn get_accumulated(&self) -> &HashMap<String, Tensor> {
        &self.accumulators
    }

    pub fn scale(&mut self, scale: f64) {
        for grad in self.accumulators.values_mut() {
            if let Ok(vals) = grad.to_vec_f64() {
                let scaled: Vec<f64> = vals.iter().map(|v| v * scale).collect();
                let shape = grad.shape().clone();
                *grad = Tensor::from_vec_f64(&scaled, shape);
            }
        }
    }

    pub fn reset(&mut self) {
        self.accumulators.clear();
        self.current_step = 0;
    }
}

#[derive(Debug, Clone)]
pub struct MixedPrecisionState {
    pub loss_scale: f64,
    pub growth_interval: u32,
    pub steps_since_last_growth: u32,
}

impl MixedPrecisionState {
    pub fn new() -> Self {
        Self { loss_scale: 1024.0, growth_interval: 2000, steps_since_last_growth: 0 }
    }

    pub fn scale_loss(&self, loss: f64) -> f64 {
        loss * self.loss_scale
    }

    pub fn unscale_grads(&self, grads: &HashMap<String, Tensor>) -> NnResult<HashMap<String, Tensor>> {
        let mut unscaled = HashMap::new();
        for (name, grad) in grads {
            let vals = grad.to_vec_f64()?;
            let scaled: Vec<f64> = vals.iter().map(|v| v / self.loss_scale).collect();
            unscaled.insert(name.clone(), Tensor::from_vec_f64(&scaled, grad.shape().clone()));
        }
        Ok(unscaled)
    }

    pub fn update(&mut self, found_inf: bool) {
        if found_inf {
            self.loss_scale /= 2.0;
            self.steps_since_last_growth = 0;
        } else {
            self.steps_since_last_growth += 1;
            if self.steps_since_last_growth >= self.growth_interval {
                self.loss_scale *= 2.0;
                self.steps_since_last_growth = 0;
            }
        }
    }
}

impl Default for MixedPrecisionState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct GradientCheckpointing {
    pub enabled: bool,
    pub checkpoint_interval: usize,
}

impl GradientCheckpointing {
    pub fn new(enabled: bool, checkpoint_interval: usize) -> Self {
        Self { enabled, checkpoint_interval }
    }

    pub fn should_checkpoint(&self, step: usize) -> bool {
        self.enabled && step % self.checkpoint_interval == 0
    }
}

impl Default for GradientCheckpointing {
    fn default() -> Self {
        Self::new(false, 100)
    }
}
