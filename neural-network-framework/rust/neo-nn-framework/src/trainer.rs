use std::collections::HashMap;

use crate::autograd::{ADTensor, backward as ad_backward, startRecording, stopRecording, GradTape};
use crate::error::NnResult;
use crate::module::{Module, TrainingMode};
use crate::optim::Optimizer;
use crate::checkpoint::{Checkpoint, CheckpointManager, CheckpointMetadata};

pub trait Metric {
    fn name(&self) -> &str;
    fn update(&mut self, predictions: &[f64], targets: &[f64]);
    fn compute(&self) -> f64;
    fn reset(&mut self);
}

pub trait Callback {
    fn on_epoch_start(&mut self, _epoch: u32) {}
    fn on_epoch_end(&mut self, _epoch: u32, _metrics: &HashMap<String, f64>) {}
    fn on_batch_start(&mut self, _batch: u64) {}
    fn on_batch_end(&mut self, _batch: u64, _loss: f64) {}
    fn on_train_start(&mut self) {}
    fn on_train_end(&mut self) {}
}

#[derive(Debug, Clone)]
pub struct TrainingConfig {
    pub epochs: u32,
    pub batch_size: usize,
    pub learning_rate: f64,
    pub log_interval: u32,
    pub checkpoint_interval: u32,
    pub max_grad_norm: Option<f64>,
    pub early_stopping_patience: Option<u32>,
}

impl Default for TrainingConfig {
    fn default() -> Self {
        Self {
            epochs: 10, batch_size: 32, learning_rate: 0.001,
            log_interval: 100, checkpoint_interval: 1000,
            max_grad_norm: Some(1.0), early_stopping_patience: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TrainingState {
    pub current_epoch: u32,
    pub current_step: u64,
    pub total_loss: f64,
    pub best_loss: f64,
    pub learning_rate: f64,
    pub is_running: bool,
    pub epochs_without_improvement: u32,
}

impl Default for TrainingState {
    fn default() -> Self {
        Self {
            current_epoch: 0, current_step: 0, total_loss: 0.0, best_loss: f64::INFINITY,
            learning_rate: 0.001, is_running: false, epochs_without_improvement: 0,
        }
    }
}

#[derive(Debug)]
pub struct EarlyStopping {
    patience: u32,
    min_delta: f64,
    best_loss: f64,
    counter: u32,
    should_stop: bool,
}

impl EarlyStopping {
    pub fn new(patience: u32, min_delta: f64) -> Self {
        Self { patience, min_delta, best_loss: f64::INFINITY, counter: 0, should_stop: false }
    }

    pub fn check(&mut self, loss: f64) -> bool {
        if loss < self.best_loss - self.min_delta {
            self.best_loss = loss;
            self.counter = 0;
        } else {
            self.counter += 1;
            if self.counter >= self.patience {
                self.should_stop = true;
            }
        }
        self.should_stop
    }
}

pub struct Trainer {
    config: TrainingConfig,
    state: TrainingState,
    checkpoint_manager: Option<CheckpointManager>,
    early_stopping: Option<EarlyStopping>,
    callbacks: Vec<Box<dyn Callback>>,
}

impl Trainer {
    pub fn new(config: TrainingConfig) -> Self {
        let early_stopping = config.early_stopping_patience.map(|p| EarlyStopping::new(p, 1e-6));
        Self {
            state: TrainingState {
                learning_rate: config.learning_rate,
                ..TrainingState::default()
            },
            config,
            checkpoint_manager: None,
            early_stopping,
            callbacks: Vec::new(),
        }
    }

    pub fn with_checkpoint_manager(mut self, manager: CheckpointManager) -> Self {
        self.checkpoint_manager = Some(manager);
        self
    }

    pub fn add_callback(&mut self, callback: Box<dyn Callback>) {
        self.callbacks.push(callback);
    }

    pub fn state(&self) -> &TrainingState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut TrainingState {
        &mut self.state
    }

    pub fn train_step<M: Module>(
        &mut self,
        model: &mut M,
        optimizer: &mut dyn Optimizer,
        input: &ADTensor,
        targets: &ADTensor,
    ) -> NnResult<f64> {
        self.state.is_running = true;
        model.train();

        startRecording();
        let output = model.forward(input)?;
        let loss = crate::loss::mse_loss(&output, targets)?;
        let loss_val = loss.item()?;
        let tape = stopRecording().unwrap_or_else(|| GradTape::new());

        let grads = ad_backward(&loss, &[&output], &tape).unwrap_or_default();

        let param_names: Vec<String> = model.parameters().into_iter().map(|(k, _)| k).collect();
        let mut grad_map = HashMap::new();
        for name in &param_names {
            if let Some(tensor) = model.parameters().get(name) {
                if let Some(grad) = grads.get(&tensor.id()) {
                    grad_map.insert(name.clone(), grad.clone());
                }
            }
        }

        let mut params = model.parameters_mut();
        optimizer.step(&mut params, &grad_map);

        self.state.current_step += 1;
        self.state.total_loss += loss_val;
        if loss_val < self.state.best_loss {
            self.state.best_loss = loss_val;
            self.state.epochs_without_improvement = 0;
        }

        for cb in &mut self.callbacks {
            cb.on_batch_end(self.state.current_step, loss_val);
        }

        Ok(loss_val)
    }

    pub fn validate<M: Module>(
        &self,
        model: &M,
        val_inputs: &[ADTensor],
        val_targets: &[ADTensor],
    ) -> NnResult<f64> {
        let mut total_loss = 0.0;
        let mut count = 0;
        for (input, target) in val_inputs.iter().zip(val_targets.iter()) {
            let output = model.forward(input)?;
            let loss = crate::loss::mse_loss(&output, target)?;
            total_loss += loss.item()?;
            count += 1;
        }
        Ok(if count > 0 { total_loss / count as f64 } else { 0.0 })
    }

    pub fn save_checkpoint<M: Module>(
        &mut self,
        model: &M,
        optimizer: &dyn Optimizer,
    ) -> NnResult<()> {
        if let Some(ref mut manager) = self.checkpoint_manager {
            let metadata = CheckpointMetadata {
                epoch: self.state.current_epoch,
                step: self.state.current_step,
                loss: self.state.total_loss,
                learning_rate: self.state.learning_rate,
                ..CheckpointMetadata::default()
            };
            let params = model.parameters();
            let param_refs: std::collections::HashMap<String, &ADTensor> = params.iter().map(|(k, v)| (k.clone(), *v)).collect();
            let opt_state = optimizer.state_dict();
            let checkpoint = Checkpoint::from_parameters(metadata, &param_refs, &opt_state);
            manager.save(&checkpoint)?;
        }
        Ok(())
    }

    pub fn next_epoch(&mut self) {
        self.state.current_epoch += 1;
        self.state.epochs_without_improvement += 1;
        for cb in &mut self.callbacks {
            let metrics = HashMap::new();
            cb.on_epoch_end(self.state.current_epoch, &metrics);
        }
    }
}
