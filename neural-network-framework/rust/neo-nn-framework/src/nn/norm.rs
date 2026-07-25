use std::collections::HashMap;

use neo_neural_engine::shape::Shape;
use neo_neural_engine::DType;

use crate::autograd::{ADTensor, ad_mul, ad_add, ad_sub, ad_div, ad_sqrt, ad_pow};
use crate::error::NnResult;
use crate::module::{Module, Parameter, TrainingMode};
use crate::init;

#[derive(Debug)]
pub struct LayerNorm {
    weight: Parameter,
    bias: Parameter,
    normalized_shape: Vec<usize>,
    eps: f64,
    mode: TrainingMode,
}

impl LayerNorm {
    pub fn new(normalized_shape: Vec<usize>, eps: f64) -> Self {
        let size: usize = normalized_shape.iter().product();
        let w = ADTensor::ones(Shape::from_1d(size), DType::Float64, true);
        let b = ADTensor::zeros(Shape::from_1d(size), DType::Float64, true);
        Self {
            weight: Parameter::new("weight", w),
            bias: Parameter::new("bias", b),
            normalized_shape,
            eps,
            mode: TrainingMode::Train,
        }
    }
}

impl Module for LayerNorm {
    fn forward(&self, input: &ADTensor) -> NnResult<ADTensor> {
        let ndim = input.ndim();
        let dims = input.shape().dims();

        let norm_size: usize = self.normalized_shape.iter().product();
        let outer: usize = dims[..ndim - self.normalized_shape.len()].iter().copied().product();

        let mut result = ADTensor::zeros(input.shape().clone(), input.dtype(), false);

        let input_data = input.to_vec_f64()?;
        let mut result_data = Vec::with_capacity(input_data.len());

        for o in 0..outer {
            let start = o * norm_size;
            let end = start + norm_size;
            let slice: Vec<f64> = input_data[start..end].to_vec();
            let mean: f64 = slice.iter().sum::<f64>() / norm_size as f64;
            let var: f64 = slice.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / norm_size as f64;
            let std_inv = 1.0 / (var + self.eps).sqrt();

            for i in 0..norm_size {
                let normed = (slice[i] - mean) * std_inv;
                let w = self.weight.tensor().data().item_f64(&[i]).unwrap_or(1.0);
                let b = self.bias.tensor().data().item_f64(&[i]).unwrap_or(0.0);
                result_data.push(normed * w + b);
            }
        }

        let _ = &mut result;
        result = ADTensor::new(
            neo_neural_engine::tensor::Tensor::from_vec_f64(&result_data, input.shape().clone()),
            input.requires_grad(),
        );
        Ok(result)
    }

    fn name(&self) -> &str {
        "LayerNorm"
    }

    fn set_mode(&mut self, mode: TrainingMode) {
        self.mode = mode;
    }

    fn parameters(&self) -> HashMap<String, &ADTensor> {
        let mut params = HashMap::new();
        params.insert("weight".to_string(), self.weight.tensor());
        params.insert("bias".to_string(), self.bias.tensor());
        params
    }

    fn parameters_mut(&mut self) -> HashMap<String, &mut ADTensor> {
        let mut params = HashMap::new();
        params.insert("weight".to_string(), self.weight.tensor_mut());
        params.insert("bias".to_string(), self.bias.tensor_mut());
        params
    }

    fn num_parameters(&self) -> usize {
        self.normalized_shape.iter().product::<usize>() * 2
    }
}

#[derive(Debug)]
pub struct BatchNorm {
    weight: Parameter,
    bias: Parameter,
    running_mean: ADTensor,
    running_var: ADTensor,
    num_features: usize,
    eps: f64,
    momentum: f64,
    mode: TrainingMode,
}

impl BatchNorm {
    pub fn new(num_features: usize, eps: f64, momentum: f64) -> Self {
        Self {
            weight: Parameter::new("weight", ADTensor::ones(Shape::from_1d(num_features), DType::Float64, true)),
            bias: Parameter::new("bias", ADTensor::zeros(Shape::from_1d(num_features), DType::Float64, true)),
            running_mean: ADTensor::zeros(Shape::from_1d(num_features), DType::Float64, false),
            running_var: ADTensor::ones(Shape::from_1d(num_features), DType::Float64, false),
            num_features,
            eps,
            momentum,
            mode: TrainingMode::Train,
        }
    }
}

