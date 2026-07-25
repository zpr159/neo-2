use neo_neural_engine::autodiff::AutodiffEngine;
use neo_neural_engine::backend::CpuBackend;
use neo_neural_engine::device::{Device, DeviceManager, DeviceType};
use neo_neural_engine::dtype::DType;
use neo_neural_engine::error::NeuralError;
use neo_neural_engine::execution::{CancellationToken, ExecStatus, ExecutionEngine};
use neo_neural_engine::graph::ComputationGraph;
use neo_neural_engine::memory::{ArenaAllocator, MemoryManager, MemoryPool};
use neo_neural_engine::ops::{
    create_default_registry, OpId, OpParams, OpType, OperationRegistry,
};
use neo_neural_engine::profiler::Profiler;
use neo_neural_engine::serialize::{deserialize_graph, deserialize_tensor, serialize_graph, serialize_tensor};
use neo_neural_engine::shape::{broadcast_shapes, compute_strides, Shape, Strides};
use neo_neural_engine::sparse::{CooTensor, CsrTensor};
use neo_neural_engine::tensor::{Tensor, TensorShape};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// DType tests
// ---------------------------------------------------------------------------

#[test]
fn dtype_byte_size() {
    assert_eq!(DType::Float32.byte_size(), 4);
    assert_eq!(DType::Float64.byte_size(), 8);
    assert_eq!(DType::Int32.byte_size(), 4);
    assert_eq!(DType::Bool.byte_size(), 1);
    assert_eq!(DType::Complex128.byte_size(), 16);
    assert_eq!(DType::Float16.byte_size(), 2);
}

#[test]
fn dtype_category_checks() {
    assert!(DType::Float32.is_float());
    assert!(DType::Float64.is_float());
    assert!(DType::Float16.is_float());
    assert!(!DType::Int32.is_float());

    assert!(DType::Int32.is_integer());
    assert!(DType::UInt8.is_integer());
    assert!(!DType::Float32.is_integer());

    assert!(DType::Complex64.is_complex());
    assert!(DType::Complex128.is_complex());
    assert!(!DType::Float32.is_complex());

    assert!(DType::Int32.is_signed());
    assert!(!DType::UInt32.is_signed());

    assert!(DType::Float32.is_numeric());
    assert!(DType::Int64.is_numeric());
}

#[test]
fn dtype_promotion() {
    assert_eq!(DType::promote_float(DType::Float32, DType::Float64), DType::Float64);
    assert_eq!(DType::promote_float(DType::Float16, DType::Float32), DType::Float32);
    assert_eq!(DType::promote(DType::Int32, DType::Float32), DType::Float32);
    assert_eq!(DType::promote(DType::Float64, DType::Float64), DType::Float64);
}

#[test]
fn dtype_name_roundtrip() {
    for name in &["float32", "float64", "int32", "int64", "bool", "uint8"] {
        let dt = DType::from_name(name).unwrap();
        assert_eq!(dt.name(), *name);
    }
    assert!(DType::from_name("nonexistent").is_none());
}

#[test]
fn dtype_default_fill() {
    assert_eq!(DType::Float32.default_fill_f64(), 0.0);
    assert_eq!(DType::Int32.default_fill_f64(), 0.0);
    assert_eq!(DType::Bool.default_fill_f64(), 0.0);
}

#[test]
fn dtype_zero_and_one_bytes() {
    let zero = DType::Float32.zero_bytes();
    let one = DType::Float32.one_bytes();
    assert_eq!(zero.len(), 4);
    assert_eq!(one.len(), 4);
    assert_ne!(zero, one);
}

// ---------------------------------------------------------------------------
// Shape tests
// ---------------------------------------------------------------------------

#[test]
fn shape_constructors() {
    let s = Shape::new(vec![2, 3, 4]);
    assert_eq!(s.ndim(), 3);
    assert_eq!(s.numel(), 24);
    assert_eq!(s.dims(), &[2, 3, 4]);

    let s1 = Shape::from_1d(5);
    assert_eq!(s1.dims(), &[5]);

    let s2 = Shape::from_2d(3, 4);
    assert_eq!(s2.dims(), &[3, 4]);
    assert_eq!(s2.numel(), 12);

    let s3 = Shape::from_3d(2, 3, 4);
    assert_eq!(s3.dims(), &[2, 3, 4]);

    let s4 = Shape::from_4d(1, 2, 3, 4);
    assert_eq!(s4.dims(), &[1, 2, 3, 4]);
}

#[test]
fn shape_scalar() {
    let s = Shape::scalar();
    assert!(s.is_scalar());
    assert_eq!(s.ndim(), 0);
    assert_eq!(s.numel(), 1);
}

#[test]
fn shape_dim_access() {
    let s = Shape::from_3d(2, 3, 4);
    assert_eq!(s.dim(0).unwrap(), 2);
    assert_eq!(s.dim(1).unwrap(), 3);
    assert_eq!(s.dim(2).unwrap(), 4);
    assert_eq!(s.first_dim().unwrap(), 2);
    assert_eq!(s.last_dim().unwrap(), 4);
    assert!(s.dim(3).is_err());
}

