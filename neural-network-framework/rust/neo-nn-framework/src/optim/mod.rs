pub mod sgd;
pub mod adam;
pub mod rmsprop;

pub use sgd::{SGD, MomentumSGD, NesterovSGD};
pub use adam::{Adam, AdamW, Adamax};
pub use rmsprop::{RMSProp};

pub trait Optimizer {
    fn step(&mut self, params: &mut std::collections::HashMap<String, &mut crate::autograd::ADTensor>, grads: &std::collections::HashMap<String, neo_neural_engine::tensor::Tensor>);
    fn learning_rate(&self) -> f64;
    fn set_learning_rate(&mut self, lr: f64);
    fn state_dict(&self) -> std::collections::HashMap<String, f64>;
    fn load_state_dict(&mut self, _state: &std::collections::HashMap<String, f64>) {}
}

#[derive(Debug, Clone)]
pub struct OptimizerState {
    pub step_count: u64,
    pub learning_rate: f64,
}

impl OptimizerState {
    pub fn new(learning_rate: f64) -> Self {
        Self { step_count: 0, learning_rate }
    }
}
