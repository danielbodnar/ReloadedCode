# llm-coding-tools

[![Crates.io - llm-coding-tools-core](https://img.shields.io/crates/v/llm-coding-tools-core.svg)](https://crates.io/crates/llm-coding-tools-core)
[![Crates.io - llm-coding-tools-serdesai](https://img.shields.io/crates/v/llm-coding-tools-serdesai.svg)](https://crates.io/crates/llm-coding-tools-serdesai)
[![Docs.rs](https://docs.rs/llm-coding-tools-serdesai/badge.svg)](https://docs.rs/llm-coding-tools-serdesai)
[![CI](https://github.com/Sewer56/llm-coding-tools/actions/workflows/rust.yml/badge.svg)](https://github.com/Sewer56/llm-coding-tools/actions)

Lightweight, high-performance coding tool implementations for LLM-powered development agents. Plug and play into your favourite frameworks.

## About This Workspace

This workspace contains multiple Rust crates for integrating coding tools with LLM agents:

- **[llm-coding-tools-core](./src/llm-coding-tools-core/)**: Framework-agnostic core operations and utilities
- **[llm-coding-tools-serdesai](./src/llm-coding-tools-serdesai/)**: serdesAI framework-specific Tool implementations

## Features

- **File Operations**: Read, write, edit files with line-numbered output
- **Search**: Glob pattern matching and regex content search  
- **Shell**: Cross-platform command execution with timeout
- **Web**: URL fetching with HTML-to-markdown conversion
- **Path Security**: Choose between unrestricted or sandboxed file access
- **Context Strings**: Embedded LLM guidance for tool usage

## Feature Flags (llm-coding-tools-core)

- `tokio` (default): Async mode with tokio runtime
- `blocking`: Sync/blocking mode, mutually exclusive with `async`

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
llm-coding-tools-serdesai = "0.1"
```

```rust,no_run
use llm_coding_tools_serdesai::{AgentBuilder, BashTool, TodoTools};
use llm_coding_tools_serdesai::absolute::{ReadTool, WriteTool, EditTool, GlobTool, GrepTool};

let mut builder = AgentBuilder::new();
let todos = TodoTools::new();

builder
    .track(ReadTool::<true>::new())
    .track(WriteTool::new())
    .track(EditTool::<true>::new())
    .track(GlobTool::new())
    .track(GrepTool::<true>::new())
    .track(BashTool::new())
    .track(&todos.read)
    .track(&todos.write);

let mut agent = builder.build();

// Use the agent
// let response = agent.invoke("List all files").await?;
```

## Examples

```bash
# serdesAI framework - Basic agent setup
cargo run --example serdesai-agents -p llm-coding-tools-serdesai

# serdesAI framework - Sandboxed file access
cargo run --example serdesai-sandboxed -p llm-coding-tools-serdesai
```

## Documentation

- [llm-coding-tools-core README](./src/llm-coding-tools-core/README.md)
- [llm-coding-tools-serdesai README](./src/llm-coding-tools-serdesai/README.md)
- [Developer Guidelines](./src/AGENTS.md)

## Contributing

Contributions are welcome! Please ensure all tests pass and the code follows our guidelines.

## Deprecation Notice

**Rig framework support (`llm-coding-tools-rig`) has been removed** (commit 17158db) due to library bugs that prevented examples from running reliably.

You're welcome to submit a PR re-adding rig support if you're willing to maintain it. Since I don't use rig personally, I'm not able to actively maintain that integration. Alternatively, you can create your own crate building on `llm-coding-tools-core` directly.

## License

Licensed under [Apache 2.0](./LICENSE).
