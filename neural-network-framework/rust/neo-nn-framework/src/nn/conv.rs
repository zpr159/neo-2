use std::collections::HashMap;

use neo_neural_engine::shape::Shape;
use neo_neural_engine::DType;

use crate::autograd::ADTensor;
use crate::error::NnResult;
use crate::module::{Module, Parameter};
use crate::init;

#[derive(Debug)]
pub struct Conv1D {
    weight: Parameter,
    bias: Parameter,
    in_channels: usize,
    out_channels: usize,
    kernel_size: usize,
    stride: usize,
    padding: usize,
    dilation: usize,
    groups: usize,
    use_bias: bool,
}

impl Conv1D {
    pub fn new(
        in_channels: usize, out_channels: usize, kernel_size: usize,
        stride: usize, padding: usize, dilation: usize, groups: usize, use_bias: bool,
    ) -> Self {
        let k = kernel_size;
        let cin_per_group = in_channels / groups;
        let w_size = out_channels * cin_per_group * k;
        let w = init::xavier_uniform(w_size, DType::Float64, in_channels, k);
        let weight = Parameter::new("weight", ADTensor::new(
            neo_neural_engine::tensor::Tensor::from_vec_f64(&w.to_vec_f64().unwrap_or_default(),
                Shape::from_3d(out_channels, cin_per_group, k)), true));
        let b = if use_bias {
            Parameter::new("bias", ADTensor::zeros(Shape::from_1d(out_channels), DType::Float64, true))
        } else {
            Parameter::new("bias", ADTensor::zeros(Shape::from_1d(1), DType::Float64, false))
        };
        Self { weight, bias: b, in_channels, out_channels, kernel_size, stride, padding, dilation, groups, use_bias }
    }

    fn compute_output_len(&self, input_len: usize) -> usize {
        (input_len + 2 * self.padding - self.dilation * (self.kernel_size - 1) - 1) / self.stride + 1
    }
}

impl Module for Conv1D {
    fn forward(&self, input: &ADTensor) -> NnResult<ADTensor> {
        let dims = input.shape().dims();
        let batch = dims[0];
        let in_len = dims[dims.len() - 1];
        let out_len = self.compute_output_len(in_len);
        let cin_per_group = self.in_channels / self.groups;
        let out_size = batch * self.out_channels * out_len;
        let mut result_data = vec![0.0f64; out_size];

        let input_data = input.to_vec_f64()?;
        for b in 0..batch {
            for og in 0..self.groups {
                for oc in 0..(self.out_channels / self.groups) {
                    let out_c = og * (self.out_channels / self.groups) + oc;
                    for o in 0..out_len {
                        let mut sum = 0.0;
                        for ic in 0..cin_per_group {
                            let in_c = og * cin_per_group + ic;
                            for k in 0..self.kernel_size {
                                let in_pos = o * self.stride + k * self.dilation;
                                if in_pos >= self.padding && in_pos - self.padding < in_len {
                                    let ip = in_pos - self.padding;
                                    let in_idx = b * self.in_channels * in_len + in_c * in_len + ip;
                                    let w_idx = out_c * cin_per_group * self.kernel_size + ic * self.kernel_size + k;
                                    if in_idx < input_data.len() {
                                        let w_val = self.weight.tensor().data().item_f64(&[out_c, ic, k]).unwrap_or(0.0);
                                        sum += input_data[in_idx] * w_val;
                                    }
                                }
                            }
                        }
                        let out_idx = b * self.out_channels * out_len + out_c * out_len + o;
                        if out_idx < result_data.len() {
                            let bias_val = if self.use_bias { self.bias.tensor().data().item_f64(&[out_c]).unwrap_or(0.0) } else { 0.0 };
                            result_data[out_idx] = sum + bias_val;
                        }
                    }
                }
            }
        }

        let out_shape = if dims.len() == 3 {
            Shape::from_3d(batch, self.out_channels, out_len)
        } else {
            Shape::from_2d(batch * self.out_channels, out_len)
        };

        Ok(ADTensor::new(
            neo_neural_engine::tensor::Tensor::from_vec_f64(&result_data, out_shape),
            input.requires_grad(),
        ))
    }

