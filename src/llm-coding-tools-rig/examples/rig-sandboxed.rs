//! Sandboxed tools example - restricted file access.
//!
//! Demonstrates using `allowed::*` tools that restrict file operations
//! to specific directories only. This is useful for:
//!
//! - Multi-tenant environments where agents should only access their workspace
//! - Security-conscious deployments limiting filesystem exposure
//! - Project-scoped agents that shouldn't touch system files
//!
//! Run: cargo run --example rig-sandboxed -p llm-coding-tools-rig

use llm_coding_tools_rig::allowed::{EditTool, GlobTool, GrepTool, ReadTool, WriteTool};
use llm_coding_tools_rig::{AllowedPathResolver, SystemPromptBuilder};
use rig::client::CompletionClient;
use rig::completion::Prompt;
use rig::providers::openrouter;
use std::path::PathBuf;

// API key below has a zero spend limit; it cannot incur charges.
// If this no longer works, find a free model to use on OpenRouter for testing.
// Note: OpenRouter is buggy on rig currently; it may not always work well.
// This is for demonstration only.
const OPENROUTER_API_KEY: &str = "";
const OPENROUTER_MODEL: &str = "z-ai/glm-4.5-air:free";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // === Define allowed directories ===
    //
    // Only these directories (and their subdirectories) will be accessible.
    // Attempts to read/write outside these paths will fail with an error.
    //
    // NOTE: Paths must exist - AllowedPathResolver canonicalizes them.
    // Using current directory and /tmp as they exist on most systems.
    let allowed_paths = vec![
        std::env::current_dir()?, // Current working directory
        PathBuf::from("/tmp"),    // Temp directory
    ];

    // === Create resolver and tools ===
    //
    // Create one resolver and share it across tools.
    // More efficient and ensures consistency.
    let resolver = AllowedPathResolver::new(allowed_paths)?;

    let read: ReadTool<true> = ReadTool::new(resolver.clone());
    let write = WriteTool::new(resolver.clone());
    let edit = EditTool::new(resolver.clone());
    let glob = GlobTool::new(resolver.clone());
    let grep: GrepTool<true> = GrepTool::new(resolver.clone());

    // === Build agent with sandboxed tools ===
    //
    // Use SystemPromptBuilder with fluent chaining:
    // - working_directory() and allowed_paths() consume self (chaining)
    // - track() takes &mut self (passthrough for agent builder)
    let mut pb = SystemPromptBuilder::new()
        .working_directory(std::env::current_dir()?.display().to_string())
        .allowed_paths(&resolver);

    let client: openrouter::Client = openrouter::Client::new(OPENROUTER_API_KEY)?;
    let agent = client
        .agent(OPENROUTER_MODEL)
        .tool(pb.track(read))
        .tool(pb.track(write))
        .tool(pb.track(edit))
        .tool(pb.track(glob))
        .tool(pb.track(grep))
        .preamble(&pb.build())
        .build();

    // === Use the agent ===
    let response = agent
        .prompt("List all Rust files in the current directory")
        .await?;
    println!("{response}");

    Ok(())
}
