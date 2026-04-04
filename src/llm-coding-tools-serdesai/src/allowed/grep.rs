//! Grep content search tool using [`AllowedPathResolver`].

use async_trait::async_trait;
use llm_coding_tools_core::ToolContext;
use llm_coding_tools_core::context::{PathMode, ToolPrompt};
use llm_coding_tools_core::path::AllowedPathResolver;
use llm_coding_tools_core::tool_metadata::grep as grep_meta;
use llm_coding_tools_core::tools::{DEFAULT_MAX_LINE_LENGTH, grep_search};
use serde::Deserialize;
use serdes_ai::tools::{RunContext, SchemaBuilder, Tool, ToolDefinition, ToolError, ToolResult};

use crate::common::grep::output_to_return as grep_output_to_return;
use crate::convert::to_serdes_result;

/// Internal args for JSON deserialization.
#[derive(Debug, Deserialize)]
struct GrepArgs {
    /// Regular expression pattern to search for in file contents.
    pattern: String,
    /// Directory path to search in (relative to allowed directories).
    path: String,
    /// File pattern to filter search results (e.g., "*.rs", "*.{ts,tsx}").
    #[serde(default)]
    include: Option<String>,
    /// Maximum number of matches to return (default: 100, max: 2000).
    #[serde(default)]
    limit: Option<usize>,
}

/// Tool for searching file contents within allowed directories.
#[derive(Debug, Clone)]
pub struct GrepTool {
    definition: ToolDefinition,
    resolver: AllowedPathResolver,
    max_line_length: usize,
    limit: usize,
    line_numbers: bool,
}

impl GrepTool {
    /// Creates a new grep tool with a shared resolver and default settings.
    ///
    /// Uses `max_line_length` of 2000 characters, `limit` of 100 matches,
    /// and enables line numbers.
    ///
    /// See [`ReadTool::new`] for usage example.
    ///
    /// [`ReadTool::new`]: super::ReadTool::new
    pub fn new(resolver: AllowedPathResolver) -> Self {
        Self::with_settings(
            resolver,
            DEFAULT_MAX_LINE_LENGTH,
            grep_meta::DEFAULT_LIMIT,
            true,
        )
    }

    /// Creates a new grep tool with custom settings.
    ///
    /// # Arguments
    ///
    /// * `resolver` - The path resolver for allowed directory access.
    /// * `max_line_length` - Maximum characters per matching line before truncation.
    ///   Longer lines will be truncated with "..." appended.
    /// * `limit` - Maximum number of matches to return when not specified in args.
    /// * `line_numbers` - Whether to prefix lines with line numbers.
    pub fn with_settings(
        resolver: AllowedPathResolver,
        max_line_length: usize,
        limit: usize,
        line_numbers: bool,
    ) -> Self {
        Self {
            definition: build_definition(line_numbers),
            resolver,
            max_line_length,
            limit,
            line_numbers,
        }
    }
}

#[async_trait]
impl<Deps: Send + Sync> Tool<Deps> for GrepTool {
    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    async fn call(&self, _ctx: &RunContext<Deps>, args: serde_json::Value) -> ToolResult {
        let args: GrepArgs = serde_json::from_value(args)
            .map_err(|e| ToolError::validation_error(grep_meta::NAME, None, e.to_string()))?;

        let pattern = args.pattern.trim();
        if pattern.is_empty() {
            return Err(ToolError::validation_error(
                grep_meta::NAME,
                Some("pattern".to_string()),
                "pattern must not be empty".to_string(),
            ));
        }

        let limit = args.limit.unwrap_or(self.limit);
        if limit == 0 {
            return Err(ToolError::validation_error(
                grep_meta::NAME,
                Some("limit".to_string()),
                "limit must be greater than zero".to_string(),
            ));
        }

        let include = args.include.as_deref().and_then(|s| {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        });

        let result = grep_search(&self.resolver, pattern, include, &args.path, limit);

        match result {
            Err(e) => to_serdes_result(grep_meta::NAME, Err(e)),
            Ok(grep_output) => Ok(grep_output_to_return(
                grep_output,
                self.line_numbers,
                limit,
                self.max_line_length,
            )),
        }
    }
}

impl ToolContext for GrepTool {
    const NAME: &'static str = grep_meta::NAME;

    fn context(&self) -> ToolPrompt {
        ToolPrompt::Grep {
            path_mode: PathMode::Allowed,
            line_numbers: self.line_numbers,
        }
    }
}

fn build_definition(line_numbers: bool) -> ToolDefinition {
    let schema = SchemaBuilder::new()
        .string(
            grep_meta::param::PATTERN.name,
            grep_meta::param::PATTERN.description,
            grep_meta::param::PATTERN.required,
        )
        .string(
            grep_meta::param::PATH_ALLOWED.name,
            grep_meta::param::PATH_ALLOWED.description,
            grep_meta::param::PATH_ALLOWED.required,
        )
        .string(
            grep_meta::param::INCLUDE.name,
            grep_meta::param::INCLUDE.description,
            grep_meta::param::INCLUDE.required,
        )
        .integer_constrained(
            grep_meta::param::LIMIT.name,
            grep_meta::param::LIMIT.description,
            grep_meta::param::LIMIT.required,
            Some(1),
            None,
        )
        .build()
        .expect("schema build should not fail");

    ToolDefinition {
        name: grep_meta::NAME.to_owned(),
        description: grep_meta::description::allowed(line_numbers).to_owned(),
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
    async fn finds_matching_content() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("test.txt"), "hello world").unwrap();

        let resolver = AllowedPathResolver::new([dir.path()]).unwrap();
        let tool = GrepTool::new(resolver);
        let result = tool
            .call(
                &mock_ctx(),
                json!({
                    "pattern": "hello",
                    "path": "."
                }),
            )
            .await
            .unwrap();

        let text = result.as_text().unwrap();
        assert!(text.contains("Found 1 matches"));
        assert!(text.contains("L1: hello world"));
    }

    #[tokio::test]
    async fn rejects_path_traversal() {
        let dir = TempDir::new().unwrap();
        let resolver = AllowedPathResolver::new([dir.path()]).unwrap();
        let tool = GrepTool::new(resolver);
        let result = tool
            .call(
                &mock_ctx(),
                json!({
                    "pattern": "test",
                    "path": "../../../etc"
                }),
            )
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn rejects_empty_pattern() {
        let dir = TempDir::new().unwrap();
        let resolver = AllowedPathResolver::new([dir.path()]).unwrap();
        let tool = GrepTool::new(resolver);
        let result = tool
            .call(
                &mock_ctx(),
                json!({
                    "pattern": "   ",
                    "path": "."
                }),
            )
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn returns_partial_json_when_search_has_errors() {
        let dir = TempDir::new().unwrap();
        let resolver = AllowedPathResolver::new([dir.path()]).unwrap();
        let tool = GrepTool::new(resolver);
        let result = tool
            .call(
                &mock_ctx(),
                json!({
                    "pattern": "hello",
                    "path": "missing-root"
                }),
            )
            .await
            .unwrap();

        let payload = result.as_json().unwrap();
        assert_eq!(payload["partial"], true);
        assert!(!payload["errors"].as_array().unwrap().is_empty());
        assert!(
            payload["content"]
                .as_str()
                .unwrap()
                .contains("Partial results")
        );
    }
}
