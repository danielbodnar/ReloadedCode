//! Build SerdesAI agents from an [`AgentRuntime`] catalog.
//!
//! Use [`AgentRuntimeExt::build`] for the default environment-backed path, or
//! [`build_agent_with_credentials`] when you want to provide explicit credential
//! overrides. The builder resolves the model, filters tools by permissions, and
//! sets up the system prompt.

use super::model::resolve_model;
use super::provider_bridge::build_serdes_model;
use crate::agent_ext::AgentBuilderExt;
use crate::{
    BashTool, EditTool, GlobTool, GrepTool, ReadTool, SystemPromptBuilder, WebFetchTool, WriteTool,
    create_todo_tools,
};
use crate::task::task_tool_definition;
use llm_coding_tools_agents::{
    summarize_callable_targets, AgentRuntime, ModelResolutionError, RulesetExt, TaskTargetSummary,
    ToolCatalogEntry, ToolCatalogKind,
};
use llm_coding_tools_core::permissions::Ruleset;
use llm_coding_tools_core::{models::ModelCatalog, CredentialLookup, CredentialResolver};
use serdes_ai::agent::{RunContext as AgentRunContext, ToolExecutor};
use serdes_ai::tools::{ToolError, ToolReturn};
use serdes_ai::{Agent, AgentBuilder};
use serdes_ai_models::BoxedModel;

/// SerdesAI-specific runtime extension methods.
pub trait AgentRuntimeExt {
    /// Builds a runnable SerdesAI agent for the named catalog entry.
    fn build(
        &self,
        name: &str,
        model_catalog: &ModelCatalog,
    ) -> Result<Agent<(), String>, AgentBuildError>;
}

impl AgentRuntimeExt for AgentRuntime {
    fn build(
        &self,
        name: &str,
        model_catalog: &ModelCatalog,
    ) -> Result<Agent<(), String>, AgentBuildError> {
        let credentials = CredentialResolver::new();
        let prepared = prepare_build(self, name, model_catalog, &credentials)?;
        let builder = AgentBuilder::<(), String>::from_arc(prepared.model.clone());
        Ok(finish_builder(builder, &prepared)?.build())
    }
}

/// Builds a runnable SerdesAI agent using the provided credential resolver.
///
/// This is useful when your application wants to provide config-file or test
/// overrides while keeping the catalog-driven provider lookup flow.
///
/// # Errors
///
/// Returns [`AgentBuildError`] when the named agent is missing, model selection
/// fails, the adapter cannot create one of the requested tools, or the model
/// backend rejects the resolved credentials or provider settings.
pub fn build_agent_with_credentials(
    runtime: &AgentRuntime,
    name: &str,
    model_catalog: &ModelCatalog,
    credentials: &impl CredentialLookup,
) -> Result<Agent<(), String>, AgentBuildError> {
    let prepared = prepare_build(runtime, name, model_catalog, credentials)?;
    let builder = AgentBuilder::<(), String>::from_arc(prepared.model.clone());
    Ok(finish_builder(builder, &prepared)?.build())
}

/// Resolved build parameters ready for agent construction.
#[derive(Clone)]
struct PreparedBuild {
    /// Agent name for [`AgentBuilder::name`].
    agent_name: Box<str>,
    /// Concrete SerdesAI model.
    model: BoxedModel,
    /// Normalized SerdesAI `provider:model` specification for diagnostics.
    #[cfg_attr(not(test), allow(dead_code))]
    model_spec: Box<str>,
    /// Agent system prompt template.
    prompt: Box<str>,
    /// Sampling temperature, if specified at agent or defaults level.
    temperature: Option<f64>,
    /// Top-p sampling parameter, if specified at agent or defaults level.
    top_p: Option<f64>,
    /// Permission-filtered tool entries to materialize.
    tools: Vec<ToolCatalogEntry>,
    /// Pre-computed callable Task target summaries for the Task tool description.
    callable_target_summaries: Vec<TaskTargetSummary>,
}

/// Resolves model configuration and collects build parameters for an agent.
fn prepare_build(
    runtime: &AgentRuntime,
    name: &str,
    model_catalog: &ModelCatalog,
    credentials: &impl CredentialLookup,
) -> Result<PreparedBuild, AgentBuildError> {
    let agent = runtime
        .catalog()
        .by_name(name)
        .ok_or_else(|| AgentBuildError::UnknownAgent { name: name.into() })?;
    let resolved = resolve_model(model_catalog, runtime.defaults(), agent)?;
    let serdes_model = build_serdes_model(model_catalog, &resolved, credentials)?;
    Ok(PreparedBuild {
        agent_name: agent.name.clone(),
        model: serdes_model.model,
        model_spec: serdes_model.spec,
        prompt: agent.prompt.clone(),
        temperature: agent
            .temperature
            .or(runtime.defaults().temperature)
            .map(f64::from),
        top_p: agent.top_p.or(runtime.defaults().top_p).map(f64::from),
        tools: Ruleset::from_permission_config(&agent.permission)
            .filter_allowed_tools(runtime.tools()),
        callable_target_summaries: summarize_callable_targets(runtime.catalog(), name),
    })
}

