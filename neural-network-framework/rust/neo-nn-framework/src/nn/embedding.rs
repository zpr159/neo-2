use std::collections::HashMap;

use neo_neural_engine::shape::Shape;
use neo_neural_engine::DType;

use crate::autograd::ADTensor;
use crate::error::{NnError, NnResult};
use crate::init;
use crate::module::{Module, Parameter};

#[derive(Debug)]
pub struct Embedding {
    weight: Parameter,
    num_embeddings: usize,
    embedding_dim: usize,
}

impl Embedding {
    pub fn new(num_embeddings: usize, embedding_dim: usize) -> Self {
        let data = init::normal_random(num_embeddings * embedding_dim, 0.0, 1.0);
        let shape = Shape::from_2d(num_embeddings, embedding_dim);
        let weight = Parameter::new("weight", ADTensor::new(data, true));
        let _ = shape;
        Self { weight, num_embeddings, embedding_dim }
    }
}

impl Module for Embedding {
    fn forward(&self, input: &ADTensor) -> NnResult<ADTensor> {
        let indices = input.to_vec_f64()?;
        let w_data = self.weight.tensor().data();
        let mut result = Vec::new();

        for &idx in &indices {
            let i = idx as usize;
            if i >= self.num_embeddings {
                return Err(NnError::InvalidInput(format!(
                    "Embedding index {} out of range [0, {})",
                    i, self.num_embeddings
                )));
            }
            for j in 0..self.embedding_dim {
                result.push(w_data.item_f64(&[i, j])?);
            }
        }

        let out_shape = if input.ndim() == 1 {
            Shape::from_2d(input.numel(), self.embedding_dim)
        } else {
            let mut dims = input.shape().dims().to_vec();
            dims.push(self.embedding_dim);
            Shape::new(dims)
        };

        Ok(ADTensor::new(
            neo_neural_engine::tensor::Tensor::from_vec_f64(&result, out_shape),
            self.weight.tensor().requires_grad(),
        ))
    }

    fn name(&self) -> &str {
        "Embedding"
    }

    fn parameters(&self) -> HashMap<String, &ADTensor> {
        let mut params = HashMap::new();
        params.insert("weight".to_string(), self.weight.tensor());
        params
    }

    fn parameters_mut(&mut self) -> HashMap<String, &mut ADTensor> {
        let mut params = HashMap::new();
        params.insert("weight".to_string(), self.weight.tensor_mut());
        params
    }

    fn num_parameters(&self) -> usize {
        self.num_embeddings * self.embedding_dim
    }
}
