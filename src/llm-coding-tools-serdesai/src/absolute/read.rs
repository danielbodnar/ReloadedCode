//! Read file tool using [`AbsolutePathResolver`].

use async_trait::async_trait;
use llm_coding_tools_core::ToolContext;
use llm_coding_tools_core::context::{PathMode, ToolPrompt};
use llm_coding_tools_core::path::AbsolutePathResolver;
use llm_coding_tools_core::tool_metadata::read as read_meta;
use llm_coding_tools_core::tools::read_file;
use serde::Deserialize;
use serdes_ai::tools::{RunContext, SchemaBuilder, Tool, ToolDefinition, ToolError, ToolResult};

use crate::convert::to_serdes_result;

/// Internal args for JSON deserialization.
#[derive(Debug, Deserialize)]
struct ReadArgs {
    /// Absolute path to the file.
    file_path: String,
    /// Line offset to start reading from (1-based). Defaults to 1.
    #[serde(default = "read_meta::default_offset")]
    offset: usize,
    /// Maximum number of lines to return. Uses tool default if not specified.
    #[serde(default)]
    limit: Option<usize>,
}

/// Tool for reading file contents with optional line numbers.
///
/// The `line_numbers` field controls output format:
/// - `true` (default): Lines prefixed with `L{number}: `
/// - `false`: Raw file content
#[derive(Debug, Clone)]
pub struct ReadTool {
    definition: ToolDefinition,
    limit: usize,
    max_line_length: usize,
    line_numbers: bool,
}

impl Default for ReadTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ReadTool {
    /// Creates a new read tool instance with default settings.
    ///
    /// Uses `limit` of 2000 lines, `max_line_length` of 2000 characters,
    /// and enables line numbers.
    #[inline]
    pub fn new() -> Self {
        Self::with_settings(read_meta::DEFAULT_LIMIT, read_meta::MAX_LINE_LENGTH, true)
    }

    /// Creates a new read tool instance with custom settings.
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum number of lines to return per read call.
    ///   This is the default used when the LLM doesn't specify a limit.
    /// * `max_line_length` - Maximum characters per line before truncation.
    ///   Longer lines will be truncated with "..." appended.
    /// * `line_numbers` - Whether to prefix lines with line numbers.
    pub fn with_settings(limit: usize, max_line_length: usize, line_numbers: bool) -> Self {
        Self {
            definition: build_definition(line_numbers),
            limit,
            max_line_length,
            line_numbers,
        }
    }
}

#[async_trait]
impl<Deps: Send + Sync> Tool<Deps> for ReadTool {
    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    async fn call(&self, _ctx: &RunContext<Deps>, args: serde_json::Value) -> ToolResult {
        let args: ReadArgs = serde_json::from_value(args)
            .map_err(|e| ToolError::validation_error(read_meta::NAME, None, e.to_string()))?;

        let resolver = AbsolutePathResolver;
        // Use provided limit or fall back to settings default
        let effective_limit = args.limit.unwrap_or(self.limit);
        // Core uses 1-indexed offset directly; args.offset defaults to 1
        let result = read_file::<_>(
            &resolver,
            &args.file_path,
            args.offset,
            effective_limit,
            self.max_line_length,
            self.line_numbers,
        )
        .await;
        to_serdes_result(read_meta::NAME, result)
    }
}

impl ToolContext for ReadTool {
    const NAME: &'static str = read_meta::NAME;

    fn context(&self) -> ToolPrompt {
        ToolPrompt::Read {
            path_mode: PathMode::Absolute,
            line_numbers: self.line_numbers,
        }
    }
}

fn build_definition(line_numbers: bool) -> ToolDefinition {
    let schema = SchemaBuilder::new()
        .string(
            read_meta::param::FILE_PATH_ABSOLUTE.name,
            read_meta::param::FILE_PATH_ABSOLUTE.description,
            read_meta::param::FILE_PATH_ABSOLUTE.required,
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

    ToolDefinition {
        name: read_meta::NAME.to_owned(),
        description: read_meta::description::absolute(line_numbers).to_owned(),
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
    use std::io::Write as _;
    use tempfile::NamedTempFile;

    fn mock_ctx() -> RunContext<()> {
        RunContext::new((), "test-model")
    }

    #[tokio::test]
    async fn reads_file_with_offset_and_limit() {
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"line1\nline2\nline3\nline4\n").unwrap();
        let tool: ReadTool = ReadTool::new();

        let result = tool
            .call(
                &mock_ctx(),
                json!({
                    "file_path": temp.path().to_string_lossy(),
                    "offset": 2,
                    "limit": 2
                }),
            )
            .await
            .unwrap();

        let text = result.as_text().unwrap();
        assert!(text.contains("L2: line2"));
        assert!(text.contains("L3: line3"));
        assert!(!text.contains("L1:"));
        assert!(!text.contains("L4:"));
    }
}
