use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::dtype::DType;
use crate::error::{NeuralError, NeuralResult};
use crate::graph::{ComputationGraph, GraphNode, NodeId, NodeKind};
use crate::shape::Shape;
use crate::tensor::Tensor;

/// Version of the serialization format.
pub const SERIALIZATION_VERSION: u32 = 1;

/// Magic bytes for Neo tensor format.
pub const TENSOR_MAGIC: &[u8; 4] = b"NTEN";

/// Magic bytes for Neo graph format.
pub const GRAPH_MAGIC: &[u8; 4] = b"NGPH";

/// Header for a serialized tensor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TensorHeader {
    pub version: u32,
    pub dtype: DType,
    pub shape: Vec<usize>,
    pub byte_size: usize,
    pub name: Option<String>,
    pub metadata: HashMap<String, String>,
}

/// Header for a serialized graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphHeader {
    pub version: u32,
    pub name: String,
    pub num_nodes: usize,
    pub num_outputs: usize,
    pub metadata: HashMap<String, String>,
}

/// Serialized graph node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedNode {
    pub id: String,
    pub kind: NodeKind,
    pub input_ids: Vec<String>,
    pub output_shape: Option<Vec<usize>>,
}

/// Serialized graph data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedGraph {
    pub header: GraphHeader,
    pub nodes: Vec<SerializedNode>,
    pub output_ids: Vec<String>,
}

/// Serializes a tensor to bytes (format: magic + header + data).
pub fn serialize_tensor(tensor: &Tensor, name: Option<&str>) -> NeuralResult<Vec<u8>> {
    let header = TensorHeader {
        version: SERIALIZATION_VERSION,
        dtype: tensor.dtype(),
        shape: tensor.shape().to_vec(),
        byte_size: tensor.byte_size(),
        name: name.map(String::from),
        metadata: HashMap::new(),
    };

    let header_bytes = bincode::serialize(&header).map_err(|e| {
        NeuralError::SerializationError {
            message: format!("header serialization failed: {}", e),
        }
    })?;

    let header_len = (header_bytes.len() as u32).to_le_bytes();

    let mut result = Vec::with_capacity(4 + 4 + header_bytes.len() + tensor.byte_size());
    result.extend_from_slice(TENSOR_MAGIC);
    result.extend_from_slice(&header_len);
    result.extend_from_slice(&header_bytes);
    result.extend_from_slice(tensor.as_bytes());

    Ok(result)
}

/// Deserializes a tensor from bytes.
pub fn deserialize_tensor(data: &[u8]) -> NeuralResult<(Tensor, Option<String>)> {
    if data.len() < 8 {
        return Err(NeuralError::SerializationError {
            message: "data too short for tensor header".to_string(),
        });
    }

    // Check magic
    if &data[..4] != TENSOR_MAGIC {
        return Err(NeuralError::SerializationError {
            message: "invalid tensor magic bytes".to_string(),
        });
    }

    let header_len = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
    let header_end = 8 + header_len;

    if data.len() < header_end {
        return Err(NeuralError::SerializationError {
            message: "data too short for header".to_string(),
        });
    }

    let header: TensorHeader = bincode::deserialize(&data[8..header_end]).map_err(|e| {
        NeuralError::SerializationError {
            message: format!("header deserialization failed: {}", e),
        }
    })?;

    let data_start = header_end;
    let data_end = data_start + header.byte_size;

    if data.len() < data_end {
        return Err(NeuralError::SerializationError {
            message: "data too short for tensor data".to_string(),
        });
    }

    let tensor_data = data[data_start..data_end].to_vec();
    let tensor = Tensor::from_bytes(tensor_data, Shape::new(header.shape), header.dtype)?;

    Ok((tensor, header.name))
}

/// Serializes a computation graph to bytes.
pub fn serialize_graph(graph: &ComputationGraph) -> NeuralResult<Vec<u8>> {
    let header = GraphHeader {
        version: SERIALIZATION_VERSION,
        name: graph.name().to_string(),
        num_nodes: graph.num_nodes(),
        num_outputs: graph.output_ids().len(),
        metadata: HashMap::new(),
    };

    let mut serialized_nodes = Vec::new();
    for node in graph.nodes() {
        serialized_nodes.push(SerializedNode {
            id: node.id.0.to_string(),
            kind: node.kind.clone(),
            input_ids: node.input_ids.iter().map(|id| id.0.to_string()).collect(),
            output_shape: node.output_shape.clone(),
        });
    }

    let output_ids: Vec<String> = graph
        .output_ids()
        .iter()
        .map(|id| id.0.to_string())
        .collect();

    let serialized = SerializedGraph {
        header,
        nodes: serialized_nodes,
        output_ids,
    };

    let json_bytes = serde_json::to_vec(&serialized).map_err(|e| {
        NeuralError::SerializationError {
            message: format!("graph JSON serialization failed: {}", e),
        }
    })?;

    let mut result = Vec::with_capacity(4 + json_bytes.len());
    result.extend_from_slice(GRAPH_MAGIC);
    let len = (json_bytes.len() as u32).to_le_bytes();
    result.extend_from_slice(&len);
    result.extend_from_slice(&json_bytes);

    Ok(result)
}

