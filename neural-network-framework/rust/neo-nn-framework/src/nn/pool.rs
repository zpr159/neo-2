use std::collections::HashMap;

use crate::autograd::ADTensor;
use crate::error::NnResult;
use crate::module::Module;

fn max_pool_1d(data: &[f64], input_len: usize, kernel: usize, stride: usize, padding: usize) -> Vec<f64> {
    let out_len = (input_len + 2 * padding - kernel) / stride + 1;
    let mut result = Vec::with_capacity(out_len);
    for o in 0..out_len {
        let start = o * stride;
        let mut max_val = f64::NEG_INFINITY;
        for k in 0..kernel {
            let pos = start + k;
            if pos >= padding && pos - padding < input_len {
                let idx = pos - padding;
                if idx < input_len && idx < data.len() {
                    max_val = max_val.max(data[idx]);
                }
            }
        }
        result.push(if max_val == f64::NEG_INFINITY { 0.0 } else { max_val });
    }
    result
}

fn avg_pool_1d(data: &[f64], input_len: usize, kernel: usize, stride: usize, padding: usize) -> Vec<f64> {
    let out_len = (input_len + 2 * padding - kernel) / stride + 1;
    let mut result = Vec::with_capacity(out_len);
    for o in 0..out_len {
        let start = o * stride;
        let mut sum = 0.0;
        let mut count = 0;
        for k in 0..kernel {
            let pos = start + k;
            if pos >= padding && pos - padding < input_len {
                let idx = pos - padding;
                if idx < input_len && idx < data.len() {
                    sum += data[idx];
                    count += 1;
                }
            }
        }
        result.push(if count > 0 { sum / count as f64 } else { 0.0 });
    }
    result
}

#[derive(Debug)]
pub struct MaxPool1D { kernel_size: usize, stride: usize, padding: usize }

impl MaxPool1D {
    pub fn new(kernel_size: usize, stride: usize, padding: usize) -> Self {
        Self { kernel_size, stride, padding }
    }
}

impl Module for MaxPool1D {
    fn forward(&self, input: &ADTensor) -> NnResult<ADTensor> {
        let dims = input.shape().dims();
        let batch = dims[0];
        let channels = dims[1];
        let input_len = dims[2];
        let out_len = (input_len + 2 * self.padding - self.kernel_size) / self.stride + 1;
        let input_data = input.to_vec_f64()?;
        let mut result_data = Vec::with_capacity(batch * channels * out_len);

        for b in 0..batch {
            for c in 0..channels {
                let offset = b * channels * input_len + c * input_len;
                let slice = &input_data[offset..offset + input_len];
                let pooled = max_pool_1d(slice, input_len, self.kernel_size, self.stride, self.padding);
                result_data.extend_from_slice(&pooled);
            }
        }

        Ok(ADTensor::new(
            neo_neural_engine::tensor::Tensor::from_vec_f64(&result_data, Shape::from_3d(batch, channels, out_len)),
            false,
        ))
    }

    fn name(&self) -> &str { "MaxPool1D" }
}

#[derive(Debug)]
pub struct AvgPool1D { kernel_size: usize, stride: usize, padding: usize }

impl AvgPool1D {
    pub fn new(kernel_size: usize, stride: usize, padding: usize) -> Self {
        Self { kernel_size, stride, padding }
    }
}

impl Module for AvgPool1D {
    fn forward(&self, input: &ADTensor) -> NnResult<ADTensor> {
        let dims = input.shape().dims();
        let batch = dims[0];
        let channels = dims[1];
        let input_len = dims[2];
        let out_len = (input_len + 2 * self.padding - self.kernel_size) / self.stride + 1;
        let input_data = input.to_vec_f64()?;
        let mut result_data = Vec::with_capacity(batch * channels * out_len);

        for b in 0..batch {
            for c in 0..channels {
                let offset = b * channels * input_len + c * input_len;
                let slice = &input_data[offset..offset + input_len];
                let pooled = avg_pool_1d(slice, input_len, self.kernel_size, self.stride, self.padding);
                result_data.extend_from_slice(&pooled);
            }
        }

        Ok(ADTensor::new(
            neo_neural_engine::tensor::Tensor::from_vec_f64(&result_data, Shape::from_3d(batch, channels, out_len)),
            false,
        ))
    }

    fn name(&self) -> &str { "AvgPool1D" }
}

#[derive(Debug)]
pub struct MaxPool2D { kernel_size: (usize, usize), stride: (usize, usize), padding: (usize, usize) }

impl MaxPool2D {
    pub fn new(kernel_size: (usize, usize), stride: (usize, usize), padding: (usize, usize)) -> Self {
        Self { kernel_size, stride, padding }
    }
}

