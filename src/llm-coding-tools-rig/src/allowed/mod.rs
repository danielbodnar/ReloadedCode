//! Tools using [`llm_coding_tools_core::path::AllowedPathResolver`].
//!
//! These tools restrict file access to configured allowed directories.
//! Use for sandboxed file system access.
//!
//! # Migration from 0.1.x
//!
//! Previously, tools could be constructed directly with paths:
//!
//! ```ignore
//! // Old API (removed)
//! let read = ReadTool::new(["/path/a", "/path/b"])?;
//! let write = WriteTool::new(["/path/a"])?;  // Different paths - bug!
//! ```
//!
//! Now, create a shared [`AllowedPathResolver`] and pass it to all tools:
//!
//! ```no_run
//! use llm_coding_tools_core::path::AllowedPathResolver;
//! use llm_coding_tools_rig::allowed::{ReadTool, WriteTool, EditTool};
//! use std::path::PathBuf;
//!
//! let resolver = AllowedPathResolver::new(vec![
//!     std::env::current_dir().unwrap(),
//!     PathBuf::from("/tmp"),
//! ]).unwrap();
//!
//! let read: ReadTool<true> = ReadTool::new(resolver.clone());
//! let write = WriteTool::new(resolver.clone());
//! let edit = EditTool::new(resolver);
//! ```
//!
//! This ensures all tools share the same allowed paths configuration.
//!
//! # Available Tools
//!
//! - [`ReadTool`] - Read file contents within allowed paths
//! - [`WriteTool`] - Write file contents within allowed paths
//! - [`EditTool`] - Edit file with search/replace within allowed paths
//! - [`GlobTool`] - Find files by pattern within allowed paths
//! - [`GrepTool`] - Search file contents within allowed paths
//!
//! [`AllowedPathResolver`]: llm_coding_tools_core::path::AllowedPathResolver

mod edit;
mod glob;
mod grep;
mod read;
mod write;

pub use edit::{EditArgs, EditError, EditTool};
pub use glob::{GlobArgs, GlobTool};
pub use grep::{GrepArgs, GrepTool};
pub use read::{ReadArgs, ReadTool};
pub use write::{WriteTool, WriteToolArgs};
