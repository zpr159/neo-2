use crate::device::{
    Backend, BinaryOp, Device, ReduceOp, TensorData, UnaryOp,
};
use crate::dtype::{self, DType};
use crate::error::{NeuralError, NeuralResult};

/// CPU compute backend implementing all tensor operations.
#[derive(Debug)]
pub struct CpuBackend {
    device: Device,
}

impl CpuBackend {
    /// Creates a new CPU backend.
    #[must_use]
    pub fn new() -> Self {
        Self {
            device: Device::cpu(),
        }
    }
}

impl Default for CpuBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for CpuBackend {
    fn device(&self) -> &Device {
        &self.device
    }

    fn name(&self) -> &str {
        "cpu"
    }

    fn alloc(&self, size: usize) -> NeuralResult<usize> {
        Ok(size)
    }

    fn free(&self, _offset: usize) {}

    fn copy_host_to_device(
        &self,
        host_src: &[u8],
        _device_dst: usize,
        size: usize,
    ) -> NeuralResult<()> {
        if host_src.len() < size {
            return Err(NeuralError::KernelError {
                message: "source buffer too small for copy".to_string(),
            });
        }
        Ok(())
    }

    fn copy_device_to_host(
        &self,
        _device_src: usize,
        host_dst: &mut [u8],
        size: usize,
    ) -> NeuralResult<()> {
        if host_dst.len() < size {
            return Err(NeuralError::KernelError {
                message: "destination buffer too small for copy".to_string(),
            });
        }
        Ok(())
    }

    fn copy_device_to_device(
        &self,
        _src: usize,
        _dst: usize,
        _size: usize,
    ) -> NeuralResult<()> {
        Ok(())
    }

    fn synchronize(&self) -> NeuralResult<()> {
        Ok(())
    }

    fn device_ptr(&self, offset: usize) -> *const u8 {
        offset as *const u8
    }

    fn device_ptr_mut(&self, offset: usize) -> *mut u8 {
        offset as *mut u8
    }

    fn binary_op(
        &self,
        op: BinaryOp,
        left: &TensorData,
        right: &TensorData,
        output: &mut TensorData,
    ) -> NeuralResult<()> {
        cpu_binary_op(op, left, right, output)
    }

    fn unary_op(
        &self,
        op: UnaryOp,
        input: &TensorData,
        output: &mut TensorData,
    ) -> NeuralResult<()> {
        cpu_unary_op(op, input, output)
    }

    fn matmul(
        &self,
        a: &TensorData,
        b: &TensorData,
        c: &mut TensorData,
        m: usize,
        n: usize,
        k: usize,
    ) -> NeuralResult<()> {
        cpu_matmul(a, b, c, m, n, k)
    }

    fn reduce(
        &self,
        op: ReduceOp,
        input: &TensorData,
        output: &mut TensorData,
        axis: usize,
    ) -> NeuralResult<()> {
        cpu_reduce(op, input, output, axis)
    }

    fn transpose(
        &self,
        input: &TensorData,
        output: &mut TensorData,
        axes: &[usize],
    ) -> NeuralResult<()> {
        cpu_transpose(input, output, axes)
    }

    fn concat(
        &self,
        inputs: &[&TensorData],
        output: &mut TensorData,
        axis: usize,
    ) -> NeuralResult<()> {
        cpu_concat(inputs, output, axis)
    }

    fn slice(
        &self,
        input: &TensorData,
        output: &mut TensorData,
        ranges: &[(usize, usize)],
    ) -> NeuralResult<()> {
        cpu_slice(input, output, ranges)
    }
}

fn read_elem(data: &[u8], dtype: DType, idx: usize) -> f64 {
    let offset = idx * dtype.byte_size();
    dtype::access::read_as_f64(data, offset, dtype)
}

fn write_elem(data: &mut [u8], dtype: DType, idx: usize, val: f64) {
    let offset = idx * dtype.byte_size();
    dtype::access::write_f64_as(data, offset, dtype, val);
}

