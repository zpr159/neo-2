use serde::{Deserialize, Serialize};

/// Types of compute backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BackendType {
    Cpu,
    Cuda,
    Metal,
    Vulkan,
    Mock,
}

impl std::fmt::Display for BackendType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendType::Cpu => write!(f, "CPU"),
            BackendType::Cuda => write!(f, "CUDA"),
            BackendType::Metal => write!(f, "Metal"),
            BackendType::Vulkan => write!(f, "Vulkan"),
            BackendType::Mock => write!(f, "Mock"),
        }
    }
}

/// An inference backend with availability and priority information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Backend {
    backend_type: BackendType,
    name: String,
    is_available: bool,
    priority: u32,
}

impl Backend {
    /// Creates a new backend instance.
    pub fn new(backend_type: BackendType, name: String, priority: u32) -> Self {
        Self {
            backend_type,
            name,
            is_available: true,
            priority,
        }
    }

    /// Returns whether this backend is currently available.
    pub fn is_available(&self) -> bool {
        self.is_available
    }

    /// Returns the backend type.
    pub fn backend_type(&self) -> BackendType {
        self.backend_type
    }

    /// Returns the backend name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the priority (higher = preferred).
    pub fn priority(&self) -> u32 {
        self.priority
    }
}

/// Probes the system and returns available backends.
pub fn detect_backends() -> Vec<Backend> {
    let mut backends = vec![Backend::new(BackendType::Cpu, "Default CPU Backend".to_string(), 100)];

    // Mock backend for testing
    backends.push(Backend::new(
        BackendType::Mock,
        "Mock Backend".to_string(),
        0,
    ));

    // In a real implementation, probe for CUDA, Metal, Vulkan here.

    backends
}
