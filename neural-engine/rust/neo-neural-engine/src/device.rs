use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::error::{NeuralError, NeuralResult};
use crate::dtype::DType;
use crate::shape::Shape;

/// Types of compute devices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeviceType {
    Cpu,
    Cuda,
    Rocm,
    Metal,
    Vulkan,
    OpenCl,
}

impl DeviceType {
    /// Returns true if this is a GPU device type.
    #[must_use]
    pub const fn is_gpu(self) -> bool {
        matches!(
            self,
            Self::Cuda | Self::Rocm | Self::Metal | Self::Vulkan | Self::OpenCl
        )
    }

    /// Returns the string name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Cpu => "CPU",
            Self::Cuda => "CUDA",
            Self::Rocm => "ROCm",
            Self::Metal => "Metal",
            Self::Vulkan => "Vulkan",
            Self::OpenCl => "OpenCL",
        }
    }
}

impl fmt::Display for DeviceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A handle to a compute device.
#[derive(Debug, Serialize, Deserialize)]
pub struct Device {
    id: u32,
    device_type: DeviceType,
    name: String,
    memory_total: u64,
    #[serde(skip)]
    memory_used: AtomicU64,
    compute_capability: Option<(u32, u32)>,
    max_threads_per_block: u32,
    max_blocks_per_grid: u32,
    warp_size: u32,
}

impl Clone for Device {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            device_type: self.device_type,
            name: self.name.clone(),
            memory_total: self.memory_total,
            memory_used: AtomicU64::new(self.memory_used.load(Ordering::Relaxed)),
            compute_capability: self.compute_capability,
            max_threads_per_block: self.max_threads_per_block,
            max_blocks_per_grid: self.max_blocks_per_grid,
            warp_size: self.warp_size,
        }
    }
}

impl Device {
    /// Creates a CPU device.
    #[must_use]
    pub fn cpu() -> Self {
        Self {
            id: 0,
            device_type: DeviceType::Cpu,
            name: "CPU".to_string(),
            memory_total: 0,
            memory_used: AtomicU64::new(0),
            compute_capability: None,
            max_threads_per_block: 1024,
            max_blocks_per_grid: 65535,
            warp_size: 1,
        }
    }

    /// Creates a CUDA device.
    #[must_use]
    pub fn cuda(device_id: u32, name: String, memory: u64, compute_cap: (u32, u32)) -> Self {
        Self {
            id: device_id,
            device_type: DeviceType::Cuda,
            name,
            memory_total: memory,
            memory_used: AtomicU64::new(0),
            compute_capability: Some(compute_cap),
            max_threads_per_block: 1024,
            max_blocks_per_grid: 2147483647,
            warp_size: 32,
        }
    }

    /// Creates a Metal device.
    #[must_use]
    pub fn metal(device_id: u32, name: String, memory: u64) -> Self {
        Self {
            id: device_id,
            device_type: DeviceType::Metal,
            name,
            memory_total: memory,
            memory_used: AtomicU64::new(0),
            compute_capability: None,
            max_threads_per_block: 1024,
            max_blocks_per_grid: 65535,
            warp_size: 32,
        }
    }

    /// Returns the device ID.
    #[must_use]
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Returns the device type.
    #[must_use]
    pub fn device_type(&self) -> DeviceType {
        self.device_type
    }

    /// Returns the device name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns total device memory in bytes.
    #[must_use]
    pub fn memory_total(&self) -> u64 {
        self.memory_total
    }

    /// Returns used device memory in bytes.
    #[must_use]
    pub fn memory_used(&self) -> u64 {
        self.memory_used.load(Ordering::Relaxed)
    }

    /// Returns available device memory in bytes.
    #[must_use]
    pub fn memory_available(&self) -> u64 {
        self.memory_total
            .saturating_sub(self.memory_used.load(Ordering::Relaxed))
    }

    /// Returns memory usage as a fraction (0.0 to 1.0).
    #[must_use]
    pub fn memory_usage_fraction(&self) -> f64 {
        if self.memory_total == 0 {
            return 0.0;
        }
        self.memory_used.load(Ordering::Relaxed) as f64 / self.memory_total as f64
    }

