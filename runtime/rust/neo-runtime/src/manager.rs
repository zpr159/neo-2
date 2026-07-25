//! Runtime manager orchestrating startup, shutdown, initialization, dependency
//! ordering, runtime state, and graceful termination.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::async_runtime::NeoAsyncRuntime;
use crate::config::{HotReloadConfig as ConfigHotReload, RuntimeConfiguration, RuntimeProfile};
use crate::dependency::{Dependency, DependencyGraph, VersionConstraint};
use crate::error::{RuntimeError, RuntimeErrorKind, RecoveryAction};
use crate::event_bus::EventBus;
use crate::health::HealthMonitor;
use crate::lifecycle::{LifecycleManager, ServiceId, ServiceState};
use crate::message_bus::MessageBus;
use crate::performance::PerformanceMonitor;
use crate::plugin::{PluginLoader, PluginSandboxConfig, HotReloadConfig as PluginHotReload};
use crate::resource::ResourceManager;
use crate::scheduler::TaskScheduler;
use crate::thread_pool::ThreadPool;

/// Overall state of the runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuntimeState {
    /// Runtime has not been started.
    Uninitialized,
    /// Runtime is initializing subsystems.
    Initializing,
    /// Runtime is starting up services.
    Starting,
    /// Runtime is fully operational.
    Running,
    /// Runtime is pausing.
    Pausing,
    /// Runtime is paused.
    Paused,
    /// Runtime is shutting down.
    ShuttingDown,
    /// Runtime has stopped.
    Stopped,
    /// Runtime encountered a fatal error.
    Failed,
}

impl RuntimeState {
    /// Check whether the runtime can transition to the target state.
    pub fn can_transition_to(self, target: RuntimeState) -> bool {
        matches!(
            (self, target),
            (Self::Uninitialized, Self::Initializing)
                | (Self::Initializing, Self::Starting)
                | (Self::Starting, Self::Running)
                | (Self::Running, Self::Pausing)
                | (Self::Running, Self::ShuttingDown)
                | (Self::Pausing, Self::Paused)
                | (Self::Paused, Self::Running)
                | (Self::Paused, Self::ShuttingDown)
                | (Self::ShuttingDown, Self::Stopped)
                | (_, Self::Failed)
        )
    }
}

impl std::fmt::Display for RuntimeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Uninitialized => write!(f, "uninitialized"),
            Self::Initializing => write!(f, "initializing"),
            Self::Starting => write!(f, "starting"),
            Self::Running => write!(f, "running"),
            Self::Pausing => write!(f, "pausing"),
            Self::Paused => write!(f, "paused"),
            Self::ShuttingDown => write!(f, "shutting_down"),
            Self::Stopped => write!(f, "stopped"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

impl Default for RuntimeState {
    fn default() -> Self {
        Self::Uninitialized
    }
}

/// Configuration for a service to be registered with the runtime.
pub struct ServiceRegistration {
    pub name: String,
    pub version: (u32, u32, u32),
    pub dependencies: Vec<ServiceDependency>,
    pub optional_dependencies: Vec<ServiceDependency>,
    pub priority: u32,
}

/// A dependency declaration for a service.
pub struct ServiceDependency {
    pub name: String,
    pub version_constraint: VersionConstraint,
    pub optional: bool,
}

/// Statistics about the runtime.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuntimeStatistics {
    pub state: RuntimeState,
    pub uptime_ms: u64,
    pub services_registered: usize,
    pub services_running: usize,
    pub tasks_scheduled: u64,
    pub events_published: u64,
    pub plugins_loaded: usize,
    pub health_status: String,
}

