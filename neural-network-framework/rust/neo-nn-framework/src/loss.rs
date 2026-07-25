use crate::autograd::ADTensor;
use crate::error::NnResult;

pub fn cross_entropy_loss(logits: &ADTensor, targets: &ADTensor) -> NnResult<ADTensor> {
    let logits_data = logits.to_vec_f64()?;
    let targets_data = targets.to_vec_f64()?;
    let dims = logits.shape().dims();
    let num_classes = *dims.last().unwrap_or(&1);
    let batch_size = dims[0];
    let mut total_loss = 0.0;

    for b in 0..batch_size {
        let target_idx = targets_data[b] as usize;
        let logits_offset = b * num_classes;
        let mut max_val = f64::NEG_INFINITY;
        for c in 0..num_classes {
            max_val = max_val.max(logits_data[logits_offset + c]);
        }
        let mut log_sum_exp = 0.0;
        for c in 0..num_classes {
            log_sum_exp += (logits_data[logits_offset + c] - max_val).exp();
        }
        total_loss += max_val + log_sum_exp.ln() - logits_data[logits_offset + target_idx];
    }

    let loss_val = total_loss / batch_size as f64;
    Ok(ADTensor::new(
        neo_neural_engine::tensor::Tensor::from_vec_f64(&[loss_val], neo_neural_engine::shape::Shape::from_1d(1)),
        true,
    ))
}

pub fn binary_cross_entropy_loss(logits: &ADTensor, targets: &ADTensor) -> NnResult<ADTensor> {
    let logits_data = logits.to_vec_f64()?;
    let targets_data = targets.to_vec_f64()?;
    let eps = 1e-7;
    let mut total_loss = 0.0;
    let n = logits_data.len();

    for i in 0..n {
        let p = logits_data[i].clamp(eps, 1.0 - eps);
        let t = targets_data[i];
        total_loss -= t * p.ln() + (1.0 - t) * (1.0 - p).ln();
    }

    let loss_val = total_loss / n as f64;
    Ok(ADTensor::new(
        neo_neural_engine::tensor::Tensor::from_vec_f64(&[loss_val], neo_neural_engine::shape::Shape::from_1d(1)),
        true,
    ))
}

pub fn focal_loss(logits: &ADTensor, targets: &ADTensor, gamma: f64) -> NnResult<ADTensor> {
    let logits_data = logits.to_vec_f64()?;
    let targets_data = targets.to_vec_f64()?;
    let eps = 1e-7;
    let mut total_loss = 0.0;
    let n = logits_data.len();

    for i in 0..n {
        let p = logits_data[i].clamp(eps, 1.0 - eps);
        let t = targets_data[i];
        let fl = -(1.0 - p).powf(gamma) * t * p.ln() - p.powf(gamma) * (1.0 - t) * (1.0 - p).ln();
        total_loss += fl;
    }

    let loss_val = total_loss / n as f64;
    Ok(ADTensor::new(
        neo_neural_engine::tensor::Tensor::from_vec_f64(&[loss_val], neo_neural_engine::shape::Shape::from_1d(1)),
        true,
    ))
}

pub fn mse_loss(predictions: &ADTensor, targets: &ADTensor) -> NnResult<ADTensor> {
    let pred = predictions.to_vec_f64()?;
    let targ = targets.to_vec_f64()?;
    let n = pred.len();
    let mut total = 0.0;
    for i in 0..n {
        let diff = pred[i] - targ[i];
        total += diff * diff;
    }
    let loss_val = total / n as f64;
    Ok(ADTensor::new(
        neo_neural_engine::tensor::Tensor::from_vec_f64(&[loss_val], neo_neural_engine::shape::Shape::from_1d(1)),
        true,
    ))
}

pub fn mae_loss(predictions: &ADTensor, targets: &ADTensor) -> NnResult<ADTensor> {
    let pred = predictions.to_vec_f64()?;
    let targ = targets.to_vec_f64()?;
    let n = pred.len();
    let mut total = 0.0;
    for i in 0..n {
        total += (pred[i] - targ[i]).abs();
    }
    let loss_val = total / n as f64;
    Ok(ADTensor::new(
        neo_neural_engine::tensor::Tensor::from_vec_f64(&[loss_val], neo_neural_engine::shape::Shape::from_1d(1)),
        true,
    ))
}

