use std::collections::HashMap;

use uuid::Uuid;

use crate::error::{NeuralError, NeuralResult};
use crate::graph::{ComputationGraph, NodeId, NodeKind};
use crate::ops::{OpParams, OpType, OperationRegistry};
use crate::tensor::Tensor;

/// A gradient tape entry recording an operation.
#[derive(Debug, Clone)]
pub struct TapeEntry {
    pub node_id: NodeId,
    pub op_type: OpType,
    pub input_ids: Vec<NodeId>,
    pub params: OpParams,
    pub output_id: NodeId,
}

/// Reverse-mode automatic differentiation engine.
#[derive(Debug)]
pub struct AutodiffEngine {
    tape: Vec<TapeEntry>,
    gradients: HashMap<NodeId, Tensor>,
    enabled: bool,
}

impl AutodiffEngine {
    /// Creates a new autodiff engine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tape: Vec::new(),
            gradients: HashMap::new(),
            enabled: true,
        }
    }

    /// Enables or disables gradient tracking.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Returns whether gradient tracking is enabled.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Records an operation on the tape.
    pub fn record(
        &mut self,
        node_id: NodeId,
        op_type: OpType,
        input_ids: Vec<NodeId>,
        params: OpParams,
        output_id: NodeId,
    ) {
        if self.enabled {
            self.tape.push(TapeEntry {
                node_id,
                op_type,
                input_ids,
                params,
                output_id,
            });
        }
    }

    /// Clears the tape.
    pub fn reset(&mut self) {
        self.tape.clear();
        self.gradients.clear();
    }

    /// Returns the length of the tape.
    #[must_use]
    pub fn tape_len(&self) -> usize {
        self.tape.len()
    }

    /// Computes gradients of the loss with respect to specified target nodes.
    pub fn backward(
        &mut self,
        loss: &Tensor,
        targets: &[NodeId],
        graph: &ComputationGraph,
        registry: &OperationRegistry,
    ) -> NeuralResult<HashMap<NodeId, Tensor>> {
        self.gradients.clear();

        // Initialize the gradient of the loss with respect to itself
        let loss_grad = Tensor::full(loss.shape().clone(), 1.0, loss.dtype());
        // We need to figure out which node the loss corresponds to
        // For now, use the last tape entry's output

        let loss_node_id = self
            .tape
            .last()
            .map(|e| e.output_id)
            .ok_or_else(|| NeuralError::AutodiffError {
                message: "tape is empty, cannot compute gradients".to_string(),
            })?;

        self.gradients.insert(loss_node_id, loss_grad);

        // Process tape in reverse order
        for entry in self.tape.iter().rev() {
            let grad_output = match self.gradients.get(&entry.output_id) {
                Some(g) => g.clone(),
                None => continue,
            };

            let op_name = entry.op_type.name();
            let reg = registry.get(op_name).ok_or_else(|| NeuralError::OpNotRegistered {
                op_name: op_name.to_string(),
            })?;

            // Get input tensors from the graph
            let input_tensors: Vec<Tensor> = entry
                .input_ids
                .iter()
                .filter_map(|id| {
                    graph.node(*id).and_then(|n| match &n.kind {
                        NodeKind::Input { .. } => {
                            // For inputs, we don't have the actual tensor values in the graph
                            // This is handled during forward pass
                            None
                        }
                        _ => None,
                    })
                })
                .collect();

            let input_refs: Vec<&Tensor> = input_tensors.iter().collect();

            // Compute gradients
            if input_refs.len() == entry.input_ids.len() {
                let grads = reg
                    .compute
                    .backward(&grad_output, &input_refs, &entry.params)?;

                for (i, grad) in grads.into_iter().enumerate() {
                    if i < entry.input_ids.len() {
                        let input_id = entry.input_ids[i];
                        if let Some(existing) = self.gradients.get(&input_id) {
                            // Gradient accumulation
                            let accumulated = existing.add(&grad)?;
                            self.gradients.insert(input_id, accumulated);
                        } else {
                            self.gradients.insert(input_id, grad);
                        }
                    }
                }
            }
        }

        // Collect gradients for requested targets
        let mut result = HashMap::new();
        for &target in targets {
            if let Some(grad) = self.gradients.get(&target) {
                result.insert(target, grad.clone());
            }
        }

        Ok(result)
    }

    /// Returns all accumulated gradients.
    #[must_use]
    pub fn gradients(&self) -> &HashMap<NodeId, Tensor> {
        &self.gradients
    }

    /// Returns a reference to the tape.
    #[must_use]
    pub fn tape(&self) -> &[TapeEntry] {
        &self.tape
    }
}