impl Module for MaxPool2D {
    fn forward(&self, input: &ADTensor) -> NnResult<ADTensor> {
        let dims = input.shape().dims();
        let (batch, channels, ih, iw) = (dims[0], dims[1], dims[2], dims[3]);
        let (kh, kw) = self.kernel_size;
        let (sh, sw) = self.stride;
        let (ph, pw) = self.padding;
        let oh = (ih + 2 * ph - kh) / sh + 1;
        let ow = (iw + 2 * pw - kw) / sw + 1;
        let input_data = input.to_vec_f64()?;
        let mut result_data = vec![0.0f64; batch * channels * oh * ow];

        for b in 0..batch {
            for c in 0..channels {
                for oh_i in 0..oh {
                    for ow_i in 0..ow {
                        let mut max_val = f64::NEG_INFINITY;
                        for kh_i in 0..kh {
                            for kw_i in 0..kw {
                                let ih_i = oh_i * sh + kh_i;
                                let iw_i = ow_i * sw + kw_i;
                                if ih_i >= ph && ih_i - ph < ih && iw_i >= pw && iw_i - pw < iw {
                                    let ip = ih_i - ph;
                                    let jp = iw_i - pw;
                                    let idx = b * channels * ih * iw + c * ih * iw + ip * iw + jp;
                                    if idx < input_data.len() {
                                        max_val = max_val.max(input_data[idx]);
                                    }
                                }
                            }
                        }
                        let out_idx = b * channels * oh * ow + c * oh * ow + oh_i * ow + ow_i;
                        result_data[out_idx] = if max_val == f64::NEG_INFINITY { 0.0 } else { max_val };
                    }
                }
            }
        }

        Ok(ADTensor::new(
            neo_neural_engine::tensor::Tensor::from_vec_f64(&result_data, Shape::from_4d(batch, channels, oh, ow)),
            false,
        ))
    }

    fn name(&self) -> &str { "MaxPool2D" }
}

#[derive(Debug)]
pub struct AvgPool2D { kernel_size: (usize, usize), stride: (usize, usize), padding: (usize, usize) }

impl AvgPool2D {
    pub fn new(kernel_size: (usize, usize), stride: (usize, usize), padding: (usize, usize)) -> Self {
        Self { kernel_size, stride, padding }
    }
}

impl Module for AvgPool2D {
    fn forward(&self, input: &ADTensor) -> NnResult<ADTensor> {
        let dims = input.shape().dims();
        let (batch, channels, ih, iw) = (dims[0], dims[1], dims[2], dims[3]);
        let (kh, kw) = self.kernel_size;
        let (sh, sw) = self.stride;
        let (ph, pw) = self.padding;
        let oh = (ih + 2 * ph - kh) / sh + 1;
        let ow = (iw + 2 * pw - kw) / sw + 1;
        let input_data = input.to_vec_f64()?;
        let mut result_data = vec![0.0f64; batch * channels * oh * ow];

        for b in 0..batch {
            for c in 0..channels {
                for oh_i in 0..oh {
                    for ow_i in 0..ow {
                        let mut sum = 0.0;
                        let mut count = 0;
                        for kh_i in 0..kh {
                            for kw_i in 0..kw {
                                let ih_i = oh_i * sh + kh_i;
                                let iw_i = ow_i * sw + kw_i;
                                if ih_i >= ph && ih_i - ph < ih && iw_i >= pw && iw_i - pw < iw {
                                    let ip = ih_i - ph;
                                    let jp = iw_i - pw;
                                    let idx = b * channels * ih * iw + c * ih * iw + ip * iw + jp;
                                    if idx < input_data.len() {
                                        sum += input_data[idx];
                                        count += 1;
                                    }
                                }
                            }
                        }
                        let out_idx = b * channels * oh * ow + c * oh * ow + oh_i * ow + ow_i;
                        result_data[out_idx] = if count > 0 { sum / count as f64 } else { 0.0 };
                    }
                }
            }
        }

        Ok(ADTensor::new(
            neo_neural_engine::tensor::Tensor::from_vec_f64(&result_data, Shape::from_4d(batch, channels, oh, ow)),
            false,
        ))
    }

    fn name(&self) -> &str { "AvgPool2D" }
}

#[derive(Debug)]
pub struct AdaptiveAvgPool1D { output_size: usize }

impl AdaptiveAvgPool1D {
    pub fn new(output_size: usize) -> Self {
        Self { output_size }
    }
}

impl Module for AdaptiveAvgPool1D {
    fn forward(&self, input: &ADTensor) -> NnResult<ADTensor> {
        let dims = input.shape().dims();
        let (batch, channels, input_len) = (dims[0], dims[1], dims[2]);
        let input_data = input.to_vec_f64()?;
        let mut result_data = Vec::with_capacity(batch * channels * self.output_size);

        for b in 0..batch {
            for c in 0..channels {
                for o in 0..self.output_size {
                    let start = (o * input_len) / self.output_size;
                    let end = ((o + 1) * input_len) / self.output_size;
                    let mut sum = 0.0;
                    let mut count = 0;
                    for i in start..end {
                        let idx = b * channels * input_len + c * input_len + i;
                        if idx < input_data.len() {
                            sum += input_data[idx];
                            count += 1;
                        }
                    }
                    result_data.push(if count > 0 { sum / count as f64 } else { 0.0 });
                }
            }
        }

        Ok(ADTensor::new(
            neo_neural_engine::tensor::Tensor::from_vec_f64(&result_data, Shape::from_3d(batch, channels, self.output_size)),
            false,
        ))
    }

