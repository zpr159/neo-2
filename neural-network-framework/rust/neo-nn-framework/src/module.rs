use std::collections::HashMap;
use std::fmt;

use crate::autograd::ADTensor;
use crate::error::{NnError, NnResult};

/// A named parameter (trainable weight) within a module.
#[derive(Debug, Clone)]
pub struct Parameter {
    name: String,
    tensor: ADTensor,
}

impl Parameter {
    pub fn new(name: impl Into<String>, tensor: ADTensor) -> Self {
        Self { name: name.into(), tensor }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn tensor(&self) -> &ADTensor {
        &self.tensor
    }

    pub fn tensor_mut(&mut self) -> &mut ADTensor {
        &mut self.tensor
    }

    pub fn into_tensor(self) -> ADTensor {
        self.tensor
    }
}

/// A named buffer (non-trainable state) within a module.
#[derive(Debug, Clone)]
pub struct Buffer {
    name: String,
    tensor: ADTensor,
}

impl Buffer {
    pub fn new(name: impl Into<String>, tensor: ADTensor) -> Self {
        Self { name: name.into(), tensor }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn tensor(&self) -> &ADTensor {
        &self.tensor
    }

    pub fn tensor_mut(&mut self) -> &mut ADTensor {
        &mut self.tensor
    }
}

/// Training mode of a module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrainingMode {
    Train,
    Eval,
}

/// Trait implemented by all neural network modules.
pub trait Module: Send + Sync + fmt::Debug {
    /// Forward pass.
    fn forward(&self, input: &ADTensor) -> NnResult<ADTensor>;

    /// Module name.
    fn name(&self) -> &str {
        "Module"
    }

    /// Set training or evaluation mode.
    fn set_mode(&mut self, mode: TrainingMode) {
        let _ = mode;
    }

    /// Get current training mode.
    fn mode(&self) -> TrainingMode {
        TrainingMode::Eval
    }

    /// Set to training mode.
    fn train(&mut self) {
        self.set_mode(TrainingMode::Train);
    }

    /// Set to evaluation mode.
    fn eval(&mut self) {
        self.set_mode(TrainingMode::Eval);
    }

    /// Returns all named parameters.
    fn parameters(&self) -> HashMap<String, &ADTensor> {
        HashMap::new()
    }

    /// Returns all named parameters mutably.
    fn parameters_mut(&mut self) -> HashMap<String, &mut ADTensor> {
        HashMap::new()
    }

    /// Returns all named buffers.
    fn buffers(&self) -> HashMap<String, &ADTensor> {
        HashMap::new()
    }

    /// Returns a state dict (name -> tensor data).
    fn state_dict(&self) -> HashMap<String, ADTensor> {
        self.parameters()
            .into_iter()
            .map(|(k, v)| (k, v.clone()))
            .collect()
    }

    /// Loads a state dict.
    fn load_state_dict(&mut self, state: &HashMap<String, ADTensor>) -> NnResult<()> {
        let _ = state;
        Ok(())
    }

    /// Returns number of parameters.
    fn num_parameters(&self) -> usize {
        self.parameters().values().map(|p| p.numel()).sum()
    }

    /// Returns number of submodules.
    fn num_submodules(&self) -> usize {
        0
    }

    /// Returns submodule by name.
    fn get_submodule(&self, _name: &str) -> Option<&dyn Module> {
        None
    }

    /// Returns mutable submodule by name.
    fn get_submodule_mut(&mut self, _name: &str) -> Option<&mut dyn Module> {
        None
    }
}

/// A sequential container that applies modules in order.
#[derive(Debug)]
pub struct Sequential {
    modules: Vec<Box<dyn Module>>,
    name: String,
}

impl Sequential {
    pub fn new(name: impl Into<String>) -> Self {
        Self { modules: Vec::new(), name: name.into() }
    }

    pub fn push(&mut self, module: Box<dyn Module>) {
        self.modules.push(module);
    }