/// Configures an [`AgentBuilder`] with name, prompt, tools, and sampling parameters.
///
/// Returns [`AgentBuildError::UnsupportedToolKind`] if a tool kind cannot be materialized.
fn finish_builder(
    mut builder: AgentBuilder<(), String>,
    prepared: &PreparedBuild,
) -> Result<AgentBuilder<(), String>, AgentBuildError> {
    let mut prompt_builder = SystemPromptBuilder::new().system_prompt(prepared.prompt.as_ref());
    let (todo_read, todo_write, _todo_state) = create_todo_tools();

    builder = builder.name(prepared.agent_name.as_ref());
    if let Some(temperature) = prepared.temperature {
        builder = builder.temperature(temperature);
    }
    if let Some(top_p) = prepared.top_p {
        builder = builder.top_p(top_p);
    }

    for entry in &prepared.tools {
        match entry.kind {
            ToolCatalogKind::Read => {
                builder = builder.tool(prompt_builder.track(ReadTool::<true>::new()))
            }
            ToolCatalogKind::Write => {
                builder = builder.tool(prompt_builder.track(WriteTool::new()))
            }
            ToolCatalogKind::Edit => builder = builder.tool(prompt_builder.track(EditTool::new())),
            ToolCatalogKind::Glob => builder = builder.tool(prompt_builder.track(GlobTool::new())),
            ToolCatalogKind::Grep => {
                builder = builder.tool(prompt_builder.track(GrepTool::<true>::new()))
            }
            ToolCatalogKind::Bash => builder = builder.tool(prompt_builder.track(BashTool::new())),
            ToolCatalogKind::WebFetch => {
                builder = builder.tool(prompt_builder.track(WebFetchTool::new()))
            }
            ToolCatalogKind::TodoRead => {
                builder = builder.tool(prompt_builder.track(todo_read.clone()))
            }
            ToolCatalogKind::TodoWrite => {
                builder = builder.tool(prompt_builder.track(todo_write.clone()))
            }
            ToolCatalogKind::Task => {
                if !prepared.callable_target_summaries.is_empty() {
                    let definition =
                        task_tool_definition(&prepared.callable_target_summaries);
                    builder = builder.tool_with_executor(definition, StubTaskExecutor);
                }
            }
            _ => {
                return Err(AgentBuildError::UnsupportedToolKind {
                    name: entry.name.into(),
                });
            }
        }
    }

    Ok(builder.system_prompt(prompt_builder.build()))
}

struct StubTaskExecutor;

#[async_trait::async_trait]
impl ToolExecutor<()> for StubTaskExecutor {
    async fn execute(
        &self,
        _args: serde_json::Value,
        _ctx: &AgentRunContext<()>,
    ) -> Result<ToolReturn, ToolError> {
        Err(ToolError::execution_failed(
            "task tool execution is not yet implemented",
        ))
    }
}

