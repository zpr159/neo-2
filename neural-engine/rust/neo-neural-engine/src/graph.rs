use std::collections::{HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{NeuralError, NeuralResult};
use crate::ops::{OpParams, OpType, OperationRegistry};
use crate::shape::Shape;

/// Unique identifier for a graph node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub Uuid);

impl NodeId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.0.to_string()[..8])
    }
}

/// Type of a graph node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeKind {
    /// An input/placeholder node.
    Input {
        name: String,
        shape: Vec<usize>,
    },
    /// A constant tensor node.
    Constant {
        name: String,
    },
    /// An operation node.
    Op {
        op_type: OpType,
        params: OpParams,
    },
}

impl std::fmt::Display for NodeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Input { name, shape } => write!(f, "Input({}, {:?})", name, shape),
            Self::Constant { name } => write!(f, "Constant({})", name),
            Self::Op { op_type, .. } => write!(f, "{}", op_type),
        }
    }
}

/// A single node in the computation graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: NodeId,
    pub kind: NodeKind,
    pub input_ids: Vec<NodeId>,
    pub output_shape: Option<Vec<usize>>,
    pub name: Option<String>,
}

impl GraphNode {
    #[must_use]
    pub fn new_input(name: String, shape: Vec<usize>) -> Self {
        let shape_clone = shape.clone();
        Self {
            id: NodeId::new(),
            kind: NodeKind::Input { name, shape },
            input_ids: Vec::new(),
            output_shape: Some(shape_clone),
            name: None,
        }
    }

    #[must_use]
    pub fn new_constant(name: String) -> Self {
        Self {
            id: NodeId::new(),
            kind: NodeKind::Constant { name },
            input_ids: Vec::new(),
            output_shape: None,
            name: None,
        }
    }

    #[must_use]
    pub fn new_op(op_type: OpType, inputs: Vec<NodeId>, params: OpParams) -> Self {
        Self {
            id: NodeId::new(),
            kind: NodeKind::Op { op_type, params },
            input_ids: inputs,
            output_shape: None,
            name: None,
        }
    }
}

/// A directed acyclic graph representing a computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputationGraph {
    nodes: Vec<GraphNode>,
    node_index: HashMap<NodeId, usize>,
    output_ids: Vec<NodeId>,
    name: String,
}

