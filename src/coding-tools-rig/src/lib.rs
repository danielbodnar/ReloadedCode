//! Rig framework Tool implementations for coding tools.
//!
//! This crate provides `rig_core::tool::Tool` implementations wrapping
//! the core operations from [`coding_tools_core`].
//!
//! # Module Organization
//!
//! - [`absolute`] - Tools requiring absolute paths (no path restriction)
//! - [`allowed`] - Tools restricted to allowed directories
//! - Standalone tools (bash, task, todo, webfetch) at crate root
//!
//! # Example
//!
//! ```ignore
//! use coding_tools_rig::absolute::ReadTool;
//! use coding_tools_rig::BashTool;
//! ```

#![warn(missing_docs)]

pub mod absolute;
pub mod allowed;
pub mod bash;
pub mod task;
pub mod todo;
pub mod webfetch;

// Re-export core types for convenience
pub use coding_tools_core::{ToolError, ToolOutput, ToolResult};

// Re-export context module for convenience
pub use coding_tools_core::context;

// Re-export path resolvers
pub use coding_tools_core::path::{AbsolutePathResolver, AllowedPathResolver, PathResolver};

// Re-export core operation types used by tools
pub use coding_tools_core::{
    BashOutput, EditError, GlobOutput, GrepFileMatches, GrepLineMatch, GrepOutput,
    MockTaskExecutor, TaskExecutor, TaskResult, Todo, TodoPriority, TodoState, TodoStatus,
    WebFetchOutput,
};

// Re-export absolute module tool types
pub use absolute::{
    EditArgs, EditTool, GlobArgs, GlobTool, GrepArgs, GrepTool, ReadArgs, ReadTool, WriteTool,
    WriteToolArgs,
};

/// Re-export allowed module tool types (namespaced to avoid conflicts)
pub mod allowed_tools {
    pub use crate::allowed::{
        EditArgs, EditError, EditTool, GlobArgs, GlobTool, GrepArgs, GrepTool, ReadArgs, ReadTool,
        WriteTool, WriteToolArgs,
    };
}

// Re-export standalone tools
pub use bash::{BashArgs, BashTool};
pub use task::{TaskArgs, TaskTool};
pub use todo::{TodoReadArgs, TodoReadTool, TodoTools, TodoWriteArgs, TodoWriteTool};
pub use webfetch::{WebFetchArgs, WebFetchTool};
