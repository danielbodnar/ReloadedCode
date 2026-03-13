#![doc = include_str!(concat!("../", env!("CARGO_PKG_README")))]

mod catalog;
mod extensions;
mod loader;
mod parser;
mod runtime;
mod types;

pub use catalog::AgentCatalog;
pub use extensions::RulesetExt;
pub use loader::AgentLoader;
pub use parser::AgentParseError;
pub use runtime::{
    default_tools, resolve_model_with_catalog, AgentDefaults, AgentRuntime, AgentRuntimeBuilder,
    ModelResolutionError, ResolvedModel, ToolCatalogEntry, ToolCatalogKind,
};
pub use types::{
    parse_model_parts, AgentConfig, AgentLoadError, AgentLoadResult, AgentMode, PermissionRule,
};
