//! Model configuration resolution for agent runtimes.
//!
//! This module translates agent model overrides and runtime defaults into
//! [`ModelConfig`] instances by validating against the models.dev catalog.
//! It provides deterministic resolution with clear error paths for invalid
//! or unknown model identifiers.
//!
//! # Public API
//!
//! ## Resolution
//!
//! - [`resolve_model_config`] - Async entrypoint that loads models.dev and resolves the effective model
//! - [`resolve_model_config_with_catalog`] - Pure testable variant accepting a pre-loaded catalog
//!
//! ## Errors
//!
//! - [`ModelTranslationError`] - All failure cases during model resolution
//!
//! # Model Resolution Precedence
//!
//! The effective model is determined by this precedence order:
//!
//! 1. **Agent override**: `model` field in agent markdown frontmatter
//! 2. **Runtime default**: `model` field in [`AgentDefaults`]
//!
//! If neither provides a valid model identifier, resolution fails with
//! [`ModelTranslationError::MissingEffectiveModel`].
//!
//! # Identifier Format
//!
//! Model identifiers use `provider/model-id` syntax (e.g., `openai/gpt-4o`,
//! `openrouter/anthropic/claude-3-5-sonnet`). Invalid formats (missing `/`,
//! empty segments) produce [`ModelTranslationError::MalformedModelIdentifier`].
//!
//! # Validation
//!
//! Resolved identifiers are validated against the models.dev catalog:
//!
//! - Unknown providers produce [`ModelTranslationError::UnknownProvider`]
//! - Unknown models produce [`ModelTranslationError::UnknownModel`]
//! - Catalog load failures produce [`ModelTranslationError::CatalogLoad`]
//!
//! # Usage
//!
//! Call [`resolve_model_config`] with agent defaults and config to obtain a
//! validated [`ModelConfig`]. The returned `model_config.spec` uses `provider:model-id`
//! format suitable for SerdesAI agent builders.
//!
//! For unit tests, use [`resolve_model_config_with_catalog`] with a synthetic catalog
//! to avoid network dependencies.
//!
//! [`ModelConfig`]: serdes_ai::ModelConfig
//! [`AgentDefaults`]: crate::agent_runtime::AgentDefaults

#![allow(dead_code)]

use crate::AgentDefaults;
use llm_coding_tools_agents::AgentConfig;
use llm_coding_tools_core::models::ModelCatalog;
use llm_coding_tools_models_dev::{CatalogError, ModelsDevCatalog};
use serdes_ai::ModelConfig;

/// Error type for model translation failures.
///
/// This enum covers all error cases when resolving and validating model
/// identifiers from agent configs and runtime defaults.
#[derive(Debug)]
pub(super) enum ModelTranslationError {
    MalformedModelIdentifier {
        agent: String,
        location: &'static str,
        model: String,
    },
    MissingEffectiveModel {
        agent: String,
    },
    UnknownProvider {
        agent: String,
        provider: String,
    },
    UnknownModel {
        agent: String,
        provider: String,
        model: String,
    },
    CatalogLoad(CatalogError),
}

impl core::fmt::Display for ModelTranslationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MalformedModelIdentifier {
                agent,
                location,
                model,
            } => write!(
                f,
                "agent `{agent}` has malformed {location} model `{model}`; expected `provider/model-id`",
            ),
            Self::MissingEffectiveModel { agent } => write!(
                f,
                "agent `{agent}` does not define a model override and runtime defaults do not define one either",
            ),
            Self::UnknownProvider { agent, provider } => {
                write!(
                    f,
                    "agent `{agent}` references unknown provider `{provider}`"
                )
            }
            Self::UnknownModel {
                agent,
                provider,
                model,
            } => write!(
                f,
                "agent `{agent}` references unknown model `{provider}/{model}`",
            ),
            Self::CatalogLoad(source) => write!(f, "failed to load models.dev catalog: {source}"),
        }
    }
}

