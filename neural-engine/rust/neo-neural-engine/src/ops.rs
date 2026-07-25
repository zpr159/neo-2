use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::dtype::DType;
use crate::error::{NeuralError, NeuralResult};
use crate::shape::{broadcast_shapes, Shape};
use crate::tensor::Tensor;

/// Unique identifier for an operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OpId(pub Uuid);

impl OpId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for OpId {
    fn default() -> Self {
        Self::new()
    }
}

/// Built-in operation types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OpType {
    MatMul,
    Add,
    Sub,
    Mul,
    Div,
    Transpose,
    Reshape,
    Concat,
    Slice,
    ReduceSum,
    ReduceMean,
    ReduceMax,
    ReduceMin,
    Relu,
    Gelu,
    Sigmoid,
    Tanh,
    Softmax,
    LayerNorm,
    BatchNorm,
    Dropout,
    Embedding,
    Conv2d,
    Pool,
    Custom,
}

impl OpType {
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::MatMul => "MatMul",
            Self::Add => "Add",
            Self::Sub => "Sub",
            Self::Mul => "Mul",
            Self::Div => "Div",
            Self::Transpose => "Transpose",
            Self::Reshape => "Reshape",
            Self::Concat => "Concat",
            Self::Slice => "Slice",
            Self::ReduceSum => "ReduceSum",
            Self::ReduceMean => "ReduceMean",
            Self::ReduceMax => "ReduceMax",
            Self::ReduceMin => "ReduceMin",
            Self::Relu => "Relu",
            Self::Gelu => "Gelu",
            Self::Sigmoid => "Sigmoid",
            Self::Tanh => "Tanh",
            Self::Softmax => "Softmax",
            Self::LayerNorm => "LayerNorm",
            Self::BatchNorm => "BatchNorm",
            Self::Dropout => "Dropout",
            Self::Embedding => "Embedding",
            Self::Conv2d => "Conv2d",
            Self::Pool => "Pool",
            Self::Custom => "Custom",
        }
    }

    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "MatMul" => Some(Self::MatMul),
            "Add" => Some(Self::Add),
            "Sub" => Some(Self::Sub),
            "Mul" => Some(Self::Mul),
            "Div" => Some(Self::Div),
            "Transpose" => Some(Self::Transpose),
            "Reshape" => Some(Self::Reshape),
            "Concat" => Some(Self::Concat),
            "Slice" => Some(Self::Slice),
            "ReduceSum" => Some(Self::ReduceSum),
            "ReduceMean" => Some(Self::ReduceMean),
            "ReduceMax" => Some(Self::ReduceMax),
            "ReduceMin" => Some(Self::ReduceMin),
            "Relu" => Some(Self::Relu),
            "Gelu" => Some(Self::Gelu),
            "Sigmoid" => Some(Self::Sigmoid),
            "Tanh" => Some(Self::Tanh),
            "Softmax" => Some(Self::Softmax),
            "LayerNorm" => Some(Self::LayerNorm),
            "BatchNorm" => Some(Self::BatchNorm),
            "Dropout" => Some(Self::Dropout),
            "Embedding" => Some(Self::Embedding),
            "Conv2d" => Some(Self::Conv2d),
            "Pool" => Some(Self::Pool),
            _ => None,
        }
    }

    #[must_use]
    pub fn num_inputs(self) -> usize {
        match self {
            Self::Add | Self::Sub | Self::Mul | Self::Div | Self::MatMul => 2,
            Self::Concat => 0,
            _ => 1,
        }
    }
}

