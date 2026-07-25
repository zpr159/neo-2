use std::sync::Arc;
use serde::{Deserialize, Serialize};
use neo_neural_engine::device::{Device, DeviceType};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ParallelismStrategy {
    None,
    TensorParallel,
    PipelineParallel,
    ExpertParallel,
    SequenceParallel,
    FSDP,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeviceRole {
    Primary,
    Secondary,
    Worker,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceAssignment {
    pub device_id: u32,
    pub device_type: DeviceType,
    pub role: DeviceRole,
    pub layers: Vec<u32>,
    pub memory_budget: u64,
    pub compute_weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TensorParallelConfig {
    pub world_size: usize,
    pub rank: usize,
    pub master_addr: String,
    pub master_port: u16,
    pub shards: Vec<DeviceAssignment>,
    pub all_reduce_backend: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineParallelConfig {
    pub num_stages: usize,
    pub stage_assignments: Vec<DeviceAssignment>,
    pub micro_batch_size: usize,
    pub num_micro_batches: usize,
    pub interleaved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiGpuPlan {
    pub strategy: ParallelismStrategy,
    pub devices: Vec<DeviceAssignment>,
    pub tensor_parallel: Option<TensorParallelConfig>,
    pub pipeline_parallel: Option<PipelineParallelConfig>,
    pub estimated_memory_per_device: u64,
    pub estimated_latency_ms: f64,
}

pub struct MultiGpuManager {
    devices: Vec<Arc<Device>>,
    active_plan: parking_lot::RwLock<Option<MultiGpuPlan>>,
}

impl std::fmt::Debug for MultiGpuManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MultiGpuManager")
            .field("device_count", &self.devices.len())
            .finish()
    }
}

impl MultiGpuManager {
    pub fn new(devices: Vec<Arc<Device>>) -> Self {
        Self {
            devices,
            active_plan: parking_lot::RwLock::new(None),
        }
    }

    #[must_use]
    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    #[must_use]
    pub fn has_gpus(&self) -> bool {
        self.devices.iter().any(|d| d.is_gpu())
    }

    #[must_use]
    pub fn gpu_devices(&self) -> Vec<Arc<Device>> {
        self.devices.iter().filter(|d| d.is_gpu()).cloned().collect()
    }

    #[must_use]
    pub fn total_gpu_memory(&self) -> u64 {
        self.gpu_devices().iter().map(|d| d.memory_total()).sum()
    }

    pub fn create_tensor_parallel_plan(
        &self,
        num_layers: u32,
        layer_memory: u64,
    ) -> Option<MultiGpuPlan> {
        let gpus = self.gpu_devices();
        if gpus.len() < 2 {
            return None;
        }
        let world_size = gpus.len();
        let layers_per_shard = num_layers / world_size as u32;
        let mut shards = Vec::new();
        for (i, gpu) in gpus.iter().enumerate() {
            let start_layer = (i as u32) * layers_per_shard;
            let end_layer = if i == world_size - 1 {
                num_layers
            } else {
                start_layer + layers_per_shard
            };
            let layers: Vec<u32> = (start_layer..end_layer).collect();
            shards.push(DeviceAssignment {
                device_id: gpu.id(),
                device_type: gpu.device_type(),
                role: if i == 0 { DeviceRole::Primary } else { DeviceRole::Worker },
                layers,
                memory_budget: gpu.memory_available(),
                compute_weight: 1.0 / world_size as f64,
            });
        }
        let mem_per_device = (num_layers as u64 * layer_memory) / world_size as u64;
        let plan = MultiGpuPlan {
            strategy: ParallelismStrategy::TensorParallel,
            devices: shards.clone(),
            tensor_parallel: Some(TensorParallelConfig {
                world_size,
                rank: 0,
                master_addr: "127.0.0.1".to_string(),
                master_port: 29500,
                shards,
                all_reduce_backend: "nccl".to_string(),
            }),
            pipeline_parallel: None,
            estimated_memory_per_device: mem_per_device,
            estimated_latency_ms: 0.0,
        };
        *self.active_plan.write() = Some(plan.clone());
        Some(plan)
    }

    pub fn create_pipeline_parallel_plan(
        &self,
        num_layers: u32,
        layer_memory: u64,
        micro_batch_size: usize,
    ) -> Option<MultiGpuPlan> {
        let gpus = self.gpu_devices();
        if gpus.len() < 2 {
            return None;
        }
        let num_stages = gpus.len();
        let layers_per_stage = num_layers / num_stages as u32;
        let mut stage_assignments = Vec::new();
        for (i, gpu) in gpus.iter().enumerate() {
            let start_layer = (i as u32) * layers_per_stage;
            let end_layer = if i == num_stages - 1 {
                num_layers
            } else {
                start_layer + layers_per_stage
            };
            let layers: Vec<u32> = (start_layer..end_layer).collect();
            stage_assignments.push(DeviceAssignment {
                device_id: gpu.id(),
                device_type: gpu.device_type(),
                role: if i == 0 { DeviceRole::Primary } else { DeviceRole::Worker },
                layers,
                memory_budget: gpu.memory_available(),
                compute_weight: 1.0 / num_stages as f64,
            });
        }
        let mem_per_device = (num_layers as u64 * layer_memory) / num_stages as u64;
        let plan = MultiGpuPlan {
            strategy: ParallelismStrategy::PipelineParallel,
            devices: stage_assignments.clone(),
            tensor_parallel: None,
            pipeline_parallel: Some(PipelineParallelConfig {
                num_stages,
                stage_assignments,
                micro_batch_size,
                num_micro_batches: 1,
                interleaved: false,
            }),
            estimated_memory_per_device: mem_per_device,
            estimated_latency_ms: 0.0,
        };
        *self.active_plan.write() = Some(plan.clone());
        Some(plan)
    }

    pub fn balance_memory(&self) -> Vec<(u32, u64)> {
        self.devices
            .iter()
            .map(|d| (d.id(), d.memory_available()))
            .collect()
    }

    pub fn select_best_device(&self, required_memory: u64) -> Option<Arc<Device>> {
        self.devices
            .iter()
            .filter(|d| d.memory_available() >= required_memory)
            .max_by_key(|d| d.memory_available())
            .cloned()
    }

    pub fn active_plan(&self) -> Option<MultiGpuPlan> {
        self.active_plan.read().clone()
    }
}