/// Error returned when a build cannot produce a SerdesAI agent.
#[derive(Debug, thiserror::Error)]
pub enum AgentBuildError {
    /// The requested agent name was not found in the runtime catalog.
    #[error("unknown agent `{name}`")]
    UnknownAgent {
        /// The missing agent name.
        name: Box<str>,
    },
    /// The runtime contains a tool kind this adapter cannot materialize.
    #[error("tool `{name}` is not supported")]
    UnsupportedToolKind {
        /// The unsupported tool name.
        name: Box<str>,
    },
    /// Resolving or validating the model configuration failed.
    #[error(transparent)]
    ModelResolution(#[from] ModelResolutionError),
    /// Initializing the SerdesAI model failed.
    #[error("failed to initialize model: {0}")]
    ModelInit(#[from] serdes_ai_models::ModelError),
}

#[cfg(test)]
mod tests {
    use super::{prepare_build, AgentBuildError};
    use ahash::AHashMap;
    use indexmap::IndexMap;
    use llm_coding_tools_agents::{
        AgentCatalog, AgentConfig, AgentDefaults, AgentMode, AgentRuntimeBuilder, PermissionRule,
    };
    use llm_coding_tools_core::models::{
        Modality, ModelCatalog, ModelInfo, ProviderIdx, ProviderInfo, ProviderModelSource,
        ProviderSource, ProviderType,
    };
    use llm_coding_tools_core::permissions::PermissionAction;
    use llm_coding_tools_core::tool_names;
    use llm_coding_tools_core::CredentialResolver;
    use serdes_ai::AgentBuilder;
    use serdes_ai_models::MockModel;
    use std::collections::HashSet;

    /// Builds an agent using a mock model instead of a real one.
    fn build_with_mock(
        prepared: &super::PreparedBuild,
        name: &str,
    ) -> serdes_ai::Agent<(), String> {
        super::finish_builder(
            AgentBuilder::<(), String>::new(MockModel::new(name)),
            prepared,
        )
        .expect("build should succeed")
        .build()
    }

    /// Creates a minimal agent config with no model or sampling overrides.
    fn agent(
        name: &str,
        permission: IndexMap<String, PermissionRule>,
        prompt: &str,
    ) -> AgentConfig {
        AgentConfig {
            name: name.into(),
            mode: AgentMode::Primary,
            description: format!("{name} description").into(),
            model: None,
            hidden: false,
            temperature: None,
            top_p: None,
            permission,
            options: AHashMap::new(),
            prompt: prompt.into(),
        }
    }

    /// Creates an agent config with explicit model and sampling settings.
    fn agent_with_sampling(
        name: &str,
        model: &str,
        temperature: Option<f32>,
        top_p: Option<f32>,
        permission: IndexMap<String, PermissionRule>,
        prompt: &str,
    ) -> AgentConfig {
        AgentConfig {
            name: name.into(),
            mode: AgentMode::All,
            description: format!("{name} description").into(),
            model: Some(model.into()),
            hidden: false,
            temperature,
            top_p,
            permission,
            options: AHashMap::new(),
            prompt: prompt.into(),
        }
    }

    /// Creates permission rules that allow the specified tools.
    fn allow_tools(names: &[&str]) -> IndexMap<String, PermissionRule> {
        names
            .iter()
            .map(|n| ((*n).into(), PermissionRule::Action(PermissionAction::Allow)))
            .collect()
    }

    /// Creates a model catalog with two OpenRouter models for testing.
    fn catalog() -> ModelCatalog {
        let providers = vec![ProviderSource::new(
            "openrouter",
            ProviderInfo {
                api_url: "https://openrouter.ai/api/v1".into(),
                env_vars: vec!["OPENROUTER_API_KEY".into()],
                api_type: ProviderType::OpenRouter,
            },
        )];
        let info = ModelInfo {
            modalities: Modality::TEXT,
            max_input: 128_000,
            max_output: 16_384,
            temperature: Some(1.0),
            top_p: Some(0.95),
        };
        let models: Vec<ProviderModelSource<'_>> =
            [("openai/gpt-4.1-mini", info), ("openai/gpt-4o", info)]
                .into_iter()
                .map(|(key, i)| ProviderModelSource::new(ProviderIdx::new(0), key, i))
                .collect();
        ModelCatalog::build(&providers, &models).expect("catalog fixture should build")
    }

    fn credentials() -> CredentialResolver<false> {
        let mut credentials = CredentialResolver::without_env();
        credentials.set_override("OPENROUTER_API_KEY", "openrouter-key");
        credentials
    }

    #[test]
    fn build_filters_tools_by_permission() {
        let credentials = credentials();
        let catalog = catalog();

        // Create runtime with two agents: one with allowed tools, one with none
        let runtime = AgentRuntimeBuilder::new()
            .catalog(AgentCatalog::from_entries([
                agent(
                    "with-tools",
                    allow_tools(&[tool_names::READ, tool_names::BASH]),
                    "prompt",
                ),
                agent("no-tools", IndexMap::new(), "prompt"),
            ]))
            .defaults(AgentDefaults::with_model("openrouter/openai/gpt-4.1-mini"))
            .build();

        // Agent with permissions gets only the allowed tools
        let prepared =
            prepare_build(&runtime, "with-tools", &catalog, &credentials).expect("should succeed");
        let agent = build_with_mock(&prepared, "with-tools");
        let names: HashSet<&str> = agent.tools().iter().map(|t| t.name()).collect();
        assert!(names.contains(tool_names::READ));
        assert!(names.contains(tool_names::BASH));
        assert_eq!(names.len(), 2);

        // Agent with empty permissions gets no tools
        let prepared =
            prepare_build(&runtime, "no-tools", &catalog, &credentials).expect("should succeed");
        let agent = build_with_mock(&prepared, "no-tools");
        assert!(agent.tools().is_empty());
    }

    #[test]
    fn build_uses_agent_model_and_sampling_over_defaults() {
        let credentials = credentials();
        let catalog = catalog();

        // Agent overrides model (gpt-4o) and sampling (0.4, 0.8) vs defaults (gpt-4.1-mini, 1.0, 0.95)
        let runtime = AgentRuntimeBuilder::new()
            .catalog(AgentCatalog::from_entries([agent_with_sampling(
                "planner",
                "openrouter/openai/gpt-4o",
                Some(0.4),
                Some(0.8),
                allow_tools(&[tool_names::READ]),
                "prompt",
            )]))
            .defaults(AgentDefaults {
                model: Some("openrouter/openai/gpt-4.1-mini".into()),
                temperature: Some(1.0),
                top_p: Some(0.95),
            })
            .build();

        // Agent-level settings win over defaults
        let prepared =
            prepare_build(&runtime, "planner", &catalog, &credentials).expect("should succeed");
        assert_eq!(prepared.model_spec.as_ref(), "openrouter:openai/gpt-4o");
        assert!((prepared.temperature.unwrap() - 0.4).abs() < 1e-6);
        assert!((prepared.top_p.unwrap() - 0.8).abs() < 1e-6);
    }

    #[test]
    fn build_handles_catalog_edge_cases() {
        let credentials = credentials();
        let catalog = catalog();

        // Unknown agent name returns clear error
        let runtime = AgentRuntimeBuilder::new()
            .defaults(AgentDefaults::with_model("openrouter/openai/gpt-4.1-mini"))
            .build();
        let err = prepare_build(&runtime, "missing", &catalog, &credentials)
            .err()
            .expect("should fail");
        assert!(matches!(err, AgentBuildError::UnknownAgent { name } if &*name == "missing"));

        // Duplicate agent names: catalog retains the last entry
        let runtime = AgentRuntimeBuilder::new()
            .catalog(AgentCatalog::from_entries([
                agent_with_sampling(
                    "dupe",
                    "openrouter/openai/gpt-4.1-mini",
                    None,
                    None,
                    allow_tools(&[tool_names::READ]),
                    "old",
                ),
                agent_with_sampling(
                    "dupe",
                    "openrouter/openai/gpt-4o",
                    None,
                    None,
                    allow_tools(&[tool_names::GLOB]),
                    "new",
                ),
            ]))
            .defaults(AgentDefaults::default())
            .build();
        let prepared =
            prepare_build(&runtime, "dupe", &catalog, &credentials).expect("should succeed");
        assert_eq!(prepared.model_spec.as_ref(), "openrouter:openai/gpt-4o");
        let agent = build_with_mock(&prepared, "dupe");
        assert_eq!(agent.tools().len(), 1);
        assert_eq!(agent.tools()[0].name(), tool_names::GLOB);
    }

    /// Creates a subagent config with no model or sampling overrides.
    fn subagent(
        name: &str,
        permission: IndexMap<String, PermissionRule>,
        prompt: &str,
    ) -> AgentConfig {
        let mut config = agent(name, permission, prompt);
        config.mode = AgentMode::Subagent;
        config
    }

    #[test]
    fn build_attaches_task_tool_when_allowed_and_targets_exist() {
        let credentials = credentials();
        let catalog = catalog();

        let runtime = AgentRuntimeBuilder::new()
            .catalog(AgentCatalog::from_entries([
                agent(
                    "caller",
                    allow_tools(&[tool_names::TASK, tool_names::READ]),
                    "prompt",
                ),
                subagent(
                    "sub-target",
                    IndexMap::new(),
                    "subagent prompt",
                ),
            ]))
            .defaults(AgentDefaults::with_model("openrouter/openai/gpt-4.1-mini"))
            .build();

        let prepared =
            prepare_build(&runtime, "caller", &catalog, &credentials).expect("should succeed");
        assert!(!prepared.callable_target_summaries.is_empty());

        let agent = build_with_mock(&prepared, "caller");
        let names: Vec<&str> = agent.tools().iter().map(|t| t.name()).collect();
        assert!(names.contains(&tool_names::READ));
        assert!(names.contains(&tool_names::TASK));
    }

    #[test]
    fn build_skips_task_tool_when_no_callable_targets() {
        let credentials = credentials();
        let catalog = catalog();

        let runtime = AgentRuntimeBuilder::new()
            .catalog(AgentCatalog::from_entries([agent(
                "solo",
                allow_tools(&[tool_names::TASK]),
                "prompt",
            )]))
            .defaults(AgentDefaults::with_model("openrouter/openai/gpt-4.1-mini"))
            .build();

        let prepared =
            prepare_build(&runtime, "solo", &catalog, &credentials).expect("should succeed");
        assert!(prepared.callable_target_summaries.is_empty());

        let agent = build_with_mock(&prepared, "solo");
        let names: Vec<&str> = agent.tools().iter().map(|t| t.name()).collect();
        assert!(!names.contains(&tool_names::TASK));
    }
}