impl ComputationGraph {
    /// Creates a new empty computation graph.
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            nodes: Vec::new(),
            node_index: HashMap::new(),
            output_ids: Vec::new(),
            name: name.to_string(),
        }
    }

    /// Returns the graph name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Adds a node and returns its ID.
    pub fn add_node(&mut self, node: GraphNode) -> NodeId {
        let id = node.id;
        let idx = self.nodes.len();
        self.node_index.insert(id, idx);
        self.nodes.push(node);
        id
    }

    /// Adds an input node.
    pub fn add_input(&mut self, name: &str, shape: Vec<usize>) -> NodeId {
        self.add_node(GraphNode::new_input(name.to_string(), shape))
    }

    /// Adds an operation node.
    pub fn add_op(
        &mut self,
        op_type: OpType,
        inputs: Vec<NodeId>,
        params: OpParams,
    ) -> NodeId {
        self.add_node(GraphNode::new_op(op_type, inputs, params))
    }

    /// Sets the output nodes.
    pub fn set_outputs(&mut self, outputs: Vec<NodeId>) {
        self.output_ids = outputs;
    }

    /// Returns a reference to a node by ID.
    #[must_use]
    pub fn node(&self, id: NodeId) -> Option<&GraphNode> {
        self.node_index.get(&id).map(|&idx| &self.nodes[idx])
    }

    /// Returns a mutable reference to a node by ID.
    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut GraphNode> {
        self.node_index.get(&id).copied().map(move |idx| &mut self.nodes[idx])
    }

    /// Returns all nodes.
    #[must_use]
    pub fn nodes(&self) -> &[GraphNode] {
        &self.nodes
    }

    /// Returns the output node IDs.
    #[must_use]
    pub fn output_ids(&self) -> &[NodeId] {
        &self.output_ids
    }

    /// Returns the number of nodes.
    #[must_use]
    pub fn num_nodes(&self) -> usize {
        self.nodes.len()
    }

    /// Computes topological sort of all nodes.
    pub fn topological_sort(&self) -> NeuralResult<Vec<NodeId>> {
        let mut in_degree: HashMap<NodeId, usize> = HashMap::new();
        let mut children: HashMap<NodeId, Vec<NodeId>> = HashMap::new();

        for node in &self.nodes {
            in_degree.entry(node.id).or_insert(0);
            for &input_id in &node.input_ids {
                children.entry(input_id).or_default().push(node.id);
                *in_degree.entry(node.id).or_insert(0) += 1;
            }
        }

        let mut queue: VecDeque<NodeId> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut sorted = Vec::with_capacity(self.nodes.len());

        while let Some(current) = queue.pop_front() {
            sorted.push(current);
            if let Some(child_list) = children.get(&current) {
                for &child in child_list {
                    if let Some(deg) = in_degree.get_mut(&child) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(child);
                        }
                    }
                }
            }
        }

        if sorted.len() != self.nodes.len() {
            return Err(NeuralError::GraphCycle {
                path: sorted.iter().map(|id| id.to_string()).collect(),
            });
        }

        Ok(sorted)
    }

    /// Validates the graph structure.
    pub fn validate(&self, registry: &OperationRegistry) -> NeuralResult<()> {
        let node_ids: HashSet<NodeId> = self.nodes.iter().map(|n| n.id).collect();

        // Check all input references exist
        for node in &self.nodes {
            for &input_id in &node.input_ids {
                if !node_ids.contains(&input_id) {
                    return Err(NeuralError::GraphValidation {
                        message: format!(
                            "Node {} references non-existent input {}",
                            node.id, input_id
                        ),
                    });
                }
            }
        }

        // Check for cycles
        self.topological_sort()?;

        // Check all ops are registered
        for node in &self.nodes {
            if let NodeKind::Op { op_type, .. } = &node.kind {
                if registry.get(op_type.name()).is_none() {
                    return Err(NeuralError::OpNotRegistered {
                        op_name: op_type.name().to_string(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Infers shapes for all nodes using the registry.
    pub fn infer_shapes(&mut self, registry: &OperationRegistry) -> NeuralResult<()> {
        let order = self.topological_sort()?;
        let node_ids: HashSet<NodeId> = order.iter().copied().collect();

        for node_id in &order {
            let node = self
                .node(*node_id)
                .ok_or_else(|| NeuralError::GraphValidation {
                    message: "node not found during shape inference".to_string(),
                })?;

            match &node.kind {
                NodeKind::Input { shape, .. } => {
                    let shape_clone = shape.clone();
                    if let Some(n) = self.node_mut(*node_id) {
                        n.output_shape = Some(shape_clone);
                    }
                }
                NodeKind::Constant { .. } => {}
                NodeKind::Op { op_type, params } => {
                    let input_shapes: Vec<Shape> = node
                        .input_ids
                        .iter()
                        .filter_map(|&id| self.node(id).and_then(|n| n.output_shape.as_ref()))
                        .map(|s| Shape::new(s.clone()))
                        .collect();

                    let shape_refs: Vec<&Shape> = input_shapes.iter().collect();

                    if let Some(reg) = registry.get(op_type.name()) {
                        let output = reg.compute.output_shape(&shape_refs, params)?;
                        if let Some(n) = self.node_mut(*node_id) {
                            n.output_shape = Some(output.to_vec());
                        }
                    }
                }
            }
            let _ = node_ids;
        }
        Ok(())
    }

    /// Returns the dependency list for a node (all ancestors).
    #[must_use]
    pub fn dependencies(&self, node_id: NodeId) -> HashSet<NodeId> {
        let mut deps = HashSet::new();
        let mut queue = VecDeque::new();

        if let Some(node) = self.node(node_id) {
            for &input_id in &node.input_ids {
                queue.push_back(input_id);
            }
        }

        while let Some(current) = queue.pop_front() {
            if deps.insert(current) {
                if let Some(node) = self.node(current) {
                    for &input_id in &node.input_ids {
                        queue.push_back(input_id);
                    }
                }
            }
        }
        deps
    }

    /// Returns nodes that have no dependents (leaf nodes).
    #[must_use]
    pub fn leaf_nodes(&self) -> Vec<NodeId> {
        let mut has_child: HashSet<NodeId> = HashSet::new();
        for node in &self.nodes {
            for &input_id in &node.input_ids {
                has_child.insert(input_id);
            }
        }
        self.nodes
            .iter()
            .filter(|n| !has_child.contains(&n.id))
            .map(|n| n.id)
            .collect()
    }

    /// Optimizes the graph by removing unused nodes.
    pub fn optimize(&mut self) -> NeuralResult<usize> {
        let reachable = self.reachable_nodes();
        let before = self.nodes.len();
        self.nodes.retain(|n| reachable.contains(&n.id));
        self.node_index.clear();
        for (idx, node) in self.nodes.iter().enumerate() {
            self.node_index.insert(node.id, idx);
        }
        Ok(before - self.nodes.len())
    }

    /// Returns all reachable nodes from outputs.
    fn reachable_nodes(&self) -> HashSet<NodeId> {
        let mut reachable = HashSet::new();
        let mut stack: VecDeque<NodeId> = self.output_ids.iter().copied().collect();

        while let Some(id) = stack.pop_back() {
            if reachable.insert(id) {
                if let Some(node) = self.node(id) {
                    for &input_id in &node.input_ids {
                        stack.push_back(input_id);
                    }
                }
            }
        }
        reachable
    }

    /// Creates a subgraph containing only specified nodes.
    pub fn subgraph(&self, node_ids: &[NodeId]) -> NeuralResult<ComputationGraph> {
        let id_set: HashSet<NodeId> = node_ids.iter().copied().collect();
        let mut sub = ComputationGraph::new(&format!("{}_sub", self.name));

        for &nid in node_ids {
            if let Some(node) = self.node(nid) {
                let mut new_node = node.clone();
                new_node.input_ids.retain(|id| id_set.contains(id));
                sub.add_node(new_node);
            }
        }
        Ok(sub)
    }
}

impl Default for ComputationGraph {
    fn default() -> Self {
        Self::new("default")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::create_default_registry;

    #[test]
    fn graph_basic() {
        let mut g = ComputationGraph::new("test");
        let x = g.add_input("x", vec![2, 3]);
        let y = g.add_input("y", vec![2, 3]);
        let z = g.add_op(OpType::Add, vec![x, y], OpParams::new());
        g.set_outputs(vec![z]);
        assert_eq!(g.num_nodes(), 3);
    }

    #[test]
    fn graph_topo_sort() {
        let mut g = ComputationGraph::new("test");
        let x = g.add_input("x", vec![2, 3]);
        let y = g.add_input("y", vec![2, 3]);
        let z = g.add_op(OpType::Add, vec![x, y], OpParams::new());
        g.set_outputs(vec![z]);
        let order = g.topological_sort().unwrap();
        assert_eq!(order.len(), 3);
        let x_pos = order.iter().position(|&id| id == x).unwrap();
        let z_pos = order.iter().position(|&id| id == z).unwrap();
        assert!(x_pos < z_pos);
    }

    #[test]
    fn graph_validate() {
        let registry = create_default_registry();
        let mut g = ComputationGraph::new("test");
        let x = g.add_input("x", vec![2, 3]);
        let y = g.add_input("y", vec![2, 3]);
        let z = g.add_op(OpType::Add, vec![x, y], OpParams::new());
        g.set_outputs(vec![z]);
        g.validate(&registry).unwrap();
    }

    #[test]
    fn graph_infer_shapes() {
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
    fn graph_optimize() {
        let mut g = ComputationGraph::new("test");
        let _x = g.add_input("x", vec![2, 3]);
        let _y = g.add_input("y", vec![2, 3]);
        let z = g.add_input("z", vec![2, 3]);
        g.set_outputs(vec![z]);
        let removed = g.optimize().unwrap();
        assert_eq!(removed, 2);
        assert_eq!(g.num_nodes(), 1);
    }
}
