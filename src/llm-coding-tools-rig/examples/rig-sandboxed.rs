//! Sandboxed tools example - restricted file access.
//!
//! Demonstrates using `allowed::*` tools that restrict file operations
//! to specific directories only. This is useful for:
//!
//! - Multi-tenant environments where agents should only access their workspace
//! - Security-conscious deployments limiting filesystem exposure
//! - Project-scoped agents that shouldn't touch system files
//!
//! Run: OPENAI_API_KEY=... cargo run --example rig-sandboxed -p llm-coding-tools-rig

use llm_coding_tools_rig::allowed::{EditTool, GlobTool, GrepTool, ReadTool, WriteTool};
use llm_coding_tools_rig::{AllowedPathResolver, SystemPromptBuilder};
use rig::client::{CompletionClient, ProviderClient};
use rig::completion::Prompt;
use rig::providers::openai;
use std::path::PathBuf;

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
        .working_directory(std::env::current_dir()?.to_string())
        .allowed_paths(&resolver);

    let client = openai::Client::from_env();
    let agent = client
        .agent("gpt-4o")
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
