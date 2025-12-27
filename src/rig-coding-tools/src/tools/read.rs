//! Read file tool for reading file contents with line numbers.

use crate::error::{ToolError, ToolResult};
use crate::output::ToolOutput;
use crate::util::{truncate_line, validate_absolute_path};
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use schemars::{schema_for, JsonSchema};
use serde::Deserialize;
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};

const MAX_LINE_LENGTH: usize = 2000;
const DEFAULT_OFFSET: usize = 1;
const DEFAULT_LIMIT: usize = 2000;

fn default_offset() -> usize {
    DEFAULT_OFFSET
}

fn default_limit() -> usize {
    DEFAULT_LIMIT
}

/// Arguments for the read file tool.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ReadArgs {
    /// Absolute path to the file to read.
    pub file_path: String,
    /// 1-indexed line number to start reading from (default: 1).
    #[serde(default = "default_offset")]
    pub offset: usize,
    /// Maximum number of lines to return (default: 2000).
    #[serde(default = "default_limit")]
    pub limit: usize,
}

/// Tool for reading file contents with line numbers.
///
/// Reads files with configurable offset and limit, returning content
/// with line numbers prefixed in the format `L{number}: {content}`.
#[derive(Debug, Clone, Default)]
pub struct ReadTool;

impl ReadTool {
    /// Creates a new read tool instance.
    #[inline]
    pub fn new() -> Self {
        Self
    }
}

impl Tool for ReadTool {
    const NAME: &'static str = "read";

    type Error = ToolError;
    type Args = ReadArgs;
    type Output = ToolOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Read file contents with line numbers. Returns lines prefixed with L{number}: format.".to_string(),
            parameters: serde_json::to_value(schema_for!(ReadArgs))
                .expect("schema serialization should never fail"),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        read_file(&args.file_path, args.offset, args.limit).await
    }
}

/// Reads a file and returns formatted content with line numbers.
async fn read_file(file_path: &str, offset: usize, limit: usize) -> ToolResult<ToolOutput> {
    // Validate arguments
    if offset == 0 {
        return Err(ToolError::OutOfBounds(
            "offset must be >= 1 (1-indexed)".into(),
        ));
    }
    if limit == 0 {
        return Err(ToolError::OutOfBounds("limit must be >= 1".into()));
    }

    let path = Path::new(file_path);
    validate_absolute_path(path)?;

    let file = File::open(path).await?;
    let mut reader = BufReader::new(file);
    let mut buffer = Vec::new();
    let mut collected = Vec::new();
    let mut line_number = 0usize;

    loop {
        buffer.clear();
        let bytes_read = reader.read_until(b'\n', &mut buffer).await?;

        if bytes_read == 0 {
            break;
        }

        // Strip trailing newline characters
        if buffer.last() == Some(&b'\n') {
            buffer.pop();
            if buffer.last() == Some(&b'\r') {
                buffer.pop();
            }
        }

        line_number += 1;

        // Skip lines before offset
        if line_number < offset {
            continue;
        }

        // Stop if we've collected enough lines
        if collected.len() >= limit {
            break;
        }

        // Convert to string with lossy UTF-8 handling
        let content = String::from_utf8_lossy(&buffer);

        // Truncate long lines
        let (truncated_content, _) = truncate_line(&content, MAX_LINE_LENGTH);

        collected.push(format!("L{}: {}", line_number, truncated_content));
    }

    // Check if offset exceeded file length
    if line_number < offset {
        return Err(ToolError::OutOfBounds(format!(
            "offset {} exceeds file length of {} lines",
            offset, line_number
        )));
    }

    Ok(ToolOutput::new(collected.join("\n")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    async fn read_temp_file(content: &[u8], offset: usize, limit: usize) -> ToolResult<ToolOutput> {
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(content).unwrap();
        read_file(temp.path().to_str().unwrap(), offset, limit).await
    }

    #[tokio::test]
    async fn reads_basic_file() {
        let result = read_temp_file(b"hello\nworld\n", 1, 2000).await.unwrap();
        assert_eq!(result.content, "L1: hello\nL2: world");
    }

    #[tokio::test]
    async fn reads_with_offset() {
        let result = read_temp_file(b"one\ntwo\nthree\n", 2, 2000).await.unwrap();
        assert_eq!(result.content, "L2: two\nL3: three");
    }

    #[tokio::test]
    async fn reads_with_limit() {
        let result = read_temp_file(b"one\ntwo\nthree\n", 1, 2).await.unwrap();
        assert_eq!(result.content, "L1: one\nL2: two");
    }

    #[tokio::test]
    async fn reads_with_offset_and_limit() {
        let result = read_temp_file(b"one\ntwo\nthree\nfour\n", 2, 2)
            .await
            .unwrap();
        assert_eq!(result.content, "L2: two\nL3: three");
    }

    #[tokio::test]
    async fn handles_crlf_line_endings() {
        let result = read_temp_file(b"line1\r\nline2\r\n", 1, 2000)
            .await
            .unwrap();
        assert_eq!(result.content, "L1: line1\nL2: line2");
    }

    #[tokio::test]
    async fn handles_non_utf8_content() {
        let result = read_temp_file(b"\xff\xfe\nplain\n", 1, 2000).await.unwrap();
        assert!(result.content.contains("L1:"));
        assert!(result.content.contains('\u{FFFD}')); // replacement char
        assert!(result.content.contains("L2: plain"));
    }

    #[tokio::test]
    async fn truncates_long_lines() {
        let long_line = "x".repeat(MAX_LINE_LENGTH + 100);
        let content = format!("{}\n", long_line);
        let result = read_temp_file(content.as_bytes(), 1, 1).await.unwrap();
        let expected = format!("L1: {}", "x".repeat(MAX_LINE_LENGTH));
        assert_eq!(result.content, expected);
    }

    #[tokio::test]
    async fn errors_on_offset_zero() {
        let err = read_temp_file(b"test\n", 0, 10).await.unwrap_err();
        assert!(matches!(err, ToolError::OutOfBounds(_)));
    }

    #[tokio::test]
    async fn errors_on_limit_zero() {
        let err = read_temp_file(b"test\n", 1, 0).await.unwrap_err();
        assert!(matches!(err, ToolError::OutOfBounds(_)));
    }

    #[tokio::test]
    async fn errors_on_offset_exceeds_file() {
        let err = read_temp_file(b"one\ntwo\n", 10, 100).await.unwrap_err();
        assert!(matches!(err, ToolError::OutOfBounds(_)));
    }

    #[tokio::test]
    async fn errors_on_relative_path() {
        let err = read_file("relative/path.txt", 1, 100).await.unwrap_err();
        assert!(matches!(err, ToolError::InvalidPath(_)));
    }

    #[tokio::test]
    async fn errors_on_nonexistent_file() {
        let err = read_file("/nonexistent/file.txt", 1, 100)
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::Io(_)));
    }

    #[tokio::test]
    async fn handles_empty_file() {
        let result = read_temp_file(b"", 1, 100).await;
        // Empty file with offset 1 should error
        assert!(matches!(result, Err(ToolError::OutOfBounds(_))));
    }

    #[tokio::test]
    async fn handles_file_without_trailing_newline() {
        let result = read_temp_file(b"no trailing newline", 1, 100)
            .await
            .unwrap();
        assert_eq!(result.content, "L1: no trailing newline");
    }
}
