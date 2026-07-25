use std::fmt;

use serde::{Deserialize, Serialize};

use crate::dtype::DType;
use crate::error::{NeuralError, NeuralResult};
use crate::shape::Shape;
use crate::tensor::Tensor;

/// Format of a sparse tensor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SparseFormat {
    Coo,
    Csr,
}

impl fmt::Display for SparseFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Coo => write!(f, "COO"),
            Self::Csr => write!(f, "CSR"),
        }
    }
}

/// A sparse tensor in COO (Coordinate) format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CooTensor {
    shape: Shape,
    dtype: DType,
    indices: Vec<Vec<usize>>,
    values: Vec<f64>,
    nnz: usize,
}

impl CooTensor {
    /// Creates a new COO tensor from indices and values.
    pub fn new(shape: Shape, indices: Vec<Vec<usize>>, values: Vec<f64>, dtype: DType) -> NeuralResult<Self> {
        let nnz = values.len();
        if indices.len() != shape.ndim() {
            return Err(NeuralError::ShapeMismatch {
                expected: vec![shape.ndim()],
                actual: vec![indices.len()],
                context: "COO indices dimensions".to_string(),
            });
        }
        for (dim_idx, dim_indices) in indices.iter().enumerate() {
            if dim_indices.len() != nnz {
                return Err(NeuralError::ShapeMismatch {
                    expected: vec![nnz],
                    actual: vec![dim_indices.len()],
                    context: format!("COO dim {} indices length", dim_idx),
                });
            }
            for &idx in dim_indices {
                if idx >= shape.dim(dim_idx)? {
                    return Err(NeuralError::OutOfBounds {
                        index: idx,
                        bound: shape.dim(dim_idx)?,
                        context: format!("COO index on dim {}", dim_idx),
                    });
                }
            }
        }
        Ok(Self {
            shape,
            dtype,
            indices,
            values,
            nnz,
        })
    }

    /// Creates a COO tensor from dense data.
    pub fn from_dense(tensor: &Tensor) -> NeuralResult<Self> {
        let dtype = tensor.dtype();
        let shape = tensor.shape().clone();
        let ndim = shape.ndim();
        let mut indices = vec![Vec::new(); ndim];
        let mut values = Vec::new();

        for i in 0..tensor.numel() {
            let mut coords = vec![0usize; ndim];
            let mut tmp = i;
            for d in (0..ndim).rev() {
                coords[d] = tmp % shape.dims()[d];
                tmp /= shape.dims()[d];
            }
            let val = tensor.item_f64(&coords)?;
            if val != 0.0 {
                for (d, &c) in coords.iter().enumerate() {
                    indices[d].push(c);
                }
                values.push(val);
            }
        }

        Self::new(shape, indices, values, dtype)
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

    /// Returns the number of non-zero elements.
    #[must_use]
    pub fn nnz(&self) -> usize {
        self.nnz
    }

    /// Returns the indices arrays.
    #[must_use]
    pub fn indices(&self) -> &[Vec<usize>] {
        &self.indices
    }

    /// Returns the values.
    #[must_use]
    pub fn values(&self) -> &[f64] {
        &self.values
    }

    /// Returns a mutable reference to the values.
    pub fn values_mut(&mut self) -> &mut Vec<f64> {
        &mut self.values
    }

    /// Converts to dense tensor.
    pub fn to_dense(&self) -> NeuralResult<Tensor> {
        let mut tensor = Tensor::zeros(self.shape.clone(), self.dtype);
        for i in 0..self.nnz {
            let coords: Vec<usize> = self.indices.iter().map(|dim_idx| dim_idx[i]).collect();
            tensor.set_item_f64(&coords, self.values[i])?;
        }
        Ok(tensor)
    }

    /// Scales all values by a scalar.
    pub fn scale(&mut self, factor: f64) {
        for val in &mut self.values {
            *val *= factor;
        }
    }

    /// Returns the sparsity ratio (fraction of zeros).
    #[must_use]
    pub fn sparsity(&self) -> f64 {
        let total = self.shape.numel();
        if total == 0 {
            return 0.0;
        }
        1.0 - (self.nnz as f64 / total as f64)
    }

    /// Returns the density ratio (fraction of non-zeros).
    #[must_use]
    pub fn density(&self) -> f64 {
        1.0 - self.sparsity()
    }
}

/// A sparse tensor in CSR (Compressed Sparse Row) format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsrTensor {
    shape: Shape,
    dtype: DType,
    row_ptr: Vec<usize>,
    col_indices: Vec<usize>,
    values: Vec<f64>,
    nnz: usize,
}

