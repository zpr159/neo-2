use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::RwLock;

use crate::device::Device;
use crate::error::{NeuralError, NeuralResult};
use crate::graph::{ComputationGraph, NodeId, NodeKind};
use crate::ops::OperationRegistry;
use crate::tensor::Tensor;

/// Execution status of a graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for ExecStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Running => write!(f, "running"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Progress callback for execution.
pub type ProgressCallback = Box<dyn Fn(usize, usize) + Send + Sync>;

/// Result of a graph execution.
#[derive(Debug)]
pub struct ExecResult {
    pub outputs: HashMap<NodeId, Tensor>,
    pub status: ExecStatus,
    pub duration: Duration,
    pub nodes_executed: usize,
}

/// Statistics about execution.
#[derive(Debug, Clone, Default)]
pub struct ExecStats {
    pub total_executions: u64,
    pub total_duration: Duration,
    pub avg_duration: Duration,
    pub max_duration: Duration,
    pub min_duration: Duration,
    pub total_nodes_executed: usize,
    pub errors: u64,
}

impl ExecStats {
    pub fn update(&mut self, duration: Duration, nodes: usize, success: bool) {
        self.total_executions += 1;
        self.total_duration += duration;
        self.total_nodes_executed += nodes;
        if self.total_executions > 0 {
            self.avg_duration =
                self.total_duration / self.total_executions as u32;
        }
        if duration > self.max_duration || self.total_executions == 1 {
            self.max_duration = duration;
        }
        if duration < self.min_duration || self.total_executions == 1 {
            self.min_duration = duration;
        }
        if !success {
            self.errors += 1;
        }
    }
}

/// Cancellation token for execution.
#[derive(Debug)]
pub struct CancellationToken {
    cancelled: AtomicBool,
}

