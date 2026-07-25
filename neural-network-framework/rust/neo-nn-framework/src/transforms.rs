use crate::autograd::ADTensor;
use crate::error::NnResult;
use neo_neural_engine::shape::Shape;

pub fn normalize(tensor: &ADTensor, mean: &[f64], std: &[f64]) -> NnResult<ADTensor> {
    let data = tensor.to_vec_f64()?;
    let ndim = tensor.ndim();
    let dims = tensor.shape().dims();
    let channels = if ndim >= 3 { dims[ndim - 3] } else { 1 };
    let spatial: usize = if ndim >= 3 {
        dims[ndim - 2] * dims[ndim - 1]
    } else {
        dims.iter().product::<usize>() / channels.max(1)
    };

    let mut result = Vec::with_capacity(data.len());
    for (i, &v) in data.iter().enumerate() {
        let c = (i / spatial) % channels;
        let m = if c < mean.len() { mean[c] } else { 0.0 };
        let s = if c < std.len() { std[c] } else { 1.0 };
        result.push((v - m) / s);
    }

    Ok(ADTensor::new(
        neo_neural_engine::tensor::Tensor::from_vec_f64(&result, tensor.shape().clone()),
        tensor.requires_grad(),
    ))
}

pub fn resize_2d(tensor: &ADTensor, new_h: usize, new_w: usize) -> NnResult<ADTensor> {
    let dims = tensor.shape().dims();
    let (batch, channels, old_h, old_w) = (dims[0], dims[1], dims[2], dims[3]);
    let data = tensor.to_vec_f64()?;
    let mut result = vec![0.0f64; batch * channels * new_h * new_w];

    for b in 0..batch {
        for c in 0..channels {
            for nh in 0..new_h {
                for nw in 0..new_w {
                    let oh = (nh * old_h) / new_h;
                    let ow = (nw * old_w) / new_w;
                    let src_idx = b * channels * old_h * old_w + c * old_h * old_w + oh * old_w + ow;
                    let dst_idx = b * channels * new_h * new_w + c * new_h * new_w + nh * new_w + nw;
                    if src_idx < data.len() && dst_idx < result.len() {
                        result[dst_idx] = data[src_idx];
                    }
                }
            }
        }
    }

    Ok(ADTensor::new(
        neo_neural_engine::tensor::Tensor::from_vec_f64(&result, Shape::from_4d(batch, channels, new_h, new_w)),
        tensor.requires_grad(),
    ))
}

pub fn crop_2d(tensor: &ADTensor, h_start: usize, h_end: usize, w_start: usize, w_end: usize) -> NnResult<ADTensor> {
    let dims = tensor.shape().dims();
    let (batch, channels) = (dims[0], dims[1]);
    let old_w = dims[3];
    let data = tensor.to_vec_f64()?;
    let new_h = h_end - h_start;
    let new_w = w_end - w_start;
    let mut result = vec![0.0f64; batch * channels * new_h * new_w];

    for b in 0..batch {
        for c in 0..channels {
            for nh in 0..new_h {
                for nw in 0..new_w {
                    let oh = h_start + nh;
                    let ow = w_start + nw;
                    let src_idx = b * channels * dims[2] * old_w + c * dims[2] * old_w + oh * old_w + ow;
                    let dst_idx = b * channels * new_h * new_w + c * new_h * new_w + nh * new_w + nw;
                    if src_idx < data.len() && dst_idx < result.len() {
                        result[dst_idx] = data[src_idx];
                    }
                }
            }
        }
    }

    Ok(ADTensor::new(
        neo_neural_engine::tensor::Tensor::from_vec_f64(&result, Shape::from_4d(batch, channels, new_h, new_w)),
        tensor.requires_grad(),
    ))
}