impl Module for BatchNorm {
    fn forward(&self, input: &ADTensor) -> NnResult<ADTensor> {
        let dims = input.shape().dims();
        let batch_size = dims[0];
        let input_data = input.to_vec_f64()?;
        let mut result_data = Vec::with_capacity(input_data.len());

        let channels = self.num_features;
        let spatial: usize = dims[2..].iter().copied().product::<usize>().max(1);

        for c in 0..channels {
            let mut sum = 0.0;
            let mut sum_sq = 0.0;
            let mut count = 0;
            for b in 0..batch_size {
                for s in 0..spatial {
                    let idx = b * channels * spatial + c * spatial + s;
                    if idx < input_data.len() {
                        let v = input_data[idx];
                        sum += v;
                        sum_sq += v * v;
                        count += 1;
                    }
                }
            }
            let mean = if count > 0 { sum / count as f64 } else { 0.0 };
            let var = if count > 0 { sum_sq / count as f64 - mean * mean } else { 0.0 };
            let std_inv = 1.0 / (var + self.eps).sqrt();

            for b in 0..batch_size {
                for s in 0..spatial {
                    let idx = b * channels * spatial + c * spatial + s;
                    if idx < input_data.len() {
                        let normed = (input_data[idx] - mean) * std_inv;
                        let w = self.weight.tensor().data().item_f64(&[c]).unwrap_or(1.0);
                        let bi = self.bias.tensor().data().item_f64(&[c]).unwrap_or(0.0);
                        result_data.push(normed * w + bi);
                    }
                }
            }
        }

        Ok(ADTensor::new(
            neo_neural_engine::tensor::Tensor::from_vec_f64(&result_data, input.shape().clone()),
            input.requires_grad(),
        ))
    }

    fn name(&self) -> &str {
        "BatchNorm"
    }

    fn set_mode(&mut self, mode: TrainingMode) {
        self.mode = mode;
    }

    fn parameters(&self) -> HashMap<String, &ADTensor> {
        let mut params = HashMap::new();
        params.insert("weight".to_string(), self.weight.tensor());
        params.insert("bias".to_string(), self.bias.tensor());
        params
    }

    fn parameters_mut(&mut self) -> HashMap<String, &mut ADTensor> {
        let mut params = HashMap::new();
        params.insert("weight".to_string(), self.weight.tensor_mut());
        params.insert("bias".to_string(), self.bias.tensor_mut());
        params
    }

    fn num_parameters(&self) -> usize {
        self.num_features * 2
    }
}

#[derive(Debug)]
pub struct GroupNorm {
    weight: Parameter,
    bias: Parameter,
    num_groups: usize,
    num_channels: usize,
    eps: f64,
}

impl GroupNorm {
    pub fn new(num_groups: usize, num_channels: usize, eps: f64) -> Self {
        Self {
            weight: Parameter::new("weight", ADTensor::ones(Shape::from_1d(num_channels), DType::Float64, true)),
            bias: Parameter::new("bias", ADTensor::zeros(Shape::from_1d(num_channels), DType::Float64, true)),
            num_groups,
            num_channels,
            eps,
        }
    }
}

impl Module for GroupNorm {
    fn forward(&self, input: &ADTensor) -> NnResult<ADTensor> {
        let input_data = input.to_vec_f64()?;
        let dims = input.shape().dims();
        let batch_size = dims[0];
        let spatial: usize = dims[2..].iter().copied().product::<usize>().max(1);
        let channels_per_group = self.num_channels / self.num_groups;
        let mut result_data = Vec::with_capacity(input_data.len());

        for b in 0..batch_size {
            for g in 0..self.num_groups {
                let mut sum = 0.0;
                let mut sum_sq = 0.0;
                let mut count = 0;
                for c_offset in 0..channels_per_group {
                    let c = g * channels_per_group + c_offset;
                    for s in 0..spatial {
                        let idx = b * self.num_channels * spatial + c * spatial + s;
                        if idx < input_data.len() {
                            let v = input_data[idx];
                            sum += v;
                            sum_sq += v * v;
                            count += 1;
                        }
                    }
                }
                let mean = if count > 0 { sum / count as f64 } else { 0.0 };
                let var = if count > 0 { sum_sq / count as f64 - mean * mean } else { 0.0 };
                let std_inv = 1.0 / (var + self.eps).sqrt();

                for c_offset in 0..channels_per_group {
                    let c = g * channels_per_group + c_offset;
                    let w = self.weight.tensor().data().item_f64(&[c]).unwrap_or(1.0);
                    let bi = self.bias.tensor().data().item_f64(&[c]).unwrap_or(0.0);
                    for s in 0..spatial {
                        let idx = b * self.num_channels * spatial + c * spatial + s;
                        if idx < input_data.len() {
                            let normed = (input_data[idx] - mean) * std_inv;
                            result_data.push(normed * w + bi);
                        }
                    }
                }
            }
        }

        Ok(ADTensor::new(
            neo_neural_engine::tensor::Tensor::from_vec_f64(&result_data, input.shape().clone()),
            input.requires_grad(),
        ))
    }