#[test]
fn shape_modify() {
    let s = Shape::from_3d(2, 3, 4);
    let s2 = s.remove_dim(1).unwrap();
    assert_eq!(s2.dims(), &[2, 4]);

    let s3 = s.insert_dim(1, 5).unwrap();
    assert_eq!(s3.dims(), &[2, 5, 3, 4]);
}

#[test]
fn shape_flatten_reshape() {
    let s = Shape::from_3d(2, 3, 4);
    assert_eq!(s.flatten().dims(), &[24]);

    let s2 = Shape::from_2d(6, 4);
    let reshaped = s2.reshape(vec![2, 3, 4]).unwrap();
    assert_eq!(reshaped.dims(), &[2, 3, 4]);

    let bad = s2.reshape(vec![5, 5]);
    assert!(bad.is_err());
}

#[test]
fn shape_slice() {
    let s = Shape::from_1d(10);
    assert_eq!(s.slice(2, 7).dims(), &[5]);
}

#[test]
fn shape_broadcasting() {
    let a = Shape::from_2d(3, 4);
    let b = Shape::from_1d(4);
    let result = broadcast_shapes(&a, &b).unwrap();
    assert_eq!(result.dims(), &[3, 4]);

    let c = Shape::from_3d(2, 1, 4);
    let d = Shape::from_2d(3, 4);
    let result = broadcast_shapes(&c, &d).unwrap();
    assert_eq!(result.dims(), &[2, 3, 4]);

    let e = Shape::from_1d(3);
    let f = Shape::from_1d(4);
    assert!(broadcast_shapes(&e, &f).is_err());
}

#[test]
fn shape_display() {
    let s = Shape::from_2d(3, 4);
    assert_eq!(format!("{s}"), "[3, 4]");
}

// ---------------------------------------------------------------------------
// Strides tests
// ---------------------------------------------------------------------------

#[test]
fn strides_contiguous() {
    let s = Shape::from_3d(2, 3, 4);
    let st = Strides::contiguous(&s);
    assert_eq!(st.ndim(), 3);
    assert_eq!(st.stride(0).unwrap(), 12);
    assert_eq!(st.stride(1).unwrap(), 4);
    assert_eq!(st.stride(2).unwrap(), 1);
}

#[test]
fn strides_byte_offset() {
    let s = Shape::from_2d(3, 4);
    let st = Strides::contiguous(&s);
    assert_eq!(st.byte_offset(&[1, 2]), 1 * 4 + 2);
}

// ---------------------------------------------------------------------------
// Memory tests
// ---------------------------------------------------------------------------

#[test]
fn memory_block_basic() {
    let mut block = neo_neural_engine::memory::MemoryBlock::new(1024, neo_neural_engine::memory::MemoryLocation::Host).unwrap();
    assert_eq!(block.size(), 1024);
    assert_eq!(block.ref_count(), 1);
    block.as_slice()[0..4].copy_from_slice(&42u32.to_le_bytes());
    assert_eq!(&block.as_slice()[0..4], &42u32.to_le_bytes());
}

#[test]
fn arena_allocator() {
    let mut arena = ArenaAllocator::new(4096);
    let off1 = arena.allocate(100).unwrap();
    let off2 = arena.allocate(200).unwrap();
    assert!(off2 > off1);
    assert_eq!(arena.total_allocated(), 300);
}

#[test]
fn memory_pool() {
    let mut pool = MemoryPool::new(256, 8);
    let b1 = pool.allocate().unwrap();
    let b2 = pool.allocate().unwrap();
    assert_ne!(b1, b2);
    assert_eq!(pool.allocated_count(), 2);
    pool.free(b1);
    assert_eq!(pool.free_count(), 1);
}

#[test]
fn memory_manager() {
    let mut mgr = MemoryManager::new();
    let _block = mgr.alloc_host(512).unwrap();
    assert_eq!(mgr.total_host_allocated(), 512);
}

// ---------------------------------------------------------------------------
// Device tests
// ---------------------------------------------------------------------------

#[test]
fn device_cpu() {
    let d = Device::cpu();
    assert_eq!(d.device_type(), DeviceType::Cpu);
    assert!(d.is_available());
    assert!(!d.is_gpu());
}

#[test]
fn device_type_name() {
    assert_eq!(DeviceType::Cpu.name(), "CPU");
    assert_eq!(DeviceType::Cuda.name(), "CUDA");
}

#[test]
fn device_manager() {
    let mgr = DeviceManager::new();
    let devices = mgr.list_devices();
    assert!(!devices.is_empty());
    assert!(devices[0].is_available());
}

#[test]
fn tensor_data_zeros() {
    let td = neo_neural_engine::device::TensorData::zeros(DType::Float32, Shape::from_2d(2, 3));
    assert_eq!(td.numel(), 6);
    assert_eq!(td.byte_size(), 24);
}

// ---------------------------------------------------------------------------
// Tensor tests
// ---------------------------------------------------------------------------

