//! # Neo Neural Engine
//!
//! GPU-accelerated neural computation engine for the Neo AGI Operating System.
//!
//! Provides tensor operations, device management, computation graph execution,
//! automatic differentiation, GPU memory pooling, and performance profiling.
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────┐
//! │                    Execution Engine                       │
//! │  (graph execution, scheduling, cancellation)             │
//! ├──────────────────────────────────────────────────────────┤
//! │  Autodiff     │  Operation Registry  │  Graph Optimizer  │
//! │  (reverse     │  (matmul, add,       │  (dead code       │
//! │   mode AD)    │   activations)       │   elimination)    │
//! ├──────────────────────────────────────────────────────────┤
//! │                    Tensor System                          │
//! │  (dense, sparse, views, slicing, broadcasting)           │
//! ├──────────────────────────────────────────────────────────┤
//! │  Memory Manager  │  Device Abstraction  │  Profiler      │
//! │  (pools, arena,  │  (CPU, CUDA, Metal,  │  (FLOPS,       │
//! │   ref counting)  │   ROCm, Vulkan)      │   timing)      │
//! └──────────────────────────────────────────────────────────┘
//! ```

pub mod error;
pub mod dtype;
pub mod shape;
pub mod memory;
pub mod device;
pub mod backend;
pub mod tensor;
pub mod sparse;
pub mod ops;
pub mod graph;
pub mod execution;
pub mod autodiff;
pub mod serialize;
pub mod profiler;

