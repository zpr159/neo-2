use std::fmt;
use std::sync::Arc;

use crate::dtype::{self, DType};
use crate::error::{NeuralError, NeuralResult};
use crate::shape::{broadcast_shapes, Shape, Strides};

/// Legacy type alias for backward compatibility.
pub type TensorShape = Vec<usize>;

/// Shared storage for tensor data.
#[derive(Debug, Clone)]
pub struct TensorStorage {
    bytes: Vec<u8>,
}

impl TensorStorage {
    /// Creates new zeroed storage.
    #[must_use]
    pub fn zeros(size: usize) -> Self {
        Self {
            bytes: vec![0u8; size],
        }
    }

    /// Creates storage from existing bytes.
    #[must_use]
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    /// Returns a reference to the raw bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns a mutable reference to the raw bytes.
    pub fn as_mut_bytes(&mut self) -> &mut [u8] {
        &mut self.bytes
    }

    /// Returns the byte length.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Returns true if empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

/// A multi-dimensional tensor supporting views, slicing, and broadcasting.
#[derive(Debug, Clone)]
pub struct Tensor {
    storage: Arc<TensorStorage>,
    dtype: DType,
    shape: Shape,
    strides: Strides,
    offset: usize,
}

impl Tensor {
    /// Creates a new zeroed tensor.
    #[must_use]
    pub fn zeros(shape: Shape, dtype: DType) -> Self {
        let numel = shape.numel();
        let byte_len = numel * dtype.byte_size();
        let storage = TensorStorage::zeros(byte_len);
        let strides = Strides::contiguous(&shape);
        Self {
            storage: Arc::new(storage),
            dtype,
            shape,
            strides,
            offset: 0,
        }
    }

    /// Creates a tensor filled with ones.
    #[must_use]
    pub fn ones(shape: Shape, dtype: DType) -> Self {
        let numel = shape.numel();
        let byte_len = numel * dtype.byte_size();
        let mut bytes = vec![0u8; byte_len];
        let elem_size = dtype.byte_size();
        let one_bytes = dtype.one_bytes();
        for i in 0..numel {
            let start = i * elem_size;
            let end = start + elem_size;
            if end <= bytes.len() && one_bytes.len() >= elem_size {
                bytes[start..end].copy_from_slice(&one_bytes[..elem_size]);
            }
        }
        let strides = Strides::contiguous(&shape);
        Self {
            storage: Arc::new(TensorStorage::from_bytes(bytes)),
            dtype,
            shape,
            strides,
            offset: 0,
        }
    }

    /// Creates a tensor from raw bytes.
    pub fn from_bytes(bytes: Vec<u8>, shape: Shape, dtype: DType) -> NeuralResult<Self> {
        let expected = shape.numel() * dtype.byte_size();
        if bytes.len() != expected {
            return Err(NeuralError::ShapeMismatch {
                expected: vec![expected],
                actual: vec![bytes.len()],
                context: "from_bytes".to_string(),
            });
        }
        let strides = Strides::contiguous(&shape);
        Ok(Self {
            storage: Arc::new(TensorStorage::from_bytes(bytes)),
            dtype,
            shape,
            strides,
            offset: 0,
        })
    }

    /// Creates a tensor filled with a scalar value.
    #[must_use]
    pub fn full(shape: Shape, value: f64, dtype: DType) -> Self {
        let numel = shape.numel();
        let byte_len = numel * dtype.byte_size();
        let mut bytes = vec![0u8; byte_len];
        for i in 0..numel {
            let offset = i * dtype.byte_size();
            dtype::access::write_f64_as(&mut bytes, offset, dtype, value);
        }
        let strides = Strides::contiguous(&shape);
        Self {
            storage: Arc::new(TensorStorage::from_bytes(bytes)),
            dtype,
            shape,
            strides,
            offset: 0,
        }
    }

    /// Creates a tensor from a Vec of f64 values.
    #[must_use]
    pub fn from_vec_f64(data: &[f64], shape: Shape) -> Self {
        let dtype = DType::Float64;
        let byte_len = data.len() * dtype.byte_size();
        let mut bytes = vec![0u8; byte_len];
        for (i, &val) in data.iter().enumerate() {
            let offset = i * dtype.byte_size();
            dtype::access::write_f64_as(&mut bytes, offset, dtype, val);
        }
        let strides = Strides::contiguous(&shape);
        Self {
            storage: Arc::new(TensorStorage::from_bytes(bytes)),
            dtype,
            shape,
            strides,
            offset: 0,
        }
    }

    /// Creates a tensor from a Vec of f32 values.
    #[must_use]
    pub fn from_vec_f32(data: &[f32], shape: Shape) -> Self {
        let dtype = DType::Float32;
        let byte_len = data.len() * dtype.byte_size();
        let mut bytes = vec![0u8; byte_len];
        for (i, &val) in data.iter().enumerate() {
            let b = val.to_le_bytes();
            let start = i * 4;
            bytes[start] = b[0];
            bytes[start + 1] = b[1];
            bytes[start + 2] = b[2];
            bytes[start + 3] = b[3];
        }
        let strides = Strides::contiguous(&shape);
        Self {
            storage: Arc::new(TensorStorage::from_bytes(bytes)),
            dtype,
            shape,
            strides,
            offset: 0,
        }
    }