    fn name(&self) -> &str { "AdaptiveAvgPool1D" }
}

#[derive(Debug)]
pub struct AdaptiveAvgPool2D { output_size: (usize, usize) }

impl AdaptiveAvgPool2D {
    pub fn new(output_size: (usize, usize)) -> Self {
        Self { output_size }
    }
}

impl Module for AdaptiveAvgPool2D {
    fn forward(&self, input: &ADTensor) -> NnResult<ADTensor> {
        let dims = input.shape().dims();
        let (batch, channels, ih, iw) = (dims[0], dims[1], dims[2], dims[3]);
        let (oh, ow) = self.output_size;
        let input_data = input.to_vec_f64()?;
        let mut result_data = vec![0.0f64; batch * channels * oh * ow];

        for b in 0..batch {
            for c in 0..channels {
                for h in 0..oh {
                    for w in 0..ow {
                        let h_start = (h * ih) / oh;
                        let h_end = ((h + 1) * ih) / oh;
                        let w_start = (w * iw) / ow;
                        let w_end = ((w + 1) * iw) / ow;
                        let mut sum = 0.0;
                        let mut count = 0;
                        for hi in h_start..h_end {
                            for wi in w_start..w_end {
                                let idx = b * channels * ih * iw + c * ih * iw + hi * iw + wi;
                                if idx < input_data.len() {
                                    sum += input_data[idx];
                                    count += 1;
                                }
                            }
                        }
                        let out_idx = b * channels * oh * ow + c * oh * ow + h * ow + w;
                        result_data[out_idx] = if count > 0 { sum / count as f64 } else { 0.0 };
                    }
                }
            }
        }

        Ok(ADTensor::new(
            neo_neural_engine::tensor::Tensor::from_vec_f64(&result_data, Shape::from_4d(batch, channels, oh, ow)),
            false,
        ))
    }

    fn name(&self) -> &str { "AdaptiveAvgPool2D" }
}

#[derive(Debug)]
pub struct GlobalAvgPool;

impl GlobalAvgPool {
    pub fn new() -> Self { Self }
}

impl Default for GlobalAvgPool {
    fn default() -> Self { Self::new() }
}

impl Module for GlobalAvgPool {
    fn forward(&self, input: &ADTensor) -> NnResult<ADTensor> {
        let dims = input.shape().dims();
        let (batch, channels) = (dims[0], dims[1]);
        let spatial: usize = dims[2..].iter().copied().product();
        let input_data = input.to_vec_f64()?;
        let mut result_data = Vec::with_capacity(batch * channels);

        for b in 0..batch {
            for c in 0..channels {
                let mut sum = 0.0;
                for s in 0..spatial {
                    let idx = b * channels * spatial + c * spatial + s;
                    if idx < input_data.len() {
                        sum += input_data[idx];
                    }
                }
                result_data.push(sum / spatial as f64);
            }
        }

        Ok(ADTensor::new(
            neo_neural_engine::tensor::Tensor::from_vec_f64(&result_data, Shape::from_2d(batch, channels)),
            false,
        ))
    }

    fn name(&self) -> &str { "GlobalAvgPool" }
}

#[derive(Debug)]
pub struct GlobalMaxPool;

impl GlobalMaxPool {
    pub fn new() -> Self { Self }
}

impl Default for GlobalMaxPool {
    fn default() -> Self { Self::new() }
}

impl Module for GlobalMaxPool {
    fn forward(&self, input: &ADTensor) -> NnResult<ADTensor> {
        let dims = input.shape().dims();
        let (batch, channels) = (dims[0], dims[1]);
        let spatial: usize = dims[2..].iter().copied().product();
        let input_data = input.to_vec_f64()?;
        let mut result_data = Vec::with_capacity(batch * channels);

        for b in 0..batch {
            for c in 0..channels {
                let mut max_val = f64::NEG_INFINITY;
                for s in 0..spatial {
                    let idx = b * channels * spatial + c * spatial + s;
                    if idx < input_data.len() {
                        max_val = max_val.max(input_data[idx]);
                    }
                }
                result_data.push(if max_val == f64::NEG_INFINITY { 0.0 } else { max_val });
            }
        }

        Ok(ADTensor::new(
            neo_neural_engine::tensor::Tensor::from_vec_f64(&result_data, Shape::from_2d(batch, channels)),
            false,
        ))
    }

    fn name(&self) -> &str { "GlobalMaxPool" }
}

use neo_neural_engine::shape::Shape;
