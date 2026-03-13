//! Builder for assembling owned agent runtime state.
//!
//! # Public API
//!
//! - [`AgentRuntimeBuilder`]: Builder for constructing [`AgentRuntime`] with
//!   custom catalog, defaults, and tool catalog.

use super::state::{AgentDefaults, AgentRuntime};
use super::tool_catalog::{default_tools, ToolCatalogEntry};
use crate::AgentCatalog;

/// Single assembly path for owned runtime state.
#[derive(Debug, Clone)]
pub struct AgentRuntimeBuilder {
    catalog: AgentCatalog,
    defaults: AgentDefaults,
    tools: Vec<ToolCatalogEntry>,
}

impl Default for AgentRuntimeBuilder {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl AgentRuntimeBuilder {
    /// Creates a builder seeded with empty catalog/defaults and the default tool catalog.
    #[inline]
    pub fn new() -> Self {
        Self {
            catalog: AgentCatalog::new(),
            defaults: AgentDefaults::default(),
            tools: default_tools(),
        }
    }

    /// Replaces the owned parsed catalog.
    #[inline]
    pub fn catalog(mut self, catalog: AgentCatalog) -> Self {
        self.catalog = catalog;
        self
    }

    /// Replaces the owned runtime defaults.
    #[inline]
    pub fn defaults(mut self, defaults: AgentDefaults) -> Self {
        self.defaults = defaults;
        self
    }

    /// Replaces the owned tool catalog metadata.
    #[inline]
    pub fn tools(mut self, tools: Vec<ToolCatalogEntry>) -> Self {
        self.tools = tools;
        self
    }

    /// Consumes the builder and returns one owned runtime.
    #[inline]
    pub fn build(self) -> AgentRuntime {
        AgentRuntime::from_parts(self.catalog, self.defaults, self.tools)
    }
}

#[cfg(test)]
mod tests {
    use super::AgentRuntimeBuilder;
    use crate::runtime::tool_catalog::{default_tools, ToolCatalogEntry, ToolCatalogKind};
    use crate::runtime::AgentDefaults;
    use crate::{AgentCatalog, AgentConfig, AgentMode};
    use llm_coding_tools_core::tool_names;

    fn sample_config(name: &str, model: Option<&str>) -> AgentConfig {
        AgentConfig {
            name: name.into(),
            mode: AgentMode::Subagent,
            description: format!("{name} description").into(),
            model: model.map(Into::into),
            hidden: false,
            temperature: Some(0.3),
            top_p: Some(0.8),
            permission: Default::default(),
            options: Default::default(),
            prompt: format!("You are {name}.").into(),
        }
    }

    #[test]
    fn builder_builds_runtime_from_owned_inputs() {
        let catalog = AgentCatalog::from_entries([sample_config("planner", Some("openai/gpt-4o"))]);
        let defaults = AgentDefaults {
            model: Some("openai/gpt-4.1-mini".into()),
            temperature: Some(0.2),
            top_p: Some(0.95),
        };
        let tools = vec![
            ToolCatalogEntry::new(tool_names::READ, ToolCatalogKind::Read),
            ToolCatalogEntry::new(tool_names::GLOB, ToolCatalogKind::Glob),
        ];

        let runtime = AgentRuntimeBuilder::new()
            .catalog(catalog)
            .defaults(defaults.clone())
            .tools(tools.clone())
            .build();

        assert_eq!(
            runtime
                .catalog()
                .by_name("planner")
                .and_then(|config| config.model.as_deref()),
            Some("openai/gpt-4o"),
        );
        assert_eq!(runtime.defaults(), &defaults);
        assert_eq!(runtime.tools(), tools.as_slice());
    }

    #[test]
    fn builder_defaults_to_empty_catalog_defaults_and_default_tools() {
        let runtime = AgentRuntimeBuilder::new().build();

        assert_eq!(runtime.catalog().iter().count(), 0);
        assert_eq!(runtime.defaults(), &AgentDefaults::default());
        assert_eq!(runtime.tools(), default_tools().as_slice());
    }
}