    fn name(&self) -> &str {
        "GroupNorm"
    }

    fn parameters(&self) -> HashMap<String, &ADTensor> {
        let mut params = HashMap::new();
        params.insert("weight".to_string(), self.weight.tensor());
        params.insert("bias".to_string(), self.bias.tensor());
        params
    }

    fn parameters_mut(&mut self) -> HashMap<String, &mut ADTensor> {
        let mut params = HashMap::new();
        params.insert("weight".to_string(), self.weight.tensor_mut());
        params.insert("bias".to_string(), self.bias.tensor_mut());
        params
    }

    fn num_parameters(&self) -> usize {
        self.num_channels * 2
    }
}

#[derive(Debug)]
pub struct InstanceNorm {
    weight: Parameter,
    bias: Parameter,
    num_channels: usize,
    eps: f64,
}

impl InstanceNorm {
    pub fn new(num_channels: usize, eps: f64) -> Self {
        Self {
            weight: Parameter::new("weight", ADTensor::ones(Shape::from_1d(num_channels), DType::Float64, true)),
            bias: Parameter::new("bias", ADTensor::zeros(Shape::from_1d(num_channels), DType::Float64, true)),
            num_channels,
            eps,
        }
    }
}

impl Module for InstanceNorm {
    fn forward(&self, input: &ADTensor) -> NnResult<ADTensor> {
        let input_data = input.to_vec_f64()?;
        let dims = input.shape().dims();
        let batch_size = dims[0];
        let spatial: usize = dims[2..].iter().copied().product::<usize>().max(1);
        let mut result_data = Vec::with_capacity(input_data.len());

        for b in 0..batch_size {
            for c in 0..self.num_channels {
                let mut sum = 0.0;
                let mut sum_sq = 0.0;
                for s in 0..spatial {
                    let idx = b * self.num_channels * spatial + c * spatial + s;
                    if idx < input_data.len() {
                        let v = input_data[idx];
                        sum += v;
                        sum_sq += v * v;
                    }
                }
                let mean = sum / spatial as f64;
                let var = sum_sq / spatial as f64 - mean * mean;
                let std_inv = 1.0 / (var + self.eps).sqrt();
                let w = self.weight.tensor().data().item_f64(&[c]).unwrap_or(1.0);
                let bi = self.bias.tensor().data().item_f64(&[c]).unwrap_or(0.0);
                for s in 0..spatial {
                    let idx = b * self.num_channels * spatial + c * spatial + s;
                    if idx < input_data.len() {
                        let normed = (input_data[idx] - mean) * std_inv;
                        result_data.push(normed * w + bi);
                    }
                }
            }
        }

        Ok(ADTensor::new(
            neo_neural_engine::tensor::Tensor::from_vec_f64(&result_data, input.shape().clone()),
            input.requires_grad(),
        ))
    }

    fn name(&self) -> &str {
        "InstanceNorm"
    }

    fn parameters(&self) -> HashMap<String, &ADTensor> {
        let mut params = HashMap::new();
        params.insert("weight".to_string(), self.weight.tensor());
        params.insert("bias".to_string(), self.bias.tensor());
        params
    }

    fn parameters_mut(&mut self) -> HashMap<String, &mut ADTensor> {
        let mut params = HashMap::new();
        params.insert("weight".to_string(), self.weight.tensor_mut());
        params.insert("bias".to_string(), self.bias.tensor_mut());
        params
    }