    /// Creates a tensor from a Vec of i64 values.
    #[must_use]
    pub fn from_vec_i64(data: &[i64], shape: Shape) -> Self {
        let dtype = DType::Int64;
        let byte_len = data.len() * dtype.byte_size();
        let mut bytes = vec![0u8; byte_len];
        for (i, &val) in data.iter().enumerate() {
            let b = val.to_le_bytes();
            let start = i * 8;
            for (j, &byte) in b.iter().enumerate() {
                bytes[start + j] = byte;
            }
        }
        let strides = Strides::contiguous(&shape);
        Self {
            storage: Arc::new(TensorStorage::from_bytes(bytes)),
            dtype,
            shape,
            strides,
            offset: 0,
        }
    }

    /// Creates a tensor from a Vec of bool values.
    #[must_use]
    pub fn from_vec_bool(data: &[bool], shape: Shape) -> Self {
        let dtype = DType::Bool;
        let bytes: Vec<u8> = data.iter().map(|&b| u8::from(b)).collect();
        let strides = Strides::contiguous(&shape);
        Self {
            storage: Arc::new(TensorStorage::from_bytes(bytes)),
            dtype,
            shape,
            strides,
            offset: 0,
        }
    }

    /// Creates a 1D range tensor [start, start+1, ..., start+len-1].
    #[must_use]
    pub fn range(start: f64, len: usize, dtype: DType) -> Self {
        let shape = Shape::from_1d(len);
        let mut bytes = vec![0u8; len * dtype.byte_size()];
        for i in 0..len {
            let offset = i * dtype.byte_size();
            dtype::access::write_f64_as(&mut bytes, offset, dtype, start + i as f64);
        }
        let strides = Strides::contiguous(&shape);
        Self {
            storage: Arc::new(TensorStorage::from_bytes(bytes)),
            dtype,
            shape,
            strides,
            offset: 0,
        }
    }

    /// Returns the shape.
    #[must_use]
    pub fn shape(&self) -> &Shape {
        &self.shape
    }

    /// Returns the data type.
    #[must_use]
    pub fn dtype(&self) -> DType {
        self.dtype
    }

    /// Returns the strides.
    #[must_use]
    pub fn strides(&self) -> &Strides {
        &self.strides
    }

    /// Returns the number of dimensions.
    #[must_use]
    pub fn ndim(&self) -> usize {
        self.shape.ndim()
    }

    /// Returns the total number of elements.
    #[must_use]
    pub fn numel(&self) -> usize {
        self.shape.numel()
    }

    /// Returns the total size in bytes.
    #[must_use]
    pub fn byte_size(&self) -> usize {
        self.numel() * self.dtype.byte_size()
    }

    /// Returns a reference to the raw bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.storage.as_bytes()
    }

