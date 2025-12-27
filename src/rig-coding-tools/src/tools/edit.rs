//! Edit tool for exact string replacements in files.

use crate::error::ToolError;
use crate::util::validate_absolute_path;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

/// Tool arguments for file editing.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct EditArgs {
    /// Absolute path to the file to modify.
    pub file_path: String,
    /// Exact text to find and replace.
    pub old_string: String,
    /// Replacement text.
    pub new_string: String,
    /// Replace all occurrences (default false).
    #[serde(default)]
    pub replace_all: bool,
}

/// Errors specific to edit operations.
#[derive(Debug, Error)]
pub enum EditError {
    /// I/O or path validation failed.
    #[error(transparent)]
    Tool(#[from] ToolError),
    /// old_string was empty.
    #[error("old_string must not be empty")]
    EmptyOldString,
    /// old_string and new_string are identical.
    #[error("old_string and new_string must be different")]
    IdenticalStrings,
    /// old_string not found in file.
    #[error("old_string not found in file content")]
    NotFound,
    /// Multiple matches found when replace_all=false.
    #[error("oldString found {0} times and requires more code context to uniquely identify the intended match")]
    AmbiguousMatch(usize),
}

/// Tool for making exact string replacements in files.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EditTool;

impl EditTool {
    /// Creates a new [`EditTool`] instance.
    #[inline]
    pub fn new() -> Self {
        Self
    }

    /// Performs the edit operation.
    async fn execute(args: EditArgs) -> Result<String, EditError> {
        // Validate arguments
        if args.old_string.is_empty() {
            return Err(EditError::EmptyOldString);
        }
        if args.old_string == args.new_string {
            return Err(EditError::IdenticalStrings);
        }

        let path = Path::new(&args.file_path);
        validate_absolute_path(path)?;

        // Read file content
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(ToolError::from)?;

        // Count occurrences
        let count = content.matches(&args.old_string).count();

        if count == 0 {
            return Err(EditError::NotFound);
        }

        if !args.replace_all && count > 1 {
            return Err(EditError::AmbiguousMatch(count));
        }

        // Perform replacement
        let new_content = if args.replace_all {
            content.replace(&args.old_string, &args.new_string)
        } else {
            content.replacen(&args.old_string, &args.new_string, 1)
        };

        // Write back
        tokio::fs::write(path, &new_content)
            .await
            .map_err(ToolError::from)?;

        Ok(format!("Successfully replaced {} occurrence(s)", count))
    }
}

impl Tool for EditTool {
    const NAME: &'static str = "edit";

    type Error = EditError;
    type Args = EditArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Makes exact string replacements in files. Use replace_all=true to replace all occurrences.".to_string(),
            parameters: serde_json::to_value(schema_for!(EditArgs))
                .expect("EditArgs schema generation should not fail"),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Self::execute(args).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    async fn create_temp_file(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file.flush().unwrap();
        file
    }

    #[tokio::test]
    async fn single_replacement_succeeds() {
        let file = create_temp_file("hello world").await;
        let args = EditArgs {
            file_path: file.path().to_string_lossy().to_string(),
            old_string: "world".to_string(),
            new_string: "rust".to_string(),
            replace_all: false,
        };
        let result = EditTool::execute(args).await.unwrap();
        assert!(result.contains("1 occurrence"));
        let content = tokio::fs::read_to_string(file.path()).await.unwrap();
        assert_eq!(content, "hello rust");
    }

    #[tokio::test]
    async fn replace_all_succeeds() {
        let file = create_temp_file("foo bar foo baz foo").await;
        let args = EditArgs {
            file_path: file.path().to_string_lossy().to_string(),
            old_string: "foo".to_string(),
            new_string: "qux".to_string(),
            replace_all: true,
        };
        let result = EditTool::execute(args).await.unwrap();
        assert!(result.contains("3 occurrence"));
        let content = tokio::fs::read_to_string(file.path()).await.unwrap();
        assert_eq!(content, "qux bar qux baz qux");
    }

    #[tokio::test]
    async fn no_match_returns_error() {
        let file = create_temp_file("hello world").await;
        let args = EditArgs {
            file_path: file.path().to_string_lossy().to_string(),
            old_string: "missing".to_string(),
            new_string: "replacement".to_string(),
            replace_all: false,
        };
        let err = EditTool::execute(args).await.unwrap_err();
        assert!(matches!(err, EditError::NotFound));
    }

    #[tokio::test]
    async fn ambiguous_match_returns_error() {
        let file = create_temp_file("foo bar foo").await;
        let args = EditArgs {
            file_path: file.path().to_string_lossy().to_string(),
            old_string: "foo".to_string(),
            new_string: "baz".to_string(),
            replace_all: false,
        };
        let err = EditTool::execute(args).await.unwrap_err();
        assert!(matches!(err, EditError::AmbiguousMatch(2)));
    }

    #[tokio::test]
    async fn empty_old_string_returns_error() {
        let file = create_temp_file("content").await;
        let args = EditArgs {
            file_path: file.path().to_string_lossy().to_string(),
            old_string: "".to_string(),
            new_string: "replacement".to_string(),
            replace_all: false,
        };
        let err = EditTool::execute(args).await.unwrap_err();
        assert!(matches!(err, EditError::EmptyOldString));
    }

    #[tokio::test]
    async fn identical_strings_returns_error() {
        let file = create_temp_file("content").await;
        let args = EditArgs {
            file_path: file.path().to_string_lossy().to_string(),
            old_string: "same".to_string(),
            new_string: "same".to_string(),
            replace_all: false,
        };
        let err = EditTool::execute(args).await.unwrap_err();
        assert!(matches!(err, EditError::IdenticalStrings));
    }

    #[tokio::test]
    async fn relative_path_returns_error() {
        let args = EditArgs {
            file_path: "relative/path.txt".to_string(),
            old_string: "old".to_string(),
            new_string: "new".to_string(),
            replace_all: false,
        };
        let err = EditTool::execute(args).await.unwrap_err();
        assert!(matches!(err, EditError::Tool(ToolError::InvalidPath(_))));
    }

    #[tokio::test]
    async fn file_not_found_returns_error() {
        let args = EditArgs {
            file_path: "/nonexistent/path/file.txt".to_string(),
            old_string: "old".to_string(),
            new_string: "new".to_string(),
            replace_all: false,
        };
        let err = EditTool::execute(args).await.unwrap_err();
        assert!(matches!(err, EditError::Tool(ToolError::Io(_))));
    }

    #[tokio::test]
    async fn preserves_whitespace_exactly() {
        let file = create_temp_file("  indented\n\tmore\n").await;
        let args = EditArgs {
            file_path: file.path().to_string_lossy().to_string(),
            old_string: "indented".to_string(),
            new_string: "REPLACED".to_string(),
            replace_all: false,
        };
        EditTool::execute(args).await.unwrap();
        let content = tokio::fs::read_to_string(file.path()).await.unwrap();
        assert_eq!(content, "  REPLACED\n\tmore\n");
    }
}
