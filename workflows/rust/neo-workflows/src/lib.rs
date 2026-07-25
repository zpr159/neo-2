//! # Neo Workflows
//!
//! Production-grade workflow orchestration engine for the Neo AGI Operating System.
//!
//! This crate provides a complete workflow execution framework supporting:
//! - Sequential, parallel, and DAG-based execution
//! - Conditional branching, loops, and sub-workflows
//! - Retry policies with exponential backoff
//! - Rollback and compensation actions
//! - Checkpointing and persistence
//! - Event-driven execution
//! - Comprehensive analytics
//! - Scheduling and variable management
//! - SDK for building workflows programmatically
//! - REST/CLI API types

pub mod analytics;
pub mod api;
pub mod checkpoint;
pub mod core;
pub mod dag;
pub mod definition;
pub mod error;
pub mod event;
pub mod execution;
pub mod integration;
pub mod rollback;
pub mod schedule;
pub mod sdk;
pub mod state_machine;
pub mod variable;

// Re-exports for convenience
pub use core::*;
pub use definition::*;
pub use error::*;
pub use execution::{WorkflowExecutor, WorkflowInstance};
pub use sdk::WorkflowBuilder;