    /// Returns the number of bytes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.storage.len()
    }

    /// Returns true if the tensor has no elements.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.numel() == 0
    }

    /// Returns the number of shared references to this tensor's storage.
    #[must_use]
    pub fn reference_count(&self) -> usize {
        Arc::strong_count(&self.storage)
    }

    /// Returns true if this tensor is contiguous in memory.
    #[must_use]
    pub fn is_contiguous(&self) -> bool {
        self.strides == Strides::contiguous(&self.shape)
    }

    /// Returns the flat byte offset of element at given indices.
    #[must_use]
    pub fn byte_offset_at(&self, indices: &[usize]) -> usize {
        self.offset + self.strides.byte_offset(indices)
    }

    /// Returns the scalar value at the given indices as f64.
    pub fn item_f64(&self, indices: &[usize]) -> NeuralResult<f64> {
        let byte_off = self.byte_offset_at(indices);
        Ok(dtype::access::read_as_f64(
            self.storage.as_bytes(),
            byte_off,
            self.dtype,
        ))
    }

    /// Sets the scalar value at the given indices from an f64.
    pub fn set_item_f64(&mut self, indices: &[usize], value: f64) -> NeuralResult<()> {
        let byte_off = self.byte_offset_at(indices);
        // For views, we need to make a copy if we want to mutate
        // This tensor must own its data
        let bytes = Arc::get_mut(&mut self.storage).ok_or_else(|| {
            NeuralError::AutodiffError {
                message: "cannot mutate tensor that is shared (view)".to_string(),
            }
        })?;
        dtype::access::write_f64_as(bytes.as_mut_bytes(), byte_off, self.dtype, value);
        Ok(())
    }

    /// Fills the entire tensor with a scalar value.
    pub fn fill_(mut self, value: f64) -> NeuralResult<Self> {
        let dtype = self.dtype;
        let byte_size = dtype.byte_size();
        let numel = self.numel();
        let ndim = self.ndim();
        let is_contiguous = self.is_contiguous();
        let dims: Vec<usize> = self.shape.dims().to_vec();
        let offset = self.offset;
        let strides: Vec<usize> = self.strides.strides().to_vec();
        let bytes = Arc::get_mut(&mut self.storage).ok_or_else(|| {
            NeuralError::AutodiffError {
                message: "cannot fill_ tensor that is shared".to_string(),
            }
        })?;
        if is_contiguous {
            for i in 0..numel {
                let off = i * byte_size;
                dtype::access::write_f64_as(bytes.as_mut_bytes(), off, dtype, value);
            }
        } else {
            for i in 0..numel {
                let mut coords = vec![0usize; ndim];
                let mut tmp = i;
                for d in (0..ndim).rev() {
                    coords[d] = tmp % dims[d];
                    tmp /= dims[d];
                }
                let mut off = offset * byte_size;
                for d in 0..ndim {
                    off += coords[d] * strides[d] * byte_size;
                }
                dtype::access::write_f64_as(bytes.as_mut_bytes(), off, dtype, value);
            }
        }
        Ok(self)
    }

    /// Reshapes the tensor (must have same total element count).
    pub fn reshape(&self, new_shape: Shape) -> NeuralResult<Tensor> {
        if new_shape.numel() != self.numel() {
            return Err(NeuralError::ShapeMismatch {
                expected: self.shape.to_vec(),
                actual: new_shape.to_vec(),
                context: "reshape".to_string(),
            });
        }
        Ok(Tensor {
            storage: Arc::clone(&self.storage),
            dtype: self.dtype,
            shape: new_shape,
            strides: Strides::contiguous(&self.shape),
            offset: self.offset,
        })
    }

    /// Transposes the tensor according to the given axes permutation.
    pub fn transpose(&self, axes: &[usize]) -> NeuralResult<Tensor> {
        if axes.len() != self.ndim() {
            return Err(NeuralError::ShapeMismatch {
                expected: vec![self.ndim()],
                actual: vec![axes.len()],
                context: "transpose".to_string(),
            });
        }
        let new_dims: Vec<usize> = axes.iter().map(|&i| self.shape.dims()[i]).collect();
        let new_strides_values: Vec<usize> = axes
            .iter()
            .map(|&i| self.strides.strides()[i])
            .collect();

        Ok(Tensor {
            storage: Arc::clone(&self.storage),
            dtype: self.dtype,
            shape: Shape::new(new_dims),
            strides: Strides::new(new_strides_values),
            offset: self.offset,
        })
    }

    /// Returns a 2D transpose (swaps last two dimensions).
    pub fn t(&self) -> NeuralResult<Tensor> {
        if self.ndim() < 2 {
            return Err(NeuralError::ShapeMismatch {
                expected: vec![2],
                actual: vec![self.ndim()],
                context: "transpose 2D".to_string(),
            });
        }
        let mut axes: Vec<usize> = (0..self.ndim()).collect();
        let last = axes.len() - 1;
        axes.swap(last - 1, last);
        self.transpose(&axes)
    }

    /// Slices the tensor along a dimension, returning a view.
    pub fn slice(&self, dim: usize, start: usize, end: usize) -> NeuralResult<Tensor> {
        let dim_size = self.shape.dim(dim)?;
        if end > dim_size || start >= end {
            return Err(NeuralError::OutOfBounds {
                index: start,
                bound: dim_size,
                context: format!("slice on dim {} with range [{}..{})", dim, start, end),
            });
        }

        let mut new_dims = self.shape.to_vec();
        new_dims[dim] = end - start;

        let byte_start = start * self.strides.stride(dim) * self.dtype.byte_size();

        Ok(Tensor {
            storage: Arc::clone(&self.storage),
            dtype: self.dtype,
            shape: Shape::new(new_dims),
            strides: self.strides.clone(),
            offset: self.offset + byte_start,
        })
    }

    /// Slices along multiple dimensions.
    pub fn slice_dims(&self, ranges: &[(usize, usize)]) -> NeuralResult<Tensor> {
        let mut result = self.clone();
        for (dim, &(start, end)) in ranges.iter().enumerate() {
            result = result.slice(dim, start, end)?;
        }
        Ok(result)
    }

    /// Returns a sub-tensor at the given flat index (for 1D tensors).
    pub fn index(&self, idx: usize) -> NeuralResult<Tensor> {
        self.slice(0, idx, idx + 1)
    }

    /// Returns the scalar value for a 0D or single-element tensor.
    pub fn item(&self) -> NeuralResult<f64> {
        if self.numel() != 1 {
            return Err(NeuralError::ShapeMismatch {
                expected: vec![1],
                actual: self.shape.to_vec(),
                context: "item()".to_string(),
            });
        }
        self.item_f64(&[])
    }

    /// Unsqueeze: adds a new dimension of size 1 at the given axis.
    pub fn unsqueeze(&self, axis: usize) -> NeuralResult<Tensor> {
        let new_shape = self.shape.insert_dim(axis, 1)?;
        Ok(Tensor {
            storage: Arc::clone(&self.storage),
            dtype: self.dtype,
            shape: new_shape,
            strides: self.strides.clone(),
            offset: self.offset,
        })
    }

    /// Squeeze: removes dimensions of size 1.
    pub fn squeeze(&self) -> NeuralResult<Tensor> {
        let new_dims: Vec<usize> = self
            .shape
            .dims()
            .iter()
            .copied()
            .filter(|&d| d != 1)
            .collect();
        let new_shape = if new_dims.is_empty() {
            Shape::scalar()
        } else {
            Shape::new(new_dims)
        };
        self.reshape(new_shape)
    }

    /// Squeeze a specific axis.
    pub fn squeeze_axis(&self, axis: usize) -> NeuralResult<Tensor> {
        if self.shape.dim(axis)? != 1 {
            return Err(NeuralError::ShapeMismatch {
                expected: vec![1],
                actual: vec![self.shape.dim(axis)?],
                context: format!("squeeze axis {}", axis),
            });
        }
        let new_shape = self.shape.remove_dim(axis)?;
        Ok(Tensor {
            storage: Arc::clone(&self.storage),
            dtype: self.dtype,
            shape: new_shape,
            strides: self.strides.clone(),
            offset: self.offset,
        })
    }

    /// Flattens the tensor to 1D.
    pub fn flatten(&self) -> NeuralResult<Tensor> {
        self.reshape(Shape::from_1d(self.numel()))
    }

    /// Contiguous: creates a contiguous copy if the tensor is not contiguous.
    pub fn contiguous(&self) -> NeuralResult<Tensor> {
        if self.is_contiguous() && self.offset == 0 {
            return Ok(self.clone());
        }
        let numel = self.numel();
        let mut new_bytes = vec![0u8; numel * self.dtype.byte_size()];
        for i in 0..numel {
            let mut coords = vec![0usize; self.ndim()];
            let mut tmp = i;
            for d in (0..self.ndim()).rev() {
                coords[d] = tmp % self.shape.dims()[d];
                tmp /= self.shape.dims()[d];
            }
            let src_off = self.byte_offset_at(&coords);
            let dst_off = i * self.dtype.byte_size();
            let elem_size = self.dtype.byte_size();
            if src_off + elem_size <= self.storage.as_bytes().len()
                && dst_off + elem_size <= new_bytes.len()
            {
                new_bytes[dst_off..dst_off + elem_size]
                    .copy_from_slice(&self.storage.as_bytes()[src_off..src_off + elem_size]);
            }
        }
        Ok(Tensor {
            storage: Arc::new(TensorStorage::from_bytes(new_bytes)),
            dtype: self.dtype,
            shape: self.shape.clone(),
            strides: Strides::contiguous(&self.shape),
            offset: 0,
        })
    }

    /// Clones the data (deep copy).
    #[must_use]
    pub fn clone_data(&self) -> Self {
        let bytes = self.storage.as_bytes().to_vec();
        Self {
            storage: Arc::new(TensorStorage::from_bytes(bytes)),
            dtype: self.dtype,
            shape: self.shape.clone(),
            strides: self.strides.clone(),
            offset: 0,
        }
    }

    /// Adds two tensors with broadcasting.
    pub fn add(&self, other: &Tensor) -> NeuralResult<Tensor> {
        let out_shape_vec =
            broadcast_shapes(self.shape.dims(), other.shape.dims())?;
        let out_shape = Shape::new(out_shape_vec);
        let mut result = Tensor::zeros(out_shape, self.dtype);
        let out_numel = result.numel();

        for i in 0..out_numel {
            let mut coords = vec![0usize; result.ndim()];
            let mut tmp = i;
            for d in (0..result.ndim()).rev() {
                coords[d] = tmp % result.shape.dims()[d];
                tmp /= result.shape.dims()[d];
            }

            let l_coords = map_broadcast_coords(&coords, self.shape.dims());
            let r_coords = map_broadcast_coords(&coords, other.shape.dims());

            let l_val = self.item_f64(&l_coords)?;
            let r_val = other.item_f64(&r_coords)?;
            result.set_item_f64(&coords, l_val + r_val)?;
        }
        Ok(result)
    }

    /// Subtracts two tensors with broadcasting.
    pub fn sub(&self, other: &Tensor) -> NeuralResult<Tensor> {
        let out_shape_vec =
            broadcast_shapes(self.shape.dims(), other.shape.dims())?;
        let out_shape = Shape::new(out_shape_vec);
        let mut result = Tensor::zeros(out_shape, self.dtype);
        let out_numel = result.numel();

        for i in 0..out_numel {
            let mut coords = vec![0usize; result.ndim()];
            let mut tmp = i;
            for d in (0..result.ndim()).rev() {
                coords[d] = tmp % result.shape.dims()[d];
                tmp /= result.shape.dims()[d];
            }
            let l_coords = map_broadcast_coords(&coords, self.shape.dims());
            let r_coords = map_broadcast_coords(&coords, other.shape.dims());
            let l_val = self.item_f64(&l_coords)?;
            let r_val = other.item_f64(&r_coords)?;
            result.set_item_f64(&coords, l_val - r_val)?;
        }
        Ok(result)
    }

    /// Multiplies two tensors with broadcasting.
    pub fn mul(&self, other: &Tensor) -> NeuralResult<Tensor> {
        let out_shape_vec =
            broadcast_shapes(self.shape.dims(), other.shape.dims())?;
        let out_shape = Shape::new(out_shape_vec);
        let mut result = Tensor::zeros(out_shape, self.dtype);
        let out_numel = result.numel();

        for i in 0..out_numel {
            let mut coords = vec![0usize; result.ndim()];
            let mut tmp = i;
            for d in (0..result.ndim()).rev() {
                coords[d] = tmp % result.shape.dims()[d];
                tmp /= result.shape.dims()[d];
            }
            let l_coords = map_broadcast_coords(&coords, self.shape.dims());
            let r_coords = map_broadcast_coords(&coords, other.shape.dims());
            let l_val = self.item_f64(&l_coords)?;
            let r_val = other.item_f64(&r_coords)?;
            result.set_item_f64(&coords, l_val * r_val)?;
        }
        Ok(result)
    }

    /// Divides two tensors with broadcasting.
    pub fn div(&self, other: &Tensor) -> NeuralResult<Tensor> {
        let out_shape_vec =
            broadcast_shapes(self.shape.dims(), other.shape.dims())?;
        let out_shape = Shape::new(out_shape_vec);
        let mut result = Tensor::zeros(out_shape, self.dtype);
        let out_numel = result.numel();

        for i in 0..out_numel {
            let mut coords = vec![0usize; result.ndim()];
            let mut tmp = i;
            for d in (0..result.ndim()).rev() {
                coords[d] = tmp % result.shape.dims()[d];
                tmp /= result.shape.dims()[d];
            }
            let l_coords = map_broadcast_coords(&coords, self.shape.dims());
            let r_coords = map_broadcast_coords(&coords, other.shape.dims());
            let l_val = self.item_f64(&l_coords)?;
            let r_val = other.item_f64(&r_coords)?;
            let result_val = if r_val == 0.0 {
                if l_val == 0.0 {
                    0.0
                } else if l_val > 0.0 {
                    f64::INFINITY
                } else {
                    f64::NEG_INFINITY
                }
            } else {
                l_val / r_val
            };
            result.set_item_f64(&coords, result_val)?;
        }
        Ok(result)
    }

    /// Negates the tensor.
    pub fn neg(&self) -> NeuralResult<Tensor> {
        let mut result = Tensor::zeros(self.shape.clone(), self.dtype);
        for i in 0..self.numel() {
            let mut coords = vec![0usize; self.ndim()];
            let mut tmp = i;
            for d in (0..self.ndim()).rev() {
                coords[d] = tmp % self.shape.dims()[d];
                tmp /= self.shape.dims()[d];
            }
            let val = self.item_f64(&coords)?;
            result.set_item_f64(&coords, -val)?;
        }
        Ok(result)
    }

    /// Element-wise ReLU.
    pub fn relu(&self) -> NeuralResult<Tensor> {
        let mut result = Tensor::zeros(self.shape.clone(), self.dtype);
        for i in 0..self.numel() {
            let mut coords = vec![0usize; self.ndim()];
            let mut tmp = i;
            for d in (0..self.ndim()).rev() {
                coords[d] = tmp % self.shape.dims()[d];
                tmp /= self.shape.dims()[d];
            }
            let val = self.item_f64(&coords)?;
            result.set_item_f64(&coords, val.max(0.0))?;
        }
        Ok(result)
    }

    /// Element-wise GELU.
    pub fn gelu(&self) -> NeuralResult<Tensor> {
        let c = 0.7978845608f64;
        let k = 0.044715f64;
        let mut result = Tensor::zeros(self.shape.clone(), self.dtype);
        for i in 0..self.numel() {
            let mut coords = vec![0usize; self.ndim()];
            let mut tmp = i;
            for d in (0..self.ndim()).rev() {
                coords[d] = tmp % self.shape.dims()[d];
                tmp /= self.shape.dims()[d];
            }
            let x = self.item_f64(&coords)?;
            let val = 0.5 * x * (1.0 + (c * x * (1.0 + k * x * x)).tanh());
            result.set_item_f64(&coords, val)?;
        }
        Ok(result)
    }

    /// Element-wise Sigmoid.
    pub fn sigmoid(&self) -> NeuralResult<Tensor> {
        let mut result = Tensor::zeros(self.shape.clone(), self.dtype);
        for i in 0..self.numel() {
            let mut coords = vec![0usize; self.ndim()];
            let mut tmp = i;
            for d in (0..self.ndim()).rev() {
                coords[d] = tmp % self.shape.dims()[d];
                tmp /= self.shape.dims()[d];
            }
            let x = self.item_f64(&coords)?;
            let val = 1.0 / (1.0 + (-x).exp());
            result.set_item_f64(&coords, val)?;
        }
        Ok(result)
    }

    /// Element-wise Tanh.
    pub fn tanh(&self) -> NeuralResult<Tensor> {
        let mut result = Tensor::zeros(self.shape.clone(), self.dtype);
        for i in 0..self.numel() {
            let mut coords = vec![0usize; self.ndim()];
            let mut tmp = i;
            for d in (0..self.ndim()).rev() {
                coords[d] = tmp % self.shape.dims()[d];
                tmp /= self.shape.dims()[d];
            }
            let x = self.item_f64(&coords)?;
            result.set_item_f64(&coords, x.tanh())?;
        }
        Ok(result)
    }

    /// Matrix multiplication.
    pub fn matmul(&self, other: &Tensor) -> NeuralResult<Tensor> {
        if self.ndim() != 2 || other.ndim() != 2 {
            return Err(NeuralError::ShapeMismatch {
                expected: vec![2],
                actual: vec![self.ndim(), other.ndim()],
                context: "matmul requires 2D tensors".to_string(),
            });
        }
        let m = self.shape.dim(0)?;
        let k1 = self.shape.dim(1)?;
        let k2 = other.shape.dim(0)?;
        let n = other.shape.dim(1)?;

        if k1 != k2 {
            return Err(NeuralError::ShapeMismatch {
                expected: vec![m, k1],
                actual: vec![k2, n],
                context: "matmul inner dimensions must match".to_string(),
            });
        }

        let mut result = Tensor::zeros(Shape::from_2d(m, n), self.dtype);
        for i in 0..m {
            for j in 0..n {
                let mut sum = 0.0;
                for p in 0..k1 {
                    let a_val = self.item_f64(&[i, p])?;
                    let b_val = other.item_f64(&[p, j])?;
                    sum += a_val * b_val;
                }
                result.set_item_f64(&[i, j], sum)?;
            }
        }
        Ok(result)
    }

    /// Sum along an axis.
    pub fn sum_axis(&self, axis: usize) -> NeuralResult<Tensor> {
        let mut out_dims = self.shape.to_vec();
        if axis >= out_dims.len() {
            return Err(NeuralError::OutOfBounds {
                index: axis,
                bound: out_dims.len(),
                context: "sum_axis".to_string(),
            });
        }
        let axis_size = out_dims[axis];
        out_dims[axis] = 1;
        let mut result = Tensor::zeros(Shape::new(out_dims), self.dtype);

        for i in 0..result.numel() {
            let mut coords = vec![0usize; result.ndim()];
            let mut tmp = i;
            for d in (0..result.ndim()).rev() {
                coords[d] = tmp % result.shape.dims()[d];
                tmp /= result.shape.dims()[d];
            }
            let mut sum = 0.0;
            for k in 0..axis_size {
                let mut full_coords = coords.clone();
                full_coords[axis] = k;
                sum += self.item_f64(&full_coords)?;
            }
            result.set_item_f64(&coords, sum)?;
        }
        result = result.squeeze_axis(axis)?;
        Ok(result)
    }

    /// Mean along an axis.
    pub fn mean_axis(&self, axis: usize) -> NeuralResult<Tensor> {
        let axis_size = self.shape.dim(axis)? as f64;
        let mut result = self.sum_axis(axis)?;
        let dtype = result.dtype;
        let result_numel = result.numel();
        let bytes = Arc::get_mut(&mut result.storage).ok_or_else(|| {
            NeuralError::AutodiffError {
                message: "cannot divide shared tensor".to_string(),
            }
        })?;
        let byte_size = dtype.byte_size();
        for i in 0..result_numel {
            let off = i * byte_size;
            let val = dtype::access::read_as_f64(bytes.as_bytes(), off, dtype);
            dtype::access::write_f64_as(bytes.as_mut_bytes(), off, dtype, val / axis_size);
        }
        Ok(result)
    }

    /// Max along an axis.
    pub fn max_axis(&self, axis: usize) -> NeuralResult<Tensor> {
        let mut out_dims = self.shape.to_vec();
        if axis >= out_dims.len() {
            return Err(NeuralError::OutOfBounds {
                index: axis,
                bound: out_dims.len(),
                context: "max_axis".to_string(),
            });
        }
        let axis_size = out_dims[axis];
        out_dims.remove(axis);
        let mut result = Tensor::zeros(Shape::new(out_dims), self.dtype);

        for i in 0..result.numel() {
            let mut coords = vec![0usize; result.ndim()];
            let mut tmp = i;
            for d in (0..result.ndim()).rev() {
                coords[d] = tmp % result.shape.dims()[d];
                tmp /= result.shape.dims()[d];
            }
            let mut max_val = f64::NEG_INFINITY;
            for k in 0..axis_size {
                let mut full_coords = coords.clone();
                full_coords.insert(axis, k);
                let val = self.item_f64(&full_coords)?;
                max_val = max_val.max(val);
            }
            result.set_item_f64(&coords, max_val)?;
        }
        Ok(result)
    }

    /// Min along an axis.
    pub fn min_axis(&self, axis: usize) -> NeuralResult<Tensor> {
        let mut out_dims = self.shape.to_vec();
        if axis >= out_dims.len() {
            return Err(NeuralError::OutOfBounds {
                index: axis,
                bound: out_dims.len(),
                context: "min_axis".to_string(),
            });
        }
        let axis_size = out_dims[axis];
        out_dims.remove(axis);
        let mut result = Tensor::zeros(Shape::new(out_dims), self.dtype);

        for i in 0..result.numel() {
            let mut coords = vec![0usize; result.ndim()];
            let mut tmp = i;
            for d in (0..result.ndim()).rev() {
                coords[d] = tmp % result.shape.dims()[d];
                tmp /= result.shape.dims()[d];
            }
            let mut min_val = f64::INFINITY;
            for k in 0..axis_size {
                let mut full_coords = coords.clone();
                full_coords.insert(axis, k);
                let val = self.item_f64(&full_coords)?;
                min_val = min_val.min(val);
            }
            result.set_item_f64(&coords, min_val)?;
        }
        Ok(result)
    }

    /// Softmax along the last axis.
    pub fn softmax(&self, axis: usize) -> NeuralResult<Tensor> {
        let axis_size = self.shape.dim(axis)?;
        let mut result = self.clone_data();
        let dtype = self.dtype;

        // For each slice along the given axis, compute softmax
        let outer: usize = self.shape.dims()[..axis].iter().copied().product();
        let inner: usize = self
            .shape
            .dims()
            .get(axis + 1..)
            .map_or(1, |s| s.iter().copied().product());

        for _o in 0..outer {
            for _i in 0..inner {
                let mut max_val = f64::NEG_INFINITY;
                let mut indices = Vec::new();
                for a in 0..axis_size {
                    let mut coords = vec![0usize; self.ndim()];
                    // compute coords for this (o, a, i) combination
                    let mut o_remaining = _o;
                    for d in 0..axis {
                        let dim_size = self.shape.dims()[d];
                        coords[d] = o_remaining % dim_size;
                        o_remaining /= dim_size;
                    }
                    coords[axis] = a;
                    let mut i_remaining = _i;
                    for d in (axis + 1)..self.ndim() {
                        let dim_size = self.shape.dims()[d];
                        coords[d] = i_remaining % dim_size;
                        i_remaining /= dim_size;
                    }
                    let val = self.item_f64(&coords)?;
                    max_val = max_val.max(val);
                    indices.push(coords);
                }
                let mut sum_exp = 0.0;
                for coords in &indices {
                    let val = self.item_f64(coords)?;
                    sum_exp += (val - max_val).exp();
                }
                for coords in &indices {
                    let val = self.item_f64(coords)?;
                    let softmax_val = (val - max_val).exp() / sum_exp;
                    result.set_item_f64(coords, softmax_val)?;
                }
            }
        }
        Ok(result)
    }

    /// Returns the data as a Vec<f64> (for small tensors).
    pub fn to_vec_f64(&self) -> NeuralResult<Vec<f64>> {
        let mut result = Vec::with_capacity(self.numel());
        for i in 0..self.numel() {
            let mut coords = vec![0usize; self.ndim()];
            let mut tmp = i;
            for d in (0..self.ndim()).rev() {
                coords[d] = tmp % self.shape.dims()[d];
                tmp /= self.shape.dims()[d];
            }
            result.push(self.item_f64(&coords)?);
        }
        Ok(result)
    }

    /// Casts the tensor to a different dtype.
    pub fn to_dtype(&self, new_dtype: DType) -> NeuralResult<Tensor> {
        let values = self.to_vec_f64()?;
        let byte_len = self.numel() * new_dtype.byte_size();
        let mut bytes = vec![0u8; byte_len];
        for (i, &val) in values.iter().enumerate() {
            let off = i * new_dtype.byte_size();
            dtype::access::write_f64_as(&mut bytes, off, new_dtype, val);
        }
        Ok(Tensor {
            storage: Arc::new(TensorStorage::from_bytes(bytes)),
            dtype: new_dtype,
            shape: self.shape.clone(),
            strides: Strides::contiguous(&self.shape),
            offset: 0,
        })
    }

    /// Creates a detached copy (same data, no gradient tracking).
    #[must_use]
    pub fn detach(&self) -> Tensor {
        self.clone_data()
    }
}

