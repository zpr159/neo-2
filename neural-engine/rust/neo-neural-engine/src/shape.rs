use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::{NeuralError, NeuralResult};

/// Represents the shape of a tensor as a list of dimension sizes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Shape {
    dims: Vec<usize>,
}

impl Shape {
    /// Creates a new shape from a list of dimensions.
    #[must_use]
    pub fn new(dims: Vec<usize>) -> Self {
        Self { dims }
    }

    /// Creates a scalar shape (0 dimensions).
    #[must_use]
    pub fn scalar() -> Self {
        Self { dims: Vec::new() }
    }

    /// Creates a 1D shape.
    #[must_use]
    pub fn from_1d(dim0: usize) -> Self {
        Self { dims: vec![dim0] }
    }

    /// Creates a 2D shape.
    #[must_use]
    pub fn from_2d(rows: usize, cols: usize) -> Self {
        Self { dims: vec![rows, cols] }
    }

    /// Creates a 3D shape.
    #[must_use]
    pub fn from_3d(d0: usize, d1: usize, d2: usize) -> Self {
        Self {
            dims: vec![d0, d1, d2],
        }
    }

    /// Creates a 4D shape (common for image data: NCHW).
    #[must_use]
    pub fn from_4d(n: usize, c: usize, h: usize, w: usize) -> Self {
        Self { dims: vec![n, c, h, w] }
    }

    /// Returns a reference to the dimensions.
    #[must_use]
    pub fn dims(&self) -> &[usize] {
        &self.dims
    }

    /// Returns the number of dimensions (rank).
    #[must_use]
    pub fn ndim(&self) -> usize {
        self.dims.len()
    }

    /// Returns the total number of elements.
    #[must_use]
    pub fn numel(&self) -> usize {
        self.dims.iter().copied().product::<usize>()
    }

    /// Returns the size of a specific dimension.
    pub fn dim(&self, index: usize) -> NeuralResult<usize> {
        if index < self.dims.len() {
            Ok(self.dims[index])
        } else {
            Err(NeuralError::OutOfBounds {
                index,
                bound: self.dims.len(),
                context: "shape.dim()".to_string(),
            })
        }
    }

    /// Returns the last dimension.
    pub fn last_dim(&self) -> NeuralResult<usize> {
        self.dims
            .last()
            .copied()
            .ok_or_else(|| NeuralError::GraphValidation {
                message: "shape has no dimensions".to_string(),
            })
    }

    /// Returns the first dimension.
    pub fn first_dim(&self) -> NeuralResult<usize> {
        self.dims
            .first()
            .copied()
            .ok_or_else(|| NeuralError::GraphValidation {
                message: "shape has no dimensions".to_string(),
            })
    }

    /// Sets a specific dimension.
    pub fn set_dim(&mut self, index: usize, value: usize) -> NeuralResult<()> {
        if index < self.dims.len() {
            self.dims[index] = value;
            Ok(())
        } else {
            Err(NeuralError::OutOfBounds {
                index,
                bound: self.dims.len(),
                context: "shape.set_dim()".to_string(),
            })
        }
    }

    /// Returns true if this is a scalar (0 dimensions).
    #[must_use]
    pub fn is_scalar(&self) -> bool {
        self.dims.is_empty()
    }

    /// Returns true if all dimensions are known (no zeros).
    #[must_use]
    pub fn is_fully_specified(&self) -> bool {
        self.dims.iter().all(|&d| d > 0)
    }

    /// Returns a new shape with the given dimension removed.
    pub fn remove_dim(&self, index: usize) -> NeuralResult<Shape> {
        if index < self.dims.len() {
            let mut new_dims = self.dims.clone();
            new_dims.remove(index);
            Ok(Shape { dims: new_dims })
        } else {
            Err(NeuralError::OutOfBounds {
                index,
                bound: self.dims.len(),
                context: "shape.remove_dim()".to_string(),
            })
        }
    }

    /// Returns a new shape with a new dimension inserted at the given position.
    pub fn insert_dim(&self, index: usize, size: usize) -> NeuralResult<Shape> {
        if index <= self.dims.len() {
            let mut new_dims = self.dims.clone();
            new_dims.insert(index, size);
            Ok(Shape { dims: new_dims })
        } else {
            Err(NeuralError::OutOfBounds {
                index,
                bound: self.dims.len() + 1,
                context: "shape.insert_dim()".to_string(),
            })
        }
    }

