//! Grep content search tool using [`AbsolutePathResolver`].

use coding_tools_core::operations::grep_search;
use coding_tools_core::path::AbsolutePathResolver;
use coding_tools_core::{ToolContext, ToolError, ToolOutput};
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use schemars::{schema_for, JsonSchema};
use serde::Deserialize;
use std::fmt::Write;

const DEFAULT_LIMIT: usize = 100;
const MAX_LIMIT: usize = 2000;
const MAX_LINE_LENGTH: usize = 2000;

fn default_limit() -> Option<usize> {
    Some(DEFAULT_LIMIT)
}

/// Arguments for the grep tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GrepArgs {
    /// Regex pattern to search for in file contents.
    pub pattern: String,
    /// Absolute directory path to search in.
    pub path: String,
    /// Optional file glob filter (e.g., "*.rs", "*.{ts,tsx}").
    #[serde(default)]
    pub include: Option<String>,
    /// Maximum number of matches to return (default: 100, max: 2000).
    #[serde(default = "default_limit")]
    pub limit: Option<usize>,
}

/// Tool for searching file contents using regex patterns.
#[derive(Debug, Clone, Default)]
pub struct GrepTool<const LINE_NUMBERS: bool = true>;

impl<const LINE_NUMBERS: bool> GrepTool<LINE_NUMBERS> {
    /// Creates a new grep tool instance.
    #[inline]
    pub fn new() -> Self {
        Self
    }
}

impl<const LINE_NUMBERS: bool> Tool for GrepTool<LINE_NUMBERS> {
    const NAME: &'static str = "grep";

    type Error = ToolError;
    type Args = GrepArgs;
    type Output = ToolOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        let description = if LINE_NUMBERS {
            "Search file contents using regex patterns. Returns matches with file paths, \
                line numbers, and content, sorted by file modification time."
        } else {
            "Search file contents using regex patterns. Returns matches with file paths \
                and content, sorted by file modification time."
        };
        ToolDefinition {
            name: <Self as Tool>::NAME.to_string(),
            description: description.to_string(),
            parameters: serde_json::to_value(schema_for!(GrepArgs))
                .expect("schema serialization should not fail"),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let pattern = args.pattern.trim();
        if pattern.is_empty() {
            return Err(ToolError::InvalidPattern(
                "pattern must not be empty".into(),
            ));
        }

        let limit = args.limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT);
        if limit == 0 {
            return Err(ToolError::InvalidPattern(
                "limit must be greater than zero".into(),
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
        let result = grep_search(&resolver, pattern, include, &args.path, limit)?;

        if result.files.is_empty() {
            return Ok(ToolOutput::new("No matches found."));
        }

        // Format output grouped by file
        let mut output = String::with_capacity(4096);
        let _ = writeln!(&mut output, "Found {} matches", result.match_count);

        for file in &result.files {
            let _ = writeln!(&mut output, "\n{}:", file.path);
            for m in &file.matches {
                let truncated_text = if m.line_text.len() > MAX_LINE_LENGTH {
                    &m.line_text[..MAX_LINE_LENGTH]
                } else {
                    &m.line_text
                };
                if LINE_NUMBERS {
                    let _ = writeln!(&mut output, "  L{}: {}", m.line_num, truncated_text);
                } else {
                    let _ = writeln!(&mut output, "  {}", truncated_text);
                }
            }
        }

        if result.truncated {
            let _ = write!(&mut output, "\n(Results truncated at {} matches)", limit);
        }

        Ok(if result.truncated {
            ToolOutput::truncated(output)
        } else {
            ToolOutput::new(output)
        })
    }
}

impl<const LINE_NUMBERS: bool> ToolContext for GrepTool<LINE_NUMBERS> {
    const NAME: &'static str = "grep";

    fn context(&self) -> &'static str {
        coding_tools_core::context::GREP_ABSOLUTE
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn finds_matching_content() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("test.txt"), "hello world").unwrap();
        let tool: GrepTool<true> = GrepTool::new();
        let result = tool
            .call(GrepArgs {
                pattern: "hello".to_string(),
                path: dir.path().to_string_lossy().to_string(),
                include: None,
                limit: None,
            })
            .await
            .unwrap();
        assert!(result.content.contains("Found 1 matches"));
        assert!(result.content.contains("L1: hello world"));
    }

    #[tokio::test]
    async fn rejects_relative_path() {
        let tool: GrepTool = GrepTool::new();
        let result = tool
            .call(GrepArgs {
                pattern: "test".to_string(),
                path: "relative/path".to_string(),
                include: None,
                limit: None,
            })
            .await;
        assert!(matches!(result, Err(ToolError::InvalidPath(_))));
    }

    #[tokio::test]
    async fn rejects_empty_pattern() {
        let tool: GrepTool = GrepTool::new();
        let result = tool
            .call(GrepArgs {
                pattern: "   ".to_string(),
                path: "/tmp".to_string(),
                include: None,
                limit: None,
            })
            .await;
        assert!(matches!(result, Err(ToolError::InvalidPattern(_))));
    }
}
