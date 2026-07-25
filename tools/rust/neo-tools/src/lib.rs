//! # Neo Tools
//!
//! Tool ecosystem for the Neo AGI Operating System — providing dynamic tool
//! registration, discovery, versioning, execution, sandboxing, permission
//! management, composition, analytics, and secure external integrations.
//!
//! ## Architecture
//!
//! The tool ecosystem is organized into several layers:
//!
//! - **Core Types**: `ToolId`, `ToolMetadata`, `ToolManifest`, `ToolVersion`, etc.
//! - **Lifecycle**: State machine managing tool states from registration to disposal.
//! - **Registry**: Central concurrent registry with indexing and search.
//! - **Executor**: Execution engine with concurrency control, retries, and streaming.
//! - **Permissions**: Fine-grained access control with rate limiting.
//! - **Sandboxing**: Filesystem, network, and resource isolation.
//! - **Composition**: Pipeline, sequential, parallel, and conditional execution.
//! - **Analytics**: Execution metrics, failure analysis, and performance tracking.
//! - **Health**: Periodic health monitoring and status reporting.
//! - **Events**: Integration with the Neo runtime event bus.
//! - **Persistence**: On-disk storage for manifests, configs, logs, and metrics.
//! - **Built-in Tools**: Filesystem, Shell, Git, HTTP, Browser, Database, Cloud,
//!   Container, IDE, and Networking tool implementations.
//! - **API**: REST-style request/response types.
//! - **SDK**: Fluent builders and convenience utilities.

#![allow(missing_docs)]

pub mod analytics;
pub mod api;
pub mod browser;
pub mod cloud;
pub mod composition;
pub mod container;
pub mod database;
pub mod error;
pub mod event;
pub mod executor;
pub mod filesystem;
pub mod git;
pub mod health;
pub mod http;
pub mod ide;
pub mod lifecycle;
pub mod networking;
pub mod permission;
pub mod persistence;
pub mod registry;
pub mod sandbox;
pub mod sdk;
pub mod shell;
pub mod tool;
pub mod types;

pub use analytics::{AggregateAnalytics, ExecutionRecord, ToolAnalytics};
pub use api::{
    AnalyticsResponse, ApiResponse, ExecuteToolRequest, HealthResponse, MetricsResponse,
    RegisterToolRequest, SearchResponse, ToolDetailResponse, ToolListResponse, ToolSummary,
    UpdateConfigRequest,
};
pub use composition::{
    Composition, CompositionResult, CompositionStep, CompositionStrategy, ToolComposer,
};
pub use error::{ToolError, ToolErrorCode, ToolErrorKind, ToolResult};
pub use event::{ToolEvent, ToolEventLog};
pub use executor::{ExecutionContext, ExecutionQueue, ToolExecutor, ToolExecutorBuilder};
pub use health::{HealthMonitor, HealthSummary};
pub use lifecycle::{LifecycleTracker, ToolLifecycleState};
pub use permission::{PermissionManager, PermissionPolicy, PermissionScope, ToolPermission};
pub use persistence::{PersistenceConfig, ToolPersistence};
pub use registry::{ToolCatalog, ToolFactory, ToolManager, ToolRegistry};
pub use sandbox::{FilesystemSandbox, NetworkSandbox, ResourceLimits, Sandbox, SandboxManager};
pub use sdk::{ToolPresets, ToolSdk};
pub use tool::{DynamicTool, Tool, ToolBuilder};
pub use types::{
    CallerType, ExecutionId, HealthStatus, SandboxConfig, ToolCategory, ToolConfiguration,
    ToolContext, ToolDependency, ToolHealth, ToolId, ToolManifest, ToolMetadata, ToolMetrics,
    ToolRequest, ToolResponse, ToolSnapshot, ToolStatistics, ToolType, ToolVersion,
};
