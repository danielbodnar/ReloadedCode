//! Runtime state for building SerdesAI agents on demand.
//!
//! This module keeps the public runtime API centered on two concepts:
//! - [`AgentRuntime`]: owned data used to build agents later on demand
//! - [`AgentRuntimeBuilder`]: the single obvious assembly path for runtime state
//!
//! Build an [`AgentRuntime`] using [`AgentRuntimeBuilder`]:
//!
//! ```no_run
//! use llm_coding_tools_serdesai::agent_runtime::{AgentDefaults, AgentRuntimeBuilder};
//! use llm_coding_tools_agents::AgentCatalog;
//!
//! let runtime = AgentRuntimeBuilder::new()
//!     .catalog(AgentCatalog::new())
//!     .defaults(AgentDefaults {
//!         model: Some("openai/gpt-4o".to_string()),
//!         temperature: Some(0.7),
//!         top_p: Some(0.9),
//!     })
//!     .build();
//! ```
//!
//! [`AgentCatalog`]: llm_coding_tools_agents::AgentCatalog

mod builder;
mod model;
mod runtime;

pub use builder::AgentRuntimeBuilder;
pub use runtime::{AgentDefaults, AgentRuntime};
