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
    /// Maximum number of lines to return. Defaults to 2000.
    #[serde(default = "read_meta::default_limit")]
    limit: usize,
}

/// Tool for reading file contents with optional line numbers.
///
/// The `LINE_NUMBERS` const generic controls output format:
/// - `true` (default): Lines prefixed with `L{number}: `
/// - `false`: Raw file content
#[derive(Debug, Clone)]
pub struct ReadTool<const LINE_NUMBERS: bool = true> {
    definition: ToolDefinition,
}

impl<const LINE_NUMBERS: bool> Default for ReadTool<LINE_NUMBERS> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const LINE_NUMBERS: bool> ReadTool<LINE_NUMBERS> {
    /// Creates a new read tool instance.
    #[inline]
    pub fn new() -> Self {
        Self {
            definition: build_definition::<LINE_NUMBERS>(),
        }
    }
}

#[async_trait]
impl<Deps: Send + Sync, const LINE_NUMBERS: bool> Tool<Deps> for ReadTool<LINE_NUMBERS> {
    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    async fn call(&self, _ctx: &RunContext<Deps>, args: serde_json::Value) -> ToolResult {
        let args: ReadArgs = serde_json::from_value(args)
            .map_err(|e| ToolError::validation_error(read_meta::NAME, None, e.to_string()))?;

        let resolver = AbsolutePathResolver;
        // Core uses 1-indexed offset directly; args.offset defaults to 1
        let result =
            read_file::<_, LINE_NUMBERS>(&resolver, &args.file_path, args.offset, args.limit).await;
        to_serdes_result(read_meta::NAME, result)
    }
}

impl<const LINE_NUMBERS: bool> ToolContext for ReadTool<LINE_NUMBERS> {
    const NAME: &'static str = read_meta::NAME;

    fn context(&self) -> ToolPrompt {
        ToolPrompt::Read {
            path_mode: PathMode::Absolute,
            line_numbers: LINE_NUMBERS,
        }
    }
}

fn build_definition<const LINE_NUMBERS: bool>() -> ToolDefinition {
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
        description: read_meta::description::absolute(LINE_NUMBERS).to_owned(),
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
        let tool: ReadTool<true> = ReadTool::new();

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
