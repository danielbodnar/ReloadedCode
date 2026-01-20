//! Basic tools example - demonstrates tool setup with serdesAI.
//!
//! Shows:
//! - Creating tools individually
//! - Using [`SystemPromptBuilder`] for context generation
//! - Using [`AgentBuilderExt`] to add tools to an agent
//! - Running the agent with tools
//!
//! Run: cargo run --example serdesai-basic -p llm-coding-tools-serdesai

use futures::StreamExt;
use llm_coding_tools_serdesai::absolute::{GlobTool, GrepTool, ReadTool};
use llm_coding_tools_serdesai::agent_ext::AgentBuilderExt;
use llm_coding_tools_serdesai::{BashTool, SystemPromptBuilder, WebFetchTool, create_todo_tools};
use serdes_ai::models::openrouter::OpenRouterModel;
use serdes_ai::prelude::*;

// API key below has a zero spend limit; it cannot incur charges.
// If this no longer works, find a free model to use on OpenRouter for testing.
const OPENROUTER_API_KEY: &str = "";
const OPENROUTER_MODEL: &str = "z-ai/glm-4.5-air:free";

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // === Create system prompt builder to track tools ===
    let mut pb = SystemPromptBuilder::new()
        .working_directory(std::env::current_dir()?.display().to_string());

    // === Create todo tools with shared state ===
    let (todo_read, todo_write, _state) = create_todo_tools();

    // === Build agent with tools - call .system_prompt() last ===
    let model = OpenRouterModel::new(OPENROUTER_MODEL, OPENROUTER_API_KEY);
    let agent = AgentBuilder::<(), String>::new(model)
        .instructions("Use tools to answer; call at least one tool before responding.")
        // File operations
        .tool(pb.track(ReadTool::<true>::new()))
        .tool(pb.track(GlobTool::new()))
        .tool(pb.track(GrepTool::<true>::new()))
        // Shell execution
        .tool(pb.track(BashTool::new()))
        // Web content fetching
        .tool(pb.track(WebFetchTool::new()))
        // Todo tools with shared state
        .tool(pb.track(todo_read))
        .tool(pb.track(todo_write))
        // System prompt last (after tracking all tools)
        .system_prompt(pb.build())
        .build();

    // === Print tool info ===
    println!("=== Agent Ready ({} tools) ===", agent.tools().len());

    // === Run the agent ===
    println!("\n=== Running Agent ===");
    let prompt = "List the Rust files in the current directory using glob";
    let mut stream = agent.run_stream(prompt, ()).await?;

    while let Some(event) = stream.next().await {
        match event? {
            AgentStreamEvent::TextDelta { text, .. } => print!("{text}"),
            AgentStreamEvent::ToolCallStart {
                tool_name,
                tool_call_id,
            } => {
                let call_id = tool_call_id.unwrap_or_else(|| "unknown".to_string());
                println!("Tool call start: {tool_name} ({call_id})");
            }
            _ => {}
        }
    }

    Ok(())
}
