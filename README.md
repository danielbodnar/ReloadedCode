# rig-coding-tools

[![CI](https://github.com/Sewer56/rig-coding-tools/actions/workflows/rust.yml/badge.svg)](https://github.com/Sewer56/rig-coding-tools/actions/workflows/rust.yml)
[![codecov](https://codecov.io/gh/Sewer56/rig-coding-tools/graph/badge.svg)](https://codecov.io/gh/Sewer56/rig-coding-tools)

Coding tools for building LLM-powered development agents with [Rig](https://github.com/0xPlaygrounds/rig).

## Crates

| Crate | Description | Docs |
|-------|-------------|------|
| [`coding-tools-core`](src/coding-tools-core) | Framework-agnostic core operations | [![docs.rs](https://docs.rs/coding-tools-core/badge.svg)](https://docs.rs/coding-tools-core) |
| [`coding-tools-rig`](src/coding-tools-rig) | Rig framework Tool implementations | [![docs.rs](https://docs.rs/coding-tools-rig/badge.svg)](https://docs.rs/coding-tools-rig) |

## Features

- **File Operations**: Read, write, edit files with line-numbered output
- **Search**: Glob pattern matching and regex content search
- **Shell**: Cross-platform command execution with timeout
- **Web**: URL fetching with HTML-to-markdown conversion
- **Task Delegation**: Sub-agent spawning for complex workflows
- **Path Security**: Choose between unrestricted or sandboxed file access

## Quick Start

```toml
[dependencies]
coding-tools-rig = "0.1"
```

```rust
use coding_tools_rig::absolute::{ReadTool, WriteTool, GlobTool, GrepTool};
use coding_tools_rig::{BashTool, WebFetchTool};

// Create tools for unrestricted file access
let read = ReadTool::new();
let write = WriteTool::new();
let glob = GlobTool::new();
let grep = GrepTool::new();
let bash = BashTool::new();
let webfetch = WebFetchTool::new();

// Use with rig agent builder...
```

For sandboxed file access, see the [coding-tools-rig README](src/coding-tools-rig/README.md).

## License

Licensed under [Apache 2.0](./LICENSE).
