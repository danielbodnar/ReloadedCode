//! Write file tool using [`AbsolutePathResolver`].

use async_trait::async_trait;
use llm_coding_tools_core::context::{PathMode, ToolPrompt};
use llm_coding_tools_core::path::AbsolutePathResolver;
use llm_coding_tools_core::tool_metadata::write as write_meta;
use llm_coding_tools_core::tools::write_file;
use llm_coding_tools_core::{ToolContext, ToolOutput};
use serde::Deserialize;
use serdes_ai::tools::{RunContext, SchemaBuilder, Tool, ToolDefinition, ToolError, ToolResult};

use crate::convert::to_serdes_result;

/// Internal args for JSON deserialization.
#[derive(Debug, Deserialize)]
struct WriteArgs {
    /// Absolute path to the file.
    file_path: String,
    /// Content to write to the file.
    content: String,
}

/// Tool for writing content to files.
///
/// Creates parent directories if needed and overwrites existing files.
#[derive(Debug, Clone)]
pub struct WriteTool {
    definition: ToolDefinition,
}

impl Default for WriteTool {
    fn default() -> Self {
        Self::new()
    }
}

impl WriteTool {
    /// Creates a new write tool instance.
    #[inline]
    pub fn new() -> Self {
        Self {
            definition: build_definition(),
        }
    }
}

#[async_trait]
impl<Deps: Send + Sync> Tool<Deps> for WriteTool {
    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    async fn call(&self, _ctx: &RunContext<Deps>, args: serde_json::Value) -> ToolResult {
        let args: WriteArgs = serde_json::from_value(args)
            .map_err(|e| ToolError::validation_error(write_meta::NAME, None, e.to_string()))?;

        let resolver = AbsolutePathResolver;
        let result = write_file(&resolver, &args.file_path, &args.content).await;

        // Convert String result to ToolOutput for consistent error handling
        to_serdes_result(write_meta::NAME, result.map(ToolOutput::new))
    }
}

impl ToolContext for WriteTool {
    const NAME: &'static str = write_meta::NAME;

    fn context(&self) -> ToolPrompt {
        ToolPrompt::Write {
            path_mode: PathMode::Absolute,
        }
    }
}

fn build_definition() -> ToolDefinition {
    let schema = SchemaBuilder::new()
        .string(
            write_meta::param::FILE_PATH_ABSOLUTE.name,
            write_meta::param::FILE_PATH_ABSOLUTE.description,
            write_meta::param::FILE_PATH_ABSOLUTE.required,
        )
        .string(
            write_meta::param::CONTENT.name,
            write_meta::param::CONTENT.description,
            write_meta::param::CONTENT.required,
        )
        .build()
        .expect("schema build should not fail");

    ToolDefinition {
        name: write_meta::NAME.to_owned(),
        description: write_meta::description::ABSOLUTE.to_owned(),
        parameters_json_schema: schema,
        strict: None,
        outer_typed_dict_key: None,
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
    async fn writes_file() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("new.txt");
        let tool = WriteTool::new();

        let result = tool
            .call(
                &mock_ctx(),
                json!({
                    "file_path": file_path.to_string_lossy(),
                    "content": "hello world"
                }),
            )
            .await
            .unwrap();

        let text = result.as_text().unwrap();
        assert!(text.contains("11 bytes"));
        assert_eq!(std::fs::read_to_string(&file_path).unwrap(), "hello world");
    }
}