impl std::error::Error for ModelTranslationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CatalogLoad(source) => Some(source),
            _ => None,
        }
    }
}

/// Resolves the effective model configuration for an agent.
///
/// Loads the models.dev catalog and delegates to the pure testable helper.
pub(super) async fn resolve_model_config(
    defaults: &AgentDefaults,
    agent: &AgentConfig,
) -> Result<ModelConfig, ModelTranslationError> {
    let load_result = ModelsDevCatalog::load()
        .await
        .map_err(ModelTranslationError::CatalogLoad)?;
    resolve_model_config_with_catalog(&load_result.catalog, defaults, agent)
}

struct ProviderModel<'a> {
    provider: &'a str,
    model: &'a str,
}

/// Pure function for resolving model configuration with a provided catalog.
///
/// Enables unit testing without network access by accepting a synthetic catalog.
fn resolve_model_config_with_catalog(
    catalog: &ModelCatalog,
    defaults: &AgentDefaults,
    agent: &AgentConfig,
) -> Result<ModelConfig, ModelTranslationError> {
    let parts = get_provider_model(defaults, agent)?;

    if catalog.lookup_provider(parts.provider).is_none() {
        return Err(ModelTranslationError::UnknownProvider {
            agent: agent.name.clone(),
            provider: parts.provider.to_string(),
        });
    }

    if catalog
        .lookup_provider_model(parts.provider, parts.model)
        .is_none()
    {
        return Err(ModelTranslationError::UnknownModel {
            agent: agent.name.clone(),
            provider: parts.provider.to_string(),
            model: parts.model.to_string(),
        });
    }

    Ok(ModelConfig::new(format!(
        "{}:{}",
        parts.provider, parts.model
    )))
}

/// Determines the effective model parts by applying override precedence.
///
/// Agent override takes precedence over runtime defaults. Returns an error
/// if neither provides a valid model identifier.
fn get_provider_model<'a>(
    defaults: &'a AgentDefaults,
    agent: &'a AgentConfig,
) -> Result<ProviderModel<'a>, ModelTranslationError> {
    if let Some(raw) = agent.model.as_deref() {
        let (provider, model) = agent.get_provider_model().ok_or_else(|| {
            ModelTranslationError::MalformedModelIdentifier {
                agent: agent.name.clone(),
                location: "agent override",
                model: raw.to_string(),
            }
        })?;
        return Ok(ProviderModel { provider, model });
    }

    if let Some(raw) = defaults.model.as_deref() {
        let parts = parse_model_parts(raw).ok_or_else(|| {
            ModelTranslationError::MalformedModelIdentifier {
                agent: agent.name.clone(),
                location: "runtime default",
                model: raw.to_string(),
            }
        })?;
        return Ok(parts);
    }

    Err(ModelTranslationError::MissingEffectiveModel {
        agent: agent.name.clone(),
    })
}

/// Parses a model identifier into `(provider, model)` parts.
///
/// Mirrors `AgentConfig::model_parts()` semantics for runtime defaults.
/// Returns `None` if the value lacks a `/` separator or has empty segments.
fn parse_model_parts(value: &str) -> Option<ProviderModel<'_>> {
    let (provider, model) = value.split_once('/')?;
    if provider.is_empty() || model.is_empty() {
        return None;
    }

    Some(ProviderModel { provider, model })
}

#[cfg(test)]
mod tests {
    use super::{ModelTranslationError, resolve_model_config_with_catalog};
    use crate::agent_runtime::AgentDefaults;
    use ahash::AHashMap;
    use indexmap::IndexMap;
    use llm_coding_tools_agents::{AgentConfig, AgentMode};
    use llm_coding_tools_core::models::{
        Modality, ModelCatalog, ModelInfo, ProviderIdx, ProviderInfo, ProviderModelSource,
        ProviderSource, ProviderType,
    };