/// The Neo Runtime Manager — the backbone of the operating system.
///
/// Orchestrates all subsystems: lifecycle, dependencies, resources,
/// scheduling, events, messaging, plugins, health, and performance.
pub struct RuntimeManager {
    state: RwLock<RuntimeState>,
    config: RwLock<RuntimeConfiguration>,
    lifecycle: LifecycleManager,
    dependencies: RwLock<DependencyGraph>,
    resource_manager: ResourceManager,
    scheduler: RwLock<TaskScheduler>,
    thread_pool: RwLock<Option<ThreadPool>>,
    async_runtime: RwLock<Option<NeoAsyncRuntime>>,
    event_bus: RwLock<EventBus>,
    message_bus: RwLock<MessageBus>,
    plugin_loader: RwLock<Option<PluginLoader>>,
    health_monitor: RwLock<HealthMonitor>,
    performance_monitor: RwLock<PerformanceMonitor>,
    config_hot_reload: RwLock<Option<ConfigHotReload>>,
    start_time: RwLock<Option<Instant>>,
    running: AtomicBool,
}

impl RuntimeManager {
    /// Create a new runtime manager with the given configuration.
    pub fn new(config: RuntimeConfiguration) -> Self {
        let lifecycle = LifecycleManager::new();
        let resource_manager = ResourceManager::new();
        let health_monitor = HealthMonitor::new(config.health.clone());
        let performance_monitor = PerformanceMonitor::new(config.performance.clone());
        let event_bus = EventBus::new(config.event_bus.clone());
        let message_bus = MessageBus::new(config.message_bus.clone());
        let scheduler = TaskScheduler::new(config.scheduler.clone());

        Self {
            state: RwLock::new(RuntimeState::Uninitialized),
            config: RwLock::new(config),
            lifecycle,
            dependencies: RwLock::new(DependencyGraph::new()),
            resource_manager,
            scheduler: RwLock::new(scheduler),
            thread_pool: RwLock::new(None),
            async_runtime: RwLock::new(None),
            event_bus: RwLock::new(event_bus),
            message_bus: RwLock::new(message_bus),
            plugin_loader: RwLock::new(None),
            health_monitor: RwLock::new(health_monitor),
            performance_monitor: RwLock::new(performance_monitor),
            config_hot_reload: RwLock::new(None),
            start_time: RwLock::new(None),
            running: AtomicBool::new(false),
        }
    }

    /// Create a runtime manager for the development profile.
    pub fn development() -> Self {
        Self::new(RuntimeConfiguration::development())
    }

    /// Create a runtime manager for the testing profile.
    pub fn testing() -> Self {
        Self::new(RuntimeConfiguration::testing())
    }

    /// Create a runtime manager for the production profile.
    pub fn production() -> Self {
        Self::new(RuntimeConfiguration::production())
    }

    /// Transition the runtime to a new state.
    pub fn transition_state(&self, target: RuntimeState) -> Result<(), RuntimeError> {
        let current = *self.state.read();
        if !current.can_transition_to(target) {
            return Err(RuntimeError::new(
                RuntimeErrorKind::Lifecycle(crate::error::LifecycleErrorKind::InvalidTransition),
                format!("cannot transition runtime from {} to {}", current, target),
            ));
        }
        *self.state.write() = target;
        Ok(())
    }

    /// Get the current runtime state.
    pub fn state(&self) -> RuntimeState {
        *self.state.read()
    }

    /// Initialize the runtime: create subsystems from configuration.
    pub fn initialize(&self) -> Result<(), RuntimeError> {
        self.transition_state(RuntimeState::Initializing)?;

        let config = self.config.read().clone();

        let thread_pool = ThreadPool::new(config.thread_pool.clone());
        *self.thread_pool.write() = Some(thread_pool);

        let async_rt = NeoAsyncRuntime::new(config.scheduler.max_concurrent_tasks)?;
        *self.async_runtime.write() = Some(async_rt);

        let plugin_loader = PluginLoader::from_config(&config.plugin);
        *self.plugin_loader.write() = Some(plugin_loader);

        let hot = ConfigHotReload::new(config.clone());
        *self.config_hot_reload.write() = Some(hot);

        // Register default resource pools.
        self.resource_manager
            .register_pool(crate::resource::ResourceType::Cpu, config.resources.total_cpu_units);
        self.resource_manager
            .register_pool(crate::resource::ResourceType::Gpu, config.resources.total_gpu_units);
        self.resource_manager
            .register_pool(crate::resource::ResourceType::Ram, config.resources.total_ram_bytes);
        self.resource_manager
            .register_pool(crate::resource::ResourceType::Disk, config.resources.total_disk_bytes);

        self.transition_state(RuntimeState::Starting)?;
        Ok(())
    }

