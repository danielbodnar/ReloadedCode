# rig-coding-tools

[![Crates.io - coding-tools-core](https://img.shields.io/crates/v/coding-tools-core.svg)](https://crates.io/crates/coding-tools-core)
[![Crates.io - coding-tools-rig](https://img.shields.io/crates/v/coding-tools-rig.svg)](https://crates.io/crates/coding-tools-rig)
[![Docs.rs](https://docs.rs/coding-tools-rig/badge.svg)](https://docs.rs/coding-tools-rig)
[![CI](https://github.com/Sewer56/rig-coding-tools/actions/workflows/rust.yml/badge.svg)](https://github.com/Sewer56/rig-coding-tools/actions)

Coding tools for building LLM-powered development agents with [Rig](https://github.com/0xPlaygrounds/rig).

## About This Workspace

This workspace contains multiple Rust crates for integrating coding tools with LLM agents:

- **[coding-tools-core](./src/coding-tools-core/)**: Framework-agnostic core operations and utilities
- **[coding-tools-rig](./src/coding-tools-rig/)**: Rig framework-specific Tool implementations

## Features

- **File Operations**: Read, write, edit files with line-numbered output
- **Search**: Glob pattern matching and regex content search  
- **Shell**: Cross-platform command execution with timeout
- **Web**: URL fetching with HTML-to-markdown conversion
- **Task Delegation**: Sub-agent spawning for complex workflows
- **Path Security**: Choose between unrestricted or sandboxed file access

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
coding-tools-rig = "0.1"
```

```rust
use coding_tools_rig::absolute::{ReadTool, WriteTool};
use coding_tools_rig::BashTool;

// Create tools for unrestricted file access
let read = ReadTool::new();
let write = WriteTool::new();
let bash = BashTool::new();

// Use with rig agent builder...
```

For more examples and documentation, see [README.md](./README.md) and the individual crate READMEs.

## Documentation

- [Workspace Overview](./README.md)
- [coding-tools-core README](./src/coding-tools-core/README.md)
- [coding-tools-rig README](./src/coding-tools-rig/README.md)
- [Developer Guidelines](./src/AGENTS.md)

## Contributing

Contributions are welcome! Please ensure all tests pass and the code follows our guidelines.

## License

Licensed under [Apache 2.0](./LICENSE).