impl CsrTensor {
    /// Creates a CSR tensor from raw components.
    pub fn new(
        shape: Shape,
        row_ptr: Vec<usize>,
        col_indices: Vec<usize>,
        values: Vec<f64>,
        dtype: DType,
    ) -> NeuralResult<Self> {
        let nnz = values.len();
        if row_ptr.len() != shape.dim(0)? + 1 {
            return Err(NeuralError::ShapeMismatch {
                expected: vec![shape.dim(0)? + 1],
                actual: vec![row_ptr.len()],
                context: "CSR row_ptr length".to_string(),
            });
        }
        if col_indices.len() != nnz {
            return Err(NeuralError::ShapeMismatch {
                expected: vec![nnz],
                actual: vec![col_indices.len()],
                context: "CSR col_indices length".to_string(),
            });
        }
        Ok(Self {
            shape,
            dtype,
            row_ptr,
            col_indices,
            values,
            nnz,
        })
    }

    /// Converts from COO format.
    pub fn from_coo(coo: &CooTensor) -> NeuralResult<Self> {
        if coo.shape.ndim() != 2 {
            return Err(NeuralError::GraphValidation {
                message: "CSR only supports 2D tensors".to_string(),
            });
        }

        let rows = coo.shape.dim(0)?;
        let cols = coo.shape.dim(1)?;

        let mut row_col_val: Vec<(usize, usize, f64)> = (0..coo.nnz)
            .map(|i| (coo.indices[0][i], coo.indices[1][i], coo.values[i]))
            .collect();
        row_col_val.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

        let mut row_ptr = vec![0usize; rows + 1];
        let mut col_indices = Vec::with_capacity(coo.nnz);
        let mut values = Vec::with_capacity(coo.nnz);

        for (row, col, val) in &row_col_val {
            row_ptr[*row + 1] += 1;
            col_indices.push(*col);
            values.push(*val);
        }

        for i in 1..=rows {
            row_ptr[i] += row_ptr[i - 1];
        }

        let _ = cols;

        Self::new(coo.shape.clone(), row_ptr, col_indices, values, coo.dtype)
    }

    /// Converts from dense tensor.
    pub fn from_dense(tensor: &Tensor) -> NeuralResult<Self> {
        let coo = CooTensor::from_dense(tensor)?;
        Self::from_coo(&coo)
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

    /// Returns the number of non-zero elements.
    #[must_use]
    pub fn nnz(&self) -> usize {
        self.nnz
    }

    /// Returns the row pointer array.
    #[must_use]
    pub fn row_ptr(&self) -> &[usize] {
        &self.row_ptr
    }

    /// Returns the column indices.
    #[must_use]
    pub fn col_indices(&self) -> &[usize] {
        &self.col_indices
    }

    /// Returns the values.
    #[must_use]
    pub fn values(&self) -> &[f64] {
        &self.values
    }

    /// Converts to dense tensor.
    pub fn to_dense(&self) -> NeuralResult<Tensor> {
        let mut tensor = Tensor::zeros(self.shape.clone(), self.dtype);
        let rows = self.shape.dim(0)?;

        for row in 0..rows {
            let start = self.row_ptr[row];
            let end = self.row_ptr[row + 1];
            for idx in start..end {
                let col = self.col_indices[idx];
                let val = self.values[idx];
                tensor.set_item_f64(&[row, col], val)?;
            }
        }
        Ok(tensor)
    }

    /// Returns the sparsity ratio.
    #[must_use]
    pub fn sparsity(&self) -> f64 {
        let total = self.shape.numel();
        if total == 0 {
            return 0.0;
        }
        1.0 - (self.nnz as f64 / total as f64)
    }
}

/// A sparse tensor that can be in any format.
#[derive(Debug, Clone)]
pub enum SparseTensor {
    Coo(CooTensor),
    Csr(CsrTensor),
}

impl SparseTensor {
    /// Returns the format.
    #[must_use]
    pub fn format(&self) -> SparseFormat {
        match self {
            Self::Coo(_) => SparseFormat::Coo,
            Self::Csr(_) => SparseFormat::Csr,
        }
    }

