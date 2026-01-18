//! PreambleBuilder example - building a complete rig agent.
//!
//! Demonstrates:
//! - Using PreambleBuilder with rig's agent builder
//! - Chained .tool() calls for registering tools
//! - TodoTools with shared state
//! - Generating and using the preamble string
//!
//! Run: OPENAI_API_KEY=... cargo run --example rig-basic -p llm-coding-tools-rig

use llm_coding_tools_rig::absolute::{GlobTool, GrepTool, ReadTool};
use llm_coding_tools_rig::{BashTool, PreambleBuilder, TodoTools};
use rig::client::{CompletionClient, ProviderClient};
use rig::completion::Prompt;
use rig::providers::openai;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // === Create shared state for todos ===
    let todos = TodoTools::new();

    // === Create preamble builder to track tools ===
    let mut pb = PreambleBuilder::<false>::new();

    // === Build agent with chained .tool() calls ===
    let client = openai::Client::from_env();
    let agent = client
        .agent("gpt-4o")
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
