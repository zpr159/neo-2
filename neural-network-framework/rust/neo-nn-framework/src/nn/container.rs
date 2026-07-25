use std::collections::HashMap;

use crate::autograd::ADTensor;
use crate::error::NnResult;
use crate::module::Module;

#[derive(Debug)]
pub struct Identity;

impl Identity {
    pub fn new() -> Self { Self }
}

impl Default for Identity {
    fn default() -> Self { Self::new() }
}

impl Module for Identity {
    fn forward(&self, input: &ADTensor) -> NnResult<ADTensor> {
        Ok(input.clone())
    }

    fn name(&self) -> &str { "Identity" }
}

#[derive(Debug)]
pub struct Flatten { start_dim: usize, end_dim: Option<usize> }

impl Flatten {
    pub fn new(start_dim: usize, end_dim: Option<usize>) -> Self {
        Self { start_dim, end_dim }
    }

    pub fn default_() -> Self {
        Self { start_dim: 1, end_dim: None }
    }
}

impl Module for Flatten {
    fn forward(&self, input: &ADTensor) -> NnResult<ADTensor> {
        let dims = input.shape().dims();
        let start = self.start_dim;
        let end = self.end_dim.unwrap_or(dims.len());
        let flat_size: usize = dims[start..end].iter().product();
        let mut new_dims: Vec<usize> = dims[..start].to_vec();
        if flat_size > 0 || new_dims.is_empty() {
            new_dims.push(flat_size);
        }
        new_dims.extend_from_slice(&dims[end..]);
        if new_dims.is_empty() {
            new_dims.push(input.numel());
        }
        crate::autograd::ad_reshape(input, neo_neural_engine::shape::Shape::new(new_dims))
    }

    fn name(&self) -> &str { "Flatten" }
}

#[derive(Debug)]
pub struct Reshape { shape: Vec<usize> }

impl Reshape {
    pub fn new(shape: Vec<usize>) -> Self {
        Self { shape }
    }
}

impl Module for Reshape {
    fn forward(&self, input: &ADTensor) -> NnResult<ADTensor> {
        crate::autograd::ad_reshape(input, neo_neural_engine::shape::Shape::new(self.shape.clone()))
    }

    fn name(&self) -> &str { "Reshape" }
}
