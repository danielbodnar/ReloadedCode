//! Generic resolver-based file tools.
//!
//! These tools work with any [`PathResolver`] implementation:
//! - [`AbsolutePathResolver`] for unrestricted absolute path access
//! - [`AllowedPathResolver`] for sandboxed directory-restricted access
//! - Custom resolvers implementing the [`PathResolver`] trait
//!
//! # Example
//!
//! ```no_run
//! use llm_coding_tools_serdesai::{ReadTool, AbsolutePathResolver};
//!
//! let read_tool = ReadTool::new(AbsolutePathResolver);
//! ```
//!
//! [`PathResolver`]: llm_coding_tools_core::path::PathResolver

mod edit;
mod glob;
mod grep;
mod read;
mod write;

pub use edit::EditTool;
pub use glob::GlobTool;
pub use grep::GrepTool;
pub use read::ReadTool;
pub use write::WriteTool;