#[test]
fn tensor_creation() {
    let t = Tensor::zeros(Shape::from_2d(2, 3), DType::Float32);
    assert_eq!(t.shape().dims(), &[2, 3]);
    assert_eq!(t.dtype(), DType::Float32);
    assert_eq!(t.numel(), 6);
    assert_eq!(t.ndim(), 2);

    let t2 = Tensor::ones(Shape::from_1d(4), DType::Float64);
    for i in 0..4 {
        assert!((t2.item_f64(&[i]).unwrap() - 1.0).abs() < 1e-10);
    }

    let t3 = Tensor::full(Shape::from_1d(3), 7.0, DType::Float32);
    for i in 0..3 {
        assert!((t3.item_f64(&[i]).unwrap() - 7.0).abs() < 1e-6);
    }
}

#[test]
fn tensor_from_vec() {
    let t = Tensor::from_vec_f32(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], Shape::from_2d(2, 3));
    assert_eq!(t.item_f64(&[0, 0]).unwrap(), 1.0);
    assert_eq!(t.item_f64(&[0, 2]).unwrap(), 3.0);
    assert_eq!(t.item_f64(&[1, 2]).unwrap(), 6.0);

    let t2 = Tensor::from_vec_f64(&[10.0, 20.0], Shape::from_1d(2));
    assert_eq!(t2.item_f64(&[0]).unwrap(), 10.0);
    assert_eq!(t2.item_f64(&[1]).unwrap(), 20.0);

    let t3 = Tensor::from_vec_i64(&[1, 2, 3], Shape::from_1d(3));
    assert_eq!(t3.item_f64(&[0]).unwrap(), 1.0);
    assert_eq!(t3.item_f64(&[2]).unwrap(), 3.0);

    let t4 = Tensor::from_vec_bool(&[true, false, true], Shape::from_1d(3));
    assert_eq!(t4.item_f64(&[0]).unwrap(), 1.0);
    assert_eq!(t4.item_f64(&[1]).unwrap(), 0.0);
}

#[test]
fn tensor_range() {
    let t = Tensor::range(0.0, 5, DType::Float32);
    assert_eq!(t.numel(), 5);
    for i in 0..5 {
        assert!((t.item_f64(&[i]).unwrap() - i as f64).abs() < 1e-10);
    }
}

#[test]
fn tensor_reshape() {
    let t = Tensor::from_vec_f32(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], Shape::from_2d(2, 3));
    let r = t.reshape(Shape::from_2d(3, 2)).unwrap();
    assert_eq!(r.shape().dims(), &[3, 2]);
    assert_eq!(r.item_f64(&[0, 0]).unwrap(), 1.0);
    assert_eq!(r.item_f64(&[2, 1]).unwrap(), 6.0);
}

#[test]
fn tensor_transpose() {
    let t = Tensor::from_vec_f32(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], Shape::from_2d(2, 3));
    let tr = t.transpose(0, 1).unwrap();
    assert_eq!(tr.shape().dims(), &[3, 2]);
    assert_eq!(tr.item_f64(&[0, 0]).unwrap(), 1.0);
    assert_eq!(tr.item_f64(&[0, 1]).unwrap(), 4.0);
    assert_eq!(tr.item_f64(&[1, 0]).unwrap(), 2.0);
    assert_eq!(tr.item_f64(&[2, 1]).unwrap(), 6.0);
}

#[test]
fn tensor_t_shortcut() {
    let t = Tensor::from_vec_f32(&[1.0, 2.0, 3.0, 4.0], Shape::from_2d(2, 2));
    let tr = t.t().unwrap();
    assert_eq!(tr.shape().dims(), &[2, 2]);
    assert_eq!(tr.item_f64(&[0, 1]).unwrap(), 3.0);
    assert_eq!(tr.item_f64(&[1, 0]).unwrap(), 2.0);
}

#[test]
fn tensor_slice() {
    let t = Tensor::from_vec_f32(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], Shape::from_2d(2, 3));
    let s = t.slice(&[(0, 1), (1, 3)]).unwrap();
    assert_eq!(s.shape().dims(), &[1, 2]);
    assert_eq!(s.item_f64(&[0, 0]).unwrap(), 2.0);
    assert_eq!(s.item_f64(&[0, 1]).unwrap(), 3.0);
}

#[test]
fn tensor_unsqueeze_squeeze() {
    let t = Tensor::from_vec_f32(&[1.0, 2.0, 3.0], Shape::from_1d(3));
    let unsqueezed = t.unsqueeze(0).unwrap();
    assert_eq!(unsqueezed.shape().dims(), &[1, 3]);
    let unsqueezed2 = unsqueezed.unsqueeze(2).unwrap();
    assert_eq!(unsqueezed2.shape().dims(), &[1, 3, 1]);

    let squeezed = unsqueezed.squeeze(0).unwrap();
    assert_eq!(squeezed.shape().dims(), &[3]);
}

#[test]
fn tensor_flatten() {
    let t = Tensor::from_vec_f32(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], Shape::from_2d(2, 3));
    let f = t.flatten().unwrap();
    assert_eq!(f.shape().dims(), &[6]);
    assert_eq!(f.item_f64(&[0]).unwrap(), 1.0);
    assert_eq!(f.item_f64(&[5]).unwrap(), 6.0);
}