// Re-exports for convenience
pub use error::{NeuralError, NeuralResult};
pub use dtype::DType;
pub use shape::{Shape, Strides};
pub use memory::{MemoryManager, MemoryPool, ArenaAllocator};
pub use device::{Device, DeviceType, DeviceManager, Backend};
pub use backend::CpuBackend;
pub use tensor::Tensor;
pub use sparse::{CooTensor, CsrTensor, SparseTensor};
pub use ops::{
    OpId, OpType, OpParams, OpCompute, OperationRegistry,
    create_default_registry,
};
pub use graph::{ComputationGraph, NodeId, GraphNode, NodeKind};
pub use execution::{
    ExecutionEngine, ExecResult, ExecStatus, ExecStats, CancellationToken,
};
pub use autodiff::{AutodiffEngine, GradientAccumulator, DetachedTensor};
pub use serialize::{
    serialize_tensor, deserialize_tensor,
    serialize_graph, deserialize_graph,
    TensorHeader, GraphHeader, VersionInfo,
};
pub use profiler::{Profiler, ProfileEvent, ProfileSummary, OpStats};

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;

    #[test]
    fn full_pipeline_matmul() {
        let registry = create_default_registry();
        let device = Arc::new(Device::cpu());
        let engine = ExecutionEngine::new(registry, device);

        let mut graph = ComputationGraph::new("matmul_test");
        let x = graph.add_input("x", vec![2, 3]);
        let y = graph.add_input("y", vec![3, 2]);
        let z = graph.add_op(OpType::MatMul, vec![x, y], OpParams::new());
        graph.set_outputs(vec![z]);

        let mut inputs = HashMap::new();
        inputs.insert(
            x,
            Tensor::from_vec_f32(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], Shape::from_2d(2, 3)),
        );
        inputs.insert(
            y,
            Tensor::from_vec_f32(&[7.0, 8.0, 9.0, 10.0, 11.0, 12.0], Shape::from_3d(3, 2)),
        );

        let result = engine.execute(&graph, &inputs, None).unwrap();
        assert_eq!(result.status, ExecStatus::Completed);

        let output = result.outputs.get(&z).unwrap();
        // [1*7+2*9+3*11, 1*8+2*10+3*12, 4*7+5*9+6*11, 4*8+5*10+6*12]
        // = [58, 64, 139, 154]
        assert_eq!(output.item_f64(&[0, 0]).unwrap(), 58.0);
        assert_eq!(output.item_f64(&[0, 1]).unwrap(), 64.0);
        assert_eq!(output.item_f64(&[1, 0]).unwrap(), 139.0);
        assert_eq!(output.item_f64(&[1, 1]).unwrap(), 154.0);
    }

    #[test]
    fn full_pipeline_activations() {
        let registry = create_default_registry();
        let device = Arc::new(Device::cpu());
        let engine = ExecutionEngine::new(registry, device);

        let mut graph = ComputationGraph::new("activation_test");
        let x = graph.add_input("x", vec![5]);
        let r = graph.add_op(OpType::Relu, vec![x], OpParams::new());
        let s = graph.add_op(OpType::Sigmoid, vec![x], OpParams::new());
        let g = graph.add_op(OpType::Gelu, vec![x], OpParams::new());
        graph.set_outputs(vec![r, s, g]);

        let mut inputs = HashMap::new();
        inputs.insert(
            x,
            Tensor::from_vec_f32(&[-2.0, -1.0, 0.0, 1.0, 2.0], Shape::from_1d(5)),
        );

        let result = engine.execute(&graph, &inputs, None).unwrap();
        assert_eq!(result.status, ExecStatus::Completed);
        assert_eq!(result.nodes_executed, 3);

        let relu_out = result.outputs.get(&r).unwrap();
        assert_eq!(relu_out.item_f64(&[0]).unwrap(), 0.0);
        assert_eq!(relu_out.item_f64(&[3]).unwrap(), 1.0);

        let sig_out = result.outputs.get(&s).unwrap();
        let sum: f64 = (0..5)
            .map(|i| sig_out.item_f64(&[i]).unwrap())
            .sum();
        assert!(sum > 2.0 && sum < 3.0);
    }

    #[test]
    fn tensor_serialization_roundtrip() {
        let t = Tensor::from_vec_f32(&[1.0, 2.0, 3.0, 4.0], Shape::from_2d(2, 2));
        let bytes = serialize::serialize_tensor(&t, Some("test")).unwrap();
        let (t2, name) = serialize::deserialize_tensor(&bytes).unwrap();
        assert_eq!(name, Some("test".to_string()));
        assert_eq!(t2.shape().dims(), &[2, 2]);
        assert!((t2.item_f64(&[0, 1]).unwrap() - 2.0).abs() < 1e-6);
    }

    #[test]
    fn profiler_integration() {
        let profiler = Arc::new(Profiler::new());
        let t1 = Tensor::from_vec_f32(&[1.0, 2.0, 3.0, 4.0], Shape::from_2d(2, 2));
        let t2 = Tensor::from_vec_f32(&[5.0, 6.0, 7.0, 8.0], Shape::from_2d(2, 2));

        {
            let _scope = profiler.scope_with_flops("matmul", vec![t1.shape().clone(), t2.shape().clone()], profiler::matmul_flops(2, 2, 2));
            let _result = t1.matmul(&t2).unwrap();
        }

        let stats = profiler.summary();
        assert_eq!(stats.total_events, 1);
        assert!(stats.total_flops > 0);
    }

    #[test]
    fn sparse_tensor_pipeline() {
        let t = Tensor::from_vec_f32(
            &[1.0, 0.0, 0.0, 0.0, 5.0, 0.0, 0.0, 0.0, 9.0],
            Shape::from_2d(3, 3),
        );
        let coo = CooTensor::from_dense(&t).unwrap();
        assert_eq!(coo.nnz(), 3);
        assert!((coo.sparsity() - 6.0 / 9.0).abs() < 1e-6);

        let csr = CsrTensor::from_coo(&coo).unwrap();
        let dense = csr.to_dense().unwrap();
        assert_eq!(dense.item_f64(&[0, 0]).unwrap(), 1.0);
        assert_eq!(dense.item_f64(&[1, 1]).unwrap(), 5.0);
        assert_eq!(dense.item_f64(&[2, 2]).unwrap(), 9.0);
    }

    #[test]
    fn dtype_comprehensive() {
        assert_eq!(DType::Float32.byte_size(), 4);
        assert!(DType::Float32.is_float());
        assert!(!DType::Float32.is_integer());
        assert!(DType::Int64.is_integer());
        assert!(DType::Complex64.is_complex());
        assert!(DType::BFloat16.is_float());
        assert_eq!(DType::promote(DType::Float32, DType::Float64), DType::Float64);
    }

    #[test]
    fn shape_broadcasting() {
        let a = vec![8, 1, 6];
        let b = vec![7, 1, 5, 6];
        let result = shape::broadcast_shapes(&a, &b).unwrap();
        assert_eq!(result, vec![7, 8, 5, 6]);
    }

    #[test]
    fn memory_manager_tracking() {
        let mgr = MemoryManager::new();
        let block = mgr.alloc_host(1024).unwrap();
        assert_eq!(mgr.total_host_allocated(), 1024);
        drop(block);
        assert_eq!(mgr.peak_host_allocated(), 1024);
    }
}
