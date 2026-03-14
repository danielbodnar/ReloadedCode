//! Markdown-agent runtime example using models.dev catalog.
//!
//! Loads markdown agents through [`AgentLoader`], builds one named agent through
//! [`AgentRuntimeBuilder`], and runs it without Task/delegation.
//!
//! The model catalog is loaded from models.dev, which provides up-to-date
//! provider and model information from <https://models.dev/api.json>.
//!
//! Run: cargo run --example serdesai-agents -p llm-coding-tools-serdesai

use llm_coding_tools_agents::{AgentCatalog, AgentLoader, AgentRuntimeBuilder};
use llm_coding_tools_models_dev::ModelsDevCatalog;
use llm_coding_tools_serdesai::{AgentDefaults, AgentRuntimeExt};
use std::path::PathBuf;

const AGENT_NAME: &str = "basic/file-reader";

// Set your OpenAI API key here or via OPENAI_API_KEY environment variable.
/// Fallback API key if env var is not set. Leave empty to require env var.
const OPENAI_API_KEY: &str = "";
const OPENAI_BASE_URL: &str = "https://api.synthetic.new/openai/v1";
const MODEL_ID: &str = "synthetic/hf:zai-org/GLM-4.7";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let examples_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples");
    let readme_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("README.md");

    // SAFETY: Setting env vars before async runtime operations; single-threaded startup.
    // serdes-ai's OpenAI provider reads OPENAI_BASE_URL to override the default endpoint.
    unsafe {
        std::env::set_var("OPENAI_API_KEY", OPENAI_API_KEY);
        std::env::set_var("OPENAI_BASE_URL", OPENAI_BASE_URL);
    }

    // Load model catalog from models.dev (online-first with local cache fallback)
    let load_result = ModelsDevCatalog::load().await?;
    println!(
        "Loaded model catalog from models.dev (source: {:?})",
        load_result.source
    );

    let mut catalog = AgentCatalog::new();
    AgentLoader::new().add_directory(&mut catalog, &examples_root)?;

    let runtime = AgentRuntimeBuilder::new()
        .catalog(catalog)
        .defaults(AgentDefaults {
            model: Some(MODEL_ID.into()),
            temperature: Some(0.2),
            top_p: Some(0.95),
        })
        .build();

    println!(
        "Loading named agent `{AGENT_NAME}` from {}",
        examples_root.display()
    );
    let agent = runtime.build(AGENT_NAME, &load_result.catalog)?;
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