pub fn pad_2d(tensor: &ADTensor, padding: (usize, usize, usize, usize)) -> NnResult<ADTensor> {
    let (ph_top, ph_bottom, pw_left, pw_right) = padding;
    let dims = tensor.shape().dims();
    let (batch, channels, h, w) = (dims[0], dims[1], dims[2], dims[3]);
    let new_h = h + ph_top + ph_bottom;
    let new_w = w + pw_left + pw_right;
    let data = tensor.to_vec_f64()?;
    let mut result = vec![0.0f64; batch * channels * new_h * new_w];

    for b in 0..batch {
        for c in 0..channels {
            for oh in 0..h {
                for ow in 0..w {
                    let src_idx = b * channels * h * w + c * h * w + oh * w + ow;
                    let dst_idx = b * channels * new_h * new_w + c * new_h * new_w + (oh + ph_top) * new_w + (ow + pw_left);
                    if src_idx < data.len() && dst_idx < result.len() {
                        result[dst_idx] = data[src_idx];
                    }
                }
            }
        }
    }

    Ok(ADTensor::new(
        neo_neural_engine::tensor::Tensor::from_vec_f64(&result, Shape::from_4d(batch, channels, new_h, new_w)),
        tensor.requires_grad(),
    ))
}

pub fn flip_horizontal(tensor: &ADTensor) -> NnResult<ADTensor> {
    let dims = tensor.shape().dims();
    let (batch, channels, h, w) = (dims[0], dims[1], dims[2], dims[3]);
    let data = tensor.to_vec_f64()?;
    let mut result = vec![0.0f64; data.len()];

    for b in 0..batch {
        for c in 0..channels {
            for oh in 0..h {
                for ow in 0..w {
                    let src_idx = b * channels * h * w + c * h * w + oh * w + ow;
                    let dst_idx = b * channels * h * w + c * h * w + oh * w + (w - 1 - ow);
                    if src_idx < data.len() && dst_idx < result.len() {
                        result[dst_idx] = data[src_idx];
                    }
                }
            }
        }
    }

    Ok(ADTensor::new(
        neo_neural_engine::tensor::Tensor::from_vec_f64(&result, tensor.shape().clone()),
        tensor.requires_grad(),
    ))
}

pub fn flip_vertical(tensor: &ADTensor) -> NnResult<ADTensor> {
    let dims = tensor.shape().dims();
    let (batch, channels, h, w) = (dims[0], dims[1], dims[2], dims[3]);
    let data = tensor.to_vec_f64()?;
    let mut result = vec![0.0f64; data.len()];

    for b in 0..batch {
        for c in 0..channels {
            for oh in 0..h {
                for ow in 0..w {
                    let src_idx = b * channels * h * w + c * h * w + oh * w + ow;
                    let dst_idx = b * channels * h * w + c * h * w + (h - 1 - oh) * w + ow;
                    if src_idx < data.len() && dst_idx < result.len() {
                        result[dst_idx] = data[src_idx];
                    }
                }
            }
        }
    }

    Ok(ADTensor::new(
        neo_neural_engine::tensor::Tensor::from_vec_f64(&result, tensor.shape().clone()),
        tensor.requires_grad(),
    ))
}

pub fn rotate_90(tensor: &ADTensor) -> NnResult<ADTensor> {
    let dims = tensor.shape().dims();
    let (batch, channels, h, w) = (dims[0], dims[1], dims[2], dims[3]);
    let data = tensor.to_vec_f64()?;
    let mut result = vec![0.0f64; data.len()];

    for b in 0..batch {
        for c in 0..channels {
            for oh in 0..h {
                for ow in 0..w {
                    let src_idx = b * channels * h * w + c * h * w + oh * w + ow;
                    let dst_idx = b * channels * h * w + c * h * w + ow * h + (h - 1 - oh);
                    if src_idx < data.len() && dst_idx < result.len() {
                        result[dst_idx] = data[src_idx];
                    }
                }
            }
        }
    }

    Ok(ADTensor::new(
        neo_neural_engine::tensor::Tensor::from_vec_f64(&result, Shape::from_4d(batch, channels, w, h)),
        tensor.requires_grad(),
    ))
}

pub fn add_noise(tensor: &ADTensor, std: f64) -> NnResult<ADTensor> {
    use rand::Rng;
    let data = tensor.to_vec_f64()?;
    let mut rng = rand::thread_rng();
    let noised: Vec<f64> = data.iter()
        .map(|&v| v + rng.gen_range(-std..std))
        .collect();
    Ok(ADTensor::new(
        neo_neural_engine::tensor::Tensor::from_vec_f64(&noised, tensor.shape().clone()),
        tensor.requires_grad(),
    ))
}

pub fn to_tensor(data: &[f64], shape: Vec<usize>) -> ADTensor {
    ADTensor::from_vec_f64(data, Shape::new(shape), false)
}
