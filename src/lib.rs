//! # Neo Core
//!
//! Core primitives and type system for the Neo AGI Operating System.
//!
//! This crate provides the foundational types, error handling, configuration,
//! component lifecycle, event system, resource management, and identity
//! primitives used by every other subsystem in Neo.
//!
//! ## Architecture
//!
//! The core crate is the lowest-level Rust crate in the Neo monorepo. It has
//! zero dependencies on other Neo crates, forming the base of the dependency
//! graph. All other Neo crates depend on `neo-core`.

pub mod error;
pub mod types;
pub mod config;
pub mod component;
pub mod event;
pub mod resource;
pub mod id;
pub mod time;
pub mod language;
pub mod conversation;
pub mod api;
pub mod observability;
pub mod security;
pub mod plugins;
pub mod research;
pub mod rest;
pub mod sdk;
pub mod cli;

pub use error::{NeoError, NeoResult};
pub use id::{AgentId, ComponentId, TaskId};
pub use types::{Severity, Environment};