#[test]
fn tensor_contiguous() {
    let t = Tensor::from_vec_f32(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], Shape::from_2d(2, 3));
    let tr = t.transpose(0, 1).unwrap();
    assert!(!tr.is_contiguous());
    let c = tr.contiguous().unwrap();
    assert!(c.is_contiguous());
    assert_eq!(c.shape().dims(), &[3, 2]);
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
fn tensor_sub() {
    let a = Tensor::from_vec_f32(&[10.0, 20.0, 30.0], Shape::from_1d(3));
    let b = Tensor::from_vec_f32(&[1.0, 2.0, 3.0], Shape::from_1d(3));
    let c = a.sub(&b).unwrap();
    assert_eq!(c.item_f64(&[0]).unwrap(), 9.0);
    assert_eq!(c.item_f64(&[1]).unwrap(), 18.0);
    assert_eq!(c.item_f64(&[2]).unwrap(), 27.0);
}

#[test]
fn tensor_mul() {
    let a = Tensor::from_vec_f32(&[2.0, 3.0, 4.0], Shape::from_1d(3));
    let b = Tensor::from_vec_f32(&[5.0, 6.0, 7.0], Shape::from_1d(3));
    let c = a.mul(&b).unwrap();
    assert_eq!(c.item_f64(&[0]).unwrap(), 10.0);
    assert_eq!(c.item_f64(&[1]).unwrap(), 18.0);
    assert_eq!(c.item_f64(&[2]).unwrap(), 28.0);
}

#[test]
fn tensor_div() {
    let a = Tensor::from_vec_f32(&[10.0, 20.0, 30.0], Shape::from_1d(3));
    let b = Tensor::from_vec_f32(&[2.0, 4.0, 5.0], Shape::from_1d(3));
    let c = a.div(&b).unwrap();
    assert_eq!(c.item_f64(&[0]).unwrap(), 5.0);
    assert_eq!(c.item_f64(&[1]).unwrap(), 5.0);
    assert_eq!(c.item_f64(&[2]).unwrap(), 6.0);
}

#[test]
fn tensor_matmul() {
    let a = Tensor::from_vec_f32(&[1.0, 2.0, 3.0, 4.0], Shape::from_2d(2, 2));
    let b = Tensor::from_vec_f32(&[5.0, 6.0, 7.0, 8.0], Shape::from_2d(2, 2));
    let c = a.matmul(&b).unwrap();
    assert_eq!(c.shape().dims(), &[2, 2]);
    assert_eq!(c.item_f64(&[0, 0]).unwrap(), 19.0);
    assert_eq!(c.item_f64(&[0, 1]).unwrap(), 22.0);
    assert_eq!(c.item_f64(&[1, 0]).unwrap(), 43.0);
    assert_eq!(c.item_f64(&[1, 1]).unwrap(), 50.0);
}

#[test]
fn tensor_matmul_rectangular() {
    let a = Tensor::from_vec_f32(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], Shape::from_2d(2, 3));
    let b = Tensor::from_vec_f32(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], Shape::from_2d(3, 2));
    let c = a.matmul(&b).unwrap();
    assert_eq!(c.shape().dims(), &[2, 2]);
    assert_eq!(c.item_f64(&[0, 0]).unwrap(), 22.0);
    assert_eq!(c.item_f64(&[0, 1]).unwrap(), 28.0);
    assert_eq!(c.item_f64(&[1, 0]).unwrap(), 49.0);
    assert_eq!(c.item_f64(&[1, 1]).unwrap(), 64.0);
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
fn tensor_activations() {
    let t = Tensor::from_vec_f32(&[-2.0, -1.0, 0.0, 1.0, 2.0], Shape::from_1d(5));

    let relu = t.relu().unwrap();
    assert_eq!(relu.item_f64(&[0]).unwrap(), 0.0);
    assert_eq!(relu.item_f64(&[2]).unwrap(), 0.0);
    assert_eq!(relu.item_f64(&[3]).unwrap(), 1.0);
    assert_eq!(relu.item_f64(&[4]).unwrap(), 2.0);

    let sigmoid = t.sigmoid().unwrap();
    assert!(sigmoid.item_f64(&[2]).unwrap() > 0.49);
    assert!(sigmoid.item_f64(&[2]).unwrap() < 0.51);
    assert!(sigmoid.item_f64(&[4]).unwrap() > 0.88);

    let tanh = t.tanh().unwrap();
    assert!((tanh.item_f64(&[2]).unwrap()).abs() < 0.01);
    assert!(tanh.item_f64(&[4]).unwrap() > 0.96);
}

#[test]
fn tensor_gelu() {
    let t = Tensor::from_vec_f32(&[0.0], Shape::from_1d(1));
    let g = t.gelu().unwrap();
    assert!((g.item_f64(&[0]).unwrap()).abs() < 0.01);
}

