# coding-tools-rig

[![Crates.io](https://img.shields.io/crates/v/coding-tools-rig.svg)](https://crates.io/crates/coding-tools-rig)
[![Docs.rs](https://docs.rs/coding-tools-rig/badge.svg)](https://docs.rs/coding-tools-rig)

Rig framework Tool implementations for coding tools.

## Features

- **Absolute-path tools** - Unrestricted file system access with absolute paths
- **Allowed-path tools** - Sandboxed access restricted to configured directories
- **Shell execution** - Cross-platform command execution with timeout
- **Web fetching** - URL content retrieval with format conversion
- **Task delegation** - Sub-agent spawning for complex tasks
- **Todo management** - Persistent todo list tracking
- **Context strings** - LLM guidance text for tool usage (re-exported from core)

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
coding-tools-rig = "0.1"
```

## Quick Start

Run the included example:

```bash
cargo run --example basic -p coding-tools-rig
```

## Usage

### Absolute-Path Tools

For unrestricted file access:

```rust
use coding_tools_rig::absolute::{ReadTool, WriteTool, EditTool, GlobTool, GrepTool};
use coding_tools_rig::BashTool;
use rig::tool::ToolSet;

let read: ReadTool<true> = ReadTool::new();
let glob = GlobTool::new();
let bash = BashTool::new();

let toolset = ToolSet::builder()
    .static_tool(read)
    .static_tool(glob)
    .static_tool(bash)
    .build();
```

### Allowed-Path Tools

For sandboxed file access:

```rust
use coding_tools_rig::allowed::ReadTool;
use std::path::PathBuf;

let read: ReadTool<true> = ReadTool::new([
    PathBuf::from("/home/user/project"),
]).unwrap();
// Paths are restricted to /home/user/project
```

### Standalone Tools

Tools without path requirements:

```rust
use coding_tools_rig::{BashTool, TaskTool, WebFetchTool};
use coding_tools_rig::todo::{TodoReadTool, TodoWriteTool};

let bash = BashTool::new();
let task = TaskTool::with_mock(); // or TaskTool::new(executor)
let webfetch = WebFetchTool::new();
```

### Context Strings

LLM guidance strings are re-exported from `coding_tools_core`:

```rust
use coding_tools_rig::context::{BASH, READ_ABSOLUTE, READ_ALLOWED};

// Use context strings in system prompts or tool descriptions
println!("{}", BASH);

// Path-based tools have absolute and allowed variants
println!("{}", READ_ABSOLUTE);  // For absolute::ReadTool
println!("{}", READ_ALLOWED);   // For allowed::ReadTool
```

## License

Apache 2.0
