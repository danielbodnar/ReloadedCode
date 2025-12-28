# coding-tools-core

Core types and utilities for coding tools - framework agnostic.

## Overview

This crate provides the foundational building blocks for coding tool implementations:

- `ToolError` - Unified error type for all tool operations
- `ToolResult<T>` - Result type alias using ToolError
- `ToolOutput` - Wrapper for tool responses with truncation metadata
- Utility functions for text processing and formatting

## Usage

```rust
use coding_tools_core::{ToolError, ToolResult, ToolOutput};
use coding_tools_core::util::{truncate_text, format_numbered_line};
```

## Design Principles

- Zero `rig-core` or framework-specific dependencies
- Minimal dependency footprint
- Performance-oriented with zero-cost abstractions