    fn num_parameters(&self) -> usize {
        self.num_channels * 2
    }
}

#[derive(Debug)]
pub struct RMSNorm {
    weight: Parameter,
    normalized_shape: Vec<usize>,
    eps: f64,
}

impl RMSNorm {
    pub fn new(normalized_shape: Vec<usize>, eps: f64) -> Self {
        let size: usize = normalized_shape.iter().product();
        Self {
            weight: Parameter::new("weight", ADTensor::ones(Shape::from_1d(size), DType::Float64, true)),
            normalized_shape,
            eps,
        }
    }
}

impl Module for RMSNorm {
    fn forward(&self, input: &ADTensor) -> NnResult<ADTensor> {
        let input_data = input.to_vec_f64()?;
        let norm_size: usize = self.normalized_shape.iter().product();
        let total: usize = input_data.len();
        let outer = total / norm_size;
        let mut result_data = Vec::with_capacity(total);

        for o in 0..outer {
            let start = o * norm_size;
            let rms: f64 = input_data[start..start + norm_size]
                .iter()
                .map(|x| x * x)
                .sum::<f64>()
                / norm_size as f64;
            let rms_inv = 1.0 / (rms + self.eps).sqrt();

            for i in 0..norm_size {
                let w = self.weight.tensor().data().item_f64(&[i]).unwrap_or(1.0);
                result_data.push(input_data[start + i] * rms_inv * w);
            }
        }

        Ok(ADTensor::new(
            neo_neural_engine::tensor::Tensor::from_vec_f64(&result_data, input.shape().clone()),
            input.requires_grad(),
        ))
    }

    fn name(&self) -> &str {
        "RMSNorm"
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
        self.normalized_shape.iter().product::<usize>()
    }
}

#[derive(Debug)]
pub struct WeightNorm {
    module: Box<dyn Module>,
    weight_g: Parameter,
    weight_v: Parameter,
}

impl WeightNorm {
    pub fn new(module: Box<dyn Module>, num_features: usize) -> Self {
        let g = ADTensor::ones(Shape::from_1d(num_features), DType::Float64, true);
        let v = init::normal_random(num_features, 0.0, 1.0);
        Self {
            module,
            weight_g: Parameter::new("weight_g", g),
            weight_v: Parameter::new("weight_v", ADTensor::new(v, true)),
        }
    }
}

impl Module for WeightNorm {
    fn forward(&self, input: &ADTensor) -> NnResult<ADTensor> {
        self.module.forward(input)
    }

    fn name(&self) -> &str {
        "WeightNorm"
    }

    fn parameters(&self) -> HashMap<String, &ADTensor> {
        let mut params = self.module.parameters();
        params.insert("weight_g".to_string(), self.weight_g.tensor());
        params.insert("weight_v".to_string(), self.weight_v.tensor());
        params
    }

    fn parameters_mut(&mut self) -> HashMap<String, &mut ADTensor> {
        let mut params = self.module.parameters_mut();
        params.insert("weight_g".to_string(), self.weight_g.tensor_mut());
        params.insert("weight_v".to_string(), self.weight_v.tensor_mut());
        params
    }

    fn num_parameters(&self) -> usize {
        self.module.num_parameters() + self.weight_g.tensor().numel() + self.weight_v.tensor().numel()
    }
}

#[derive(Debug)]
pub struct SpectralNorm {
    module: Box<dyn Module>,
    u: Parameter,
    n_power: usize,
}

impl SpectralNorm {
    pub fn new(module: Box<dyn Module>, num_features: usize, n_power: usize) -> Self {
        let u_data = init::normal_random(num_features, 0.0, 1.0);
        Self {
            module,
            u: Parameter::new("u", ADTensor::new(u_data, false)),
            n_power,
        }
    }
}

impl Module for SpectralNorm {
    fn forward(&self, input: &ADTensor) -> NnResult<ADTensor> {
        self.module.forward(input)
    }

    fn name(&self) -> &str {
        "SpectralNorm"
    }

    fn parameters(&self) -> HashMap<String, &ADTensor> {
        self.module.parameters()
    }

    fn parameters_mut(&mut self) -> HashMap<String, &mut ADTensor> {
        self.module.parameters_mut()
    }

    fn num_parameters(&self) -> usize {
        self.module.num_parameters()
    }
}