impl fmt::Display for OpType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Parameters for an operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpParams {
    pub dtype: Option<DType>,
    pub axis: Option<usize>,
    pub axes: Option<Vec<usize>>,
    pub shape: Option<Vec<usize>>,
    pub keep_dims: Option<bool>,
    pub eps: Option<f64>,
    pub momentum: Option<f64>,
    pub training: Option<bool>,
    pub dropout_rate: Option<f64>,
    pub kernel_size: Option<Vec<usize>>,
    pub stride: Option<Vec<usize>>,
    pub padding: Option<Vec<usize>>,
    pub groups: Option<usize>,
    pub ranges: Option<Vec<(usize, usize)>>,
    pub custom_name: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl OpParams {
    #[must_use]
    pub fn new() -> Self {
        Self {
            dtype: None,
            axis: None,
            axes: None,
            shape: None,
            keep_dims: None,
            eps: None,
            momentum: None,
            training: None,
            dropout_rate: None,
            kernel_size: None,
            stride: None,
            padding: None,
            groups: None,
            ranges: None,
            custom_name: None,
            metadata: HashMap::new(),
        }
    }

    #[must_use]
    pub fn with_axis(mut self, axis: usize) -> Self {
        self.axis = Some(axis);
        self
    }

    #[must_use]
    pub fn with_axes(mut self, axes: Vec<usize>) -> Self {
        self.axes = Some(axes);
        self
    }

    #[must_use]
    pub fn with_shape(mut self, shape: Vec<usize>) -> Self {
        self.shape = Some(shape);
        self
    }

    #[must_use]
    pub fn with_eps(mut self, eps: f64) -> Self {
        self.eps = Some(eps);
        self
    }
}

impl Default for OpParams {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for computing operations and their gradients.
pub trait OpCompute: Send + Sync + fmt::Debug {
    /// Returns the operation type.
    fn op_type(&self) -> OpType;

    /// Computes the output shape from input shapes.
    fn output_shape(&self, input_shapes: &[&Shape], params: &OpParams) -> NeuralResult<Shape>;

    /// Executes the operation.
    fn execute(&self, inputs: &[&Tensor], params: &OpParams) -> NeuralResult<Vec<Tensor>>;

    /// Computes the backward pass (gradient) for this operation.
    fn backward(
        &self,
        grad_output: &Tensor,
        inputs: &[&Tensor],
        params: &OpParams,
    ) -> NeuralResult<Vec<Tensor>>;
}

/// Registration entry for an operation.
#[derive(Debug, Clone)]
pub struct OpRegistration {
    pub op_type: OpType,
    pub compute: Arc<dyn OpCompute>,
}

/// Registry of all available operations.
#[derive(Debug)]
pub struct OperationRegistry {
    ops: RwLock<HashMap<String, OpRegistration>>,
}

impl OperationRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            ops: RwLock::new(HashMap::new()),
        }
    }

    /// Registers a built-in operation.
    pub fn register_builtin(&self, compute: Arc<dyn OpCompute>) {
        let op_type = compute.op_type();
        let name = op_type.name().to_string();
        self.ops.write().insert(
            name,
            OpRegistration {
                op_type,
                compute,
            },
        );
    }

    /// Registers a custom operation.
    pub fn register_custom(
        &self,
        name: String,
        compute: Arc<dyn OpCompute>,
    ) {
        let op_type = compute.op_type();
        self.ops.write().insert(
            name,
            OpRegistration {
                op_type,
                compute,
            },
        );
    }

    /// Looks up an operation by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<OpRegistration> {
        self.ops.read().get(name).cloned()
    }

    /// Returns all registered operation names.
    #[must_use]
    pub fn list(&self) -> Vec<String> {
        self.ops.read().keys().cloned().collect()
    }

    /// Returns the number of registered operations.
    #[must_use]
    pub fn count(&self) -> usize {
        self.ops.read().len()
    }
}

impl Default for OperationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Creates the default registry with all built-in operations.
#[must_use]
pub fn create_default_registry() -> Arc<OperationRegistry> {
    let registry = Arc::new(OperationRegistry::new());

    registry.register_builtin(Arc::new(MatMulOp));
    registry.register_builtin(Arc::new(AddOp));
    registry.register_builtin(Arc::new(SubOp));
    registry.register_builtin(Arc::new(MulOp));
    registry.register_builtin(Arc::new(DivOp));
    registry.register_builtin(Arc::new(TransposeOp));
    registry.register_builtin(Arc::new(ReshapeOp));
    registry.register_builtin(Arc::new(ConcatOp));
    registry.register_builtin(Arc::new(SliceOp));
    registry.register_builtin(Arc::new(ReduceSumOp));
    registry.register_builtin(Arc::new(ReduceMeanOp));
    registry.register_builtin(Arc::new(ReduceMaxOp));
    registry.register_builtin(Arc::new(ReduceMinOp));
    registry.register_builtin(Arc::new(ReluOp));
    registry.register_builtin(Arc::new(GeluOp));
    registry.register_builtin(Arc::new(SigmoidOp));
    registry.register_builtin(Arc::new(TanhOp));
    registry.register_builtin(Arc::new(SoftmaxOp));

    registry
}