    fn config_with_model(name: &str, model: Option<&str>) -> AgentConfig {
        AgentConfig {
            name: name.to_string(),
            mode: AgentMode::All,
            description: String::new(),
            model: model.map(str::to_string),
            hidden: false,
            temperature: None,
            top_p: None,
            permission: IndexMap::new(),
            options: AHashMap::new(),
            prompt: String::new(),
        }
    }

    fn provider(api_url: &str, env_vars: &[&str], api_type: ProviderType) -> ProviderInfo {
        ProviderInfo {
            api_url: api_url.to_string(),
            env_vars: env_vars.iter().map(|value| (*value).to_string()).collect(),
            api_type,
        }
    }

    fn model_info(max_input: u32, max_output: u32) -> ModelInfo {
        ModelInfo {
            modalities: Modality::TEXT,
            max_input,
            max_output,
            temperature: Some(0.2),
            top_p: Some(0.95),
        }
    }

    fn build_catalog(
        providers: Vec<(&str, ProviderInfo)>,
        provider_models: Vec<(&str, &str, ModelInfo)>,
    ) -> ModelCatalog {
        let provider_sources: Vec<ProviderSource> = providers
            .into_iter()
            .map(|(key, info)| ProviderSource::new(key, info))
            .collect();
        let provider_model_sources: Vec<ProviderModelSource<'_>> = provider_models
            .into_iter()
            .map(|(provider_key, model_key, info)| {
                let provider_idx = ProviderIdx::new(
                    provider_sources
                        .iter()
                        .position(|provider| provider.provider_key == provider_key)
                        .expect("provider key should exist") as u16,
                );
                ProviderModelSource::new(provider_idx, model_key, info)
            })
            .collect();

        ModelCatalog::build(&provider_sources, &provider_model_sources)
            .expect("catalog fixture should build")
    }

    #[test]
    fn resolves_runtime_default_when_agent_has_no_override() {
        let catalog = build_catalog(
            vec![(
                "openai",
                provider(
                    "https://api.openai.com/v1",
                    &["OPENAI_API_KEY"],
                    ProviderType::OpenAiResponses,
                ),
            )],
            vec![("openai", "gpt-4.1-mini", model_info(128_000, 16_384))],
        );
        let defaults = AgentDefaults {
            model: Some("openai/gpt-4.1-mini".to_string()),
            temperature: None,
            top_p: None,
        };
        let agent = config_with_model("planner", None);

        let resolved = resolve_model_config_with_catalog(&catalog, &defaults, &agent)
            .expect("runtime default should resolve");

        assert_eq!(resolved.spec, "openai:gpt-4.1-mini");
    }

    #[test]
    fn agent_override_wins_over_runtime_default() {
        let catalog = build_catalog(
            vec![(
                "openrouter",
                provider(
                    "https://openrouter.ai/api/v1",
                    &["OPENROUTER_API_KEY"],
                    ProviderType::OpenRouter,
                ),
            )],
            vec![
                (
                    "openrouter",
                    "openai/gpt-4.1-mini",
                    model_info(128_000, 16_384),
                ),
                ("openrouter", "openai/gpt-4o", model_info(128_000, 16_384)),
            ],
        );
        let defaults = AgentDefaults {
            model: Some("openrouter/openai/gpt-4.1-mini".to_string()),
            temperature: None,
            top_p: None,
        };
        let agent = config_with_model("planner", Some("openrouter/openai/gpt-4o"));

        let resolved = resolve_model_config_with_catalog(&catalog, &defaults, &agent)
            .expect("override should resolve");

        assert_eq!(resolved.spec, "openrouter:openai/gpt-4o");
    }

