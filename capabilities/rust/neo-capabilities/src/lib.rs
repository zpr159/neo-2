//! # Neo Capabilities
//!
//! The capability framework for the Neo AGI Operating System.
//!
//! A Capability is any reusable action Neo can perform. Everything Neo can
//! do—from summarization to code generation, image analysis, planning, search,
//! scheduling, file management and future robotics—is represented as a capability.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                     CapabilityApi                                │
//! │  (register, unregister, execute, inspect, list, search,        │
//! │   enable, disable, export, import)                              │
//! ├──────────────┬──────────────┬──────────────┬───────────────────┤
//! │  Capability  │  Execution   │ Composition  │   Discovery       │
//! │  Registry    │  Engine      │ Engine       │   Engine          │
//! │ (register,   │ (context,    │ (sequential, │ (filesystem,      │
//! │  lifecycle,  │  pipeline,   │  parallel,   │  plugin, remote,  │
//! │  versioning) │  retry,      │  conditional,│  hot reload)      │
//! │              │  timeout,    │  fallback)   │                   │
//! │              │  streaming)  │              │                   │
//! ├──────────────┴──────────────┴──────────────┴───────────────────┤
//! │  Permission       │  Resource         │  Analytics             │
//! │  Manager          │  Manager          │  Engine                │
//! │  (RBAC, sandbox,  │  (CPU, GPU, mem,  │  (exec count,         │
//! │   approval,       │   inference,      │   success rate,       │
//! │   audit)          │   quotas)         │   latency, popularity)│
//! ├───────────────────┴───────────────────┴───────────────────────┤
//! │  Marketplace      │  Integration       │  SDK                   │
//! │  (manifest,       │  (executive,       │  (builder, plugin     │
//! │   signing,        │   reasoning,       │   authoring,          │
//! │   install)        │   memory, knowledge│   foreign hooks)      │
//! └───────────────────┴────────────────────┴──────────────────────┘
//! ```

#![allow(missing_docs)]

pub mod error;
pub mod core;
pub mod discovery;
pub mod execution;
pub mod types;
pub mod composition;
pub mod permission;
pub mod resource;
pub mod marketplace;
pub mod analytics;
pub mod api;
pub mod registry;
pub mod integration;
pub mod sdk;

// Re-exports for convenience
pub use error::{CapabilityError, CapabilityResult, CapabilityErrorCode};
pub use core::{
    Capability, CapabilityId, CapabilityMetadata, CapabilityVersion, CapabilityState,
    CapabilityCategory, CapabilityNamespace, CapabilityTags, CapabilityAliases,
    ExecutionContext, CancellationToken, CapabilityResult_output, ProgressUpdate,
    StreamChunk, ResourceRequirements, InputSchema, OutputSchema, CapabilityEntry,
    CapabilitySummary,
};
pub use discovery::{
    DiscoveryEngine, DiscoveryStrategy, CapabilitySource, CapabilityManifest,
    ManifestDependency, ConflictType, HotReloadEvent,
};
pub use execution::{
    ExecutionMode, RetryConfig, TimeoutConfig, ExecutionRequest, ExecutionRecord,
    ExecutionStatus, ExecutionPipeline, PipelineStep, StreamingOutput, CapabilityExecutor,
};
pub use types::{
    ReasoningCapability, InferenceCapability, MemoryCapability, MemoryOperation,
    KnowledgeCapability, KnowledgeOperation, ToolCapability, WorkflowCapability,
    CommunicationCapability, FilesystemCapability, FsOperation, NetworkCapability,
    DeveloperCapability, SystemCapability, SystemOperation, CustomCapability,
    CapabilityTypeRegistry, create_builtin_capabilities,
};
pub use composition::{
    CompositionStrategy, CompositionStep, FailureAction, CompositionTemplate,
    ComposedCapability, CompositionRegistry,
};
pub use permission::{
    Role, RolePermissions, AllowDenyList, SandboxConfig, Sandbox,
    ApprovalRequest, ApprovalStatus, ApprovalManager,
    AuditEntry, AuditAction, AuditLog, CapabilityPermissionManager,
};
pub use resource::{
    ResourceType as CapabilityResourceType, ResourceBudget, ExecutionQuota,
    ResourcePool, ResourcePoolStatus, ResourceManager, BudgetStatus,
    AllocationRecord,
};
pub use marketplace::{
    MarketplaceManifest, SigningInfo, SignatureAlgorithm, CapabilitySignature,
    VersionCompatibility, InstallationRecord, InstallationSource, InstallationState,
    HookType, InstallationHook, Marketplace,
};
pub use analytics::{
    ExecutionMetric, LatencyStats, CapabilityAnalytics, CapabilityAnalyticsStore,
    GlobalStats, SortMetric,
};
pub use api::CapabilityApi;
pub use registry::{CapabilityRegistry, RegistrySummary};
pub use integration::{
    ExecutiveIntegration, ReasoningIntegration, MemoryIntegration,
    KnowledgeIntegration, CliIntegration, CapabilityIntegrator,
    TaskCapabilityLink, SelectionRecord, CapabilityRelationship,
};
pub use sdk::{
    CapabilityBuilder, PluginManifest, PluginCapability,
    PluginAuthoringKit, ForeignLanguageHook, ForeignLanguage, SdkRegistry,
};
