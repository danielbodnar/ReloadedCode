//! Basic example showing coding-tools-rig tool setup.
//!
//! Run: cargo run --example basic -p coding-tools-rig

use coding_tools_rig::absolute::{GlobTool, GrepTool, ReadTool};
use coding_tools_rig::allowed_tools::ReadTool as AllowedReadTool;
use coding_tools_rig::context::{BASH, GLOB_ABSOLUTE, READ_ABSOLUTE, READ_ALLOWED};
use coding_tools_rig::BashTool;
use rig::tool::ToolSet;

#[tokio::main]
async fn main() {
    // === Absolute path tools (unrestricted filesystem access) ===
    let read: ReadTool<true> = ReadTool::new();
    let glob = GlobTool::new();
    let grep: GrepTool<true> = GrepTool::new();
    let bash = BashTool::new();

    // === Allowed path tools (sandboxed to specific directories) ===
    let read_sandboxed: AllowedReadTool<true> =
        AllowedReadTool::new([std::env::current_dir().unwrap()]).unwrap();

    // === ToolSet for dynamic tool management ===
    let toolset = ToolSet::builder()
        .static_tool(read)
        .static_tool(glob)
        .static_tool(grep)
        .static_tool(bash)
        .static_tool(read_sandboxed)
        .build();

    // === Print tool definitions ===
    println!("Available tools:");
    for def in toolset.get_tool_definitions().await.unwrap() {
        println!(
            "  - {}: {}",
            def.name,
            &def.description[..50.min(def.description.len())]
        );
    }

    // === Context strings for LLM system prompts ===
    println!("\nContext string snippets:");
    println!("  READ_ABSOLUTE: {}...", &READ_ABSOLUTE[..60]);
    println!("  READ_ALLOWED: {}...", &READ_ALLOWED[..60]);
    println!("  GLOB_ABSOLUTE: {}...", &GLOB_ABSOLUTE[..60]);
    println!("  BASH: {}...", &BASH[..60]);
}