fn cpu_binary_op(
    op: BinaryOp,
    left: &TensorData,
    right: &TensorData,
    output: &mut TensorData,
) -> NeuralResult<()> {
    let dtype = output.dtype;
    let numel = output.numel();
    let op_name = match op {
        BinaryOp::Add => "add",
        BinaryOp::Sub => "sub",
        BinaryOp::Mul => "mul",
        BinaryOp::Div => "div",
        BinaryOp::Pow => "pow",
        BinaryOp::Modulo => "mod",
        BinaryOp::Maximum => "max",
        BinaryOp::Minimum => "min",
    };

    if dtype == DType::Float32 && left.dtype == DType::Float32 && right.dtype == DType::Float32 {
        let left_f32 = cast_to_f32_slice(&left.bytes);
        let right_f32 = cast_to_f32_slice(&right.bytes);
        let mut out_f32 = vec![0f32; numel];

        for i in 0..numel {
            let l_idx = i % left_f32.len();
            let r_idx = i % right_f32.len();
            out_f32[i] = match op {
                BinaryOp::Add => left_f32[l_idx] + right_f32[r_idx],
                BinaryOp::Sub => left_f32[l_idx] - right_f32[r_idx],
                BinaryOp::Mul => left_f32[l_idx] * right_f32[r_idx],
                BinaryOp::Div => {
                    let divisor = right_f32[r_idx];
                    if divisor == 0.0 {
                        f32::INFINITY
                    } else {
                        left_f32[l_idx] / divisor
                    }
                }
                BinaryOp::Pow => left_f32[l_idx].powf(right_f32[r_idx]),
                BinaryOp::Modulo => {
                    let r = right_f32[r_idx];
                    if r == 0.0 {
                        0.0
                    } else {
                        left_f32[l_idx] % r
                    }
                }
                BinaryOp::Maximum => left_f32[l_idx].max(right_f32[r_idx]),
                BinaryOp::Minimum => left_f32[l_idx].min(right_f32[r_idx]),
            };
        }
        cast_from_f32_slice(&out_f32, &mut output.bytes);
        return Ok(());
    }

    if dtype == DType::Float64 && left.dtype == DType::Float64 && right.dtype == DType::Float64 {
        for i in 0..numel {
            let l_idx = i % left.numel();
            let r_idx = i % right.numel();
            let l = read_elem(&left.bytes, DType::Float64, l_idx);
            let r = read_elem(&right.bytes, DType::Float64, r_idx);
            let result = match op {
                BinaryOp::Add => l + r,
                BinaryOp::Sub => l - r,
                BinaryOp::Mul => l * r,
                BinaryOp::Div => if r == 0.0 { f64::INFINITY } else { l / r },
                BinaryOp::Pow => l.powf(r),
                BinaryOp::Modulo => {
                    if r == 0.0 {
                        0.0
                    } else {
                        l % r
                    }
                }
                BinaryOp::Maximum => l.max(r),
                BinaryOp::Minimum => l.min(r),
            };
            write_elem(&mut output.bytes, DType::Float64, i, result);
        }
        return Ok(());
    }

    for i in 0..numel {
        let l_idx = i % left.numel();
        let r_idx = i % right.numel();
        let l = read_elem(&left.bytes, left.dtype, l_idx);
        let r = read_elem(&right.bytes, right.dtype, r_idx);
        let result = match op {
            BinaryOp::Add => l + r,
            BinaryOp::Sub => l - r,
            BinaryOp::Mul => l * r,
            BinaryOp::Div => if r == 0.0 { f64::INFINITY } else { l / r },
            BinaryOp::Pow => l.powf(r),
            BinaryOp::Modulo => {
                if r == 0.0 {
                    0.0
                } else {
                    l % r
                }
            }
            BinaryOp::Maximum => l.max(r),
            BinaryOp::Minimum => l.min(r),
        };
        write_elem(&mut output.bytes, dtype, i, result);
    }

    if output.numel() == 0 {
        return Err(NeuralError::KernelError {
            message: format!("{}: empty output", op_name),
        });
    }
    Ok(())
}