// ============================================
// Built-in operation implementations
// ============================================

#[derive(Debug)]
struct MatMulOp;

impl OpCompute for MatMulOp {
    fn op_type(&self) -> OpType {
        OpType::MatMul
    }

    fn output_shape(&self, input_shapes: &[&Shape], _params: &OpParams) -> NeuralResult<Shape> {
        if input_shapes.len() != 2 {
            return Err(NeuralError::GraphValidation {
                message: "MatMul requires exactly 2 inputs".to_string(),
            });
        }
        let a = input_shapes[0];
        let b = input_shapes[1];
        if a.ndim() != 2 || b.ndim() != 2 {
            return Err(NeuralError::GraphValidation {
                message: "MatMul requires 2D inputs".to_string(),
            });
        }
        Ok(Shape::from_2d(a.dim(0)?, b.dim(1)?))
    }

    fn execute(&self, inputs: &[&Tensor], _params: &OpParams) -> NeuralResult<Vec<Tensor>> {
        Ok(vec![inputs[0].matmul(inputs[1])?])
    }

    fn backward(
        &self,
        grad_output: &Tensor,
        inputs: &[&Tensor],
        _params: &OpParams,
    ) -> NeuralResult<Vec<Tensor>> {
        let grad_a = grad_output.matmul(&inputs[1].t()?)?;
        let grad_b = inputs[0].t()?.matmul(grad_output)?;
        Ok(vec![grad_a, grad_b])
    }
}

macro_rules! impl_binary_op {
    ($name:ident, $op_type:ident, $method:ident) => {
        #[derive(Debug)]
        struct $name;

        impl OpCompute for $name {
            fn op_type(&self) -> OpType {
                OpType::$op_type
            }

            fn output_shape(
                &self,
                input_shapes: &[&Shape],
                _params: &OpParams,
            ) -> NeuralResult<Shape> {
                if input_shapes.len() != 2 {
                    return Err(NeuralError::GraphValidation {
                        message: concat!(stringify!($op_type), " requires 2 inputs").to_string(),
                    });
                }
                let dims =
                    broadcast_shapes(input_shapes[0].dims(), input_shapes[1].dims())?;
                Ok(Shape::new(dims))
            }

            fn execute(
                &self,
                inputs: &[&Tensor],
                _params: &OpParams,
            ) -> NeuralResult<Vec<Tensor>> {
                Ok(vec![inputs[0].$method(inputs[1])?])
            }

            fn backward(
                &self,
                grad_output: &Tensor,
                inputs: &[&Tensor],
                _params: &OpParams,
            ) -> NeuralResult<Vec<Tensor>> {
                match OpType::$op_type {
                    OpType::Add => Ok(vec![grad_output.clone(), grad_output.clone()]),
                    OpType::Sub => Ok(vec![grad_output.clone(), grad_output.neg()?]),
                    OpType::Mul => Ok(vec![
                        grad_output.mul(inputs[1])?,
                        grad_output.mul(inputs[0])?,
                    ]),
                    OpType::Div => {
                        let neg_a = inputs[0].neg()?;
                        let b_squared = inputs[1].mul(inputs[1])?;
                        Ok(vec![
                            grad_output.div(inputs[1])?,
                            grad_output.mul(&neg_a)?.div(&b_squared)?,
                        ])
                    }
                    _ => Ok(vec![grad_output.clone(), grad_output.clone()]),
                }
            }
        }
    };
}

impl_binary_op!(AddOp, Add, add);
impl_binary_op!(SubOp, Sub, sub);
impl_binary_op!(MulOp, Mul, mul);
impl_binary_op!(DivOp, Div, div);

#[derive(Debug)]
struct TransposeOp;

impl OpCompute for TransposeOp {
    fn op_type(&self) -> OpType {
        OpType::Transpose
    }

    fn output_shape(&self, input_shapes: &[&Shape], params: &OpParams) -> NeuralResult<Shape> {
        if input_shapes.len() != 1 {
            return Err(NeuralError::GraphValidation {
                message: "Transpose requires 1 input".to_string(),
            });
        }
        let axes = params.axes.as_ref().ok_or_else(|| NeuralError::GraphValidation {
            message: "Transpose requires axes parameter".to_string(),
        })?;
        let new_dims: Vec<usize> = axes.iter().map(|&i| input_shapes[0].dims()[i]).collect();
        Ok(Shape::new(new_dims))
    }

