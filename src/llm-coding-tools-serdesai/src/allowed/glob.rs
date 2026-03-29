//! Glob pattern file finding tool using [`AllowedPathResolver`].

use async_trait::async_trait;
use llm_coding_tools_core::ToolContext;
use llm_coding_tools_core::context::{PathMode, ToolPrompt};
use llm_coding_tools_core::path::AllowedPathResolver;
use llm_coding_tools_core::tool_metadata::glob as glob_meta;
use llm_coding_tools_core::tools::glob_files;
use serde::Deserialize;
use serdes_ai::tools::{RunContext, SchemaBuilder, Tool, ToolDefinition, ToolError, ToolResult};

use crate::common::glob::output_to_return as glob_output_to_return;
use crate::convert::to_serdes_result;

/// Internal args for JSON deserialization.
#[derive(Debug, Deserialize)]
struct GlobArgs {
    /// Glob pattern to match files (e.g., "**/*.rs", "src/**/*.ts").
    pattern: String,
    /// Directory path to search in (relative to allowed directories).
    path: String,
}

/// Tool for finding files matching glob patterns within allowed directories.
#[derive(Debug, Clone)]
pub struct GlobTool {
    definition: ToolDefinition,
    resolver: AllowedPathResolver,
    limit: usize,
}

impl GlobTool {
    /// Creates a new glob tool with a shared resolver and default settings.
    ///
    /// Uses `limit` of 1000 files.
    ///
    /// See [`ReadTool::new`] for usage example.
    ///
    /// [`ReadTool::new`]: super::ReadTool::new
    pub fn new(resolver: AllowedPathResolver) -> Self {
        Self::with_settings(resolver, glob_meta::MAX_RESULTS)
    }

    /// Creates a new glob tool with custom settings.
    ///
    /// # Arguments
    ///
    /// * `resolver` - The path resolver for allowed directory access.
    /// * `limit` - Maximum number of files to return per glob call.
    ///   Results are sorted by modification time (newest first) and truncated
    ///   to this limit if more files match the pattern.
    pub fn with_settings(resolver: AllowedPathResolver, limit: usize) -> Self {
        Self {
            definition: build_definition(),
            resolver,
            limit,
        }
    }
}

#[async_trait]
impl<Deps: Send + Sync> Tool<Deps> for GlobTool {
    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    async fn call(&self, _ctx: &RunContext<Deps>, args: serde_json::Value) -> ToolResult {
        let args: GlobArgs = serde_json::from_value(args)
            .map_err(|e| ToolError::validation_error(glob_meta::NAME, None, e.to_string()))?;

        let result = glob_files(&self.resolver, &args.pattern, &args.path, self.limit);
        match result {
            Err(e) => to_serdes_result(glob_meta::NAME, Err(e)),
            Ok(output) => Ok(glob_output_to_return(output)),
        }
    }
}

impl ToolContext for GlobTool {
    const NAME: &'static str = glob_meta::NAME;

    fn context(&self) -> ToolPrompt {
        ToolPrompt::Glob {
            path_mode: PathMode::Allowed,
        }
    }
}

fn build_definition() -> ToolDefinition {
    let schema = SchemaBuilder::new()
        .string(
            glob_meta::param::PATTERN.name,
            glob_meta::param::PATTERN.description,
            glob_meta::param::PATTERN.required,
        )
        .string(
            glob_meta::param::PATH_ALLOWED.name,
            glob_meta::param::PATH_ALLOWED.description,
            glob_meta::param::PATH_ALLOWED.required,
        )
        .build()
        .expect("schema build should not fail");

    ToolDefinition {
        name: glob_meta::NAME.to_owned(),
        description: glob_meta::description::ALLOWED.to_owned(),
        parameters_json_schema: schema,
        strict: None,
        outer_typed_dict_key: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use llm_coding_tools_core::tools::GlobOutput;
    use serde_json::json;
    use serdes_ai::tools::RunContext;
    use std::fs::{self, File};
    use tempfile::TempDir;

    fn mock_ctx() -> RunContext<()> {
        RunContext::new((), "test-model")
    }

    #[tokio::test]
    async fn finds_matching_files() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        File::create(dir.path().join("src/lib.rs")).unwrap();

        let resolver = AllowedPathResolver::new([dir.path()]).unwrap();
        let tool = GlobTool::new(resolver);
        let result = tool
            .call(
                &mock_ctx(),
                json!({
                    "pattern": "**/*.rs",
                    "path": "."
                }),
            )
            .await
            .unwrap();

        let text = result.as_text().unwrap();
        assert!(text.contains("lib.rs"));
    }

    #[tokio::test]
    async fn rejects_path_traversal() {
        let dir = TempDir::new().unwrap();
        let resolver = AllowedPathResolver::new([dir.path()]).unwrap();
        let tool = GlobTool::new(resolver);
        let result = tool
            .call(
                &mock_ctx(),
                json!({
                    "pattern": "*.rs",
                    "path": "../../../etc"
                }),
            )
            .await;

        assert!(result.is_err());
    }

    #[test]
    fn partial_glob_output_returns_json_payload() {
        let payload = glob_output_to_return(GlobOutput {
            files: vec!["src/lib.rs".to_string()],
            truncated: false,
            partial: true,
            errors: vec!["walk error: denied".to_string()],
        });

        let json = payload.as_json().unwrap();
        assert_eq!(json["partial"], true);
        assert_eq!(json["errors"][0], "walk error: denied");
    }
}