fn cpu_unary_op(
    op: UnaryOp,
    input: &TensorData,
    output: &mut TensorData,
) -> NeuralResult<()> {
    let dtype = output.dtype;
    let numel = output.numel();

    if dtype == DType::Float32 {
        let in_f32 = cast_to_f32_slice(&input.bytes);
        let mut out_f32 = vec![0f32; numel];
        for i in 0..numel {
            let idx = i % in_f32.len();
            let x = in_f32[idx];
            out_f32[i] = match op {
                UnaryOp::Neg => -x,
                UnaryOp::Abs => x.abs(),
                UnaryOp::Exp => x.exp(),
                UnaryOp::Log => {
                    if x > 0.0 {
                        x.ln()
                    } else {
                        f32::NEG_INFINITY
                    }
                }
                UnaryOp::Sqrt => {
                    if x >= 0.0 {
                        x.sqrt()
                    } else {
                        f32::NAN
                    }
                }
                UnaryOp::Sin => x.sin(),
                UnaryOp::Cos => x.cos(),
                UnaryOp::Tanh => x.tanh(),
                UnaryOp::Relu => x.max(0.0),
                UnaryOp::Gelu => {
                    let c = 0.7978845608f32;
                    let k = 0.044715f32;
                    0.5 * x * (1.0 + (c * x * (1.0 + k * x * x)).tanh())
                }
                UnaryOp::Sigmoid => 1.0 / (1.0 + (-x).exp()),
                UnaryOp::Silu => x / (1.0 + (-x).exp()),
            };
        }
        cast_from_f32_slice(&out_f32, &mut output.bytes);
        return Ok(());
    }

    for i in 0..numel {
        let idx = i % input.numel();
        let x = read_elem(&input.bytes, input.dtype, idx);
        let result = match op {
            UnaryOp::Neg => -x,
            UnaryOp::Abs => x.abs(),
            UnaryOp::Exp => x.exp(),
            UnaryOp::Log => {
                if x > 0.0 {
                    x.ln()
                } else {
                    f64::NEG_INFINITY
                }
            }
            UnaryOp::Sqrt => {
                if x >= 0.0 {
                    x.sqrt()
                } else {
                    f64::NAN
                }
            }
            UnaryOp::Sin => x.sin(),
            UnaryOp::Cos => x.cos(),
            UnaryOp::Tanh => x.tanh(),
            UnaryOp::Relu => x.max(0.0),
            UnaryOp::Gelu => {
                let c = 0.7978845608f64;
                let k = 0.044715f64;
                0.5 * x * (1.0 + (c * x * (1.0 + k * x * x)).tanh())
            }
            UnaryOp::Sigmoid => 1.0 / (1.0 + (-x).exp()),
            UnaryOp::Silu => x / (1.0 + (-x).exp()),
        };
        write_elem(&mut output.bytes, dtype, i, result);
    }

    Ok(())
}

fn cpu_matmul(
    a: &TensorData,
    b: &TensorData,
    c: &mut TensorData,
    m: usize,
    n: usize,
    k: usize,
) -> NeuralResult<()> {
    if a.dtype == DType::Float32 && b.dtype == DType::Float32 {
        let a_f32 = cast_to_f32_slice(&a.bytes);
        let b_f32 = cast_to_f32_slice(&b.bytes);
        let mut c_f32 = vec![0f32; m * n];

        for i in 0..m {
            for j in 0..n {
                let mut sum = 0.0f32;
                for p in 0..k {
                    sum += a_f32[i * k + p] * b_f32[p * n + j];
                }
                c_f32[i * n + j] = sum;
            }
        }
        cast_from_f32_slice(&c_f32, &mut c.bytes);
        return Ok(());
    }

    for i in 0..m {
        for j in 0..n {
            let mut sum = 0.0;
            for p in 0..k {
                let a_val = read_elem(&a.bytes, a.dtype, i * k + p);
                let b_val = read_elem(&b.bytes, b.dtype, p * n + j);
                sum += a_val * b_val;
            }
            write_elem(&mut c.bytes, c.dtype, i * n + j, sum);
        }
    }
    Ok(())
}

