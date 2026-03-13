//! Owned runtime state for later agent construction.
//!
//! This module provides the core [`AgentRuntime`] type that holds catalog,
//! defaults, and tool metadata for on-demand agent materialization. Framework
//! adapters consume this state and add concrete execution/build behavior.
//!
//! # Public API
//!
//! - [`AgentDefaults`] - Runtime-wide fallback settings applied when agent
//!   configs omit them
//! - [`AgentRuntime`] - Owned runtime state used for later agent construction

use super::tool_catalog::ToolCatalogEntry;
use crate::AgentCatalog;

/// Runtime-wide fallback settings applied when an agent config omits them.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AgentDefaults {
    /// Default model identifier in `provider/model-id` format.
    pub model: Option<Box<str>>,
    /// Default sampling temperature.
    pub temperature: Option<f32>,
    /// Default nucleus sampling top-p.
    pub top_p: Option<f32>,
}

/// Owned runtime state used for later on-demand agent construction.
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
