//! Read file tool using [`AllowedPathResolver`].

use llm_coding_tools_core::operations::read_file;
use llm_coding_tools_core::path::AllowedPathResolver;
use llm_coding_tools_core::tool_names;
use llm_coding_tools_core::{ToolContext, ToolError, ToolOutput};
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use schemars::{schema_for, JsonSchema};
use serde::Deserialize;

const DEFAULT_OFFSET: usize = 1;
const DEFAULT_LIMIT: usize = 2000;

fn default_offset() -> usize {
    DEFAULT_OFFSET
}

fn default_limit() -> usize {
    DEFAULT_LIMIT
}

/// Arguments for the read file tool.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ReadArgs {
    /// Relative path to the file to read (within allowed directories).
    pub file_path: String,
    /// 1-indexed line number to start reading from (default: 1).
    #[serde(default = "default_offset")]
    pub offset: usize,
    /// Maximum number of lines to return (default: 2000).
    #[serde(default = "default_limit")]
    pub limit: usize,
}

/// Tool for reading file contents with optional line numbers.
///
/// Restricts access to configured allowed directories.
#[derive(Debug, Clone)]
pub struct ReadTool<const LINE_NUMBERS: bool = true> {
    resolver: AllowedPathResolver,
}

impl<const LINE_NUMBERS: bool> ReadTool<LINE_NUMBERS> {
    /// Creates a new read tool with a shared resolver.
    ///
    /// Use a single [`AllowedPathResolver`] across all allowed tools to ensure
    /// consistent path access:
    ///
    /// ```no_run
    /// use llm_coding_tools_core::path::AllowedPathResolver;
    /// use llm_coding_tools_rig::allowed::{ReadTool, WriteTool, EditTool};
    /// use std::path::PathBuf;
    ///
    /// let resolver = AllowedPathResolver::new(vec![
    ///     std::env::current_dir().unwrap(),
    ///     PathBuf::from("/tmp"),
    /// ]).unwrap();
    ///
    /// let read: ReadTool<true> = ReadTool::new(resolver.clone());
    /// let write = WriteTool::new(resolver.clone());
    /// let edit = EditTool::new(resolver);
    /// ```
    pub fn new(resolver: AllowedPathResolver) -> Self {
        Self { resolver }
    }
}

impl<const LINE_NUMBERS: bool> Tool for ReadTool<LINE_NUMBERS> {
    const NAME: &'static str = tool_names::READ;

    type Error = ToolError;
    type Args = ReadArgs;
    type Output = ToolOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        let description = if LINE_NUMBERS {
            "Read file contents with line numbers from allowed directories. \
             Paths are relative to configured base directories."
        } else {
            "Read file contents from allowed directories. \
             Paths are relative to configured base directories."
        };
        ToolDefinition {
            name: <Self as Tool>::NAME.to_string(),
            description: description.to_string(),
            parameters: serde_json::to_value(schema_for!(ReadArgs))
                .expect("schema serialization should never fail"),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        read_file::<_, LINE_NUMBERS>(&self.resolver, &args.file_path, args.offset, args.limit).await
    }
}

impl<const LINE_NUMBERS: bool> ToolContext for ReadTool<LINE_NUMBERS> {
    const NAME: &'static str = tool_names::READ;

    fn context(&self) -> &'static str {
        llm_coding_tools_core::context::READ_ALLOWED
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn reads_file_with_line_numbers() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "hello\nworld\n").unwrap();

        let resolver = AllowedPathResolver::new([dir.path()]).unwrap();
        let tool: ReadTool<true> = ReadTool::new(resolver);
        let args = ReadArgs {
            file_path: "test.txt".to_string(),
            offset: 1,
            limit: 2000,
        };
        let result = tool.call(args).await.unwrap();
        assert_eq!(result.content, "L1: hello\nL2: world");
    }

    #[tokio::test]
    async fn rejects_path_traversal() {
        let dir = TempDir::new().unwrap();
        let resolver = AllowedPathResolver::new([dir.path()]).unwrap();
        let tool: ReadTool = ReadTool::new(resolver);
        let args = ReadArgs {
            file_path: "../../../etc/passwd".to_string(),
            offset: 1,
            limit: 100,
        };
        let result = tool.call(args).await;
        assert!(matches!(result, Err(ToolError::InvalidPath(_))));
    }
}
