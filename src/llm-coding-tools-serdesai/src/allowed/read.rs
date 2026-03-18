//! Read file tool using [`AllowedPathResolver`].

use async_trait::async_trait;
use llm_coding_tools_core::ToolContext;
use llm_coding_tools_core::path::AllowedPathResolver;
use llm_coding_tools_core::tool_metadata::read as read_meta;
use llm_coding_tools_core::tools::read_file;
use serde::Deserialize;
use serdes_ai::tools::{RunContext, SchemaBuilder, Tool, ToolDefinition, ToolError, ToolResult};

use crate::convert::to_serdes_result;

/// Internal args for JSON deserialization.
#[derive(Debug, Deserialize)]
struct ReadArgs {
    /// Path to the file (relative to allowed directories).
    file_path: String,
    /// Line offset to start reading from (1-based). Defaults to 1.
    #[serde(default = "read_meta::default_offset")]
    offset: usize,
    /// Maximum number of lines to return. Defaults to 2000.
    #[serde(default = "read_meta::default_limit")]
    limit: usize,
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
    /// use llm_coding_tools_serdesai::allowed::{ReadTool, WriteTool, EditTool};
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

#[async_trait]
impl<Deps: Send + Sync, const LINE_NUMBERS: bool> Tool<Deps> for ReadTool<LINE_NUMBERS> {
    fn definition(&self) -> ToolDefinition {
        let schema = SchemaBuilder::new()
            .string(
                read_meta::param::FILE_PATH_ALLOWED.name,
                read_meta::param::FILE_PATH_ALLOWED.description,
                read_meta::param::FILE_PATH_ALLOWED.required,
            )
            .integer_constrained(
                read_meta::param::OFFSET.name,
                read_meta::param::OFFSET.description,
                read_meta::param::OFFSET.required,
                Some(1),
                None,
            )
            .integer_constrained(
                read_meta::param::LIMIT.name,
                read_meta::param::LIMIT.description,
                read_meta::param::LIMIT.required,
                Some(1),
                None,
            )
            .build()
            .expect("schema build should not fail");

        ToolDefinition::new(
            read_meta::NAME,
            read_meta::description::allowed(LINE_NUMBERS),
        )
        .with_parameters(schema)
    }

    async fn call(&self, _ctx: &RunContext<Deps>, args: serde_json::Value) -> ToolResult {
        let args: ReadArgs = serde_json::from_value(args)
            .map_err(|e| ToolError::validation_error(read_meta::NAME, None, e.to_string()))?;

        let result =
            read_file::<_, LINE_NUMBERS>(&self.resolver, &args.file_path, args.offset, args.limit)
                .await;
        to_serdes_result(read_meta::NAME, result)
    }
}

impl<const LINE_NUMBERS: bool> ToolContext for ReadTool<LINE_NUMBERS> {
    const NAME: &'static str = read_meta::NAME;

    fn context(&self) -> &'static str {
        llm_coding_tools_core::context::READ_ALLOWED
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use serdes_ai::tools::RunContext;
    use tempfile::TempDir;

    fn mock_ctx() -> RunContext<()> {
        RunContext::new((), "test-model")
    }

    #[tokio::test]
    async fn reads_file_with_line_numbers() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("test.txt"), "hello\nworld\n").unwrap();

        let resolver = AllowedPathResolver::new([dir.path()]).unwrap();
        let tool: ReadTool<true> = ReadTool::new(resolver);
        let result = tool
            .call(
                &mock_ctx(),
                json!({
                    "file_path": "test.txt",
                    "offset": 1,
                    "limit": 2000
                }),
            )
            .await
            .unwrap();

        let text = result.as_text().unwrap();
        assert!(text.contains("L1: hello"));
        assert!(text.contains("L2: world"));
    }

    #[tokio::test]
    async fn rejects_path_traversal() {
        let dir = TempDir::new().unwrap();
        let resolver = AllowedPathResolver::new([dir.path()]).unwrap();
        let tool: ReadTool<true> = ReadTool::new(resolver);
        let result = tool
            .call(
                &mock_ctx(),
                json!({
                    "file_path": "../../../etc/passwd"
                }),
            )
            .await;

        assert!(result.is_err());
    }
}