    fn name(&self) -> &str { "Conv1D" }

    fn parameters(&self) -> HashMap<String, &ADTensor> {
        let mut p = HashMap::new();
        p.insert("weight".to_string(), self.weight.tensor());
        if self.use_bias { p.insert("bias".to_string(), self.bias.tensor()); }
        p
    }

    fn parameters_mut(&mut self) -> HashMap<String, &mut ADTensor> {
        let mut p = HashMap::new();
        p.insert("weight".to_string(), self.weight.tensor_mut());
        if self.use_bias { p.insert("bias".to_string(), self.bias.tensor_mut()); }
        p
    }

    fn num_parameters(&self) -> usize {
        let k = self.kernel_size;
        let cin_per_group = self.in_channels / self.groups;
        let n = self.out_channels * cin_per_group * k;
        if self.use_bias { n + self.out_channels } else { n }
    }
}

#[derive(Debug)]
pub struct Conv2D {
    weight: Parameter,
    bias: Parameter,
    in_channels: usize,
    out_channels: usize,
    kernel_size: (usize, usize),
    stride: (usize, usize),
    padding: (usize, usize),
    dilation: (usize, usize),
    groups: usize,
    use_bias: bool,
}

impl Conv2D {
    pub fn new(
        in_channels: usize, out_channels: usize, kernel_size: (usize, usize),
        stride: (usize, usize), padding: (usize, usize), dilation: (usize, usize),
        groups: usize, use_bias: bool,
    ) -> Self {
        let (kh, kw) = kernel_size;
        let cin_per_group = in_channels / groups;
        let w = init::xavier_uniform(out_channels * cin_per_group * kh * kw, DType::Float64, in_channels, kh * kw);
        let weight = Parameter::new("weight", ADTensor::new(w, true));
        let b = if use_bias {
            Parameter::new("bias", ADTensor::zeros(Shape::from_1d(out_channels), DType::Float64, true))
        } else {
            Parameter::new("bias", ADTensor::zeros(Shape::from_1d(1), DType::Float64, false))
        };
        Self { weight, bias: b, in_channels, out_channels, kernel_size, stride, padding, dilation, groups, use_bias }
    }

    fn compute_out(&self, input: usize, k: usize, s: usize, p: usize, d: usize) -> usize {
        (input + 2 * p - d * (k - 1) - 1) / s + 1
    }
}

impl Module for Conv2D {
    fn forward(&self, input: &ADTensor) -> NnResult<ADTensor> {
        let dims = input.shape().dims();
        let (batch, _, ih, iw) = (dims[0], dims[1], dims[2], dims[3]);
        let oh = self.compute_out(ih, self.kernel_size.0, self.stride.0, self.padding.0, self.dilation.0);
        let ow = self.compute_out(iw, self.kernel_size.1, self.stride.1, self.padding.1, self.dilation.1);
        let (kh, kw) = self.kernel_size;
        let cin_per_group = self.in_channels / self.groups;
        let out_size = batch * self.out_channels * oh * ow;
        let mut result_data = vec![0.0f64; out_size];

        let input_data = input.to_vec_f64()?;
        for b in 0..batch {
            for og in 0..self.groups {
                for oc in 0..(self.out_channels / self.groups) {
                    let out_c = og * (self.out_channels / self.groups) + oc;
                    for oh_idx in 0..oh {
                        for ow_idx in 0..ow {
                            let mut sum = 0.0;
                            for ic in 0..cin_per_group {
                                let in_c = og * cin_per_group + ic;
                                for kh_idx in 0..kh {
                                    for kw_idx in 0..kw {
                                        let ih_idx = oh_idx * self.stride.0 + kh_idx * self.dilation.0;
                                        let iw_idx = ow_idx * self.stride.1 + kw_idx * self.dilation.1;
                                        if ih_idx >= self.padding.0 && ih_idx - self.padding.0 < ih
                                            && iw_idx >= self.padding.1 && iw_idx - self.padding.1 < iw
                                        {
                                            let ip = ih_idx - self.padding.0;
                                            let jp = iw_idx - self.padding.1;
                                            let in_idx = b * self.in_channels * ih * iw + in_c * ih * iw + ip * iw + jp;
                                            let w_val = self.weight.tensor().data().item_f64(&[out_c, in_c, kh_idx, kw_idx]).unwrap_or(0.0);
                                            if in_idx < input_data.len() {
                                                sum += input_data[in_idx] * w_val;
                                            }
                                        }
                                    }
                                }
                            }
                            let out_idx = b * self.out_channels * oh * ow + out_c * oh * ow + oh_idx * ow + ow_idx;
                            let bias_val = if self.use_bias { self.bias.tensor().data().item_f64(&[out_c]).unwrap_or(0.0) } else { 0.0 };
                            if out_idx < result_data.len() {
                                result_data[out_idx] = sum + bias_val;
                            }
                        }
                    }
                }
            }
        }

        Ok(ADTensor::new(
            neo_neural_engine::tensor::Tensor::from_vec_f64(&result_data, Shape::new(vec![batch, self.out_channels, oh, ow])),
            input.requires_grad(),
        ))
    }

