//! Holds your loaded agents, default settings, Task settings, and available tools.
//!
//! ## Public API
//!
//! - [`AgentRuntime`] — Container for loaded agents, defaults, Task settings, and tools.
//! - [`AgentDefaults`] — Fallback settings when an agent doesn't specify them.

use super::task::{resolve_allowed_tools, resolve_callable_target_summaries};
use super::tool_catalog::ToolCatalogEntry;
use crate::{AgentCatalog, RulesetExt};
use ahash::AHashMap;
use llm_coding_tools_core::permissions::Ruleset;
use llm_coding_tools_core::TaskSettings;
use std::sync::Arc;

use super::task::TaskTargetSummary;

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
    permission_rulesets: AHashMap<String, Arc<Ruleset>>,
}

impl AgentRuntime {
    #[inline]
    pub(super) fn from_parts(
        catalog: AgentCatalog,
        defaults: AgentDefaults,
        task_settings: TaskSettings,
        tools: Vec<ToolCatalogEntry>,
    ) -> Self {
        let permission_rulesets = catalog
            .iter()
            .map(|agent| {
                (
                    agent.name.to_string(),
                    Arc::new(Ruleset::from_permission_config(&agent.permission)),
                )
            })
            .collect();

        Self {
            catalog,
            defaults,
            task_settings,
            tools,
            permission_rulesets,
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

    /// Returns the cached permission ruleset for the named caller.
    ///
    /// The returned [`Arc`] is cheap to clone and reuses the ruleset built when
    /// the runtime was constructed.
    #[inline]
    pub fn permission_ruleset(&self, caller_name: &str) -> Option<Arc<Ruleset>> {
        self.permission_rulesets.get(caller_name).cloned()
    }

    /// Returns the tool entries exposed to the named caller.
    ///
    /// Most tools use the standard wildcard permission check (`permission -> "*"`).
    /// `task` is only included when at least one `mode: all` or `mode: subagent`
    /// target remains callable after applying `permission.task`.
    #[inline]
    pub fn allowed_tools(&self, caller_name: &str) -> Vec<ToolCatalogEntry> {
        resolve_allowed_tools(self, caller_name)
    }

    /// Returns stable summaries for every agent the named caller may delegate to via Task.
    ///
    /// Only agents with [`AgentMode::All`] or [`AgentMode::Subagent`] are
    /// considered. When the caller defines `permission.task`, targets are
    /// filtered by those rules; otherwise all non-primary agents are included.
    /// Results are sorted alphabetically by target name.
    ///
    /// # Arguments
    ///
    /// * `caller_name` - Name of the agent that wants to delegate.
    ///
    /// # Returns
    ///
    /// A [`TaskTargetSummary`] per callable target. Empty if `caller_name`
    /// is not in the catalog or no targets survive permission filtering.
    ///
    /// [`AgentMode::All`]: crate::AgentMode::All
    /// [`AgentMode::Subagent`]: crate::AgentMode::Subagent
    #[inline]
    pub fn summarize_callable_targets(&self, caller_name: &str) -> Vec<TaskTargetSummary> {
        resolve_callable_target_summaries(self, caller_name)
    }
}