#[test]
fn tensor_sum_axis() {
    let t = Tensor::from_vec_f32(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], Shape::from_2d(2, 3));
    let s = t.sum_axis(1).unwrap();
    assert_eq!(s.shape().dims(), &[2]);
    assert_eq!(s.item_f64(&[0]).unwrap(), 6.0);
    assert_eq!(s.item_f64(&[1]).unwrap(), 15.0);

    let s2 = t.sum_axis(0).unwrap();
    assert_eq!(s2.shape().dims(), &[3]);
    assert_eq!(s2.item_f64(&[0]).unwrap(), 5.0);
    assert_eq!(s2.item_f64(&[1]).unwrap(), 7.0);
    assert_eq!(s2.item_f64(&[2]).unwrap(), 9.0);
}

#[test]
fn tensor_mean_axis() {
    let t = Tensor::from_vec_f32(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], Shape::from_2d(2, 3));
    let m = t.mean_axis(1).unwrap();
    assert_eq!(m.shape().dims(), &[2]);
    assert!((m.item_f64(&[0]).unwrap() - 2.0).abs() < 1e-6);
    assert!((m.item_f64(&[1]).unwrap() - 5.0).abs() < 1e-6);
}

#[test]
fn tensor_max_axis() {
    let t = Tensor::from_vec_f32(&[3.0, 1.0, 2.0, 6.0, 4.0, 5.0], Shape::from_2d(2, 3));
    let m = t.max_axis(1).unwrap();
    assert_eq!(m.shape().dims(), &[2]);
    assert_eq!(m.item_f64(&[0]).unwrap(), 3.0);
    assert_eq!(m.item_f64(&[1]).unwrap(), 6.0);
}

#[test]
fn tensor_min_axis() {
    let t = Tensor::from_vec_f32(&[3.0, 1.0, 2.0, 6.0, 4.0, 5.0], Shape::from_2d(2, 3));
    let m = t.min_axis(1).unwrap();
    assert_eq!(m.shape().dims(), &[2]);
    assert_eq!(m.item_f64(&[0]).unwrap(), 1.0);
    assert_eq!(m.item_f64(&[1]).unwrap(), 4.0);
}

#[test]
fn tensor_softmax() {
    let t = Tensor::from_vec_f32(&[1.0, 2.0, 3.0], Shape::from_1d(3));
    let s = t.softmax(0).unwrap();
    let sum: f64 = (0..3).map(|i| s.item_f64(&[i]).unwrap()).sum();
    assert!((sum - 1.0).abs() < 1e-6);
    assert!(s.item_f64(&[2]).unwrap() > s.item_f64(&[0]).unwrap());
}

#[test]
fn tensor_to_dtype() {
    let t = Tensor::from_vec_f32(&[1.5, 2.5, 3.5], Shape::from_1d(3));
    let t64 = t.to_dtype(DType::Float64).unwrap();
    assert_eq!(t64.dtype(), DType::Float64);
    assert!((t64.item_f64(&[0]).unwrap() - 1.5).abs() < 1e-6);

    let ti32 = t.to_dtype(DType::Int32).unwrap();
    assert_eq!(ti32.dtype(), DType::Int32);
    assert_eq!(ti32.item_f64(&[0]).unwrap(), 1.0);
}

#[test]
fn tensor_detach() {
    let t = Tensor::from_vec_f32(&[1.0, 2.0], Shape::from_1d(2));
    let d = t.detach();
    assert_eq!(d.item_f64(&[0]).unwrap(), 1.0);
    assert_eq!(d.item_f64(&[1]).unwrap(), 2.0);
}

#[test]
fn tensor_to_vec() {
    let t = Tensor::from_vec_f32(&[1.0, 2.0, 3.0], Shape::from_1d(3));
    let v = t.to_vec_f64().unwrap();
    assert_eq!(v, vec![1.0, 2.0, 3.0]);
}

#[test]
fn tensor_display() {
    let t = Tensor::from_vec_f32(&[1.0, 2.0], Shape::from_1d(2));
    let s = format!("{t}");
    assert!(s.contains("Tensor"));
    assert!(s.contains("Float32"));
}

#[test]
fn tensor_clone_data() {
    let t = Tensor::from_vec_f32(&[1.0, 2.0, 3.0], Shape::from_1d(3));
    let c = t.clone_data();
    assert_eq!(c.item_f64(&[0]).unwrap(), 1.0);
}

// ---------------------------------------------------------------------------
// Broadcasting tests
// ---------------------------------------------------------------------------

#[test]
fn tensor_broadcasting_add() {
    let a = Tensor::from_vec_f32(&[1.0, 2.0, 3.0], Shape::from_1d(3));
    let b = Tensor::from_vec_f32(&[10.0, 20.0, 30.0], Shape::from_2d(2, 3));
    let c = a.add(&b).unwrap();
    assert_eq!(c.shape().dims(), &[2, 3]);
    assert_eq!(c.item_f64(&[0, 0]).unwrap(), 11.0);
    assert_eq!(c.item_f64(&[1, 2]).unwrap(), 33.0);
}

// ---------------------------------------------------------------------------
// Sparse tests
// ---------------------------------------------------------------------------

