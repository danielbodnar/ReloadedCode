//! File writing operation.

use crate::error::{ToolError, ToolResult};
use crate::fs;
use crate::path::PathResolver;
use serde::Deserialize;
use serde_json::Value;

/// Serde-friendly write request owned by the core crate.
#[derive(Debug, Deserialize)]
pub struct WriteRequest {
    pub file_path: String,
    pub content: String,
}

impl WriteRequest {
    /// Parses a raw JSON tool payload into a write request.
    pub fn parse(args: Value) -> ToolResult<Self> {
        serde_json::from_value(args).map_err(ToolError::from)
    }
}

/// Writes content to a file, creating parent directories if needed.
///
/// Overwrites existing files. Returns a success message with byte count.
#[maybe_async::maybe_async]
pub async fn write_file<R: PathResolver>(
    resolver: &R,
    request: WriteRequest,
) -> ToolResult<String> {
    let path = resolver.resolve(&request.file_path)?;

    // Create parent directories if they don't exist
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).await?;
        }
    }

    let bytes = request.content.as_bytes();
    fs::write(&path, bytes).await?;

    Ok(format!(
        "Successfully wrote {} bytes to {}",
        bytes.len(),
        path.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path::AbsolutePathResolver;
    use tempfile::TempDir;

    #[maybe_async::test(feature = "blocking", async(feature = "tokio", tokio::test))]
    async fn write_creates_new_file() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("new_file.txt");
        let resolver = AbsolutePathResolver;

        let result = write_file(
            &resolver,
            WriteRequest {
                file_path: file_path.to_str().unwrap().to_string(),
                content: "hello world".to_string(),
            },
        )
        .await
        .unwrap();

        assert!(result.contains("11 bytes"));
        assert_eq!(std::fs::read_to_string(&file_path).unwrap(), "hello world");
    }

    #[maybe_async::test(feature = "blocking", async(feature = "tokio", tokio::test))]
    async fn write_creates_parent_directories() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("a/b/c/deep.txt");
        let resolver = AbsolutePathResolver;

        write_file(
            &resolver,
            WriteRequest {
                file_path: file_path.to_str().unwrap().to_string(),
                content: "nested".to_string(),
            },
        )
        .await
        .unwrap();

        assert!(file_path.exists());
    }
}
