//! Write file tool using [`AllowedPathResolver`].

use async_trait::async_trait;
use llm_coding_tools_core::context::{PathMode, ToolPrompt};
use llm_coding_tools_core::path::AllowedPathResolver;
use llm_coding_tools_core::tool_metadata::write as write_meta;
use llm_coding_tools_core::tools::write_file;
use llm_coding_tools_core::{ToolContext, ToolOutput};
use serde::Deserialize;
use serdes_ai::tools::{RunContext, SchemaBuilder, Tool, ToolDefinition, ToolError, ToolResult};

use crate::convert::to_serdes_result;

/// Internal args for JSON deserialization.
#[derive(Debug, Deserialize)]
struct WriteArgs {
    /// Path to the file (relative to allowed directories).
    file_path: String,
    content: String,
}

/// Tool for writing content to files within allowed directories.
#[derive(Debug, Clone)]
pub struct WriteTool {
    resolver: AllowedPathResolver,
}

impl WriteTool {
    /// Creates a new write tool with a shared resolver.
    ///
    /// See [`ReadTool::new`] for usage example.
    ///
    /// [`ReadTool::new`]: super::ReadTool::new
    pub fn new(resolver: AllowedPathResolver) -> Self {
        Self { resolver }
    }
}

#[async_trait]
impl<Deps: Send + Sync> Tool<Deps> for WriteTool {
    fn definition(&self) -> ToolDefinition {
        let schema = SchemaBuilder::new()
            .string(
                write_meta::param::FILE_PATH_ALLOWED.name,
                write_meta::param::FILE_PATH_ALLOWED.description,
                write_meta::param::FILE_PATH_ALLOWED.required,
            )
            .string(
                write_meta::param::CONTENT.name,
                write_meta::param::CONTENT.description,
                write_meta::param::CONTENT.required,
            )
            .build()
            .expect("schema build should not fail");

        ToolDefinition::new(write_meta::NAME, write_meta::description::ALLOWED)
            .with_parameters(schema)
    }

    async fn call(&self, _ctx: &RunContext<Deps>, args: serde_json::Value) -> ToolResult {
        let args: WriteArgs = serde_json::from_value(args)
            .map_err(|e| ToolError::validation_error(write_meta::NAME, None, e.to_string()))?;

        let result = write_file(&self.resolver, &args.file_path, &args.content).await;
        to_serdes_result(write_meta::NAME, result.map(ToolOutput::new))
    }
}

impl ToolContext for WriteTool {
    const NAME: &'static str = write_meta::NAME;

    fn context(&self) -> ToolPrompt {
        ToolPrompt::Write {
            path_mode: PathMode::Allowed,
        }
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
    async fn writes_new_file() {
        let dir = TempDir::new().unwrap();
        let resolver = AllowedPathResolver::new([dir.path()]).unwrap();
        let tool = WriteTool::new(resolver);
        let result = tool
            .call(
                &mock_ctx(),
                json!({
                    "file_path": "new.txt",
                    "content": "hello"
                }),
            )
            .await
            .unwrap();

        let text = result.as_text().unwrap();
        assert!(text.contains("5 bytes"));
        assert!(dir.path().join("new.txt").exists());
    }

    #[tokio::test]
    async fn rejects_path_traversal() {
        let dir = TempDir::new().unwrap();
        let resolver = AllowedPathResolver::new([dir.path()]).unwrap();
        let tool = WriteTool::new(resolver);
        let result = tool
            .call(
                &mock_ctx(),
                json!({
                    "file_path": "../../../tmp/escape.txt",
                    "content": "content"
                }),
            )
            .await;

        assert!(result.is_err());
    }
}