    #[test]
    fn malformed_agent_override_does_not_fall_back_to_defaults() {
        let catalog = build_catalog(
            vec![(
                "openai",
                provider(
                    "https://api.openai.com/v1",
                    &["OPENAI_API_KEY"],
                    ProviderType::OpenAiResponses,
                ),
            )],
            vec![("openai", "gpt-4.1-mini", model_info(128_000, 16_384))],
        );
        let defaults = AgentDefaults {
            model: Some("openai/gpt-4.1-mini".to_string()),
            temperature: None,
            top_p: None,
        };
        let agent = config_with_model("planner", Some("openai-only"));

        let err = resolve_model_config_with_catalog(&catalog, &defaults, &agent)
            .expect_err("malformed override should fail");

        match err {
            ModelTranslationError::MalformedModelIdentifier {
                location, model, ..
            } => {
                assert_eq!(location, "agent override");
                assert_eq!(model, "openai-only");
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn unknown_provider_returns_specific_error() {
        let catalog = build_catalog(
            vec![(
                "openai",
                provider(
                    "https://api.openai.com/v1",
                    &["OPENAI_API_KEY"],
                    ProviderType::OpenAiResponses,
                ),
            )],
            vec![("openai", "gpt-4o", model_info(128_000, 16_384))],
        );
        let defaults = AgentDefaults::default();
        let agent = config_with_model("planner", Some("anthropic/claude-3-5-sonnet"));

        let err = resolve_model_config_with_catalog(&catalog, &defaults, &agent)
            .expect_err("missing provider should fail");

        match err {
            ModelTranslationError::UnknownProvider { provider, .. } => {
                assert_eq!(provider, "anthropic");
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn unknown_model_returns_specific_error() {
        let catalog = build_catalog(
            vec![(
                "openai",
                provider(
                    "https://api.openai.com/v1",
                    &["OPENAI_API_KEY"],
                    ProviderType::OpenAiResponses,
                ),
            )],
            vec![("openai", "gpt-4o", model_info(128_000, 16_384))],
        );
        let defaults = AgentDefaults::default();
        let agent = config_with_model("planner", Some("openai/gpt-4.1-mini"));

        let err = resolve_model_config_with_catalog(&catalog, &defaults, &agent)
            .expect_err("missing provider/model pair should fail");

        match err {
            ModelTranslationError::UnknownModel {
                provider, model, ..
            } => {
                assert_eq!(provider, "openai");
                assert_eq!(model, "gpt-4.1-mini");
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn missing_agent_override_and_runtime_default_returns_dedicated_error() {
        let catalog = build_catalog(
            vec![(
                "openai",
                provider(
                    "https://api.openai.com/v1",
                    &["OPENAI_API_KEY"],
                    ProviderType::OpenAiResponses,
                ),
            )],
            vec![("openai", "gpt-4o", model_info(128_000, 16_384))],
        );
        let defaults = AgentDefaults::default();
        let agent = config_with_model("planner", None);

        let err = resolve_model_config_with_catalog(&catalog, &defaults, &agent)
            .expect_err("missing effective model should fail");

        match err {
            ModelTranslationError::MissingEffectiveModel { agent } => {
                assert_eq!(agent, "planner");
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn malformed_runtime_default_returns_clear_error() {
        let catalog = build_catalog(
            vec![(
                "openai",
                provider(
                    "https://api.openai.com/v1",
                    &["OPENAI_API_KEY"],
                    ProviderType::OpenAiResponses,
                ),
            )],
            vec![("openai", "gpt-4o", model_info(128_000, 16_384))],
        );
        let defaults = AgentDefaults {
            model: Some("openai-only".to_string()),
            temperature: None,
            top_p: None,
        };
        let agent = config_with_model("planner", None);

        let err = resolve_model_config_with_catalog(&catalog, &defaults, &agent)
            .expect_err("malformed runtime default should fail");

        match err {
            ModelTranslationError::MalformedModelIdentifier {
                location, model, ..
            } => {
                assert_eq!(location, "runtime default");
                assert_eq!(model, "openai-only");
            }
            other => panic!("unexpected error: {other}"),
        }
    }
}