fn cpu_reduce(
    op: ReduceOp,
    input: &TensorData,
    output: &mut TensorData,
    axis: usize,
) -> NeuralResult<()> {
    let in_shape = input.shape.dims();
    let out_shape = output.shape.dims();
    let dtype = input.dtype;
    let out_numel = output.numel();

    let axis_size = if axis < in_shape.len() {
        in_shape[axis]
    } else {
        return Err(NeuralError::KernelError {
            message: "reduce axis out of bounds".to_string(),
        });
    };

    for out_idx in 0..out_numel {
        let mut accum = match op {
            ReduceOp::Max => f64::NEG_INFINITY,
            ReduceOp::Min => f64::INFINITY,
            ReduceOp::Sum | ReduceOp::Mean | ReduceOp::Prod => 0.0,
            ReduceOp::Std => 0.0,
        };

        if matches!(op, ReduceOp::Prod) {
            accum = 1.0;
        }

        for k in 0..axis_size {
            let mut in_idx = vec![0usize; in_shape.len()];

            let mut tmp = out_idx;
            for d in (0..out_shape.len()).rev() {
                in_idx[d] = tmp % out_shape[d];
                tmp /= out_shape[d];
            }
            in_idx.insert(axis, k);

            let mut flat_idx = 0;
            let mut stride = 1;
            for d in (0..in_shape.len()).rev() {
                flat_idx += in_idx[d] * stride;
                stride *= in_shape[d];
            }

            let val = read_elem(&input.bytes, dtype, flat_idx);
            accum = match op {
                ReduceOp::Sum | ReduceOp::Mean => accum + val,
                ReduceOp::Max => accum.max(val),
                ReduceOp::Min => accum.min(val),
                ReduceOp::Prod => accum * val,
                ReduceOp::Std => accum + val,
            };
        }

        if matches!(op, ReduceOp::Mean) {
            accum /= axis_size as f64;
        }

        if matches!(op, ReduceOp::Std) {
            let mean = accum / axis_size as f64;
            let mut var_sum = 0.0;
            for k in 0..axis_size {
                let mut in_idx = vec![0usize; in_shape.len()];
                let mut tmp = out_idx;
                for d in (0..out_shape.len()).rev() {
                    in_idx[d] = tmp % out_shape[d];
                    tmp /= out_shape[d];
                }
                in_idx.insert(axis, k);
                let mut flat_idx = 0;
                let mut stride = 1;
                for d in (0..in_shape.len()).rev() {
                    flat_idx += in_idx[d] * stride;
                    stride *= in_shape[d];
                }
                let val = read_elem(&input.bytes, dtype, flat_idx);
                let diff = val - mean;
                var_sum += diff * diff;
            }
            accum = (var_sum / axis_size as f64).sqrt();
        }

        write_elem(&mut output.bytes, output.dtype, out_idx, accum);
    }

    Ok(())
}

fn cpu_transpose(
    input: &TensorData,
    output: &mut TensorData,
    axes: &[usize],
) -> NeuralResult<()> {
    let in_shape = input.shape.dims();
    let out_shape = output.shape.dims();
    let out_numel = output.numel();
    let dtype = input.dtype;

    let in_strides = crate::shape::compute_strides(in_shape);

    for out_idx in 0..out_numel {
        let mut out_coords = vec![0usize; out_shape.len()];
        let mut tmp = out_idx;
        for d in (0..out_shape.len()).rev() {
            out_coords[d] = tmp % out_shape[d];
            tmp /= out_shape[d];
        }

        let mut in_coords = vec![0usize; in_shape.len()];
        for (i, &axis) in axes.iter().enumerate() {
            in_coords[axis] = out_coords[i];
        }

        let mut flat_in_idx = 0;
        for (i, &s) in in_strides.iter().enumerate() {
            flat_in_idx += in_coords[i] * s;
        }

        let val = read_elem(&input.bytes, dtype, flat_in_idx);
        write_elem(&mut output.bytes, dtype, out_idx, val);
    }

    Ok(())
}

