use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use neo_neural_engine::shape::Shape;
use neo_neural_engine::tensor::Tensor;
use neo_neural_engine::DType;

use crate::error::{NnError, NnResult};

static GLOBAL_TENSOR_ID: AtomicUsize = AtomicUsize::new(1);

/// Unique identifier for a tensor in the computation graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TensorId(usize);

impl TensorId {
    /// Creates a new unique tensor identifier.
    pub fn new() -> Self {
        Self(GLOBAL_TENSOR_ID.fetch_add(1, Ordering::Relaxed))
    }

    /// Returns the raw ID value.
    #[must_use]
    pub fn raw(self) -> usize {
        self.0
    }
}

impl Default for TensorId {
    fn default() -> Self {
        Self::new()
    }
}

struct TapeEntry {
    #[allow(dead_code)]
    output_id: TensorId,
    input_ids: Vec<TensorId>,
    #[allow(dead_code)]
    saved_tensors: Vec<Tensor>,
    backward_fn: Box<dyn Fn(&Tensor) -> NnResult<Vec<Tensor>>>,
}

pub struct GradTape {
    entries: Vec<TapeEntry>,
}

impl GradTape {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    fn record(
        &mut self,
        output_id: TensorId,
        input_ids: Vec<TensorId>,
        saved_tensors: Vec<Tensor>,
        backward_fn: Box<dyn Fn(&Tensor) -> NnResult<Vec<Tensor>>>,
    ) {
        self.entries.push(TapeEntry {
            output_id,
            input_ids,
            saved_tensors,
            backward_fn,
        });
    }

    fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

thread_local! {
    static GRAD_TAPE: RefCell<Option<GradTape>> = const { RefCell::new(None) };
}

pub fn startRecording() {
    GRAD_TAPE.with(|tape| {
        *tape.borrow_mut() = Some(GradTape::new());
    });
}

pub fn stopRecording() -> Option<GradTape> {
    GRAD_TAPE.with(|tape| tape.borrow_mut().take())
}

pub fn isRecording() -> bool {
    GRAD_TAPE.with(|tape| tape.borrow().is_some())
}

fn record_to_tape(
    output_id: TensorId,
    input_ids: Vec<TensorId>,
    saved_tensors: Vec<Tensor>,
    backward_fn: Box<dyn Fn(&Tensor) -> NnResult<Vec<Tensor>>>,
) {
    GRAD_TAPE.with(|tape| {
        if let Some(ref mut t) = *tape.borrow_mut() {
            t.record(output_id, input_ids, saved_tensors, backward_fn);
        }
    });
}

/// An autograd tensor that tracks computation for automatic differentiation.
#[derive(Debug, Clone)]
pub struct ADTensor {
    id: TensorId,
    data: Tensor,
    requires_grad: bool,
}

impl ADTensor {
    #[must_use]
    pub fn new(data: Tensor, requires_grad: bool) -> Self {
        Self { id: TensorId::new(), data, requires_grad }
    }

    #[must_use]
    pub fn from_vec_f32(data: &[f32], shape: Shape, requires_grad: bool) -> Self {
        Self::new(Tensor::from_vec_f32(data, shape), requires_grad)
    }

    #[must_use]
    pub fn from_vec_f64(data: &[f64], shape: Shape, requires_grad: bool) -> Self {
        Self::new(Tensor::from_vec_f64(data, shape), requires_grad)
    }

    #[must_use]
    pub fn zeros(shape: Shape, dtype: DType, requires_grad: bool) -> Self {
        Self::new(Tensor::zeros(shape, dtype), requires_grad)
    }

    #[must_use]
    pub fn ones(shape: Shape, dtype: DType, requires_grad: bool) -> Self {
        Self::new(Tensor::ones(shape, dtype), requires_grad)
    }

    #[must_use]
    pub fn full(shape: Shape, value: f64, dtype: DType, requires_grad: bool) -> Self {
        Self::new(Tensor::full(shape, value, dtype), requires_grad)
    }

    #[must_use]
    pub fn id(&self) -> TensorId {
        self.id
    }

