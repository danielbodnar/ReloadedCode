//! Owned runtime state for later SerdesAI agent construction.

use crate::tool_catalog::ToolCatalogEntry;
use llm_coding_tools_agents::AgentCatalog;

/// Runtime-wide fallback settings applied when an agent config omits them.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AgentDefaults {
    /// Default model identifier in `provider/model-id` format.
    pub model: Option<String>,
    /// Default sampling temperature.
    pub temperature: Option<f64>,
    /// Default nucleus sampling top-p.
    pub top_p: Option<f64>,
}

/// Owned runtime state used for later on-demand SerdesAI agent construction.
#[derive(Debug, Clone)]
pub struct AgentRuntime {
    catalog: AgentCatalog,
    defaults: AgentDefaults,
    tools: Vec<ToolCatalogEntry>,
}

impl AgentRuntime {
    #[inline]
    pub(super) fn from_parts(
        catalog: AgentCatalog,
        defaults: AgentDefaults,
        tools: Vec<ToolCatalogEntry>,
    ) -> Self {
        Self {
            catalog,
            defaults,
            tools,
        }
    }

    /// Returns the parsed catalog that remains the runtime source of truth.
    #[inline]
    pub fn catalog(&self) -> &AgentCatalog {
        &self.catalog
    }

    /// Returns the runtime fallback settings.
    #[inline]
    pub fn defaults(&self) -> &AgentDefaults {
        &self.defaults
    }

    /// Returns the owned tool-catalog metadata used for later tool materialization.
    #[inline]
    pub fn tools(&self) -> &[ToolCatalogEntry] {
        &self.tools
    }
}