    /// Returns compute capability if applicable.
    #[must_use]
    pub fn compute_capability(&self) -> Option<(u32, u32)> {
        self.compute_capability
    }

    /// Returns max threads per block.
    #[must_use]
    pub fn max_threads_per_block(&self) -> u32 {
        self.max_threads_per_block
    }

    /// Returns max blocks per grid.
    #[must_use]
    pub fn max_blocks_per_grid(&self) -> u32 {
        self.max_blocks_per_grid
    }

    /// Returns warp/wavefront size.
    #[must_use]
    pub fn warp_size(&self) -> u32 {
        self.warp_size
    }

    /// Records memory allocation.
    pub fn record_allocation(&self, bytes: u64) {
        self.memory_used.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Records memory deallocation.
    pub fn record_deallocation(&self, bytes: u64) {
        self.memory_used.fetch_sub(bytes, Ordering::Relaxed);
    }

    /// Returns true if this device is available.
    #[must_use]
    pub fn is_available(&self) -> bool {
        true
    }

    /// Returns true if this is a GPU device.
    #[must_use]
    pub fn is_gpu(&self) -> bool {
        self.device_type.is_gpu()
    }
}

impl Default for Device {
    fn default() -> Self {
        Self::cpu()
    }
}

impl fmt::Display for Device {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} (id={}, mem={}/{})",
            self.name,
            self.id,
            self.memory_used(),
            self.memory_total
        )
    }
}

/// Trait for compute backend implementations.
pub trait Backend: Send + Sync + fmt::Debug {
    /// Returns the device this backend operates on.
    fn device(&self) -> &Device;

    /// Returns the backend name.
    fn name(&self) -> &str;

    /// Allocates device memory.
    fn alloc(&self, size: usize) -> NeuralResult<usize>;

    /// Frees device memory.
    fn free(&self, offset: usize);

    /// Copies data from host to device.
    fn copy_host_to_device(
        &self,
        host_src: &[u8],
        device_dst: usize,
        size: usize,
    ) -> NeuralResult<()>;

    /// Copies data from device to host.
    fn copy_device_to_host(
        &self,
        device_src: usize,
        host_dst: &mut [u8],
        size: usize,
    ) -> NeuralResult<()>;

    /// Copies data within device.
    fn copy_device_to_device(
        &self,
        src: usize,
        dst: usize,
        size: usize,
    ) -> NeuralResult<()>;

    /// Synchronizes the device (waits for all operations to complete).
    fn synchronize(&self) -> NeuralResult<()>;

    /// Returns the pointer to device memory at the given offset.
    fn device_ptr(&self, offset: usize) -> *const u8;

    /// Returns the mutable pointer to device memory at the given offset.
    fn device_ptr_mut(&self, offset: usize) -> *mut u8;

    /// Performs element-wise binary operation.
    fn binary_op(
        &self,
        op: BinaryOp,
        left: &TensorData,
        right: &TensorData,
        output: &mut TensorData,
    ) -> NeuralResult<()>;

    /// Performs element-wise unary operation.
    fn unary_op(
        &self,
        op: UnaryOp,
        input: &TensorData,
        output: &mut TensorData,
    ) -> NeuralResult<()>;

    /// Performs matrix multiplication.
    fn matmul(
        &self,
        a: &TensorData,
        b: &TensorData,
        c: &mut TensorData,
        m: usize,
        n: usize,
        k: usize,
    ) -> NeuralResult<()>;

    /// Performs reduction operation.
    fn reduce(
        &self,
        op: ReduceOp,
        input: &TensorData,
        output: &mut TensorData,
        axis: usize,
    ) -> NeuralResult<()>;

    /// Performs a transpose operation.
    fn transpose(
        &self,
        input: &TensorData,
        output: &mut TensorData,
        axes: &[usize],
    ) -> NeuralResult<()>;

    /// Performs concatenation.
    fn concat(
        &self,
        inputs: &[&TensorData],
        output: &mut TensorData,
        axis: usize,
    ) -> NeuralResult<()>;

    /// Performs slicing.
    fn slice(
        &self,
        input: &TensorData,
        output: &mut TensorData,
        ranges: &[(usize, usize)],
    ) -> NeuralResult<()>;
}

/// Binary operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Modulo,
    Maximum,
    Minimum,
}

