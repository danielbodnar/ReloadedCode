//! SystemPromptBuilder example - building a complete rig agent.
//!
//! Demonstrates:
//! - Using SystemPromptBuilder with rig's agent builder
//! - Chained .tool() calls for registering tools
//! - TodoTools with shared state
//! - Generating and using the system prompt string
//!
//! Run: cargo run --example rig-basic -p llm-coding-tools-rig

use llm_coding_tools_rig::absolute::{GlobTool, GrepTool, ReadTool};
use llm_coding_tools_rig::{BashTool, SystemPromptBuilder, TodoTools};
use rig::client::CompletionClient;
use rig::completion::Prompt;
use rig::providers::openrouter;

// API key below has a zero spend limit; it cannot incur charges.
// If this no longer works, find a free model to use on OpenRouter for testing.
// Note: OpenRouter is buggy on rig currently; it may not always work well.
// This is for demonstration only.
const OPENROUTER_API_KEY: &str = "";
const OPENROUTER_MODEL: &str = "z-ai/glm-4.5-air:free";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // === Create shared state for todos ===
    let todos = TodoTools::new();

    // === Create system prompt builder to track tools ===
    let mut pb = SystemPromptBuilder::new()
        .working_directory(std::env::current_dir()?.display().to_string());

    // === Build agent with chained .tool() calls ===
    let client: openrouter::Client = openrouter::Client::new(OPENROUTER_API_KEY)?;
    let agent = client
        .agent(OPENROUTER_MODEL)
        .tool(pb.track(ReadTool::<true>::new()))
        .tool(pb.track(GlobTool::new()))
        .tool(pb.track(GrepTool::<true>::new()))
        .tool(pb.track(BashTool::new()))
        // Todo tools share state for read/write coordination
        .tool(pb.track(todos.read))
        .tool(pb.track(todos.write))
        .preamble(&pb.build())
        .build();

    // === Use the agent ===
    let response = agent
        .prompt("What files are in the current directory?")
        .await?;
    println!("{response}");

    Ok(())
}
