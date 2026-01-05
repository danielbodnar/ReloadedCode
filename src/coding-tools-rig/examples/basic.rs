//! PreambleBuilder example - pass-through tracking for ToolSet.
//!
//! Demonstrates:
//! - Using PreambleBuilder alongside ToolSet::builder()
//! - Full access to Rig's API (no wrapper limitations)
//! - Generating and using the preamble string
//!
//! Run: cargo run --example basic -p coding-tools-rig

use coding_tools_rig::absolute::{GlobTool, GrepTool, ReadTool};
use coding_tools_rig::{BashTool, PreambleBuilder};
use rig::tool::ToolSet;

#[tokio::main]
async fn main() {
    // === Create preamble builder to track tools ===
    let mut pb = PreambleBuilder::new();

    // === Use ToolSet::builder() directly - full Rig API! ===
    let toolset = ToolSet::builder()
        .static_tool(pb.track(ReadTool::<true>::new()))
        .static_tool(pb.track(GlobTool::new()))
        .static_tool(pb.track(GrepTool::<true>::new()))
        .static_tool(pb.track(BashTool::new()))
        // Can use any ToolSet method here - dynamic_tool, etc.
        .build();

    // === Generate preamble string ===
    let preamble = pb.build();

    // === Print tool definitions from ToolSet ===
    println!("=== Tools in ToolSet ===");
    for def in toolset.get_tool_definitions().await.unwrap() {
        println!(
            "  - {}: {}",
            def.name,
            &def.description[..60.min(def.description.len())]
        );
    }

    // === Print generated preamble ===
    println!("\n=== Generated Preamble ({} chars) ===\n", preamble.len());
    println!("{}", &preamble[..1000.min(preamble.len())]);
    if preamble.len() > 1000 {
        println!("\n... ({} more chars)", preamble.len() - 1000);
    }

    // === Integration with Rig agent ===
    // IMPORTANT: You must call .preamble() to actually use the generated string!
    //
    // let agent = openai::Client::from_env()
    //     .agent("gpt-4o")
    //     .preamble(&preamble)  // <-- Pass preamble to Rig
    //     .tools(toolset)
    //     .build();
    //
    // let response = agent.prompt("Read main.rs").await?;
}