    fn name(&self) -> &str { "Conv2D" }

    fn parameters(&self) -> HashMap<String, &ADTensor> {
        let mut p = HashMap::new();
        p.insert("weight".to_string(), self.weight.tensor());
        if self.use_bias { p.insert("bias".to_string(), self.bias.tensor()); }
        p
    }

    fn parameters_mut(&mut self) -> HashMap<String, &mut ADTensor> {
        let mut p = HashMap::new();
        p.insert("weight".to_string(), self.weight.tensor_mut());
        if self.use_bias { p.insert("bias".to_string(), self.bias.tensor_mut()); }
        p
    }

    fn num_parameters(&self) -> usize {
        let (kh, kw) = self.kernel_size;
        let cin_per_group = self.in_channels / self.groups;
        let n = self.out_channels * cin_per_group * kh * kw;
        if self.use_bias { n + self.out_channels } else { n }
    }
}

#[derive(Debug)]
pub struct Conv3D {
    weight: Parameter,
    bias: Parameter,
    in_channels: usize,
    out_channels: usize,
    kernel_size: (usize, usize, usize),
    stride: (usize, usize, usize),
    padding: (usize, usize, usize),
    groups: usize,
    use_bias: bool,
}

impl Conv3D {
    pub fn new(
        in_channels: usize, out_channels: usize, kernel_size: (usize, usize, usize),
        stride: (usize, usize, usize), padding: (usize, usize, usize),
        groups: usize, use_bias: bool,
    ) -> Self {
        let (kd, kh, kw) = kernel_size;
        let cin_per_group = in_channels / groups;
        let w = init::xavier_uniform(out_channels * cin_per_group * kd * kh * kw, DType::Float64, in_channels, kd * kh * kw);
        let weight = Parameter::new("weight", ADTensor::new(w, true));
        let b = if use_bias {
            Parameter::new("bias", ADTensor::zeros(Shape::from_1d(out_channels), DType::Float64, true))
        } else {
            Parameter::new("bias", ADTensor::zeros(Shape::from_1d(1), DType::Float64, false))
        };
        Self { weight, bias: b, in_channels, out_channels, kernel_size, stride, padding, groups, use_bias }
    }
}