    /// Start the runtime and all registered services in dependency order.
    pub fn start(&self) -> Result<(), RuntimeError> {
        if self.state() == RuntimeState::Uninitialized {
            self.initialize()?;
        }

        if self.state() != RuntimeState::Starting {
            self.transition_state(RuntimeState::Starting)?;
        }

        let order = {
            let deps = self.dependencies.read();
            deps.topological_sort().unwrap_or_else(|_| deps.node_ids())
        };

        for id in order {
            let _ = self.lifecycle.transition(
                id,
                ServiceState::Starting,
                "runtime starting",
            );
            let _ = self.lifecycle.transition(
                id,
                ServiceState::Running,
                "runtime started",
            );
        }

        self.running.store(true, Ordering::SeqCst);
        *self.start_time.write() = Some(Instant::now());
        self.transition_state(RuntimeState::Running)?;

        tracing::info!("neo runtime started");
        Ok(())
    }

    /// Shut down the runtime gracefully, stopping services in reverse order.
    pub fn shutdown(&self) -> Result<(), RuntimeError> {
        if self.state() == RuntimeState::Stopped || self.state() == RuntimeState::Failed {
            return Ok(());
        }

        self.transition_state(RuntimeState::ShuttingDown)?;
        tracing::info!("neo runtime shutting down");

        let order = {
            let deps = self.dependencies.read();
            let mut ids = deps.node_ids();
            ids.reverse();
            ids
        };

        for id in order {
            let _ = self.lifecycle.transition(
                id,
                ServiceState::Stopping,
                "runtime shutting down",
            );
            let _ = self.lifecycle.transition(
                id,
                ServiceState::Stopped,
                "runtime stopped",
            );
        }

        if let Some(tp) = self.thread_pool.read().as_ref() {
            tp.shutdown();
        }

        self.health_monitor.read().shutdown();
        self.running.store(false, Ordering::SeqCst);

        self.transition_state(RuntimeState::Stopped)?;
        tracing::info!("neo runtime stopped");
        Ok(())
    }

    /// Pause the runtime.
    pub fn pause(&self) -> Result<(), RuntimeError> {
        self.transition_state(RuntimeState::Pausing)?;
        let order: Vec<ServiceId> = {
            self.lifecycle
                .list_services()
                .iter()
                .filter(|(_, _, s)| *s == ServiceState::Running)
                .map(|(id, _, _)| *id)
                .collect()
        };

        for id in order {
            let _ = self.lifecycle.transition(id, ServiceState::Paused, "runtime pausing");
        }

        self.transition_state(RuntimeState::Paused)?;
        Ok(())
    }

    /// Resume the runtime from a paused state.
    pub fn resume(&self) -> Result<(), RuntimeError> {
        let order: Vec<ServiceId> = {
            self.lifecycle
                .list_services()
                .iter()
                .filter(|(_, _, s)| *s == ServiceState::Paused)
                .map(|(id, _, _)| *id)
                .collect()
        };

        for id in order {
            let _ = self.lifecycle.transition(id, ServiceState::Running, "runtime resuming");
        }

        self.transition_state(RuntimeState::Running)?;
        Ok(())
    }

    /// Register a service with the runtime.
    pub fn register_service(&self, registration: ServiceRegistration) -> ServiceId {
        let id = self.lifecycle.register_service(&registration.name);

        self.dependencies.write().add_node(
            id,
            &registration.name,
            registration.version,
        );

        for dep in &registration.dependencies {
            if let Some(dep_id) = self.dependencies.read().find_by_name(&dep.name) {
                self.dependencies.write().add_dependency(
                    id,
                    Dependency {
                        service_id: dep_id,
                        service_name: dep.name.clone(),
                        version_constraint: dep.version_constraint.clone(),
                        optional: false,
                    },
                );
            }
        }

        for dep in &registration.optional_dependencies {
            if let Some(dep_id) = self.dependencies.read().find_by_name(&dep.name) {
                self.dependencies.write().add_dependency(
                    id,
                    Dependency {
                        service_id: dep_id,
                        service_name: dep.name.clone(),
                        version_constraint: dep.version_constraint.clone(),
                        optional: true,
                    },
                );
            }
        }

        let _ = self.lifecycle.transition(
            id,
            ServiceState::Registered,
            "service registered",
        );

        id
    }

