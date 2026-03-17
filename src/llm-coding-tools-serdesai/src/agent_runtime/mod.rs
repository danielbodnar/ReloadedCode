//! SerdesAI adapter for the generic agent runtime.
//!
//! The data-only runtime foundation lives in [`llm_coding_tools_agents`]. This
//! module re-exports those generic types and adds SerdesAI-specific agent
//! building through [`AgentRuntimeExt`] and [`build_agent_with_credentials`],
//! both of which accept caller-provided model catalogs and credential lookups.
//!
//! # Public API
//! - [`AgentRuntimeExt`] - Builds a runnable SerdesAI agent for the named catalog entry.
//! - [`build_agent_with_credentials`] - Builds with explicit caller-provided credentials.
//! - [`AgentRuntimeTaskExt`] - Builds with conditional Task support.
//! - [`build_agent_with_credentials_and_task`] - Task-enabled build with explicit credentials.

mod build;
mod model;
mod provider_bridge;
mod task;

pub use build::{AgentBuildError, AgentRuntimeExt, build_agent_with_credentials};
pub use llm_coding_tools_agents::{
    AgentDefaults, AgentRuntime, AgentRuntimeBuilder, ModelResolutionError, ResolvedModel,
    ToolCatalogEntry, ToolCatalogKind, default_tools, resolve_model_with_catalog,
};
pub use task::{AgentRuntimeTaskExt, build_agent_with_credentials_and_task};
pub(crate) use task::{TaskBuildContext, build_task_enabled_agent};