    /// Returns the shape.
    #[must_use]
    pub fn shape(&self) -> &Shape {
        match self {
            Self::Coo(t) => t.shape(),
            Self::Csr(t) => t.shape(),
        }
    }

    /// Returns the number of non-zero elements.
    #[must_use]
    pub fn nnz(&self) -> usize {
        match self {
            Self::Coo(t) => t.nnz(),
            Self::Csr(t) => t.nnz(),
        }
    }

    /// Converts to dense tensor.
    pub fn to_dense(&self) -> NeuralResult<Tensor> {
        match self {
            Self::Coo(t) => t.to_dense(),
            Self::Csr(t) => t.to_dense(),
        }
    }

    /// Converts to CSR format.
    pub fn to_csr(&self) -> NeuralResult<CsrTensor> {
        match self {
            Self::Csr(t) => Ok(t.clone()),
            Self::Coo(t) => CsrTensor::from_coo(t),
        }
    }

    /// Converts to COO format.
    pub fn to_coo(&self) -> NeuralResult<CooTensor> {
        match self {
            Self::Coo(t) => Ok(t.clone()),
            Self::Csr(t) => {
                let dense = t.to_dense()?;
                CooTensor::from_dense(&dense)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coo_from_dense() {
        let t = Tensor::from_vec_f32(
            &[1.0, 0.0, 3.0, 0.0, 5.0, 0.0],
            Shape::from_2d(2, 3),
        );
        let coo = CooTensor::from_dense(&t).unwrap();
        assert_eq!(coo.nnz(), 3);
        assert_eq!(coo.shape().dims(), &[2, 3]);
    }

    #[test]
    fn coo_to_dense() {
        let indices = vec![vec![0, 1, 2], vec![0, 1, 2]];
        let values = vec![1.0, 2.0, 3.0];
        let coo =
            CooTensor::new(Shape::from_2d(3, 3), indices, values, DType::Float32).unwrap();
        let dense = coo.to_dense().unwrap();
        assert_eq!(dense.item_f64(&[0, 0]).unwrap(), 1.0);
        assert_eq!(dense.item_f64(&[1, 1]).unwrap(), 2.0);
        assert_eq!(dense.item_f64(&[2, 2]).unwrap(), 3.0);
        assert_eq!(dense.item_f64(&[0, 1]).unwrap(), 0.0);
    }

    #[test]
    fn csr_from_dense() {
        let t = Tensor::from_vec_f32(
            &[1.0, 0.0, 3.0, 0.0, 5.0, 0.0],
            Shape::from_2d(2, 3),
        );
        let csr = CsrTensor::from_dense(&t).unwrap();
        assert_eq!(csr.nnz(), 3);
        assert_eq!(csr.shape().dims(), &[2, 3]);
    }

    #[test]
    fn csr_to_dense() {
        let t = Tensor::from_vec_f32(
            &[1.0, 0.0, 3.0, 0.0, 5.0, 0.0],
            Shape::from_2d(2, 3),
        );
        let csr = CsrTensor::from_dense(&t).unwrap();
        let dense = csr.to_dense().unwrap();
        assert_eq!(dense.item_f64(&[0, 0]).unwrap(), 1.0);
        assert_eq!(dense.item_f64(&[0, 2]).unwrap(), 3.0);
        assert_eq!(dense.item_f64(&[1, 1]).unwrap(), 5.0);
    }

    #[test]
    fn coo_csr_roundtrip() {
        let t = Tensor::from_vec_f32(
            &[1.0, 0.0, 3.0, 0.0, 5.0, 0.0, 7.0, 0.0, 9.0],
            Shape::from_2d(3, 3),
        );
        let coo = CooTensor::from_dense(&t).unwrap();
        let csr = CsrTensor::from_coo(&coo).unwrap();
        let back = csr.to_dense().unwrap();
        assert_eq!(back.item_f64(&[0, 0]).unwrap(), 1.0);
        assert_eq!(back.item_f64(&[2, 2]).unwrap(), 9.0);
    }

    #[test]
    fn sparsity() {
        let t = Tensor::from_vec_f32(
            &[0.0, 0.0, 0.0, 0.0, 5.0, 0.0],
            Shape::from_2d(2, 3),
        );
        let coo = CooTensor::from_dense(&t).unwrap();
        assert!((coo.sparsity() - 5.0 / 6.0).abs() < 1e-6);
    }
}
