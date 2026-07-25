//! # Neo Runtime
//!
//! The execution backbone of the Neo AGI Operating System.
//!
//! Every subsystem executes through this runtime. Nothing executes directly.
//! Every component must be registered. Every service must have a lifecycle.
//! Every task must be scheduled. Every resource must be tracked.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │                   Runtime Manager                    │
//! │  (startup, shutdown, initialization, state)         │
//! ├──────────┬──────────┬──────────┬──────────┬─────────┤
//! │ Lifecycle│ Dependency│ Resource │ Scheduler │ Health  │
//! │ Manager  │ Resolver  │ Manager  │          │ Monitor │
//! ├──────────┼──────────┼──────────┼──────────┼─────────┤
//! │ Thread   │ Async    │ Event    │ Message  │ Plugin  │
//! │ Pool     │ Runtime  │ Bus      │ Bus      │ Loader  │
//! ├──────────┴──────────┴──────────┴──────────┴─────────┤
//! │              Performance Monitor                     │
//! └─────────────────────────────────────────────────────┘
//! ```

pub mod error;
pub mod config;
pub mod lifecycle;
pub mod dependency;
pub mod resource;
pub mod thread_pool;
pub mod async_runtime;
pub mod scheduler;
pub mod event_bus;
pub mod message_bus;
pub mod plugin;
pub mod health;
pub mod performance;
pub mod manager;
pub mod process;
pub mod memory;
pub mod sandbox;

pub use error::{
    RuntimeError, RuntimeErrorKind, RecoveryAction,
    LifecycleError, LifecycleErrorKind,
    SchedulerError, SchedulerErrorKind,
    DependencyError, DependencyErrorKind,
    PluginError, PluginErrorKind,
    ResourceError, ResourceErrorKind,
    TimeoutError, TimeoutErrorKind,
    RuntimeResult,
};
pub use config::{
    RuntimeConfiguration, RuntimeProfile, HotReloadConfig,
    ThreadPoolConfig, SchedulerConfig, ResourceManagerConfig,
    EventBusConfig, MessageBusConfig, PluginConfig,
    HealthConfig, PerformanceConfig,
};
pub use lifecycle::{ServiceId, ServiceState, StateTransition, LifecycleManager};
pub use dependency::{DependencyGraph, Dependency, VersionConstraint};
pub use resource::{ResourceManager, ResourceType, ResourceHandle, ConsumerId, MemoryPool};
pub use thread_pool::{ThreadPool, ThreadPoolStatistics, TaskPriority as ThreadPoolTaskPriority};
pub use async_runtime::{NeoAsyncRuntime, CancellationToken, Backpressure, StructuredScope};
pub use scheduler::{
    TaskScheduler, ScheduledTask, ScheduledTaskId, TaskPriority,
    TaskStatus, SchedulerStatistics,
};
pub use event_bus::{EventBus, Event, EventId, EventFilter, EventPriority, SubscriptionId};
pub use message_bus::{MessageBus, Message, MessageId};
pub use plugin::{
    PluginLoader, PluginId, PluginDescriptor, PluginState,
    PluginVerifier, PluginSandbox,
};
pub use health::{HealthMonitor, MonitorId, HealthStatus, Alert, AlertSeverity};
pub use performance::{PerformanceMonitor, PerformanceSnapshot};
pub use manager::{RuntimeManager, RuntimeState, RuntimeStatistics, ServiceRegistration};
pub use process::{Process, ProcessId, ProcessState, ProcessManager};
pub use memory::{MemoryManager, MemoryRegion, MemoryProtection};
pub use sandbox::{Sandbox, SandboxConfig, SandboxLevel};
