use std::collections::HashMap;

use neo_neural_engine::shape::Shape;
use neo_neural_engine::DType;

use crate::autograd::{ADTensor, ad_matmul, ad_add};
use crate::error::{NnError, NnResult};
use crate::init;
use crate::module::{Module, Parameter, TrainingMode};

#[derive(Debug)]
pub struct Linear {
    weight: Parameter,
    bias: Parameter,
    in_features: usize,
    out_features: usize,
    use_bias: bool,
}

impl Linear {
    pub fn new(in_features: usize, out_features: usize, use_bias: bool) -> Self {
        let w_data = init::xavier_uniform(in_features * out_features, DType::Float64, in_features, out_features);
        let weight = Parameter::new("weight", ADTensor::new(w_data, true));
        let bias = if use_bias {
            let b_data = ADTensor::zeros(Shape::from_1d(out_features), DType::Float64, true);
            Some(Parameter::new("bias", b_data))
        } else {
            None
        };
        Self {
            weight,
            bias: bias.unwrap_or_else(|| Parameter::new("bias", ADTensor::zeros(Shape::from_1d(1), DType::Float64, false))),
            in_features,
            out_features,
            use_bias,
        }
    }
}

impl Module for Linear {
    fn forward(&self, input: &ADTensor) -> NnResult<ADTensor> {
        let w = self.weight.tensor().data();
        let w_ad = ADTensor::new(w.clone(), self.weight.tensor().requires_grad());
        let out = ad_matmul(input, &w_ad)?;
        if self.use_bias {
            let b = self.bias.tensor();
            ad_add(&out, b)
        } else {
            Ok(out)
        }
    }

    fn name(&self) -> &str {
        "Linear"
    }

    fn parameters(&self) -> HashMap<String, &ADTensor> {
        let mut params = HashMap::new();
        params.insert("weight".to_string(), self.weight.tensor());
        if self.use_bias {
            params.insert("bias".to_string(), self.bias.tensor());
        }
        params
    }

    fn parameters_mut(&mut self) -> HashMap<String, &mut ADTensor> {
        let mut params = HashMap::new();
        params.insert("weight".to_string(), self.weight.tensor_mut());
        if self.use_bias {
            params.insert("bias".to_string(), self.bias.tensor_mut());
        }
        params
    }

    fn state_dict(&self) -> HashMap<String, ADTensor> {
        let mut state = HashMap::new();
        state.insert("weight".to_string(), self.weight.tensor().clone());
        if self.use_bias {
            state.insert("bias".to_string(), self.bias.tensor().clone());
        }
        state
    }

    fn load_state_dict(&mut self, state: &HashMap<String, ADTensor>) -> NnResult<()> {
        if let Some(w) = state.get("weight") {
            if w.shape() == self.weight.tensor().shape() {
                *self.weight.tensor_mut() = w.clone();
            } else {
                return Err(NnError::ShapeMismatch {
                    expected: self.weight.tensor().shape().to_vec(),
                    actual: w.shape().to_vec(),
                    context: "Linear weight".to_string(),
                });
            }
        }
        if self.use_bias {
            if let Some(b) = state.get("bias") {
                *self.bias.tensor_mut() = b.clone();
            }
        }
        Ok(())
    }

    fn num_parameters(&self) -> usize {
        let mut n = self.in_features * self.out_features;
        if self.use_bias {
            n += self.out_features;
        }
        n
    }
}