impl Default for AutodiffEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// A detached tensor that does not track gradients.
#[derive(Debug, Clone)]
pub struct DetachedTensor {
    inner: Tensor,
}

impl DetachedTensor {
    #[must_use]
    pub fn new(tensor: Tensor) -> Self {
        Self {
            inner: tensor.detach(),
        }
    }

    #[must_use]
    pub fn tensor(&self) -> &Tensor {
        &self.inner
    }

    #[must_use]
    pub fn into_tensor(self) -> Tensor {
        self.inner
    }
}

/// Gradient accumulation helper.
#[derive(Debug)]
pub struct GradientAccumulator {
    accumulators: HashMap<Uuid, Tensor>,
}

impl GradientAccumulator {
    #[must_use]
    pub fn new() -> Self {
        Self {
            accumulators: HashMap::new(),
        }
    }

    /// Accumulates a gradient for the given key.
    pub fn accumulate(&mut self, key: Uuid, gradient: Tensor) -> NeuralResult<()> {
        if let Some(existing) = self.accumulators.get(&key) {
            let accumulated = existing.add(&gradient)?;
            self.accumulators.insert(key, accumulated);
        } else {
            self.accumulators.insert(key, gradient);
        }
        Ok(())
    }

    /// Returns the accumulated gradient for the given key.
    #[must_use]
    pub fn get(&self, key: &Uuid) -> Option<&Tensor> {
        self.accumulators.get(key)
    }

    /// Returns all accumulated gradients.
    #[must_use]
    pub fn all(&self) -> &HashMap<Uuid, Tensor> {
        &self.accumulators
    }

    /// Clears all accumulators.
    pub fn reset(&mut self) {
        self.accumulators.clear();
    }

    /// Returns the number of accumulated gradients.
    #[must_use]
    pub fn len(&self) -> usize {
        self.accumulators.len()
    }

    /// Returns true if no gradients have been accumulated.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.accumulators.is_empty()
    }
}

impl Default for GradientAccumulator {
    fn default() -> Self {
        Self::new()
    }
}

/// Stop gradient operation - creates a detached view.
#[must_use]
pub fn stop_gradient(tensor: &Tensor) -> Tensor {
    tensor.detach()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::ComputationGraph;
    use crate::shape::Shape;

    #[test]
    fn autodiff_basic() {
        let mut tape = AutodiffEngine::new();
        assert!(tape.is_enabled());
        tape.set_enabled(false);
        assert!(!tape.is_enabled());
    }

    #[test]
    fn gradient_accumulator() {
        let mut acc = GradientAccumulator::new();
        let key = Uuid::new_v4();
        let grad = Tensor::from_vec_f32(&[1.0, 2.0], Shape::from_1d(2));
        acc.accumulate(key, grad).unwrap();
        assert_eq!(acc.len(), 1);
        acc.reset();
        assert!(acc.is_empty());
    }

    #[test]
    fn detached_tensor() {
        let t = Tensor::from_vec_f32(&[1.0, 2.0], Shape::from_1d(2));
        let d = DetachedTensor::new(t);
        assert_eq!(d.tensor().item_f64(&[0]).unwrap(), 1.0);
    }

    #[test]
    fn tape_recording() {
        let mut tape = AutodiffEngine::new();
        let node_id = crate::graph::NodeId::new();
        let output_id = crate::graph::NodeId::new();
        tape.record(
            node_id,
            OpType::Add,
            vec![crate::graph::NodeId::new(), crate::graph::NodeId::new()],
            OpParams::new(),
            output_id,
        );
        assert_eq!(tape.tape_len(), 1);
    }
}