    /// Returns the product of dimensions from start to end (exclusive).
    #[must_use]
    pub fn dim_range_product(&self, start: usize, end: usize) -> usize {
        self.dims[start..end.min(self.dims.len())]
            .iter()
            .copied()
            .product()
    }

    /// Returns a slice of the shape from start to end.
    #[must_use]
    pub fn slice(&self, start: usize, end: usize) -> Shape {
        let end = end.min(self.dims.len());
        Shape {
            dims: self.dims[start..end].to_vec(),
        }
    }

    /// Flattens the shape to a 1D shape.
    #[must_use]
    pub fn flatten(&self) -> Shape {
        Shape {
            dims: vec![self.numel()],
        }
    }

    /// Reshapes to new dimensions if the total element count matches.
    pub fn reshape(&self, new_dims: Vec<usize>) -> NeuralResult<Shape> {
        let new_numel: usize = new_dims.iter().copied().product();
        if new_numel != self.numel() {
            return Err(NeuralError::ShapeMismatch {
                expected: self.dims.clone(),
                actual: new_dims,
                context: "reshape".to_string(),
            });
        }
        Ok(Shape { dims: new_dims })
    }

    /// Returns -1 positions for inferring shapes (e.g., reshape with -1).
    #[must_use]
    pub fn infer_placeholder(&self) -> Option<usize> {
        self.dims.iter().position(|&d| d == usize::MAX)
    }

    /// Returns the shape as a vector.
    #[must_use]
    pub fn to_vec(&self) -> Vec<usize> {
        self.dims.clone()
    }
}

impl fmt::Display for Shape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.dims)
    }
}

impl Default for Shape {
    fn default() -> Self {
        Self::scalar()
    }
}

impl From<Vec<usize>> for Shape {
    fn from(dims: Vec<usize>) -> Self {
        Self::new(dims)
    }
}

impl From<&[usize]> for Shape {
    fn from(dims: &[usize]) -> Self {
        Self::new(dims.to_vec())
    }
}

impl From<usize> for Shape {
    fn from(dim: usize) -> Self {
        Self::from_1d(dim)
    }
}

impl From<(usize,)> for Shape {
    fn from((d0,): (usize,)) -> Self {
        Self::from_1d(d0)
    }
}

impl From<(usize, usize)> for Shape {
    fn from((d0, d1): (usize, usize)) -> Self {
        Self::from_2d(d0, d1)
    }
}

impl From<(usize, usize, usize)> for Shape {
    fn from((d0, d1, d2): (usize, usize, usize)) -> Self {
        Self::from_3d(d0, d1, d2)
    }
}

impl From<(usize, usize, usize, usize)> for Shape {
    fn from((d0, d1, d2, d3): (usize, usize, usize, usize)) -> Self {
        Self::from_4d(d0, d1, d2, d3)
    }
}

/// Strides for navigating a tensor's memory layout.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Strides {
    strides: Vec<usize>,
}

impl Strides {
    /// Creates strides from explicit values.
    #[must_use]
    pub fn new(strides: Vec<usize>) -> Self {
        Self { strides }
    }

    /// Computes contiguous (row-major) strides from a shape.
    #[must_use]
    pub fn contiguous(shape: &Shape) -> Self {
        let dims = shape.dims();
        if dims.is_empty() {
            return Self {
                strides: Vec::new(),
            };
        }
        let ndim = dims.len();
        let mut strides = vec![0usize; ndim];
        strides[ndim - 1] = 1;
        for i in (0..ndim - 1).rev() {
            strides[i] = strides[i + 1] * dims[i + 1];
        }
        Self { strides }
    }

    /// Returns a reference to the stride values.
    #[must_use]
    pub fn strides(&self) -> &[usize] {
        &self.strides
    }

    /// Returns the number of dimensions.
    #[must_use]
    pub fn ndim(&self) -> usize {
        self.strides.len()
    }

    /// Computes the byte offset for a given multi-dimensional index.
    #[must_use]
    pub fn byte_offset(&self, indices: &[usize]) -> usize {
        self.strides
            .iter()
            .zip(indices.iter())
            .map(|(s, i)| s * i)
            .sum()
    }

    /// Returns the stride for a given dimension.
    #[must_use]
    pub fn stride(&self, dim: usize) -> usize {
        self.strides.get(dim).copied().unwrap_or(0)
    }
}