fn cpu_concat(
    inputs: &[&TensorData],
    output: &mut TensorData,
    axis: usize,
) -> NeuralResult<()> {
    let dtype = output.dtype;
    let out_shape = output.shape.dims();
    let out_numel = output.numel();

    let mut offset = 0usize;
    for input in inputs {
        let in_shape = input.shape.dims();
        let axis_size = if axis < in_shape.len() {
            in_shape[axis]
        } else {
            return Err(NeuralError::KernelError {
                message: "concat axis out of bounds".to_string(),
            });
        };

        let chunk_numel: usize = in_shape.iter().copied().product();

        for i in 0..chunk_numel {
            let mut in_coords = vec![0usize; in_shape.len()];
            let mut tmp = i;
            for d in (0..in_shape.len()).rev() {
                in_coords[d] = tmp % in_shape[d];
                tmp /= in_shape[d];
            }

            let mut out_coords = in_coords.clone();
            out_coords[axis] += offset;

            let mut out_flat = 0;
            let mut stride = 1;
            for d in (0..out_shape.len()).rev() {
                out_flat += out_coords[d] * stride;
                stride *= out_shape[d];
            }

            let val = read_elem(&input.bytes, dtype, i);
            write_elem(&mut output.bytes, dtype, out_flat, val);
        }

        offset += axis_size;
    }

    let _ = out_numel;
    Ok(())
}

fn cpu_slice(
    input: &TensorData,
    output: &mut TensorData,
    ranges: &[(usize, usize)],
) -> NeuralResult<()> {
    let dtype = input.dtype;
    let in_shape = input.shape.dims();
    let out_shape = output.shape.dims();
    let out_numel = output.numel();
    let in_strides = crate::shape::compute_strides(in_shape);

    for out_idx in 0..out_numel {
        let mut out_coords = vec![0usize; out_shape.len()];
        let mut tmp = out_idx;
        for d in (0..out_shape.len()).rev() {
            out_coords[d] = tmp % out_shape[d];
            tmp /= out_shape[d];
        }

        let mut in_coords = out_coords;
        for (i, &(start, _)) in ranges.iter().enumerate() {
            if i < in_coords.len() {
                in_coords[i] += start;
            }
        }

        let mut flat_in_idx = 0;
        for (i, &s) in in_strides.iter().enumerate() {
            if i < in_coords.len() {
                flat_in_idx += in_coords[i] * s;
            }
        }

        let val = read_elem(&input.bytes, dtype, flat_in_idx);
        write_elem(&mut output.bytes, dtype, out_idx, val);
    }

    Ok(())
}

/// Converts a slice of f32 values to bytes (safe, no transmute).
fn cast_to_f32_slice(bytes: &[u8]) -> Vec<f32> {
    let count = bytes.len() / 4;
    let mut result = vec![0f32; count];
    for i in 0..count {
        let offset = i * 4;
        result[i] = f32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);
    }
    result
}

