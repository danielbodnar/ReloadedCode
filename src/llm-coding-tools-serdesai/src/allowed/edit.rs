//! Edit file tool using [`AllowedPathResolver`].

use async_trait::async_trait;
use llm_coding_tools_core::ToolContext;
use llm_coding_tools_core::context::{PathMode, ToolPrompt};
use llm_coding_tools_core::path::AllowedPathResolver;
use llm_coding_tools_core::tool_metadata::edit as edit_meta;
use llm_coding_tools_core::tools::edit_file;
use serde::Deserialize;
use serdes_ai::tools::{
    RunContext, SchemaBuilder, Tool, ToolDefinition, ToolError, ToolResult, ToolReturn,
};

use crate::common::edit::error_to_serdes;

/// Internal args for JSON deserialization.
#[derive(Debug, Deserialize)]
struct EditArgs {
    /// Path to the file (relative to allowed directories).
    file_path: String,
    /// The exact text to find and replace.
    old_string: String,
    /// The text to replace with.
    new_string: String,
    /// Replace all occurrences instead of just the first. Defaults to false.
    #[serde(default)]
    replace_all: bool,
}

/// Tool for making exact string replacements in files within allowed directories.
#[derive(Debug, Clone)]
pub struct EditTool {
    resolver: AllowedPathResolver,
}

impl EditTool {
    /// Creates a new edit tool with a shared resolver.
    ///
    /// See [`ReadTool::new`] for usage example.
    ///
    /// [`ReadTool::new`]: super::ReadTool::new
    pub fn new(resolver: AllowedPathResolver) -> Self {
        Self { resolver }
    }
}

#[async_trait]
impl<Deps: Send + Sync> Tool<Deps> for EditTool {
    fn definition(&self) -> ToolDefinition {
        let schema = SchemaBuilder::new()
            .string(
                edit_meta::param::FILE_PATH_ALLOWED.name,
                edit_meta::param::FILE_PATH_ALLOWED.description,
                edit_meta::param::FILE_PATH_ALLOWED.required,
            )
            .string(
                edit_meta::param::OLD_STRING.name,
                edit_meta::param::OLD_STRING.description,
                edit_meta::param::OLD_STRING.required,
            )
            .string(
                edit_meta::param::NEW_STRING.name,
                edit_meta::param::NEW_STRING.description,
                edit_meta::param::NEW_STRING.required,
            )
            .boolean(
                edit_meta::param::REPLACE_ALL.name,
                edit_meta::param::REPLACE_ALL.description,
                edit_meta::param::REPLACE_ALL.required,
            )
            .build()
            .expect("schema build should not fail");

        ToolDefinition::new(edit_meta::NAME, edit_meta::description::ALLOWED)
            .with_parameters(schema)
    }

    async fn call(&self, _ctx: &RunContext<Deps>, args: serde_json::Value) -> ToolResult {
        let args: EditArgs = serde_json::from_value(args)
            .map_err(|e| ToolError::validation_error(edit_meta::NAME, None, e.to_string()))?;

        let result = edit_file(
            &self.resolver,
            &args.file_path,
            &args.old_string,
            &args.new_string,
            args.replace_all,
        )
        .await;

        result.map(ToolReturn::text).map_err(error_to_serdes)
    }
}

impl ToolContext for EditTool {
    const NAME: &'static str = edit_meta::NAME;

    fn context(&self) -> ToolPrompt {
        ToolPrompt::Edit {
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
    async fn replaces_single_occurrence() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("test.txt"), "hello world").unwrap();

        let resolver = AllowedPathResolver::new([dir.path()]).unwrap();
        let tool = EditTool::new(resolver);
        let result = tool
            .call(
                &mock_ctx(),
                json!({
                    "file_path": "test.txt",
                    "old_string": "world",
                    "new_string": "rust"
                }),
            )
            .await
            .unwrap();

        let text = result.as_text().unwrap();
        assert!(text.contains("1 occurrence"));
    }

    #[tokio::test]
    async fn rejects_path_traversal() {
        let dir = TempDir::new().unwrap();
        let resolver = AllowedPathResolver::new([dir.path()]).unwrap();
        let tool = EditTool::new(resolver);
        let result = tool
            .call(
                &mock_ctx(),
                json!({
                    "file_path": "../../../etc/passwd",
                    "old_string": "old",
                    "new_string": "new"
                }),
            )
            .await;

        assert!(result.is_err());
    }
}
