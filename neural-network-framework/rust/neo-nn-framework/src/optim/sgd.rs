use std::collections::HashMap;
use neo_neural_engine::tensor::Tensor;

use crate::autograd::ADTensor;
use crate::optim::{Optimizer, OptimizerState};

#[derive(Debug)]
pub struct SGD {
    state: OptimizerState,
    momentum: f64,
    velocity: HashMap<String, Tensor>,
}

impl SGD {
    pub fn new(learning_rate: f64) -> Self {
        Self { state: OptimizerState::new(learning_rate), momentum: 0.0, velocity: HashMap::new() }
    }

    pub fn with_momentum(learning_rate: f64, momentum: f64) -> Self {
        Self { state: OptimizerState::new(learning_rate), momentum, velocity: HashMap::new() }
    }

    fn apply_update(param_data: &mut Tensor, grad: &Tensor, lr: f64) {
        let param_vals = param_data.to_vec_f64().unwrap_or_default();
        let grad_vals = grad.to_vec_f64().unwrap_or_default();
        let shape = param_data.shape().clone();
        let dtype = param_data.dtype();
        let mut new_vals = Vec::with_capacity(param_vals.len());
        for i in 0..param_vals.len() {
            let g = if i < grad_vals.len() { grad_vals[i] } else { 0.0 };
            new_vals.push(param_vals[i] - lr * g);
        }
        *param_data = Tensor::from_vec_f64(&new_vals, shape);
    }
}

impl Optimizer for SGD {
    fn step(&mut self, params: &mut HashMap<String, &mut ADTensor>, grads: &HashMap<String, Tensor>) {
        for (name, param) in params.iter_mut() {
            if let Some(grad) = grads.get(name) {
                if self.momentum > 0.0 {
                    let vel = self.velocity.entry(name.clone()).or_insert_with(|| {
                        Tensor::zeros(param.shape().clone(), param.dtype())
                    });
                    let vel_vals = vel.to_vec_f64().unwrap_or_default();
                    let grad_vals = grad.to_vec_f64().unwrap_or_default();
                    let shape = vel.shape().clone();
                    let dtype = vel.dtype();
                    let mut new_vel = Vec::with_capacity(vel_vals.len());
                    let mut new_param = Vec::with_capacity(vel_vals.len());
                    let p_vals = param.data().to_vec_f64().unwrap_or_default();
                    for i in 0..vel_vals.len() {
                        let g = if i < grad_vals.len() { grad_vals[i] } else { 0.0 };
                        let v = self.momentum * vel_vals[i] + g;
                        new_vel.push(v);
                        new_param.push(p_vals[i] - self.state.learning_rate * v);
                    }
                    *vel = Tensor::from_vec_f64(&new_vel, shape);
                    let p_shape = param.data().shape().clone();
                    *param.data_mut() = Tensor::from_vec_f64(&new_param, p_shape);
                } else {
                    Self::apply_update(param.data_mut(), grad, self.state.learning_rate);
                }
            }
        }
        self.state.step_count += 1;
    }

    fn learning_rate(&self) -> f64 { self.state.learning_rate }

    fn set_learning_rate(&mut self, lr: f64) { self.state.learning_rate = lr; }

    fn state_dict(&self) -> HashMap<String, f64> {
        let mut s = HashMap::new();
        s.insert("learning_rate".to_string(), self.state.learning_rate);
        s.insert("momentum".to_string(), self.momentum);
        s
    }
}

#[derive(Debug)]
pub struct MomentumSGD {
    inner: SGD,
}

impl MomentumSGD {
    pub fn new(learning_rate: f64, momentum: f64) -> Self {
        Self { inner: SGD::with_momentum(learning_rate, momentum) }
    }
}

impl Optimizer for MomentumSGD {
    fn step(&mut self, params: &mut HashMap<String, &mut ADTensor>, grads: &HashMap<String, Tensor>) {
        self.inner.step(params, grads);
    }

    fn learning_rate(&self) -> f64 { self.inner.learning_rate() }

    fn set_learning_rate(&mut self, lr: f64) { self.inner.set_learning_rate(lr); }

    fn state_dict(&self) -> HashMap<String, f64> { self.inner.state_dict() }
}

#[derive(Debug)]
pub struct NesterovSGD {
    inner: SGD,
}

impl NesterovSGD {
    pub fn new(learning_rate: f64, momentum: f64) -> Self {
        Self { inner: SGD::with_momentum(learning_rate, momentum) }
    }
}

impl Optimizer for NesterovSGD {
    fn step(&mut self, params: &mut HashMap<String, &mut ADTensor>, grads: &HashMap<String, Tensor>) {
        self.inner.step(params, grads);
    }

    fn learning_rate(&self) -> f64 { self.inner.learning_rate() }

    fn set_learning_rate(&mut self, lr: f64) { self.inner.set_learning_rate(lr); }

    fn state_dict(&self) -> HashMap<String, f64> { self.inner.state_dict() }
}
