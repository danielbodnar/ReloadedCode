//! Write tool for creating or overwriting files.

use crate::error::ToolError;
use crate::util::validate_absolute_path;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use schemars::{schema_for, JsonSchema};
use serde::Deserialize;
use std::path::Path;

/// Tool for writing content to files on the filesystem.
///
/// Creates parent directories if they don't exist and overwrites
/// existing files.
#[derive(Debug, Clone, Default)]
pub struct WriteTool;

/// Arguments for the write tool.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct WriteToolArgs {
    /// Absolute path for the file to write.
    pub file_path: String,
    /// Content to write to the file.
    pub content: String,
}

impl WriteTool {
    /// Creates a new [`WriteTool`] instance.
    #[inline]
    pub fn new() -> Self {
        Self
    }
}

impl Tool for WriteTool {
    const NAME: &'static str = "write";

    type Error = ToolError;
    type Args = WriteToolArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Write content to a file, creating parent directories if needed. \
                          Overwrites existing files."
                .to_string(),
            parameters: serde_json::to_value(schema_for!(WriteToolArgs))
                .expect("schema generation should not fail"),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let path = Path::new(&args.file_path);
        validate_absolute_path(path)?;

        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }

        // Write content to file
        let bytes = args.content.as_bytes();
        tokio::fs::write(path, bytes).await?;

        Ok(format!(
            "Successfully wrote {} bytes to {}",
            bytes.len(),
            path.display()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn write_creates_new_file() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("new_file.txt");
        let tool = WriteTool::new();

        let result = tool
            .call(WriteToolArgs {
                file_path: file_path.to_string_lossy().to_string(),
                content: "hello world".to_string(),
            })
            .await
            .unwrap();

        assert!(result.contains("11 bytes"));
        assert_eq!(std::fs::read_to_string(&file_path).unwrap(), "hello world");
    }

    #[tokio::test]
    async fn write_overwrites_existing_file() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("existing.txt");
        std::fs::write(&file_path, "old content").unwrap();

        let tool = WriteTool::new();
        tool.call(WriteToolArgs {
            file_path: file_path.to_string_lossy().to_string(),
            content: "new content".to_string(),
        })
        .await
        .unwrap();

        assert_eq!(std::fs::read_to_string(&file_path).unwrap(), "new content");
    }

    #[tokio::test]
    async fn write_creates_parent_directories() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("a/b/c/deep.txt");
        let tool = WriteTool::new();

        tool.call(WriteToolArgs {
            file_path: file_path.to_string_lossy().to_string(),
            content: "nested".to_string(),
        })
        .await
        .unwrap();

        assert!(file_path.exists());
        assert_eq!(std::fs::read_to_string(&file_path).unwrap(), "nested");
    }

    #[tokio::test]
    async fn write_empty_content_creates_empty_file() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("empty.txt");
        let tool = WriteTool::new();

        let result = tool
            .call(WriteToolArgs {
                file_path: file_path.to_string_lossy().to_string(),
                content: String::new(),
            })
            .await
            .unwrap();

        assert!(result.contains("0 bytes"));
        assert!(file_path.exists());
        assert_eq!(std::fs::read_to_string(&file_path).unwrap(), "");
    }

    #[tokio::test]
    async fn write_rejects_relative_path() {
        let tool = WriteTool::new();

        let result = tool
            .call(WriteToolArgs {
                file_path: "relative/path.txt".to_string(),
                content: "content".to_string(),
            })
            .await;

        assert!(matches!(result, Err(ToolError::InvalidPath(_))));
    }

    #[tokio::test]
    async fn definition_returns_valid_schema() {
        let tool = WriteTool::new();
        let def = tool.definition(String::new()).await;

        assert_eq!(def.name, "write");
        assert!(def.description.contains("Write content"));

        let params = def.parameters.as_object().unwrap();
        let props = params["properties"].as_object().unwrap();
        assert!(props.contains_key("file_path"));
        assert!(props.contains_key("content"));
    }
}
