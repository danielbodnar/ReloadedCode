//! Generic JIT runtime foundation for agent construction.
//!
//! This module provides framework-agnostic runtime types used for on-demand
//! agent construction. Framework adapters (like `llm-coding-tools-serdesai`)
//! consume these types and add concrete execution/build behavior.
//!
//! # Public API
//!
//! - [`AgentDefaults`] - Runtime-wide fallback settings
//! - [`AgentRuntime`] - Owned runtime state for later agent construction
//! - [`AgentRuntimeBuilder`] - Builder for assembling runtime state
//! - [`ToolCatalogEntry`] - Cloneable metadata for a runtime tool
//! - [`ToolCatalogKind`] - Tool variants supported by the default surface
//! - [`default_tools()`] - Returns the default non-Task tool catalog
//! - [`ResolvedModel`] - A resolved and validated model identifier
//! - [`ModelResolutionError`] - Error type for model resolution failures
//! - [`resolve_model_with_catalog`] - Resolves the effective model for an agent
//!
//! # Usage
//!
//! Build an [`AgentRuntime`] using [`AgentRuntimeBuilder`]:
//!
//! ```no_run
//! use llm_coding_tools_agents::{AgentCatalog, AgentDefaults, AgentRuntimeBuilder};
//!
//! let runtime = AgentRuntimeBuilder::new()
//!     .catalog(AgentCatalog::new())
//!     .defaults(AgentDefaults {
//!         model: Some("openai/gpt-4o".into()),
//!         temperature: Some(0.7),
//!         top_p: Some(0.9),
//!     })
//!     .build();
//!
//! assert!(runtime.catalog().iter().count() == 0);
//! ```

mod builder;
mod model;
mod state;
mod tool_catalog;

pub use builder::AgentRuntimeBuilder;
pub use model::{resolve_model_with_catalog, ModelResolutionError, ResolvedModel};
pub use state::{AgentDefaults, AgentRuntime};
pub use tool_catalog::{default_tools, ToolCatalogEntry, ToolCatalogKind};