pub fn huber_loss(predictions: &ADTensor, targets: &ADTensor, delta: f64) -> NnResult<ADTensor> {
    let pred = predictions.to_vec_f64()?;
    let targ = targets.to_vec_f64()?;
    let n = pred.len();
    let mut total = 0.0;
    for i in 0..n {
        let diff = pred[i] - targ[i];
        let abs_diff = diff.abs();
        if abs_diff <= delta {
            total += 0.5 * diff * diff;
        } else {
            total += delta * abs_diff - 0.5 * delta * delta;
        }
    }
    let loss_val = total / n as f64;
    Ok(ADTensor::new(
        neo_neural_engine::tensor::Tensor::from_vec_f64(&[loss_val], neo_neural_engine::shape::Shape::from_1d(1)),
        true,
    ))
}

pub fn kl_divergence_loss(predictions: &ADTensor, targets: &ADTensor) -> NnResult<ADTensor> {
    let pred = predictions.to_vec_f64()?;
    let targ = targets.to_vec_f64()?;
    let eps = 1e-7;
    let n = pred.len();
    let mut total = 0.0;
    for i in 0..n {
        let p = targ[i];
        let q = pred[i].clamp(eps, 1.0);
        if p > eps {
            total += p * (p / q).ln();
        }
    }
    let loss_val = total;
    Ok(ADTensor::new(
        neo_neural_engine::tensor::Tensor::from_vec_f64(&[loss_val], neo_neural_engine::shape::Shape::from_1d(1)),
        true,
    ))
}

pub fn triplet_loss(anchor: &ADTensor, positive: &ADTensor, negative: &ADTensor, margin: f64) -> NnResult<ADTensor> {
    let a = anchor.to_vec_f64()?;
    let p = positive.to_vec_f64()?;
    let n = negative.to_vec_f64()?;
    let mut pos_dist = 0.0;
    let mut neg_dist = 0.0;
    for i in 0..a.len() {
        let d = a[i] - p[i];
        pos_dist += d * d;
        let d = a[i] - n[i];
        neg_dist += d * d;
    }
    pos_dist = pos_dist.sqrt();
    neg_dist = neg_dist.sqrt();
    let loss_val = (pos_dist - neg_dist + margin).max(0.0);
    Ok(ADTensor::new(
        neo_neural_engine::tensor::Tensor::from_vec_f64(&[loss_val], neo_neural_engine::shape::Shape::from_1d(1)),
        true,
    ))
}

pub fn contrastive_loss(anchor: &ADTensor, positive: &ADTensor, target: &ADTensor, margin: f64) -> NnResult<ADTensor> {
    let a = anchor.to_vec_f64()?;
    let p = positive.to_vec_f64()?;
    let t = target.to_vec_f64()?;
    let mut dist = 0.0;
    for i in 0..a.len() {
        let d = a[i] - p[i];
        dist += d * d;
    }
    dist = dist.sqrt();
    let loss_val = t[0] * dist.powi(2) + (1.0 - t[0]) * (margin - dist).max(0.0).powi(2);
    Ok(ADTensor::new(
        neo_neural_engine::tensor::Tensor::from_vec_f64(&[loss_val], neo_neural_engine::shape::Shape::from_1d(1)),
        true,
    ))
}

pub fn dice_loss(predictions: &ADTensor, targets: &ADTensor, smooth: f64) -> NnResult<ADTensor> {
    let pred = predictions.to_vec_f64()?;
    let targ = targets.to_vec_f64()?;
    let n = pred.len();
    let mut intersection = 0.0;
    let mut sum_pred = 0.0;
    let mut sum_targ = 0.0;
    for i in 0..n {
        intersection += pred[i] * targ[i];
        sum_pred += pred[i];
        sum_targ += targ[i];
    }
    let dice = (2.0 * intersection + smooth) / (sum_pred + sum_targ + smooth);
    let loss_val = 1.0 - dice;
    Ok(ADTensor::new(
        neo_neural_engine::tensor::Tensor::from_vec_f64(&[loss_val], neo_neural_engine::shape::Shape::from_1d(1)),
        true,
    ))
}