#[test]
fn coo_sparse_basic() {
    let t = Tensor::from_vec_f32(&[1.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 3.0], Shape::from_3d(1, 3, 3));
    let coo = CooTensor::from_dense(&t).unwrap();
    assert_eq!(coo.nnz(), 3);
    assert!(coo.sparsity() > 0.6);
    assert!(coo.density() < 0.4);
    let dense = coo.to_dense().unwrap();
    assert_eq!(dense.item_f64(&[0, 0, 0]).unwrap(), 1.0);
    assert_eq!(dense.item_f64(&[0, 1, 1]).unwrap(), 2.0);
    assert_eq!(dense.item_f64(&[0, 2, 2]).unwrap(), 3.0);
    assert_eq!(dense.item_f64(&[0, 0, 1]).unwrap(), 0.0);
}

#[test]
fn csr_sparse_basic() {
    let t = Tensor::from_vec_f32(&[1.0, 0.0, 3.0, 0.0, 5.0, 0.0], Shape::from_2d(2, 3));
    let csr = CsrTensor::from_dense(&t).unwrap();
    assert_eq!(csr.nnz(), 3);
    let dense = csr.to_dense().unwrap();
    assert_eq!(dense.item_f64(&[0, 0]).unwrap(), 1.0);
    assert_eq!(dense.item_f64(&[0, 2]).unwrap(), 3.0);
    assert_eq!(dense.item_f64(&[1, 1]).unwrap(), 5.0);
    assert_eq!(dense.item_f64(&[0, 1]).unwrap(), 0.0);
}

#[test]
fn sparse_coo_csr_roundtrip() {
    let t = Tensor::from_vec_f32(&[1.0, 0.0, 3.0, 0.0, 5.0, 6.0], Shape::from_2d(2, 3));
    let coo = CooTensor::from_dense(&t).unwrap();
    let csr = CsrTensor::from_coo(&coo).unwrap();
    let dense = csr.to_dense().unwrap();
    assert_eq!(dense.item_f64(&[0, 0]).unwrap(), 1.0);
    assert_eq!(dense.item_f64(&[0, 2]).unwrap(), 3.0);
    assert_eq!(dense.item_f64(&[1, 1]).unwrap(), 5.0);
    assert_eq!(dense.item_f64(&[1, 2]).unwrap(), 6.0);
}

// ---------------------------------------------------------------------------
// Op registry tests
// ---------------------------------------------------------------------------

#[test]
fn op_registry_basics() {
    let registry = create_default_registry();
    assert!(registry.count() > 0);
    assert!(registry.get("Add").is_some());
    assert!(registry.get("MatMul").is_some());
    assert!(registry.get("Nonexistent").is_none());
}

#[test]
fn op_type_properties() {
    assert_eq!(OpType::MatMul.name(), "MatMul");
    assert_eq!(OpType::from_name("Add"), Some(OpType::Add));
    assert_eq!(OpType::from_name("xyz"), None);
    assert_eq!(OpType::MatMul.num_inputs(), 2);
}

#[test]
fn op_params_builder() {
    let p = OpParams::new()
        .with_axis(0)
        .with_axes(vec![0, 1])
        .with_shape(vec![2, 3])
        .with_eps(1e-5);
    assert_eq!(p.axis, Some(0));
    assert_eq!(p.axes, Some(vec![0, 1]));
    assert_eq!(p.shape, Some(vec![2, 3]));
    assert_eq!(p.eps, Some(1e-5));
}

#[test]
fn op_matmul_execute() {
    let registry = create_default_registry();
    let op = registry.get("MatMul").unwrap();
    let a = Tensor::from_vec_f32(&[1.0, 2.0, 3.0, 4.0], Shape::from_2d(2, 2));
    let b = Tensor::from_vec_f32(&[5.0, 6.0, 7.0, 8.0], Shape::from_2d(2, 2));
    let result = op.compute.execute(&[&a, &b], &OpParams::new()).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].item_f64(&[0, 0]).unwrap(), 19.0);
}

#[test]
fn op_add_execute() {
    let registry = create_default_registry();
    let op = registry.get("Add").unwrap();
    let a = Tensor::from_vec_f32(&[1.0, 2.0], Shape::from_1d(2));
    let b = Tensor::from_vec_f32(&[3.0, 4.0], Shape::from_1d(2));
    let result = op.compute.execute(&[&a, &b], &OpParams::new()).unwrap();
    assert_eq!(result[0].item_f64(&[0]).unwrap(), 4.0);
    assert_eq!(result[0].item_f64(&[1]).unwrap(), 6.0);
}

#[test]
fn op_backward() {
    let registry = create_default_registry();
    let grad = Tensor::from_vec_f32(&[1.0, 1.0], Shape::from_1d(2));

    let add_op = registry.get("Add").unwrap();
    let a = Tensor::from_vec_f32(&[1.0, 2.0], Shape::from_1d(2));
    let b = Tensor::from_vec_f32(&[3.0, 4.0], Shape::from_1d(2));
    let grads = add_op.compute.backward(&grad, &[&a, &b], &OpParams::new()).unwrap();
    assert_eq!(grads.len(), 2);

    let mul_op = registry.get("Mul").unwrap();
    let grads = mul_op.compute.backward(&grad, &[&a, &b], &OpParams::new()).unwrap();
    assert_eq!(grads.len(), 2);
}

