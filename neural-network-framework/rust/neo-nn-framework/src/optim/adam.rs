use std::collections::HashMap;
use neo_neural_engine::tensor::Tensor;

use crate::autograd::ADTensor;
use crate::optim::{Optimizer, OptimizerState};

#[derive(Debug)]
pub struct Adam {
    state: OptimizerState,
    beta1: f64,
    beta2: f64,
    eps: f64,
    weight_decay: f64,
    m: HashMap<String, Tensor>,
    v: HashMap<String, Tensor>,
}

impl Adam {
    pub fn new(learning_rate: f64) -> Self {
        Self {
            state: OptimizerState::new(learning_rate),
            beta1: 0.9, beta2: 0.999, eps: 1e-8, weight_decay: 0.0,
            m: HashMap::new(), v: HashMap::new(),
        }
    }

    pub fn with_params(learning_rate: f64, beta1: f64, beta2: f64, eps: f64, weight_decay: f64) -> Self {
        Self {
            state: OptimizerState::new(learning_rate),
            beta1, beta2, eps, weight_decay,
            m: HashMap::new(), v: HashMap::new(),
        }
    }
}

impl Optimizer for Adam {
    fn step(&mut self, params: &mut HashMap<String, &mut ADTensor>, grads: &HashMap<String, Tensor>) {
        let t = (self.state.step_count + 1) as f64;
        let bias_correction1 = 1.0 - self.beta1.powf(t);
        let bias_correction2 = 1.0 - self.beta2.powf(t);

        for (name, param) in params.iter_mut() {
            if let Some(grad) = grads.get(name) {
                let m = self.m.entry(name.clone()).or_insert_with(|| {
                    Tensor::zeros(param.shape().clone(), param.dtype())
                });
                let v = self.v.entry(name.clone()).or_insert_with(|| {
                    Tensor::zeros(param.shape().clone(), param.dtype())
                });

                let p_vals = param.data().to_vec_f64().unwrap_or_default();
                let g_vals = grad.to_vec_f64().unwrap_or_default();
                let m_vals = m.to_vec_f64().unwrap_or_default();
                let v_vals = v.to_vec_f64().unwrap_or_default();

                let mut new_m = Vec::with_capacity(p_vals.len());
                let mut new_v = Vec::with_capacity(p_vals.len());
                let mut new_p = Vec::with_capacity(p_vals.len());

                for i in 0..p_vals.len() {
                    let g = if i < g_vals.len() { g_vals[i] } else { 0.0 };
                    let gi = if self.weight_decay > 0.0 { g + self.weight_decay * p_vals[i] } else { g };
                    let mi = self.beta1 * if i < m_vals.len() { m_vals[i] } else { 0.0 } + (1.0 - self.beta1) * gi;
                    let vi = self.beta2 * if i < v_vals.len() { v_vals[i] } else { 0.0 } + (1.0 - self.beta2) * gi * gi;
                    new_m.push(mi);
                    new_v.push(vi);
                    let m_hat = mi / bias_correction1;
                    let v_hat = vi / bias_correction2;
                    new_p.push(p_vals[i] - self.state.learning_rate * m_hat / (v_hat.sqrt() + self.eps));
                }

                *m = Tensor::from_vec_f64(&new_m, m.shape().clone());
                *v = Tensor::from_vec_f64(&new_v, v.shape().clone());
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
        s.insert("eps".to_string(), self.eps);
        s.insert("weight_decay".to_string(), self.weight_decay);
        s
    }
}

#[derive(Debug)]
pub struct AdamW {
    inner: Adam,
}

impl AdamW {
    pub fn new(learning_rate: f64, weight_decay: f64) -> Self {
        Self { inner: Adam::with_params(learning_rate, 0.9, 0.999, 1e-8, weight_decay) }
    }
}

impl Optimizer for AdamW {
    fn step(&mut self, params: &mut HashMap<String, &mut ADTensor>, grads: &HashMap<String, Tensor>) {
        self.inner.step(params, grads);
    }

    fn learning_rate(&self) -> f64 { self.inner.learning_rate() }

    fn set_learning_rate(&mut self, lr: f64) { self.inner.set_learning_rate(lr); }

    fn state_dict(&self) -> HashMap<String, f64> { self.inner.state_dict() }
}

#[derive(Debug)]
pub struct Adamax {
    state: OptimizerState,
    beta1: f64,
    beta2: f64,
    eps: f64,
    m: HashMap<String, Tensor>,
    u: HashMap<String, Tensor>,
}

impl Adamax {
    pub fn new(learning_rate: f64) -> Self {
        Self {
            state: OptimizerState::new(learning_rate),
            beta1: 0.9, beta2: 0.999, eps: 1e-8,
            m: HashMap::new(), u: HashMap::new(),
        }
    }
}

impl Optimizer for Adamax {
    fn step(&mut self, params: &mut HashMap<String, &mut ADTensor>, grads: &HashMap<String, Tensor>) {
        let t = (self.state.step_count + 1) as f64;

        for (name, param) in params.iter_mut() {
            if let Some(grad) = grads.get(name) {
                let m = self.m.entry(name.clone()).or_insert_with(|| {
                    Tensor::zeros(param.shape().clone(), param.dtype())
                });
                let u = self.u.entry(name.clone()).or_insert_with(|| {
                    Tensor::zeros(param.shape().clone(), param.dtype())
                });

                let p_vals = param.data().to_vec_f64().unwrap_or_default();
                let g_vals = grad.to_vec_f64().unwrap_or_default();
                let m_vals = m.to_vec_f64().unwrap_or_default();
                let u_vals = u.to_vec_f64().unwrap_or_default();

                let mut new_m = Vec::with_capacity(p_vals.len());
                let mut new_u = Vec::with_capacity(p_vals.len());
                let mut new_p = Vec::with_capacity(p_vals.len());

                for i in 0..p_vals.len() {
                    let g = if i < g_vals.len() { g_vals[i] } else { 0.0 };
                    let mi = self.beta1 * if i < m_vals.len() { m_vals[i] } else { 0.0 } + (1.0 - self.beta1) * g;
                    let ui = (self.beta2 * if i < u_vals.len() { u_vals[i] } else { 0.0 }).max(g.abs());
                    new_m.push(mi);
                    new_u.push(ui);
                    let denom = ui + self.eps;
                    new_p.push(p_vals[i] - (self.state.learning_rate / (1.0 - self.beta1.powf(t))) * mi / denom);
                }

                *m = Tensor::from_vec_f64(&new_m, m.shape().clone());
                *u = Tensor::from_vec_f64(&new_u, u.shape().clone());
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
        s
    }
}
