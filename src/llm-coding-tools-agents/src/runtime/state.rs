//! Holds your loaded agents, default settings, Task settings, and available tools.
//!
//! ## Public API
//!
//! - [`AgentRuntime`] — Container for loaded agents, defaults, Task settings, and tools.
//! - [`AgentDefaults`] — Fallback settings when an agent doesn't specify them.

use super::tool_catalog::ToolCatalogEntry;
use crate::AgentCatalog;
use llm_coding_tools_core::TaskSettings;

/// Default settings used when an agent doesn't specify them.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AgentDefaults {
    /// Default model in `provider/model-id` format.
    pub model: Option<Box<str>>,
    /// Default sampling temperature.
    pub temperature: Option<f32>,
    /// Default nucleus sampling top-p.
    pub top_p: Option<f32>,
}

impl AgentDefaults {
    /// Creates defaults with only a model specified; temperature and top_p inherit provider defaults.
    #[inline]
    pub fn with_model(model: impl Into<Box<str>>) -> Self {
        Self {
            model: Some(model.into()),
            temperature: None,
            top_p: None,
        }
    }
}

/// Your loaded agents plus their default settings, Task settings, and available tools.
#[derive(Debug, Clone)]
pub struct AgentRuntime {
    catalog: AgentCatalog,
    defaults: AgentDefaults,
    task_settings: TaskSettings,
    tools: Vec<ToolCatalogEntry>,
}

impl AgentRuntime {
    #[inline]
    pub(super) fn from_parts(
        catalog: AgentCatalog,
        defaults: AgentDefaults,
        task_settings: TaskSettings,
        tools: Vec<ToolCatalogEntry>,
    ) -> Self {
        Self {
            catalog,
            defaults,
            task_settings,
            tools,
        }
    }

    /// Returns the loaded agent definitions.
    #[inline]
    pub fn catalog(&self) -> &AgentCatalog {
        &self.catalog
    }

    /// Returns the default settings.
    #[inline]
    pub fn defaults(&self) -> &AgentDefaults {
        &self.defaults
    }

    /// Returns the shared Task delegation settings.
    #[inline]
    pub fn task_settings(&self) -> TaskSettings {
        self.task_settings
    }

    /// Returns the tools available to agents.
    #[inline]
    pub fn tools(&self) -> &[ToolCatalogEntry] {
        &self.tools
    }
}
