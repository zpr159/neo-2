use std::collections::HashMap;

use crate::autograd::{ad_relu, ad_sigmoid, ad_tanh, ad_gelu, ad_softplus, ad_softsign, ad_swish, ad_mish, ad_hard_swish, ad_hard_sigmoid, ad_elu, ad_selu, ad_prelu, ADTensor};
use crate::error::NnResult;
use crate::module::{Module, Parameter};

macro_rules! simple_activation {
    ($name:ident, $fn:path, $display:expr) => {
        #[derive(Debug)]
        pub struct $name;

        impl $name {
            pub fn new() -> Self { Self }
        }

        impl Default for $name {
            fn default() -> Self { Self::new() }
        }

        impl Module for $name {
            fn forward(&self, input: &ADTensor) -> NnResult<ADTensor> {
                $fn(input)
            }

            fn name(&self) -> &str { $display }
        }
    };
}

simple_activation!(ReLU, ad_relu, "ReLU");
simple_activation!(GELU, ad_gelu, "GELU");
simple_activation!(Sigmoid, ad_sigmoid, "Sigmoid");
simple_activation!(Tanh, ad_tanh, "Tanh");
simple_activation!(Softplus, ad_softplus, "Softplus");
simple_activation!(Softsign, ad_softsign, "Softsign");
simple_activation!(Swish, ad_swish, "Swish");
simple_activation!(Mish, ad_mish, "Mish");
simple_activation!(HardSwish, ad_hard_swish, "HardSwish");
simple_activation!(HardSigmoid, ad_hard_sigmoid, "HardSigmoid");
simple_activation!(SELU, ad_selu, "SELU");
simple_activation!(SiLU, ad_swish, "SiLU");

#[derive(Debug)]
pub struct LeakyReLU {
    alpha: f64,
}

impl LeakyReLU {
    pub fn new(alpha: f64) -> Self {
        Self { alpha }
    }
}

impl Default for LeakyReLU {
    fn default() -> Self {
        Self::new(0.01)
    }
}

impl Module for LeakyReLU {
    fn forward(&self, input: &ADTensor) -> NnResult<ADTensor> {
        let numel = input.numel();
        let input_data = input.to_vec_f64()?;
        let mut result_data = Vec::with_capacity(numel);

        for &v in &input_data {
            result_data.push(if v > 0.0 { v } else { self.alpha * v });
        }

        Ok(ADTensor::new(
            neo_neural_engine::tensor::Tensor::from_vec_f64(&result_data, input.shape().clone()),
            input.requires_grad(),
        ))
    }

    fn name(&self) -> &str {
        "LeakyReLU"
    }
}

#[derive(Debug)]
pub struct PReLU {
    weight: Parameter,
    num_params: usize,
}

impl PReLU {
    pub fn new(num_parameters: usize) -> Self {
        let w = ADTensor::from_vec_f32(
            &vec![0.25; num_parameters],
            neo_neural_engine::shape::Shape::from_1d(num_parameters),
            true,
        );
        Self {
            weight: Parameter::new("weight", w),
            num_params: num_parameters,
        }
    }
}

impl Module for PReLU {
    fn forward(&self, input: &ADTensor) -> NnResult<ADTensor> {
        ad_prelu(input, self.weight.tensor())
    }

    fn name(&self) -> &str {
        "PReLU"
    }

    fn parameters(&self) -> HashMap<String, &ADTensor> {
        let mut p = HashMap::new();
        p.insert("weight".to_string(), self.weight.tensor());
        p
    }

    fn parameters_mut(&mut self) -> HashMap<String, &mut ADTensor> {
        let mut p = HashMap::new();
        p.insert("weight".to_string(), self.weight.tensor_mut());
        p
    }

    fn num_parameters(&self) -> usize {
        self.num_params
    }
}

#[derive(Debug)]
pub struct ELU {
    alpha: f64,
}

impl ELU {
    pub fn new(alpha: f64) -> Self {
        Self { alpha }
    }
}

impl Default for ELU {
    fn default() -> Self {
        Self::new(1.0)
    }
}

impl Module for ELU {
    fn forward(&self, input: &ADTensor) -> NnResult<ADTensor> {
        ad_elu(input, self.alpha)
    }

    fn name(&self) -> &str {
        "ELU"
    }
}

#[derive(Debug)]
pub struct GLU;

impl GLU {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GLU {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for GLU {
    fn forward(&self, input: &ADTensor) -> NnResult<ADTensor> {
        let dims = input.shape().dims();
        let ndim = dims.len();
        let mid = dims[ndim - 1] / 2;
        let a = crate::autograd::ad_slice(input, ndim - 1, 0, mid)?;
        let b = crate::autograd::ad_slice(input, ndim - 1, mid, mid * 2)?;
        let sig = ad_sigmoid(&b)?;
        crate::autograd::ad_mul(&a, &sig)
    }

    fn name(&self) -> &str {
        "GLU"
    }
}