impl CancellationToken {
    #[must_use]
    pub fn new() -> Self {
        Self {
            cancelled: AtomicBool::new(false),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

/// The execution engine for computation graphs.
pub struct ExecutionEngine {
    registry: Arc<OperationRegistry>,
    device: Arc<Device>,
    stats: RwLock<ExecStats>,
    execution_count: AtomicUsize,
}

impl ExecutionEngine {
    /// Creates a new execution engine with the given registry and device.
    #[must_use]
    pub fn new(registry: Arc<OperationRegistry>, device: Arc<Device>) -> Self {
        Self {
            registry,
            device,
            stats: RwLock::new(ExecStats::default()),
            execution_count: AtomicUsize::new(0),
        }
    }

    /// Executes a computation graph with the given inputs.
    pub fn execute(
        &self,
        graph: &ComputationGraph,
        inputs: &HashMap<NodeId, Tensor>,
        cancel: Option<&CancellationToken>,
    ) -> NeuralResult<ExecResult> {
        let start = Instant::now();
        let order = graph.topological_sort()?;
        let total_nodes = order.len();
        let mut node_values: HashMap<NodeId, Tensor> = HashMap::new();
        let mut nodes_executed = 0;

        // Load input values
        for (&node_id, tensor) in inputs {
            node_values.insert(node_id, tensor.clone());
        }

        for (step, &node_id) in order.iter().enumerate() {
            // Check cancellation
            if let Some(ct) = cancel {
                if ct.is_cancelled() {
                    return Ok(ExecResult {
                        outputs: node_values,
                        status: ExecStatus::Cancelled,
                        duration: start.elapsed(),
                        nodes_executed,
                    });
                }
            }

            let node = graph.node(node_id).ok_or_else(|| NeuralError::ExecutionFailed {
                message: format!("node {} not found", node_id),
            })?;

            match &node.kind {
                NodeKind::Input { .. } | NodeKind::Constant { .. } => {
                    // Inputs and constants are already loaded
                }
                NodeKind::Op { op_type, params } => {
                    let op_name = op_type.name();
                    let reg = self
                        .registry
                        .get(op_name)
                        .ok_or_else(|| NeuralError::OpNotRegistered {
                            op_name: op_name.to_string(),
                        })?;

                    let input_tensors: Vec<Tensor> = node
                        .input_ids
                        .iter()
                        .filter_map(|id| node_values.get(id).cloned())
                        .collect();

                    let input_refs: Vec<&Tensor> = input_tensors.iter().collect();
                    let results = reg.compute.execute(&input_refs, params)?;

                    if let Some(result) = results.into_iter().next() {
                        node_values.insert(node_id, result);
                    }
                    nodes_executed += 1;
                }
            }

            let _ = step;
        }

        let duration = start.elapsed();
        self.stats
            .write()
            .update(duration, nodes_executed, true);
        self.execution_count.fetch_add(1, Ordering::Relaxed);

        let outputs: HashMap<NodeId, Tensor> = graph
            .output_ids()
            .iter()
            .filter_map(|&id| node_values.get(&id).map(|t| (id, t.clone())))
            .collect();

        Ok(ExecResult {
            outputs,
            status: ExecStatus::Completed,
            duration,
            nodes_executed,
        })
    }

    /// Executes a graph and returns the first output tensor.
    pub fn execute_simple(
        &self,
        graph: &ComputationGraph,
        inputs: &HashMap<NodeId, Tensor>,
    ) -> NeuralResult<Tensor> {
        let result = self.execute(graph, inputs, None)?;
        result
            .outputs
            .into_iter()
            .next()
            .map(|(_, t)| t)
            .ok_or_else(|| NeuralError::ExecutionFailed {
                message: "no output produced".to_string(),
            })
    }

    /// Returns execution statistics.
    #[must_use]
    pub fn stats(&self) -> ExecStats {
        self.stats.read().clone()
    }

    /// Returns the number of executions.
    #[must_use]
    pub fn execution_count(&self) -> usize {
        self.execution_count.load(Ordering::Relaxed)
    }

    /// Returns the device.
    #[must_use]
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Returns the registry.
    #[must_use]
    pub fn registry(&self) -> &OperationRegistry {
        &self.registry
    }
}

impl std::fmt::Debug for ExecutionEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecutionEngine")
            .field("device", &self.device)
            .field("execution_count", &self.execution_count())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::OpType;
    use crate::ops::OpParams;

    #[test]
    fn engine_basic_execution() {
        let registry = crate::ops::create_default_registry();
        let device = Arc::new(Device::cpu());
        let engine = ExecutionEngine::new(registry, device);

        let mut graph = ComputationGraph::new("test");
        let x = graph.add_input("x", vec![2, 2]);
        let y = graph.add_input("y", vec![2, 2]);
        let z = graph.add_op(OpType::Add, vec![x, y], OpParams::new());
        graph.set_outputs(vec![z]);

        let mut inputs = HashMap::new();
        inputs.insert(x, Tensor::from_vec_f32(&[1.0, 2.0, 3.0, 4.0], crate::shape::Shape::from_2d(2, 2)));
        inputs.insert(y, Tensor::from_vec_f32(&[5.0, 6.0, 7.0, 8.0], crate::shape::Shape::from_2d(2, 2)));

        let result = engine.execute(&graph, &inputs, None).unwrap();
        assert_eq!(result.status, ExecStatus::Completed);
        let out = result.outputs.get(&z).unwrap();
        assert_eq!(out.item_f64(&[0, 0]).unwrap(), 6.0);
        assert_eq!(out.item_f64(&[1, 1]).unwrap(), 12.0);
    }

    #[test]
    fn engine_cancellation() {
        let registry = crate::ops::create_default_registry();
        let device = Arc::new(Device::cpu());
        let engine = ExecutionEngine::new(registry, device);

        let mut graph = ComputationGraph::new("test");
        let x = graph.add_input("x", vec![2, 2]);
        let y = graph.add_input("y", vec![2, 2]);
        let _z = graph.add_op(OpType::Add, vec![x, y], OpParams::new());
        graph.set_outputs(vec![_z]);

        let ct = CancellationToken::new();
        ct.cancel();

        let mut inputs = HashMap::new();
        inputs.insert(x, Tensor::from_vec_f32(&[1.0; 4], crate::shape::Shape::from_2d(2, 2)));
        inputs.insert(y, Tensor::from_vec_f32(&[1.0; 4], crate::shape::Shape::from_2d(2, 2)));

        let result = engine.execute(&graph, &inputs, Some(&ct)).unwrap();
        assert_eq!(result.status, ExecStatus::Cancelled);
    }

    #[test]
    fn engine_stats() {
        let registry = crate::ops::create_default_registry();
        let device = Arc::new(Device::cpu());
        let engine = ExecutionEngine::new(registry, device);

        let mut graph = ComputationGraph::new("test");
        let x = graph.add_input("x", vec![1]);
        graph.set_outputs(vec![x]);

        let mut inputs = HashMap::new();
        inputs.insert(x, Tensor::from_vec_f32(&[1.0], crate::shape::Shape::from_1d(1)));

        let _ = engine.execute(&graph, &inputs, None);
        let stats = engine.stats();
        assert_eq!(stats.total_executions, 1);
    }
}