    pub fn len(&self) -> usize {
        self.modules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }
}

impl Module for Sequential {
    fn forward(&self, input: &ADTensor) -> NnResult<ADTensor> {
        let mut x = input.clone();
        for m in &self.modules {
            x = m.forward(&x)?;
        }
        Ok(x)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn set_mode(&mut self, mode: TrainingMode) {
        for m in &mut self.modules {
            m.set_mode(mode);
        }
    }

    fn parameters(&self) -> HashMap<String, &ADTensor> {
        let mut params = HashMap::new();
        for (i, m) in self.modules.iter().enumerate() {
            for (k, v) in m.parameters() {
                params.insert(format!("{}.{}", i, k), v);
            }
        }
        params
    }

    fn parameters_mut(&mut self) -> HashMap<String, &mut ADTensor> {
        let mut params = HashMap::new();
        for (i, m) in self.modules.iter_mut().enumerate() {
            for (k, v) in m.parameters_mut() {
                params.insert(format!("{}.{}", i, k), v);
            }
        }
        params
    }

    fn buffers(&self) -> HashMap<String, &ADTensor> {
        let mut bufs = HashMap::new();
        for (i, m) in self.modules.iter().enumerate() {
            for (k, v) in m.buffers() {
                bufs.insert(format!("{}.{}", i, k), v);
            }
        }
        bufs
    }

    fn num_parameters(&self) -> usize {
        self.modules.iter().map(|m| m.num_parameters()).sum()
    }

    fn num_submodules(&self) -> usize {
        self.modules.len()
    }

    fn get_submodule(&self, name: &str) -> Option<&dyn Module> {
        for m in &self.modules {
            if m.name() == name {
                return Some(m.as_ref());
            }
        }
        None
    }

    fn get_submodule_mut(&mut self, name: &str) -> Option<&mut dyn Module> {
        for m in &mut self.modules {
            if m.name() == name {
                return Some(m.as_mut());
            }
        }
        None
    }

    fn state_dict(&self) -> HashMap<String, ADTensor> {
        let mut state = HashMap::new();
        for (i, m) in self.modules.iter().enumerate() {
            for (k, v) in m.state_dict() {
                state.insert(format!("{}.{}", i, k), v);
            }
        }
        state
    }

    fn load_state_dict(&mut self, state: &HashMap<String, ADTensor>) -> NnResult<()> {
        for (i, m) in self.modules.iter_mut().enumerate() {
            let prefix = format!("{}.", i);
            let sub_state: HashMap<String, ADTensor> = state
                .iter()
                .filter(|(k, _)| k.starts_with(&prefix))
                .map(|(k, v)| (k[prefix.len()..].to_string(), v.clone()))
                .collect();
            if !sub_state.is_empty() {
                m.load_state_dict(&sub_state)?;
            }
        }
        Ok(())
    }
}

/// A module list for storing submodules.
#[derive(Debug)]
pub struct ModuleList {
    modules: Vec<Box<dyn Module>>,
}

impl ModuleList {
    pub fn new() -> Self {
        Self { modules: Vec::new() }
    }

    pub fn push(&mut self, module: Box<dyn Module>) {
        self.modules.push(module);
    }

    pub fn len(&self) -> usize {
        self.modules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&dyn Module> {
        self.modules.get(index).map(|m| m.as_ref())
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut dyn Module> {
        self.modules.get_mut(index).map(|m| &mut **m as &mut dyn Module)
    }

    pub fn iter(&self) -> impl Iterator<Item = &dyn Module> {
        self.modules.iter().map(|m| m.as_ref())
    }
}

impl Default for ModuleList {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for ModuleList {
    fn forward(&self, input: &ADTensor) -> NnResult<ADTensor> {
        let mut x = input.clone();
        for m in &self.modules {
            x = m.forward(&x)?;
        }
        Ok(x)
    }

    fn parameters(&self) -> HashMap<String, &ADTensor> {
        let mut params = HashMap::new();
        for (i, m) in self.modules.iter().enumerate() {
            for (k, v) in m.parameters() {
                params.insert(format!("{}.{}", i, k), v);
            }
        }
        params
    }

    fn parameters_mut(&mut self) -> HashMap<String, &mut ADTensor> {
        let mut params = HashMap::new();
        for (i, m) in self.modules.iter_mut().enumerate() {
            for (k, v) in m.parameters_mut() {
                params.insert(format!("{}.{}", i, k), v);
            }
        }
        params
    }

    fn num_parameters(&self) -> usize {
        self.modules.iter().map(|m| m.num_parameters()).sum()
    }

    fn num_submodules(&self) -> usize {
        self.modules.len()
    }
}

/// Macro to create a sequential model.
#[macro_export]
macro_rules! sequential {
    ($name:expr, [ $($module:expr),* $(,)? ]) => {
        {
            let mut seq = $crate::module::Sequential::new($name);
            $(seq.push(Box::new($module));)*
            seq
        }
    };
}
