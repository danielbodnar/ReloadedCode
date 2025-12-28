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

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
coding-tools-rig = "0.1.0"
```

## Usage

### Absolute-Path Tools

For unrestricted file access:

```rust
use coding_tools_rig::absolute::{ReadTool, WriteTool, EditTool, GlobTool, GrepTool};
use rig::tool::Tool;

let read_tool = ReadTool::new();
// Use with rig agent...
```

### Allowed-Path Tools

For sandboxed file access:

```rust
use coding_tools_rig::allowed::{ReadTool, WriteTool};
use coding_tools_rig::AllowedPathResolver;
use std::path::PathBuf;

let resolver = AllowedPathResolver::new(vec![
    PathBuf::from("/home/user/project"),
]).unwrap();

let read_tool = ReadTool::new(resolver);
// Use with rig agent - paths restricted to /home/user/project
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

## License

Apache 2.0
