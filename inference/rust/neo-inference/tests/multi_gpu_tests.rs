use neo_neural_engine::device::{Device, DeviceType};
use neo_inference::multi_gpu::{MultiGpuManager, ParallelismStrategy};
use std::sync::Arc;

fn make_gpu(id: u32, memory: u64) -> Arc<Device> {
    Arc::new(Device::cuda(
        id,
        format!("GPU-{}", id),
        memory,
        (8, 0),
    ))
}

fn make_manager(devices: Vec<Arc<Device>>) -> MultiGpuManager {
    MultiGpuManager::new(devices)
}

#[test]
fn test_device_count() {
    let gpus = vec![make_gpu(0, 8_000_000_000), make_gpu(1, 8_000_000_000)];
    let mgr = make_manager(gpus);
    assert_eq!(mgr.device_count(), 2);
}

#[test]
fn test_device_count_empty() {
    let mgr = make_manager(vec![]);
    assert_eq!(mgr.device_count(), 0);
}

#[test]
fn test_has_gpus() {
    let gpus = vec![make_gpu(0, 8_000_000_000)];
    let mgr = make_manager(gpus);
    assert!(mgr.has_gpus());
}

#[test]
fn test_has_gpus_false() {
    let cpu = Arc::new(Device::cpu());
    let mgr = make_manager(vec![cpu]);
    assert!(!mgr.has_gpus());
}

#[test]
fn test_total_gpu_memory() {
    let gpus = vec![make_gpu(0, 8_000_000_000), make_gpu(1, 16_000_000_000)];
    let mgr = make_manager(gpus);
    assert_eq!(mgr.total_gpu_memory(), 24_000_000_000);
}

#[test]
fn test_select_best_device() {
    let gpus = vec![make_gpu(0, 4_000_000_000), make_gpu(1, 16_000_000_000)];
    let mgr = make_manager(gpus);
    let best = mgr.select_best_device(8_000_000_000);
    assert!(best.is_some());
    assert_eq!(best.unwrap().id(), 1);
}

#[test]
fn test_select_best_device_none_available() {
    let gpus = vec![make_gpu(0, 2_000_000_000)];
    let mgr = make_manager(gpus);
    let best = mgr.select_best_device(8_000_000_000);
    assert!(best.is_none());
}

#[test]
fn test_create_tensor_parallel_plan() {
    let gpus = vec![make_gpu(0, 16_000_000_000), make_gpu(1, 16_000_000_000)];
    let mgr = make_manager(gpus);
    let plan = mgr.create_tensor_parallel_plan(24, 100_000_000);
    assert!(plan.is_some());
    let plan = plan.unwrap();
    assert_eq!(plan.strategy, ParallelismStrategy::TensorParallel);
    assert_eq!(plan.devices.len(), 2);
    assert!(plan.tensor_parallel.is_some());
}

#[test]
fn test_tensor_parallel_insufficient_gpus() {
    let gpus = vec![make_gpu(0, 16_000_000_000)];
    let mgr = make_manager(gpus);
    let plan = mgr.create_tensor_parallel_plan(24, 100_000_000);
    assert!(plan.is_none());
}

#[test]
fn test_balance_memory() {
    let gpus = vec![make_gpu(0, 4_000_000_000), make_gpu(1, 8_000_000_000)];
    let mgr = make_manager(gpus);
    let balances = mgr.balance_memory();
    assert_eq!(balances.len(), 2);
    assert_eq!(balances[0].1, 4_000_000_000);
    assert_eq!(balances[1].1, 8_000_000_000);
}

#[test]
fn test_active_plan() {
    let gpus = vec![make_gpu(0, 16_000_000_000), make_gpu(1, 16_000_000_000)];
    let mgr = make_manager(gpus);
    assert!(mgr.active_plan().is_none());

    mgr.create_tensor_parallel_plan(24, 100_000_000);
    assert!(mgr.active_plan().is_some());
}

#[test]
fn test_gpu_devices() {
    let cpu = Arc::new(Device::cpu());
    let gpu0 = make_gpu(0, 8_000_000_000);
    let gpu1 = make_gpu(1, 8_000_000_000);
    let mgr = make_manager(vec![cpu, gpu0, gpu1]);
    assert_eq!(mgr.gpu_devices().len(), 2);
}

#[test]
fn test_empty_manager() {
    let mgr = make_manager(vec![]);
    assert!(!mgr.has_gpus());
    assert_eq!(mgr.total_gpu_memory(), 0);
    assert!(mgr.select_best_device(0).is_none());
    assert!(mgr.create_tensor_parallel_plan(12, 100_000_000).is_none());
}
