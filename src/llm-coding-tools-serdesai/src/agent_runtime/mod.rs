//! SerdesAI adapter for the generic agent runtime.
//!
//! The data-only runtime foundation lives in [`llm_coding_tools_agents`]. This
//! module re-exports those generic types and adds SerdesAI-specific agent
//! building through [`AgentRuntimeExt`] and [`build_agent_with_credentials`],
//! both of which accept a caller-provided [`llm_coding_tools_core::models::ModelCatalog`].

mod build;
mod model;
mod provider_bridge;

pub use build::{AgentBuildError, AgentRuntimeExt, build_agent_with_credentials};
pub use llm_coding_tools_agents::{
    AgentDefaults, AgentRuntime, AgentRuntimeBuilder, ModelResolutionError, ResolvedModel,
    ToolCatalogEntry, ToolCatalogKind, default_tools, resolve_model_with_catalog,
};