// ---------------------------------------------------------------------------
// Computation graph tests
// ---------------------------------------------------------------------------

#[test]
fn graph_basic() {
    let mut g = ComputationGraph::new("test");
    let x = g.add_input("x", vec![2, 3]);
    let y = g.add_input("y", vec![2, 3]);
    let z = g.add_op(OpType::Add, vec![x, y], OpParams::new());
    g.set_outputs(vec![z]);
    assert_eq!(g.num_nodes(), 3);
    assert_eq!(g.output_ids().len(), 1);
}

#[test]
fn graph_topological_sort() {
    let mut g = ComputationGraph::new("test");
    let x = g.add_input("x", vec![2, 2]);
    let y = g.add_input("y", vec![2, 2]);
    let z = g.add_op(OpType::Add, vec![x, y], OpParams::new());
    g.set_outputs(vec![z]);
    let order = g.topological_sort().unwrap();
    assert_eq!(order.len(), 3);
    let x_pos = order.iter().position(|&id| id == x).unwrap();
    let z_pos = order.iter().position(|&id| id == z).unwrap();
    assert!(x_pos < z_pos);
}

#[test]
fn graph_validation() {
    let registry = create_default_registry();
    let mut g = ComputationGraph::new("test");
    let x = g.add_input("x", vec![2, 2]);
    let y = g.add_input("y", vec![2, 2]);
    let z = g.add_op(OpType::Add, vec![x, y], OpParams::new());
    g.set_outputs(vec![z]);
    assert!(g.validate(&registry).is_ok());
}

#[test]
fn graph_shape_inference() {
    let registry = create_default_registry();
    let mut g = ComputationGraph::new("test");
    let x = g.add_input("x", vec![2, 3]);
    let y = g.add_input("y", vec![2, 3]);
    let z = g.add_op(OpType::Add, vec![x, y], OpParams::new());
    g.set_outputs(vec![z]);
    g.infer_shapes(&registry).unwrap();
    let z_node = g.node(z).unwrap();
    assert_eq!(z_node.output_shape, Some(vec![2, 3]));
}

#[test]
fn graph_dead_code_elimination() {
    let mut g = ComputationGraph::new("test");
    let x = g.add_input("x", vec![2]);
    let _y = g.add_input("y", vec![2]);
    let _z = g.add_op(OpType::Add, vec![x, _y], OpParams::new());
    g.set_outputs(vec![x]);
    let removed = g.optimize().unwrap();
    assert!(removed > 0);
    assert_eq!(g.num_nodes(), 1);
}

// ---------------------------------------------------------------------------
// Execution engine tests
// ---------------------------------------------------------------------------

#[test]
fn execution_basic() {
    let registry = create_default_registry();
    let device = Arc::new(Device::cpu());
    let engine = ExecutionEngine::new(registry, device);

    let mut graph = ComputationGraph::new("test");
    let x = graph.add_input("x", vec![2, 2]);
    let y = graph.add_input("y", vec![2, 2]);
    let z = graph.add_op(OpType::Add, vec![x, y], OpParams::new());
    graph.set_outputs(vec![z]);

    let mut inputs = std::collections::HashMap::new();
    inputs.insert(x, Tensor::from_vec_f32(&[1.0, 2.0, 3.0, 4.0], Shape::from_2d(2, 2)));
    inputs.insert(y, Tensor::from_vec_f32(&[5.0, 6.0, 7.0, 8.0], Shape::from_2d(2, 2)));

    let result = engine.execute(&graph, &inputs, None).unwrap();
    assert_eq!(result.status, ExecStatus::Completed);
    let out = result.outputs.get(&z).unwrap();
    assert_eq!(out.item_f64(&[0, 0]).unwrap(), 6.0);
    assert_eq!(out.item_f64(&[1, 1]).unwrap(), 12.0);
}

#[test]
fn execution_matmul_chain() {
    let registry = create_default_registry();
    let device = Arc::new(Device::cpu());
    let engine = ExecutionEngine::new(registry, device);

    let mut graph = ComputationGraph::new("matmul_chain");
    let x = graph.add_input("x", vec![2, 3]);
    let y = graph.add_input("y", vec![3, 4]);
    let z = graph.add_op(OpType::MatMul, vec![x, y], OpParams::new());
    graph.set_outputs(vec![z]);

    let mut inputs = std::collections::HashMap::new();
    inputs.insert(x, Tensor::from_vec_f32(&[1.0; 6], Shape::from_2d(2, 3)));
    inputs.insert(y, Tensor::from_vec_f32(&[1.0; 12], Shape::from_3d(1, 3, 4)));

    let result = engine.execute(&graph, &inputs, None).unwrap();
    assert_eq!(result.status, ExecStatus::Completed);
}

#[test]
fn execution_cancellation() {
    let ct = CancellationToken::new();
    assert!(!ct.is_cancelled());
    ct.cancel();
    assert!(ct.is_cancelled());
}