/// Unary operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Abs,
    Exp,
    Log,
    Sqrt,
    Sin,
    Cos,
    Tanh,
    Relu,
    Gelu,
    Sigmoid,
    Silu,
}

/// Reduction operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReduceOp {
    Sum,
    Mean,
    Max,
    Min,
    Prod,
    Std,
}

/// Raw tensor data reference for backend operations.
#[derive(Debug)]
pub struct TensorData {
    pub bytes: Vec<u8>,
    pub dtype: DType,
    pub shape: Shape,
}

impl TensorData {
    /// Creates new zeroed tensor data.
    #[must_use]
    pub fn zeros(dtype: DType, shape: Shape) -> Self {
        let numel = shape.numel();
        let size = numel * dtype.byte_size();
        Self {
            bytes: vec![0u8; size],
            dtype,
            shape,
        }
    }

    /// Returns the number of elements.
    #[must_use]
    pub fn numel(&self) -> usize {
        self.shape.numel()
    }

    /// Returns total byte size.
    #[must_use]
    pub fn byte_size(&self) -> usize {
        self.bytes.len()
    }
}

/// Manages discovery and selection of compute devices.
#[derive(Debug)]
pub struct DeviceManager {
    devices: RwLock<Vec<Arc<Device>>>,
}

impl DeviceManager {
    /// Creates a new device manager and probes available devices.
    #[must_use]
    pub fn new() -> Self {
        let devices = Self::probe_devices();
        Self {
            devices: RwLock::new(devices),
        }
    }

    /// Lists all detected devices.
    #[must_use]
    pub fn list_devices(&self) -> Vec<Arc<Device>> {
        self.devices.read().clone()
    }

    /// Returns the default device (CPU).
    #[must_use]
    pub fn default_device(&self) -> Arc<Device> {
        self.devices
            .read()
            .first()
            .cloned()
            .unwrap_or_else(|| Arc::new(Device::cpu()))
    }

    /// Selects the best device matching the preferred type.
    pub fn select_device(&self, preferred: Option<DeviceType>) -> NeuralResult<Arc<Device>> {
        let devices = self.devices.read();
        if let Some(pref) = preferred {
            for dev in devices.iter() {
                if dev.device_type() == pref {
                    return Ok(Arc::clone(dev));
                }
            }
            return Err(NeuralError::DeviceNotAvailable {
                device: pref.to_string(),
            });
        }
        Ok(devices.first().cloned().unwrap_or_else(|| Arc::new(Device::cpu())))
    }

    /// Registers a device.
    pub fn register_device(&self, device: Device) {
        self.devices.write().push(Arc::new(device));
    }

    fn probe_devices() -> Vec<Arc<Device>> {
        vec![Arc::new(Device::cpu())]
    }
}

impl Default for DeviceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_type_properties() {
        assert!(!DeviceType::Cpu.is_gpu());
        assert!(DeviceType::Cuda.is_gpu());
        assert!(DeviceType::Metal.is_gpu());
    }

    #[test]
    fn cpu_device() {
        let dev = Device::cpu();
        assert_eq!(dev.device_type(), DeviceType::Cpu);
        assert!(!dev.is_gpu());
        assert_eq!(dev.memory_total(), 0);
    }

    #[test]
    fn cuda_device() {
        let dev = Device::cuda(0, "RTX 4090".to_string(), 24_000_000_000, (8, 9));
        assert_eq!(dev.device_type(), DeviceType::Cuda);
        assert!(dev.is_gpu());
        assert_eq!(dev.compute_capability(), Some((8, 9)));
    }

    #[test]
    fn device_memory_tracking() {
        let dev = Device::cpu();
        dev.record_allocation(1024);
        assert_eq!(dev.memory_used(), 1024);
        dev.record_deallocation(512);
        assert_eq!(dev.memory_used(), 512);
    }

    #[test]
    fn device_manager() {
        let mgr = DeviceManager::new();
        let devices = mgr.list_devices();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].device_type(), DeviceType::Cpu);
    }

    #[test]
    fn backend_binary_op_types() {
        assert_eq!(BinaryOp::Add, BinaryOp::Add);
        assert_ne!(BinaryOp::Add, BinaryOp::Mul);
    }
}