/// Converts a slice of f32 values to bytes (safe, no transmute).
fn cast_from_f32_slice(f32s: &[f32], bytes: &mut [u8]) {
    for (i, &val) in f32s.iter().enumerate() {
        let b = val.to_le_bytes();
        let offset = i * 4;
        if offset + 4 <= bytes.len() {
            bytes[offset] = b[0];
            bytes[offset + 1] = b[1];
            bytes[offset + 2] = b[2];
            bytes[offset + 3] = b[3];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_backend_creation() {
        let backend = CpuBackend::new();
        assert_eq!(backend.name(), "cpu");
        assert_eq!(backend.device().device_type(), DeviceType::Cpu);
    }

    #[test]
    fn cpu_add_f32() {
        let left = TensorData {
            bytes: vec![1.0f32.to_le_bytes()[0], 1.0f32.to_le_bytes()[1], 1.0f32.to_le_bytes()[2], 1.0f32.to_le_bytes()[3],
                        2.0f32.to_le_bytes()[0], 2.0f32.to_le_bytes()[1], 2.0f32.to_le_bytes()[2], 2.0f32.to_le_bytes()[3]],
            dtype: DType::Float32,
            shape: Shape::from_1d(2),
        };
        let right = TensorData {
            bytes: vec![3.0f32.to_le_bytes()[0], 3.0f32.to_le_bytes()[1], 3.0f32.to_le_bytes()[2], 3.0f32.to_le_bytes()[3],
                        4.0f32.to_le_bytes()[0], 4.0f32.to_le_bytes()[1], 4.0f32.to_le_bytes()[2], 4.0f32.to_le_bytes()[3]],
            dtype: DType::Float32,
            shape: Shape::from_1d(2),
        };
        let mut output = TensorData {
            bytes: vec![0u8; 8],
            dtype: DType::Float32,
            shape: Shape::from_1d(2),
        };

        let backend = CpuBackend::new();
        backend
            .binary_op(BinaryOp::Add, &left, &right, &mut output)
            .unwrap();

        let out_f32 = cast_to_f32_slice(&output.bytes);
        assert!((out_f32[0] - 4.0).abs() < 1e-6);
        assert!((out_f32[1] - 6.0).abs() < 1e-6);
    }

    #[test]
    fn cpu_relu() {
        let input = TensorData {
            bytes: vec![
                (-2.0f32).to_le_bytes()[0], (-2.0f32).to_le_bytes()[1], (-2.0f32).to_le_bytes()[2], (-2.0f32).to_le_bytes()[3],
                3.0f32.to_le_bytes()[0], 3.0f32.to_le_bytes()[1], 3.0f32.to_le_bytes()[2], 3.0f32.to_le_bytes()[3],
            ],
            dtype: DType::Float32,
            shape: Shape::from_1d(2),
        };
        let mut output = TensorData {
            bytes: vec![0u8; 8],
            dtype: DType::Float32,
            shape: Shape::from_1d(2),
        };

        let backend = CpuBackend::new();
        backend
            .unary_op(UnaryOp::Relu, &input, &mut output)
            .unwrap();

        let out_f32 = cast_to_f32_slice(&output.bytes);
        assert!((out_f32[0] - 0.0).abs() < 1e-6);
        assert!((out_f32[1] - 3.0).abs() < 1e-6);
    }

    #[test]
    fn cpu_matmul_2x2() {
        let a = TensorData {
            bytes: f32_slice_to_bytes(&[1.0, 2.0, 3.0, 4.0]),
            dtype: DType::Float32,
            shape: Shape::from_2d(2, 2),
        };
        let b = TensorData {
            bytes: f32_slice_to_bytes(&[5.0, 6.0, 7.0, 8.0]),
            dtype: DType::Float32,
            shape: Shape::from_2d(2, 2),
        };
        let mut c = TensorData {
            bytes: vec![0u8; 16],
            dtype: DType::Float32,
            shape: Shape::from_2d(2, 2),
        };

        let backend = CpuBackend::new();
        backend.matmul(&a, &b, &mut c, 2, 2, 2).unwrap();

        let out = cast_to_f32_slice(&c.bytes);
        assert!((out[0] - 19.0).abs() < 1e-5);
        assert!((out[1] - 22.0).abs() < 1e-5);
        assert!((out[2] - 43.0).abs() < 1e-5);
        assert!((out[3] - 50.0).abs() < 1e-5);
    }

    fn f32_slice_to_bytes(vals: &[f32]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(vals.len() * 4);
        for &v in vals {
            let b = v.to_le_bytes();
            bytes.extend_from_slice(&b);
        }
        bytes
    }
}