/// Deserializes a computation graph from bytes.
pub fn deserialize_graph(data: &[u8]) -> NeuralResult<ComputationGraph> {
    if data.len() < 8 {
        return Err(NeuralError::SerializationError {
            message: "data too short for graph header".to_string(),
        });
    }

    if &data[..4] != GRAPH_MAGIC {
        return Err(NeuralError::SerializationError {
            message: "invalid graph magic bytes".to_string(),
        });
    }

    let json_len = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
    let json_start = 8;

    if data.len() < json_start + json_len {
        return Err(NeuralError::SerializationError {
            message: "data too short for graph JSON".to_string(),
        });
    }

    let serialized: SerializedGraph =
        serde_json::from_slice(&data[json_start..json_start + json_len]).map_err(|e| {
            NeuralError::SerializationError {
                message: format!("graph JSON deserialization failed: {}", e),
            }
        })?;

    let mut graph = ComputationGraph::new(&serialized.header.name);
    let mut id_map: HashMap<String, NodeId> = HashMap::new();

    for snode in &serialized.nodes {
        let mut node = match &snode.kind {
            NodeKind::Input { name, shape } => GraphNode::new_input(name.clone(), shape.clone()),
            NodeKind::Constant { name } => GraphNode::new_constant(name.clone()),
            NodeKind::Op {
                op_type,
                params,
            } => {
                let input_ids: Vec<NodeId> = snode
                    .input_ids
                    .iter()
                    .filter_map(|id| id_map.get(id).copied())
                    .collect();
                GraphNode::new_op(*op_type, input_ids, params.clone())
            }
        };

        // Restore original ID
        if let Ok(uuid) = uuid::Uuid::parse_str(&snode.id) {
            node.id = NodeId(uuid);
        }

        id_map.insert(snode.id.clone(), node.id);
        graph.add_node(node);
    }

    let output_ids: Vec<NodeId> = serialized
        .output_ids
        .iter()
        .filter_map(|id| id_map.get(id).copied())
        .collect();
    graph.set_outputs(output_ids);

    Ok(graph)
}

/// Metadata for versioned serialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub format_version: u32,
    pub engine_version: String,
    pub created_at: String,
}

impl VersionInfo {
    #[must_use]
    pub fn current() -> Self {
        Self {
            format_version: SERIALIZATION_VERSION,
            engine_version: env!("CARGO_PKG_VERSION").to_string(),
            created_at: chrono_placeholder(),
        }
    }
}

fn chrono_placeholder() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::{OpParams, OpType};

    #[test]
    fn tensor_roundtrip() {
        let t = Tensor::from_vec_f32(&[1.0, 2.0, 3.0, 4.0], Shape::from_2d(2, 2));
        let bytes = serialize_tensor(&t, Some("test_tensor")).unwrap();
        let (t2, name) = deserialize_tensor(&bytes).unwrap();
        assert_eq!(name, Some("test_tensor".to_string()));
        assert_eq!(t2.shape().dims(), &[2, 2]);
        assert!((t2.item_f64(&[0, 0]).unwrap() - 1.0).abs() < 1e-6);
        assert!((t2.item_f64(&[1, 1]).unwrap() - 4.0).abs() < 1e-6);
    }

    #[test]
    fn graph_roundtrip() {
        let mut graph = ComputationGraph::new("test_graph");
        let x = graph.add_input("x", vec![2, 3]);
        let y = graph.add_input("y", vec![2, 3]);
        let z = graph.add_op(
            OpType::Add,
            vec![x, y],
            OpParams::new(),
        );
        graph.set_outputs(vec![z]);

        let bytes = serialize_graph(&graph).unwrap();
        let graph2 = deserialize_graph(&bytes).unwrap();
        assert_eq!(graph2.name(), "test_graph");
        assert_eq!(graph2.num_nodes(), 3);
    }

    #[test]
    fn version_info() {
        let v = VersionInfo::current();
        assert_eq!(v.format_version, SERIALIZATION_VERSION);
    }

    #[test]
    fn invalid_magic() {
        let data = b"XXXX";
        assert!(deserialize_tensor(data).is_err());
    }
}