    /// Validate all dependencies and detect cycles.
    pub fn validate_dependencies(&self) -> Result<(), RuntimeError> {
        let graph = self.dependencies.read();
        if let Some(cycle) = graph.detect_cycle() {
            let path: Vec<String> = cycle
                .iter()
                .filter_map(|id| graph.node(*id).map(|n| n.name.clone()))
                .collect();
            return Err(RuntimeError::dependency(format!(
                "circular dependency: {}",
                path.join(" -> ")
            )));
        }

        let errors = graph.validate();
        if let Some((_, err)) = errors.into_iter().next() {
            return Err(RuntimeError::dependency(err.message));
        }

        Ok(())
    }

    /// Get the dependency resolution order.
    pub fn dependency_order(&self) -> Vec<ServiceId> {
        self.dependencies
            .read()
            .topological_sort()
            .unwrap_or_default()
    }

    /// Get a reference to the lifecycle manager.
    pub fn lifecycle(&self) -> &LifecycleManager {
        &self.lifecycle
    }

    /// Get a reference to the resource manager.
    pub fn resource_manager(&self) -> &ResourceManager {
        &self.resource_manager
    }

    /// Get a read guard on the health monitor.
    pub fn health_monitor(&self) -> parking_lot::RwLockReadGuard<'_, HealthMonitor> {
        self.health_monitor.read()
    }

    /// Get a read guard on the performance monitor.
    pub fn performance_monitor(&self) -> parking_lot::RwLockReadGuard<'_, PerformanceMonitor> {
        self.performance_monitor.read()
    }

    /// Get a read guard on the event bus.
    pub fn event_bus(&self) -> parking_lot::RwLockReadGuard<'_, EventBus> {
        self.event_bus.read()
    }

    /// Get a read guard on the message bus.
    pub fn message_bus(&self) -> parking_lot::RwLockReadGuard<'_, MessageBus> {
        self.message_bus.read()
    }

    /// Get a write guard on the scheduler.
    pub fn scheduler_mut(&self) -> parking_lot::RwLockWriteGuard<'_, TaskScheduler> {
        self.scheduler.write()
    }

    /// Get the runtime configuration.
    pub fn configuration(&self) -> RuntimeConfiguration {
        self.config.read().clone()
    }

    /// Check whether the runtime is currently running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Get runtime statistics.
    pub fn statistics(&self) -> RuntimeStatistics {
        let start = self.start_time.read().unwrap_or_else(|| Instant::now());
        let services = self.lifecycle.list_services();
        let running = services
            .iter()
            .filter(|(_, _, s)| *s == ServiceState::Running)
            .count();

        RuntimeStatistics {
            state: self.state(),
            uptime_ms: start.elapsed().as_millis() as u64,
            services_registered: services.len(),
            services_running: running,
            tasks_scheduled: self.scheduler.read().statistics().tasks_submitted,
            events_published: self.event_bus.read().statistics().events_published,
            plugins_loaded: self
                .plugin_loader
                .read()
                .as_ref()
                .map_or(0, |p| p.statistics().active_plugins),
            health_status: format!("{:?}", self.health_monitor.read().summary()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_state_transitions() {
        assert!(RuntimeState::Uninitialized.can_transition_to(RuntimeState::Initializing));
        assert!(RuntimeState::Initializing.can_transition_to(RuntimeState::Starting));
        assert!(RuntimeState::Starting.can_transition_to(RuntimeState::Running));
        assert!(RuntimeState::Running.can_transition_to(RuntimeState::ShuttingDown));
        assert!(RuntimeState::Running.can_transition_to(RuntimeState::Pausing));
        assert!(RuntimeState::Paused.can_transition_to(RuntimeState::Running));
        assert!(RuntimeState::ShuttingDown.can_transition_to(RuntimeState::Stopped));
        assert!(!RuntimeState::Stopped.can_transition_to(RuntimeState::Running));
        assert!(RuntimeState::Running.can_transition_to(RuntimeState::Failed));
    }

    #[test]
    fn runtime_creation() {
        let rt = RuntimeManager::testing();
        assert_eq!(rt.state(), RuntimeState::Uninitialized);
        assert!(!rt.is_running());
    }

    #[test]
    fn runtime_initialize() {
        let rt = RuntimeManager::testing();
        rt.initialize().unwrap();
        assert_eq!(rt.state(), RuntimeState::Starting);
    }

    #[test]
    fn runtime_start_and_shutdown() {
        let rt = RuntimeManager::testing();
        rt.start().unwrap();
        assert_eq!(rt.state(), RuntimeState::Running);
        assert!(rt.is_running());

        rt.shutdown().unwrap();
        assert_eq!(rt.state(), RuntimeState::Stopped);
        assert!(!rt.is_running());
    }

    #[test]
    fn register_and_list_services() {
        let rt = RuntimeManager::testing();
        rt.initialize().unwrap();

        let id1 = rt.register_service(ServiceRegistration {
            name: "service-a".to_string(),
            version: (1, 0, 0),
            dependencies: Vec::new(),
            optional_dependencies: Vec::new(),
            priority: 0,
        });

        let id2 = rt.register_service(ServiceRegistration {
            name: "service-b".to_string(),
            version: (1, 0, 0),
            dependencies: Vec::new(),
            optional_dependencies: Vec::new(),
            priority: 0,
        });

        let services = rt.lifecycle().list_services();
        assert_eq!(services.len(), 2);

        rt.shutdown().unwrap();
    }

    #[test]
    fn dependency_ordering() {
        let rt = RuntimeManager::testing();
        rt.initialize().unwrap();

        let id_a = rt.register_service(ServiceRegistration {
            name: "a".to_string(),
            version: (1, 0, 0),
            dependencies: Vec::new(),
            optional_dependencies: Vec::new(),
            priority: 0,
        });

        let id_b = rt.register_service(ServiceRegistration {
            name: "b".to_string(),
            version: (1, 0, 0),
            dependencies: vec![ServiceDependency {
                name: "a".to_string(),
                version_constraint: VersionConstraint::Any,
                optional: false,
            }],
            optional_dependencies: Vec::new(),
            priority: 0,
        });

        let order = rt.dependency_order();
        assert!(order.len() >= 2);
        let pos_a = order.iter().position(|&id| id == id_a);
        let pos_b = order.iter().position(|&id| id == id_b);
        assert!(pos_a.is_some());
        assert!(pos_b.is_some());
        assert!(pos_a.unwrap() < pos_b.unwrap());

        rt.shutdown().unwrap();
    }

    #[test]
    fn pause_and_resume() {
        let rt = RuntimeManager::testing();
        rt.start().unwrap();

        rt.register_service(ServiceRegistration {
            name: "svc".to_string(),
            version: (1, 0, 0),
            dependencies: Vec::new(),
            optional_dependencies: Vec::new(),
            priority: 0,
        });

        rt.pause().unwrap();
        assert_eq!(rt.state(), RuntimeState::Paused);

        rt.resume().unwrap();
        assert_eq!(rt.state(), RuntimeState::Running);

        rt.shutdown().unwrap();
    }

    #[test]
    fn statistics() {
        let rt = RuntimeManager::testing();
        rt.start().unwrap();
        let stats = rt.statistics();
        assert_eq!(stats.state, RuntimeState::Running);
        rt.shutdown().unwrap();
    }

    #[test]
    fn production_profile() {
        let rt = RuntimeManager::production();
        assert_eq!(rt.state(), RuntimeState::Uninitialized);
        assert_eq!(
            rt.configuration().profile,
            RuntimeProfile::Production
        );
    }
}
