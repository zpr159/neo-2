use std::collections::HashMap;
use neo_neural_engine::tensor::Tensor;

use crate::autograd::ADTensor;
use crate::optim::{Optimizer, OptimizerState};
use crate::optim::adam::Adam;

#[derive(Debug)]
pub struct RMSProp {
    state: OptimizerState,
    alpha: f64,
    eps: f64,
    weight_decay: f64,
    momentum: f64,
    cache: HashMap<String, Tensor>,
    velocity: HashMap<String, Tensor>,
}

impl RMSProp {
    pub fn new(learning_rate: f64) -> Self {
        Self {
            state: OptimizerState::new(learning_rate),
            alpha: 0.99, eps: 1e-8, weight_decay: 0.0, momentum: 0.0,
            cache: HashMap::new(), velocity: HashMap::new(),
        }
    }

    pub fn with_params(learning_rate: f64, alpha: f64, eps: f64, weight_decay: f64, momentum: f64) -> Self {
        Self {
            state: OptimizerState::new(learning_rate),
            alpha, eps, weight_decay, momentum,
            cache: HashMap::new(), velocity: HashMap::new(),
        }
    }
}

impl Optimizer for RMSProp {
    fn step(&mut self, params: &mut HashMap<String, &mut ADTensor>, grads: &HashMap<String, Tensor>) {
        for (name, param) in params.iter_mut() {
            if let Some(grad) = grads.get(name) {
                let cache = self.cache.entry(name.clone()).or_insert_with(|| {
                    Tensor::zeros(param.shape().clone(), param.dtype())
                });

                let p_vals = param.data().to_vec_f64().unwrap_or_default();
                let g_vals = grad.to_vec_f64().unwrap_or_default();
                let c_vals = cache.to_vec_f64().unwrap_or_default();

                if self.momentum > 0.0 {
                    let vel = self.velocity.entry(name.clone()).or_insert_with(|| {
                        Tensor::zeros(param.shape().clone(), param.dtype())
                    });
                    let v_vals = vel.to_vec_f64().unwrap_or_default();
                    let mut new_c = Vec::with_capacity(p_vals.len());
                    let mut new_v = Vec::with_capacity(p_vals.len());
                    let mut new_p = Vec::with_capacity(p_vals.len());

                    for i in 0..p_vals.len() {
                        let g = if i < g_vals.len() { g_vals[i] } else { 0.0 };
                        let gi = if self.weight_decay > 0.0 { g + self.weight_decay * p_vals[i] } else { g };
                        let ci = self.alpha * if i < c_vals.len() { c_vals[i] } else { 0.0 } + (1.0 - self.alpha) * gi * gi;
                        let vi = self.momentum * if i < v_vals.len() { v_vals[i] } else { 0.0 } + self.state.learning_rate * gi / (ci + self.eps).sqrt();
                        new_c.push(ci);
                        new_v.push(vi);
                        new_p.push(p_vals[i] - vi);
                    }

                    *cache = Tensor::from_vec_f64(&new_c, cache.shape().clone());
                    *vel = Tensor::from_vec_f64(&new_v, vel.shape().clone());
                    let p_shape = param.data().shape().clone();
                    *param.data_mut() = Tensor::from_vec_f64(&new_p, p_shape);
                } else {
                    let mut new_c = Vec::with_capacity(p_vals.len());
                    let mut new_p = Vec::with_capacity(p_vals.len());

                    for i in 0..p_vals.len() {
                        let g = if i < g_vals.len() { g_vals[i] } else { 0.0 };
                        let gi = if self.weight_decay > 0.0 { g + self.weight_decay * p_vals[i] } else { g };
                        let ci = self.alpha * if i < c_vals.len() { c_vals[i] } else { 0.0 } + (1.0 - self.alpha) * gi * gi;
                        new_c.push(ci);
                        new_p.push(p_vals[i] - self.state.learning_rate * gi / (ci + self.eps).sqrt());
                    }

                    *cache = Tensor::from_vec_f64(&new_c, cache.shape().clone());
                    let p_shape = param.data().shape().clone();
                    *param.data_mut() = Tensor::from_vec_f64(&new_p, p_shape);
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
        s.insert("alpha".to_string(), self.alpha);
        s.insert("eps".to_string(), self.eps);
        s.insert("weight_decay".to_string(), self.weight_decay);
        s.insert("momentum".to_string(), self.momentum);
        s
    }
}

#[derive(Debug)]
pub struct Lion {
    state: OptimizerState,
    beta1: f64,
    beta2: f64,
    weight_decay: f64,
    m: HashMap<String, Tensor>,
}

impl Lion {
    pub fn new(learning_rate: f64) -> Self {
        Self {
            state: OptimizerState::new(learning_rate),
            beta1: 0.9, beta2: 0.99, weight_decay: 1.0,
            m: HashMap::new(),
        }
    }
}

impl Optimizer for Lion {
    fn step(&mut self, params: &mut HashMap<String, &mut ADTensor>, grads: &HashMap<String, Tensor>) {
        for (name, param) in params.iter_mut() {
            if let Some(grad) = grads.get(name) {
                let m = self.m.entry(name.clone()).or_insert_with(|| {
                    Tensor::zeros(param.shape().clone(), param.dtype())
                });

                let p_vals = param.data().to_vec_f64().unwrap_or_default();
                let g_vals = grad.to_vec_f64().unwrap_or_default();
                let m_vals = m.to_vec_f64().unwrap_or_default();

                let mut new_m = Vec::with_capacity(p_vals.len());
                let mut new_p = Vec::with_capacity(p_vals.len());

                for i in 0..p_vals.len() {
                    let g = if i < g_vals.len() { g_vals[i] } else { 0.0 };
                    let mi = self.beta1 * if i < m_vals.len() { m_vals[i] } else { 0.0 } + (1.0 - self.beta1) * g;
                    new_m.push(mi);
                    let update = if mi > 0.0 { 1.0 } else if mi < 0.0 { -1.0 } else { 0.0 };
                    new_p.push(p_vals[i] * (1.0 - self.state.learning_rate * self.weight_decay) - self.state.learning_rate * update);
                }

                *m = Tensor::from_vec_f64(&new_m, m.shape().clone());
                let p_shape = param.data().shape().clone();
                *param.data_mut() = Tensor::from_vec_f64(&new_p, p_shape);
            }
        }
        self.state.step_count += 1;
    }

    fn learning_rate(&self) -> f64 { self.state.learning_rate }

    fn set_learning_rate(&mut self, lr: f64) { self.state.learning_rate = lr; }

    fn state_dict(&self) -> HashMap<String, f64> {
        let mut s = HashMap::new();
        s.insert("learning_rate".to_string(), self.state.learning_rate);
        s.insert("beta1".to_string(), self.beta1);
        s.insert("beta2".to_string(), self.beta2);
        s.insert("weight_decay".to_string(), self.weight_decay);
        s
    }
}

#[derive(Debug)]
pub struct LAMB {
    inner: Adam,
    trust_ratio: f64,
}

impl LAMB {
    pub fn new(learning_rate: f64) -> Self {
        Self { inner: Adam::new(learning_rate), trust_ratio: 1.0 }
    }
}

impl Optimizer for LAMB {
    fn step(&mut self, params: &mut HashMap<String, &mut ADTensor>, grads: &HashMap<String, Tensor>) {
        self.inner.step(params, grads);
    }

    fn learning_rate(&self) -> f64 { self.inner.learning_rate() }

    fn set_learning_rate(&mut self, lr: f64) { self.inner.set_learning_rate(lr); }

    fn state_dict(&self) -> HashMap<String, f64> { self.inner.state_dict() }
}
