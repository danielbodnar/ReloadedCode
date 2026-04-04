//! Generic read file tool using any [`PathResolver`].
//!
//! Works with [`AbsolutePathResolver`], [`AllowedPathResolver`], or custom resolvers.
//!
//! [`PathResolver`]: llm_coding_tools_core::path::PathResolver
//! [`AbsolutePathResolver`]: llm_coding_tools_core::path::AbsolutePathResolver
//! [`AllowedPathResolver`]: llm_coding_tools_core::path::AllowedPathResolver

use async_trait::async_trait;
use llm_coding_tools_core::ToolContext;
use llm_coding_tools_core::context::{PathMode, ToolPrompt};
use llm_coding_tools_core::path::PathResolver;
use llm_coding_tools_core::tool_metadata::read as read_meta;
use llm_coding_tools_core::tools::read_file;
use serde::Deserialize;
use serdes_ai::tools::{RunContext, SchemaBuilder, Tool, ToolDefinition, ToolError, ToolResult};

use crate::convert::to_serdes_result;

/// Internal args for JSON deserialization.
#[derive(Debug, Deserialize)]
struct ReadArgs {
    file_path: String,
    #[serde(default = "read_meta::default_offset")]
    offset: usize,
    #[serde(default)]
    limit: Option<usize>,
}

/// Tool for reading file contents with optional line numbers.
///
/// Generic over any [`PathResolver`] implementation.
///
/// # Example
///
/// ```no_run
/// use llm_coding_tools_serdesai::{ReadTool, AbsolutePathResolver};
///
/// let tool = ReadTool::new(AbsolutePathResolver);
/// ```
#[derive(Debug, Clone)]
pub struct ReadTool<R: PathResolver + Clone> {
    definition: ToolDefinition,
    resolver: R,
    path_mode: PathMode,
    limit: usize,
    max_line_length: usize,
    line_numbers: bool,
}

impl<R: PathResolver + Clone> ReadTool<R> {
    /// Creates a new read tool with the given resolver and default settings.
    ///
    /// Uses `limit` of 2000 lines, `max_line_length` of 2000 characters,
    /// and enables line numbers.
    ///
    /// # Type Parameters
    ///
    /// * `R` - A path resolver implementing [`PathResolver`]. The tool will
    ///   automatically determine the correct path mode (Absolute or Allowed)
    ///   based on the resolver type at construction.
    pub fn new(resolver: R) -> Self {
        Self::with_settings(resolver, read_meta::DEFAULT_LIMIT, read_meta::MAX_LINE_LENGTH, true)
    }

    /// Creates a new read tool with custom settings.
    ///
    /// # Arguments
    ///
    /// * `resolver` - The path resolver for path validation and resolution.
    /// * `limit` - Maximum number of lines to return per read call.
    /// * `max_line_length` - Maximum characters per line before truncation.
    /// * `line_numbers` - Whether to prefix lines with line numbers.
    pub fn with_settings(resolver: R, limit: usize, max_line_length: usize, line_numbers: bool) -> Self {
        let path_mode = determine_path_mode::<R>();
        Self {
            definition: build_definition(path_mode, line_numbers),
            resolver,
            path_mode,
            limit,
            max_line_length,
            line_numbers,
        }
    }

    /// Returns the path mode for this tool instance.
    ///
    /// The path mode is determined at construction based on the resolver type.
    #[must_use]
    pub fn path_mode(&self) -> PathMode {
        self.path_mode
    }
}

