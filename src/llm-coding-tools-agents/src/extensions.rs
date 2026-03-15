//! # Ruleset Extensions
//!
//! Helpers for converting agent permission config into runtime [`Ruleset`] values.
//!
//! ## What This Module Provides
//! - [`RulesetExt`] trait for building a [`Ruleset`] from frontmatter data and
//!   filtering tool entries by permission.
//! - Support for scalar (`allow`/`deny`) and pattern-map permission rules.
//! - Iteration-order preservation via [`IndexMap`] (important for precedence).

use crate::runtime::ToolCatalogEntry;
use crate::types::PermissionRule;
use indexmap::IndexMap;
use llm_coding_tools_core::permissions::{Rule, Ruleset};

/// Extension trait for building [`Ruleset`] from agent permission configs.
pub trait RulesetExt: Sized {
    /// Creates a [`Ruleset`] from frontmatter permission configuration.
    ///
    /// The config maps permission keys to either:
    /// - A direct action (`"allow"` or `"deny"`) applying to pattern `"*"`
    /// - A map of `{ pattern: action }` for per-pattern rules
    ///
    /// Rules are added in iteration order (preserved by [`IndexMap`]).
    ///
    /// # Example
    ///
    /// ```
    /// use llm_coding_tools_agents::{RulesetExt, PermissionRule};
    /// use llm_coding_tools_core::permissions::{PermissionAction, Ruleset};
    /// use indexmap::IndexMap;
    ///
    /// let mut config = IndexMap::new();
    /// config.insert(
    ///     "bash".to_string(),
    ///     PermissionRule::Action(PermissionAction::Allow),
    /// );
    ///
    /// let ruleset = Ruleset::from_permission_config(&config);
    /// assert!(ruleset.is_allowed("bash", "*"));
    /// ```
    fn from_permission_config(config: &IndexMap<String, PermissionRule>) -> Self;

    /// Filters tool entries to those allowed by this ruleset.
    ///
    /// Returns only entries whose `name` passes `is_allowed(name, "*")`.
    ///
    /// # Arguments
    ///
    /// * `tools` - Slice of tool entries to filter.
    ///
    /// # Returns
    ///
    /// A vector containing only the tool entries allowed by this ruleset,
    /// preserving the original order.
    ///
    /// # Example
    ///
    /// ```
    /// use llm_coding_tools_agents::{
    ///     default_tools, PermissionRule, RulesetExt,
    /// };
    /// use llm_coding_tools_core::permissions::{PermissionAction, Ruleset};
    /// use indexmap::IndexMap;
    ///
    /// let mut config = IndexMap::new();
    /// config.insert("read".to_string(), PermissionRule::Action(PermissionAction::Allow));
    ///
    /// let ruleset = Ruleset::from_permission_config(&config);
    /// let allowed = ruleset.filter_allowed_tools(&default_tools());
    /// assert!(allowed.iter().any(|t| t.name == "read"));
    /// ```
    fn filter_allowed_tools(&self, tools: &[ToolCatalogEntry]) -> Vec<ToolCatalogEntry>;
}

impl RulesetExt for Ruleset {
    fn from_permission_config(config: &IndexMap<String, PermissionRule>) -> Self {
        let mut ruleset = Self::with_capacity(config.len() * 2);

        for (key, rule) in config {
            match rule {
                PermissionRule::Action(action) => {
                    ruleset.push(Rule::new(key, "*", *action));
                }
                PermissionRule::Pattern(patterns) => {
                    for (pattern, action) in patterns {
                        ruleset.push(Rule::new(key, pattern, *action));
                    }
                }
            }
        }

        ruleset
    }

    fn filter_allowed_tools(&self, tools: &[ToolCatalogEntry]) -> Vec<ToolCatalogEntry> {
        tools
            .iter()
            .copied()
            .filter(|entry| self.is_allowed(entry.name, "*"))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::default_tools;
    use llm_coding_tools_core::permissions::PermissionAction;

    #[test]
    fn from_permission_config_simple_action() {
        let mut config = IndexMap::new();
        config.insert(
            "bash".to_string(),
            PermissionRule::Action(PermissionAction::Allow),
        );

        let ruleset = Ruleset::from_permission_config(&config);

        assert_eq!(ruleset.len(), 1);
        assert!(ruleset.is_allowed("bash", "*"));
        assert!(!ruleset.is_allowed("task", "*"));
    }

    #[test]
    fn from_permission_config_pattern_map() {
        let mut patterns = IndexMap::new();
        patterns.insert("*".to_string(), PermissionAction::Deny);
        patterns.insert("orchestrator-*".to_string(), PermissionAction::Allow);

        let mut config = IndexMap::new();
        config.insert("task".to_string(), PermissionRule::Pattern(patterns));

        let ruleset = Ruleset::from_permission_config(&config);

        assert_eq!(ruleset.len(), 2);
        assert_eq!(
            ruleset.evaluate("task", "orchestrator-builder"),
            PermissionAction::Allow
        );
        assert_eq!(
            ruleset.evaluate("task", "other-agent"),
            PermissionAction::Deny
        );
    }

    #[test]
    fn filter_allowed_tools_returns_allowed_entries() {
        let mut config = IndexMap::new();
        config.insert(
            "read".to_string(),
            PermissionRule::Action(PermissionAction::Allow),
        );
        config.insert(
            "glob".to_string(),
            PermissionRule::Action(PermissionAction::Allow),
        );

        let ruleset = Ruleset::from_permission_config(&config);
        let allowed = ruleset.filter_allowed_tools(&default_tools());

        assert!(allowed.iter().any(|t| t.name == "read"));
        assert!(allowed.iter().any(|t| t.name == "glob"));
        assert!(!allowed.iter().any(|t| t.name == "bash"));
    }
}