    fn execute(&self, inputs: &[&Tensor], params: &OpParams) -> NeuralResult<Vec<Tensor>> {
        let axes = params.axes.as_deref().unwrap_or(&[]);
        if axes.is_empty() && inputs[0].ndim() == 2 {
            return Ok(vec![inputs[0].t()?]);
        }
        Ok(vec![inputs[0].transpose(axes)?])
    }

    fn backward(
        &self,
        grad_output: &Tensor,
        _inputs: &[&Tensor],
        params: &OpParams,
    ) -> NeuralResult<Vec<Tensor>> {
        let axes = params.axes.as_deref().unwrap_or(&[]);
        if axes.is_empty() && grad_output.ndim() == 2 {
            return Ok(vec![grad_output.t()?]);
        }
        let n = grad_output.ndim();
        let mut inv_axes: Vec<usize> = vec![0; n];
        for (i, &a) in axes.iter().enumerate() {
            inv_axes[a] = i;
        }
        Ok(vec![grad_output.transpose(&inv_axes)?])
    }
}

#[derive(Debug)]
struct ReshapeOp;

impl OpCompute for ReshapeOp {
    fn op_type(&self) -> OpType {
        OpType::Reshape
    }

    fn output_shape(&self, _input_shapes: &[&Shape], params: &OpParams) -> NeuralResult<Shape> {
        let shape = params.shape.as_ref().ok_or_else(|| NeuralError::GraphValidation {
            message: "Reshape requires shape parameter".to_string(),
        })?;
        Ok(Shape::new(shape.clone()))
    }

    fn execute(&self, inputs: &[&Tensor], params: &OpParams) -> NeuralResult<Vec<Tensor>> {
        let shape = params.shape.as_ref().ok_or_else(|| NeuralError::GraphValidation {
            message: "Reshape requires shape parameter".to_string(),
        })?;
        Ok(vec![inputs[0].reshape(Shape::new(shape.clone()))?])
    }

    fn backward(
        &self,
        grad_output: &Tensor,
        inputs: &[&Tensor],
        _params: &OpParams,
    ) -> NeuralResult<Vec<Tensor>> {
        Ok(vec![grad_output.reshape(inputs[0].shape().clone())?])
    }
}

#[derive(Debug)]
struct ConcatOp;

impl OpCompute for ConcatOp {
    fn op_type(&self) -> OpType {
        OpType::Concat
    }

    fn output_shape(&self, input_shapes: &[&Shape], params: &OpParams) -> NeuralResult<Shape> {
        let axis = params.axis.unwrap_or(0);
        let mut out_shape = input_shapes[0].to_vec();
        for shape in input_shapes.iter().skip(1) {
            out_shape[axis] += shape.dim(axis)?;
        }
        Ok(Shape::new(out_shape))
    }

    fn execute(&self, inputs: &[&Tensor], params: &OpParams) -> NeuralResult<Vec<Tensor>> {
        let axis = params.axis.unwrap_or(0);
        let dtype = inputs[0].dtype();
        let out_shape_vec = {
            let mut dims = inputs[0].shape().to_vec();
            for inp in inputs.iter().skip(1) {
                dims[axis] += inp.shape().dim(axis)?;
            }
            dims
        };
        let mut output = Tensor::zeros(Shape::new(out_shape_vec), dtype);
        let mut offset = 0;
        for inp in inputs {
            let axis_size = inp.shape().dim(axis)?;
            for i in 0..inp.numel() {
                let mut coords = vec![0usize; inp.ndim()];
                let mut tmp = i;
                for d in (0..inp.ndim()).rev() {
                    coords[d] = tmp % inp.shape().dims()[d];
                    tmp /= inp.shape().dims()[d];
                }
                let mut out_coords = coords.clone();
                out_coords[axis] += offset;
                let val = inp.item_f64(&coords)?;
                output.set_item_f64(&out_coords, val)?;
            }
            offset += axis_size;
        }
        Ok(vec![output])
    }

    fn backward(
        &self,
        grad_output: &Tensor,
        inputs: &[&Tensor],
        params: &OpParams,
    ) -> NeuralResult<Vec<Tensor>> {
        let axis = params.axis.unwrap_or(0);
        let mut grads = Vec::new();
        let mut offset = 0;
        for inp in inputs {
            let axis_size = inp.shape().dim(axis)?;
            let mut ranges = vec![(0, 0); grad_output.ndim()];
            ranges[axis] = (offset, offset + axis_size);
            grads.push(grad_output.slice_dims(&ranges)?);
            offset += axis_size;
        }
        Ok(grads)
    }
}