#[async_trait]
impl<R: PathResolver + Clone + Send + Sync, Deps: Send + Sync> Tool<Deps> for ReadTool<R> {
    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    async fn call(&self, _ctx: &RunContext<Deps>, args: serde_json::Value) -> ToolResult {
        let args: ReadArgs = serde_json::from_value(args)
            .map_err(|e| ToolError::validation_error(read_meta::NAME, None, e.to_string()))?;

        let effective_limit = args.limit.unwrap_or(self.limit);
        let result = read_file(
            &self.resolver,
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

impl<R: PathResolver + Clone> ToolContext for ReadTool<R> {
    const NAME: &'static str = read_meta::NAME;

    fn context(&self) -> ToolPrompt {
        ToolPrompt::Read {
            path_mode: self.path_mode,
            line_numbers: self.line_numbers,
        }
    }
}

/// Determine the path mode for a resolver type.
///
/// This is used at construction to set the correct path mode based on
/// the resolver type. The path mode affects:
/// - ToolContext::context() return value
/// - Schema parameter names and descriptions
fn determine_path_mode<R: PathResolver>() -> PathMode {
    // Use type name to determine path mode at compile time
    // AbsolutePathResolver -> Absolute
    // AllowedPathResolver -> Allowed
    // Any other resolver defaults to Absolute
    let type_name = std::any::type_name::<R>();
    if type_name.contains("AllowedPathResolver") {
        PathMode::Allowed
    } else {
        PathMode::Absolute
    }
}

fn build_definition(path_mode: PathMode, line_numbers: bool) -> ToolDefinition {
    let (file_path_param, description) = match path_mode {
        PathMode::Absolute => (
            read_meta::param::FILE_PATH_ABSOLUTE,
            read_meta::description::absolute(line_numbers),
        ),
        PathMode::Allowed => (
            read_meta::param::FILE_PATH_ALLOWED,
            read_meta::description::allowed(line_numbers),
        ),
    };

    let schema = SchemaBuilder::new()
        .string(file_path_param.name, file_path_param.description, file_path_param.required)
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
        description: description.to_owned(),
        parameters_json_schema: schema,
        strict: None,
        outer_typed_dict_key: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use llm_coding_tools_core::path::AbsolutePathResolver;
    use llm_coding_tools_core::path::AllowedPathResolver;
    use serde_json::json;
    use serdes_ai::tools::{RunContext, Tool, ToolDefinition};
    use std::io::Write as _;
    use tempfile::{NamedTempFile, TempDir};

    fn mock_ctx() -> RunContext<()> {
        RunContext::new((), "test-model")
    }

    #[tokio::test]
    async fn reads_file_with_absolute_resolver() {
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"line1\nline2\nline3\nline4\n").unwrap();
        let tool = ReadTool::new(AbsolutePathResolver);

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

    #[tokio::test]
    async fn reads_file_with_allowed_resolver() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("test.txt"), "hello\nworld\n").unwrap();

        let resolver = AllowedPathResolver::new([dir.path()]).unwrap();
        let tool = ReadTool::new(resolver);
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
    async fn rejects_path_traversal_with_allowed_resolver() {
        let dir = TempDir::new().unwrap();
        let resolver = AllowedPathResolver::new([dir.path()]).unwrap();
        let tool: ReadTool<AllowedPathResolver> = ReadTool::new(resolver);
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

    #[tokio::test]
    async fn respects_custom_settings() {
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"line1\nline2\n").unwrap();
        let tool = ReadTool::with_settings(AbsolutePathResolver, 1, 100, false);

        let result = tool
            .call(
                &mock_ctx(),
                json!({
                    "file_path": temp.path().to_string_lossy()
                }),
            )
            .await
            .unwrap();

        let text = result.as_text().unwrap();
        assert!(text.contains("line1"));
        assert!(!text.contains("L1:")); // line numbers disabled
    }

    #[test]
    fn determines_correct_path_mode_for_absolute_resolver() {
        let tool = ReadTool::new(AbsolutePathResolver);
        assert_eq!(tool.path_mode(), PathMode::Absolute);
        
        // Verify context returns correct path mode
        match tool.context() {
            ToolPrompt::Read { path_mode, .. } => {
                assert_eq!(path_mode, PathMode::Absolute);
            }
            _ => panic!("Expected ToolPrompt::Read"),
        }
    }

    #[test]
    fn determines_correct_path_mode_for_allowed_resolver() {
        let dir = TempDir::new().unwrap();
        let resolver = AllowedPathResolver::new([dir.path()]).unwrap();
        let tool = ReadTool::new(resolver);
        assert_eq!(tool.path_mode(), PathMode::Allowed);
        
        // Verify context returns correct path mode
        match tool.context() {
            ToolPrompt::Read { path_mode, .. } => {
                assert_eq!(path_mode, PathMode::Allowed);
            }
            _ => panic!("Expected ToolPrompt::Read"),
        }
    }

    #[test]
    fn uses_correct_metadata_for_absolute_resolver() {
        let tool = ReadTool::new(AbsolutePathResolver);
        let def: ToolDefinition = Tool::<()>::definition(&tool);
        
        // Verify the description is the absolute variant
        assert!(def.description.contains("Read a file"));
        assert!(!def.description.contains("allowed"));
        
        // Verify the schema uses absolute parameter
        let schema = &def.parameters_json_schema;
        let props = schema.get("properties").unwrap().as_object().unwrap();
        let file_path = props.get("file_path").unwrap();
        let description = file_path.get("description").unwrap().as_str().unwrap();
        assert_eq!(description, "Absolute file path.");
    }

    #[test]
    fn uses_correct_metadata_for_allowed_resolver() {
        let dir = TempDir::new().unwrap();
        let resolver = AllowedPathResolver::new([dir.path()]).unwrap();
        let tool = ReadTool::new(resolver);
        let def: ToolDefinition = Tool::<()>::definition(&tool);
        
        // Verify the description is the allowed variant
        assert!(def.description.contains("allowed"));
        
        // Verify the schema uses allowed parameter
        let schema = &def.parameters_json_schema;
        let props = schema.get("properties").unwrap().as_object().unwrap();
        let file_path = props.get("file_path").unwrap();
        let description = file_path.get("description").unwrap().as_str().unwrap();
        assert!(description.contains("allowed directory"));
    }
}