impl fmt::Display for Strides {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.strides)
    }
}

/// Checks whether two shapes are identical.
#[must_use]
pub fn shapes_match(a: &[usize], b: &[usize]) -> bool {
    a == b
}

/// Computes the broadcast shape of two shapes following NumPy broadcasting rules.
pub fn broadcast_shapes(a: &[usize], b: &[usize]) -> NeuralResult<Vec<usize>> {
    let max_ndim = a.len().max(b.len());
    let mut result = vec![0usize; max_ndim];

    for i in 0..max_ndim {
        let dim_a = if i < max_ndim - a.len() {
            1
        } else {
            a[i - (max_ndim - a.len())]
        };
        let dim_b = if i < max_ndim - b.len() {
            1
        } else {
            b[i - (max_ndim - b.len())]
        };

        if dim_a == dim_b {
            result[i] = dim_a;
        } else if dim_a == 1 {
            result[i] = dim_b;
        } else if dim_b == 1 {
            result[i] = dim_a;
        } else {
            return Err(NeuralError::BroadcastingError {
                left_shape: a.to_vec(),
                right_shape: b.to_vec(),
            });
        }
    }

    Ok(result)
}

/// Returns true if shape `a` can be broadcast to shape `b`.
#[must_use]
pub fn can_broadcast_to(a: &[usize], b: &[usize]) -> bool {
    broadcast_shapes(a, b).is_ok()
}

/// Computes contiguous strides for a shape.
#[must_use]
pub fn compute_strides(shape: &[usize]) -> Vec<usize> {
    if shape.is_empty() {
        return Vec::new();
    }
    let ndim = shape.len();
    let mut strides = vec![0usize; ndim];
    strides[ndim - 1] = 1;
    for i in (0..ndim - 1).rev() {
        strides[i] = strides[i + 1] * shape[i + 1];
    }
    strides
}

/// Computes the total number of elements from a shape.
#[must_use]
pub fn compute_numel(shape: &[usize]) -> usize {
    shape.iter().copied().product()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shape_basics() {
        let s = Shape::from_2d(3, 4);
        assert_eq!(s.ndim(), 2);
        assert_eq!(s.numel(), 12);
        assert_eq!(s.dim(0).unwrap(), 3);
        assert_eq!(s.dim(1).unwrap(), 4);
    }

    #[test]
    fn shape_scalar() {
        let s = Shape::scalar();
        assert!(s.is_scalar());
        assert_eq!(s.numel(), 1);
    }

    #[test]
    fn shape_reshape() {
        let s = Shape::from_2d(2, 6);
        let r = s.reshape(vec![3, 4]).unwrap();
        assert_eq!(r, Shape::from_2d(3, 4));
    }

    #[test]
    fn shape_reshape_mismatch() {
        let s = Shape::from_2d(2, 3);
        let r = s.reshape(vec![2, 4]);
        assert!(r.is_err());
    }

    #[test]
    fn contiguous_strides() {
        let s = Shape::from_2d(3, 4);
        let st = Strides::contiguous(&s);
        assert_eq!(st.strides(), &[4, 1]);
    }

    #[test]
    fn broadcast_basic() {
        let a = vec![3, 4];
        let b = vec![1, 4];
        let result = broadcast_shapes(&a, &b).unwrap();
        assert_eq!(result, vec![3, 4]);
    }

    #[test]
    fn broadcast_different_ranks() {
        let a = vec![8, 1, 6];
        let b = vec![7, 1, 5, 6];
        let result = broadcast_shapes(&a, &b).unwrap();
        assert_eq!(result, vec![7, 8, 5, 6]);
    }

    #[test]
    fn broadcast_error() {
        let a = vec![3, 4];
        let b = vec![5, 4];
        assert!(broadcast_shapes(&a, &b).is_err());
    }

    #[test]
    fn compute_strides_test() {
        let strides = compute_strides(&[2, 3, 4]);
        assert_eq!(strides, &[12, 4, 1]);
    }

    #[test]
    fn byte_offset() {
        let s = Shape::from_2d(3, 4);
        let st = Strides::contiguous(&s);
        assert_eq!(st.byte_offset(&[1, 2]), 6);
        assert_eq!(st.byte_offset(&[2, 3]), 11);
    }
}