impl Module for Conv3D {
    fn forward(&self, input: &ADTensor) -> NnResult<ADTensor> {
        let dims = input.shape().dims();
        let batch = dims[0];
        let (kd, kh, kw) = self.kernel_size;
        let (sd, sh, sw) = self.stride;
        let (pd, ph, pw) = self.padding;
        let id = dims[2]; let ih = dims[3]; let iw = dims[4];
        let od = (id + 2 * pd - kd) / sd + 1;
        let oh = (ih + 2 * ph - kh) / sh + 1;
        let ow = (iw + 2 * pw - kw) / sw + 1;
        let cin_per_group = self.in_channels / self.groups;
        let out_size = batch * self.out_channels * od * oh * ow;
        let mut result_data = vec![0.0f64; out_size];
        let input_data = input.to_vec_f64()?;

        for b in 0..batch {
            for out_c in 0..self.out_channels {
                for d_idx in 0..od {
                    for h_idx in 0..oh {
                        for w_idx in 0..ow {
                            let mut sum = 0.0;
                            let og = out_c / (self.out_channels / self.groups);
                            for ic in 0..cin_per_group {
                                let in_c = og * cin_per_group + ic;
                                for kd_i in 0..kd {
                                    for kh_i in 0..kh {
                                        for kw_i in 0..kw {
                                            let id_i = d_idx * sd + kd_i - pd;
                                            let ih_i = h_idx * sh + kh_i - ph;
                                            let iw_i = w_idx * sw + kw_i - pw;
                                            if id_i < id && ih_i < ih && iw_i < iw {
                                                let in_idx = b * self.in_channels * id * ih * iw
                                                    + in_c * id * ih * iw + id_i * ih * iw + ih_i * iw + iw_i;
                                                let w_val = self.weight.tensor().data().item_f64(&[out_c, in_c, kd_i, kh_i, kw_i]).unwrap_or(0.0);
                                                if in_idx < input_data.len() {
                                                    sum += input_data[in_idx] * w_val;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            let out_idx = b * self.out_channels * od * oh * ow + out_c * od * oh * ow + d_idx * oh * ow + h_idx * ow + w_idx;
                            let bias_val = if self.use_bias { self.bias.tensor().data().item_f64(&[out_c]).unwrap_or(0.0) } else { 0.0 };
                            if out_idx < result_data.len() {
                                result_data[out_idx] = sum + bias_val;
                            }
                        }
                    }
                }
            }
        }

        Ok(ADTensor::new(
            neo_neural_engine::tensor::Tensor::from_vec_f64(&result_data, Shape::new(vec![batch, self.out_channels, od, oh, ow])),
            input.requires_grad(),
        ))
    }

    fn name(&self) -> &str { "Conv3D" }

    fn parameters(&self) -> HashMap<String, &ADTensor> {
        let mut p = HashMap::new();
        p.insert("weight".to_string(), self.weight.tensor());
        if self.use_bias { p.insert("bias".to_string(), self.bias.tensor()); }
        p
    }

    fn parameters_mut(&mut self) -> HashMap<String, &mut ADTensor> {
        let mut p = HashMap::new();
        p.insert("weight".to_string(), self.weight.tensor_mut());
        if self.use_bias { p.insert("bias".to_string(), self.bias.tensor_mut()); }
        p
    }

    fn num_parameters(&self) -> usize {
        let (kd, kh, kw) = self.kernel_size;
        let cin_per_group = self.in_channels / self.groups;
        let n = self.out_channels * cin_per_group * kd * kh * kw;
        if self.use_bias { n + self.out_channels } else { n }
    }
}

#[derive(Debug)]
pub struct TransposeConv {
    inner: Conv2D,
    output_padding: (usize, usize),
}

impl TransposeConv {
    pub fn new(
        in_channels: usize, out_channels: usize, kernel_size: (usize, usize),
        stride: (usize, usize), padding: (usize, usize),
        output_padding: (usize, usize), groups: usize, use_bias: bool,
    ) -> Self {
        Self {
            inner: Conv2D::new(out_channels, in_channels, kernel_size, stride, padding, (1, 1), groups, use_bias),
            output_padding,
        }
    }
}

impl Module for TransposeConv {
    fn forward(&self, input: &ADTensor) -> NnResult<ADTensor> {
        self.inner.forward(input)
    }

    fn name(&self) -> &str { "TransposeConv" }

    fn parameters(&self) -> HashMap<String, &ADTensor> {
        self.inner.parameters()
    }

    fn parameters_mut(&mut self) -> HashMap<String, &mut ADTensor> {
        self.inner.parameters_mut()
    }

    fn num_parameters(&self) -> usize {
        self.inner.num_parameters()
    }
}

#[derive(Debug)]
pub struct DepthwiseConv {
    inner: Conv2D,
}

impl DepthwiseConv {
    pub fn new(
        channels: usize, kernel_size: (usize, usize),
        stride: (usize, usize), padding: (usize, usize),
        dilation: (usize, usize), use_bias: bool,
    ) -> Self {
        Self {
            inner: Conv2D::new(channels, channels, kernel_size, stride, padding, dilation, channels, use_bias),
        }
    }
}

impl Module for DepthwiseConv {
    fn forward(&self, input: &ADTensor) -> NnResult<ADTensor> {
        self.inner.forward(input)
    }

    fn name(&self) -> &str { "DepthwiseConv" }

    fn parameters(&self) -> HashMap<String, &ADTensor> { self.inner.parameters() }

    fn parameters_mut(&mut self) -> HashMap<String, &mut ADTensor> { self.inner.parameters_mut() }

    fn num_parameters(&self) -> usize { self.inner.num_parameters() }
}

#[derive(Debug)]
pub struct PointwiseConv {
    inner: Conv2D,
}

impl PointwiseConv {
    pub fn new(in_channels: usize, out_channels: usize, use_bias: bool) -> Self {
        Self {
            inner: Conv2D::new(in_channels, out_channels, (1, 1), (1, 1), (0, 0), (1, 1), 1, use_bias),
        }
    }
}

impl Module for PointwiseConv {
    fn forward(&self, input: &ADTensor) -> NnResult<ADTensor> {
        self.inner.forward(input)
    }

    fn name(&self) -> &str { "PointwiseConv" }

    fn parameters(&self) -> HashMap<String, &ADTensor> { self.inner.parameters() }

    fn parameters_mut(&mut self) -> HashMap<String, &mut ADTensor> { self.inner.parameters_mut() }

    fn num_parameters(&self) -> usize { self.inner.num_parameters() }
}

#[derive(Debug)]
pub struct GroupedConv {
    inner: Conv2D,
}

impl GroupedConv {
    pub fn new(
        in_channels: usize, out_channels: usize, kernel_size: (usize, usize),
        stride: (usize, usize), padding: (usize, usize),
        groups: usize, use_bias: bool,
    ) -> Self {
        Self {
            inner: Conv2D::new(in_channels, out_channels, kernel_size, stride, padding, (1, 1), groups, use_bias),
        }
    }
}

impl Module for GroupedConv {
    fn forward(&self, input: &ADTensor) -> NnResult<ADTensor> {
        self.inner.forward(input)
    }

    fn name(&self) -> &str { "GroupedConv" }

    fn parameters(&self) -> HashMap<String, &ADTensor> { self.inner.parameters() }

    fn parameters_mut(&mut self) -> HashMap<String, &mut ADTensor> { self.inner.parameters_mut() }

    fn num_parameters(&self) -> usize { self.inner.num_parameters() }
}

#[derive(Debug)]
pub struct DilatedConv {
    inner: Conv2D,
}

impl DilatedConv {
    pub fn new(
        in_channels: usize, out_channels: usize, kernel_size: (usize, usize),
        stride: (usize, usize), padding: (usize, usize),
        dilation: (usize, usize), groups: usize, use_bias: bool,
    ) -> Self {
        Self {
            inner: Conv2D::new(in_channels, out_channels, kernel_size, stride, padding, dilation, groups, use_bias),
        }
    }
}

impl Module for DilatedConv {
    fn forward(&self, input: &ADTensor) -> NnResult<ADTensor> {
        self.inner.forward(input)
    }

    fn name(&self) -> &str { "DilatedConv" }

    fn parameters(&self) -> HashMap<String, &ADTensor> { self.inner.parameters() }

    fn parameters_mut(&mut self) -> HashMap<String, &mut ADTensor> { self.inner.parameters_mut() }

    fn num_parameters(&self) -> usize { self.inner.num_parameters() }
}
