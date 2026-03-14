//! Markdown-agent runtime example.
//!
//! Loads markdown agents through [`AgentLoader`], builds one named agent through
//! [`AgentRuntimeBuilder`], and runs it without Task/delegation.
//!
//! Run: cargo run --example serdesai-agents -p llm-coding-tools-serdesai

use llm_coding_tools_agents::{AgentCatalog, AgentLoader, AgentRuntimeBuilder};
use llm_coding_tools_core::models::{
    Modality, ModelCatalog, ModelInfo, ProviderIdx, ProviderInfo, ProviderModelSource,
    ProviderSource, ProviderType,
};
use llm_coding_tools_serdesai::{AgentDefaults, AgentRuntimeExt};
use std::path::PathBuf;

const AGENT_NAME: &str = "basic/file-reader";

// API key below has a zero spend limit; it cannot incur charges.
// If this no longer works, find a free model to use on OpenRouter for testing.
const OPENROUTER_API_KEY: &str = "";
const OPENROUTER_MODEL: &str = "z-ai/glm-4.5-air:free";

fn build_model_catalog() -> ModelCatalog {
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
    let models: Vec<ProviderModelSource<'_>> = vec![ProviderModelSource::new(
        ProviderIdx::new(0),
        OPENROUTER_MODEL,
        info,
    )];
    ModelCatalog::build(&providers, &models).expect("model catalog should build")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let examples_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples");
    let readme_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("README.md");

    // SAFETY: Setting API key before async runtime operations; single-threaded startup.
    unsafe { std::env::set_var("OPENROUTER_API_KEY", OPENROUTER_API_KEY) };

    let mut catalog = AgentCatalog::new();
    AgentLoader::new().add_directory(&mut catalog, &examples_root)?;

    let model_catalog = build_model_catalog();

    let runtime = AgentRuntimeBuilder::new()
        .catalog(catalog)
        .defaults(AgentDefaults {
            model: Some(format!("openrouter/{OPENROUTER_MODEL}").into()),
            temperature: Some(0.2),
            top_p: Some(0.95),
        })
        .build();

    println!(
        "Loading named agent `{AGENT_NAME}` from {}",
        examples_root.display()
    );
    let agent = runtime.build(AGENT_NAME, &model_catalog)?;
    println!(
        "Built `{AGENT_NAME}` on demand with {} tools.",
        agent.tools().len()
    );

    let prompt = format!(
        "Read {} and summarize the runtime flow in three bullets.",
        readme_path.display()
    );
    let response = agent.run(prompt.as_str(), ()).await?;
    println!("{}", response.output());

    Ok(())
}
