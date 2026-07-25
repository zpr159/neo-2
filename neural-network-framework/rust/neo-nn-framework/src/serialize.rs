use std::collections::HashMap;
use std::path::Path;

use crate::autograd::ADTensor;
use crate::error::NnResult;
use crate::module::Module;

#[derive(Debug, Clone)]
pub struct ModelCheckpoint {
    pub version: u32,
    pub framework: String,
    pub pytorch_compatible: bool,
}

impl Default for ModelCheckpoint {
    fn default() -> Self {
        Self { version: 1, framework: "neo-nn".to_string(), pytorch_compatible: false }
    }
}

pub fn save_model_weights<M: Module>(model: &M, path: &Path) -> NnResult<()> {
    let state = model.state_dict();
    let serializable: HashMap<String, Vec<f64>> = state.iter()
        .map(|(k, v)| (k.clone(), v.to_vec_f64().unwrap_or_default()))
        .collect();
    let data = bincode::serialize(&serializable)?;
    std::fs::write(path, data)?;
    Ok(())
}

pub fn load_model_weights<M: Module>(model: &mut M, path: &Path) -> NnResult<()> {
    let data = std::fs::read(path)?;
    let serializable: HashMap<String, Vec<f64>> = bincode::deserialize(&data)?;
    let state: HashMap<String, ADTensor> = serializable.iter()
        .map(|(k, v)| {
            let tensor = neo_neural_engine::tensor::Tensor::from_vec_f64(v, neo_neural_engine::shape::Shape::from_1d(v.len()));
            (k.clone(), ADTensor::new(tensor, true))
        })
        .collect();
    model.load_state_dict(&state)
}

pub fn save_model_json<M: Module>(model: &M, path: &Path) -> NnResult<()> {
    let state = model.state_dict();
    let json_data: HashMap<String, Vec<f64>> = state.iter()
        .map(|(k, v)| (k.clone(), v.to_vec_f64().unwrap_or_default()))
        .collect();
    let json = serde_json::to_string_pretty(&json_data)?;
    std::fs::write(path, json)?;
    Ok(())
}

pub fn load_model_json<M: Module>(model: &mut M, path: &Path) -> NnResult<()> {
    let json_str = std::fs::read_to_string(path)?;
    let json_data: HashMap<String, Vec<f64>> = serde_json::from_str(&json_str)?;
    let state: HashMap<String, ADTensor> = json_data.iter()
        .map(|(k, v)| {
            let tensor = neo_neural_engine::tensor::Tensor::from_vec_f64(v, neo_neural_engine::shape::Shape::from_1d(v.len()));
            (k.clone(), ADTensor::new(tensor, true))
        })
        .collect();
    model.load_state_dict(&state)
}

pub fn export_onnx_metadata<M: Module>(model: &M, path: &Path) -> NnResult<()> {
    let metadata = serde_json::json!({
        "framework": "neo-nn",
        "version": "0.1.0",
        "num_parameters": model.num_parameters(),
        "num_submodules": model.num_submodules(),
        "parameters": model.parameters().keys().collect::<Vec<_>>(),
    });
    std::fs::write(path, serde_json::to_string_pretty(&metadata)?)?;
    Ok(())
}

pub fn save_state_dict(state: &HashMap<String, ADTensor>, path: &Path) -> NnResult<()> {
    let serializable: HashMap<String, Vec<f64>> = state.iter()
        .map(|(k, v)| (k.clone(), v.to_vec_f64().unwrap_or_default()))
        .collect();
    let data = bincode::serialize(&serializable)?;
    std::fs::write(path, data)?;
    Ok(())
}

pub fn load_state_dict(path: &Path) -> NnResult<HashMap<String, ADTensor>> {
    let data = std::fs::read(path)?;
    let serializable: HashMap<String, Vec<f64>> = bincode::deserialize(&data)?;
    Ok(serializable.iter()
        .map(|(k, v)| {
            let tensor = neo_neural_engine::tensor::Tensor::from_vec_f64(v, neo_neural_engine::shape::Shape::from_1d(v.len()));
            (k.clone(), ADTensor::new(tensor, true))
        })
        .collect())
}