    #[must_use]
    pub fn data(&self) -> &Tensor {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut Tensor {
        &mut self.data
    }

    #[must_use]
    pub fn into_data(self) -> Tensor {
        self.data
    }

    #[must_use]
    pub fn requires_grad(&self) -> bool {
        self.requires_grad
    }

    pub fn set_requires_grad(&mut self, requires_grad: bool) {
        self.requires_grad = requires_grad;
    }

    #[must_use]
    pub fn shape(&self) -> &Shape {
        self.data.shape()
    }

    #[must_use]
    pub fn dtype(&self) -> DType {
        self.data.dtype()
    }

    #[must_use]
    pub fn numel(&self) -> usize {
        self.data.numel()
    }

    #[must_use]
    pub fn ndim(&self) -> usize {
        self.data.ndim()
    }

    #[must_use]
    pub fn detach(&self) -> Self {
        Self { id: TensorId::new(), data: self.data.detach(), requires_grad: false }
    }

    pub fn item(&self) -> NnResult<f64> {
        self.data.item().map_err(NnError::from)
    }

    pub fn to_vec_f64(&self) -> NnResult<Vec<f64>> {
        self.data.to_vec_f64().map_err(NnError::from)
    }
}

fn iter_coords(ndim: usize, shape: &[usize], flat_idx: usize) -> Vec<usize> {
    let mut coords = vec![0usize; ndim];
    let mut tmp = flat_idx;
    for d in (0..ndim).rev() {
        coords[d] = tmp % shape[d];
        tmp /= shape[d];
    }
    coords
}

fn needs_grad(a: &ADTensor, b: &ADTensor) -> bool {
    a.requires_grad || b.requires_grad
}

fn broadcast_grad(grad: &Tensor, target_shape: &Shape) -> NnResult<Tensor> {
    if grad.shape() == target_shape {
        return Ok(grad.clone());
    }
    let mut result = grad.clone();
    let r_dims = result.shape().dims().to_vec();
    for axis in (0..r_dims.len()).rev() {
        if axis < target_shape.dims().len() && r_dims[axis] != target_shape.dims()[axis] && r_dims[axis] != 1 {
            result = result.sum_axis(axis)?;
        }
    }
    while result.shape().dims().len() > target_shape.dims().len() {
        let nd = result.shape().dims().len();
        result = result.sum_axis(nd - 1)?;
    }
    for axis in 0..target_shape.dims().len() {
        if axis < result.shape().dims().len()
            && result.shape().dims()[axis] == 1
            && target_shape.dims()[axis] != 1
        {
            let reps = target_shape.dims()[axis];
            let data = result.to_vec_f64()?;
            let mut expanded = Vec::with_capacity(data.len() * reps);
            for &v in &data {
                for _ in 0..reps {
                    expanded.push(v);
                }
            }
            let mut new_dims = result.shape().dims().to_vec();
            new_dims[axis] = reps;
            result = Tensor::from_vec_f64(&expanded, Shape::new(new_dims));
        }
    }
    Ok(result)
}

fn invert_permutation(perm: &[usize]) -> Vec<usize> {
    let n = perm.len();
    let mut inv = vec![0usize; n];
    for (i, &p) in perm.iter().enumerate() {
        inv[p] = i;
    }
    inv
}

pub fn ad_add(a: &ADTensor, b: &ADTensor) -> NnResult<ADTensor> {
    let out_data = a.data.add(&b.data)?;
    let requires_grad = needs_grad(a, b);
    let out_id = TensorId::new();
    if requires_grad && isRecording() {
        let a_id = a.id;
        let b_id = b.id;
        let a_shape = a.shape().clone();
        let b_shape = b.shape().clone();
        record_to_tape(out_id, vec![a_id, b_id], vec![], Box::new(move |grad: &Tensor| {
            let ga = broadcast_grad(grad, &a_shape)?;
            let gb = broadcast_grad(grad, &b_shape)?;
            Ok(vec![ga, gb])
        }));
    }
    Ok(ADTensor { id: out_id, data: out_data, requires_grad })
}

pub fn ad_sub(a: &ADTensor, b: &ADTensor) -> NnResult<ADTensor> {
    let out_data = a.data.sub(&b.data)?;
    let requires_grad = needs_grad(a, b);
    let out_id = TensorId::new();
    if requires_grad && isRecording() {
        let a_id = a.id;
        let b_id = b.id;
        let a_shape = a.shape().clone();
        let b_shape = b.shape().clone();
        record_to_tape(out_id, vec![a_id, b_id], vec![], Box::new(move |grad: &Tensor| {
            let ga = broadcast_grad(grad, &a_shape)?;
            let neg_g = grad.neg()?;
            let gb = broadcast_grad(&neg_g, &b_shape)?;
            Ok(vec![ga, gb])
        }));
    }
    Ok(ADTensor { id: out_id, data: out_data, requires_grad })
}

pub fn ad_mul(a: &ADTensor, b: &ADTensor) -> NnResult<ADTensor> {
    let out_data = a.data.mul(&b.data)?;
    let requires_grad = needs_grad(a, b);
    let out_id = TensorId::new();
    if requires_grad && isRecording() {
        let a_id = a.id;
        let b_id = b.id;
        let a_data_cloned = a.data.clone();
        let b_data_cloned = b.data.clone();
        let a_shape = a.shape().clone();
        let b_shape = b.shape().clone();
        record_to_tape(out_id, vec![a_id, b_id], vec![a_data_cloned.clone(), b_data_cloned.clone()],
            Box::new(move |grad: &Tensor| {
                let ga_raw = grad.mul(&b_data_cloned)?;
                let ga = broadcast_grad(&ga_raw, &a_shape)?;
                let gb_raw = grad.mul(&a_data_cloned)?;
                let gb = broadcast_grad(&gb_raw, &b_shape)?;
                Ok(vec![ga, gb])
            }),
        );
    }
    Ok(ADTensor { id: out_id, data: out_data, requires_grad })
}

pub fn ad_div(a: &ADTensor, b: &ADTensor) -> NnResult<ADTensor> {
    let out_data = a.data.div(&b.data)?;
    let requires_grad = needs_grad(a, b);
    let out_id = TensorId::new();
    if requires_grad && isRecording() {
        let a_id = a.id;
        let b_id = b.id;
        let a_data_cloned = a.data.clone();
        let b_data_cloned = b.data.clone();
        let a_shape = a.shape().clone();
        let b_shape = b.shape().clone();
        record_to_tape(out_id, vec![a_id, b_id], vec![a_data_cloned.clone(), b_data_cloned.clone()],
            Box::new(move |grad: &Tensor| {
                let ga_raw = grad.div(&b_data_cloned)?;
                let ga = broadcast_grad(&ga_raw, &a_shape)?;
                let b_sq = b_data_cloned.mul(&b_data_cloned)?;
                let neg_g = grad.neg()?;
                let gb_inner = neg_g.mul(&a_data_cloned)?;
                let gb_raw = gb_inner.div(&b_sq)?;
                let gb = broadcast_grad(&gb_raw, &b_shape)?;
                Ok(vec![ga, gb])
            }),
        );
    }
    Ok(ADTensor { id: out_id, data: out_data, requires_grad })
}

pub fn ad_neg(a: &ADTensor) -> NnResult<ADTensor> {
    let out_data = a.data.neg()?;
    let requires_grad = a.requires_grad;
    let out_id = TensorId::new();
    if requires_grad && isRecording() {
        let a_id = a.id;
        record_to_tape(out_id, vec![a_id], vec![], Box::new(move |grad: &Tensor| {
            Ok(vec![grad.neg()?])
        }));
    }
    Ok(ADTensor { id: out_id, data: out_data, requires_grad })
}

pub fn ad_matmul(a: &ADTensor, b: &ADTensor) -> NnResult<ADTensor> {
    let out_data = a.data.matmul(&b.data)?;
    let requires_grad = needs_grad(a, b);
    let out_id = TensorId::new();
    if requires_grad && isRecording() {
        let a_id = a.id;
        let b_id = b.id;
        let a_data_cloned = a.data.clone();
        let b_data_cloned = b.data.clone();
        record_to_tape(out_id, vec![a_id, b_id], vec![a_data_cloned.clone(), b_data_cloned.clone()],
            Box::new(move |grad: &Tensor| {
                let b_t = b_data_cloned.t()?;
                let ga = grad.matmul(&b_t)?;
                let a_t = a_data_cloned.t()?;
                let gb = a_t.matmul(grad)?;
                Ok(vec![ga, gb])
            }),
        );
    }
    Ok(ADTensor { id: out_id, data: out_data, requires_grad })
}

pub fn ad_relu(a: &ADTensor) -> NnResult<ADTensor> {
    let out_data = a.data.relu()?;
    let requires_grad = a.requires_grad;
    let out_id = TensorId::new();
    if requires_grad && isRecording() {
        let a_id = a.id;
        let ndim = a.ndim();
        let shape = a.shape().clone();
        let numel = a.numel();
        let mut mask_vals = Vec::with_capacity(numel);
        for i in 0..numel {
            let coords = iter_coords(ndim, shape.dims(), i);
            mask_vals.push(a.data().item_f64(&coords)? > 0.0);
        }
        let mask = Tensor::from_vec_bool(&mask_vals, shape);
        record_to_tape(out_id, vec![a_id], vec![mask.clone()], Box::new(move |grad: &Tensor| {
            let nd = mask.shape().dims().len();
            let n = mask.numel();
            let mut gi = Tensor::zeros(mask.shape().clone(), grad.dtype());
            for i in 0..n {
                let c = iter_coords(nd, mask.shape().dims(), i);
                let m = mask.item_f64(&c)?;
                let g = grad.item_f64(&c)?;
                gi.set_item_f64(&c, m * g)?;
            }
            Ok(vec![gi])
        }));
    }
    Ok(ADTensor { id: out_id, data: out_data, requires_grad })
}

pub fn ad_gelu(a: &ADTensor) -> NnResult<ADTensor> {
    let out_data = a.data.gelu()?;
    let requires_grad = a.requires_grad;
    let out_id = TensorId::new();
    if requires_grad && isRecording() {
        let a_id = a.id;
        let saved_clone = out_data.clone();
        let ndim = saved_clone.shape().dims().len();
        let numel = saved_clone.numel();
        let shape = saved_clone.shape().clone();
        let c: f64 = 0.7978845608;
        let k: f64 = 0.044715;
        let mut derivative_vals = Vec::with_capacity(numel);
        for i in 0..numel {
            let coords = iter_coords(ndim, shape.dims(), i);
            let x = saved_clone.item_f64(&coords)?;
            derivative_vals.push(0.5 * (1.0 + (c * x * (1.0 + k * x * x)).tanh()) + 0.5 * x * (1.0 - ((c * x * (1.0 + k * x * x)).tanh()).powi(2)) * c * (1.0 + 3.0 * k * x * x));
        }
        let deriv = Tensor::from_vec_f64(&derivative_vals, shape);
        record_to_tape(out_id, vec![a_id], vec![deriv.clone()], Box::new(move |grad: &Tensor| {
            let nd = deriv.shape().dims().len();
            let n = deriv.numel();
            let mut gi = Tensor::zeros(deriv.shape().clone(), grad.dtype());
            for i in 0..n {
                let c = iter_coords(nd, deriv.shape().dims(), i);
                let d = deriv.item_f64(&c)?;
                let g = grad.item_f64(&c)?;
                gi.set_item_f64(&c, g * d)?;
            }
            Ok(vec![gi])
        }));
    }
    Ok(ADTensor { id: out_id, data: out_data, requires_grad })
}

pub fn ad_sigmoid(a: &ADTensor) -> NnResult<ADTensor> {
    let out_data = a.data.sigmoid()?;
    let requires_grad = a.requires_grad;
    let out_id = TensorId::new();
    if requires_grad && isRecording() {
        let a_id = a.id;
        let saved_clone = out_data.clone();
        record_to_tape(out_id, vec![a_id], vec![saved_clone.clone()], Box::new(move |grad: &Tensor| {
            let nd = saved_clone.shape().dims().len();
            let n = saved_clone.numel();
            let mut gi = Tensor::zeros(saved_clone.shape().clone(), grad.dtype());
            for i in 0..n {
                let c = iter_coords(nd, saved_clone.shape().dims(), i);
                let s = saved_clone.item_f64(&c)?;
                let g = grad.item_f64(&c)?;
                gi.set_item_f64(&c, g * s * (1.0 - s))?;
            }
            Ok(vec![gi])
        }));
    }
    Ok(ADTensor { id: out_id, data: out_data, requires_grad })
}

pub fn ad_tanh(a: &ADTensor) -> NnResult<ADTensor> {
    let out_data = a.data.tanh()?;
    let requires_grad = a.requires_grad;
    let out_id = TensorId::new();
    if requires_grad && isRecording() {
        let a_id = a.id;
        let saved_clone = out_data.clone();
        record_to_tape(out_id, vec![a_id], vec![saved_clone.clone()], Box::new(move |grad: &Tensor| {
            let nd = saved_clone.shape().dims().len();
            let n = saved_clone.numel();
            let mut gi = Tensor::zeros(saved_clone.shape().clone(), grad.dtype());
            for i in 0..n {
                let c = iter_coords(nd, saved_clone.shape().dims(), i);
                let t = saved_clone.item_f64(&c)?;
                let g = grad.item_f64(&c)?;
                gi.set_item_f64(&c, g * (1.0 - t * t))?;
            }
            Ok(vec![gi])
        }));
    }
    Ok(ADTensor { id: out_id, data: out_data, requires_grad })
}

fn elementwise_forward(a: &ADTensor, f: impl Fn(f64) -> f64) -> NnResult<(Tensor, bool)> {
    let ndim = a.ndim();
    let numel = a.numel();
    let mut out = Tensor::zeros(a.shape().clone(), a.dtype());
    for i in 0..numel {
        let c = iter_coords(ndim, a.shape().dims(), i);
        let v = a.data().item_f64(&c)?;
        out.set_item_f64(&c, f(v))?;
    }
    Ok((out, a.requires_grad))
}

pub fn ad_exp(a: &ADTensor) -> NnResult<ADTensor> {
    let (out_data, requires_grad) = elementwise_forward(a, f64::exp)?;
    let out_id = TensorId::new();
    if requires_grad && isRecording() {
        let a_id = a.id;
        let saved = out_data.clone();
        record_to_tape(out_id, vec![a_id], vec![saved.clone()], Box::new(move |grad: &Tensor| {
            let nd = saved.shape().dims().len();
            let n = saved.numel();
            let mut gi = Tensor::zeros(saved.shape().clone(), grad.dtype());
            for i in 0..n {
                let c = iter_coords(nd, saved.shape().dims(), i);
                let y = saved.item_f64(&c)?;
                let g = grad.item_f64(&c)?;
                gi.set_item_f64(&c, g * y)?;
            }
            Ok(vec![gi])
        }));
    }
    Ok(ADTensor { id: out_id, data: out_data, requires_grad })
}

pub fn ad_log(a: &ADTensor) -> NnResult<ADTensor> {
    let (out_data, requires_grad) = elementwise_forward(a, f64::ln)?;
    let out_id = TensorId::new();
    if requires_grad && isRecording() {
        let a_id = a.id;
        let saved_data = a.data().clone();
        record_to_tape(out_id, vec![a_id], vec![saved_data.clone()], Box::new(move |grad: &Tensor| {
            let nd = saved_data.shape().dims().len();
            let n = saved_data.numel();
            let mut gi = Tensor::zeros(saved_data.shape().clone(), grad.dtype());
            for i in 0..n {
                let c = iter_coords(nd, saved_data.shape().dims(), i);
                let x = saved_data.item_f64(&c)?;
                let g = grad.item_f64(&c)?;
                gi.set_item_f64(&c, g / x)?;
            }
            Ok(vec![gi])
        }));
    }
    Ok(ADTensor { id: out_id, data: out_data, requires_grad })
}

pub fn ad_sqrt(a: &ADTensor) -> NnResult<ADTensor> {
    let (out_data, requires_grad) = elementwise_forward(a, f64::sqrt)?;
    let out_id = TensorId::new();
    if requires_grad && isRecording() {
        let a_id = a.id;
        let saved_out = out_data.clone();
        record_to_tape(out_id, vec![a_id], vec![saved_out.clone()], Box::new(move |grad: &Tensor| {
            let nd = saved_out.shape().dims().len();
            let n = saved_out.numel();
            let mut gi = Tensor::zeros(saved_out.shape().clone(), grad.dtype());
            for i in 0..n {
                let c = iter_coords(nd, saved_out.shape().dims(), i);
                let y = saved_out.item_f64(&c)?;
                let g = grad.item_f64(&c)?;
                let d = if y > 1e-12 { 0.5 / y } else { 0.0 };
                gi.set_item_f64(&c, g * d)?;
            }
            Ok(vec![gi])
        }));
    }
    Ok(ADTensor { id: out_id, data: out_data, requires_grad })
}

pub fn ad_abs(a: &ADTensor) -> NnResult<ADTensor> {
    let (out_data, requires_grad) = elementwise_forward(a, f64::abs)?;
    let out_id = TensorId::new();
    if requires_grad && isRecording() {
        let a_id = a.id;
        let saved_data = a.data().clone();
        record_to_tape(out_id, vec![a_id], vec![saved_data.clone()], Box::new(move |grad: &Tensor| {
            let nd = saved_data.shape().dims().len();
            let n = saved_data.numel();
            let mut gi = Tensor::zeros(saved_data.shape().clone(), grad.dtype());
            for i in 0..n {
                let c = iter_coords(nd, saved_data.shape().dims(), i);
                let x = saved_data.item_f64(&c)?;
                let g = grad.item_f64(&c)?;
                let sign = if x > 0.0 { 1.0 } else if x < 0.0 { -1.0 } else { 0.0 };
                gi.set_item_f64(&c, g * sign)?;
            }
            Ok(vec![gi])
        }));
    }
    Ok(ADTensor { id: out_id, data: out_data, requires_grad })
}

pub fn ad_pow(a: &ADTensor, exp: f64) -> NnResult<ADTensor> {
    let (out_data, requires_grad) = elementwise_forward(a, |x| x.powf(exp))?;
    let out_id = TensorId::new();
    if requires_grad && isRecording() {
        let a_id = a.id;
        let saved_data = a.data().clone();
        record_to_tape(out_id, vec![a_id], vec![saved_data.clone()], Box::new(move |grad: &Tensor| {
            let nd = saved_data.shape().dims().len();
            let n = saved_data.numel();
            let mut gi = Tensor::zeros(saved_data.shape().clone(), grad.dtype());
            for i in 0..n {
                let c = iter_coords(nd, saved_data.shape().dims(), i);
                let x = saved_data.item_f64(&c)?;
                let g = grad.item_f64(&c)?;
                gi.set_item_f64(&c, g * exp * x.powf(exp - 1.0))?;
            }
            Ok(vec![gi])
        }));
    }
    Ok(ADTensor { id: out_id, data: out_data, requires_grad })
}

pub fn ad_softplus(a: &ADTensor) -> NnResult<ADTensor> {
    let ndim = a.ndim();
    let numel = a.numel();
    let shape = a.shape().clone();
    let mut out = Tensor::zeros(shape.clone(), a.dtype());
    for i in 0..numel {
        let c = iter_coords(ndim, shape.dims(), i);
        let x = a.data().item_f64(&c)?;
        let v = if x > 20.0 { x } else if x < -20.0 { 0.0 } else { (1.0 + x.exp()).ln() };
        out.set_item_f64(&c, v)?;
    }
    let requires_grad = a.requires_grad;
    let out_id = TensorId::new();
    if requires_grad && isRecording() {
        let a_id = a.id;
        let saved = out.clone();
        record_to_tape(out_id, vec![a_id], vec![saved.clone()], Box::new(move |grad: &Tensor| {
            let nd = saved.shape().dims().len();
            let n = saved.numel();
            let mut gi = Tensor::zeros(saved.shape().clone(), grad.dtype());
            for i in 0..n {
                let c = iter_coords(nd, saved.shape().dims(), i);
                let y = saved.item_f64(&c)?;
                let g = grad.item_f64(&c)?;
                gi.set_item_f64(&c, g * (1.0 - (-y).exp()))?;
            }
            Ok(vec![gi])
        }));
    }
    Ok(ADTensor { id: out_id, data: out, requires_grad })
}

pub fn ad_softsign(a: &ADTensor) -> NnResult<ADTensor> {
    let (out_data, requires_grad) = elementwise_forward(a, |x| x / (1.0 + x.abs()))?;
    let out_id = TensorId::new();
    if requires_grad && isRecording() {
        let a_id = a.id;
        let saved = out_data.clone();
        record_to_tape(out_id, vec![a_id], vec![saved.clone()], Box::new(move |grad: &Tensor| {
            let nd = saved.shape().dims().len();
            let n = saved.numel();
            let mut gi = Tensor::zeros(saved.shape().clone(), grad.dtype());
            for i in 0..n {
                let c = iter_coords(nd, saved.shape().dims(), i);
                let y = saved.item_f64(&c)?;
                let g = grad.item_f64(&c)?;
                gi.set_item_f64(&c, g * (1.0 - y).powi(2))?;
            }
            Ok(vec![gi])
        }));
    }
    Ok(ADTensor { id: out_id, data: out_data, requires_grad })
}

pub fn ad_elu(a: &ADTensor, alpha: f64) -> NnResult<ADTensor> {
    let ndim = a.ndim();
    let numel = a.numel();
    let shape = a.shape().clone();
    let mut out = Tensor::zeros(shape.clone(), a.dtype());
    for i in 0..numel {
        let c = iter_coords(ndim, shape.dims(), i);
        let x = a.data().item_f64(&c)?;
        let v = if x > 0.0 { x } else { alpha * (x.exp() - 1.0) };
        out.set_item_f64(&c, v)?;
    }
    let requires_grad = a.requires_grad;
    let out_id = TensorId::new();
    if requires_grad && isRecording() {
        let a_id = a.id;
        let saved = out.clone();
        record_to_tape(out_id, vec![a_id], vec![saved.clone()], Box::new(move |grad: &Tensor| {
            let nd = saved.shape().dims().len();
            let n = saved.numel();
            let mut gi = Tensor::zeros(saved.shape().clone(), grad.dtype());
            for i in 0..n {
                let c = iter_coords(nd, saved.shape().dims(), i);
                let y = saved.item_f64(&c)?;
                let g = grad.item_f64(&c)?;
                let d = if y > 0.0 { 1.0 } else { y + alpha };
                gi.set_item_f64(&c, g * d)?;
            }
            Ok(vec![gi])
        }));
    }
    Ok(ADTensor { id: out_id, data: out, requires_grad })
}

pub fn ad_selu(a: &ADTensor) -> NnResult<ADTensor> {
    let alpha: f64 = 1.6732632423543772;
    let scale: f64 = 1.0507009873554805;
    let elu_out = ad_elu(a, alpha)?;
    let sc = ADTensor::full(elu_out.shape().clone(), scale, elu_out.dtype(), false);
    ad_mul(&elu_out, &sc)
}

pub fn ad_swish(a: &ADTensor) -> NnResult<ADTensor> {
    let sig = ad_sigmoid(a)?;
    ad_mul(a, &sig)
}

pub fn ad_mish(a: &ADTensor) -> NnResult<ADTensor> {
    let sp = ad_softplus(a)?;
    let t = ad_tanh(&sp)?;
    ad_mul(a, &t)
}

pub fn ad_hard_sigmoid(a: &ADTensor) -> NnResult<ADTensor> {
    let ndim = a.ndim();
    let numel = a.numel();
    let shape = a.shape().clone();
    let mut out = Tensor::zeros(shape.clone(), a.dtype());
    for i in 0..numel {
        let c = iter_coords(ndim, shape.dims(), i);
        let x = a.data().item_f64(&c)?;
        let v = (x / 6.0 + 0.5).clamp(0.0, 1.0);
        out.set_item_f64(&c, v)?;
    }
    let requires_grad = a.requires_grad;
    let out_id = TensorId::new();
    if requires_grad && isRecording() {
        let a_id = a.id;
        let saved = out.clone();
        record_to_tape(out_id, vec![a_id], vec![saved.clone()], Box::new(move |grad: &Tensor| {
            let nd = saved.shape().dims().len();
            let n = saved.numel();
            let mut gi = Tensor::zeros(saved.shape().clone(), grad.dtype());
            for i in 0..n {
                let c = iter_coords(nd, saved.shape().dims(), i);
                let y = saved.item_f64(&c)?;
                let g = grad.item_f64(&c)?;
                let d = if y > 0.0 && y < 1.0 { 1.0 / 6.0 } else { 0.0 };
                gi.set_item_f64(&c, g * d)?;
            }
            Ok(vec![gi])
        }));
    }
    Ok(ADTensor { id: out_id, data: out, requires_grad })
}

pub fn ad_hard_swish(a: &ADTensor) -> NnResult<ADTensor> {
    let ndim = a.ndim();
    let numel = a.numel();
    let shape = a.shape().clone();
    let mut out = Tensor::zeros(shape.clone(), a.dtype());
    for i in 0..numel {
        let c = iter_coords(ndim, shape.dims(), i);
        let x = a.data().item_f64(&c)?;
        let v = x * (x + 3.0).clamp(0.0, 6.0) / 6.0;
        out.set_item_f64(&c, v)?;
    }
    let requires_grad = a.requires_grad;
    let out_id = TensorId::new();
    if requires_grad && isRecording() {
        let a_id = a.id;
        let saved = out.clone();
        let saved_x = a.data().clone();
        record_to_tape(out_id, vec![a_id], vec![saved.clone(), saved_x.clone()],
            Box::new(move |grad: &Tensor| {
                let nd = saved.shape().dims().len();
                let n = saved.numel();
                let mut gi = Tensor::zeros(saved.shape().clone(), grad.dtype());
                for i in 0..n {
                    let c = iter_coords(nd, saved.shape().dims(), i);
                    let x = saved_x.item_f64(&c)?;
                    let g = grad.item_f64(&c)?;
                    let d = if x <= -3.0 {
                        0.0
                    } else if x >= 3.0 {
                        1.0
                    } else {
                        (2.0 * x + 3.0) / 6.0
                    };
                    gi.set_item_f64(&c, g * d)?;
                }
                Ok(vec![gi])
            }),
        );
    }
    Ok(ADTensor { id: out_id, data: out, requires_grad })
}

pub fn ad_prelu(a: &ADTensor, weight: &ADTensor) -> NnResult<ADTensor> {
    let ndim = a.ndim();
    let numel = a.numel();
    let shape = a.shape().clone();
    let mut out = Tensor::zeros(shape.clone(), a.dtype());
    for i in 0..numel {
        let c = iter_coords(ndim, shape.dims(), i);
        let x = a.data().item_f64(&c)?;
        let v = if x > 0.0 {
            x
        } else {
            let w_idx = if weight.numel() == 1 { 0 } else { c.last().copied().unwrap_or(0) % weight.numel() };
            let mut wc = vec![0usize; weight.ndim()];
            if weight.ndim() > 0 { wc[weight.ndim() - 1] = w_idx; }
            let w = weight.data().item_f64(&wc).unwrap_or(0.0);
            w * x
        };
        out.set_item_f64(&c, v)?;
    }
    let requires_grad = a.requires_grad || weight.requires_grad;
    let out_id = TensorId::new();
    if requires_grad && isRecording() {
        let a_id = a.id;
        let w_id = weight.id;
        let saved_data = a.data().clone();
        let saved_w = weight.data().clone();
        record_to_tape(out_id, vec![a_id, w_id], vec![saved_data.clone(), saved_w.clone()],
            Box::new(move |grad: &Tensor| {
                let nd = grad.shape().dims().len();
                let n = grad.numel();
                let mut ga = Tensor::zeros(grad.shape().clone(), grad.dtype());
                let mut gw = Tensor::zeros(saved_w.shape().clone(), saved_w.dtype());
                for i in 0..n {
                    let c = iter_coords(nd, grad.shape().dims(), i);
                    let x = saved_data.item_f64(&c)?;
                    let g = grad.item_f64(&c)?;
                    if x > 0.0 {
                        ga.set_item_f64(&c, g)?;
                    } else {
                        let w_idx = if saved_w.numel() == 1 { 0 } else { c.last().copied().unwrap_or(0) % saved_w.numel() };
                        let mut wc = vec![0usize; saved_w.ndim()];
                        if saved_w.ndim() > 0 { wc[saved_w.ndim() - 1] = w_idx; }
                        let w = saved_w.item_f64(&wc).unwrap_or(0.0);
                        ga.set_item_f64(&c, g * w)?;
                        let prev = gw.item_f64(&wc).unwrap_or(0.0);
                        gw.set_item_f64(&wc, prev + g * x)?;
                    }
                }
                Ok(vec![ga, gw])
            }),
        );
    }
    Ok(ADTensor { id: out_id, data: out, requires_grad })
}

pub fn ad_clamp(a: &ADTensor, min_val: f64, max_val: f64) -> NnResult<ADTensor> {
    let ndim = a.ndim();
    let numel = a.numel();
    let shape = a.shape().clone();
    let mut out = Tensor::zeros(shape.clone(), a.dtype());
    for i in 0..numel {
        let c = iter_coords(ndim, shape.dims(), i);
        let x = a.data().item_f64(&c)?;
        out.set_item_f64(&c, x.clamp(min_val, max_val))?;
    }
    let requires_grad = a.requires_grad;
    let out_id = TensorId::new();
    if requires_grad && isRecording() {
        let a_id = a.id;
        let saved = out.clone();
        record_to_tape(out_id, vec![a_id], vec![], Box::new(move |grad: &Tensor| {
            let nd = grad.shape().dims().len();
            let n = grad.numel();
            let mut gi = Tensor::zeros(grad.shape().clone(), grad.dtype());
            for i in 0..n {
                let c = iter_coords(nd, grad.shape().dims(), i);
                let g = grad.item_f64(&c)?;
                let y = saved.item_f64(&c)?;
                let pass = if y >= min_val && y <= max_val { 1.0 } else { 0.0 };
                gi.set_item_f64(&c, g * pass)?;
            }
            Ok(vec![gi])
        }));
    }
    Ok(ADTensor { id: out_id, data: out, requires_grad })
}

pub fn ad_softmax(a: &ADTensor, axis: usize) -> NnResult<ADTensor> {
    let out_data = a.data.softmax(axis)?;
    let requires_grad = a.requires_grad;
    let out_id = TensorId::new();
    if requires_grad && isRecording() {
        let a_id = a.id;
        let saved = out_data.clone();
        let ndim = saved.shape().dims().len();
        let axis_size = saved.shape().dims()[axis];
        let shape_clone = saved.shape().clone();
        let outer: usize = shape_clone.dims()[..axis].iter().copied().product();
        let inner: usize = shape_clone.dims()[axis + 1..].iter().copied().product();
        record_to_tape(out_id, vec![a_id], vec![saved.clone()], Box::new(move |grad: &Tensor| {
            let mut gi = Tensor::zeros(shape_clone.clone(), grad.dtype());
            for o in 0..outer {
                for i in 0..inner {
                    let mut sum = 0.0;
                    for k in 0..axis_size {
                        let mut coords = vec![0usize; ndim];
                        let mut o_tmp = o;
                        for d in 0..axis { coords[d] = o_tmp % shape_clone.dims()[d]; o_tmp /= shape_clone.dims()[d]; }
                        coords[axis] = k;
                        let mut i_tmp = i;
                        for d in (axis + 1)..ndim { coords[d] = i_tmp % shape_clone.dims()[d]; i_tmp /= shape_clone.dims()[d]; }
                        sum += grad.item_f64(&coords)? * saved.item_f64(&coords)?;
                    }
                    for k in 0..axis_size {
                        let mut coords = vec![0usize; ndim];
                        let mut o_tmp = o;
                        for d in 0..axis { coords[d] = o_tmp % shape_clone.dims()[d]; o_tmp /= shape_clone.dims()[d]; }
                        coords[axis] = k;
                        let mut i_tmp = i;
                        for d in (axis + 1)..ndim { coords[d] = i_tmp % shape_clone.dims()[d]; i_tmp /= shape_clone.dims()[d]; }
                        let s = saved.item_f64(&coords)?;
                        let g = grad.item_f64(&coords)?;
                        gi.set_item_f64(&coords, s * (g - sum))?;
                    }
                }
            }
            Ok(vec![gi])
        }));
    }
    Ok(ADTensor { id: out_id, data: out_data, requires_grad })
}

pub fn ad_sum(a: &ADTensor, axis: usize) -> NnResult<ADTensor> {
    let out_data = a.data.sum_axis(axis)?;
    let requires_grad = a.requires_grad;
    let out_id = TensorId::new();
    if requires_grad && isRecording() {
        let a_id = a.id;
        let a_shape = a.shape().clone();
        let target_len = a_shape.dims()[axis];
        let a_shape_clone = a_shape.clone();
        record_to_tape(out_id, vec![a_id], vec![], Box::new(move |grad: &Tensor| {
            let mut expanded = grad.unsqueeze(axis)?;
            let e_shape = expanded.shape().dims().to_vec();
            let mut data = expanded.to_vec_f64()?;
            let mut new_data = Vec::new();
            for chunk in data.chunks(1) {
                for _ in 0..target_len {
                    new_data.extend_from_slice(chunk);
                }
            }
            Ok(vec![Tensor::from_vec_f64(&new_data, a_shape_clone.clone())])
        }));
    }
    Ok(ADTensor { id: out_id, data: out_data, requires_grad })
}

pub fn ad_mean(a: &ADTensor, axis: usize) -> NnResult<ADTensor> {
    let axis_size = a.shape().dims()[axis] as f64;
    let s = ad_sum(a, axis)?;
    let sc = ADTensor::full(s.shape().clone(), 1.0 / axis_size, s.dtype(), false);
    ad_mul(&s, &sc)
}

pub fn ad_sum_all(a: &ADTensor) -> NnResult<ADTensor> {
    let ndim = a.ndim();
    let mut sum = 0.0;
    for i in 0..a.numel() {
        let c = iter_coords(ndim, a.shape().dims(), i);
        sum += a.data().item_f64(&c)?;
    }
    let out_data = Tensor::from_vec_f64(&[sum], Shape::from_1d(1));
    let requires_grad = a.requires_grad;
    let out_id = TensorId::new();
    if requires_grad && isRecording() {
        let a_id = a.id;
        let a_shape = a.shape().clone();
        let a_shape_clone = a_shape.clone();
        record_to_tape(out_id, vec![a_id], vec![], Box::new(move |grad: &Tensor| {
            let g = grad.item_f64(&[0]).unwrap_or(0.0);
            Ok(vec![Tensor::full(a_shape_clone.clone(), g, grad.dtype())])
        }));
    }
    Ok(ADTensor { id: out_id, data: out_data, requires_grad })
}

pub fn ad_mean_all(a: &ADTensor) -> NnResult<ADTensor> {
    let numel = a.numel() as f64;
    let ndim = a.ndim();
    let mut sum = 0.0;
    for i in 0..a.numel() {
        let c = iter_coords(ndim, a.shape().dims(), i);
        sum += a.data().item_f64(&c)?;
    }
    let out_data = Tensor::from_vec_f64(&[sum / numel], Shape::from_1d(1));
    let requires_grad = a.requires_grad;
    let out_id = TensorId::new();
    if requires_grad && isRecording() {
        let a_id = a.id;
        let a_shape = a.shape().clone();
        let a_shape_clone = a_shape.clone();
        let scale = 1.0 / numel;
        record_to_tape(out_id, vec![a_id], vec![], Box::new(move |grad: &Tensor| {
            let g = grad.item_f64(&[0]).unwrap_or(0.0);
            Ok(vec![Tensor::full(a_shape_clone.clone(), g * scale, grad.dtype())])
        }));
    }
    Ok(ADTensor { id: out_id, data: out_data, requires_grad })
}

pub fn ad_reshape(a: &ADTensor, new_shape: Shape) -> NnResult<ADTensor> {
    let out_data = a.data.reshape(new_shape)?;
    let requires_grad = a.requires_grad;
    let out_id = TensorId::new();
    if requires_grad && isRecording() {
        let a_id = a.id;
        let orig_shape = a.shape().clone();
        let orig_shape_clone = orig_shape.clone();
        record_to_tape(out_id, vec![a_id], vec![], Box::new(move |grad: &Tensor| {
            Ok(vec![grad.reshape(orig_shape_clone.clone())?])
        }));
    }
    Ok(ADTensor { id: out_id, data: out_data, requires_grad })
}

pub fn ad_flatten(a: &ADTensor) -> NnResult<ADTensor> {
    ad_reshape(a, Shape::from_1d(a.numel()))
}

pub fn ad_squeeze(a: &ADTensor) -> NnResult<ADTensor> {
    let out_data = a.data.squeeze()?;
    let requires_grad = a.requires_grad;
    let out_id = TensorId::new();
    if requires_grad && isRecording() {
        let a_id = a.id;
        let orig_shape = a.shape().clone();
        let orig_shape_clone = orig_shape.clone();
        record_to_tape(out_id, vec![a_id], vec![], Box::new(move |grad: &Tensor| {
            Ok(vec![grad.reshape(orig_shape_clone.clone())?])
        }));
    }
    Ok(ADTensor { id: out_id, data: out_data, requires_grad })
}

pub fn ad_unsqueeze(a: &ADTensor, axis: usize) -> NnResult<ADTensor> {
    let out_data = a.data.unsqueeze(axis)?;
    let requires_grad = a.requires_grad;
    let out_id = TensorId::new();
    if requires_grad && isRecording() {
        let a_id = a.id;
        record_to_tape(out_id, vec![a_id], vec![], Box::new(move |grad: &Tensor| {
            Ok(vec![grad.squeeze_axis(axis)?])
        }));
    }
    Ok(ADTensor { id: out_id, data: out_data, requires_grad })
}

pub fn ad_transpose(a: &ADTensor, axes: &[usize]) -> NnResult<ADTensor> {
    let out_data = a.data.transpose(axes)?;
    let requires_grad = a.requires_grad;
    let out_id = TensorId::new();
    if requires_grad && isRecording() {
        let a_id = a.id;
        let inv = invert_permutation(axes);
        record_to_tape(out_id, vec![a_id], vec![], Box::new(move |grad: &Tensor| {
            Ok(vec![grad.transpose(&inv)?])
        }));
    }
    Ok(ADTensor { id: out_id, data: out_data, requires_grad })
}

pub fn ad_t(a: &ADTensor) -> NnResult<ADTensor> {
    let out_data = a.data.t()?;
    let requires_grad = a.requires_grad;
    let out_id = TensorId::new();
    if requires_grad && isRecording() {
        let a_id = a.id;
        record_to_tape(out_id, vec![a_id], vec![], Box::new(move |grad: &Tensor| {
            Ok(vec![grad.t()?])
        }));
    }
    Ok(ADTensor { id: out_id, data: out_data, requires_grad })
}

pub fn ad_slice(a: &ADTensor, dim: usize, start: usize, end: usize) -> NnResult<ADTensor> {
    let out_data = a.data.slice(dim, start, end)?;
    let requires_grad = a.requires_grad;
    let out_id = TensorId::new();
    if requires_grad && isRecording() {
        let a_id = a.id;
        let a_shape = a.shape().clone();
        let ndim = a_shape.dims().len();
        record_to_tape(out_id, vec![a_id], vec![], Box::new(move |grad: &Tensor| {
            let mut gi = Tensor::zeros(a_shape.clone(), grad.dtype());
            let g_dims = grad.shape().dims();
            let outer: usize = g_dims[..dim].iter().copied().product();
            let slice_len = end - start;
            let inner: usize = if dim + 1 < g_dims.len() { g_dims[dim + 1..].iter().copied().product() } else { 1 };
            for o in 0..outer {
                for s in 0..slice_len {
                    for i in 0..inner {
                        let mut src_c = vec![0usize; ndim];
                        let mut o_tmp = o;
                        for d in 0..dim { src_c[d] = o_tmp % a_shape.dims()[d]; o_tmp /= a_shape.dims()[d]; }
                        src_c[dim] = start + s;
                        let mut i_tmp = i;
                        for d in (dim + 1)..ndim { src_c[d] = i_tmp % a_shape.dims()[d]; i_tmp /= a_shape.dims()[d]; }
                        let mut dst_c = vec![0usize; ndim];
                        let mut o_tmp = o;
                        for d in 0..dim { dst_c[d] = o_tmp % g_dims[d]; o_tmp /= g_dims[d]; }
                        dst_c[dim] = s;
                        let mut i_tmp = i;
                        for d in (dim + 1)..ndim { dst_c[d] = i_tmp % g_dims[d]; i_tmp /= g_dims[d]; }
                        let g = grad.item_f64(&dst_c)?;
                        let prev = gi.item_f64(&src_c).unwrap_or(0.0);
                        gi.set_item_f64(&src_c, prev + g)?;
                    }
                }
            }
            Ok(vec![gi])
        }));
    }
    Ok(ADTensor { id: out_id, data: out_data, requires_grad })
}

pub fn ad_cat(tensors: &[ADTensor], axis: usize) -> NnResult<ADTensor> {
    if tensors.is_empty() {
        return Err(NnError::InvalidInput("Cannot concat empty list".to_string()));
    }
    let ndim = tensors[0].ndim();
    let mut out_dims = tensors[0].shape().dims().to_vec();
    for t in &tensors[1..] {
        out_dims[axis] += t.shape().dims()[axis];
    }
    let mut flat_data: Vec<f64> = Vec::new();
    for t in tensors {
        flat_data.extend(t.to_vec_f64()?);
    }
    let out_data = Tensor::from_vec_f64(&flat_data, Shape::new(out_dims.clone()));
    let requires_grad = tensors.iter().any(|t| t.requires_grad);
    let out_id = TensorId::new();
    if requires_grad && isRecording() {
        let ids: Vec<TensorId> = tensors.iter().map(|t| t.id).collect();
        let split_sizes: Vec<usize> = tensors.iter().map(|t| t.shape().dims()[axis]).collect();
        let base_shape: Vec<usize> = tensors[0].shape().dims().iter().enumerate()
            .map(|(i, &d)| if i == axis { 0 } else { d }).collect();
        record_to_tape(out_id, ids, vec![], Box::new(move |grad: &Tensor| {
            let mut grads = Vec::new();
            let mut offset = 0;
            for &size in &split_sizes {
                let mut sd = base_shape.clone();
                sd[axis] = size;
                let mut slice_data = Vec::new();
                for i in 0..sd.iter().product::<usize>() {
                    let c = iter_coords(ndim, &sd, i);
                    let mut fc = c.clone();
                    fc[axis] += offset;
                    slice_data.push(grad.item_f64(&fc)?);
                }
                grads.push(Tensor::from_vec_f64(&slice_data, Shape::new(sd)));
                offset += size;
            }
            Ok(grads)
        }));
    }
    Ok(ADTensor { id: out_id, data: out_data, requires_grad })
}

pub fn backward(loss: &ADTensor, targets: &[&ADTensor], tape: &GradTape) -> NnResult<HashMap<TensorId, Tensor>> {
    let mut gradients: HashMap<TensorId, Tensor> = HashMap::new();
    let loss_grad = Tensor::full(loss.shape().clone(), 1.0, loss.dtype());
    gradients.insert(loss.id, loss_grad);
    for entry in tape.entries.iter().rev() {
        let grad_output = match gradients.get(&entry.output_id) {
            Some(g) => g.clone(),
            None => continue,
        };
        let input_grads = (entry.backward_fn)(&grad_output)?;
        for (i, grad) in input_grads.into_iter().enumerate() {
            if i < entry.input_ids.len() {
                let input_id = entry.input_ids[i];
                if let Some(existing) = gradients.get(&input_id) {
                    let accumulated = existing.add(&grad)?;
                    gradients.insert(input_id, accumulated);
                } else {
                    gradients.insert(input_id, grad);
                }
            }
        }
    }
    let mut result = HashMap::new();
    for target in targets {
        if let Some(grad) = gradients.get(&target.id) {
            result.insert(target.id, grad.clone());
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ad_basic_add() {
        startRecording();
        let a = ADTensor::from_vec_f32(&[1.0, 2.0, 3.0], Shape::from_1d(3), true);
        let b = ADTensor::from_vec_f32(&[4.0, 5.0, 6.0], Shape::from_1d(3), true);
        let c = ad_add(&a, &b).unwrap();
        assert_eq!(c.item().unwrap(), 5.0);
        assert_eq!(c.data().item_f64(&[1]).unwrap(), 7.0);
        let tape = stopRecording().unwrap();
        assert!(!tape.is_empty());
    }

    #[test]
    fn ad_mul_backward() {
        startRecording();
        let a = ADTensor::from_vec_f32(&[2.0, 3.0], Shape::from_1d(2), true);
        let b = ADTensor::from_vec_f32(&[4.0, 5.0], Shape::from_1d(2), true);
        let c = ad_mul(&a, &b).unwrap();
        let tape = stopRecording().unwrap();
        let grads = backward(&c, &[&a, &b], &tape).unwrap();
        assert!(grads.contains_key(&a.id()));
        assert!(grads.contains_key(&b.id()));
        let ga = grads.get(&a.id()).unwrap();
        assert_eq!(ga.item_f64(&[0]).unwrap(), 4.0);
        assert_eq!(ga.item_f64(&[1]).unwrap(), 5.0);
    }

    #[test]
    fn ad_relu_backward() {
        startRecording();
        let x = ADTensor::from_vec_f32(&[-1.0, 0.5, 2.0], Shape::from_1d(3), true);
        let y = ad_relu(&x).unwrap();
        let tape = stopRecording().unwrap();
        let grads = backward(&y, &[&x], &tape).unwrap();
        let g = grads.get(&x.id()).unwrap();
        assert_eq!(g.item_f64(&[0]).unwrap(), 0.0);
        assert_eq!(g.item_f64(&[1]).unwrap(), 1.0);
        assert_eq!(g.item_f64(&[2]).unwrap(), 1.0);
    }

    #[test]
    fn ad_matmul_backward() {
        startRecording();
        let a = ADTensor::from_vec_f32(&[1.0, 2.0, 3.0, 4.0], Shape::from_2d(2, 2), true);
        let b = ADTensor::from_vec_f32(&[5.0, 6.0, 7.0, 8.0], Shape::from_2d(2, 2), true);
        let c = ad_matmul(&a, &b).unwrap();
        let tape = stopRecording().unwrap();
        let grads = backward(&c, &[&a, &b], &tape).unwrap();
        assert!(grads.contains_key(&a.id()));
        assert!(grads.contains_key(&b.id()));
    }

    #[test]
    fn ad_softmax_backward() {
        startRecording();
        let x = ADTensor::from_vec_f32(&[1.0, 2.0, 3.0], Shape::from_1d(3), true);
        let y = ad_softmax(&x, 0).unwrap();
        let tape = stopRecording().unwrap();
        let grads = backward(&y, &[&x], &tape).unwrap();
        assert!(grads.contains_key(&x.id()));
    }

    #[test]
    fn ad_chained_ops() {
        startRecording();
        let x = ADTensor::from_vec_f32(&[1.0, 2.0, 3.0], Shape::from_1d(3), true);
        let y = ADTensor::from_vec_f32(&[4.0, 5.0, 6.0], Shape::from_1d(3), true);
        let z = ad_add(&x, &y).unwrap();
        let w = ad_mul(&z, &x).unwrap();
        let loss = ad_sum_all(&w)?;
        let tape = stopRecording().unwrap();
        let grads = backward(&loss, &[&x, &y], &tape).unwrap();
        assert!(grads.contains_key(&x.id()));
        assert!(grads.contains_key(&y.id()));
    }
}