#[derive(Debug)]
struct SliceOp;

impl OpCompute for SliceOp {
    fn op_type(&self) -> OpType {
        OpType::Slice
    }

    fn output_shape(&self, input_shapes: &[&Shape], params: &OpParams) -> NeuralResult<Shape> {
        let ranges = params.ranges.as_ref().ok_or_else(|| NeuralError::GraphValidation {
            message: "Slice requires ranges parameter".to_string(),
        })?;
        let mut out_shape = input_shapes[0].to_vec();
        for (dim, &(start, end)) in ranges.iter().enumerate() {
            if dim < out_shape.len() {
                out_shape[dim] = end - start;
            }
        }
        Ok(Shape::new(out_shape))
    }

    fn execute(&self, inputs: &[&Tensor], params: &OpParams) -> NeuralResult<Vec<Tensor>> {
        let ranges = params.ranges.as_ref().ok_or_else(|| NeuralError::GraphValidation {
            message: "Slice requires ranges parameter".to_string(),
        })?;
        Ok(vec![inputs[0].slice_dims(ranges)?])
    }

    fn backward(
        &self,
        grad_output: &Tensor,
        inputs: &[&Tensor],
        params: &OpParams,
    ) -> NeuralResult<Vec<Tensor>> {
        let ranges = params.ranges.as_ref().ok_or_else(|| NeuralError::GraphValidation {
            message: "Slice requires ranges".to_string(),
        })?;
        let mut grad = Tensor::zeros(inputs[0].shape().clone(), grad_output.dtype());
        let out_numel = grad_output.numel();
        for i in 0..out_numel {
            let mut coords = vec![0usize; grad_output.ndim()];
            let mut tmp = i;
            for d in (0..grad_output.ndim()).rev() {
                coords[d] = tmp % grad_output.shape().dims()[d];
                tmp /= grad_output.shape().dims()[d];
            }
            let mut src_coords = coords.clone();
            for (dim, &(start, _)) in ranges.iter().enumerate() {
                src_coords[dim] += start;
            }
            let val = grad_output.item_f64(&coords)?;
            grad.set_item_f64(&src_coords, val)?;
        }
        Ok(vec![grad])
    }
}

macro_rules! impl_reduce_op {
    ($name:ident, $op_type:ident, $method:ident) => {
        #[derive(Debug)]
        struct $name;

        impl OpCompute for $name {
            fn op_type(&self) -> OpType {
                OpType::$op_type
            }

            fn output_shape(
                &self,
                input_shapes: &[&Shape],
                params: &OpParams,
            ) -> NeuralResult<Shape> {
                let axis = params.axis.ok_or_else(|| NeuralError::GraphValidation {
                    message: concat!(stringify!($op_type), " requires axis").to_string(),
                })?;
                let mut out_shape = input_shapes[0].to_vec();
                out_shape.remove(axis);
                Ok(Shape::new(out_shape))
            }

            fn execute(
                &self,
                inputs: &[&Tensor],
                params: &OpParams,
            ) -> NeuralResult<Vec<Tensor>> {
                let axis = params.axis.unwrap_or(0);
                Ok(vec![inputs[0].$method(axis)?])
            }

            fn backward(
                &self,
                grad_output: &Tensor,
                inputs: &[&Tensor],
                params: &OpParams,
            ) -> NeuralResult<Vec<Tensor>> {
                let _ = inputs;
                let axis = params.axis.unwrap_or(0);
                let axis_size = inputs[0].shape().dim(axis)?;
                let mut grad = grad_output.unsqueeze(axis)?;
                let shape = grad.shape().clone();
                let filled = Tensor::full(shape, axis_size as f64, grad.dtype());
                Ok(vec![grad.mul(&filled)?])
            }
        }
    };
}

impl_reduce_op!(ReduceSumOp, ReduceSum, sum_axis);
impl_reduce_op!(ReduceMeanOp, ReduceMean, mean_axis);
impl_reduce_op!(ReduceMaxOp, ReduceMax, max_axis);
impl_reduce_op!(ReduceMinOp, ReduceMin, min_axis);