impl fmt::Display for Tensor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Tensor(shape={}, dtype={}, numel={}, bytes={})",
            self.shape,
            self.dtype,
            self.numel(),
            self.byte_size()
        )
    }
}

/// Maps broadcast coordinates from output to input coordinates.
fn map_broadcast_coords(out_coords: &[usize], in_dims: &[usize]) -> Vec<usize> {
    let ndim_out = out_coords.len();
    let ndim_in = in_dims.len();
    let mut in_coords = vec![0usize; ndim_in];
    for d in 0..ndim_in {
        let out_idx = d + (ndim_out - ndim_in);
        if out_idx < ndim_out {
            if in_dims[d] == 1 {
                in_coords[d] = 0;
            } else {
                in_coords[d] = out_coords[out_idx];
            }
        }
    }
    in_coords
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tensor_zeros() {
        let t = Tensor::zeros(Shape::from_2d(2, 3), DType::Float32);
        assert_eq!(t.shape().dims(), &[2, 3]);
        assert_eq!(t.numel(), 6);
        assert_eq!(t.dtype(), DType::Float32);
    }

    #[test]
    fn tensor_from_vec() {
        let t = Tensor::from_vec_f32(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], Shape::from_2d(2, 3));
        assert_eq!(t.numel(), 6);
        assert_eq!(t.item_f64(&[0, 0]).unwrap(), 1.0);
        assert_eq!(t.item_f64(&[1, 2]).unwrap(), 6.0);
    }

    #[test]
    fn tensor_ones() {
        let t = Tensor::ones(Shape::from_1d(4), DType::Float32);
        for i in 0..4 {
            assert_eq!(t.item_f64(&[i]).unwrap(), 1.0);
        }
    }

    #[test]
    fn tensor_reshape() {
        let t = Tensor::from_vec_f32(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], Shape::from_2d(2, 3));
        let r = t.reshape(Shape::from_2d(3, 2)).unwrap();
        assert_eq!(r.shape().dims(), &[3, 2]);
        assert_eq!(r.item_f64(&[0, 0]).unwrap(), 1.0);
        assert_eq!(r.item_f64(&[0, 1]).unwrap(), 2.0);
    }

    #[test]
    fn tensor_reshape_error() {
        let t = Tensor::zeros(Shape::from_2d(2, 3), DType::Float32);
        let r = t.reshape(Shape::from_2d(2, 4));
        assert!(r.is_err());
    }

    #[test]
    fn tensor_add() {
        let a = Tensor::from_vec_f32(&[1.0, 2.0, 3.0], Shape::from_1d(3));
        let b = Tensor::from_vec_f32(&[4.0, 5.0, 6.0], Shape::from_1d(3));
        let c = a.add(&b).unwrap();
        assert_eq!(c.item_f64(&[0]).unwrap(), 5.0);
        assert_eq!(c.item_f64(&[1]).unwrap(), 7.0);
        assert_eq!(c.item_f64(&[2]).unwrap(), 9.0);
    }

    #[test]
    fn tensor_mul() {
        let a = Tensor::from_vec_f32(&[2.0, 3.0], Shape::from_1d(2));
        let b = Tensor::from_vec_f32(&[4.0, 5.0], Shape::from_1d(2));
        let c = a.mul(&b).unwrap();
        assert_eq!(c.item_f64(&[0]).unwrap(), 8.0);
        assert_eq!(c.item_f64(&[1]).unwrap(), 15.0);
    }

    #[test]
    fn tensor_matmul() {
        let a = Tensor::from_vec_f32(&[1.0, 2.0, 3.0, 4.0], Shape::from_2d(2, 2));
        let b = Tensor::from_vec_f32(&[5.0, 6.0, 7.0, 8.0], Shape::from_2d(2, 2));
        let c = a.matmul(&b).unwrap();
        assert_eq!(c.item_f64(&[0, 0]).unwrap(), 19.0);
        assert_eq!(c.item_f64(&[0, 1]).unwrap(), 22.0);
        assert_eq!(c.item_f64(&[1, 0]).unwrap(), 43.0);
        assert_eq!(c.item_f64(&[1, 1]).unwrap(), 50.0);
    }

    #[test]
    fn tensor_transpose() {
        let t = Tensor::from_vec_f32(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], Shape::from_2d(2, 3));
        let tt = t.t().unwrap();
        assert_eq!(tt.shape().dims(), &[3, 2]);
        assert_eq!(tt.item_f64(&[0, 0]).unwrap(), 1.0);
        assert_eq!(tt.item_f64(&[0, 1]).unwrap(), 4.0);
        assert_eq!(tt.item_f64(&[2, 0]).unwrap(), 3.0);
        assert_eq!(tt.item_f64(&[2, 1]).unwrap(), 6.0);
    }

    #[test]
    fn tensor_slice() {
        let t = Tensor::from_vec_f32(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], Shape::from_2d(2, 3));
        let s = t.slice(1, 1, 3).unwrap();
        assert_eq!(s.shape().dims(), &[2, 2]);
        assert_eq!(s.item_f64(&[0, 0]).unwrap(), 2.0);
        assert_eq!(s.item_f64(&[0, 1]).unwrap(), 3.0);
        assert_eq!(s.item_f64(&[1, 0]).unwrap(), 5.0);
        assert_eq!(s.item_f64(&[1, 1]).unwrap(), 6.0);
    }

    #[test]
    fn tensor_relu() {
        let t = Tensor::from_vec_f32(&[-2.0, -1.0, 0.0, 1.0, 2.0], Shape::from_1d(5));
        let r = t.relu().unwrap();
        assert_eq!(r.item_f64(&[0]).unwrap(), 0.0);
        assert_eq!(r.item_f64(&[1]).unwrap(), 0.0);
        assert_eq!(r.item_f64(&[2]).unwrap(), 0.0);
        assert_eq!(r.item_f64(&[3]).unwrap(), 1.0);
        assert_eq!(r.item_f64(&[4]).unwrap(), 2.0);
    }

    #[test]
    fn tensor_softmax() {
        let t = Tensor::from_vec_f32(&[1.0, 2.0, 3.0], Shape::from_1d(3));
        let s = t.softmax(0).unwrap();
        let sum: f64 = (0..3).map(|i| s.item_f64(&[i]).unwrap()).sum();
        assert!((sum - 1.0).abs() < 1e-5);
    }

    #[test]
    fn tensor_broadcast_add() {
        let a = Tensor::from_vec_f32(&[1.0, 2.0, 3.0], Shape::from_2d(1, 3));
        let b = Tensor::from_vec_f32(&[10.0, 20.0, 30.0, 40.0, 50.0, 60.0], Shape::from_2d(2, 3));
        let c = a.add(&b).unwrap();
        assert_eq!(c.shape().dims(), &[2, 3]);
        assert_eq!(c.item_f64(&[0, 0]).unwrap(), 11.0);
        assert_eq!(c.item_f64(&[1, 2]).unwrap(), 63.0);
    }

    #[test]
    fn tensor_sum_axis() {
        let t = Tensor::from_vec_f32(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], Shape::from_2d(2, 3));
        let s = t.sum_axis(1).unwrap();
        assert_eq!(s.shape().dims(), &[2]);
        assert_eq!(s.item_f64(&[0]).unwrap(), 6.0);
        assert_eq!(s.item_f64(&[1]).unwrap(), 15.0);
    }

    #[test]
    fn tensor_mean_axis() {
        let t = Tensor::from_vec_f32(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], Shape::from_2d(2, 3));
        let m = t.mean_axis(1).unwrap();
        assert_eq!(m.shape().dims(), &[2]);
        assert!((m.item_f64(&[0]).unwrap() - 2.0).abs() < 1e-5);
        assert!((m.item_f64(&[1]).unwrap() - 5.0).abs() < 1e-5);
    }

    #[test]
    fn tensor_neg() {
        let t = Tensor::from_vec_f32(&[1.0, -2.0, 3.0], Shape::from_1d(3));
        let n = t.neg().unwrap();
        assert_eq!(n.item_f64(&[0]).unwrap(), -1.0);
        assert_eq!(n.item_f64(&[1]).unwrap(), 2.0);
        assert_eq!(n.item_f64(&[2]).unwrap(), -3.0);
    }

    #[test]
    fn tensor_squeeze_unsqueeze() {
        let t = Tensor::from_vec_f32(&[1.0, 2.0, 3.0], Shape::from_1d(3));
        let u = t.unsqueeze(0).unwrap();
        assert_eq!(u.shape().dims(), &[1, 3]);
        let s = u.squeeze_axis(0).unwrap();
        assert_eq!(s.shape().dims(), &[3]);
    }

    #[test]
    fn tensor_flatten() {
        let t = Tensor::zeros(Shape::from_3d(2, 3, 4), DType::Float32);
        let f = t.flatten().unwrap();
        assert_eq!(f.shape().dims(), &[24]);
    }

    #[test]
    fn tensor_contiguous() {
        let t = Tensor::from_vec_f32(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], Shape::from_2d(2, 3));
        let tt = t.t().unwrap();
        assert!(!tt.is_contiguous());
        let c = tt.contiguous().unwrap();
        assert!(c.is_contiguous());
        assert_eq!(c.item_f64(&[0, 0]).unwrap(), 1.0);
        assert_eq!(c.item_f64(&[1, 0]).unwrap(), 2.0);
    }

    #[test]
    fn tensor_to_dtype() {
        let t = Tensor::from_vec_f32(&[1.0, 2.0, 3.0], Shape::from_1d(3));
        let t64 = t.to_dtype(DType::Float64).unwrap();
        assert_eq!(t64.dtype(), DType::Float64);
        assert!((t64.item_f64(&[0]).unwrap() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn tensor_from_bool() {
        let t = Tensor::from_vec_bool(&[true, false, true], Shape::from_1d(3));
        assert_eq!(t.dtype(), DType::Bool);
        assert_eq!(t.item_f64(&[0]).unwrap(), 1.0);
        assert_eq!(t.item_f64(&[1]).unwrap(), 0.0);
    }

    #[test]
    fn tensor_detach() {
        let t = Tensor::from_vec_f32(&[1.0, 2.0], Shape::from_1d(2));
        let d = t.detach();
        assert_eq!(d.item_f64(&[0]).unwrap(), 1.0);
    }
}
