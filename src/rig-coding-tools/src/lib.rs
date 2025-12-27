#![doc = include_str!(concat!("../", env!("CARGO_PKG_README")))]
#![warn(missing_docs)]

pub mod error;
pub mod output;
pub mod tools;
pub mod util;

// Re-export primary types at crate root
pub use error::{ToolError, ToolResult};
pub use output::ToolOutput;
pub use tools::bash::BashTool;
pub use tools::edit::{EditArgs, EditError, EditTool};
pub use tools::grep::GrepTool;
pub use tools::read::{ReadArgs, ReadTool};
pub use tools::task::{MockTaskExecutor, TaskArgs, TaskExecutor, TaskResult, TaskTool};
pub use tools::todo::{Todo, TodoPriority, TodoReadTool, TodoState, TodoStatus, TodoWriteTool};
pub use tools::webfetch::WebFetchTool;
pub use tools::write::{WriteTool, WriteToolArgs};