macro_rules! impl_unary_op {
    ($name:ident, $op_type:ident, $method:ident) => {
        #[derive(Debug)]
        struct $name;

        impl OpCompute for $name {
            fn op_type(&self) -> OpType {
                OpType::$op_type
            }

            fn output_shape(
                &self,
                input_shapes: &[&Shape],
                _params: &OpParams,
            ) -> NeuralResult<Shape> {
                Ok(input_shapes[0].clone())
            }

            fn execute(
                &self,
                inputs: &[&Tensor],
                _params: &OpParams,
            ) -> NeuralResult<Vec<Tensor>> {
                Ok(vec![inputs[0].$method()?])
            }

            fn backward(
                &self,
                grad_output: &Tensor,
                inputs: &[&Tensor],
                _params: &OpParams,
            ) -> NeuralResult<Vec<Tensor>> {
                match OpType::$op_type {
                    OpType::Relu => {
                        let mask = inputs[0].relu()?;
                        Ok(vec![grad_output.mul(&mask)?])
                    }
                    _ => Ok(vec![grad_output.clone()]),
                }
            }
        }
    };
}

impl_unary_op!(ReluOp, Relu, relu);
impl_unary_op!(GeluOp, Gelu, gelu);
impl_unary_op!(SigmoidOp, Sigmoid, sigmoid);
impl_unary_op!(TanhOp, Tanh, tanh);

#[derive(Debug)]
struct SoftmaxOp;

impl OpCompute for SoftmaxOp {
    fn op_type(&self) -> OpType {
        OpType::Softmax
    }

    fn output_shape(&self, input_shapes: &[&Shape], _params: &OpParams) -> NeuralResult<Shape> {
        Ok(input_shapes[0].clone())
    }

    fn execute(&self, inputs: &[&Tensor], params: &OpParams) -> NeuralResult<Vec<Tensor>> {
        let axis = params.axis.unwrap_or(inputs[0].ndim() - 1);
        Ok(vec![inputs[0].softmax(axis)?])
    }

    fn backward(
        &self,
        grad_output: &Tensor,
        inputs: &[&Tensor],
        params: &OpParams,
    ) -> NeuralResult<Vec<Tensor>> {
        let axis = params.axis.unwrap_or(inputs[0].ndim() - 1);
        let output = inputs[0].softmax(axis)?;
        // d softmax = output * (grad - sum(grad * output))
        let grad_times_out = grad_output.mul(&output)?;
        let sum_term = grad_times_out.sum_axis(axis)?;
        let diff = grad_output.sub(&sum_term.unsqueeze(axis)?)?;
        Ok(vec![diff.mul(&output)?])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn op_type_names() {
        assert_eq!(OpType::MatMul.name(), "MatMul");
        assert_eq!(OpType::from_name("Add"), Some(OpType::Add));
        assert_eq!(OpType::from_name("Unknown"), None);
    }

    #[test]
    fn registry_basics() {
        let registry = create_default_registry();
        assert!(registry.count() > 0);
        assert!(registry.get("MatMul").is_some());
        assert!(registry.get("Nonexistent").is_none());
    }

    #[test]
    fn matmul_shape_inference() {
        let op = MatMulOp;
        let shapes = vec![Shape::from_2d(2, 3), Shape::from_2d(3, 4)];
        let shape_refs: Vec<&Shape> = shapes.iter().collect();
        let out = op.output_shape(&shape_refs, &OpParams::new()).unwrap();
        assert_eq!(out, Shape::from_2d(2, 4));
    }

    #[test]
    fn add_shape_inference() {
        let op = AddOp;
        let shapes = vec![Shape::from_2d(3, 4), Shape::from_2d(1, 4)];
        let shape_refs: Vec<&Shape> = shapes.iter().collect();
        let out = op.output_shape(&shape_refs, &OpParams::new()).unwrap();
        assert_eq!(out, Shape::from_2d(3, 4));
    }

    #[test]
    fn matmul_execute() {
        let op = MatMulOp;
        let a = Tensor::from_vec_f32(&[1.0, 2.0, 3.0, 4.0], Shape::from_2d(2, 2));
        let b = Tensor::from_vec_f32(&[5.0, 6.0, 7.0, 8.0], Shape::from_2d(2, 2));
        let result = op.execute(&[&a, &b], &OpParams::new()).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].item_f64(&[0, 0]).unwrap(), 19.0);
    }
}