// ---------------------------------------------------------------------------
// Autodiff tests
// ---------------------------------------------------------------------------

#[test]
fn autodiff_basic() {
    let mut engine = AutodiffEngine::new();
    assert!(engine.is_enabled());
    engine.set_enabled(false);
    assert!(!engine.is_enabled());
    engine.set_enabled(true);
    assert!(engine.is_enabled());
}

#[test]
fn gradient_accumulator() {
    let mut acc = neo_neural_engine::autodiff::GradientAccumulator::new();
    let key = uuid::Uuid::new_v4();
    let grad = Tensor::from_vec_f32(&[1.0, 2.0], Shape::from_1d(2));
    acc.accumulate(key, grad).unwrap();
    assert_eq!(acc.len(), 1);
    acc.reset();
    assert!(acc.is_empty());
}

#[test]
fn detached_tensor() {
    let t = Tensor::from_vec_f32(&[1.0, 2.0, 3.0], Shape::from_1d(3));
    let d = neo_neural_engine::autodiff::DetachedTensor::new(t.clone());
    assert_eq!(d.tensor().item_f64(&[0]).unwrap(), 1.0);
    let owned = d.into_tensor();
    assert_eq!(owned.item_f64(&[2]).unwrap(), 3.0);
}

#[test]
fn stop_gradient() {
    let t = Tensor::from_vec_f32(&[1.0, 2.0], Shape::from_1d(2));
    let sg = neo_neural_engine::autodiff::stop_gradient(&t);
    assert_eq!(sg.item_f64(&[0]).unwrap(), 1.0);
}

// ---------------------------------------------------------------------------
// Serialization tests
// ---------------------------------------------------------------------------

#[test]
fn tensor_serialization_roundtrip() {
    let t = Tensor::from_vec_f32(&[1.0, 2.0, 3.0, 4.0], Shape::from_2d(2, 2));
    let bytes = serialize_tensor(&t, Some("my_tensor")).unwrap();
    let (t2, name) = deserialize_tensor(&bytes).unwrap();
    assert_eq!(name, Some("my_tensor".to_string()));
    assert_eq!(t2.shape().dims(), &[2, 2]);
    assert!((t2.item_f64(&[0, 0]).unwrap() - 1.0).abs() < 1e-6);
    assert!((t2.item_f64(&[1, 1]).unwrap() - 4.0).abs() < 1e-6);
}

#[test]
fn graph_serialization_roundtrip() {
    let mut graph = ComputationGraph::new("test_graph");
    let x = graph.add_input("x", vec![2, 3]);
    let y = graph.add_input("y", vec![2, 3]);
    let z = graph.add_op(OpType::Add, vec![x, y], OpParams::new());
    graph.set_outputs(vec![z]);

    let bytes = serialize_graph(&graph).unwrap();
    let graph2 = deserialize_graph(&bytes).unwrap();
    assert_eq!(graph2.name(), "test_graph");
    assert_eq!(graph2.num_nodes(), 3);
}

#[test]
fn serialization_invalid_magic() {
    let data = b"XXXX";
    assert!(deserialize_tensor(data).is_err());
}

// ---------------------------------------------------------------------------
// Profiler tests
// ---------------------------------------------------------------------------

#[test]
fn profiler_basic() {
    let profiler = Arc::new(Profiler::new());
    assert!(profiler.is_enabled());

    profiler.set_enabled(false);
    assert!(!profiler.is_enabled());
    profiler.set_enabled(true);

    let summary = profiler.summary();
    assert_eq!(summary.total_events, 0);

    profiler.clear();
    assert_eq!(profiler.total_flops(), 0);
    assert_eq!(profiler.total_memory(), 0);
}

#[test]
fn profiler_scope() {
    let profiler = Arc::new(Profiler::new());
    let scope = profiler.scope("test_op", vec![Shape::from_2d(2, 3)]);
    drop(scope);

    let summary = profiler.summary();
    assert_eq!(summary.total_events, 1);
    assert!(summary.total_duration_us > 0);
}

#[test]
fn profiler_flops_calculation() {
    let flops = neo_neural_engine::profiler::matmul_flops(2, 3, 4);
    assert_eq!(flops, 2 * 3 * 4 * 2);

    let flops = neo_neural_engine::profiler::elementwise_flops(100);
    assert_eq!(flops, 100);

    let flops = neo_neural_engine::profiler::reduce_flops(100, 10);
    assert_eq!(flops, 1000);
}

// ---------------------------------------------------------------------------
// Error tests
// ---------------------------------------------------------------------------

#[test]
fn error_display() {
    let err = NeuralError::ShapeMismatch {
        expected: vec![2, 3],
        actual: vec![2, 4],
        context: "test".to_string(),
    };
    let s = format!("{err}");
    assert!(s.contains("ShapeMismatch") || s.contains("shape") || s.contains("Shape"));
}

#[test]
fn error_code() {
    let err = NeuralError::DtypeMismatch {
        expected: "float32",
        actual: "float64",
        context: "test".to_string(),
    };
    let code = err.code();
    assert_eq!(code as u16, 2001);
}
