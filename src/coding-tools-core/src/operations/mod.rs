//! Core operations for file systems and utilities.
//!
//! This module contains framework-agnostic implementations of:
//! - File operations (read, write, edit, glob, grep)
//! - Command execution (bash)
//! - Web fetching (fetch_url)
//! - Task delegation (task execution and mocking)
//! - Todo management (todo list)

pub mod bash;
pub mod edit;
pub mod glob;
pub mod grep;
pub mod read;
pub mod task;
pub mod todo;
pub mod webfetch;
pub mod write;

pub use bash::{execute_command, BashOutput};
pub use edit::{edit_file, EditError};
pub use glob::{glob_files, GlobOutput};
pub use grep::{grep_search, GrepFileMatches, GrepLineMatch, GrepOutput};
pub use read::read_file;
pub use task::{MockTaskExecutor, TaskArgs, TaskExecutor, TaskResult};
pub use todo::{read_todos, write_todos, Todo, TodoPriority, TodoState, TodoStatus};
pub use webfetch::{fetch_url, format_json, html_to_markdown, WebFetchOutput};
pub use write::write_file;
