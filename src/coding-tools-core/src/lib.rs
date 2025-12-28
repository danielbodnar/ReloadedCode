//! Core types and utilities for coding tools.
//!
//! This crate provides framework-agnostic building blocks:
//! - [`ToolError`] and [`ToolResult`] for error handling
//! - [`ToolOutput`] for tool responses with truncation metadata
//! - Utility functions for text processing

#![warn(missing_docs)]

pub mod error;
pub mod operations;
pub mod output;
pub mod path;
pub mod util;

pub use error::{ToolError, ToolResult};
pub use operations::{
    edit_file, execute_command, fetch_url, glob_files, grep_search, read_file, read_todos,
    write_file, write_todos, BashOutput, EditError, GlobOutput, GrepFileMatches, GrepLineMatch,
    GrepOutput, MockTaskExecutor, TaskArgs, TaskExecutor, TaskResult, Todo, TodoPriority,
    TodoState, TodoStatus, WebFetchOutput,
};
pub use output::ToolOutput;
pub use path::{AbsolutePathResolver, AllowedPathResolver, PathResolver};
