//! # Shared Types
//!
//! Central module for types used across loading and catalog operations.
//!
//! ## Re-exports
//! - Config types: [`AgentConfig`], [`AgentMode`], [`PermissionRule`], [`parse_model_parts`]
//! - Load errors: [`AgentLoadError`], [`AgentLoadResult`]

mod config;
mod error;

pub use config::{parse_model_parts, AgentConfig, AgentMode, PermissionRule};
pub use error::{AgentLoadError, AgentLoadResult};

pub(crate) use config::RawFrontmatter;
