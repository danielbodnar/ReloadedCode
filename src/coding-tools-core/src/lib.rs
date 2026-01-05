//! Core types and utilities for coding tools.
//!
//! This crate provides framework-agnostic building blocks:
//! - [`ToolError`] and [`ToolResult`] for error handling
//! - [`ToolOutput`] for tool responses with truncation metadata
//! - Utility functions for text processing
//!
//! # Features
//!
//! - `async`: Enables async function signatures and async-only modules.
//! - `tokio` (default): Enables async via tokio runtime (implies `async`).
//!   When disabled, all operations are synchronous.

#![warn(missing_docs)]

pub mod error;
pub mod fs;
pub mod operations;
pub mod output;
pub mod path;
pub mod util;

pub use error::{ToolError, ToolResult};
pub use output::ToolOutput;
pub use path::{AbsolutePathResolver, AllowedPathResolver, PathResolver};

// Re-export operations (always available, sync or async based on runtime feature)
pub use operations::{
    edit_file, execute_command, glob_files, grep_search, read_file, read_todos, write_file,
    write_todos, BashOutput, EditError, GlobOutput, GrepFileMatches, GrepLineMatch, GrepOutput,
    Todo, TodoPriority, TodoState, TodoStatus,
};

// Re-export async-only operations (requires async feature)
#[cfg(feature = "async")]
pub use operations::{
    fetch_url, MockTaskExecutor, TaskArgs, TaskExecutor, TaskResult, WebFetchOutput,
};
