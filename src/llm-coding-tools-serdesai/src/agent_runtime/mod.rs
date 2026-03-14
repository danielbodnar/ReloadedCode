//! SerdesAI adapter for the generic agent runtime.
//!
//! The data-only runtime foundation lives in [`llm_coding_tools_agents`]. This
//! module re-exports those generic types and adds SerdesAI-specific agent
//! materialization through [`AgentRuntimeExt`], which accepts a caller-provided
//! [`llm_coding_tools_core::models::ModelCatalog`].

mod build;
mod model;
mod provider_bridge;

pub use build::{AgentBuildError, AgentRuntimeExt};
pub use llm_coding_tools_agents::{
    AgentDefaults, AgentRuntime, AgentRuntimeBuilder, ModelResolutionError, ResolvedModel,
    ToolCatalogEntry, ToolCatalogKind, default_tools, resolve_model_with_catalog,
};
