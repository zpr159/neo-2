use neo_neural_engine::shape::Shape;
use neo_neural_engine::tensor::Tensor;
use neo_neural_engine::DType;
use rand::Rng;

pub fn xavier_uniform(numel: usize, dtype: DType, fan_in: usize, fan_out: usize) -> Tensor {
    let std = (2.0 / (fan_in + fan_out) as f64).sqrt();
    let mut rng = rand::thread_rng();
    let data: Vec<f64> = (0..numel).map(|_| rng.gen_range(-std..std)).collect();
    Tensor::from_vec_f64(&data, Shape::from_1d(numel))
}

pub fn xavier_normal(numel: usize, dtype: DType, fan_in: usize, fan_out: usize) -> Tensor {
    let std = (2.0 / (fan_in + fan_out) as f64).sqrt();
    let mut rng = rand::thread_rng();
    let data: Vec<f64> = (0..numel).map(|_| {
        let u1: f64 = rng.gen();
        let u2: f64 = rng.gen();
        let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        z * std
    }).collect();
    Tensor::from_vec_f64(&data, Shape::from_1d(numel))
}

pub fn kaiming_uniform(numel: usize, dtype: DType, fan_in: usize, mode: &str) -> Tensor {
    let std = match mode {
        "fan_out" => (2.0 / fan_in as f64).sqrt(),
        _ => (2.0 / fan_in as f64).sqrt(),
    };
    let bound = 3.0_f64.sqrt() * std;
    let mut rng = rand::thread_rng();
    let data: Vec<f64> = (0..numel).map(|_| rng.gen_range(-bound..bound)).collect();
    Tensor::from_vec_f64(&data, Shape::from_1d(numel))
}

pub fn kaiming_normal(numel: usize, dtype: DType, fan_in: usize, mode: &str) -> Tensor {
    let std = match mode {
        "fan_out" => (2.0 / fan_in as f64).sqrt(),
        _ => (2.0 / fan_in as f64).sqrt(),
    };
    let mut rng = rand::thread_rng();
    let data: Vec<f64> = (0..numel).map(|_| {
        let u1: f64 = rng.gen();
        let u2: f64 = rng.gen();
        let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        z * std
    }).collect();
    Tensor::from_vec_f64(&data, Shape::from_1d(numel))
}

pub fn orthogonal(numel: usize, gain: f64, rows: usize, cols: usize) -> Tensor {
    let mut rng = rand::thread_rng();
    let flat_dim = if rows >= cols { rows } else { cols };
    let mut data: Vec<f64> = (0..flat_dim * cols).map(|_| rng.gen_range(-1.0..1.0)).collect();

    for _ in 0..100 {
        let mut new_data = data.clone();
        for i in 0..flat_dim {
            for j in 0..i.min(cols) {
                let dot: f64 = (0..cols).map(|k| new_data[i * cols + k] * new_data[j * cols + k]).sum();
                for k in 0..cols {
                    new_data[i * cols + k] -= dot * new_data[j * cols + k];
                }
            }
            let norm: f64 = (0..cols).map(|k| new_data[i * cols + k].powi(2)).sum::<f64>().sqrt();
            if norm > 1e-8 {
                for k in 0..cols {
                    new_data[i * cols + k] /= norm;
                }
            }
        }
        data = new_data;
    }

    let mut result = Vec::with_capacity(numel);
    for i in 0..rows {
        for j in 0..cols {
            let idx = (i % flat_dim) * cols + (j % cols);
            if idx < data.len() {
                result.push(data[idx] * gain);
            } else {
                result.push(0.0);
            }
        }
    }
    Tensor::from_vec_f64(&result, Shape::from_1d(numel))
}

pub fn normal_random(numel: usize, mean: f64, std: f64) -> Tensor {
    let mut rng = rand::thread_rng();
    let data: Vec<f64> = (0..numel).map(|_| {
        let u1: f64 = rng.gen();
        let u2: f64 = rng.gen();
        let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        mean + z * std
    }).collect();
    Tensor::from_vec_f64(&data, Shape::from_1d(numel))
}

pub fn uniform_random(numel: usize, low: f64, high: f64) -> Tensor {
    let mut rng = rand::thread_rng();
    let data: Vec<f64> = (0..numel).map(|_| rng.gen_range(low..high)).collect();
    Tensor::from_vec_f64(&data, Shape::from_1d(numel))
}

pub fn sparse_init(numel: usize, sparsity: f64, std: f64) -> Tensor {
    let mut rng = rand::thread_rng();
    let data: Vec<f64> = (0..numel).map(|_| {
        if rng.gen::<f64>() < sparsity {
            0.0
        } else {
            let u1: f64 = rng.gen();
            let u2: f64 = rng.gen();
            let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
            z * std
        }
    }).collect();
    Tensor::from_vec_f64(&data, Shape::from_1d(numel))
}
