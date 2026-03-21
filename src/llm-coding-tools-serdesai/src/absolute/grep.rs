//! Grep content search tool using [`AbsolutePathResolver`].

use async_trait::async_trait;
use llm_coding_tools_core::ToolContext;
use llm_coding_tools_core::context::{PathMode, ToolPrompt};
use llm_coding_tools_core::path::AbsolutePathResolver;
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
    /// Absolute directory path to search in.
    path: String,
    /// File pattern to filter search results (e.g., "*.rs", "*.{ts,tsx}").
    #[serde(default)]
    include: Option<String>,
    /// Maximum number of matches to return (default: 100, max: 2000).
    #[serde(default)]
    limit: Option<usize>,
}

/// Tool for searching file contents using regex patterns.
///
/// The `LINE_NUMBERS` const generic controls output format:
/// - `true` (default): Lines prefixed with `L{number}: `
/// - `false`: Raw matching lines
#[derive(Debug, Clone, Default)]
pub struct GrepTool<const LINE_NUMBERS: bool = true>;

impl<const LINE_NUMBERS: bool> GrepTool<LINE_NUMBERS> {
    /// Creates a new grep tool instance.
    #[inline]
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl<Deps: Send + Sync, const LINE_NUMBERS: bool> Tool<Deps> for GrepTool<LINE_NUMBERS> {
    fn definition(&self) -> ToolDefinition {
        let schema = SchemaBuilder::new()
            .string(
                grep_meta::param::PATTERN.name,
                grep_meta::param::PATTERN.description,
                grep_meta::param::PATTERN.required,
            )
            .string(
                grep_meta::param::PATH_ABSOLUTE.name,
                grep_meta::param::PATH_ABSOLUTE.description,
                grep_meta::param::PATH_ABSOLUTE.required,
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
                Some(grep_meta::MAX_LIMIT as i64),
            )
            .build()
            .expect("schema build should not fail");

        ToolDefinition::new(
            grep_meta::NAME,
            grep_meta::description::absolute(LINE_NUMBERS),
        )
        .with_parameters(schema)
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

        let limit = args
            .limit
            .unwrap_or(grep_meta::DEFAULT_LIMIT)
            .min(grep_meta::MAX_LIMIT);
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

        let resolver = AbsolutePathResolver;
        let result = grep_search(&resolver, pattern, include, &args.path, limit);

        match result {
            Err(e) => to_serdes_result(grep_meta::NAME, Err(e)),
            Ok(grep_output) => Ok(grep_output_to_return::<LINE_NUMBERS>(
                grep_output,
                limit,
                DEFAULT_MAX_LINE_LENGTH,
            )),
        }
    }
}

impl<const LINE_NUMBERS: bool> ToolContext for GrepTool<LINE_NUMBERS> {
    const NAME: &'static str = grep_meta::NAME;

    fn context(&self) -> ToolPrompt {
        ToolPrompt::Grep {
            path_mode: PathMode::Absolute,
            line_numbers: LINE_NUMBERS,
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
    async fn finds_content_with_required_path() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("test.txt"), "hello world\nfoo bar").unwrap();

        let tool: GrepTool<true> = GrepTool::new();
        let result = tool
            .call(
                &mock_ctx(),
                json!({
                    "pattern": "hello",
                    "path": dir.path().to_string_lossy()
                }),
            )
            .await
            .unwrap();

        let text = result.as_text().unwrap();
        assert!(text.contains("Found 1 matches"));
        assert!(text.contains("L1: hello world"));
    }

    #[tokio::test]
    async fn validates_limit() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("test.txt"), "hello").unwrap();

        let tool: GrepTool<true> = GrepTool::new();
        let result = tool
            .call(
                &mock_ctx(),
                json!({
                    "pattern": "hello",
                    "path": dir.path().to_string_lossy(),
                    "limit": 0
                }),
            )
            .await;

        let err = result.unwrap_err();
        assert!(matches!(err, ToolError::ValidationFailed { .. }));
        // Check the error contains the validation message
        match err {
            ToolError::ValidationFailed { errors, .. } => {
                assert!(!errors.is_empty());
                assert!(errors[0].message.contains("limit"));
            }
            _ => panic!("Expected ValidationFailed"),
        }
    }

    #[tokio::test]
    async fn returns_no_matches_message_when_empty() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("test.txt"), "hello world").unwrap();

        let tool: GrepTool<true> = GrepTool::new();
        let result = tool
            .call(
                &mock_ctx(),
                json!({
                    "pattern": "nonexistent_pattern_xyz",
                    "path": dir.path().to_string_lossy()
                }),
            )
            .await
            .unwrap();

        let text = result.as_text().unwrap();
        assert_eq!(text, "No matches found.");
    }

    #[tokio::test]
    async fn include_filter_restricts_to_matching_files() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("code.rs"), "fn hello() {}").unwrap();
        std::fs::write(dir.path().join("code.py"), "def hello(): pass").unwrap();
        std::fs::write(dir.path().join("readme.txt"), "hello world").unwrap();

        let tool: GrepTool<true> = GrepTool::new();

        // Search only .rs files
        let result = tool
            .call(
                &mock_ctx(),
                json!({
                    "pattern": "hello",
                    "path": dir.path().to_string_lossy(),
                    "include": "*.rs"
                }),
            )
            .await
            .unwrap();

        let text = result.as_text().unwrap();
        assert!(text.contains("Found 1 matches"));
        assert!(text.contains("code.rs"));
        assert!(!text.contains("code.py"));
        assert!(!text.contains("readme.txt"));
    }

    #[tokio::test]
    async fn returns_partial_json_when_search_has_errors() {
        let dir = TempDir::new().unwrap();
        let missing_path = dir.path().join("missing-root");
        let tool: GrepTool<true> = GrepTool::new();

        let result = tool
            .call(
                &mock_ctx(),
                json!({
                    "pattern": "hello",
                    "path": missing_path.to_string_lossy()
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

    #[tokio::test]
    async fn truncates_long_lines_at_max_length() {
        let dir = TempDir::new().unwrap();
        // Create a line longer than MAX_LINE_LENGTH (2000 chars)
        let long_line = format!("prefix_{}_suffix", "x".repeat(2500));
        std::fs::write(dir.path().join("long.txt"), &long_line).unwrap();

        let tool: GrepTool<true> = GrepTool::new();
        let result = tool
            .call(
                &mock_ctx(),
                json!({
                    "pattern": "prefix",
                    "path": dir.path().to_string_lossy()
                }),
            )
            .await
            .unwrap();

        let text = result.as_text().unwrap();
        assert!(text.contains("Found 1 matches"));
        // The line should be truncated - it should contain prefix but not suffix
        assert!(text.contains("prefix_"));
        assert!(!text.contains("_suffix"));
        // Verify the match line doesn't exceed DEFAULT_MAX_LINE_LENGTH
        for line in text.lines() {
            if line.contains("prefix_") {
                // Line format is "  L1: content", so actual content is line.len() - prefix
                let content_start = line.find("prefix_").unwrap();
                let content = &line[content_start..];
                assert!(content.len() <= DEFAULT_MAX_LINE_LENGTH);
            }
        }
    }
}
